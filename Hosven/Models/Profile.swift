import Foundation

/// Logical grouping of host entries. Maps to the `## [tag:Name]` markers parsed from `/etc/hosts`.
/// The `tag` string in `HostEntry.tag` links an entry to a profile by name.
///
/// An exclusive profile is an environment: switching to it turns every other environment's
/// entries off and writes the file, so at most one environment is on. A profile that is not
/// exclusive is a block, switched on and off independently of everything else.
struct Profile: Identifiable, Codable, Hashable {
    let id: UUID
    var name: String
    var color: ProfileColor
    /// Optional keyboard shortcut (1, 2, 3, …) for ⌘N quick switch.
    var shortcutNumber: Int?
    /// Environments exclude each other; blocks are switched independently.
    var isExclusive: Bool

    init(id: UUID = UUID(), name: String, color: ProfileColor, shortcutNumber: Int? = nil, isExclusive: Bool = false) {
        self.id = id
        self.name = name
        self.color = color
        self.shortcutNumber = shortcutNumber
        self.isExclusive = isExclusive
    }

    private enum CodingKeys: String, CodingKey {
        case id, name, color, shortcutNumber, isExclusive
    }

    /// Profiles saved before the flag existed become environments when their colour
    /// names one (Local, Staging, Production) and blocks otherwise.
    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        name = try container.decode(String.self, forKey: .name)
        color = try container.decode(ProfileColor.self, forKey: .color)
        shortcutNumber = try container.decodeIfPresent(Int.self, forKey: .shortcutNumber)
        isExclusive = try container.decodeIfPresent(Bool.self, forKey: .isExclusive) ?? color.isEnvironment
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
    static let release = Profile(name: "Release", color: .purple, shortcutNumber: 1, isExclusive: true)
    static let production = Profile(name: "Production", color: .red, shortcutNumber: 2, isExclusive: true)
    static let master = Profile(name: "Master", color: .amber, shortcutNumber: 3, isExclusive: true)

    static let defaults: [Profile] = [.release, .production, .master]
}
