# Android App Guide: HermesGate

The **HermesGate Android App** is a native Kotlin application designed for reliable, 24/7 background SMS forwarding across single-SIM and dual-SIM devices.

---

## 1. Key Features

- **High-Priority BroadcastReceiver**: Intercepts incoming SMS PDUs immediately upon receipt via `SMS_RECEIVED`.
- **Multi-Part SMS Concatenation**: Reassembles long multipart text messages into a single coherent message body.
- **Dual-SIM Support**: Automatically detects which SIM slot (SIM 1 vs SIM 2) received the SMS, and reads the associated phone number.
- **Offline Resilience Queue**: Enqueues messages locally in Room (SQLite). If cellular or Wi-Fi data drops, the app automatically retries with exponential backoff using Android `WorkManager`.
- **Foreground Service**: Optional persistent foreground service with low-importance notification to prevent OEM aggressive background kills.
- **Boot Persistence**: Automatically restarts listeners when the phone restarts via `RECEIVE_BOOT_COMPLETED`.

---

## 2. Building & Installing the App

### Prerequisites
- Android Studio Hedgehog (2023.1.1) or later, OR Java 17+ with the Android SDK command-line tools.

### Building via Command Line
```bash
cd android
./gradlew assembleDebug
```
The compiled APK will be located at:
`android/app/build/outputs/apk/debug/app-debug.apk`

### Installing onto your Android Device
```bash
adb install android/app/build/outputs/apk/debug/app-debug.apk
```

---

## 3. Initial Configuration

1. Launch **HermesGate** on your phone.
2. Grant the requested permissions:
   - `Receive SMS` & `Read SMS`
   - `Phone State & Numbers` (used to detect SIM 1 / SIM 2 phone numbers)
   - `Notifications` (for foreground service status)
3. Enter your **Server Configuration**:
   - **Server URL**: Your Rust server address (e.g. `http://192.168.1.100:8080` or `https://sms.yourdomain.com`).
   - **Device Token**: The device pairing token generated from the Web UI (`sms_dev_...`).
   - **Phone Number Override**: Enter your phone number if your carrier does not write the MSISDN onto the SIM card.
4. Tap **Save Settings**.
5. Tap **Test Server**: Verifies connectivity with `/api/v1/health`.
6. Tap **Send Test SMS**: Sends a simulated forward to verify the entire pipeline.

---

## 4. Preventing Background Kills (Doze Mode & OEMs)

Modern Android versions (Android 12+) and OEM skins (Xiaomi MIUI/HyperOS, Huawei EMUI, Samsung One UI, Oppo/Vivo) implement aggressive background task termination.

To guarantee 100% reliable background SMS forwarding:

1. Tap **Battery Doze** in the app to open the Android system prompt requesting **Ignore Battery Optimization**. Tap **Allow**.
2. **Auto-start**:
   - **Samsung**: Settings -> Apps -> HermesGate -> Battery -> Unrestricted.
   - **Xiaomi / Redmi**: Settings -> Apps -> Manage Apps -> HermesGate -> Enable "Autostart" and set Battery Saver to "No restrictions".
   - **OnePlus / Oppo / Realme**: App info -> Battery usage -> Allow background activity & Allow auto-launch.
3. Keep the **Keep Background Service Running** toggle enabled in the app.
