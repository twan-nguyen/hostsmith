import SwiftUI
import AppKit

/// The Ports pane: what is taken, never what is free. `screens/Ports.dc.html` and `components/PortRow/README.md`.
struct PortsPane: View {
    enum Filter: String { case all, repos, system }

    @Environment(PortsManager.self) private var portsManager
    @Environment(EnvFileManager.self) private var envManager
    @AppStorage("portsFilter") private var filterRaw: String = Filter.all.rawValue
    @State private var killTarget: ListeningPort?
    @State private var forceKillTarget: ListeningPort?

    private var filter: Filter { Filter(rawValue: filterRaw) ?? .all }

    private var visiblePorts: [ListeningPort] {
        switch filter {
        case .all: return portsManager.userPorts
        case .repos: return portsManager.ports.filter { if case .yours = $0.owner { return true } else { return false } }
        case .system: return portsManager.ports.filter(\.isSystem)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            VStack(spacing: DS.Size.paneInset) {
                banner
                content
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .dsPaneContent()
        }
        .task {
            portsManager.knownRepos = envManager.repos.map { ($0.name, $0.path) }
            portsManager.scan()
        }
        .sheet(item: $killTarget) { port in
            KillConfirmSheet(port: port, force: false) { confirmKill(port, force: false) }
        }
        .sheet(item: $forceKillTarget) { port in
            KillConfirmSheet(port: port, force: true) { confirmKill(port, force: true) }
        }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        ToolbarView(title: "Ports", path: subtitle) {
            DSSegmentedControl([(Filter.all.rawValue, "Tất cả"), (Filter.repos.rawValue, "Repo của bạn"), (Filter.system.rawValue, "Hệ thống")], selection: $filterRaw)
            Button(portsManager.isScanning ? "Đang quét" : "Quét lại") { portsManager.scan() }
                .buttonStyle(.dsBordered)
                .disabled(portsManager.isScanning)
                .keyboardShortcut("r", modifiers: .command)
        }
    }

    private var subtitle: String {
        let count = "\(portsManager.userPorts.count) đang dùng"
        guard let lastScan = portsManager.lastScan else { return count }
        let seconds = Int(Date().timeIntervalSince(lastScan))
        return "\(count) · quét \(seconds < 5 ? "vừa xong" : "\(seconds) giây trước")"
    }

    // MARK: - Banner

    @ViewBuilder
    private var banner: some View {
        if let toast = portsManager.toast {
            DSBanner(level: toast.type.bannerLevel, title: toast.message)
        } else if let unknown = portsManager.attentionPorts.first, filter == .all {
            DSBanner(
                level: .danger,
                title: "\(unknown.process) trên cổng \(unknown.port) không thuộc repo nào Hosven biết",
                detail: "Dừng nó sẽ mở hộp thoại xác nhận nêu tên tiến trình và pid. Hosven gửi SIGTERM trước.",
                actions: [.init(label: "Hiện tiến trình") { revealProcess(unknown) }]
            )
        }
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if visiblePorts.isEmpty {
            DSEmptyState(
                icon: "bolt.horizontal.circle",
                title: portsManager.isScanning ? "Đang quét" : "Không có cổng nào đang lắng nghe",
                message: filter == .repos
                    ? "Không có tiến trình nào chạy từ repo Hosven biết đang giữ cổng."
                    : "Hosven liệt kê các cổng đang bị chiếm, không liệt kê cổng trống.",
                actionLabel: "Quét lại",
                action: { portsManager.scan() }
            )
        } else {
            VStack(spacing: 0) {
                header
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(visiblePorts) { port in
                            portRow(port)
                        }
                    }
                }
                .background(DS.Colors.surfaceContent)
            }
            .dsContentCard()
        }
    }

    private var header: some View {
        HStack(spacing: DS.Spacing.space6) {
            Text("Cổng").frame(width: 60, alignment: .leading)
            Text("Tiến trình")
            Spacer()
            Text("Chủ sở hữu")
        }
        .font(DS.Text.textMenuLabel)
        .foregroundStyle(DS.Colors.inkSecondary)
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space4)
        .background(DS.Colors.surfaceSunken)
        .dsHairline(.bottom)
    }

    private func portRow(_ port: ListeningPort) -> some View {
        HStack(spacing: DS.Spacing.space6) {
            Text("\(port.port)")
                .font(DS.Text.monoStrong)
                .foregroundStyle(DS.Colors.inkPrimary)
                .padding(.horizontal, DS.Spacing.space4)
                .frame(minWidth: 60)
                .frame(height: 26)
                .background(RoundedRectangle(cornerRadius: DS.Radius.xs, style: .continuous).fill(DS.Colors.surfaceSunken))

            VStack(alignment: .leading, spacing: 2) {
                Text(port.process)
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(DS.Colors.inkPrimary)
                    .lineLimit(1)
                Text(detail(port))
                    .font(DS.Text.monoCaption)
                    .foregroundStyle(DS.Colors.inkSecondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            StatusDot(state: dotState(port), label: port.ownerLabel)

            HStack(spacing: DS.Spacing.space3) {
                switch port.owner {
                case .yours:
                    Button("Mở") { open(port) }
                        .buttonStyle(.ds(.bordered, size: .sm))
                    Button("Dừng") { confirmKill(port, force: false) }
                        .buttonStyle(.ds(.danger, size: .sm))
                case .container:
                    Button("Dừng") { portsManager.stopContainer(port) }
                        .buttonStyle(.ds(.bordered, size: .sm))
                case .unknown:
                    Button("Dừng") { killTarget = port }
                        .buttonStyle(.ds(.danger, size: .sm))
                }
            }
        }
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space4)
        .frame(minHeight: DS.Size.rowHeight)
        .contentShape(Rectangle())
        .dsRowHover()
        .dsHairline(.bottom)
        .contextMenu {
            Button("Mở localhost:\(port.port)") { open(port) }
            if let cwd = port.cwd, cwd != "/" {
                Button("Hiện thư mục trong Finder") { NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: cwd) }
                if let terminal = ExternalTools.terminal {
                    Button("Mở thư mục trong \(terminal.name)") { ExternalTools.openInTerminal(cwd) }
                }
                if let editor = ExternalTools.editor {
                    Button("Mở thư mục bằng \(editor.name)") { ExternalTools.openInEditor(cwd) }
                }
            }
            Divider()
            Button("Buộc dừng (SIGKILL)", role: .destructive) { forceKillTarget = port }
        }
    }

    private func detail(_ port: ListeningPort) -> String {
        var parts = ["pid \(port.pid)"]
        switch port.owner {
        case .yours(let repo): parts.append(repo)
        case .container(let name): parts.append(name ?? "docker")
        case .unknown: parts.append(port.user)
        }
        if let cwd = port.cwd, cwd != "/" { parts.append(abbreviated(cwd)) }
        return parts.joined(separator: " · ")
    }

    private func abbreviated(_ path: String) -> String {
        let home = NSHomeDirectory()
        return path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }

    private func dotState(_ port: ListeningPort) -> StatusDot.State {
        switch port.owner {
        case .yours: return .ok
        case .container: return .idle
        case .unknown: return port.needsAttention ? .danger : .idle
        }
    }

    // MARK: - Actions

    private func open(_ port: ListeningPort) {
        if let url = URL(string: "http://localhost:\(port.port)") { NSWorkspace.shared.open(url) }
    }

    private func revealProcess(_ port: ListeningPort) {
        if let app = NSRunningApplication(processIdentifier: port.pid) {
            app.activate()
        } else if let cwd = port.cwd, cwd != "/" {
            NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: cwd)
        }
    }

    private func confirmKill(_ port: ListeningPort, force: Bool) {
        portsManager.kill(port, force: force) { released in
            if !released && !force {
                forceKillTarget = port
            }
        }
    }
}

