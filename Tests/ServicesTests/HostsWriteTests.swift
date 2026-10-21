import Testing
import Foundation
@testable import Hosven

/// Stands in for the privileged helper: records each command and answers with a fixed outcome.
@MainActor
final class FakePrivilegedRunner {
    var outcome: (Bool, String?) = (true, nil)
    private(set) var commands: [String] = []

    /// Commands that copy a temp file over `/etc/hosts`.
    var writes: [String] { commands.filter { $0.contains("/etc/hosts &&") } }

    /// Content of the temp file the last write command copies.
    var lastWrittenContent: String? {
        guard let command = writes.last,
              let start = command.range(of: "cp '")?.upperBound,
              let end = command[start...].range(of: "'")?.lowerBound else { return nil }
        return try? String(contentsOfFile: String(command[start..<end]), encoding: .utf8)
    }

    var runner: HostsFileManager.PrivilegedRunner {
        { command, completion in
            self.commands.append(command)
            completion(self.outcome.0, self.outcome.1)
        }
    }
}

@MainActor
func makeHostsManager(_ content: String, runner: FakePrivilegedRunner, profiles: [Profile] = []) -> HostsFileManager {
    let m = HostsFileManager(
        profileStore: MockProfileStore(initial: profiles),
        autoLoad: false,
        privilegedRunner: runner.runner,
        resolver: { _ in ["127.0.0.1"] }
    )
    m.parseHostsContent(content)
    m.captureClean()
    return m
}

@MainActor
func waitForVerification(_ m: HostsFileManager) async -> HostsVerification? {
    for _ in 0..<100 {
        if let v = m.verification { return v }
        try? await Task.sleep(for: .milliseconds(20))
    }
    return nil
}

@Suite("Every /etc/hosts write goes through one path")
@MainActor
struct HostsWriteTests {
    private let base = "127.0.0.1\tlocalhost\n"

    @Test("Raw editor write verifies the hostnames it changed")
    func rawWriteVerifies() async {
        let runner = FakePrivilegedRunner()
        let m = makeHostsManager(base, runner: runner)
        m.applyRawText(base + "127.0.0.1\tapi.test\n")
        #expect(runner.writes.count == 1)
        let verification = await waitForVerification(m)
        #expect(verification?.results.map(\.hostname) == ["api.test"])
        #expect(verification?.mismatches.isEmpty == true)
        #expect(!m.hasUnsavedChanges)
        #expect(m.entries.contains { $0.hostname == "api.test" })
    }

    @Test("Model write verifies and keeps entry ids")
    func modelWriteVerifies() async {
        let runner = FakePrivilegedRunner()
        let m = makeHostsManager(base, runner: runner)
        m.addEntry(ip: "127.0.0.1", hostname: "web.test", comment: "")
        let ids = m.entries.map(\.id)
        m.applyChanges()
        #expect(runner.writes.count == 1)
        #expect(runner.lastWrittenContent == m.generateHostsContent())
        #expect(await waitForVerification(m)?.results.map(\.hostname) == ["web.test"])
        #expect(m.entries.map(\.id) == ids)
        #expect(!m.hasUnsavedChanges)
    }

    @Test("A cancelled prompt keeps the edits pending and reports nothing")
    func cancelledKeepsDirty() {
        let runner = FakePrivilegedRunner()
        runner.outcome = (false, nil)
        let m = makeHostsManager(base, runner: runner)
        m.addEntry(ip: "127.0.0.1", hostname: "web.test", comment: "")
        m.toast = nil
        var result: (Bool, String?)?
        m.applyChanges { result = ($0, $1) }
        #expect(result?.0 == false && result?.1 == nil)
        #expect(m.hasUnsavedChanges)
        #expect(m.toast == nil)
        #expect(!m.isApplying)
    }

    @Test("A refused write shows the error")
    func failedShowsError() {
        let runner = FakePrivilegedRunner()
        runner.outcome = (false, "quyền quản trị bị từ chối (mã -60005)")
        let m = makeHostsManager(base, runner: runner)
        m.applyRawText(base + "127.0.0.1\tapi.test\n")
        #expect(m.toast?.type == .error)
        #expect(m.verification == nil)
    }

    @Test("Edits made while the prompt was open stay dirty after the write")
    func editsDuringWriteStayDirty() {
        var pending: ((Bool, String?) -> Void)?
        let m = HostsFileManager(
            profileStore: MockProfileStore(),
            autoLoad: false,
            privilegedRunner: { _, completion in pending = completion },
            resolver: { _ in [] }
        )
        m.parseHostsContent(base)
        m.captureClean()
        m.addEntry(ip: "127.0.0.1", hostname: "a.test", comment: "")
        m.applyChanges()
        m.addEntry(ip: "127.0.0.1", hostname: "b.test", comment: "")
        pending?(true, nil)
        #expect(m.hasUnsavedChanges)
    }
}
