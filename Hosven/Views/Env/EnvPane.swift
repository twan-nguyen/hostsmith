import SwiftUI

/// The environment-file pane: toolbar with the file name and full path, one
/// banner, a Table / Raw segmented control, and the key/value table. `screens/EnvTable.dc.html`.
struct EnvPane: View {
    enum Mode: String { case table, raw }

    let repoId: UUID
    let filePath: String?
    let select: (AppSidebarSelection) -> Void

    @Environment(EnvFileManager.self) private var envManager

    @State private var discoverResult: EnvDiscoverResult = .ok([])
    @State private var searchText = ""
    @State private var editingEntry: EnvEntry?
    @State private var showAddSheet = false
    @State private var deleteTarget: EnvEntry?
    @State private var loadError: String?
    @State private var isLoadingFile = false
    @State private var loadingPath: String?
    @State private var profileSheetMode: EnvProfileSheetMode?
    @State private var pendingApplyProfileId: UUID?
    @State private var showApplyConfirm = false
    @AppStorage("envViewMode") private var modeRaw: String = Mode.table.rawValue
    @State private var rawText = ""
    @State private var revealedEntryId: UUID?
    @State private var gitExposure: GitExposure = .safe
    /// Keys whose variant list is unfolded under their row.
    @State private var expandedKeys: Set<String> = []
    @State private var revealTask: Task<Void, Never>?
    @FocusState private var searchFocused: Bool
    @AppStorage("editorFontSize") private var editorFontSize: Double = 13

