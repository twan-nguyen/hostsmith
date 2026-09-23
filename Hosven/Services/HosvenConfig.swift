import Foundation

/// Where Hosven keeps its plain-text configuration: `$XDG_CONFIG_HOME/hosven`,
/// falling back to `~/.config/hosven`, so a developer can version it in dotfiles.
enum HosvenConfig {
    static var directory: URL {
        let env = ProcessInfo.processInfo.environment
        let base: URL
        if let xdg = env["XDG_CONFIG_HOME"], !xdg.isEmpty {
            base = URL(fileURLWithPath: xdg, isDirectory: true)
        } else {
            base = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".config", isDirectory: true)
        }
        return base.appendingPathComponent("hosven", isDirectory: true)
    }

    static var profilesURL: URL { directory.appendingPathComponent("profiles.json") }
    static var envURL: URL { directory.appendingPathComponent("env.json") }

    /// The Application Support file env config lived in before it moved here.
    static var legacyEnvURL: URL {
        FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)
            .first!
            .appendingPathComponent("com.twannguyen.hosven")
            .appendingPathComponent("env-config.json")
    }

    static func ensureDirectory(for fileURL: URL) throws {
        try FileManager.default.createDirectory(at: fileURL.deletingLastPathComponent(), withIntermediateDirectories: true)
    }
}
