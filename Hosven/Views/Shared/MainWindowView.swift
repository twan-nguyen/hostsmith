import SwiftUI

/// The window: a flat `surfaceSidebar` source list with a hairline on its
/// trailing edge, and the working pane on the ivory ground beside it.
/// The traffic lights sit in the sidebar's top strip; when the sidebar is
/// hidden, the pane leaves that strip free above its toolbar.
struct MainWindowView: View {
    @Environment(HostsFileManager.self) private var hostsManager
    @Environment(EnvFileManager.self) private var envManager
    @AppStorage("sidebarSelection") private var selectionRaw: String = AppSidebarSelection.systemHosts.rawValue
    @AppStorage("sidebarWidth") private var sidebarWidth: Double = Double(DS.Size.sidebarWidth)
    @AppStorage("sidebarCollapsed") private var sidebarCollapsed = false
    @AppStorage("appearanceMode") private var appearanceRaw: String = AppearanceMode.system.rawValue
    @State private var showCommandPalette = false
    @State private var dragStartWidth: Double?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var appearance: AppearanceMode {
        AppearanceMode(rawValue: appearanceRaw) ?? .system
    }

    private var selection: Binding<AppSidebarSelection> {
        Binding(
            get: { AppSidebarSelection(rawValue: selectionRaw) ?? .systemHosts },
            set: { selectionRaw = $0.rawValue }
        )
    }

    var body: some View {
        ZStack {
            DS.Colors.surfaceWindow.ignoresSafeArea()

            HStack(spacing: 0) {
                if !sidebarCollapsed {
                    SidebarView(selection: selection)
                        .frame(width: sidebarWidth)
                        .dsHairline(.trailing)
                        .overlay(alignment: .trailing) { resizeHandle }
                        .transition(.move(edge: .leading).combined(with: .opacity))
                }

                pane
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .padding(.top, sidebarCollapsed ? WindowChromeConfigurator.titleStripHeight : 0)
            }
            .animation(reduceMotion ? nil : .dsSheet, value: sidebarCollapsed)
        }
        .overlay(alignment: .topLeading) { sidebarToggle }
        .ignoresSafeArea()
        .onOpenURL { url in handle(url) }
        .preferredColorScheme(appearance.colorScheme)
        .background(WindowChromeConfigurator())
        .background(commandPaletteShortcut)
        .overlay(alignment: .top) {
            if showCommandPalette {
                CommandPaletteView(isPresented: $showCommandPalette, select: { selection.wrappedValue = $0 })
                    .zIndex(100)
            }
        }
        .animation(reduceMotion ? nil : .dsPopover, value: showCommandPalette)
        .alert(
            "Có \(hostsManager.pendingSwitch?.pendingCount ?? 0) thay đổi chưa áp dụng",
            isPresented: pendingSwitchAlertPresented,
            presenting: hostsManager.pendingSwitch
        ) { _ in
            Button("Áp dụng cùng") { hostsManager.confirmPendingSwitch() }
            Button("Huỷ", role: .cancel) { hostsManager.cancelPendingSwitch() }
        } message: { pending in
            Text(pendingSwitchMessage(pending))
        }
    }

    // MARK: - URL scheme

    private func handle(_ url: URL) {
        NSApp.activate(ignoringOtherApps: true)
        guard let command = URLCommand(url: url) else {
            hostsManager.showToast("Không hiểu lệnh \(url.absoluteString)", type: .error)
            return
        }
        switch command {
        case .activate(let name):
            guard let profile = hostsManager.profiles.first(where: { $0.name.caseInsensitiveCompare(name) == .orderedSame }) else {
                hostsManager.showToast("Không có profile tên \(name)", type: .error)
                return
            }
            selection.wrappedValue = .profile(profile.id)
            guard profile.isExclusive else {
                hostsManager.showToast("\(profile.name) là block, không phải môi trường", type: .error)
                return
            }
            hostsManager.switchEnvironment(to: profile.id, policy: .refuseIfPending) { reportRefusal($0) }
        case .deactivate:
            hostsManager.switchEnvironment(to: nil, policy: .refuseIfPending) { reportRefusal($0) }
        case .applyEnvProfile(let repoName, let profileName):
            guard let repo = envManager.repos.first(where: { $0.name.caseInsensitiveCompare(repoName) == .orderedSame }) else {
                hostsManager.showToast("Không có repo tên \(repoName)", type: .error)
                return
            }
            selection.wrappedValue = .repo(repo.id)
            guard let profile = repo.profiles.first(where: { $0.name.caseInsensitiveCompare(profileName) == .orderedSame }) else {
                envManager.showToast("Repo \(repo.name) không có profile tên \(profileName)", type: .error)
                return
            }
            do {
                try envManager.applyProfile(repoId: repo.id, profileId: profile.id)
            } catch {
                envManager.showToast("Không áp dụng được profile: \(error.localizedDescription)", type: .error)
            }
        case .open(let repoName):
            guard let repo = envManager.repos.first(where: { $0.name.caseInsensitiveCompare(repoName) == .orderedSame }) else {
                hostsManager.showToast("Không có repo tên \(repoName)", type: .error)
                return
            }
            selection.wrappedValue = .repo(repo.id)
        case .ports:
            selection.wrappedValue = .ports
        }
    }

