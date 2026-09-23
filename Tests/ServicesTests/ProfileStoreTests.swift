import Testing
import Foundation
@testable import Hosven

@Suite("ProfileStore")
struct ProfileStoreTests {
    /// A store on an isolated temp file with no legacy defaults.
    private func makeStore() -> (store: ProfileStore, fileURL: URL) {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("profile-store-\(UUID().uuidString)")
            .appendingPathComponent("profiles.json")
        return (ProfileStore(fileURL: url, legacyDefaults: nil), url)
    }

    @Test("Empty store returns empty array")
    func emptyLoadReturnsEmpty() {
        let (store, _) = makeStore()
        #expect(store.load().isEmpty)
    }

    @Test("save and load round-trips profile data")
    func roundTrip() {
        let (store, _) = makeStore()
        let profiles = Profile.defaults
        store.save(profiles)
        let loaded = store.load()
        #expect(loaded == profiles)
    }

    @Test("Overwrite previous data on subsequent save")
    func overwriteOnResave() {
        let (store, _) = makeStore()
        store.save([.release])
        store.save([.production, .master])
        #expect(store.load().map(\.name) == ["Production", "Master"])
    }

    @Test("Corrupt file returns empty array, does not crash")
    func corruptFileIsTolerated() throws {
        let (store, url) = makeStore()
        try HosvenConfig.ensureDirectory(for: url)
        try Data([0xFF, 0x00, 0xAB]).write(to: url)
        #expect(store.load().isEmpty)
    }

    @Test("Legacy UserDefaults blob migrates into the file")
    func legacyMigration() {
        let suiteName = "test.profile-store.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defaults.removePersistentDomain(forName: suiteName)
        defaults.set(try! JSONEncoder().encode([Profile.release]), forKey: ProfileStore.legacyDefaultsKey)
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("profile-store-\(UUID().uuidString)")
            .appendingPathComponent("profiles.json")
        let store = ProfileStore(fileURL: url, legacyDefaults: defaults)
        #expect(store.load() == [.release])
        #expect(FileManager.default.fileExists(atPath: url.path))
    }

    @Test("MockProfileStore mirrors real semantics")
    func mockBehaviour() {
        let mock = MockProfileStore(initial: [.release])
        #expect(mock.load() == [.release])
        mock.save([.production])
        #expect(mock.load() == [.production])
    }
}
