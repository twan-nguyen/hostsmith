import SwiftUI
import AppKit

/// The panel that drops from the status item. Spec: `components/MenuBarPanel/README.md`
/// and `guidelines/20-menu-bar.md`. Exactly `menubarWidth` wide, never scrolls:
/// active profile card, up to eight quick toggles, up to three ports, footer.
struct MenuBarContentView: View {
    @Environment(HostsFileManager.self) private var hostsManager
    @Environment(EnvFileManager.self) private var envManager
    @Environment(PortsManager.self) private var portsManager
    @AppStorage("sidebarSelection") private var selectionRaw: String = AppSidebarSelection.systemHosts.rawValue
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismiss) private var dismiss
    @State private var showProfileSwitcher = false
    @State private var failure: String?

    private static let maxToggles = 8

    private var activeProfile: Profile? { hostsManager.activeProfile }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            activeProfileCard

            if showProfileSwitcher {
                profileSwitcher
            }

            separator

            if let failure {
                DSBanner(level: .danger, title: failure, actions: [.init(label: "Đóng") { self.failure = nil }])
                    .padding(.horizontal, DS.Spacing.space3)
                    .padding(.bottom, DS.Spacing.space3)
            }

            groupLabel("Bật nhanh")
            quickToggles

            separator

            groupLabel("Cổng đang dùng · \(portsManager.userPorts.count)")
            portsSection

            separator

            footer
        }
        .task {
            portsManager.knownRepos = envManager.repos.map { ($0.name, $0.path) }
            portsManager.scan()
        }
        .padding(DS.Spacing.space3)
        .frame(width: DS.Size.menubarWidth)
        .dsPopoverSurface(radius: DS.Radius.lg, menubar: true)
        .padding(DS.Spacing.space2)
    }

    // MARK: - Active profile

    private var activeProfileCard: some View {
        Button {
            showProfileSwitcher.toggle()
        } label: {
            HStack(spacing: DS.Spacing.space4) {
                Image(systemName: "square.stack.3d.up")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(DS.Colors.inkSecondary)
                Text(activeProfile?.name ?? (hostsManager.activeProfileState == .none ? "Không có môi trường" : "Nhiều môi trường đang bật"))
                    .font(.system(size: 17, weight: .medium, design: .serif))
                    .foregroundStyle(DS.Colors.inkPrimary)
                    .lineLimit(1)
                Spacer(minLength: DS.Spacing.space4)
                if let profile = activeProfile {
                    DSChip(text: profile.color.displayName, color: profile.color, style: .solid)
                } else {
                    Image(systemName: "chevron.down")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(DS.Colors.inkSecondary)
                }
            }
            .padding(.horizontal, DS.Spacing.space5)
            .padding(.vertical, DS.Spacing.space4)
            .background(
                RoundedRectangle(cornerRadius: DS.Radius.md, style: .continuous)
                    .fill(DS.Colors.surfaceSidebar)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .focusEffectDisabled()
        .accessibilityLabel(activeProfile.map { "Môi trường đang bật: \($0.name)" } ?? "Không có môi trường đang bật")
    }

    private var profileSwitcher: some View {
        VStack(spacing: 0) {
            ForEach(hostsManager.environmentProfiles.filter { hostsManager.tagEntryCount(name: $0.name) > 0 }, id: \.id) { profile in
                MenuBarItem(
                    title: profile.name,
                    subtitle: "\(hostsManager.tagEntryCount(name: profile.name)) entry",
                    trailingText: profile.id == hostsManager.activeProfileID ? "Đang bật" : nil
                ) {
                    switchEnvironment(to: profile.id)
                }
            }
            if hostsManager.activeProfileState != .none {
                MenuBarItem(title: "Tắt môi trường", subtitle: nil, trailingText: "⌘0") {
                    switchEnvironment(to: nil)
                }
            }
        }
        .padding(.top, DS.Spacing.space2)
    }

    // MARK: - Quick toggles

    /// Blocks only: environments are switched in the card above, one at a time.
    private var quickToggles: some View {
        let profiles = hostsManager.blockProfiles.filter { hostsManager.tagEntryCount(name: $0.name) > 0 }
        let shown = Array(profiles.prefix(Self.maxToggles))
        return VStack(spacing: 0) {
            if shown.isEmpty {
                Text("Chưa có block nào trong /etc/hosts")
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkTertiary)
                    .padding(.horizontal, DS.Spacing.space5)
                    .frame(height: 34, alignment: .leading)
            }
            ForEach(Array(shown.enumerated()), id: \.element.id) { index, profile in
                quickToggleRow(profile, position: index + 1)
            }
            if profiles.count > shown.count {
                MenuBarItem(title: "Xem tất cả trong Hosven", subtitle: nil, trailingText: nil, accent: true) {
                    showMainWindow()
                }
            }
        }
    }

    private func quickToggleRow(_ profile: Profile, position: Int) -> some View {
        let state = hostsManager.tagState(name: profile.name)
        let entries = hostsManager.entries.filter { $0.tag == profile.name }
        let mapping = entries.first.map { "\($0.ip) → \(entries.count == 1 ? $0.hostname : "\(entries.count) entry")" } ?? ""
        return HStack(spacing: DS.Spacing.space4) {
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: DS.Spacing.space2) {
                    Text(profile.name)
                        .font(DS.Text.textBody)
                        .foregroundStyle(DS.Colors.inkPrimary)
                        .lineLimit(1)
                    Image(systemName: "lock.fill")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(DS.Colors.inkSecondary)
                }
                Text(mapping)
                    .font(DS.Text.monoCaption)
                    .foregroundStyle(DS.Colors.inkSecondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
            Spacer(minLength: DS.Spacing.space4)
            DSSwitch(
                isOn: Binding(get: { state != .allDisabled }, set: { flip(profile, enabled: $0) })
            )
            .accessibilityLabel("\(profile.name), \(state == .allDisabled ? "tắt" : "bật")")
        }
        .padding(.horizontal, DS.Spacing.space5)
        .padding(.vertical, DS.Spacing.space2)
        .frame(minHeight: DS.Size.rowHeight)
    }

    /// Flips a block and writes at once. The switch moves optimistically; a failed
    /// write reverts it and reports inside the panel.
    private func flip(_ profile: Profile, enabled: Bool) {
        failure = nil
        // Same rule as switching environments: never write edits pending in the window.
        guard !hostsManager.hasUnsavedChanges else {
            failure = "Có \(hostsManager.pendingChangeCount) thay đổi chưa áp dụng trong Hosven. Mở Hosven để áp dụng hoặc huỷ trước."
            return
        }
        hostsManager.setTagEnabled(name: profile.name, enabled: enabled)
        hostsManager.applyChanges { success, message in
            guard !success else { return }
            hostsManager.setTagEnabled(name: profile.name, enabled: !enabled)
            if let message {
                failure = "Không ghi được /etc/hosts vì \(message). Thử lại và chấp nhận hộp thoại xác thực."
            }
        }
    }

    /// Switches and writes at once. The panel cannot hold a dialog, so pending edits
    /// made in the window block the switch and the panel says so.
    private func switchEnvironment(to id: UUID?) {
        failure = nil
        hostsManager.switchEnvironment(to: id, policy: .refuseIfPending) { outcome in
            switch outcome {
            case .written:
                showProfileSwitcher = false
            case .refused(let count):
                failure = "Có \(count) thay đổi chưa áp dụng trong Hosven. Mở Hosven để áp dụng hoặc huỷ trước."
            case .failed(let message):
                failure = "Không ghi được /etc/hosts vì \(message). Thử lại và chấp nhận hộp thoại xác thực."
            case .cancelled, .needsConfirmation, .nothingToDo:
                break
            }
        }
    }

    // MARK: - Ports

    private static let maxPorts = 3

    private var portsSection: some View {
        let shown = Array(portsManager.userPorts.prefix(Self.maxPorts))
        return VStack(spacing: 0) {
            if shown.isEmpty {
                Text(portsManager.isScanning ? "Đang quét" : "Không có cổng nào đang lắng nghe")
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkTertiary)
                    .padding(.horizontal, DS.Spacing.space5)
                    .frame(height: 34, alignment: .leading)
            }
            ForEach(shown) { port in
                let unknown = port.needsAttention
                MenuBarItem(
                    title: "\(port.port)  \(port.process)\(unknown ? " · không rõ" : "")",
                    subtitle: nil,
                    trailingText: unknown ? "Dừng" : ownerText(port),
                    danger: unknown
                ) {
                    selectionRaw = AppSidebarSelection.ports.rawValue
                    showMainWindow()
                }
            }
            if portsManager.userPorts.count > shown.count {
                MenuBarItem(title: "Xem tất cả trong Hosven", subtitle: nil, trailingText: nil, accent: true) {
                    selectionRaw = AppSidebarSelection.ports.rawValue
                    showMainWindow()
                }
            }
        }
    }

    private func ownerText(_ port: ListeningPort) -> String {
        switch port.owner {
        case .yours(let repo): return repo
        case .container(let name): return name ?? "docker"
        case .unknown: return port.user
        }
    }

    // MARK: - Footer

    private var footer: some View {
        VStack(spacing: 0) {
            MenuBarItem(title: "Mở Hosven", subtitle: nil, trailingText: "⌘O") { showMainWindow() }
                .keyboardShortcut("o", modifiers: .command)
            MenuBarItem(title: "Cài đặt", subtitle: nil, trailingText: "⌘,") { openSettings() }
                .keyboardShortcut(",", modifiers: .command)
            MenuBarItem(title: "Thoát Hosven", subtitle: nil, trailingText: "⌘Q") { NSApp.terminate(nil) }
                .keyboardShortcut("q", modifiers: .command)
        }
    }

    private var separator: some View {
        Color.clear
            .frame(height: DS.Size.hairlineWidth)
            .dsHairline(.bottom)
            .padding(.horizontal, DS.Spacing.space5)
            .padding(.vertical, DS.Spacing.space3)
    }

    private func groupLabel(_ text: String) -> some View {
        Text(text)
            .font(DS.Text.textMenuLabel)
            .foregroundStyle(DS.Colors.inkSecondary)
            .padding(.horizontal, DS.Spacing.space5)
            .padding(.top, DS.Spacing.space3)
            .padding(.bottom, DS.Spacing.space2)
    }

    // MARK: - Actions

    private func openSettings() {
        dismiss()
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
    }

    private func showMainWindow() {
        dismiss()
        NSApp.setActivationPolicy(.regular)
        NSApp.unhide(nil)
        openWindow(id: "main")

        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            NSApp.activate(ignoringOtherApps: true)
            NSRunningApplication.current.activate(options: [.activateIgnoringOtherApps, .activateAllWindows])

            for window in NSApp.windows where window.canBecomeKey && !(window is NSPanel) {
                window.deminiaturize(nil)
                window.makeKeyAndOrderFront(nil)
                window.orderFrontRegardless()
                return
            }
        }
    }
}

