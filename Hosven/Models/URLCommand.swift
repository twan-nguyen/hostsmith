import Foundation

/// Commands other tools send through the `hosven://` scheme. Names are matched
/// case-insensitively; segments are percent-decoded by `URLComponents`.
///
/// - `hosven://activate/<profile>`: activate a hosts profile and apply.
/// - `hosven://deactivate`: clear the active hosts profile.
/// - `hosven://env/<repo>/profile/<name>`: apply a saved env profile to a repo.
/// - `hosven://open/<repo>`: show a repo.
/// - `hosven://ports`: show the ports pane.
enum URLCommand: Equatable {
    case activate(profile: String)
    case deactivate
    case applyEnvProfile(repo: String, profile: String)
    case open(repo: String)
    case ports

    static let scheme = "hosven"

    init?(url: URL) {
        guard url.scheme?.lowercased() == Self.scheme,
              let components = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return nil }
        var segments = [components.host ?? ""] + components.path.split(separator: "/").map(String.init)
        segments = segments.filter { !$0.isEmpty }.map { $0.removingPercentEncoding ?? $0 }
        guard let verb = segments.first?.lowercased() else { return nil }
        let rest = Array(segments.dropFirst())
        switch (verb, rest.count) {
        case ("activate", 1): self = .activate(profile: rest[0])
        case ("deactivate", 0): self = .deactivate
        case ("env", 3) where rest[1].lowercased() == "profile": self = .applyEnvProfile(repo: rest[0], profile: rest[2])
        case ("open", 1): self = .open(repo: rest[0])
        case ("ports", 0): self = .ports
        default: return nil
        }
    }
}
