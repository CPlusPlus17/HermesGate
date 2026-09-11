# iOS Client & Shortcuts Guide: HermesGate

Due to Apple's security and privacy sandboxing, third-party iOS apps cannot silently inspect incoming SMS messages in the background. However, iOS provides two first-class paths for seamless SMS forwarding.

---

## Path 1: Apple Shortcuts Personal Automation (Recommended)

In iOS 17 and later, **Personal Automations** run unattended in the background without user intervention.

### How it works:
1. iPhone receives an SMS.
2. iOS triggers your Personal Automation.
3. The Shortcut extracts the sender, message text, and timestamp.
4. The Shortcut issues an HTTP POST to `https://<YOUR_SERVER>/api/v1/sms/forward` with your `X-Device-Token`.
5. The Rust server ingests the message, extracts any 2FA/OTP code, and broadcasts it to the Web UI, WebSocket, and MCP AI agents!

*See [SMS_Forwarder_Shortcut_Guide.md](../ios/Shortcuts/SMS_Forwarder_Shortcut_Guide.md) for full step-by-step screenshots and instructions.*

---

## Path 2: Swift Companion App (`ios/SMSForwarder/`)

The repository includes a ready-to-run Xcode SwiftUI project in `ios/SMSForwarder/`:

- **Device Pairing & Health Check**: Tests connection to the Rust backend and validates the device token.
- **Shortcuts Configuration Assistant**: Generates the exact webhook URL and JSON schema formatted for your specific device and phone number with 1-click copy buttons.
- **Test Ingestion**: Lets you simulate and verify SMS forwarding directly from iOS.

### Opening in Xcode:
1. Open Xcode on macOS.
2. Open the `ios/SMSForwarder` directory.
3. Select your connected iPhone or an iOS Simulator and click **Run** (`Cmd + R`).
