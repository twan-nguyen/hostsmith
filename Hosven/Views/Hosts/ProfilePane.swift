import SwiftUI

/// A profile, listed one entry per row. `screens/Profiles.dc.html`.
/// An environment is switched to from the toolbar, which writes `/etc/hosts` at once.
/// A block has one switch for all its entries. Entry edits wait for Áp dụng.
struct ProfilePane: View {
    let profileId: UUID
    let select: (AppSidebarSelection) -> Void

    @Environment(HostsFileManager.self) private var hostsManager
    @State private var profileSheet: ProfileFormSheet.Mode?
    @State private var entrySheet: EntryFormMode?
    @State private var deleteEntryTarget: HostEntry?
    @State private var confirmDelete = false

    private var profile: Profile? {
        hostsManager.profiles.first { $0.id == profileId }
    }

    var body: some View {
        if let profile {
            content(profile)
        } else {
            DSEmptyState(icon: "square.stack.3d.up", title: "Không tìm thấy profile")
                .dsPaneContent()
        }
    }

    private func content(_ profile: Profile) -> some View {
        let entries = hostsManager.entries.filter { $0.tag == profile.name }
        let isActive = profile.isExclusive
            ? hostsManager.activeProfileID == profile.id
            : hostsManager.tagState(name: profile.name) != .allDisabled
        let enabledCount = entries.filter(\.isEnabled).count
        let shortcut = profile.isExclusive
            ? profile.shortcutNumber.flatMap { (1...9).contains($0) ? " · ⌘\($0)" : nil } ?? ""
            : ""
        let kind = profile.isExclusive ? "Môi trường" : "Block"
        let pending = hostsManager.hasUnsavedChanges

        return VStack(spacing: 0) {
            ToolbarView(title: profile.name, path: "\(kind) · \(entries.count) entry\(shortcut)") {
                DSToolbarStatus(state: isActive ? .ok : .idle, label: isActive ? "Đang bật" : "Đang tắt")
                if profile.color.isEnvironment {
                    DSChip(text: profile.color.displayName, color: profile.color)
                        .padding(.trailing, DS.Spacing.space1)
                }
                overflowMenu(profile)
                if profile.isExclusive {
                    if isActive {
                        Button("Tắt") { hostsManager.switchEnvironment(to: nil, policy: .askIfPending) }
                            .buttonStyle(.dsBordered)
                            .disabled(hostsManager.isApplying)
                    } else {
                        Button("Kích hoạt") { hostsManager.switchEnvironment(to: profile.id, policy: .askIfPending) }
                            .buttonStyle(pending ? .dsBordered : .dsPrimary)
                            .disabled(entries.isEmpty || hostsManager.isApplying)
                            .help(entries.isEmpty ? "Thêm entry để kích hoạt môi trường này" : "")
                            .accessibilityIdentifier("profile-activate")
                    }
                } else {
                    DSSwitch(
                        isOn: Binding(
                            get: { isActive },
                            set: { hostsManager.setTagEnabled(name: profile.name, enabled: $0) }
                        ),
                        disabled: entries.isEmpty
                    )
                    .accessibilityLabel("\(profile.name), \(isActive ? "bật" : "tắt")")
                    .accessibilityIdentifier("profile-block-toggle")
                }
                if pending {
                    Button(hostsManager.isApplying ? "Đang ghi" : "Áp dụng") { hostsManager.applyChanges() }
                        .buttonStyle(.dsPrimary)
                        .keyboardShortcut("s", modifiers: .command)
                        .disabled(hostsManager.isApplying)
                        .accessibilityIdentifier("profile-apply")
                }
            }

            VStack(spacing: DS.Size.paneInset) {
                banner(profile, entries: entries, isActive: isActive, enabledCount: enabledCount)

                VStack(alignment: .leading, spacing: DS.Spacing.space4) {
                    sectionHeader(profile, entries: entries, enabledCount: enabledCount)
                    if entries.isEmpty {
                        DSEmptyState(
                            icon: "network",
                            title: "Chưa có entry",
                            message: "Thêm hostname vào \(profile.name) để profile này có tác dụng khi kích hoạt.",
                            actionLabel: "Thêm entry",
                            action: { entrySheet = .add }
                        )
                    } else {
                        entriesCard(profile, entries: entries)
                    }
                }
                .frame(maxHeight: .infinity, alignment: .top)
            }
            .dsPaneContent()
        }
        .sheet(item: $profileSheet) { ProfileFormSheet(mode: $0) }
        .sheet(item: $entrySheet) { mode in
            EntryFormSheet(hostsManager: hostsManager, mode: mode, initialTag: profile.name)
        }
        .alert("Xoá profile?", isPresented: $confirmDelete) {
            Button("Xoá profile", role: .destructive) {
                hostsManager.removeProfile(id: profile.id)
                select(.systemHosts)
            }
            Button("Huỷ", role: .cancel) {}
        } message: {
            Text("Xoá metadata của profile \(profile.name). Các entry trong /etc/hosts được giữ nguyên.")
        }
        .alert(
            "Xoá entry?",
            isPresented: Binding(get: { deleteEntryTarget != nil }, set: { if !$0 { deleteEntryTarget = nil } }),
            presenting: deleteEntryTarget
        ) { entry in
            Button("Xoá entry", role: .destructive) { hostsManager.deleteEntry(id: entry.id) }
            Button("Huỷ", role: .cancel) { deleteEntryTarget = nil }
        } message: { entry in
            Text("Xoá \(entry.hostname) khỏi \(profile.name). Không ghi gì vào /etc/hosts cho đến khi bạn nhấn Áp dụng.")
        }
    }

