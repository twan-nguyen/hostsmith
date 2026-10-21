import Foundation
import SwiftUI

/// Section of palette results — one per `CommandCategory` with at least one match.
struct CommandSection: Identifiable {
    let category: CommandCategory
    let items: [PaletteCommand]
    var id: String { category.rawValue }
}

/// Drives the ⌘K palette: keeps query state, regenerates the command universe
/// from current managers, fuzzy-matches it, groups by category.
@Observable
@MainActor
final class CommandPaletteViewModel {

    // MARK: - User-facing state

    var query: String = "" {
        didSet { rebuildResults() }
    }

    private(set) var sections: [CommandSection] = []
    private(set) var flat: [PaletteCommand] = []
    var selectedIndex: Int = 0

    /// Cap result count per section so a 1000-entry hosts file doesn't drown
    /// the palette in matches. 8 is enough to spot the right one.
    private static let maxPerSection = 8

    // MARK: - Sources

    // Strong refs — managers are app-lifetime, deallocation isn't a concern.
    private var hostsManager: HostsFileManager?
    private var envManager: EnvFileManager?

    func bind(hosts: HostsFileManager, env: EnvFileManager) {
        self.hostsManager = hosts
        self.envManager = env
        rebuildResults()
    }

    func reset() {
        query = ""
        selectedIndex = 0
        rebuildResults()
    }

    // MARK: - Selection

    func moveSelection(by delta: Int) {
        guard !flat.isEmpty else { return }
        let newIndex = (selectedIndex + delta + flat.count) % flat.count
        selectedIndex = newIndex
    }

    var selectedCommand: PaletteCommand? {
        guard flat.indices.contains(selectedIndex) else { return nil }
        return flat[selectedIndex]
    }

    // MARK: - Building results

    private func rebuildResults() {
        let universe = buildUniverse()
        let trimmed = query.trimmingCharacters(in: .whitespaces)
        let scored: [(PaletteCommand, Double)]

        if trimmed.isEmpty {
            // Show top profile actions + tab suggestions when query is empty.
            scored = universe.compactMap { cmd in
                switch cmd.category {
                case .profileActions, .suggestions: return (cmd, 0)
                default: return nil
                }
            }
        } else {
            var matched = universe.compactMap { cmd -> (PaletteCommand, Double)? in
                let titleMatch = FuzzySearch.match(query: trimmed, in: cmd.title)
                let subMatch = cmd.subtitle.flatMap { FuzzySearch.match(query: trimmed, in: $0) }
                let best = max(titleMatch?.score ?? -.infinity, (subMatch?.score ?? -.infinity) * 0.6)
                guard best > -.infinity else { return nil }
                return (cmd, best)
            }
            // Always offer "search in Hosts/Env" with the raw query — score 0 so
            // exact-name matches still rank above generic search jumps.
            matched.append((SearchInHostsCommand(query: trimmed), 0))
            matched.append((SearchInEnvCommand(query: trimmed), 0))
            scored = matched
        }

        // Group by category, sort within each group by score desc.
        var byCategory: [CommandCategory: [(PaletteCommand, Double)]] = [:]
        for entry in scored {
            byCategory[entry.0.category, default: []].append(entry)
        }

        var built: [CommandSection] = []
        for category in CommandCategory.displayOrder {
            guard let items = byCategory[category], !items.isEmpty else { continue }
            let sorted = items.sorted { $0.1 > $1.1 }.prefix(Self.maxPerSection).map(\.0)
            built.append(CommandSection(category: category, items: Array(sorted)))
        }
        sections = built
        flat = built.flatMap(\.items)
        if selectedIndex >= flat.count { selectedIndex = 0 }
    }

    // MARK: - Universe

    /// Generate every command currently available. Cheap to recompute — called
    /// once per query change and palette open. ~few hundred items at most.
    private func buildUniverse() -> [PaletteCommand] {
        var commands: [PaletteCommand] = []

        commands.append(OpenSelectionCommand(selection: .systemHosts, title: "Mở Hosts hệ thống", subtitle: "/etc/hosts", icon: "network"))
        commands.append(OpenSelectionCommand(selection: .ports, title: "Mở Ports", subtitle: "Cổng đang bị chiếm", icon: "bolt.horizontal.circle"))
        if let env = envManager {
            for repo in env.repos {
                commands.append(OpenSelectionCommand(selection: .repo(repo.id), title: "Mở repo \(repo.name)", subtitle: repo.path, icon: "folder"))
            }
        }

        if let hosts = hostsManager {
            let active = hosts.activeProfileID
            for profile in hosts.environmentProfiles where hosts.tagEntryCount(name: profile.name) > 0 {
                commands.append(SwitchProfileCommand(profile: profile, isActive: active == profile.id))
            }
            if hosts.activeProfileState != .none {
                commands.append(ClearProfileCommand())
            }
        }

        return commands
    }
}
