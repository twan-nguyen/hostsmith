import Foundation

/// Whether a file inside a repo can leak through git. Runs `git` off the main
/// actor; every failure to ask git counts as "not a git repo" so the UI stays quiet.
enum GitExposure: Equatable {
    /// git ignores the file, or the folder is not a git checkout.
    case safe
    /// The file is not ignored, so `git add .` would stage it.
    case notIgnored
    /// The file is already committed or staged.
    case tracked

    static func check(repoPath: String, relativePath: String) async -> GitExposure {
        await Task.detached(priority: .utility) {
            guard run(["rev-parse", "--is-inside-work-tree"], in: repoPath) == 0 else { return .safe }
            if run(["ls-files", "--error-unmatch", "--", relativePath], in: repoPath) == 0 { return .tracked }
            return run(["check-ignore", "-q", "--", relativePath], in: repoPath) == 0 ? .safe : .notIgnored
        }.value
    }

    /// Appends the file name to the repo's `.gitignore`, creating the file if needed.
    static func appendToGitignore(repoPath: String, relativePath: String) throws {
        let url = URL(fileURLWithPath: repoPath).appendingPathComponent(".gitignore")
        var content = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        if !content.isEmpty, !content.hasSuffix("\n") { content += "\n" }
        content += relativePath + "\n"
        try content.write(to: url, atomically: true, encoding: .utf8)
    }

    private static func run(_ arguments: [String], in directory: String) -> Int32 {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = arguments
        process.currentDirectoryURL = URL(fileURLWithPath: directory)
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
        } catch {
            return 128
        }
        process.waitUntilExit()
        return process.terminationStatus
    }
}
