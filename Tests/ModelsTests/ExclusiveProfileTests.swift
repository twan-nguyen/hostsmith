import Testing
import Foundation
@testable import Hosven

@Suite("Environment profiles exclude each other and switch in one write")
@MainActor
struct ExclusiveProfileTests {
    nonisolated private static let fixture = """
    127.0.0.1\tlocalhost

    ## [tag:Local]
    127.0.0.1\tapi.test
    ## [/tag:Local]

    ## [tag:Staging]
    # 10.0.0.5\tapi.test
    ## [/tag:Staging]

    ## [tag:Docker]
    127.0.0.1\tdb.test
    ## [/tag:Docker]
    """

    private static let profiles = [
        Profile(name: "Local", color: .blue, isExclusive: true),
        Profile(name: "Staging", color: .amber, isExclusive: true),
        Profile(name: "Docker", color: .purple, isExclusive: false),
    ]

    private func make(_ content: String = fixture) -> (HostsFileManager, FakePrivilegedRunner) {
        let runner = FakePrivilegedRunner()
        return (makeHostsManager(content, runner: runner, profiles: Self.profiles), runner)
    }

    private func id(_ m: HostsFileManager, _ name: String) -> UUID {
        m.profiles.first { $0.name == name }!.id
    }

    private func enabled(_ m: HostsFileManager, _ tag: String) -> [Bool] {
        m.entries.filter { $0.tag == tag }.map(\.isEnabled)
    }

    @Test("Active profile is read from the entries")
    func activeIsDerived() {
        let (m, _) = make()
        #expect(m.activeProfileState == .active(id(m, "Local")))
        #expect(m.activeProfile?.name == "Local")
    }

