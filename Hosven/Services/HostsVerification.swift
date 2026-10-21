import Foundation

/// Asks the system resolver what it now returns for the hostnames a write touched,
/// so the banner can report a counted fact instead of assuming the write took.
struct HostsVerification: Equatable {
    struct Result: Equatable, Identifiable {
        let hostname: String
        let expectedIP: String
        let resolvedIPs: [String]
        var id: String { hostname }
        var matches: Bool { resolvedIPs.contains(expectedIP) }
    }

    let results: [Result]
    let checkedAt: Date

    var mismatches: [Result] { results.filter { !$0.matches } }

    /// Enabled `ip hostname` pairs present in `new` but not in `old`, capped so a
    /// bulk import does not turn into a hundred resolver calls.
    static func changedPairs(old: String, new: String, limit: Int = 20) -> [(hostname: String, ip: String)] {
        let before = Set(old.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) })
        var pairs: [(hostname: String, ip: String)] = []
        for raw in new.split(separator: "\n") {
            let line = raw.trimmingCharacters(in: .whitespaces)
            guard !line.isEmpty, !line.hasPrefix("#"), !before.contains(line) else { continue }
            let body = line.split(separator: "#", maxSplits: 1).first.map(String.init) ?? line
            let fields = body.split(whereSeparator: { $0 == " " || $0 == "\t" }).map(String.init)
            guard fields.count >= 2 else { continue }
            for hostname in fields.dropFirst() {
                pairs.append((hostname, fields[0]))
                if pairs.count == limit { return pairs }
            }
        }
        return pairs
    }

    static func run(
        _ pairs: [(hostname: String, ip: String)],
        resolve: @escaping @Sendable (String) -> [String] = { HostsVerification.resolve($0) }
    ) async -> HostsVerification {
        let results = await Task.detached(priority: .utility) {
            pairs.map { Result(hostname: $0.hostname, expectedIP: $0.ip, resolvedIPs: resolve($0.hostname)) }
        }.value
        return HostsVerification(results: results, checkedAt: Date())
    }

    /// `dscacheutil -q host -a name X` prints `ip_address:` and `ipv6_address:` lines.
    static func resolve(_ hostname: String) -> [String] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/dscacheutil")
        process.arguments = ["-q", "host", "-a", "name", hostname]
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
        } catch {
            return []
        }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        let output = String(data: data, encoding: .utf8) ?? ""
        return output.split(separator: "\n").compactMap { line in
            let parts = line.split(separator: ":", maxSplits: 1).map { $0.trimmingCharacters(in: .whitespaces) }
            guard parts.count == 2, parts[0] == "ip_address" || parts[0] == "ipv6_address" else { return nil }
            return parts[1]
        }
    }
}
