import Testing
import Foundation
@testable import Hosven

@Suite("Profile + HostsFileManager integration")
@MainActor
struct ProfileIntegrationTests {
    private static let fixture = """
    127.0.0.1\tlocalhost
    ## [tag:Release]
    127.0.0.1\tapi.dev.example.com
    ## [/tag:Release]
    ## [tag:Production]
    10.0.0.5\tapi.prod.example.com
    ## [/tag:Production]
    """

    private func makeManager(initialProfiles: [Profile] = []) -> (HostsFileManager, MockProfileStore) {
        let store = MockProfileStore(initial: initialProfiles)
        let m = HostsFileManager(profileStore: store, autoLoad: false)
        m.parseHostsContent(Self.fixture)
        return (m, store)
    }

    @Test("New tags from /etc/hosts auto-create profiles with heuristic colors")
    func newTagsCreateProfiles() {
        let (m, store) = makeManager()
        let names = m.profiles.map(\.name).sorted()
        #expect(names == ["Production", "Release"])
        // Heuristic: "prod" → production red, anything unrecognised → neutral
        #expect(m.profiles.first(where: { $0.name == "Release" })?.color == .purple)
        #expect(m.profiles.first(where: { $0.name == "Production" })?.color == .red)
        #expect(m.profiles.allSatisfy { $0.shortcutNumber != nil })
        #expect(store.stored.count == 2)
    }

    @Test("Existing profiles preserved across re-parse (orphan tag)")
    func orphanProfilesPreserved() {
        let archived = Profile(name: "Staging", color: .amber, shortcutNumber: 5)
        let (m, _) = makeManager(initialProfiles: [archived])
        // Staging not in fixture but should still be in profiles (orphan)
        #expect(m.profiles.contains(where: { $0.name == "Staging" && $0.color == .amber }))
        #expect(m.profiles.count == 3) // Staging + Release + Production
    }

    @Test("addProfile rejects duplicate name (case-insensitive) and invalid name")
    func addProfileValidation() {
        let (m, _) = makeManager()
        #expect(m.addProfile(name: "release", color: .blue) == nil, "case-insensitive duplicate")
        #expect(m.addProfile(name: "", color: .blue) == nil, "empty name")
        #expect(m.addProfile(name: "Bad[Name", color: .blue) == nil, "bracket name")
        let added = m.addProfile(name: "QA", color: .blue)
        #expect(added != nil)
        #expect(m.profiles.contains(where: { $0.name == "QA" }))
    }

    @Test("removeProfile drops the profile and persists")
    func removeProfilePersists() {
        let (m, store) = makeManager()
        let release = m.profiles.first(where: { $0.name == "Release" })!
        m.removeProfile(id: release.id)
        #expect(!m.profiles.contains(where: { $0.id == release.id }))
        #expect(!store.stored.contains(where: { $0.id == release.id }))
    }

    @Test("renameProfile updates Profile.name AND HostEntry.tag references")
    func renamePropagatesToEntries() {
        let (m, _) = makeManager()
        let release = m.profiles.first(where: { $0.name == "Release" })!
        let ok = m.renameProfile(id: release.id, to: "Dev")
        #expect(ok)
        #expect(m.profiles.first(where: { $0.id == release.id })?.name == "Dev")
        #expect(m.entries.contains(where: { $0.tag == "Dev" }))
        #expect(!m.entries.contains(where: { $0.tag == "Release" }))
        #expect(m.hasUnsavedChanges)
    }

    @Test("renameProfile rejects duplicates and invalid names")
    func renameValidation() {
        let (m, _) = makeManager()
        let release = m.profiles.first(where: { $0.name == "Release" })!
        #expect(!m.renameProfile(id: release.id, to: "Production"), "rename to existing name")
        #expect(!m.renameProfile(id: release.id, to: ""), "empty name")
        #expect(!m.renameProfile(id: UUID(), to: "Whatever"), "unknown ID")
    }

    @Test("Auto-assigned shortcuts increment from highest existing")
    func shortcutAutoAssign() {
        let seed = Profile(name: "Existing", color: .red, shortcutNumber: 3)
        let (m, _) = makeManager(initialProfiles: [seed])
        // Release + Production created with shortcuts > 3
        let releaseShortcut = m.profiles.first(where: { $0.name == "Release" })?.shortcutNumber
        let productionShortcut = m.profiles.first(where: { $0.name == "Production" })?.shortcutNumber
        #expect(releaseShortcut != nil)
        #expect(productionShortcut != nil)
        #expect(releaseShortcut! > 3)
        #expect(productionShortcut! > 3)
        #expect(releaseShortcut != productionShortcut)
    }
}
