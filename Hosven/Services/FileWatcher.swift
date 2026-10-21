import Foundation

/// Watches one file (`/etc/hosts`, the selected `.env`) for external modifications.
/// Coalesces FS events with a debounce so a single edit doesn't fire 5 callbacks.
/// The descriptor follows the inode: after an atomic save (write temp, rename over) the
/// watcher reports `.deleted`, and the caller restarts it to pick up the new file.
///
/// Lifecycle: caller creates instance → calls `start()` → receives `onChange` → calls
/// `stop()` (or lets deinit run). Safe to start/stop repeatedly. Self-write suppression
/// is the caller's responsibility — typically suspend before applyChanges, resume after.
final class FileWatcher {
    enum Event {
        case modified
        case deleted
    }

    var onChange: ((Event) -> Void)?

    private let path: String
    private let debounceInterval: TimeInterval
    private var source: DispatchSourceFileSystemObject?
    private var fileDescriptor: Int32 = -1
    private var debounceWorkItem: DispatchWorkItem?
    private let queue = DispatchQueue(label: "com.twannguyen.hosven.filewatcher", qos: .utility)
    private var isSuspended = false

    init(path: String = "/etc/hosts", debounceInterval: TimeInterval = 0.4) {
        self.path = path
        self.debounceInterval = debounceInterval
    }

    deinit {
        stop()
    }

    /// Begin watching. No-op if already running.
    func start() {
        guard source == nil else { return }
        let fd = open(path, O_EVTONLY)
        guard fd >= 0 else { return }
        fileDescriptor = fd

        let src = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fd,
            eventMask: [.write, .extend, .delete, .rename],
            queue: queue
        )
        src.setEventHandler { [weak self] in
            guard let self else { return }
            let mask = src.data
            let event: Event = (mask.contains(.delete) || mask.contains(.rename)) ? .deleted : .modified
            self.scheduleFire(event)
        }
        src.setCancelHandler { [weak self] in
            guard let self else { return }
            if self.fileDescriptor >= 0 {
                close(self.fileDescriptor)
                self.fileDescriptor = -1
            }
        }
        source = src
        src.resume()
    }

    /// Stop watching and release the file descriptor.
    func stop() {
        debounceWorkItem?.cancel()
        debounceWorkItem = nil
        source?.cancel()
        source = nil
    }

    /// Pause callback delivery without tearing down the descriptor — used to skip
    /// our own writes so we don't show a conflict warning for self-induced changes.
    func suspend() {
        guard !isSuspended, let src = source else { return }
        src.suspend()
        isSuspended = true
    }

    func resume() {
        guard isSuspended, let src = source else { return }
        src.resume()
        isSuspended = false
    }

    private func scheduleFire(_ event: Event) {
        debounceWorkItem?.cancel()
        let work = DispatchWorkItem { [weak self] in
            DispatchQueue.main.async {
                self?.onChange?(event)
            }
        }
        debounceWorkItem = work
        queue.asyncAfter(deadline: .now() + debounceInterval, execute: work)
    }
}
