# HermesGate

> **Multi-Device SMS Ingestion, Web UI Dashboard, Model Context Protocol (MCP) AI Interface, and REST Automation Engine.**

HermesGate connects your physical mobile phones (Android and iOS) to a high-performance Rust server. Incoming SMS messages (especially 2-factor authentication / OTP codes) are instantly captured, parsed, and made accessible to:
- **Humans**: Through a modern, responsive embedded Web UI with live WebSocket streaming, multi-number filtering, and 1-click OTP copying.
- **AI Agents**: Through a **Model Context Protocol (MCP)** server supporting both STDIO mode (Claude Desktop, Antigravity, Cursor) and Server-Sent Events (SSE).
- **Automation**: Through a secure, token-authenticated **REST API** with granular per-number permissions.

---

## 🌟 Key Features

- 📱 **Multi-Phone-Number & Multi-Device Support**: Receive and filter SMS from multiple SIM cards and multiple Android/iOS devices in one unified place.
- 🔐 **Granular Per-Number Authorization**: Restrict AI agents (MCP) or automation tokens (REST) so they only have access to designated phone numbers (e.g. Work 2FA line vs Personal SIM).
- ⚡ **Auto 2FA / OTP Extraction**: High-precision regex engine automatically detects and extracts verification codes (Google, Apple, WhatsApp, Banks, etc.).
- 🌐 **Embedded Web UI**: Single-binary deployment with zero external dependencies. The responsive SPA dashboard is compiled directly into the Rust binary!
- 🤖 **Model Context Protocol (MCP) Built-in**:
  - `list_allowed_numbers`: Discovers authorized phone numbers.
  - `get_latest_otp`: Fetches the latest 2FA verification code received within the last N minutes.
  - `get_latest_messages`: Retrieves recent SMS messages with filters.
  - `search_messages`: Full-text search across message content and senders.
- 📲 **Native Android App**: Full Kotlin project featuring high-priority `SMS_RECEIVED` BroadcastReceiver, multipart SMS concatenation, multi-SIM slot detection, offline SQLite queue, and Android `WorkManager` retry sync.
- 🍎 **iOS Ready**: Complete Swift companion app and Apple Shortcuts Personal Automation workflow for automated, unattended SMS forwarding in iOS 17+.

---

## 📁 Repository Structure

```
SMSForwarder/
├── server/                     # Rust Backend Server
│   ├── src/
│   │   ├── api/                # REST endpoints, WebSockets, State
│   │   ├── auth/               # Bearer auth & per-number permission checks
│   │   ├── config.rs           # CLI parser (clap)
│   │   ├── db/                 # SQLite WAL storage & queries
│   │   ├── mcp/                # Model Context Protocol (tools, sse, stdio)
│   │   ├── otp.rs              # 2FA/OTP code regex extractor
│   │   ├── ui/                 # Embedded Web UI assets
│   │   └── main.rs             # Application entrypoint
│   └── tests/                  # REST & MCP integration test suite
├── charts/                     # Kubernetes Helm Chart
│   └── hermesgate/          # Production Helm Chart (Deployment, Service, Ingress, PVC, Secret)
├── android/                    # Native Android App (Kotlin)
│   ├── app/src/main/           # BroadcastReceiver, Room DB, WorkManager, UI
│   ├── build.gradle
│   └── gradlew
├── ios/                        # iOS Client & Shortcuts Integration
│   ├── SMSForwarder/           # SwiftUI Companion App
│   └── Shortcuts/              # Apple Shortcuts setup guide and JSON schema
├── docs/                       # Technical Documentation
│   ├── ARCHITECTURE.md         # Full architecture and data flow
│   ├── HELM_GUIDE.md           # Kubernetes & Helm deployment guide
│   ├── MCP_SETUP.md            # Claude Desktop, Antigravity, and AI setup
│   ├── REST_API.md             # REST API reference with cURL examples
│   ├── ANDROID_GUIDE.md        # Android build, install, and Doze mode setup
│   └── IOS_GUIDE.md            # iOS setup & Shortcuts automation
├── scripts/                    # Testing & Demo Utility Scripts
│   ├── seed_demo_data.sh       # Seeds sample numbers, messages, and tokens
│   ├── test_forward.sh         # Simulates mobile phone SMS forward
│   └── test_mcp.sh             # Tests MCP JSON-RPC protocol
├── Dockerfile                  # Multi-stage container build
├── Cargo.toml                  # Cargo workspace manifest
└── README.md                   # This guide
```

