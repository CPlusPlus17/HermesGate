# iOS SMS Forwarding Setup via Apple Shortcuts Automation

Because iOS sandboxes third-party applications and prevents apps from silently intercepting text messages in the background, Apple provides an official, native mechanism for background message automation: **Apple Shortcuts Personal Automations**.

In iOS 17 and later, Personal Automations run seamlessly in the background **without asking for confirmation**.

---

## Prerequisites
1. An iPhone running iOS 14+ (iOS 17+ recommended for unattended background execution).
2. The `SMS Forwarder` Rust server reachable from your iPhone (via Local Wi-Fi IP e.g. `http://192.168.1.100:8080`, Tailscale/WireGuard, or a public HTTPS URL / domain / ngrok).
3. A Device Auth Token generated from the SMS Forwarder Web UI (`sms_dev_...`).

---

## 3-Minute Step-by-Step Setup

### Step 1: Open the Shortcuts App
1. Open the built-in **Shortcuts** app on iOS.
2. Tap the **Automation** tab at the bottom of the screen.
3. Tap **New Automation** (or the **+** button in the top-right corner).

### Step 2: Choose the Message Trigger
1. Scroll down and tap **Message**.
2. Under **Message Contains**:
   - Leave it empty to trigger on **all incoming messages**, OR
   - Type keywords like `code`, `verification`, `login`, `pin` if you only want 2FA messages forwarded.
3. Under **Sender**: Leave set to **Any** (or choose specific senders if desired).
4. Select **Run Immediately** (iOS 17+).
5. Toggle OFF **Notify When Run** so it executes silently in the background.
6. Tap **Next**.

### Step 3: Add the Action to Forward to Rust Server
1. Tap **New Blank Automation**.
2. Tap **Add Action** and search for **Get Contents of URL**.
3. Configure the action:
   - **URL**: `http://YOUR_SERVER_IP:8080/api/v1/sms/forward`
   - Tap the arrow / expand icon to show advanced options:
   - **Method**: `POST`
   - **Headers**:
     - Add Header: `Content-Type` = `application/json`
     - Add Header: `X-Device-Token` = `YOUR_DEVICE_TOKEN` (e.g. `sms_dev_...`)
   - **Request Body**: Choose `JSON`
   - Add fields to the JSON body:
     - `recipient_number` (Text): Your phone number, e.g. `+15551234567`
     - `sender` (Text): Tap variable, choose **Shortcut Input**, then tap it and choose **Sender**
     - `body` (Text): Tap variable, choose **Shortcut Input**, then tap it and choose **Content**
     - `device_id` (Text): `my_iphone`

### Step 4: Test & Verify
1. Tap **Done** to save the automation.
2. Receive an SMS or send yourself a text.
3. Watch the message instantly appear on the SMS Forwarder Web UI and stream over WebSocket and MCP!

---

## Sample Request Payload Format
```json
{
  "recipient_number": "+15551234567",
  "sender": "+18005550199",
  "body": "Your security code is 492104. Valid for 5 minutes.",
  "device_id": "iPhone_15_Pro",
  "sim_slot": 0,
  "metadata": {
    "platform": "ios_shortcuts"
  }
}
```