    /// Scripts cannot answer a dialog, so a URL switch refuses to write pending edits and says so.
    private func reportRefusal(_ outcome: HostsFileManager.SwitchOutcome) {
        if case .refused(let count) = outcome {
            hostsManager.showToast("Có \(count) thay đổi chưa áp dụng. Áp dụng hoặc huỷ chúng trước khi đổi môi trường.", type: .error)
        }
    }

    /// Shown when a switch from the window would also write other pending changes.
    private var pendingSwitchAlertPresented: Binding<Bool> {
        Binding(
            get: { hostsManager.pendingSwitch != nil },
            set: { if !$0 { hostsManager.cancelPendingSwitch() } }
        )
    }

    private func pendingSwitchMessage(_ pending: HostsFileManager.PendingSwitch) -> String {
        let action = pending.target
            .flatMap { id in hostsManager.profiles.first { $0.id == id } }
            .map { "Đổi sang \($0.name)" } ?? "Tắt môi trường"
        return "\(action) sẽ ghi cả \(pending.pendingCount) thay đổi này vào /etc/hosts."
    }

    // MARK: - Pane routing

    @ViewBuilder
    private var pane: some View {
        switch selection.wrappedValue {
        case .systemHosts:
            HostsPane(focusTag: nil)
        case .hostsTag(let name):
            HostsPane(focusTag: name)
        case .profile(let id):
            if hostsManager.profiles.contains(where: { $0.id == id }) {
                ProfilePane(profileId: id, select: { selection.wrappedValue = $0 })
            } else {
                HostsPane(focusTag: nil)
            }
        case .repo(let id):
            EnvPane(repoId: id, filePath: nil, select: { selection.wrappedValue = $0 })
        case .envFile(let repoId, let path):
            EnvPane(repoId: repoId, filePath: path, select: { selection.wrappedValue = $0 })
        case .ports:
            PortsPane()
        }
    }

    // MARK: - Sidebar toggle and resize

    /// Sits in the title strip after the traffic lights, in the sidebar's top
    /// strip when it is shown and above the pane when it is hidden.
    private var sidebarToggle: some View {
        Button {
            sidebarCollapsed.toggle()
        } label: {
            Image(systemName: "sidebar.leading")
                .font(.system(size: 14, weight: .medium))
                .frame(width: 20)
        }
        .buttonStyle(.ds(.plain, size: .sm))
        .help(sidebarCollapsed ? "Hiện thanh bên (⌃⌘S)" : "Ẩn thanh bên (⌃⌘S)")
        .accessibilityLabel(sidebarCollapsed ? "Hiện thanh bên" : "Ẩn thanh bên")
        .padding(.leading, WindowChromeConfigurator.trafficLightCenter.x + 56)
        .padding(.top, WindowChromeConfigurator.titleStripHeight / 2 - DS.Size.controlHeightSm / 2)
    }

    /// Invisible strip on the sidebar's trailing edge; resizes 200–320pt and remembers the width.
    private var resizeHandle: some View {
        Color.clear
            .frame(width: 8)
            .contentShape(Rectangle())
            .onHover { inside in
                if inside { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() }
            }
            .gesture(
                DragGesture(minimumDistance: 1)
                    .onChanged { value in
                        if dragStartWidth == nil { dragStartWidth = sidebarWidth }
                        let proposed = (dragStartWidth ?? sidebarWidth) + value.translation.width
                        sidebarWidth = min(320, max(200, proposed))
                    }
                    .onEnded { value in
                        // Dragging well past the minimum collapses the column instead of squeezing it.
                        if (dragStartWidth ?? sidebarWidth) + value.translation.width < 140 {
                            sidebarCollapsed = true
                        }
                        dragStartWidth = nil
                    }
            )
    }

    /// Invisible button hosting the ⌘K shortcut.
    private var commandPaletteShortcut: some View {
        Button {
            showCommandPalette.toggle()
        } label: { EmptyView() }
        .buttonStyle(.plain)
        .frame(width: 0, height: 0)
        .opacity(0)
        .keyboardShortcut("k", modifiers: .command)
        .accessibilityHidden(true)
    }
}
