import Foundation
import SwiftUI

@Observable
@MainActor
final class EnvFileManager {
    var repos: [EnvRepo] = []
    var selectedRepoId: UUID?
    var selectedFilePath: String?
    var loadedFiles: [UUID: [EnvFile]] = [:]
    /// Pending query pushed from the ⌘K palette. EnvFilePane observes and applies.
    var pendingSearchQuery: String?
    var toast: ToastMessage?

    private let storageURL: URL

    init(storageURL: URL? = nil) {
        self.storageURL = storageURL ?? Self.defaultStorageURL()
        loadPersistedState()
    }

    // MARK: - Storage

    /// `~/.config/hosven/env.json`; the Application Support file older versions
    /// wrote is copied there once so nothing is lost on upgrade.
    private static func defaultStorageURL() -> URL {
        let url = HosvenConfig.envURL
        let fm = FileManager.default
        if !fm.fileExists(atPath: url.path), fm.fileExists(atPath: HosvenConfig.legacyEnvURL.path) {
            try? HosvenConfig.ensureDirectory(for: url)
            try? fm.copyItem(at: HosvenConfig.legacyEnvURL, to: url)
        }
        return url
    }

    private struct PersistedState: Codable {
        var version: Int
        var repos: [EnvRepo]
    }

    private func loadPersistedState() {
        guard FileManager.default.fileExists(atPath: storageURL.path) else { return }
        do {
            let data = try Data(contentsOf: storageURL)
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            let state = try decoder.decode(PersistedState.self, from: data)
            self.repos = state.repos
            self.selectedRepoId = state.repos.first?.id
        } catch {
            showToast("Không đọc được cấu hình env, Hosven khởi tạo lại danh sách repo", type: .error)
        }
    }

    private func persist() {
        let state = PersistedState(version: 1, repos: repos)
        do {
            try HosvenConfig.ensureDirectory(for: storageURL)
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            let data = try encoder.encode(state)
            try data.write(to: storageURL, options: .atomic)
        } catch {
            showToast("Không lưu được cấu hình: \(error.localizedDescription)", type: .error)
        }
    }

    // MARK: - Repo Management

    func addRepo(at url: URL) throws {
        let path = url.path
        if repos.contains(where: { $0.path == path }) {
            throw EnvError.duplicateRepoPath
        }
        let repo = EnvRepo(name: url.lastPathComponent, path: path)
        repos.append(repo)
        selectedRepoId = repo.id
        persist()
        showToast("Đã thêm repo \(repo.name)", type: .success)
    }

    func removeRepo(id: UUID) {
        guard let idx = repos.firstIndex(where: { $0.id == id }) else { return }
        let name = repos[idx].name
        repos.remove(at: idx)
        loadedFiles[id] = nil
        if selectedRepoId == id {
            selectedRepoId = repos.first?.id
        }
        persist()
        showToast("Đã xoá repo \(name) khỏi danh sách", type: .info)
    }