    // MARK: - Toolbar

    private func overflowMenu(_ profile: Profile) -> some View {
        Menu {
            Button("Thêm entry") { entrySheet = .add }
            Button("Sửa") { profileSheet = .rename(profile) }
            Button(profile.isExclusive ? "Chuyển thành block" : "Chuyển thành môi trường") {
                hostsManager.updateProfileExclusive(id: profile.id, !profile.isExclusive)
            }
            Button("Hiện trong Hosts hệ thống") { select(.hostsTag(profile.name)) }
                .disabled(hostsManager.tagEntryCount(name: profile.name) == 0)
            Divider()
            Button("Xoá profile", role: .destructive) { confirmDelete = true }
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
    }

    // MARK: - Banner

    @ViewBuilder
    private func banner(_ profile: Profile, entries: [HostEntry], isActive: Bool, enabledCount: Int) -> some View {
        if let toast = hostsManager.toast {
            DSBanner(level: toast.type.bannerLevel, title: toast.message)
        } else if case .conflict(let ids) = hostsManager.activeProfileState, profile.isExclusive {
            EnvironmentConflictBanner(profileIds: ids)
        } else if hostsManager.hasUnsavedChanges {
            DSBanner(
                level: .info,
                title: "\(hostsManager.pendingChangeCount) thay đổi chưa ghi vào /etc/hosts",
                detail: "Cần quyền quản trị một lần mỗi phiên.",
                glyph: "lock.fill"
            )
        } else if isActive {
            DSBanner(level: .success, title: "\(profile.name) đang bật với \(enabledCount) trong \(entries.count) entry")
        } else if entries.isEmpty || !profile.isExclusive {
            EmptyView()
        } else {
            DSBanner(
                level: .info,
                title: activationSummary(profile),
                detail: "Cần quyền quản trị một lần mỗi phiên.",
                glyph: "lock.fill"
            )
        }
    }

    /// "Kích hoạt sẽ bật 2 entry của Staging, tắt 3 entry của Local và ghi vào /etc/hosts"
    private func activationSummary(_ profile: Profile) -> String {
        let preview = hostsManager.switchPreview(to: profile.id)
        var parts: [String] = []
        if preview.enable > 0 { parts.append("bật \(preview.enable) entry của \(profile.name)") }
        parts += preview.disable.map { "tắt \($0.count) entry của \($0.name)" }
        if parts.isEmpty { return "Kích hoạt sẽ đánh dấu \(profile.name) là môi trường đang bật" }
        return "Kích hoạt sẽ " + parts.joined(separator: ", ") + " và ghi vào /etc/hosts"
    }

    // MARK: - Entries

    private func sectionHeader(_ profile: Profile, entries: [HostEntry], enabledCount: Int) -> some View {
        HStack(spacing: DS.Spacing.space4) {
            Text(entries.isEmpty ? "Entries" : "Entries · \(enabledCount) trong \(entries.count) bật")
                .font(DS.Text.textMenuLabel)
                .foregroundStyle(DS.Colors.inkSecondary)
            Spacer(minLength: 0)
            if entries.count > 1 {
                let allOn = enabledCount == entries.count
                Button(allOn ? "Tắt tất cả" : "Bật tất cả") {
                    hostsManager.setTagEnabled(name: profile.name, enabled: !allOn)
                }
                .buttonStyle(.ds(.plain, size: .sm))
            }
            if !entries.isEmpty {
                Button("Thêm entry") { entrySheet = .add }
                    .buttonStyle(.ds(.plain, size: .sm))
            }
        }
        .padding(.horizontal, DS.Spacing.space1)
    }

    private func entriesCard(_ profile: Profile, entries: [HostEntry]) -> some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(Array(entries.enumerated()), id: \.element.id) { index, entry in
                    DSListRow(
                        glyph: "network",
                        glyphInWell: false,
                        title: entry.hostname,
                        sublabel: entry.comment.isEmpty ? entry.ip : "\(entry.ip) · \(entry.comment)",
                        showsHairline: index < entries.count - 1
                    ) {
                        DSSwitch(
                            isOn: Binding(get: { entry.isEnabled }, set: { _ in hostsManager.toggleEntry(id: entry.id) })
                        )
                        .accessibilityLabel("\(entry.hostname), \(entry.isEnabled ? "bật" : "tắt")")
                    }
                    .onTapGesture(count: 2) { entrySheet = .edit(entry) }
                    .contextMenu {
                        Button("Sửa") { entrySheet = .edit(entry) }
                        Button("Sao chép hostname") {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(entry.hostname, forType: .string)
                        }
                        Divider()
                        Button("Xoá entry", role: .destructive) { deleteEntryTarget = entry }
                    }
                }
            }
        }
        .background(DS.Colors.surfaceContent)
        .dsContentCard(radius: DS.Radius.md)
    }
}
