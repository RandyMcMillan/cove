//
//  CoveAppDelegate.swift
//  Cove
//
//  Created by Praveen Perera on 11/18/25.
//

import UIKit

final class CoveAppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _: UIApplication,
        configurationForConnecting connectingSceneSession: UISceneSession,
        options _: UIScene.ConnectionOptions
    ) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(
            name: nil,
            sessionRole: connectingSceneSession.role
        )
        configuration.delegateClass = CovePopupSceneDelegate.self
        return configuration
    }

    @available(iOS 16.0, *)
    override func buildMenu(with builder: UIMenuBuilder) {
        super.buildMenu(with: builder)

        guard builder.system == .main else { return }

        let helpCommand = UICommand(
            title: "Cove Help",
            action: #selector(openHelp),
            input: "?",
            modifierFlags: .command
        )

        let helpMenu = UIMenu(
            title: "Help",
            image: nil,
            identifier: .help,
            options: .displayAsInline,
            children: [helpCommand]
        )

        builder.insertSibling(helpMenu, afterMenu: .about)
    }

    @objc private func openHelp() {
        if let url = URL(string: "https://covebitcoinwallet.com/support") {
            UIApplication.shared.open(url)
        }
    }
}
