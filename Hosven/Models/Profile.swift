import Foundation

/// Logical grouping of host entries. Maps to the `## [tag:Name]` markers parsed from `/etc/hosts`.
///
/// In v2, exactly one profile is "active" at a time. Activating a profile enables only its host entries
/// (others stay in the file but get commented out). The `tag` string in `HostEntry.tag` links an entry to
/// a profile by name.
struct Profile: Identifiable, Codable, Hashable {
    let id: UUID
    var name: String
    var color: ProfileColor
    /// Optional keyboard shortcut (1, 2, 3, …) for ⌘N quick switch.
    var shortcutNumber: Int?

    init(id: UUID = UUID(), name: String, color: ProfileColor, shortcutNumber: Int? = nil) {
        self.id = id
        self.name = name
        self.color = color
        self.shortcutNumber = shortcutNumber
    }

    /// Validates that `name` is non-empty and contains no characters that would break the
    /// `## [tag:Name]` parser (square brackets, newlines).
    var isNameValid: Bool {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty { return false }
        return !trimmed.contains(where: { "[]\n\r".contains($0) })
    }
}

extension Profile {
    /// Built-in defaults shown on first launch.
    static let release = Profile(name: "Release", color: .purple, shortcutNumber: 1)
    static let production = Profile(name: "Production", color: .red, shortcutNumber: 2)
    static let master = Profile(name: "Master", color: .amber, shortcutNumber: 3)

    static let defaults: [Profile] = [.release, .production, .master]
}