---

## 🚀 Quick Start (Rust Server)

### 1. Build and Run the Server
```bash
cargo run --bin hermesgate
```
By default, the server listens on `http://0.0.0.0:8080` and creates an SQLite database at `sms_forwarder.db`.

On first startup, the console displays your **Master Admin Token**:
```text
🔑 Initialized Master Admin Token: sms_adm_7b8a1c94e...
🌐 Web UI Dashboard:   http://localhost:8080
🤖 MCP SSE Endpoint:    http://localhost:8080/mcp/sse?token=<TOKEN>
📡 REST API Endpoint:   http://localhost:8080/api/v1/sms/forward
```

### 2. Access the Web Dashboard
Open [http://localhost:8080](http://localhost:8080) in your browser:
- Enter your Master Admin Token to unlock admin features.
- Watch live SMS messages arrive in real time over WebSockets.
- Filter by individual phone numbers or view all streams.
- Generate API/MCP tokens with granular per-number permissions.
- Pair Android/iOS devices using QR codes.

### 3. Seed Demo Data & Test Immediately
In another terminal, run:
```bash
./scripts/seed_demo_data.sh http://localhost:8080 <YOUR_ADMIN_TOKEN>
```
This populates the server with 3 phone numbers (`+1...`, `+41...`, `+44...`), sample 2FA codes, devices, and AI tokens.

---

## ☸️ Deploying with Helm (Kubernetes)

Deploy HermesGate with persistent storage, automated health probes, and optional Ingress:

```bash
# 1. Build & push container
docker build -t your-registry/hermesgate:0.1.0 .
docker push your-registry/hermesgate:0.1.0

# 2. Deploy using Helm
helm install hermesgate ./charts/hermesgate \
  --namespace hermesgate \
  --create-namespace \
  --set image.repository=your-registry/hermesgate \
  --set image.tag=0.1.0
```

*See [docs/HELM_GUIDE.md](docs/HELM_GUIDE.md) for full configuration options (Ingress, TLS, PVC, secrets).*

---

## 🤖 Connecting AI Agents (Claude / Antigravity / Cursor)

### Claude Desktop (Stdio Mode)
Add to your `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "hermesgate": {
      "command": "/path/to/hermesgate",
      "args": [
        "--mcp-stdio",
        "--token", "sms_tok_YOUR_GENERATED_TOKEN",
        "--db-path", "sms_forwarder.db"
      ]
    }
  }
}
```

### Remote AI Agents (SSE Mode)
Connect via Server-Sent Events:
- **SSE URL**: `http://<SERVER_HOST>:8080/mcp/sse?token=<YOUR_TOKEN>`
- The AI will automatically have access to `get_latest_otp`, `list_allowed_numbers`, `get_latest_messages`, and `search_messages` strictly scoped to the numbers you authorized!

---

## 📲 Setting up Mobile Forwarding

### Android
1. Open the `android/` project in Android Studio or compile with `./gradlew assembleDebug`.
2. Install the APK on your device.
3. Open the app, grant SMS and phone state permissions, and enter your Server URL and Device Token (or scan the QR code from the Web UI).
4. Tap **Test Server** and **Battery Doze** to ensure continuous background forwarding.

*See [docs/ANDROID_GUIDE.md](docs/ANDROID_GUIDE.md) for details.*

### iOS
1. Open the built-in **Shortcuts** app on iOS.
2. Create a new **Personal Automation** with the **Message** trigger.
3. Add a **Get Contents of URL** action pointing to `http://<SERVER>:8080/api/v1/sms/forward` with your `X-Device-Token` header.
4. Set to **Run Immediately** (iOS 17+) for silent, background execution.

*See [docs/IOS_GUIDE.md](docs/IOS_GUIDE.md) and [ios/Shortcuts/SMS_Forwarder_Shortcut_Guide.md](ios/Shortcuts/SMS_Forwarder_Shortcut_Guide.md) for details.*

---

## 🧪 Running Tests

```bash
cargo test --workspace
```
All unit tests and end-to-end integration tests (database isolation, per-number permission enforcement, REST API, OTP regex parsing, and MCP JSON-RPC tools) run automatically.
