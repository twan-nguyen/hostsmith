import Foundation

/// One taken TCP port. Spec: `components/PortRow/README.md`.
struct ListeningPort: Identifiable, Equatable {
    enum Owner: Equatable {
        /// Started from a repo Hosven knows; carries the repo name.
        case yours(repo: String)
        /// Published by Docker; carries the container name when known.
        case container(name: String?)
        case unknown
    }

    let port: Int
    let process: String
    let pid: Int32
    let user: String
    let cwd: String?
    let owner: Owner
    /// macOS daemons and helpers (ControlCenter, rapportd, sharingd…): listed under
    /// the Hệ thống filter only and never warned about.
    var isSystem: Bool = false

    var id: String { "\(port)-\(pid)" }

    /// Ports a dev server would pick: above the privileged range and below the
    /// ephemeral range macOS hands to background helpers.
    static func isDevPort(_ port: Int) -> Bool { (1024...49151).contains(port) }

    /// An unknown process on a dev port is the one case worth a banner.
    var needsAttention: Bool { owner == .unknown && !isSystem && Self.isDevPort(port) }

    var ownerLabel: String {
        switch owner {
        case .yours: return "Của bạn"
        case .container: return "Container"
        case .unknown: return "Không rõ"
        }
    }
}
