import Foundation
import Darwin

/// Scans listening TCP ports with `lsof` and attributes each one to a known repo,
/// Docker, or nothing. Spec: `guidelines/10-app-anatomy.md` → Ports, `PortRow.md`.
/// Results stream into `ports` per scan; nothing here is cached across launches.
@Observable
@MainActor
final class PortsManager {
    var ports: [ListeningPort] = []
    var isScanning = false
    var lastScan: Date?
    var toast: ToastMessage?

    private static let lsofPath = "/usr/sbin/lsof"
    private static let dockerCandidates = ["/usr/local/bin/docker", "/opt/homebrew/bin/docker", "/Applications/Docker.app/Contents/Resources/bin/docker"]

    /// Repo paths used to attribute a process as "yours".
    var knownRepos: [(name: String, path: String)] = []

    /// Everything except macOS's own daemons; what the sidebar count and menu bar show.
    var userPorts: [ListeningPort] { ports.filter { !$0.isSystem } }
    /// Unknown processes on dev ports, in port order.
    var attentionPorts: [ListeningPort] { ports.filter(\.needsAttention) }

    // MARK: - Scan

    func scan() {
        guard !isScanning else { return }
        isScanning = true
        let repos = knownRepos
        Task.detached(priority: .userInitiated) {
            let listing = Self.run(Self.lsofPath, ["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcuLn"]) ?? ""
            var records = Self.parseListing(listing)
            let pids = Array(Set(records.map(\.pid)))
            let cwds = pids.isEmpty ? [:] : Self.cwdByPid(Self.run(Self.lsofPath, ["-a", "-d", "cwd", "-Fpn", "-p", pids.map(String.init).joined(separator: ",")]) ?? "")
            let containers = Self.dockerContainersByPort()
            records = records.map { record in
                var r = record
                r.cwd = cwds[record.pid]
                r.executable = Self.executablePath(pid: record.pid)
                return r
            }
            let result: [ListeningPort] = records
                .sorted { $0.port == $1.port ? $0.pid < $1.pid : $0.port < $1.port }
                .map { r in
                    ListeningPort(
                        port: r.port,
                        process: r.command,
                        pid: r.pid,
                        user: r.user,
                        cwd: r.cwd,
                        owner: Self.owner(for: r, repos: repos, containers: containers),
                        isSystem: Self.isSystemProcess(r)
                    )
                }
            await MainActor.run {
                self.ports = result
                self.isScanning = false
                self.lastScan = Date()
            }
        }
    }

    // MARK: - Kill / stop

    /// Sends SIGTERM (or SIGKILL when `force`). Falls back to the privileged
    /// helper when the process belongs to another user. Reports as a fact.
    func kill(_ port: ListeningPort, force: Bool = false, completion: @escaping (Bool) -> Void) {
        let signal = force ? SIGKILL : SIGTERM
        if Darwin.kill(port.pid, signal) == 0 {
            finishKill(port, completion: completion)
            return
        }
        guard errno == EPERM else {
            showToast("Không gửi được tín hiệu tới \(port.process) (pid \(port.pid))", type: .error)
            completion(false)
            return
        }
        let command = "kill -\(force ? "KILL" : "TERM") \(port.pid)"
        DispatchQueue.global(qos: .userInitiated).async {
            let status = runPrivilegedShellCommand(command)
            DispatchQueue.main.async {
                if status == errAuthorizationSuccess {
                    self.finishKill(port, completion: completion)
                } else if status != errAuthorizationCanceled {
                    self.showToast("Không dừng được \(port.process) vì quyền quản trị bị từ chối", type: .error)
                    completion(false)
                } else {
                    completion(false)
                }
            }
        }
    }