    @Test("Switching turns the other environment off, leaves blocks alone and writes once")
    func switchIsExclusive() {
        let (m, runner) = make()
        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: id(m, "Staging"), policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .written)
        #expect(runner.writes.count == 1)
        #expect(enabled(m, "Local") == [false])
        #expect(enabled(m, "Staging") == [true])
        #expect(enabled(m, "Docker") == [true])
        #expect(m.activeProfileState == .active(id(m, "Staging")))
        #expect(!m.hasUnsavedChanges)
        #expect(runner.lastWrittenContent?.contains("10.0.0.5\tapi.test") == true)
        #expect(runner.lastWrittenContent?.contains("# 127.0.0.1\tapi.test") == true)
    }

    @Test("Deactivating turns the active environment off and writes")
    func deactivate() {
        let (m, runner) = make()
        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: nil, policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .written)
        #expect(runner.writes.count == 1)
        #expect(m.activeProfileState == HostsFileManager.ActiveProfileState.none)
        #expect(enabled(m, "Docker") == [true])
        #expect(m.toast?.message == "Đã tắt Local")
    }

    @Test("Two environments on is a conflict that a switch resolves")
    func conflictResolves() {
        let (m, _) = make(Self.fixture.replacingOccurrences(of: "# 10.0.0.5", with: "10.0.0.5"))
        #expect(m.activeProfileState == .conflict([id(m, "Local"), id(m, "Staging")]))
        m.switchEnvironment(to: id(m, "Local"), policy: .askIfPending)
        #expect(m.activeProfileState == .active(id(m, "Local")))
    }

    @Test("Blocks and empty profiles cannot be switched to")
    func nonEnvironmentsIgnored() {
        let (m, runner) = make()
        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: id(m, "Docker"), policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .nothingToDo)
        _ = m.addProfile(name: "QA", color: .amber, isExclusive: true)
        m.switchEnvironment(to: id(m, "QA"), policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .nothingToDo)
        #expect(runner.writes.isEmpty)
    }

    @Test("Switching to the active environment with nothing pending writes nothing")
    func alreadyActive() {
        let (m, runner) = make()
        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: id(m, "Local"), policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .nothingToDo)
        #expect(runner.writes.isEmpty)
    }

    @Test("Pending edits: the window asks, background callers refuse, confirmation writes both")
    func pendingPolicy() {
        let (m, runner) = make()
        m.addEntry(ip: "127.0.0.1", hostname: "extra.test", comment: "")
        let staging = id(m, "Staging")

        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: staging, policy: .refuseIfPending) { outcome = $0 }
        #expect(outcome == .refused(pendingCount: 1))
        #expect(m.pendingSwitch == nil)

        m.switchEnvironment(to: staging, policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .needsConfirmation(pendingCount: 1))
        #expect(m.pendingSwitch?.target == staging)
        #expect(runner.writes.isEmpty)

        m.confirmPendingSwitch()
        #expect(m.pendingSwitch == nil)
        #expect(runner.writes.count == 1)
        #expect(runner.lastWrittenContent?.contains("extra.test") == true)
        #expect(m.activeProfileState == .active(staging))
    }

    @Test("Cancelling the confirmation writes nothing")
    func cancelConfirmation() {
        let (m, runner) = make()
        m.addEntry(ip: "127.0.0.1", hostname: "extra.test", comment: "")
        m.switchEnvironment(to: id(m, "Staging"), policy: .askIfPending)
        m.cancelPendingSwitch()
        #expect(m.pendingSwitch == nil)
        #expect(runner.writes.isEmpty)
        #expect(m.activeProfileState == .active(id(m, "Local")))
    }

    @Test("A cancelled prompt restores the entries")
    func cancelledPromptRestores() {
        let (m, runner) = make()
        runner.outcome = (false, nil)
        let before = m.entries
        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: id(m, "Staging"), policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .cancelled)
        #expect(m.entries == before)
        #expect(!m.hasUnsavedChanges)
        #expect(!m.canUndo)
    }

    @Test("A failed write reports the error and restores the entries")
    func failedRestores() {
        let (m, runner) = make()
        runner.outcome = (false, "quyền quản trị bị từ chối (mã 1)")
        var outcome: HostsFileManager.SwitchOutcome?
        m.switchEnvironment(to: id(m, "Staging"), policy: .askIfPending) { outcome = $0 }
        #expect(outcome == .failed("quyền quản trị bị từ chối (mã 1)"))
        #expect(m.activeProfileState == .active(id(m, "Local")))
    }

    @Test("Unmerged editor typing is folded in, not lost")
    func rawDraftMerged() {
        let (m, runner) = make()
        m.rawDraft = m.generateHostsContent() + "127.0.0.1\ttyped.test\n"
        m.markRawTextDirty()
        m.switchEnvironment(to: id(m, "Staging"), policy: .askIfPending)
        #expect(m.pendingSwitch != nil)
        #expect(m.entries.contains { $0.hostname == "typed.test" })
        m.confirmPendingSwitch()
        #expect(runner.lastWrittenContent?.contains("typed.test") == true)
    }

    @Test("Editor text that matches the model does not count as pending")
    func unchangedDraftIsNotPending() {
        let (m, runner) = make()
        m.rawDraft = m.generateHostsContent()
        m.markRawTextDirty()
        m.switchEnvironment(to: id(m, "Staging"), policy: .refuseIfPending)
        #expect(runner.writes.count == 1)
    }

    @Test("Preview counts what a switch turns on and off")
    func preview() {
        let (m, _) = make()
        let preview = m.switchPreview(to: id(m, "Staging"))
        #expect(preview.enable == 1)
        #expect(preview.disable.map(\.name) == ["Local"])
        #expect(preview.disable.map(\.count) == [1])
    }

    @Test("Blocks found in the file start independent")
    func newTagsAreBlocks() {
        let (m, _) = make(Self.fixture + "\n## [tag:Mock]\n127.0.0.1\tmock.test\n## [/tag:Mock]\n")
        #expect(m.profiles.first { $0.name == "Mock" }?.isExclusive == false)
    }

    @Test("Stored profiles without the flag migrate by colour; the flag round-trips")
    func migrationDecode() throws {
        let id = UUID()
        let json = """
        [{"id":"\(id.uuidString)","name":"Staging","color":"amber"},
         {"id":"\(UUID().uuidString)","name":"Mock","color":"purple"}]
        """
        let decoded = try JSONDecoder().decode([Profile].self, from: Data(json.utf8))
        #expect(decoded.map(\.isExclusive) == [true, false])

        var block = decoded[0]
        block.isExclusive = false
        let roundTrip = try JSONDecoder().decode(Profile.self, from: JSONEncoder().encode(block))
        #expect(roundTrip.isExclusive == false)
    }
}