    private static let keyColumnWidth: CGFloat = 240
    private static let secretKeyPattern = try! NSRegularExpression(pattern: #"_(KEY|SECRET|TOKEN|PASSWORD|DSN|CREDENTIALS)$"#, options: .caseInsensitive)

    private var repo: EnvRepo? { envManager.repos.first { $0.id == repoId } }
    private var availableFiles: [String] { discoverResult.paths }
    private var mode: Mode {
        get { Mode(rawValue: modeRaw) ?? .table }
    }
    private var selectedFilePath: String? { envManager.selectedFilePath }
    private var currentFile: EnvFile? {
        guard let path = selectedFilePath else { return nil }
        return envManager.loadedFile(repoId: repoId, relativePath: path)
    }
    private var hasUnsavedChanges: Bool { currentFile?.hasUnsavedChanges == true }

    /// Entries of `.env.example` when the current file is not the example itself.
    private var exampleEntries: [EnvEntry]? {
        guard selectedFilePath != EnvFileManager.exampleFileName else { return nil }
        return envManager.exampleEntries(repoId: repoId)
    }

    /// Keys defined in `.env.example` that the current file lacks.
    private func missingKeys(in file: EnvFile) -> [EnvEntry] {
        guard let example = exampleEntries else { return [] }
        let present = Set(file.entries.filter { !$0.isBlankOrComment }.map(\.key))
        return example.filter { !$0.isBlankOrComment && !present.contains($0.key) }
    }

    private func addMissing(_ keys: [EnvEntry], to file: EnvFile) {
        for entry in keys {
            envManager.addEntry(repoId: repoId, fileId: file.id, key: entry.key, value: "", comment: "bắt buộc theo .env.example")
        }
    }

    var body: some View {
        if let repo {
            content(repo)
        } else {
            DSEmptyState(
                icon: "folder",
                title: "Chưa có repo",
                message: "Thêm thư mục của một project và Hosven sẽ tìm các file .env bên trong.",
                actionLabel: "Thêm repo",
                action: { EnvRepoPicker.pick(into: envManager) { select(.repo($0)) } }
            )
            .dsPaneContent()
        }
    }

    private func content(_ repo: EnvRepo) -> some View {
        VStack(spacing: 0) {
            toolbar(repo)
            VStack(spacing: DS.Size.paneInset) {
                banner(repo)
                mainContent(repo)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .dsPaneContent()
        }
        .task(id: "\(repoId)|\(filePath ?? "")") {
            envManager.selectedRepoId = repoId
            refreshFiles(preferred: filePath)
        }
        .onChange(of: selectedFilePath, initial: true) { _, path in
            envManager.watchSelectedFile(repoId: repoId, relativePath: path)
        }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            // Backstop for watcher events missed while Hosven was in the background.
            if let path = selectedFilePath { envManager.refreshFromDisk(repoId: repoId, relativePath: path) }
        }
        .onChange(of: currentFile?.diskContent) { _, _ in
            // A reload from disk replaced the entries; keep the raw editor on the new text.
            guard mode == .raw, let file = currentFile, !file.hasUnsavedChanges else { return }
            rawText = EnvParser.format(file.entries)
        }
        .task(id: "\(repoId)|\(selectedFilePath ?? "")") {
            gitExposure = .safe
            guard let path = selectedFilePath, path != EnvFileManager.exampleFileName else { return }
            gitExposure = await GitExposure.check(repoPath: repo.path, relativePath: path)
        }
        .onChange(of: envManager.pendingSearchQuery) { _, query in
            if let query, !query.isEmpty {
                searchText = query
                searchFocused = true
                envManager.pendingSearchQuery = nil
            }
        }
        .onChange(of: modeRaw) { old, new in syncModeChange(from: Mode(rawValue: old) ?? .table, to: Mode(rawValue: new) ?? .table) }
        .onChange(of: rawText) { _, text in
            guard mode == .raw, let file = currentFile, !file.hasUnsavedChanges else { return }
            // Text the pane set itself (file switch, reload, apply) matches the entries and is not an edit.
            guard text != EnvParser.format(file.entries) else { return }
            let fileId = file.id
            DispatchQueue.main.async { envManager.markFileDirty(repoId: repoId, fileId: fileId) }
        }
        .sheet(isPresented: $showAddSheet) {
            if let file = currentFile {
                EnvKeyFormSheet(mode: .add) { key, value, comment in
                    envManager.addEntry(repoId: repoId, fileId: file.id, key: key, value: value, comment: comment)
                }
            }
        }
        .sheet(item: $editingEntry) { entry in
            if let file = currentFile {
                EnvKeyFormSheet(mode: .edit(entry)) { key, value, comment in
                    envManager.updateEntry(repoId: repoId, fileId: file.id, entryId: entry.id, key: key, value: value, comment: comment)
                }
            }
        }
        .alert("Xoá biến?", isPresented: Binding(get: { deleteTarget != nil }, set: { if !$0 { deleteTarget = nil } }), presenting: deleteTarget) { entry in
            Button("Xoá biến", role: .destructive) {
                if let file = currentFile {
                    envManager.deleteEntry(repoId: repoId, fileId: file.id, entryId: entry.id)
                }
            }
            Button("Huỷ", role: .cancel) {}
        } message: { entry in
            Text("Xoá \(entry.key) khỏi \(selectedFilePath ?? "file"). Không ghi gì cho đến khi bạn nhấn Áp dụng.")
        }
        .sheet(item: $profileSheetMode) { mode in
            EnvProfileSheet(mode: mode).environment(envManager)
        }
        .alert("Áp dụng profile?", isPresented: $showApplyConfirm) {
            Button("Áp dụng profile", role: .destructive) { performApplyProfile() }
            Button("Huỷ", role: .cancel) { pendingApplyProfileId = nil }
        } message: {
            Text("File hiện tại có thay đổi chưa áp dụng. Hosven sao lưu trạng thái hiện tại trước khi ghi đè.")
        }
    }

    // MARK: - Toolbar

    private func toolbar(_ repo: EnvRepo) -> some View {
        let fileName = selectedFilePath ?? repo.name
        let path = selectedFilePath.map { (repo.path as NSString).appendingPathComponent($0) } ?? repo.path
        return ToolbarView(title: fileName, path: abbreviatedPath(path)) {
            if let file = currentFile {
                let missing = missingKeys(in: file).count
                if missing > 0 {
                    DSToolbarStatus(state: .warn, label: "\(missing) biến thiếu")
                } else if file.hasUnsavedChanges {
                    DSToolbarStatus(state: .warn, label: "Chưa áp dụng")
                } else {
                    DSToolbarStatus(state: .ok, label: "\(file.entries.filter { !$0.isBlankOrComment }.count) biến")
                }
            }
            DSSearchField(placeholder: "Tìm biến", text: $searchText, isFocused: $searchFocused)
                .accessibilityIdentifier("env-search-field")
            DSSegmentedControl([(Mode.table.rawValue, "Bảng"), (Mode.raw.rawValue, "Raw")], selection: $modeRaw)
                .disabled(currentFile == nil)
            Menu {
                    Button("Thêm biến") { showAddSheet = true }
                        .disabled(currentFile == nil || mode == .raw)
                    Divider()
                    if !repo.profiles.isEmpty {
                        Menu("Áp dụng profile") {
                            ForEach(repo.profiles) { profile in
                                Button("\(profile.name) · \(profile.files.count) file") { requestApplyProfile(profile.id) }
                            }
                        }
                    }
                    Button("Lưu trạng thái thành profile") { profileSheetMode = .save(repoId: repoId) }
                    Button("Quản lý profile") { profileSheetMode = .manage(repoId: repoId) }
                        .disabled(repo.profiles.isEmpty)
                    Divider()
                    Button("Thêm repo") { EnvRepoPicker.pick(into: envManager) { select(.repo($0)) } }
                    Button("Tải lại danh sách file") { refreshFiles(preferred: selectedFilePath) }
                    Divider()
                    if let terminal = ExternalTools.terminal {
                        Button("Mở repo trong \(terminal.name)") { ExternalTools.openInTerminal(repo.path) }
                    }
                    if let editor = ExternalTools.editor {
                        Button("Mở \(selectedFilePath ?? repo.name) bằng \(editor.name)") {
                            ExternalTools.openInEditor(selectedFilePath.map { (repo.path as NSString).appendingPathComponent($0) } ?? repo.path)
                        }
                    }
            } label: {
                Image(systemName: "ellipsis")
                    .font(.system(size: 14, weight: .medium))
                    .frame(width: 20)
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .buttonStyle(.dsBordered)
            .accessibilityLabel("Thêm")

            if let file = currentFile, file.hasUnsavedChanges {
                Button("Huỷ thay đổi") { discard(file) }
                    .buttonStyle(.dsDanger)
            }

            Button("Áp dụng") { apply() }
                .buttonStyle(.dsPrimary)
                .keyboardShortcut("s", modifiers: .command)
                .disabled(currentFile == nil)
                .accessibilityIdentifier("env-apply")
        }
    }

    private func addToGitignore(_ repo: EnvRepo, path: String) {
        do {
            try GitExposure.appendToGitignore(repoPath: repo.path, relativePath: path)
            gitExposure = .safe
            envManager.showToast("Đã thêm \(path) vào .gitignore", type: .success)
        } catch {
            envManager.showToast("Không ghi được .gitignore: \(error.localizedDescription)", type: .error)
        }
    }

    private func abbreviatedPath(_ path: String) -> String {
        let home = NSHomeDirectory()
        return path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }

    // MARK: - Banner

    @ViewBuilder
    private func banner(_ repo: EnvRepo) -> some View {
        if gitExposure == .tracked, let path = selectedFilePath {
            DSBanner(
                level: .danger,
                title: "\(path) đang được git theo dõi",
                detail: "Mọi commit sẽ mang theo giá trị trong file này. Chạy `git rm --cached \(path)` rồi thêm vào .gitignore.",
                glyph: "exclamationmark.triangle.fill",
                actions: [.init(label: "Thêm vào .gitignore") { addToGitignore(repo, path: path) }]
            )
        } else if let path = selectedFilePath, let change = envManager.externalChange(repoId: repoId, relativePath: path) {
            externalChangeBanner(change, path: path)
        } else if let toast = envManager.toast {
            DSBanner(level: toast.type.bannerLevel, title: toast.message)
        } else if gitExposure == .notIgnored, let path = selectedFilePath {
            DSBanner(
                level: .warning,
                title: "\(path) chưa có trong .gitignore",
                detail: "git add . sẽ đưa file này vào commit.",
                actions: [.init(label: "Thêm vào .gitignore") { addToGitignore(repo, path: path) }]
            )
        } else if case .repoMissing = discoverResult {
            DSBanner(
                level: .danger,
                title: "Không tìm thấy thư mục \(abbreviatedPath(repo.path))",
                detail: "Thư mục đã bị chuyển hoặc xoá. Xoá repo khỏi danh sách hoặc thêm lại từ vị trí mới.",
                actions: [.init(label: "Xoá khỏi danh sách") {
                    envManager.removeRepo(id: repoId)
                    select(.systemHosts)
                }]
            )
        } else if let file = currentFile, !missingKeys(in: file).isEmpty {
            let missing = missingKeys(in: file)
            DSBanner(
                level: .warning,
                title: "\(missing.count) biến trong .env.example thiếu ở đây",
                detail: missing.map(\.key).joined(separator: ", ") + " hiện thành hàng trống để không bị bỏ sót.",
                actions: [.init(label: "Thêm tất cả") { addMissing(missing, to: file) }]
            )
        } else if let file = currentFile, file.hasUnsavedChanges {
            DSBanner(
                level: .info,
                title: "Áp dụng sẽ ghi thay đổi vào \(file.relativePath)",
                detail: "Comment, thứ tự và dòng trống được giữ nguyên."
            )
        }
    }

    /// The file changed on disk while it held edits here, or disappeared. Nothing is
    /// written until the person picks one side.
    private func externalChangeBanner(_ change: EnvFileManager.ExternalChange, path: String) -> DSBanner {
        switch change {
        case .changed:
            return DSBanner(
                level: .warning,
                title: "\(path) vừa đổi bên ngoài Hosven",
                detail: "Bạn có thay đổi chưa áp dụng ở đây. Tải lại để lấy bản trên đĩa, hoặc ghi đè bằng bản của bạn.",
                actions: [
                    .init(label: "Tải lại từ đĩa") { reloadFromDisk(path) },
                    .init(label: "Ghi đè bằng bản của tôi") { apply(force: true) },
                ]
            )
        case .missing:
            return DSBanner(
                level: .warning,
                title: "\(path) đã bị xoá hoặc đổi tên bên ngoài Hosven",
                detail: "Nội dung vẫn còn trong Hosven. Tạo lại file để ghi nội dung này ra đĩa.",
                actions: [
                    .init(label: "Tạo lại file") { apply(force: true) },
                    .init(label: "Đóng") { envManager.dismissExternalChange(repoId: repoId, relativePath: path) },
                ]
            )
        }
    }

    private func reloadFromDisk(_ path: String) {
        do {
            try envManager.reloadFromDisk(repoId: repoId, relativePath: path)
            if mode == .raw, let file = currentFile { rawText = EnvParser.format(file.entries) }
        } catch {
            envManager.showToast("Không đọc được \(path): \(error.localizedDescription)", type: .error)
        }
    }

    // MARK: - Content

    @ViewBuilder
    private func mainContent(_ repo: EnvRepo) -> some View {
        if let error = loadError {
            DSEmptyState(icon: "xmark.octagon.fill", title: "Không đọc được file", message: error, actionLabel: "Thử lại", action: { refreshFiles(preferred: selectedFilePath) })
        } else if isLoadingFile && currentFile == nil {
            DSLoadingState(label: "Đang đọc \(loadingPath ?? "")")
        } else if let file = currentFile {
            switch mode {
            case .raw:
                CodeEditor(text: $rawText, language: .env, fontSize: editorFontSize, searchQuery: searchText)
                    .dsContentCard()
            case .table:
                entriesTable(file)
            }
        } else if case .envMissing = discoverResult {
            DSEmptyState(
                icon: "doc.text",
                title: "Không có file .env",
                message: "Repo \(repo.name) không có file .env hoặc .env.* ở thư mục gốc.",
                actionLabel: "Mở trong Finder",
                action: { NSWorkspace.shared.open(URL(fileURLWithPath: repo.path)) }
            )
        } else {
            DSEmptyState(icon: "doc.text", title: "Chọn một file .env", message: "Chọn file ở nhóm Repos trong thanh bên.")
        }
    }

    /// What the table draws: one entry, or one row standing for every definition
    /// of a key the file defines more than once.
    private enum TableItem: Identifiable {
        case entry(EnvEntry)
        case variants(key: String, entries: [EnvEntry])

        var id: String {
            switch self {
            case .entry(let entry): return entry.id.uuidString
            case .variants(let key, _): return "variants:" + key
            }
        }
    }

    private func tableItems(_ file: EnvFile) -> [TableItem] {
        var counts: [String: Int] = [:]
        for entry in file.entries where !entry.isBlankOrComment { counts[entry.key, default: 0] += 1 }
        var seen = Set<String>()
        var items: [TableItem] = []
        for entry in filteredEntries(file) {
            if entry.isBlankOrComment || counts[entry.key, default: 0] < 2 {
                items.append(.entry(entry))
            } else if seen.insert(entry.key).inserted {
                items.append(.variants(key: entry.key, entries: file.entries.filter { !$0.isBlankOrComment && $0.key == entry.key }))
            }
        }
        return items
    }

    private func entriesTable(_ file: EnvFile) -> some View {
        let rows = tableItems(file)
        let missing = searchText.isEmpty ? missingKeys(in: file) : missingKeys(in: file).filter { $0.key.lowercased().contains(searchText.lowercased()) }
        return Group {
            if rows.isEmpty && missing.isEmpty {
                DSEmptyState(
                    icon: searchText.isEmpty ? "doc.text" : "magnifyingglass",
                    title: searchText.isEmpty ? "File trống" : "Không có kết quả",
                    message: searchText.isEmpty ? "File này chưa có biến nào." : "Không có biến nào khớp \(searchText).",
                    actionLabel: searchText.isEmpty ? "Thêm biến" : nil,
                    action: searchText.isEmpty ? { showAddSheet = true } : nil
                )
            } else {
                VStack(spacing: 0) {
                    tableHeader
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(rows) { item in
                                switch item {
                                case .entry(let entry): keyValueRow(entry, file: file)
                                case .variants(let key, let entries):
                                    variantRow(key: key, variants: entries, file: file)
                                    if expandedKeys.contains(key) {
                                        ForEach(entries) { variant in
                                            variantChoiceRow(variant, key: key, file: file)
                                        }
                                    }
                                }
                            }
                            ForEach(missing) { entry in
                                missingRow(entry, file: file)
                            }
                        }
                    }
                    .background(DS.Colors.surfaceContent)
                }
                .dsContentCard()
            }
        }
    }

