import AppKit
import Foundation

/// External links reachable from the app menu.
enum AppLinks {
    static let homepage = URL(string: "https://github.com/twan-nguyen/hosven")!

    static func openHomepage() {
        NSWorkspace.shared.open(homepage)
    }

    static func openFeedback() {
        let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "?"
        let build = Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "?"
        var components = URLComponents(string: "https://github.com/twan-nguyen/hosven/issues/new")
        components?.queryItems = [
            URLQueryItem(name: "title", value: "Feedback: "),
            URLQueryItem(name: "body", value: """
            ## Feedback


            ---
            Hosven \(version) (\(build))
            \(ProcessInfo.processInfo.operatingSystemVersionString)
            """),
        ]
        if let url = components?.url { NSWorkspace.shared.open(url) }
    }
}
