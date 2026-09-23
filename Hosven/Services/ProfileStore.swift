import Foundation

/// Persists `Profile` definitions (color, shortcut, name) outside `/etc/hosts`.
/// Hosts file only stores tag-name markers; this store layers metadata on top.
protocol ProfileStoring {
    func load() -> [Profile]
    func save(_ profiles: [Profile])
}

/// JSON file under `HosvenConfig.directory`. The first load migrates the blob
/// older versions kept in UserDefaults, then the file is the only source.
final class ProfileStore: ProfileStoring {
    static let legacyDefaultsKey = "com.twannguyen.hosven.profiles.v2"

    private let fileURL: URL
    private let legacyDefaults: UserDefaults?

    init(fileURL: URL = HosvenConfig.profilesURL, legacyDefaults: UserDefaults? = .standard) {
        self.fileURL = fileURL
        self.legacyDefaults = legacyDefaults
    }

    func load() -> [Profile] {
        if let data = try? Data(contentsOf: fileURL) {
            // Corrupt file → drop it rather than crash; UI will reseed defaults.
            return (try? JSONDecoder().decode([Profile].self, from: data)) ?? []
        }
        guard let legacy = legacyDefaults?.data(forKey: Self.legacyDefaultsKey),
              let profiles = try? JSONDecoder().decode([Profile].self, from: legacy) else { return [] }
        save(profiles)
        return profiles
    }

    func save(_ profiles: [Profile]) {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(profiles) else { return }
        try? HosvenConfig.ensureDirectory(for: fileURL)
        try? data.write(to: fileURL, options: .atomic)
    }
}

/// In-memory mock for tests — does not touch disk.
final class MockProfileStore: ProfileStoring {
    private(set) var stored: [Profile]

    init(initial: [Profile] = []) {
        stored = initial
    }

    func load() -> [Profile] { stored }
    func save(_ profiles: [Profile]) { stored = profiles }
}
