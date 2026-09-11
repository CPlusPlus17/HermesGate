# Model Context Protocol (MCP) Setup Guide

The **Model Context Protocol (MCP)** interface exposes high-precision tools and resources to LLMs and AI agents (such as Claude Desktop, Google Antigravity, Cursor, and custom agentic workflows).

---

## 1. Why Per-Number Permissions Matter for AI

When deploying an AI agent with access to SMS (especially for automated login and 2FA), you usually **do not want the agent to see all personal or corporate text messages**.

With HermesGate:
- An AI token can be locked to **one specific phone number** (e.g. `+15551234567` for AWS/GitHub 2FA).
- If the agent calls `get_latest_otp` or `get_latest_messages` for another number (e.g. your personal SIM `+41791234567`), the server **strictly returns a Permission Denied error**.
- The AI can only discover numbers authorized specifically for it.

---

## 2. Integration Modes

### Mode A: Stdio Mode (Claude Desktop & Cursor)
The Rust server binary can be invoked directly in STDIO mode without needing an HTTP connection:

#### Claude Desktop Configuration
File location:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

Add the server:
```json
{
  "mcpServers": {
    "hermesgate": {
      "command": "/path/to/hermesgate",
      "args": [
        "--mcp-stdio",
        "--token", "sms_tok_YOUR_GENERATED_TOKEN",
        "--db-path", "/path/to/sms_forwarder.db"
      ]
    }
  }
}
```

---

### Mode B: Remote SSE Stream (Antigravity CLI / Web / Remote Agents)
For remote AI hosts connecting across the local network or internet:

- **SSE Endpoint**: `http://<SERVER_HOST>:8080/mcp/sse?token=<API_TOKEN>`
- **Messages Endpoint**: `http://<SERVER_HOST>:8080/mcp/messages?session_id=<ID>&token=<API_TOKEN>`

When configuring an MCP SSE client, simply supply the SSE URL with your token query parameter.

---

## 3. Available MCP Tools

### `get_latest_otp`
Fetches the most recent verification / 2FA code received on an authorized number.
```json
{
  "phone_number": "+15551234567", // optional if token has access to only 1 number
  "max_age_minutes": 10           // optional, default 10
}
```
**Example Response**:
```json
{
  "found": true,
  "otp_code": "849201",
  "recipient_number": "+15551234567",
  "sender": "Google",
  "received_at": "2026-09-11T21:30:00Z",
  "full_message": "G-849201 is your Google verification code."
}
```

### `list_allowed_numbers`
Lists only the numbers this AI token has permission to access.
```json
{}
```

### `get_latest_messages`
Retrieves recent SMS messages for authorized numbers.
```json
{
  "phone_number": "+15551234567",
  "sender": "Google",
  "limit": 10
}
```

### `search_messages`
Searches received SMS by text query or sender.
```json
{
  "query": "verification",
  "phone_number": "+15551234567"
}
```

### `get_message_details`
Retrieves full metadata of a specific message by its UUID.
```json
{
  "message_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d"
}
```

---

## 4. MCP Resources

- `sms://numbers`: JSON listing of all phone numbers authorized for the token.
- `sms://messages/{number}`: Live message feed for a specific phone number.
