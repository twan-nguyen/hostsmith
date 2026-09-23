import SwiftUI

/// ⌘K command palette overlay. Renders centered above main content.
struct CommandPaletteView: View {
    @Binding var isPresented: Bool
    @Environment(HostsFileManager.self) private var hostsManager
    @Environment(EnvFileManager.self) private var envManager

    let select: (AppSidebarSelection) -> Void

    @State private var viewModel = CommandPaletteViewModel()
    @State private var keyMonitor: Any?
    @FocusState private var queryFocused: Bool

    var body: some View {
        ZStack {
            // Click-outside to dismiss.
            DS.Colors.surfaceScrim
                .ignoresSafeArea()
                .onTapGesture { dismiss() }

            paletteCard
                .frame(maxWidth: 540, maxHeight: 480)
                .padding(.top, 60)
                .frame(maxHeight: .infinity, alignment: .top)
        }
        .transition(.opacity.combined(with: .scale(scale: 0.96)))
        .onAppear {
            viewModel.bind(hosts: hostsManager, env: envManager)
            queryFocused = true
            installKeyMonitor()
        }
        .onDisappear { removeKeyMonitor() }
    }

    /// Window-local NSEvent monitor for arrow nav, Enter, Esc — TextField swallows
    /// SwiftUI `.onKeyPress` for arrows on macOS 14, so we intercept at the AppKit layer.
    /// Lifecycle is tied to View appearance so paired install/remove always run together.
    private func installKeyMonitor() {
        guard keyMonitor == nil else { return }
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            switch event.keyCode {
            case 126: viewModel.moveSelection(by: -1); return nil   // up
            case 125: viewModel.moveSelection(by: 1);  return nil   // down
            case 53:  dismiss(); return nil                          // esc
            case 36, 76: execute(); return nil                       // return
            default: return event
            }
        }
    }

    private func removeKeyMonitor() {
        if let m = keyMonitor { NSEvent.removeMonitor(m) }
        keyMonitor = nil
    }

    private var paletteCard: some View {
        VStack(spacing: 0) {
            searchField
                .padding(.horizontal, 14)
                .padding(.vertical, 10)

            Color.clear.frame(height: 0).dsHairline(.bottom)

            if viewModel.flat.isEmpty {
                emptyState
            } else {
                resultsList
            }
        }
        .dsPopoverSurface(radius: DS.Radius.lg)
    }

    private var searchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(DS.Colors.inkSecondary)

            TextField("", text: $viewModel.query, prompt: Text("Tìm host, repo hoặc lệnh").foregroundStyle(DS.Colors.inkTertiary))
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .foregroundStyle(DS.Colors.inkPrimary)
                .focused($queryFocused)
                .onSubmit { execute() }

            Button(action: dismiss) {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(DS.Colors.inkTertiary)
            }
            .buttonStyle(.plain)
            .opacity(viewModel.query.isEmpty ? 0 : 1)
        }
    }

    private var resultsList: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 2) {
                    ForEach(viewModel.sections) { section in
                        sectionHeader(section.category.title)
                        ForEach(section.items, id: \.id) { command in
                            let absIdx = flatIndex(for: command)
                            CommandPaletteRow(
                                command: command,
                                isSelected: absIdx == viewModel.selectedIndex
                            )
                            .id(command.id)
                            .onTapGesture {
                                viewModel.selectedIndex = absIdx
                                execute()
                            }
                        }
                    }
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
            }
            .frame(maxHeight: 380)
            .onChange(of: viewModel.selectedIndex) { _, _ in
                if let cmd = viewModel.selectedCommand {
                    proxy.scrollTo(cmd.id, anchor: .center)
                }
            }
        }
    }

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(DS.Text.textMenuLabel)
            .foregroundStyle(DS.Colors.inkSecondary)
            .padding(.horizontal, DS.Spacing.space5)
            .padding(.top, DS.Spacing.space5)
            .padding(.bottom, DS.Spacing.space2)
    }

    private var emptyState: some View {
        Text("Không có kết quả")
            .font(DS.Text.textCallout)
            .foregroundStyle(DS.Colors.inkSecondary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 28)
    }

    // MARK: - Helpers

    private func flatIndex(for command: PaletteCommand) -> Int {
        viewModel.flat.firstIndex(where: { $0.id == command.id }) ?? 0
    }

    private func execute() {
        guard let cmd = viewModel.selectedCommand else { return }
        let context = PaletteContext(
            hostsManager: hostsManager,
            envManager: envManager,
            select: select,
            dismiss: { dismiss() }
        )
        cmd.execute(in: context)
    }

    private func dismiss() {
        viewModel.reset()
        isPresented = false
    }
}

