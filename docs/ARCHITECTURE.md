# System Architecture: HermesGate

The **HermesGate** system provides an end-to-end, multi-device, multi-phone-number SMS ingestion, forwarding, and consumption pipeline built for humans, automated systems, and AI agents.

```
                  ┌───────────────────────────────┐
                  │      Android Phone (SIM 1/2)  │
                  │   SmsBroadcastReceiver        │
                  │   WorkManager / SQLite Queue  │
                  └───────────────┬───────────────┘
                                  │ POST /api/v1/sms/forward
                                  │ (X-Device-Token)
                  ┌───────────────┴───────────────┐
                  │         iPhone (iOS 17+)      │
                  │   Apple Shortcuts Automation  │
                  │   Swift Companion App         │
                  └───────────────┬───────────────┘
                                  │
                                  ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        RUST BACKEND SERVER                             │
│                                                                        │
│   ┌────────────────────┐  ┌──────────────────┐  ┌──────────────────┐   │
│   │   Axum Web Router  │  │  2FA / OTP Engine│  │  SQLite Database │   │
│   │   HTTP/1.1 & WS    │  │  Regex Extractor │  │  WAL Mode        │   │
│   └─────────┬──────────┘  └────────┬─────────┘  └────────┬─────────┘   │
│             │                      │                     │             │
│   ┌─────────┴──────────────────────┴─────────────────────┴─────────┐   │
│   │      Authorization Engine: Strict Per-Number Permissions       │   │
│   └────────────────────────────────┬───────────────────────────────┘   │
│                                    │                                   │
│            ┌───────────────────────┼───────────────────────┐           │
│            ▼                       ▼                       ▼           │
│     Human Web UI             REST API             MCP Interface        │
│     Real-Time WebSocket      Automation & CI      AI Agents / Tools    │
└────────────┬───────────────────────┬───────────────────────┬───────────┘
             │                       │                       │
             ▼                       ▼                       ▼
      Browser Client           Automation Scripts      Claude / Gemini /
     (Zero-dep embed)          (GitHub Actions/cURL)    Antigravity AI
```

---

## 1. Core Principles

1. **Multi-Number Support**: The system can ingest SMS messages for dozens or hundreds of phone numbers across multiple physical Android and iOS devices.
2. **Granular Per-Number Authorization**:
   - Every API token and MCP session is assigned a specific list of permitted phone numbers (e.g. `["+15551234567"]`).
   - Tokens can never read messages or extract OTPs for numbers they are not authorized to view.
   - Wildcard (`"*"`) tokens exist only for administrative oversight.
3. **High Reliability & Offline Resilience**:
   - Mobile clients store incoming SMS in a local SQLite queue immediately.
   - If Wi-Fi/cellular network drops, messages are enqueued and retried with exponential backoff via Android `WorkManager`.
4. **Single-Binary Zero-Dependency Deployment**:
   - The Rust server compiles to a standalone binary containing the SQLite database engine, API router, MCP server, and embedded SPA Web UI.

---

## 2. Ingestion Pipeline

1. **SMS Arrival**: The mobile OS receives a cellular SMS.
   - **Android**: `SmsBroadcastReceiver` intercepts `android.provider.Telephony.SMS_RECEIVED` with priority 999. It combines multi-part PDU fragments into full text and identifies the SIM slot index.
   - **iOS**: Apple Shortcuts Automation intercepts incoming SMS and passes sender and content to an HTTP POST action.
2. **Device Authentication**: Mobile device sends `POST /api/v1/sms/forward` with `X-Device-Token`. The server validates the device token hash.
3. **OTP / 2FA Extraction**: The server runs the regex extraction pipeline on the message text. High-confidence codes (e.g. `492810`, `G-582910`, `839-201`) are tagged in `extracted_code`.
4. **Persistence**: Message is inserted into SQLite `messages` table, indexing `recipient_number`, `received_at`, and `sender`.
5. **Real-Time Dispatch**: The server broadcasts an `Arc<Message>` to the in-memory Tokio broadcast channel. Connected WebSocket clients and SSE listeners receive the message if authorized for that phone number.

---

## 3. Database Schema

The SQLite database uses Write-Ahead Logging (WAL) for concurrent read access:

### `phone_numbers`
| Column | Type | Description |
|---|---|---|
| `phone_number` | TEXT PRIMARY KEY | E.164 phone number e.g. `+15551234567` |
| `label` | TEXT | Friendly name (e.g. "US Work SIM") |
| `created_at` | TEXT | ISO 8601 creation timestamp |
| `last_seen_at` | TEXT | Timestamp of latest message received |
| `device_id` | TEXT | ID of device that ingested it |
| `is_active` | INTEGER | Active flag (1 or 0) |

### `messages`
| Column | Type | Description |
|---|---|---|
| `id` | TEXT PRIMARY KEY | UUID v4 |
| `recipient_number` | TEXT NOT NULL | The phone number receiving the SMS (FK) |
| `sender` | TEXT NOT NULL | Originating address (e.g. "Google", "Bank") |
| `body` | TEXT NOT NULL | Full text content |
| `received_at` | TEXT NOT NULL | ISO 8601 timestamp |
| `device_id` | TEXT | Ingesting device identifier |
| `sim_slot` | INTEGER | 0 for SIM 1, 1 for SIM 2 |
| `extracted_code` | TEXT | Automatically extracted 2FA/OTP code |
| `raw_metadata` | TEXT | Optional JSON metadata |
| `is_read` | INTEGER | Read flag |

### `api_tokens`
| Column | Type | Description |
|---|---|---|
| `id` | TEXT PRIMARY KEY | UUID v4 |
| `token_hash` | TEXT UNIQUE | SHA-256 hash of secret token |
| `token_prefix` | TEXT NOT NULL | Redacted prefix for identification |
| `name` | TEXT NOT NULL | Token description (e.g. "Claude AI") |
| `allowed_numbers` | TEXT NOT NULL | JSON array: `["+15551234567"]` or `["*"]` |
| `can_read` | INTEGER | Read permission |
| `can_delete` | INTEGER | Delete permission |
| `is_admin` | INTEGER | Administrative permission |
| `created_at` | TEXT | Creation date |
| `last_used_at` | TEXT | Latest access date |

### `devices`
| Column | Type | Description |
|---|---|---|
| `id` | TEXT PRIMARY KEY | Unique device ID (e.g. `dev_...`) |
| `name` | TEXT NOT NULL | Friendly name (e.g. "Pixel 8 Pro") |
| `platform` | TEXT NOT NULL | "android", "ios", or "api" |
| `token_hash` | TEXT UNIQUE | SHA-256 hash of device token |
| `created_at` | TEXT | Registration timestamp |
| `last_seen_at` | TEXT | Last ping or forward timestamp |
