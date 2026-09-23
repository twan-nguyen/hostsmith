import AppKit
import Foundation

/// Hands a folder or file to the tools a developer already has installed.
/// Detection is by bundle id, first match wins, so labels can name the app.
enum ExternalTools {
    struct App {
        let name: String
        let url: URL
    }

    private static let terminalCandidates: [(String, String)] = [
        ("iTerm", "com.googlecode.iterm2"),
        ("Ghostty", "com.mitchellh.ghostty"),
        ("Warp", "dev.warp.Warp-Stable"),
        ("Terminal", "com.apple.Terminal"),
    ]

    private static let editorCandidates: [(String, String)] = [
        ("VS Code", "com.microsoft.VSCode"),
        ("Cursor", "com.todesktop.230313mzl4w4u92"),
        ("Zed", "dev.zed.Zed"),
        ("VSCodium", "com.vscodium"),
    ]

    static var terminal: App? { first(of: terminalCandidates) }
    static var editor: App? { first(of: editorCandidates) }

    /// Opens `path` (a folder) in the terminal app. Every candidate accepts a
    /// folder as its document and starts a session there.
    static func openInTerminal(_ path: String) {
        guard let app = terminal else { return }
        open(path, with: app)
    }

    /// Opens a folder or file in the editor app.
    static func openInEditor(_ path: String) {
        guard let app = editor else { return }
        open(path, with: app)
    }

    private static let chromeBundleId = "com.google.Chrome"

    static var hasChrome: Bool {
        NSWorkspace.shared.urlForApplication(withBundleIdentifier: chromeBundleId) != nil
    }

    /// Chrome only accepts its internal pages through `open -b`, not `NSWorkspace.open(URL)`.
    static func openChromeDNSPage() {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        process.arguments = ["-b", chromeBundleId, "chrome://net-internals/#dns"]
        try? process.run()
    }

    private static func open(_ path: String, with app: App) {
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = true
        NSWorkspace.shared.open([URL(fileURLWithPath: path)], withApplicationAt: app.url, configuration: configuration)
    }

    private static func first(of candidates: [(String, String)]) -> App? {
        for (name, bundleId) in candidates {
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleId) {
                return App(name: name, url: url)
            }
        }
        return nil
    }
}
