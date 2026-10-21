import SwiftUI
import AppKit

/// The source list: a `surfaceSidebar` column one warm step below the ground,
/// with fixed groups Môi trường, Hosts, Repos, Tools. Spec: `components/Sidebar/README.md`.
/// The top strip holds the traffic lights, then the app mark and name. The sidebar never holds an action: adding,
/// renaming and removing live in context menus, the toolbar and the app menu.
struct SidebarView: View {
    @Binding var selection: AppSidebarSelection
    @Environment(HostsFileManager.self) private var hostsManager
    @Environment(EnvFileManager.self) private var envManager
    @Environment(PortsManager.self) private var portsManager
    @Environment(\.controlActiveState) private var activeState

    @State private var envFilesByRepo: [UUID: [String]] = [:]
    @State private var profileSheet: ProfileFormSheet.Mode?
    @State private var deleteTarget: Profile?
    @State private var repoRenameTarget: EnvRepo?
    @State private var repoRenameBuffer = ""
    /// Names of the groups the person folded, comma-separated so AppStorage can hold it.
    @AppStorage("sidebarCollapsedGroups") private var collapsedGroupsRaw = ""

    private func isExpanded(_ group: String) -> Bool {
        !collapsedGroupsRaw.split(separator: ",").contains(Substring(group))
    }

