import SwiftUI

/// The hosts pane: toolbar, one banner, a Blocks list of switches and the
/// `/etc/hosts` editor. `screens/Main.dc.html` and `guidelines/10-app-anatomy.md`.
/// The editor text is the working copy; block switches merge pending typing
/// into `entries` first, then regenerate the text.
struct HostsPane: View {
    /// When set, the Blocks section shows only this block and the editor jumps to it.
    let focusTag: String?

    @Environment(HostsFileManager.self) private var hostsManager
    @State private var rawText = ""
    @State private var searchText = ""
    @State private var jump: EditorJump?
    @State private var showAddSheet = false
    @State private var showImportSheet = false
    @State private var isSyncingFromEntries = false
    @FocusState private var searchFocused: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var blocks: [HostTag] {
        if let focusTag { return hostsManager.tags.filter { $0.name == focusTag } }
        return hostsManager.tags
    }

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            VStack(spacing: DS.Size.paneInset) {
                banner
                if !blocks.isEmpty {
                    VStack(alignment: .leading, spacing: DS.Spacing.space4) {
                        sectionLabel(focusTag == nil ? "Blocks" : "Block")
                        blocksCard
                    }
                }
                editorCard
            }
            .dsPaneContent()
        }
        .task { syncTextFromEntries() }
        .onChange(of: hostsManager.entries) { _, _ in syncTextFromEntries() }
        .onChange(of: focusTag) { _, _ in jumpToFocusedBlock() }
        .onChange(of: rawText) { _, _ in
            guard !isSyncingFromEntries, !hostsManager.hasUnsavedChanges else { return }
            hostsManager.markRawTextDirty()
        }
        .onChange(of: hostsManager.isSearchFocused) { _, focused in
            if focused {
                searchFocused = true
                hostsManager.isSearchFocused = false
            }
        }
        .onChange(of: hostsManager.pendingSearchQuery) { _, query in
            if let query, !query.isEmpty {
                searchText = query
                searchFocused = true
                hostsManager.pendingSearchQuery = nil
            }
        }
        .sheet(isPresented: $showAddSheet) {
            EntryFormSheet(hostsManager: hostsManager, mode: .add)
        }
        .sheet(isPresented: $showImportSheet) {
            ImportSheet(hostsManager: hostsManager)
        }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        ToolbarView(title: "Hosts hệ thống", path: "/etc/hosts") {
            status
            DSSearchField(placeholder: "Tìm entry", text: $searchText, isFocused: $searchFocused)
                .accessibilityIdentifier("hosts-search-field")
            if hostsManager.hasUnsavedChanges || rawText != hostsManager.generateHostsContent() {
                Button("Huỷ thay đổi") { discard() }
                    .buttonStyle(.dsDanger)
            }
            Menu {
                    Button("Thêm entry") { showAddSheet = true }
                    Button("Nhập từ văn bản") { showImportSheet = true }
                    Button("Sao chép vào clipboard") { hostsManager.exportToClipboard() }
                    Divider()
                    Button("Tạo bản sao lưu") { hostsManager.createBackup() }
                    Button("Tải lại từ đĩa") { hostsManager.loadHostsFile() }
                    if let editor = ExternalTools.editor {
                        Divider()
                        Button("Mở /etc/hosts bằng \(editor.name)") { ExternalTools.openInEditor("/etc/hosts") }
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

            Button(hostsManager.isApplying ? "Đang ghi" : "Áp dụng") { apply() }
                .buttonStyle(.dsPrimary)
                .keyboardShortcut("s", modifiers: .command)
                .disabled(hostsManager.isApplying)
                .accessibilityIdentifier("hosts-apply")
        }
    }

    @ViewBuilder
    private var status: some View {
        if hostsManager.hasUnsavedChanges {
            DSToolbarStatus(state: .warn, label: "\(hostsManager.pendingChangeCount) chưa áp dụng")
        } else {
            DSToolbarStatus(state: .ok, label: "Đã áp dụng")
        }
    }

    // MARK: - Banner

    @ViewBuilder
    private var banner: some View {
        if let toast = hostsManager.toast {
            DSBanner(level: toast.type.bannerLevel, title: toast.message)
        } else if hostsManager.externalChangeDetected {
            DSBanner(
                level: .warning,
                title: "/etc/hosts đã thay đổi bên ngoài Hosven",
                detail: "Tải lại để xem nội dung mới nhất. Thay đổi chưa áp dụng sẽ bị bỏ.",
                actions: [.init(label: "Tải lại") { hostsManager.loadHostsFile() }]
            )
        } else if let verification = hostsManager.verification, !hostsManager.hasUnsavedChanges {
            verificationBanner(verification)
        } else if hostsManager.hasUnsavedChanges {
            DSBanner(
                level: .info,
                title: "Áp dụng sẽ ghi \(hostsManager.pendingChangeCount) thay đổi vào /etc/hosts",
                detail: "Cần quyền quản trị một lần mỗi phiên. ⌘Z hoàn tác thay đổi trong trình soạn thảo.",
                glyph: "lock.fill"
            )
        }
    }

    private func verificationBanner(_ verification: HostsVerification) -> some View {
        let mismatches = verification.mismatches
        var actions: [DSBanner.Action] = []
        if ExternalTools.hasChrome {
            actions.append(.init(label: "Xoá cache DNS của Chrome") { ExternalTools.openChromeDNSPage() })
        }
        actions.append(.init(label: "Đóng") { hostsManager.verification = nil })
        if mismatches.isEmpty {
            return DSBanner(
                level: .success,
                title: "\(verification.results.count) hostname vừa ghi phân giải đúng",
                detail: "Chrome giữ cache DNS riêng, nên trình duyệt có thể vẫn trỏ về địa chỉ cũ thêm một lúc.",
                actions: actions
            )
        }
        let lines = mismatches.prefix(4).map { result in
            "\(result.hostname) → \(result.resolvedIPs.isEmpty ? "không phân giải" : result.resolvedIPs.joined(separator: ", ")) (mong đợi \(result.expectedIP))"
        }
        return DSBanner(
            level: .warning,
            title: "\(mismatches.count) trong \(verification.results.count) hostname vừa ghi chưa phân giải như /etc/hosts",
            detail: lines.joined(separator: "\n") + (mismatches.count > 4 ? "\n…" : ""),
            actions: actions
        )
    }

    // MARK: - Blocks

    private var blocksCard: some View {
        VStack(spacing: 0) {
            ForEach(Array(blocks.enumerated()), id: \.element.id) { index, tag in
                let count = hostsManager.tagEntryCount(name: tag.name)
                let state = hostsManager.tagState(name: tag.name)
                DSListRow(
                    glyph: "network",
                    title: tag.name,
                    sublabel: blockSublabel(tag.name, count: count),
                    showsHairline: index < blocks.count - 1
                ) {
                    DSSwitch(
                        isOn: Binding(
                            get: { state != .allDisabled },
                            set: { setBlock(tag.name, enabled: $0) }
                        ),
                        label: nil,
                        disabled: count == 0
                    )
                    .accessibilityLabel("\(tag.name), \(state == .allDisabled ? "tắt" : "bật")")
                    .accessibilityIdentifier("profile-toggle-\(tag.name)")
                }
                .contentShape(Rectangle())
                .onTapGesture { jump(toTag: tag.name) }
            }
        }
        .dsContentCard(radius: DS.Radius.md)
    }

    private func blockSublabel(_ name: String, count: Int) -> String {
        let entries = hostsManager.entries.filter { $0.tag == name }
        guard let first = entries.first else { return "0 entry" }
        let ips = Set(entries.map(\.ip))
        let target = ips.count == 1 ? first.ip : "\(ips.count) địa chỉ"
        return "\(count) entry · \(target) → \(first.hostname)"
    }

    // MARK: - Editor

    private var editorCard: some View {
        CodeEditor(text: $rawText, language: .hosts, fontSize: editorFontSize, searchQuery: searchText, jump: jump)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .dsContentCard()
            .accessibilityIdentifier("hosts-editor")
    }

    @AppStorage("editorFontSize") private var editorFontSize: Double = 13

    private func sectionLabel(_ text: String) -> some View {
        Text(text)
            .font(DS.Text.textMenuLabel)
            .foregroundStyle(DS.Colors.inkSecondary)
            .padding(.horizontal, DS.Spacing.space1)
    }

    // MARK: - Actions

    private func setBlock(_ name: String, enabled: Bool) {
        mergePendingTyping()
        hostsManager.setTagEnabled(name: name, enabled: enabled)
    }

    /// Fold what the person typed into `entries` so a block toggle does not discard it.
    private func mergePendingTyping() {
        let generated = hostsManager.generateHostsContent()
        guard rawText != generated else { return }
        hostsManager.replaceContentFromRawText(rawText)
    }

    private func syncTextFromEntries() {
        isSyncingFromEntries = true
        rawText = hostsManager.generateHostsContent()
        DispatchQueue.main.async {
            isSyncingFromEntries = false
            jumpToFocusedBlock()
        }
    }

    private func jumpToFocusedBlock() {
        guard let focusTag else { return }
        jump(toTag: focusTag)
    }

    private func jump(toTag name: String) {
        let marker = "## [tag:\(name)]"
        let lines = rawText.split(separator: "\n", omittingEmptySubsequences: false)
        guard let index = lines.firstIndex(where: { $0.trimmingCharacters(in: .whitespaces) == marker }) else { return }
        jump = EditorJump(line: index)
    }

    private func discard() {
        hostsManager.discardChanges()
        syncTextFromEntries()
    }

    private func apply() {
        guard hostsManager.hasUnsavedChanges || rawText != hostsManager.generateHostsContent() else {
            hostsManager.showToast("Không có gì để áp dụng", type: .info)
            return
        }
        hostsManager.applyRawText(rawText)
    }
}

extension ToastType {
    var bannerLevel: DSBanner.Level {
        switch self {
        case .success: return .success
        case .error: return .danger
        case .info: return .info
        }
    }
}