/// Dangerous-level confirmation: the target named in `monoBody`, one sentence
/// of consequence, and a `dangerFill` confirm whose label is the verb.
private struct KillConfirmSheet: View {
    let port: ListeningPort
    let force: Bool
    let confirm: () -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        DSSheetContainer(
            title: force ? "Buộc dừng tiến trình?" : "Dừng tiến trình?",
            bodyContent: {
                VStack(alignment: .leading, spacing: DS.Spacing.space4) {
                    Text("\(port.process) (pid \(port.pid)) · cổng \(port.port)")
                        .font(DS.Text.monoBody)
                        .foregroundStyle(DS.Colors.inkPrimary)
                    Text(force
                         ? "Tiến trình vẫn chạy sau SIGTERM. SIGKILL dừng nó ngay và không cho nó lưu gì cả."
                         : "Hosven không biết tiến trình này thuộc project nào. Nó nhận SIGTERM và có thể mất dữ liệu chưa lưu.")
                        .font(DS.Text.textCallout)
                        .foregroundStyle(DS.Colors.inkSecondary)
                }
            },
            footer: {
                Button("Huỷ") { dismiss() }
                    .buttonStyle(.ds(.bordered, size: .lg))
                    .keyboardShortcut(.escape)
                Button(force ? "Buộc dừng" : "Dừng tiến trình") {
                    dismiss()
                    confirm()
                }
                .buttonStyle(.ds(.dangerFill, size: .lg))
            }
        )
        .frame(width: 440)
    }
}
