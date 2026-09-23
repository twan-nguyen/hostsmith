import SwiftUI

// MARK: - ProfileColor → Color bridge

extension Color {
    /// Resolve a `ProfileColor` to its design token. The design fixes three
    /// environment hues; every other stored color renders as the neutral chip ink.
    static func ds(_ profileColor: ProfileColor) -> Color {
        switch profileColor {
        case .blue:  return DS.Colors.envLocal
        case .amber: return DS.Colors.envStaging
        case .red:   return DS.Colors.envProduction
        case .green, .purple: return DS.Colors.inkSecondary
        }
    }

    /// Tinted ground for a chip carrying `profileColor`.
    static func dsTint(_ profileColor: ProfileColor) -> Color {
        switch profileColor {
        case .blue:  return DS.Colors.successTint
        case .amber: return DS.Colors.warningTint
        case .red:   return DS.Colors.dangerTint
        case .green, .purple: return DS.Colors.surfaceSunken
        }
    }
}