    private func toggleGroup(_ group: String) {
        var names = collapsedGroupsRaw.split(separator: ",").map(String.init).filter { !$0.isEmpty }
        if let idx = names.firstIndex(of: group) { names.remove(at: idx) } else { names.append(group) }
        collapsedGroupsRaw = names.joined(separator: ",")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Color.clear.frame(height: WindowChromeConfigurator.titleStripHeight)
            header
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    profilesGroup
                    hostsGroup
                    reposGroup
                    toolsGroup
                }
                .padding(.bottom, DS.Spacing.space4)
            }
            .scrollIndicators(.hidden)
            .animation(.dsSwitch, value: collapsedGroupsRaw)
        }
        .padding(.horizontal, DS.Spacing.space4)
        .padding(.bottom, DS.Spacing.space5)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(DS.Colors.surfaceSidebar)
        .task(id: envManager.repos.map(\.path)) { refreshEnvFiles() }
        .onChange(of: selection) { _, new in
            if new.repoId != nil { refreshEnvFiles() }
        }
        .sheet(item: $profileSheet) { mode in
            ProfileFormSheet(mode: mode)
        }
        .alert(
            "Xoá profile?",
            isPresented: Binding(get: { deleteTarget != nil }, set: { if !$0 { deleteTarget = nil } }),
            presenting: deleteTarget
        ) { target in
            Button("Xoá profile", role: .destructive) {
                if case .profile(target.id) = selection { selection = .systemHosts }
                if case .hostsTag(target.name) = selection { selection = .systemHosts }
                hostsManager.removeProfile(id: target.id)
            }
            Button("Huỷ", role: .cancel) { deleteTarget = nil }
        } message: { target in
            Text("Xoá metadata của profile \(target.name). Các entry trong /etc/hosts được giữ nguyên.")
        }
        .alert(
            "Đổi tên repo",
            isPresented: Binding(get: { repoRenameTarget != nil }, set: { if !$0 { repoRenameTarget = nil } })
        ) {
            TextField("Tên repo", text: $repoRenameBuffer)
            Button("Đổi tên") {
                if let repo = repoRenameTarget {
                    try? envManager.renameRepo(id: repo.id, newName: repoRenameBuffer)
                }
                repoRenameTarget = nil
            }
            Button("Huỷ", role: .cancel) { repoRenameTarget = nil }
        }
    }

    // MARK: - Header

    /// The app mark and name, the way the Claude app names itself above its list.
    private var header: some View {
        HStack(spacing: DS.Spacing.space4 + 2) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .interpolation(.high)
                .frame(width: 22, height: 22)
                .accessibilityHidden(true)
            Text("Hosven")
                .font(.system(size: 17, weight: .medium, design: .serif))
                .foregroundStyle(DS.Colors.inkPrimary)
        }
        .padding(.horizontal, DS.Spacing.space4)
        .padding(.top, DS.Spacing.space4)
        .padding(.bottom, DS.Spacing.space2)
    }

    // MARK: - Groups

    /// Environments: switching to one turns the others off and writes `/etc/hosts`.
    private var profilesGroup: some View {
        Group {
            groupLabel("Môi trường")
                .contextMenu { addProfileMenuItem }
            if isExpanded("Môi trường") {
                if hostsManager.environmentProfiles.isEmpty {
                    emptyHint("Chưa có môi trường")
                }
                ForEach(hostsManager.environmentProfiles, id: \.id) { profile in
                    let isActive = hostsManager.activeProfileID == profile.id
                    let count = hostsManager.tagEntryCount(name: profile.name)
                    sidebarRow(
                        glyph: "square.stack.3d.up",
                        title: profile.name,
                        isSelected: selection == .profile(profile.id),
                        identifier: "profile-row-\(profile.name)"
                    ) {
                        if isActive {
                            DSChip(text: activeChipText(profile), color: profile.color)
                        } else {
                            trailingCount(count)
                        }
                    } action: {
                        selection = .profile(profile.id)
                    }
                    .contextMenu {
                        if isActive {
                            Button("Tắt \(profile.name)") {
                                hostsManager.switchEnvironment(to: nil, policy: .askIfPending)
                            }
                        } else {
                            Button("Chuyển sang \(profile.name)") {
                                hostsManager.switchEnvironment(to: profile.id, policy: .askIfPending)
                            }
                            .disabled(count == 0)
                        }
                        profileMenuItems(profile)
                    }
                }
            }
        }
    }

    /// System hosts, then blocks: profiles that are not environments, and tags without a profile.
    private var hostsGroup: some View {
        Group {
            groupLabel("Hosts")
            if isExpanded("Hosts") {
                sidebarRow(
                    glyph: "network",
                    title: "Hosts hệ thống",
                    isSelected: selection == .systemHosts,
                    identifier: "sidebar-system-hosts"
                ) {
                    trailingCount(hostsManager.entries.count)
                } action: {
                    selection = .systemHosts
                }
                ForEach(hostsManager.blockProfiles, id: \.id) { profile in
                    let state = hostsManager.tagState(name: profile.name)
                    sidebarRow(
                        title: profile.name,
                        isChild: true,
                        isSelected: selection == .profile(profile.id),
                        identifier: "sidebar-block-\(profile.name)"
                    ) {
                        HStack(spacing: DS.Spacing.space2) {
                            if state != .allDisabled {
                                Circle().fill(DS.Colors.success).frame(width: 6, height: 6)
                                    .accessibilityLabel("Đang bật")
                            }
                            trailingCount(hostsManager.tagEntryCount(name: profile.name))
                        }
                    } action: {
                        selection = .profile(profile.id)
                    }
                    .contextMenu { profileMenuItems(profile) }
                }
                let profileNames = Set(hostsManager.profiles.map(\.name))
                ForEach(hostsManager.tags.filter { !profileNames.contains($0.name) }) { tag in
                    sidebarRow(
                        title: tag.name,
                        isChild: true,
                        isSelected: selection == .hostsTag(tag.name),
                        identifier: "sidebar-block-\(tag.name)"
                    ) {
                        trailingCount(hostsManager.tagEntryCount(name: tag.name))
                    } action: {
                        selection = .hostsTag(tag.name)
                    }
                }
            }
        }
    }

    /// Edit, change kind and delete, shared by environment and block rows.
    @ViewBuilder
    private func profileMenuItems(_ profile: Profile) -> some View {
        Button("Sửa") { profileSheet = .rename(profile) }
        Button(profile.isExclusive ? "Chuyển thành block" : "Chuyển thành môi trường") {
            hostsManager.updateProfileExclusive(id: profile.id, !profile.isExclusive)
        }
        Divider()
        Button("Xoá profile", role: .destructive) { deleteTarget = profile }
    }

    private var reposGroup: some View {
        Group {
            groupLabel("Repos")
                .contextMenu { addRepoMenuItem }
            if isExpanded("Repos") {
                if envManager.repos.isEmpty {
                    emptyHint("Chưa có repo")
                        .contextMenu { addRepoMenuItem }
                }
                ForEach(envManager.repos) { repo in
                    let exists = envManager.repoPathExists(repo)
                    let files = envFilesByRepo[repo.id] ?? []
                    // A repo with a single env file is one row: the repo opens that file directly.
                    let onlyFile = files.count == 1 ? files[0] : nil
                    sidebarRow(
                        glyph: exists ? "folder" : "folder.badge.questionmark",
                        title: repo.name,
                        isSelected: selection == .repo(repo.id)
                            || onlyFile.map { selection == .envFile(repo: repo.id, path: $0) } == true,
                        identifier: "sidebar-repo-\(repo.name)"
                    ) {
                        if !exists {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .font(.system(size: 11))
                                .foregroundStyle(DS.Colors.warning)
                                .accessibilityLabel("Không tìm thấy thư mục")
                        } else if let onlyFile {
                            if envManager.loadedFile(repoId: repo.id, relativePath: onlyFile)?.hasUnsavedChanges == true {
                                unsavedDot
                            }
                        } else {
                            trailingCount(files.count)
                        }
                    } action: {
                        selection = onlyFile.map { .envFile(repo: repo.id, path: $0) } ?? .repo(repo.id)
                    }
                    .help(repo.path)
                    .contextMenu {
                        Button("Đổi tên") {
                            repoRenameBuffer = repo.name
                            repoRenameTarget = repo
                        }
                        Button("Mở trong Finder") {
                            NSWorkspace.shared.open(URL(fileURLWithPath: repo.path))
                        }
                        if let terminal = ExternalTools.terminal {
                            Button("Mở trong \(terminal.name)") { ExternalTools.openInTerminal(repo.path) }
                        }
                        if let editor = ExternalTools.editor {
                            Button("Mở bằng \(editor.name)") { ExternalTools.openInEditor(repo.path) }
                        }
                        Divider()
                        Button("Xoá khỏi danh sách", role: .destructive) {
                            if selection.repoId == repo.id { selection = .systemHosts }
                            envManager.removeRepo(id: repo.id)
                        }
                    }

                    ForEach(onlyFile == nil ? files : [], id: \.self) { path in
                        let file = envManager.loadedFile(repoId: repo.id, relativePath: path)
                        sidebarRow(
                            title: path,
                            isChild: true,
                            isSelected: selection == .envFile(repo: repo.id, path: path),
                            identifier: "sidebar-envfile-\(repo.name)-\(path)"
                        ) {
                            if file?.hasUnsavedChanges == true {
                                unsavedDot
                            }
                        } action: {
                            selection = .envFile(repo: repo.id, path: path)
                        }
                    }
                }
            }
        }
    }

    private var unsavedDot: some View {
        Circle().fill(DS.Colors.warning).frame(width: 6, height: 6)
            .accessibilityLabel("Chưa áp dụng")
    }

    private var toolsGroup: some View {
        Group {
            groupLabel("Tools")
            if isExpanded("Tools") {
                sidebarRow(
                    glyph: "bolt.horizontal.circle",
                    title: "Ports",
                    isSelected: selection == .ports,
                    identifier: "sidebar-ports"
                ) {
                    trailingCount(portsManager.userPorts.count)
                } action: {
                    selection = .ports
                }
            }
        }
    }

    // MARK: - Rows

    private func sidebarRow<Trailing: View>(
        glyph: String? = nil,
        title: String,
        isChild: Bool = false,
        isSelected: Bool,
        identifier: String,
        @ViewBuilder trailing: () -> Trailing,
        action: @escaping () -> Void
    ) -> some View {
        // Selection is quiet: the hover fill, a medium label and the glyph in accent.
        // An inactive window keeps the fill and drops the accent.
        let isKey = activeState == .key
        let glyphInk = isSelected && isKey ? DS.Colors.accent : DS.Colors.inkSecondary
        return Button(action: action) {
            HStack(spacing: DS.Spacing.space4) {
                if let glyph {
                    Image(systemName: glyph)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(glyphInk)
                        .frame(width: 18)
                }
                Text(title)
                    .font(.system(size: 14, weight: isSelected ? .medium : .regular))
                    .foregroundStyle(DS.Colors.inkPrimary)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: DS.Spacing.space4)
                trailing()
                    .foregroundStyle(DS.Colors.inkSecondary)
            }
            .padding(.leading, isChild ? DS.Spacing.space9 : DS.Spacing.space4)
            .padding(.trailing, DS.Spacing.space4)
            .frame(height: DS.Size.rowHeight)
            .background(rowBackground(isSelected: isSelected))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .focusEffectDisabled()
        .accessibilityIdentifier(identifier)
        .accessibilityAddTraits(isSelected ? [.isSelected] : [])
    }

    @ViewBuilder
    private func rowBackground(isSelected: Bool) -> some View {
        if isSelected {
            RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                .fill(DS.Colors.surfaceHover)
        } else {
            SidebarHoverFill()
        }
    }

    private func trailingCount(_ count: Int) -> some View {
        Text("\(count)")
            .font(DS.Text.textCaption)
    }

    /// Group heading that folds its rows on click. The chevron is the only
    /// affordance; the label itself keeps the menu-label style.
    private func groupLabel(_ text: String) -> some View {
        let expanded = isExpanded(text)
        return HStack(spacing: DS.Spacing.space2) {
            Text(text)
                .font(DS.Text.textMenuLabel)
            Image(systemName: "chevron.right")
                .font(.system(size: 9, weight: .bold))
                .rotationEffect(.degrees(expanded ? 90 : 0))
                .foregroundStyle(DS.Colors.inkTertiary)
            Spacer(minLength: 0)
        }
        .foregroundStyle(DS.Colors.inkSecondary)
        .padding(.top, DS.Spacing.space6)
        .padding(.bottom, DS.Spacing.space2)
        .padding(.horizontal, DS.Spacing.space4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .onTapGesture { toggleGroup(text) }
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel("\(text), \(expanded ? "đang mở" : "đang thu gọn")")
    }

    private func emptyHint(_ text: String) -> some View {
        Text(text)
            .font(DS.Text.textCaption)
            .foregroundStyle(DS.Colors.inkTertiary)
            .padding(.horizontal, DS.Spacing.space4)
            .frame(height: DS.Size.rowHeight)
    }

    private func activeChipText(_ profile: Profile) -> String {
        switch profile.color {
        case .blue, .amber, .red: return profile.color.displayName
        case .green, .purple: return "Đang bật"
        }
    }

    // MARK: - Menu items

    private var addProfileMenuItem: some View {
        Button("Thêm profile") { profileSheet = .create }
    }

    private var addRepoMenuItem: some View {
        Button("Thêm repo") { EnvRepoPicker.pick(into: envManager) { selection = .repo($0) } }
    }

    // MARK: - Data

    private func refreshEnvFiles() {
        var result: [UUID: [String]] = [:]
        for repo in envManager.repos {
            result[repo.id] = envManager.discoverEnvFiles(in: repo.path).paths
        }
        envFilesByRepo = result
    }
}

/// Hover fill for an unselected sidebar row: `surfaceHover` at `Radius.sm`.
private struct SidebarHoverFill: View {
    @State private var hovering = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
            .fill(hovering ? DS.Colors.surfaceHover : Color.clear)
            .onHover { hovering = $0 }
            .animation(reduceMotion ? nil : .dsSwitch, value: hovering)
    }
}

/// Shared open-panel flow for adding a repo folder.
enum EnvRepoPicker {
    @MainActor
    static func pick(into envManager: EnvFileManager, onAdded: ((UUID) -> Void)? = nil) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.message = "Chọn thư mục gốc của project"
        panel.prompt = "Thêm"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try envManager.addRepo(at: url)
            if let repo = envManager.repos.first(where: { $0.path == url.path }) {
                onAdded?(repo.id)
            }
        } catch {
            envManager.showToast("Không thêm được repo: \(error.localizedDescription)", type: .error)
        }
    }
}
