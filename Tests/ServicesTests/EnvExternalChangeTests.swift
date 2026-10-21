import Testing
import Foundation
@testable import Hosven

@Suite("A .env changed on disk is never overwritten silently")
@MainActor
struct EnvExternalChangeTests {
    @MainActor
    private struct Fixture {
        let manager: EnvFileManager
        let repo: EnvRepo
        let dir: URL
        var envURL: URL { dir.appendingPathComponent(".env") }
        var file: EnvFile { manager.loadedFile(repoId: repo.id, relativePath: ".env")! }

        func disk() throws -> String { try String(contentsOf: envURL, encoding: .utf8) }

        /// Saves the way editors do: write a temp file, then rename it over the original.
        func saveAtomically(_ content: String) throws {
            let temp = dir.appendingPathComponent(".env.tmp-\(UUID().uuidString)")
            try content.write(to: temp, atomically: false, encoding: .utf8)
            _ = try FileManager.default.replaceItemAt(envURL, withItemAt: temp)
        }
    }

    private func makeFixture() throws -> Fixture {
        let dir = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("env-ext-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try "FOO=1\nBAR=2\n".write(to: dir.appendingPathComponent(".env"), atomically: true, encoding: .utf8)
        let m = EnvFileManager(storageURL: dir.appendingPathComponent("env-config.json"))
        try m.addRepo(at: dir)
        let repo = m.repos[0]
        m.setLoadedFile(repoId: repo.id, file: try m.loadFile(repoId: repo.id, relativePath: ".env"))
        return Fixture(manager: m, repo: repo, dir: dir)
    }

    private func edit(_ f: Fixture, value: String) {
        f.manager.updateEntry(repoId: f.repo.id, fileId: f.file.id, entryId: f.file.entries[0].id, key: "FOO", value: value, comment: "")
    }

    private func waitUntil(_ condition: () -> Bool) async -> Bool {
        for _ in 0..<150 {
            if condition() { return true }
            try? await Task.sleep(for: .milliseconds(20))
        }
        return condition()
    }

    @Test("Writing over an external edit throws and keeps the disk as it is")
    func guardRefuses() throws {
        let f = try makeFixture()
        defer { try? FileManager.default.removeItem(at: f.dir) }
        edit(f, value: "mine")
        try "FOO=theirs\nBAR=2\n".write(to: f.envURL, atomically: true, encoding: .utf8)

        #expect(throws: EnvError.self) { try f.manager.applyChanges(repoId: f.repo.id, file: f.file) }
        #expect(try f.disk() == "FOO=theirs\nBAR=2\n")
        #expect(f.manager.externalChange(repoId: f.repo.id, relativePath: ".env") == .changed)
        #expect(f.file.hasUnsavedChanges)
    }

    @Test("Forcing the write replaces the external edit and clears the conflict")
    func forceWrites() throws {
        let f = try makeFixture()
        defer { try? FileManager.default.removeItem(at: f.dir) }
        edit(f, value: "mine")
        try "FOO=theirs\nBAR=2\n".write(to: f.envURL, atomically: true, encoding: .utf8)
        try? f.manager.applyChanges(repoId: f.repo.id, file: f.file)

        try f.manager.applyChanges(repoId: f.repo.id, file: f.file, force: true)
        #expect(try f.disk().contains("FOO=mine"))
        #expect(f.manager.externalChange(repoId: f.repo.id, relativePath: ".env") == nil)
        #expect(!f.file.hasUnsavedChanges)
    }

    @Test("Raw writes are guarded the same way")
    func rawGuarded() throws {
        let f = try makeFixture()
        defer { try? FileManager.default.removeItem(at: f.dir) }
        try "FOO=theirs\n".write(to: f.envURL, atomically: true, encoding: .utf8)
        #expect(throws: EnvError.self) {
            try f.manager.applyRawText(repoId: f.repo.id, fileId: f.file.id, relativePath: ".env", rawText: "FOO=mine\n")
        }
        #expect(try f.disk() == "FOO=theirs\n")
    }

    @Test("Hosven's own write leaves the disk state unchanged")
    func ownWriteIsUnchanged() throws {
        let f = try makeFixture()
        defer { try? FileManager.default.removeItem(at: f.dir) }
        edit(f, value: "mine")
        try f.manager.applyChanges(repoId: f.repo.id, file: f.file)
        #expect(f.manager.diskState(repoId: f.repo.id, relativePath: ".env") == .unchanged)
        edit(f, value: "again")
        try f.manager.applyChanges(repoId: f.repo.id, file: f.file)
        #expect(try f.disk().contains("FOO=again"))
    }

    @Test("A clean file reloads in place; a dirty one becomes a conflict")
    func refreshBehaviour() throws {
        let f = try makeFixture()
        defer { try? FileManager.default.removeItem(at: f.dir) }
        try "FOO=theirs\nBAR=2\n".write(to: f.envURL, atomically: true, encoding: .utf8)
        f.manager.refreshFromDisk(repoId: f.repo.id, relativePath: ".env")
        #expect(f.file.entries.first?.value == "theirs")
        #expect(!f.file.hasUnsavedChanges)

        edit(f, value: "mine")
        try "FOO=later\nBAR=2\n".write(to: f.envURL, atomically: true, encoding: .utf8)
        f.manager.refreshFromDisk(repoId: f.repo.id, relativePath: ".env")
        #expect(f.file.entries.first?.value == "mine")
        #expect(f.manager.externalChange(repoId: f.repo.id, relativePath: ".env") == .changed)

        try f.manager.reloadFromDisk(repoId: f.repo.id, relativePath: ".env")
        #expect(f.file.entries.first?.value == "later")
        #expect(f.manager.externalChange(repoId: f.repo.id, relativePath: ".env") == nil)
    }

    @Test("A deleted file is reported missing and can be recreated")
    func missingFile() throws {
        let f = try makeFixture()
        defer { try? FileManager.default.removeItem(at: f.dir) }
        try FileManager.default.removeItem(at: f.envURL)
        #expect(f.manager.diskState(repoId: f.repo.id, relativePath: ".env") == .missing)
        f.manager.refreshFromDisk(repoId: f.repo.id, relativePath: ".env")
        #expect(f.manager.externalChange(repoId: f.repo.id, relativePath: ".env") == .missing)

        try f.manager.applyChanges(repoId: f.repo.id, file: f.file, force: true)
        #expect(try f.disk() == "FOO=1\nBAR=2\n")
    }

    @Test("The watcher follows atomic saves across more than one save")
    func watcherRearms() async throws {
        let f = try makeFixture()
        defer {
            f.manager.watchSelectedFile(repoId: f.repo.id, relativePath: nil)
            try? FileManager.default.removeItem(at: f.dir)
        }
        f.manager.watchSelectedFile(repoId: f.repo.id, relativePath: ".env")
        try await Task.sleep(for: .milliseconds(100))

        try f.saveAtomically("FOO=first\nBAR=2\n")
        #expect(await waitUntil { f.file.entries.first?.value == "first" })

        try f.saveAtomically("FOO=second\nBAR=2\n")
        #expect(await waitUntil { f.file.entries.first?.value == "second" })
    }
}
