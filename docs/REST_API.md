# REST API Reference: HermesGate

The REST API allows external systems, CI/CD pipelines, mobile devices, and automation scripts to interact with HermesGate.

All authenticated requests require a Bearer token in the `Authorization` header:
`Authorization: Bearer <TOKEN>` or `X-API-Token: <TOKEN>` or `?token=<TOKEN>`.

Mobile devices send `X-Device-Token: <DEVICE_TOKEN>` to the forward endpoint.

---

## 1. Device Ingestion Endpoints

### `POST /api/v1/sms/forward`
Receives an incoming SMS forwarded by an Android phone or iOS Shortcut.

**Headers**:
- `Content-Type: application/json`
- `X-Device-Token: sms_dev_...` (or Master Admin Bearer token)

**Request Body**:
```json
{
  "recipient_number": "+15551234567",
  "sender": "BankOne",
  "body": "Your authorization code is 918230. Valid for 5 minutes.",
  "sim_slot": 0,
  "device_id": "pixel8_home",
  "received_at": "2026-09-11T21:30:00Z",
  "metadata": { "carrier": "Verizon" }
}
```

**Response** (`201 Created`):
```json
{
  "success": true,
  "message_id": "a50c8227-bb19-4db8-b570-fd1e22080a29",
  "recipient_number": "+15551234567",
  "extracted_code": "918230",
  "received_at": "2026-09-11T21:30:00Z"
}
```

---

## 2. Automation & Consumption Endpoints

### `GET /api/v1/messages/latest-otp`
Retrieves the most recent OTP code for an authorized number.

**Query Parameters**:
- `number`: Optional recipient phone number. Must be authorized for this token.
- `max_age_minutes`: Optional max age in minutes (default `10`).

**cURL Example**:
```bash
curl -H "Authorization: Bearer sms_tok_..." \
  "http://localhost:8080/api/v1/messages/latest-otp?number=%2B15551234567"
```

**Response** (`200 OK`):
```json
{
  "found": true,
  "otp": {
    "code": "918230",
    "message_id": "a50c8227-bb19-4db8-b570-fd1e22080a29",
    "recipient_number": "+15551234567",
    "sender": "BankOne",
    "body": "Your authorization code is 918230. Valid for 5 minutes.",
    "received_at": "2026-09-11T21:30:00Z"
  }
}
```

---

### `GET /api/v1/messages`
Queries incoming SMS messages, strictly filtered by the token's allowed phone numbers.

**Query Parameters**:
- `number`: Filter by recipient number (returns 403 Forbidden if not permitted).
- `sender`: Substring search on sender.
- `search`: Substring search in message body and sender.
- `limit`: Integer limit (default 50, max 500).
- `offset`: Pagination offset.
- `since`: ISO 8601 timestamp cutoff.

**cURL Example**:
```bash
curl -H "Authorization: Bearer sms_tok_..." \
  "http://localhost:8080/api/v1/messages?limit=10"
```

**Response** (`200 OK`):
```json
{
  "messages": [
    {
      "id": "a50c8227-bb19-4db8-b570-fd1e22080a29",
      "recipient_number": "+15551234567",
      "sender": "BankOne",
      "body": "Your authorization code is 918230. Valid for 5 minutes.",
      "received_at": "2026-09-11T21:30:00Z",
      "device_id": "pixel8_home",
      "sim_slot": 0,
      "extracted_code": "918230",
      "raw_metadata": "{\"carrier\":\"Verizon\"}",
      "is_read": false
    }
  ],
  "count": 1
}
```

---

### `GET /api/v1/numbers`
Lists phone numbers the authenticated token has permission to access.

**Response** (`200 OK`):
```json
{
  "phone_numbers": [
    {
      "phone_number": "+15551234567",
      "label": "US Work Line (SIM 1)",
      "created_at": "2026-09-11T20:00:00Z",
      "last_seen_at": "2026-09-11T21:30:00Z",
      "device_id": "pixel8_home",
      "is_active": true,
      "message_count": 42
    }
  ],
  "count": 1
}
```

---

### `DELETE /api/v1/messages/{id}`
Deletes a message. Requires `can_delete` permission and recipient number authorization.

**Response** (`200 OK`):
```json
{ "success": true, "message": "Message deleted" }
```

---

## 3. Real-Time Streaming

### `GET /api/v1/ws?token=<TOKEN>`
Upgrades HTTP connection to WebSocket.
Receives real-time JSON events whenever an SMS is forwarded to any number the token is authorized to access:
```json
{
  "type": "new_sms",
  "message": {
    "id": "...",
    "recipient_number": "+15551234567",
    "sender": "Google",
    "body": "G-551920 is your verification code.",
    "extracted_code": "G-551920",
    "received_at": "..."
  }
}
```

---

## 4. Admin Management Endpoints

*(Requires Admin Token `is_admin: true`)*

- `POST /api/v1/admin/tokens`: Create new API / MCP token with specific numbers.
- `GET /api/v1/admin/tokens`: List all tokens.
- `DELETE /api/v1/admin/tokens/{id}`: Revoke a token.
- `POST /api/v1/admin/devices`: Register new device and generate pairing token + QR payload.
- `GET /api/v1/admin/devices`: List registered devices.
- `POST /api/v1/admin/numbers`: Add or update phone number label.
