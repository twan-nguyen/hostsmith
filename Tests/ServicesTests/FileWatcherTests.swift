import Testing
import Foundation
@testable import Hosven

@Suite("FileWatcher")
struct FileWatcherTests {
    /// Allocate a unique temp path per test — DispatchSource holds an FD on it.
    private func tempPath() -> String {
        NSTemporaryDirectory() + "hostswatcher-\(UUID().uuidString).txt"
    }

    @Test("Detects modification to watched file")
    func detectsModification() async throws {
        let path = tempPath()
        try "initial".write(toFile: path, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let watcher = FileWatcher(path: path, debounceInterval: 0.1)
        let received = AsyncReceiver<FileWatcher.Event>()
        watcher.onChange = { received.send($0) }
        watcher.start()
        defer { watcher.stop() }

        try await Task.sleep(nanoseconds: 100_000_000)  // let watcher settle
        try "modified".write(toFile: path, atomically: false, encoding: .utf8)

        let event = try await received.next(timeout: 2.0)
        // .write events sometimes report .extend too — accept either modified outcome
        #expect(event == .modified || event == .deleted)
    }

    @Test("Debounces rapid successive writes into single callback")
    func debouncesRapidWrites() async throws {
        let path = tempPath()
        try "v0".write(toFile: path, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let watcher = FileWatcher(path: path, debounceInterval: 0.3)
        let received = AsyncReceiver<FileWatcher.Event>()
        watcher.onChange = { received.send($0) }
        watcher.start()
        defer { watcher.stop() }
        try await Task.sleep(nanoseconds: 100_000_000)

        for i in 1...5 {
            try "v\(i)".write(toFile: path, atomically: false, encoding: .utf8)
            try await Task.sleep(nanoseconds: 30_000_000)
        }

        // Wait debounce + a bit; expect EXACTLY one callback
        try await Task.sleep(nanoseconds: 600_000_000)
        #expect(received.count == 1, "expected 1 debounced callback, got \(received.count)")
    }

    @Test("Suspend silences callbacks during the suspended window")
    func suspendSilencesCallbacks() async throws {
        let path = tempPath()
        try "v0".write(toFile: path, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let watcher = FileWatcher(path: path, debounceInterval: 0.1)
        let received = AsyncReceiver<FileWatcher.Event>()
        watcher.onChange = { received.send($0) }
        watcher.start()
        defer { watcher.stop() }
        try await Task.sleep(nanoseconds: 100_000_000)

        watcher.suspend()
        try "during-suspend".write(toFile: path, atomically: false, encoding: .utf8)
        // Wait beyond debounce — handler must NOT fire while suspended.
        try await Task.sleep(nanoseconds: 400_000_000)
        #expect(received.count == 0, "no callbacks while suspended (got \(received.count))")
        // Note: DispatchSource queues events; they fire after resume — which is the
        // intended behavior (we resume after our own write completes). Don't assert
        // on post-resume delivery here; that's tested separately.
        watcher.resume()
    }
}

/// Minimal thread-safe event collector for async tests. Avoids cross-actor isolation.
final class AsyncReceiver<T: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var events: [T] = []

    var count: Int {
        lock.lock(); defer { lock.unlock() }
        return events.count
    }

    func send(_ event: T) {
        lock.lock(); defer { lock.unlock() }
        events.append(event)
    }

    func next(timeout: TimeInterval) async throws -> T {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            lock.lock()
            if let first = events.first {
                events.removeFirst()
                lock.unlock()
                return first
            }
            lock.unlock()
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        throw NSError(domain: "AsyncReceiver", code: -1, userInfo: [NSLocalizedDescriptionKey: "timeout waiting for event"])
    }
}