/// A panel row: title, optional mono subtitle, trailing shortcut or status text.
/// Hover is `surfaceHover` at `Radius.sm`; the text colour does not change.
private struct MenuBarItem: View {
    let title: String
    let subtitle: String?
    let trailingText: String?
    var accent: Bool = false
    var danger: Bool = false
    let action: () -> Void

    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: DS.Spacing.space4) {
                VStack(alignment: .leading, spacing: 1) {
                    Text(title)
                        .font(DS.Text.textBody)
                        .foregroundStyle(danger ? DS.Colors.danger : (accent ? DS.Colors.accent : DS.Colors.inkPrimary))
                    if let subtitle {
                        Text(subtitle)
                            .font(DS.Text.monoCaption)
                            .foregroundStyle(DS.Colors.inkSecondary)
                    }
                }
                Spacer(minLength: DS.Spacing.space4)
                if let trailingText {
                    Text(trailingText)
                        .font(DS.Text.textCaption)
                        .foregroundStyle(DS.Colors.inkSecondary)
                }
            }
            .padding(.horizontal, DS.Spacing.space5)
            .padding(.vertical, DS.Spacing.space2)
            .frame(minHeight: DS.Size.rowHeight)
            .background(
                RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                    .fill(hovering ? DS.Colors.surfaceHover : Color.clear)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .focusEffectDisabled()
        .onHover { hovering = $0 }
    }
}