    private var tableHeader: some View {
        HStack(spacing: DS.Spacing.space6) {
            Text("Khoá")
                .frame(width: Self.keyColumnWidth, alignment: .leading)
            Text("Giá trị")
            Spacer()
        }
        .font(DS.Text.textMenuLabel)
        .foregroundStyle(DS.Colors.inkSecondary)
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space4)
        .background(DS.Colors.surfaceSunken)
        .dsHairline(.bottom)
    }

    @ViewBuilder
    private func keyValueRow(_ entry: EnvEntry, file: EnvFile) -> some View {
        if entry.isBlankOrComment {
            let line = (entry.rawLine ?? "").trimmingCharacters(in: .whitespaces)
            if line.isEmpty {
                Color.clear.frame(height: DS.Spacing.space5)
            } else {
                Text(line)
                    .font(DS.Text.monoBody)
                    .foregroundStyle(DS.Colors.codeComment)
                    .lineLimit(1)
                    .padding(.horizontal, DS.Spacing.space6)
                    .padding(.vertical, DS.Spacing.space3)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        } else {
            variableRow(entry, file: file)
        }
    }

    private func variableRow(_ entry: EnvEntry, file: EnvFile) -> some View {
        let secret = isSecret(entry.key)
        let revealed = revealedEntryId == entry.id
        return HStack(spacing: DS.Spacing.space6) {
            Group {
                Text(entry.key)
                    .font(DS.Text.monoStrong)
                    .foregroundStyle(DS.Colors.codeKey)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .frame(width: Self.keyColumnWidth, alignment: .leading)
                    .help(entry.key)

                Group {
                    if secret && !revealed {
                        Text(String(repeating: "•", count: max(8, min(entry.value.count, 24))))
                            .foregroundStyle(DS.Colors.inkTertiary)
                            .tracking(1.5)
                    } else {
                        Text(entry.value)
                            .foregroundStyle(DS.Colors.codeValue)
                    }
                }
                .font(DS.Text.monoBody)
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(maxWidth: .infinity, alignment: .leading)
                .help(entry.comment.isEmpty ? "" : entry.comment)

                HStack(spacing: DS.Spacing.space3) {
                    if secret {
                        Image(systemName: "key.fill")
                            .font(.system(size: 11))
                            .foregroundStyle(DS.Colors.inkSecondary)
                            .accessibilityLabel("Giá trị bí mật")
                        Button(revealed ? "Ẩn" : "Hiện") { toggleReveal(entry) }
                            .buttonStyle(.ds(.bordered, size: .sm))
                    }
                    DSSwitch(
                        isOn: Binding(get: { entry.isEnabled }, set: { _ in envManager.toggleEntry(repoId: repoId, fileId: file.id, entryId: entry.id) })
                    )
                    .accessibilityLabel("\(entry.key), \(entry.isEnabled ? "bật" : "tắt")")
                }
            }
        }
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space4)
        .frame(minHeight: DS.Size.rowHeight)
        .opacity(entry.isEnabled ? 1 : DS.Opacity.disabled)
        .contentShape(Rectangle())
        .dsRowHover()
        .dsHairline(.bottom)
        .onTapGesture(count: 2) { editingEntry = entry }
        .contextMenu {
            Button("Sửa") { editingEntry = entry }
            Button("Nhân đôi") {
                if let copy = envManager.duplicateEntry(repoId: repoId, fileId: file.id, entryId: entry.id) {
                    editingEntry = copy
                }
            }
            Divider()
            Button("Xoá", role: .destructive) { deleteTarget = entry }
        }
    }

    /// One row for a key defined several times: the enabled value, and a menu that
    /// enables one definition and disables the rest. Replaces a column of switches
    /// the person would otherwise have to flip in pairs.
    private func variantRow(key: String, variants: [EnvEntry], file: EnvFile) -> some View {
        let enabled = variants.filter(\.isEnabled)
        let active = enabled.first
        let secret = isSecret(key)
        let expanded = expandedKeys.contains(key)
        return HStack(spacing: DS.Spacing.space6) {
            Text(key)
                .font(DS.Text.monoStrong)
                .foregroundStyle(DS.Colors.codeKey)
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(width: Self.keyColumnWidth, alignment: .leading)
                .help(key)

            Group {
                if let active {
                    Text(secret ? masked(active.value) : active.value)
                        .foregroundStyle(DS.Colors.codeValue)
                } else {
                    Text("Tắt")
                        .foregroundStyle(DS.Colors.inkTertiary)
                }
            }
            .font(DS.Text.monoBody)
            .lineLimit(1)
            .truncationMode(.tail)
            .frame(maxWidth: .infinity, alignment: .leading)
            .help(active?.comment ?? "")

            if enabled.count > 1 {
                StatusDot(state: .warn, label: "\(enabled.count) đang bật")
            }

            HStack(spacing: DS.Spacing.space2) {
                Text("\(variants.count) giá trị")
                Image(systemName: expanded ? "chevron.up" : "chevron.down")
                    .font(.system(size: 10, weight: .semibold))
            }
            .font(DS.Text.textCaption)
            .foregroundStyle(DS.Colors.inkSecondary)
            .accessibilityLabel("\(key), \(variants.count) giá trị, \(active.map { "đang dùng " + $0.value } ?? "tắt")")
        }
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space4)
        .frame(minHeight: DS.Size.rowHeight)
        .opacity(active == nil ? DS.Opacity.disabled : 1)
        .contentShape(Rectangle())
        .dsRowHover()
        .dsHairline(.bottom)
        .onTapGesture(count: 2) { if let active { editingEntry = active } }
        .onTapGesture(count: 1) {
            if expanded { expandedKeys.remove(key) } else { expandedKeys.insert(key) }
        }
        .accessibilityAddTraits(.isButton)
        .accessibilityHint(expanded ? "Thu gọn danh sách giá trị" : "Mở danh sách giá trị")
        .contextMenu {
            if let active {
                Button("Sửa giá trị đang dùng") { editingEntry = active }
                Divider()
                Button("Xoá giá trị đang dùng", role: .destructive) { deleteTarget = active }
            }
        }
    }

    /// One selectable definition under an unfolded variant row. Click selects it;
    /// clicking the selected one turns the key off.
    private func variantChoiceRow(_ variant: EnvEntry, key: String, file: EnvFile) -> some View {
        let secret = isSecret(key)
        return HStack(spacing: DS.Spacing.space4) {
            Image(systemName: variant.isEnabled ? "largecircle.fill.circle" : "circle")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(variant.isEnabled ? DS.Colors.accent : DS.Colors.inkTertiary)
            Text(secret ? masked(variant.value) : variant.value)
                .font(DS.Text.monoBody)
                .foregroundStyle(variant.isEnabled ? DS.Colors.inkPrimary : DS.Colors.inkSecondary)
                .lineLimit(1)
                .truncationMode(.tail)
            if !variant.comment.isEmpty {
                Text(variant.comment)
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkTertiary)
                    .lineLimit(1)
            }
            Spacer(minLength: 0)
        }
        .padding(.leading, Self.keyColumnWidth + DS.Spacing.space6 * 2)
        .padding(.trailing, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space3)
        .frame(minHeight: DS.Size.controlHeightMd)
        .contentShape(Rectangle())
        .dsRowHover()
        .dsHairline(.bottom)
        .onTapGesture {
            envManager.selectVariant(repoId: repoId, fileId: file.id, key: key, entryId: variant.isEnabled ? nil : variant.id)
        }
        .contextMenu {
            Button("Sửa") { editingEntry = variant }
            Divider()
            Button("Xoá", role: .destructive) { deleteTarget = variant }
        }
        .accessibilityAddTraits(variant.isEnabled ? [.isButton, .isSelected] : [.isButton])
        .accessibilityLabel("\(variant.value)\(variant.comment.isEmpty ? "" : ", " + variant.comment)")
    }

    /// Bullets plus the last four characters, enough to tell two secrets apart.
    private func masked(_ value: String) -> String {
        String(repeating: "•", count: 6) + value.suffix(4)
    }

    /// A variable that exists in `.env.example` but not here: a real row with a
    /// `Missing` value and an `Add` button, so it cannot be overlooked.
    private func missingRow(_ entry: EnvEntry, file: EnvFile) -> some View {
        HStack(spacing: DS.Spacing.space6) {
            Text(entry.key)
                .font(DS.Text.monoStrong)
                .foregroundStyle(DS.Colors.codeKey)
                .lineLimit(1)
                .frame(width: Self.keyColumnWidth, alignment: .leading)
            Text("Thiếu — có trong .env.example")
                .font(DS.Text.monoBody)
                .foregroundStyle(DS.Colors.inkTertiary)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button("Thêm") { addMissing([entry], to: file) }
                .buttonStyle(.ds(.bordered, size: .sm))
        }
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space4)
        .frame(minHeight: DS.Size.rowHeight)
        .contentShape(Rectangle())
        .dsRowHover()
        .dsHairline(.bottom)
        .accessibilityLabel("\(entry.key), thiếu, có trong .env.example")
    }

    private func discard(_ file: EnvFile) {
        envManager.discardChanges(repoId: repoId, fileId: file.id)
        if let refreshed = envManager.loadedFile(repoId: repoId, relativePath: file.relativePath) {
            rawText = EnvParser.format(refreshed.entries)
        }
    }

    // MARK: - Secrets

    private func isSecret(_ key: String) -> Bool {
        Self.secretKeyPattern.firstMatch(in: key, range: NSRange(key.startIndex..., in: key)) != nil
    }

    /// Reveals one value and re-masks after 30 seconds.
    private func toggleReveal(_ entry: EnvEntry) {
        revealTask?.cancel()
        if revealedEntryId == entry.id {
            revealedEntryId = nil
            return
        }
        revealedEntryId = entry.id
        revealTask = Task { @MainActor in
            try? await Task.sleep(for: .seconds(30))
            if !Task.isCancelled, revealedEntryId == entry.id { revealedEntryId = nil }
        }
    }

    // MARK: - Profiles

    private func requestApplyProfile(_ id: UUID) {
        pendingApplyProfileId = id
        if hasUnsavedChanges { showApplyConfirm = true } else { performApplyProfile() }
    }

    private func performApplyProfile() {
        guard let id = pendingApplyProfileId else { return }
        pendingApplyProfileId = nil
        do {
            try envManager.applyProfile(repoId: repoId, profileId: id)
            refreshFiles(preferred: selectedFilePath)
            if mode == .raw, let file = currentFile { rawText = EnvParser.format(file.entries) }
        } catch {
            envManager.showToast("Không áp dụng được profile: \(error.localizedDescription)", type: .error)
        }
    }

    // MARK: - Files

    private func refreshFiles(preferred: String?) {
        guard let repo else { return }
        discoverResult = envManager.discoverEnvFiles(in: repo.path)
        loadError = nil
        if let preferred, availableFiles.contains(preferred) {
            selectFile(preferred)
        } else if let selected = selectedFilePath, availableFiles.contains(selected), filePath == nil {
            selectFile(selected)
        } else if let first = availableFiles.first {
            selectFile(first)
        } else {
            envManager.selectedFilePath = nil
        }
    }

    private func selectFile(_ path: String) {
        if mode == .raw, let oldFile = currentFile, oldFile.relativePath != path {
            let oldId = oldFile.id
            let snapshot = rawText
            DispatchQueue.main.async {
                envManager.replaceEntriesFromRawText(repoId: repoId, fileId: oldId, rawText: snapshot)
            }
        }
        envManager.selectedFilePath = path
        loadSelected(path)
        if mode == .raw, let file = currentFile, file.relativePath == path {
            rawText = EnvParser.format(file.entries)
        }
    }

    /// Leaving Raw folds the typed text back into entries; entering Raw formats them.
    private func syncModeChange(from oldMode: Mode, to newMode: Mode) {
        guard let file = currentFile else { return }
        if newMode == .raw {
            rawText = EnvParser.format(file.entries)
        } else if oldMode == .raw {
            let fileId = file.id
            let snapshot = rawText
            DispatchQueue.main.async {
                envManager.replaceEntriesFromRawText(repoId: repoId, fileId: fileId, rawText: snapshot)
            }
        }
    }

    private func loadSelected(_ path: String) {
        if envManager.loadedFile(repoId: repoId, relativePath: path) != nil {
            loadError = nil
            envManager.refreshFromDisk(repoId: repoId, relativePath: path)
            return
        }
        loadError = nil
        isLoadingFile = true
        loadingPath = path
        Task { @MainActor in
            do {
                let file = try await envManager.loadFileAsync(repoId: repoId, relativePath: path)
                guard loadingPath == path else { return }
                envManager.setLoadedFile(repoId: repoId, file: file)
                loadError = nil
                if mode == .raw, selectedFilePath == path { rawText = EnvParser.format(file.entries) }
            } catch {
                guard loadingPath == path else { return }
                loadError = error.localizedDescription
            }
            if loadingPath == path {
                isLoadingFile = false
                loadingPath = nil
            }
        }
    }

    /// `force` writes over a file that changed on disk since Hosven read it.
    private func apply(force: Bool = false) {
        guard let file = currentFile else { return }
        guard file.hasUnsavedChanges || force else {
            envManager.showToast("Không có gì để áp dụng", type: .info)
            return
        }
        do {
            if mode == .raw {
                try envManager.applyRawText(repoId: repoId, fileId: file.id, relativePath: file.relativePath, rawText: rawText, force: force)
                if let refreshed = envManager.loadedFile(repoId: repoId, relativePath: file.relativePath) {
                    rawText = EnvParser.format(refreshed.entries)
                }
            } else {
                try envManager.applyChanges(repoId: repoId, file: file, force: force)
            }
        } catch EnvError.externalConflict {
            // The conflict banner explains it and offers both ways out.
        } catch {
            envManager.showToast("Không ghi được \(file.relativePath): \(error.localizedDescription)", type: .error)
        }
    }

    private func filteredEntries(_ file: EnvFile) -> [EnvEntry] {
        guard !searchText.isEmpty else { return file.entries }
        let q = searchText.lowercased()
        return file.entries.filter { entry in
            if entry.isBlankOrComment { return (entry.rawLine ?? "").lowercased().contains(q) }
            return entry.key.lowercased().contains(q) || entry.value.lowercased().contains(q) || entry.comment.lowercased().contains(q)
        }
    }
}
