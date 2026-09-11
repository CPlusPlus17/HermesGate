//
//  ContentView.swift
//  SMSForwarder for iOS
//

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var network: NetworkManager
    @State private var showingTestAlert = false
    @State private var alertMessage = ""
    @State private var isTesting = false

    var body: some View {
        NavigationView {
            Form {
                // Connection Status Section
                Section(header: Text("Status")) {
                    HStack {
                        Circle()
                            .fill(network.isConnected ? Color.green : Color.red)
                            .frame(width: 10, height: 10)
                        Text(network.lastSyncStatus)
                            .font(.subheadline)
                        Spacer()
                        Button(action: {
                            Task {
                                isTesting = true
                                let success = await network.checkHealth()
                                isTesting = false
                                alertMessage = success ? "Server connected!" : "Failed to connect to server."
                                showingTestAlert = true
                            }
                        }) {
                            if isTesting {
                                ProgressView()
                            } else {
                                Text("Check")
                            }
                        }
                    }
                }

                // Configuration Section
                Section(header: Text("Server & Device Settings")) {
                    HStack {
                        Text("Server URL")
                            .foregroundColor(.secondary)
                            .frame(width: 100, alignment: .leading)
                        TextField("http://192.168.1.100:8080", text: $network.serverUrl)
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                    }

                    HStack {
                        Text("Device Token")
                            .foregroundColor(.secondary)
                            .frame(width: 100, alignment: .leading)
                        SecureField("sms_dev_...", text: $network.deviceToken)
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                    }

                    HStack {
                        Text("Device Name")
                            .foregroundColor(.secondary)
                            .frame(width: 100, alignment: .leading)
                        TextField("iPhone 15 Pro", text: $network.deviceName)
                    }

                    HStack {
                        Text("Phone Number")
                            .foregroundColor(.secondary)
                            .frame(width: 100, alignment: .leading)
                        TextField("+15551234567", text: $network.phoneNumber)
                            .keyboardType(.phonePad)
                    }
                }

                // Actions Section
                Section(header: Text("Testing")) {
                    Button(action: {
                        Task {
                            let ok = await network.sendTestSms()
                            alertMessage = ok ? "Sample SMS forwarded successfully!" : "Failed to send SMS."
                            showingTestAlert = true
                        }
                    }) {
                        Label("Send Test SMS Forward", systemImage: "paperplane.fill")
                    }
                }

                // iOS Shortcuts Integration Section
                Section(header: Text("iOS Automatic SMS Forwarding")) {
                    NavigationLink(destination: ShortcutsGuideView()) {
                        Label("Apple Shortcuts Automation Guide", systemImage: "gearshape.2.fill")
                    }
                    Text("Due to iOS privacy sandboxing, automatic background SMS forwarding on iOS is handled via Apple Shortcuts Automation. Tap above for the 2-minute setup.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("HermesGate")
            .alert(isPresented: $showingTestAlert) {
                Alert(title: Text("HermesGate"), message: Text(alertMessage), dismissButton: .default(Text("OK")))
            }
        }
    }
}

struct ShortcutsGuideView: View {
    @EnvironmentObject var network: NetworkManager
    @State private var copiedUrl = false
    @State private var copiedPayload = false

    var webhookUrl: String {
        "\(network.serverUrl.trimmingCharacters(in: CharacterSet(charactersIn: "/")))/api/v1/sms/forward"
    }

    var samplePayload: String {
        """
        {
          "recipient_number": "\(network.phoneNumber.isEmpty ? "+15551234567" : network.phoneNumber)",
          "sender": "ShortcutInput.Sender",
          "body": "ShortcutInput.Content",
          "device_id": "\(network.deviceName)"
        }
        """
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("How to set up iOS SMS Automation")
                    .font(.headline)

                Text("Apple Shortcuts allows iOS to automatically trigger when any SMS message arrives and post it directly to your Rust server:")
                    .font(.subheadline)
                    .foregroundColor(.secondary)

                VStack(alignment: .leading, spacing: 12) {
                    StepRow(number: "1", text: "Open the built-in 'Shortcuts' app on your iPhone.")
                    StepRow(number: "2", text: "Tap the 'Automation' tab at the bottom, then tap '+' (New Automation).")
                    StepRow(number: "3", text: "Select 'Message' trigger -> 'Message Contains' (leave blank for all, or type 'code').")
                    StepRow(number: "4", text: "Choose 'Run Immediately' (iOS 17+) so it forwards without confirmation.")
                    StepRow(number: "5", text: "Add Action: 'Get Contents of URL'. Set Method to POST and enter your webhook URL below:")
                }

                // Webhook URL Box
                VStack(alignment: .leading, spacing: 8) {
                    Text("Webhook URL:")
                        .font(.caption)
                        .bold()
                    HStack {
                        Text(webhookUrl)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1)
                        Spacer()
                        Button(copiedUrl ? "Copied!" : "Copy") {
                            UIPasteboard.general.string = webhookUrl
                            copiedUrl = true
                            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { copiedUrl = false }
                        }
                        .font(.caption)
                    }
                    .padding(8)
                    .background(Color(UIColor.secondarySystemBackground))
                    .cornerRadius(8)
                }

                // Header / Token Box
                VStack(alignment: .leading, spacing: 8) {
                    Text("Headers to add:")
                        .font(.caption)
                        .bold()
                    Text("X-Device-Token: \(network.deviceToken)")
                        .font(.system(.caption, design: .monospaced))
                        .padding(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color(UIColor.secondarySystemBackground))
                        .cornerRadius(8)
                }

                // JSON Payload Box
                VStack(alignment: .leading, spacing: 8) {
                    Text("Request Body (JSON):")
                        .font(.caption)
                        .bold()
                    Text(samplePayload)
                        .font(.system(.caption, design: .monospaced))
                        .padding(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color(UIColor.secondarySystemBackground))
                        .cornerRadius(8)
                    Button(copiedPayload ? "Copied JSON!" : "Copy JSON Template") {
                        UIPasteboard.general.string = samplePayload
                        copiedPayload = true
                        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { copiedPayload = false }
                    }
                    .font(.caption)
                }
            }
            .padding()
        }
        .navigationTitle("Shortcuts Setup")
    }
}

struct StepRow: View {
    let number: String
    let text: String

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Text(number)
                .font(.caption)
                .bold()
                .frame(width: 20, height: 20)
                .background(Color.blue)
                .foregroundColor(.white)
                .clipShape(Circle())
            Text(text)
                .font(.subheadline)
        }
    }
}
