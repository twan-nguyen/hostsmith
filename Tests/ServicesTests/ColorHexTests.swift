import Testing
import SwiftUI
@testable import Hosven

@Suite("Profile colour bridge")
struct ProfileColorBridgeTests {
    /// Pull RGBA out of the SwiftUI `Color` for round-trip checks.
    private func rgba(_ color: Color) -> (r: Double, g: Double, b: Double, a: Double) {
        let ns = NSColor(color).usingColorSpace(.sRGB)!
        return (ns.redComponent, ns.greenComponent, ns.blueComponent, ns.alphaComponent)
    }

    @Test("Color.ds(profileColor) maps to the fixed environment tokens")
    func dsBridgeReturnsTokens() {
        let pairs: [(ProfileColor, Color)] = [
            (.blue,   DS.Colors.envLocal),
            (.amber,  DS.Colors.envStaging),
            (.red,    DS.Colors.envProduction),
            (.green,  DS.Colors.inkSecondary),
            (.purple, DS.Colors.inkSecondary),
        ]
        for (pc, token) in pairs {
            let (r, g, b, _) = rgba(Color.ds(pc))
            let (er, eg, eb, _) = rgba(token)
            #expect(abs(r - er) < 0.01)
            #expect(abs(g - eg) < 0.01)
            #expect(abs(b - eb) < 0.01)
        }
    }

    @Test("Picker offers the three environment hues plus neutral")
    func pickerCases() {
        #expect(ProfileColor.pickerCases == [.blue, .amber, .red, .purple])
    }
}
