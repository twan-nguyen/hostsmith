import Testing
import Foundation
@testable import Hosven

@Suite("Profile model")
struct ProfileTests {
    @Test("Profile encodes and decodes round-trip")
    func codableRoundTrip() throws {
        let original = Profile(name: "Release", color: .purple, shortcutNumber: 1)
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(Profile.self, from: data)
        #expect(decoded == original)
    }

    @Test("ProfileColor hex matches design tokens")
    func colorHexMapping() {
        #expect(ProfileColor.blue.hex == "3f6b30")
        #expect(ProfileColor.amber.hex == "8a5a00")
        #expect(ProfileColor.red.hex == "b3311f")
        #expect(ProfileColor.green.hex == "5e5d59")
        #expect(ProfileColor.purple.hex == "5e5d59")
    }

    @Test("Default profiles have unique IDs and expected colors")
    func defaultProfiles() {
        let defaults = Profile.defaults
        #expect(defaults.count == 3)
        #expect(defaults.map(\.name) == ["Release", "Production", "Master"])
        #expect(defaults.map(\.color) == [.purple, .red, .amber])
        #expect(Set(defaults.map(\.id)).count == 3, "IDs must be unique")
    }

    @Test("Profile name validation rejects empty, brackets, newlines")
    func nameValidation() {
        #expect(Profile(name: "Release", color: .purple).isNameValid)
        #expect(!Profile(name: "", color: .purple).isNameValid)
        #expect(!Profile(name: "  ", color: .purple).isNameValid)
        #expect(!Profile(name: "Bad[Name", color: .purple).isNameValid)
        #expect(!Profile(name: "Multi\nLine", color: .purple).isNameValid)
    }

    @Test("Profile is Hashable with stable ID")
    func hashableUsesAllFields() {
        let id = UUID()
        let a = Profile(id: id, name: "Release", color: .purple, shortcutNumber: 1)
        let b = Profile(id: id, name: "Release", color: .purple, shortcutNumber: 1)
        let c = Profile(id: id, name: "Release", color: .green, shortcutNumber: 1)
        #expect(a == b)
        #expect(a != c)
        #expect(a.hashValue == b.hashValue)
    }

    @Test("ProfileColor is CaseIterable with 5 colors")
    func colorCaseIterable() {
        #expect(ProfileColor.allCases.count == 5)
    }
}
