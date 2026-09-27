import SwiftUI

extension Notification.Name {
    static let openAppSettings = Notification.Name("CoveOpenAppSettings")
}

@main
struct CoveApp: App {
    @UIApplicationDelegateAdaptor(CoveAppDelegate.self) private var appDelegate

    private let root = CoveApplicationRoot(dependencies: .production())

    var body: some Scene {
        WindowGroup {
            root
        }
        .commands {
            CommandMenu("Cove") {
                Button("Settings...") {
                    NotificationCenter.default.post(name: .openAppSettings, object: nil)
                }
                .keyboardShortcut(",", modifiers: .command)
            }
        }
    }
}
