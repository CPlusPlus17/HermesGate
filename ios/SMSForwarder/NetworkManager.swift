//
//  NetworkManager.swift
//  SMSForwarder for iOS
//

import Foundation
import Combine

class NetworkManager: ObservableObject {
    static let shared = NetworkManager()

    @Published var serverUrl: String {
        didSet { UserDefaults.standard.set(serverUrl, forKey: "serverUrl") }
    }

    @Published var deviceToken: String {
        didSet { UserDefaults.standard.set(deviceToken, forKey: "deviceToken") }
    }

    @Published var deviceName: String {
        didSet { UserDefaults.standard.set(deviceName, forKey: "deviceName") }
    }

    @Published var phoneNumber: String {
        didSet { UserDefaults.standard.set(phoneNumber, forKey: "phoneNumber") }
    }

    @Published var isConnected: Bool = false
    @Published var lastSyncStatus: String = "Ready"
    @Published var serverVersion: String = ""

    private init() {
        self.serverUrl = UserDefaults.standard.string(forKey: "serverUrl") ?? "http://192.168.1.100:8080"
        self.deviceToken = UserDefaults.standard.string(forKey: "deviceToken") ?? ""
        self.deviceName = UserDefaults.standard.string(forKey: "deviceName") ?? "iPhone"
        self.phoneNumber = UserDefaults.standard.string(forKey: "phoneNumber") ?? ""
    }

    func checkHealth() async -> Bool {
        guard let url = URL(string: "\(serverUrl.trimmingCharacters(in: CharacterSet(charactersIn: "/")))/api/v1/health") else {
            DispatchQueue.main.async {
                self.lastSyncStatus = "Invalid URL"
                self.isConnected = false
            }
            return false
        }

        do {
            let (data, response) = try await URLSession.shared.data(from: url)
            guard let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 else {
                DispatchQueue.main.async {
                    self.lastSyncStatus = "Server error"
                    self.isConnected = false
                }
                return false
            }

            if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let status = json["status"] as? String, status == "ok" {
                let version = json["version"] as? String ?? "0.1.0"
                DispatchQueue.main.async {
                    self.isConnected = true
                    self.serverVersion = version
                    self.lastSyncStatus = "Connected (v\(version))"
                }
                return true
            }
        } catch {
            DispatchQueue.main.async {
                self.lastSyncStatus = "Connection failed: \(error.localizedDescription)"
                self.isConnected = false
            }
        }
        return false
    }

    func sendTestSms(sender: String = "TestSender", body: String = "Your verification code is 654321") async -> Bool {
        guard let url = URL(string: "\(serverUrl.trimmingCharacters(in: CharacterSet(charactersIn: "/")))/api/v1/sms/forward") else {
            return false
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        if !deviceToken.isEmpty {
            request.setValue(deviceToken, forHTTPHeaderField: "X-Device-Token")
            request.setValue("Bearer \(deviceToken)", forHTTPHeaderField: "Authorization")
        }

        let payload: [String: Any] = [
            "recipient_number": phoneNumber.isEmpty ? "+15551234567" : phoneNumber,
            "sender": sender,
            "body": body,
            "device_id": deviceName,
            "sim_slot": 0,
            "metadata": ["platform": "ios_app"]
        ]

        guard let bodyData = try? JSONSerialization.data(withJSONObject: payload) else {
            return false
        }
        request.httpBody = bodyData

        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            if let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 201 {
                DispatchQueue.main.async {
                    self.lastSyncStatus = "Test SMS sent successfully!"
                }
                return true
            } else {
                let errStr = String(data: data, encoding: .utf8) ?? "Unknown error"
                DispatchQueue.main.async {
                    self.lastSyncStatus = "Failed: \(errStr)"
                }
            }
        } catch {
            DispatchQueue.main.async {
                self.lastSyncStatus = "Send error: \(error.localizedDescription)"
            }
        }

        return false
    }
}
