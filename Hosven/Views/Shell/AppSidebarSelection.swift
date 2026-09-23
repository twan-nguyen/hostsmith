import Foundation

/// What the source list points at. One value drives the whole working pane.
/// Stored in `@AppStorage` through `rawValue` so the last selection survives relaunch.
enum AppSidebarSelection: Hashable, RawRepresentable {
    case profile(UUID)
    case systemHosts
    case hostsTag(String)
    case repo(UUID)
    case envFile(repo: UUID, path: String)
    case ports

    var rawValue: String {
        switch self {
        case .profile(let id): return "profile:\(id.uuidString)"
        case .systemHosts: return "systemHosts"
        case .hostsTag(let name): return "hostsTag:\(name)"
        case .repo(let id): return "repo:\(id.uuidString)"
        case .envFile(let repo, let path): return "envFile:\(repo.uuidString):\(path)"
        case .ports: return "ports"
        }
    }

    init?(rawValue: String) {
        if rawValue == "systemHosts" { self = .systemHosts; return }
        if rawValue == "ports" { self = .ports; return }
        let parts = rawValue.split(separator: ":", maxSplits: 2, omittingEmptySubsequences: false).map(String.init)
        guard parts.count >= 2 else { return nil }
        switch parts[0] {
        case "profile":
            guard let id = UUID(uuidString: parts[1]) else { return nil }
            self = .profile(id)
        case "hostsTag":
            self = .hostsTag(parts[1])
        case "repo":
            guard let id = UUID(uuidString: parts[1]) else { return nil }
            self = .repo(id)
        case "envFile":
            guard parts.count == 3, let id = UUID(uuidString: parts[1]) else { return nil }
            self = .envFile(repo: id, path: parts[2])
        default:
            return nil
        }
    }

    /// The repo this selection belongs to, when it is an env selection.
    var repoId: UUID? {
        switch self {
        case .repo(let id), .envFile(let id, _): return id
        default: return nil
        }
    }
}