    private func finishKill(_ port: ListeningPort, completion: @escaping (Bool) -> Void) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
            let alive = Darwin.kill(port.pid, 0) == 0
            if alive {
                self.showToast("\(port.process) (pid \(port.pid)) vẫn chạy sau SIGTERM", type: .info)
            } else {
                self.showToast("Đã giải phóng cổng \(port.port)", type: .success)
            }
            self.scan()
            completion(!alive)
        }
    }

    /// Stops a Docker container by name through the docker CLI.
    func stopContainer(_ port: ListeningPort) {
        guard case .container(let name?) = port.owner, let docker = Self.dockerPath() else {
            showToast("Không tìm thấy docker CLI để dừng container trên cổng \(port.port)", type: .error)
            return
        }
        Task.detached {
            _ = Self.run(docker, ["stop", name])
            await MainActor.run {
                self.showToast("Đã dừng container \(name)", type: .success)
                self.scan()
            }
        }
    }

    func showToast(_ message: String, type: ToastType) {
        toast = ToastMessage(message: message, type: type)
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
            if self?.toast?.message == message { self?.toast = nil }
        }
    }

    // MARK: - Parsing

    struct Record {
        var pid: Int32
        var command: String
        var user: String
        var port: Int
        var cwd: String?
        var executable: String?
    }

    // MARK: - System processes

    private static let systemPathPrefixes = ["/System/", "/usr/libexec/", "/usr/sbin/", "/Library/Apple/", "/private/var/db/"]
    /// Fallback by name for the few daemons whose path cannot be read.
    private static let systemCommandNames: Set<String> = [
        "ControlCenter", "rapportd", "sharingd", "AirPlayXPCHelper", "remoted", "launchd",
        "identityservicesd", "cupsd", "bluetoothd", "airportd", "mDNSResponder", "remotepairingd",
    ]

    nonisolated static func isSystemProcess(_ record: Record) -> Bool {
        if let path = record.executable, systemPathPrefixes.contains(where: { path.hasPrefix($0) }) { return true }
        return systemCommandNames.contains(record.command)
    }

    nonisolated static func executablePath(pid: Int32) -> String? {
        var buffer = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
        guard proc_pidpath(pid, &buffer, UInt32(buffer.count)) > 0 else { return nil }
        return String(cString: buffer)
    }

    /// Parses `lsof -F pcuLn` output: one field per line, `p` starts a process,
    /// `n` carries `host:port`.
    nonisolated static func parseListing(_ text: String) -> [Record] {
        var records: [Record] = []
        var seen = Set<String>()
        var pid: Int32 = 0
        var command = ""
        var user = ""
        var login = ""
        for line in text.split(separator: "\n") {
            guard let tag = line.first else { continue }
            let value = String(line.dropFirst())
            switch tag {
            case "p": pid = Int32(value) ?? 0; command = ""; user = ""; login = ""
            case "c": command = value
            case "u": user = value
            case "L": login = value
            case "n":
                guard let colon = value.lastIndex(of: ":"), let port = Int(value[value.index(after: colon)...]) else { continue }
                let key = "\(pid):\(port)"
                guard !seen.contains(key) else { continue }
                seen.insert(key)
                records.append(Record(pid: pid, command: command, user: login.isEmpty ? user : login, port: port, cwd: nil))
            default: break
            }
        }
        return records
    }

    nonisolated static func cwdByPid(_ text: String) -> [Int32: String] {
        var result: [Int32: String] = [:]
        var pid: Int32 = 0
        for line in text.split(separator: "\n") {
            guard let tag = line.first else { continue }
            let value = String(line.dropFirst())
            if tag == "p" { pid = Int32(value) ?? 0 }
            if tag == "n" { result[pid] = value }
        }
        return result
    }

    nonisolated static func owner(for record: Record, repos: [(name: String, path: String)], containers: [Int: String]) -> ListeningPort.Owner {
        let lower = record.command.lowercased()
        if lower.contains("docker") {
            return .container(name: containers[record.port])
        }
        if let cwd = record.cwd {
            for repo in repos where cwd == repo.path || cwd.hasPrefix(repo.path + "/") {
                return .yours(repo: repo.name)
            }
        }
        return .unknown
    }

    nonisolated private static func dockerContainersByPort() -> [Int: String] {
        guard let docker = dockerPath(), let out = run(docker, ["ps", "--format", "{{.Names}}\t{{.Ports}}"]) else { return [:] }
        var map: [Int: String] = [:]
        for line in out.split(separator: "\n") {
            let parts = line.split(separator: "\t", maxSplits: 1)
            guard parts.count == 2 else { continue }
            let name = String(parts[0])
            for token in parts[1].split(separator: ",") {
                guard let arrow = token.range(of: "->") else { continue }
                let host = token[..<arrow.lowerBound]
                if let colon = host.lastIndex(of: ":"), let port = Int(host[host.index(after: colon)...].trimmingCharacters(in: .whitespaces)) {
                    map[port] = name
                }
            }
        }
        return map
    }

    nonisolated private static func dockerPath() -> String? {
        dockerCandidates.first { FileManager.default.isExecutableFile(atPath: $0) }
    }

    nonisolated private static func run(_ path: String, _ arguments: [String]) -> String? {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: path)
        process.arguments = arguments
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
        } catch {
            return nil
        }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return String(data: data, encoding: .utf8)
    }
}
