import Foundation

/// Predefined color choices for `Profile`. Hex values match the design tokens in `design/hostess/tokens.json`.
enum ProfileColor: String, CaseIterable, Identifiable, Codable, Hashable {
    case purple
    case green
    case amber
    case blue
    case red

    var id: String { rawValue }

    /// Light-appearance hex (no `#` prefix) of the design token this colour renders as:
    /// `env-local`, `env-staging`, `env-production`, or `ink-secondary` for neutral.
    var hex: String {
        switch self {
        case .blue:   return "3f6b30"
        case .amber:  return "8a5a00"
        case .red:    return "b3311f"
        case .green, .purple: return "5e5d59"
        }
    }

    /// Design mapping: three environment hues plus neutral. `green` and
    /// `purple` remain decodable for stored profiles and render as neutral.
    var displayName: String {
        switch self {
        case .blue:   return "Local"
        case .amber:  return "Staging"
        case .red:    return "Production"
        case .green, .purple: return "Trung tính"
        }
    }

    /// True for the three named environments; neutral colours carry no meaning to show.
    var isEnvironment: Bool {
        switch self {
        case .blue, .amber, .red: return true
        case .green, .purple: return false
        }
    }

    /// Cases offered in the color picker, in the design's order.
    static let pickerCases: [ProfileColor] = [.blue, .amber, .red, .purple]
}