    func renameRepo(id: UUID, newName: String) throws {
        let trimmed = newName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw EnvError.invalidName }
        guard let idx = repos.firstIndex(where: { $0.id == id }) else {
            throw EnvError.repoNotFound
        }
        repos[idx].name = trimmed
        persist()
    }

    func repoPathExists(_ repo: EnvRepo) -> Bool {
        FileManager.default.fileExists(atPath: repo.path)
    }

    // MARK: - File Discovery

    /// `.env`, its local override, and the example file the design diffs against.
    private static let supportedEnvFiles = [".env", ".env.local", ".env.example"]
    static let exampleFileName = ".env.example"

    func discoverEnvFiles(in repoPath: String) -> EnvDiscoverResult {
        let fm = FileManager.default
        var repoIsDir: ObjCBool = false
        // Repo path phải tồn tại và là directory — nếu không, báo rõ để user biết folder bị xoá/đổi tên
        guard fm.fileExists(atPath: repoPath, isDirectory: &repoIsDir), repoIsDir.boolValue else {
            return .repoMissing
        }
        let found = Self.supportedEnvFiles.filter { name in
            let full = (repoPath as NSString).appendingPathComponent(name)
            var isDir: ObjCBool = false
            return fm.fileExists(atPath: full, isDirectory: &isDir) && !isDir.boolValue
        }
        return found.isEmpty ? .envMissing : .ok(found)
    }

    // MARK: - File Load / Save

    func loadFile(repoId: UUID, relativePath: String) throws -> EnvFile {
        guard let repo = repos.first(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        try validateRelativePath(relativePath)

        let url = URL(fileURLWithPath: repo.path).appendingPathComponent(relativePath)
        let content: String
        do {
            content = try String(contentsOf: url, encoding: .utf8)
        } catch {
            throw EnvError.fileReadFailed(error.localizedDescription)
        }
        let entries = EnvParser.parse(content)
        return EnvFile(relativePath: relativePath, entries: entries)
    }

    /// Async variant: chạy disk read + parse trên background để tránh block main actor khi lần đầu load.
    /// Trả về `EnvFile` để caller tự gọi `setLoadedFile`. Validate path đồng bộ trước khi rời main actor.
    func loadFileAsync(repoId: UUID, relativePath: String) async throws -> EnvFile {
        guard let repo = repos.first(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        try validateRelativePath(relativePath)
        let repoPath = repo.path

        return try await Task.detached(priority: .userInitiated) {
            let url = URL(fileURLWithPath: repoPath).appendingPathComponent(relativePath)
            let content: String
            do {
                content = try String(contentsOf: url, encoding: .utf8)
            } catch {
                throw EnvError.fileReadFailed(error.localizedDescription)
            }
            let entries = EnvParser.parse(content)
            return EnvFile(relativePath: relativePath, entries: entries)
        }.value
    }

    /// Convenience for the StatusBar Apply button: applies whichever env file is currently
    /// selected (selectedRepoId + selectedFilePath). No-op when nothing is selected or no
    /// pending changes.
    func applyCurrentSelection() {
        guard
            let repoId = selectedRepoId,
            let path = selectedFilePath,
            let file = loadedFile(repoId: repoId, relativePath: path),
            file.hasUnsavedChanges
        else { return }
        do {
            try applyChanges(repoId: repoId, file: file)
        } catch {
            showToast("Không áp dụng được: \(error.localizedDescription)", type: .error)
        }
    }

    func applyChanges(repoId: UUID, file: EnvFile) throws {
        guard let repo = repos.first(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        try validateRelativePath(file.relativePath)

        let targetURL = URL(fileURLWithPath: repo.path).appendingPathComponent(file.relativePath)
        let content = EnvParser.format(file.entries)
        try writeContent(content, to: targetURL)

        updateLoadedFile(repoId: repoId, fileId: file.id) { cached in
            cached.entries = file.entries
            cached.pristineEntries = file.entries
            cached.rawTextDirty = false
        }

        showToast("Đã ghi \(file.entries.filter { !$0.isBlankOrComment }.count) biến vào \(file.relativePath)", type: .success)
    }

    // MARK: - Raw text editing

    /// Parse raw text and replace cached entries for a file. Dirty state derives
    /// from comparison against pristineEntries — no manual flag toggle needed.
    /// Used when the user switches from raw-text mode back to form mode.
    func replaceEntriesFromRawText(repoId: UUID, fileId: UUID, rawText: String) {
        let newEntries = EnvParser.parse(rawText)
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            file.entries = newEntries
            file.rawTextDirty = false
        }
    }

    /// Mark a cached file as mid-edit in raw-text mode. Called per keystroke so
    /// the dirty indicator reflects pending raw text without paying parse cost.
    func markFileDirty(repoId: UUID, fileId: UUID) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            file.rawTextDirty = true
        }
    }

    /// Write raw text directly to disk (preserving user formatting verbatim), then
    /// re-parse so the in-memory entry cache stays in sync with disk.
    func applyRawText(
        repoId: UUID,
        fileId: UUID,
        relativePath: String,
        rawText: String
    ) throws {
        guard let repo = repos.first(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        try validateRelativePath(relativePath)

        let content = rawText.hasSuffix("\n") ? rawText : rawText + "\n"
        let targetURL = URL(fileURLWithPath: repo.path).appendingPathComponent(relativePath)
        try writeContent(content, to: targetURL)

        let reparsed = EnvParser.parse(content)
        updateLoadedFile(repoId: repoId, fileId: fileId) { cached in
            cached.entries = reparsed
            cached.pristineEntries = reparsed
            cached.rawTextDirty = false
        }

        showToast("Đã ghi \(reparsed.filter { !$0.isBlankOrComment }.count) biến vào \(relativePath)", type: .success)
    }

    /// Atomic write via temp file + replaceItemAt, preserves POSIX perms. Shared by
    /// both form-mode and raw-mode apply paths.
    private func writeContent(_ content: String, to targetURL: URL) throws {
        let tempURL = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("env_\(UUID().uuidString)")

        do {
            try content.write(to: tempURL, atomically: true, encoding: .utf8)
        } catch {
            throw EnvError.fileWriteFailed(error.localizedDescription)
        }

        let existingAttrs = try? FileManager.default.attributesOfItem(atPath: targetURL.path)
        let targetExisted = FileManager.default.fileExists(atPath: targetURL.path)

        do {
            if targetExisted {
                _ = try FileManager.default.replaceItemAt(targetURL, withItemAt: tempURL)
            } else {
                try FileManager.default.moveItem(at: tempURL, to: targetURL)
            }
        } catch {
            try? FileManager.default.removeItem(at: tempURL)
            throw EnvError.fileWriteFailed(error.localizedDescription)
        }

        if let perms = existingAttrs?[.posixPermissions] {
            try? FileManager.default.setAttributes(
                [.posixPermissions: perms],
                ofItemAtPath: targetURL.path
            )
        }
    }

    private func validateRelativePath(_ relativePath: String) throws {
        // Reject path traversal and absolute paths
        if relativePath.contains("..") || relativePath.hasPrefix("/") {
            throw EnvError.invalidPath
        }
    }

    /// Entries of the repo's `.env.example`, read from disk, or nil when absent.
    func exampleEntries(repoId: UUID) -> [EnvEntry]? {
        guard let repo = repos.first(where: { $0.id == repoId }) else { return nil }
        let url = URL(fileURLWithPath: repo.path).appendingPathComponent(Self.exampleFileName)
        guard let content = try? String(contentsOf: url, encoding: .utf8) else { return nil }
        return EnvParser.parse(content)
    }

    /// Drops pending edits on a file, returning it to the last loaded or applied state.
    func discardChanges(repoId: UUID, fileId: UUID) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            file.entries = file.pristineEntries
            file.rawTextDirty = false
        }
    }

    // MARK: - In-memory cache

    func setLoadedFile(repoId: UUID, file: EnvFile) {
        var files = loadedFiles[repoId] ?? []
        if let idx = files.firstIndex(where: { $0.relativePath == file.relativePath }) {
            files[idx] = file
        } else {
            files.append(file)
        }
        loadedFiles[repoId] = files
    }

    func loadedFile(repoId: UUID, relativePath: String) -> EnvFile? {
        loadedFiles[repoId]?.first(where: { $0.relativePath == relativePath })
    }

    // MARK: - Entry CRUD (mutates cached EnvFile)

    func addEntry(repoId: UUID, fileId: UUID, key: String, value: String, comment: String) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            let entry = EnvEntry(key: key, value: value, comment: comment, isEnabled: true)
            file.entries.append(entry)
        }
    }

    func updateEntry(
        repoId: UUID,
        fileId: UUID,
        entryId: UUID,
        key: String,
        value: String,
        comment: String
    ) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            guard let idx = file.entries.firstIndex(where: { $0.id == entryId }) else { return }
            file.entries[idx].key = key
            file.entries[idx].value = value
            file.entries[idx].comment = comment
        }
    }

    func deleteEntry(repoId: UUID, fileId: UUID, entryId: UUID) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            let before = file.entries.count
            file.entries.removeAll { $0.id == entryId }
            if file.entries.count != before {
            }
        }
    }

    @discardableResult
    func duplicateEntry(repoId: UUID, fileId: UUID, entryId: UUID) -> EnvEntry? {
        var inserted: EnvEntry?
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            guard let idx = file.entries.firstIndex(where: { $0.id == entryId }) else { return }
            let source = file.entries[idx]
            let copy = EnvEntry(
                key: source.key,
                value: source.value,
                comment: source.comment,
                isEnabled: source.isEnabled,
                isBlankOrComment: source.isBlankOrComment,
                rawLine: source.isBlankOrComment ? source.rawLine : nil
            )
            file.entries.insert(copy, at: idx + 1)
            inserted = copy
        }
        return inserted
    }

    /// Makes `entryId` the one enabled definition of `key`; `nil` disables every
    /// definition. This is how a file that keeps several values for one key switches.
    func selectVariant(repoId: UUID, fileId: UUID, key: String, entryId: UUID?) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            for idx in file.entries.indices where !file.entries[idx].isBlankOrComment && file.entries[idx].key == key {
                file.entries[idx].isEnabled = file.entries[idx].id == entryId
            }
        }
    }

    func toggleEntry(repoId: UUID, fileId: UUID, entryId: UUID) {
        updateLoadedFile(repoId: repoId, fileId: fileId) { file in
            guard let idx = file.entries.firstIndex(where: { $0.id == entryId }) else { return }
            file.entries[idx].isEnabled.toggle()
        }
    }

    private func updateLoadedFile(
        repoId: UUID,
        fileId: UUID,
        _ mutation: (inout EnvFile) -> Void
    ) {
        guard var files = loadedFiles[repoId],
              let idx = files.firstIndex(where: { $0.id == fileId }) else { return }
        mutation(&files[idx])
        loadedFiles[repoId] = files
    }

    // MARK: - Profiles

    @discardableResult
    func saveCurrentAsProfile(repoId: UUID, profileName: String) throws -> EnvProfile {
        let trimmed = profileName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw EnvError.invalidName }
        guard let repoIdx = repos.firstIndex(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        if repos[repoIdx].profiles.contains(where: { $0.name.lowercased() == trimmed.lowercased() }) {
            throw EnvError.duplicateProfileName
        }

        let repoPath = repos[repoIdx].path
        let discoveredPaths = discoverEnvFiles(in: repoPath).paths
        var snapshots: [ProfileFileSnapshot] = []
        for relative in discoveredPaths {
            let url = URL(fileURLWithPath: repoPath).appendingPathComponent(relative)
            guard let content = try? String(contentsOf: url, encoding: .utf8) else { continue }
            snapshots.append(ProfileFileSnapshot(relativePath: relative, content: content))
        }

        let profile = EnvProfile(name: trimmed, files: snapshots)
        repos[repoIdx].profiles.append(profile)
        persist()
        showToast("Đã lưu profile \(trimmed) với \(snapshots.count) file", type: .success)
        return profile
    }

    func applyProfile(repoId: UUID, profileId: UUID, autoBackup: Bool = true) throws {
        guard let repoIdx = repos.firstIndex(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        guard let profile = repos[repoIdx].profiles.first(where: { $0.id == profileId }) else {
            throw EnvError.profileNotFound
        }

        if autoBackup {
            let backupName = "before-\(profile.name)-\(Self.shortTimestamp())"
            // Ignore errors (e.g. dup name) — best effort
            _ = try? saveCurrentAsProfile(repoId: repoId, profileName: backupName)
        }

        let repoPath = repos[repoIdx].path
        for snapshot in profile.files {
            try validateRelativePath(snapshot.relativePath)
            let targetURL = URL(fileURLWithPath: repoPath).appendingPathComponent(snapshot.relativePath)
            let tempURL = URL(fileURLWithPath: NSTemporaryDirectory())
                .appendingPathComponent("env_apply_\(UUID().uuidString)")

            do {
                try snapshot.content.write(to: tempURL, atomically: true, encoding: .utf8)
            } catch {
                throw EnvError.fileWriteFailed(error.localizedDescription)
            }

            let existingAttrs = try? FileManager.default.attributesOfItem(atPath: targetURL.path)
            let targetExisted = FileManager.default.fileExists(atPath: targetURL.path)

            do {
                if targetExisted {
                    _ = try FileManager.default.replaceItemAt(targetURL, withItemAt: tempURL)
                } else {
                    try FileManager.default.moveItem(at: tempURL, to: targetURL)
                }
            } catch {
                try? FileManager.default.removeItem(at: tempURL)
                throw EnvError.fileWriteFailed(error.localizedDescription)
            }

            if let perms = existingAttrs?[.posixPermissions] {
                try? FileManager.default.setAttributes(
                    [.posixPermissions: perms],
                    ofItemAtPath: targetURL.path
                )
            }
        }

        // Invalidate cached files so next read pulls from disk
        loadedFiles[repoId] = nil
        showToast("Đã áp dụng profile \(profile.name)", type: .success)
    }

    func deleteProfile(repoId: UUID, profileId: UUID) {
        guard let repoIdx = repos.firstIndex(where: { $0.id == repoId }) else { return }
        let name = repos[repoIdx].profiles.first(where: { $0.id == profileId })?.name
        repos[repoIdx].profiles.removeAll { $0.id == profileId }
        persist()
        if let name = name {
            showToast("Đã xoá profile \(name)", type: .info)
        }
    }

    func renameProfile(repoId: UUID, profileId: UUID, newName: String) throws {
        let trimmed = newName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw EnvError.invalidName }
        guard let repoIdx = repos.firstIndex(where: { $0.id == repoId }) else {
            throw EnvError.repoNotFound
        }
        if repos[repoIdx].profiles.contains(where: {
            $0.name.lowercased() == trimmed.lowercased() && $0.id != profileId
        }) {
            throw EnvError.duplicateProfileName
        }
        guard let profileIdx = repos[repoIdx].profiles.firstIndex(where: { $0.id == profileId }) else {
            throw EnvError.profileNotFound
        }
        repos[repoIdx].profiles[profileIdx].name = trimmed
        persist()
    }

    private static func shortTimestamp() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyyMMdd-HHmmss"
        return formatter.string(from: Date())
    }

    // MARK: - Toast

    func showToast(_ message: String, type: ToastType) {
        toast = ToastMessage(message: message, type: type)
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
            if self?.toast?.message == message {
                self?.toast = nil
            }
        }
    }
}
