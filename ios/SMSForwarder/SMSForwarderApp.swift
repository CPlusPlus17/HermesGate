//
//  SMSForwarderApp.swift
//  SMSForwarder for iOS
//

import SwiftUI

@main
struct SMSForwarderApp: App {
    @StateObject private var networkManager = NetworkManager.shared

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(networkManager)
        }
    }
}
