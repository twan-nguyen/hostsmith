import SwiftUI

@main
struct HosvenApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @State private var hostsManager = HostsFileManager()
    @State private var envManager = EnvFileManager()
    @State private var portsManager = PortsManager()
    @State private var autoUpdater = AutoUpdater()
    @AppStorage("showMenuBarExtra") private var showMenuBarExtra: Bool = true
    @AppStorage("sidebarCollapsed") private var sidebarCollapsed = false

    private var appVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? ""
    }

    var body: some Scene {
        WindowGroup(id: "main") {
            MainWindowView()
                .environment(hostsManager)
                .environment(envManager)
                .environment(portsManager)
                .environment(autoUpdater)
                .frame(minWidth: 960, minHeight: 560)
        }
        // .hiddenTitleBar collapses the native title bar so the sidebar's top strip
        // can hold the traffic lights. WindowChromeConfigurator positions them
        // and re-asserts their visibility as defense against AppKit edge cases.
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
        .defaultSize(width: 980, height: 640)
        .commands {
            CommandGroup(after: .appInfo) {
                Button("Kiểm tra cập nhật") { autoUpdater.checkForUpdates() }
                Button("Gửi phản hồi") { AppLinks.openFeedback() }
                Button("Trang chủ dự án") { AppLinks.openHomepage() }
            }
            CommandGroup(replacing: .undoRedo) {
                Button("Hoàn tác") { hostsManager.undo() }
                    .keyboardShortcut("z", modifiers: .command)
                    .disabled(!hostsManager.canUndo)
                Button("Làm lại") { hostsManager.redo() }
                    .keyboardShortcut("z", modifiers: [.command, .shift])
                    .disabled(!hostsManager.canRedo)
            }
            CommandGroup(after: .textEditing) {
                Button("Tìm kiếm") {
                    hostsManager.isSearchFocused = true
                }
                .keyboardShortcut("f", modifiers: .command)
            }
            CommandGroup(after: .sidebar) {
                Button(sidebarCollapsed ? "Hiện thanh bên" : "Ẩn thanh bên") { sidebarCollapsed.toggle() }
                    .keyboardShortcut("s", modifiers: [.control, .command])
            }
            CommandMenu("Profile") {
                profileShortcutCommands
                Divider()
                Button("Bỏ kích hoạt profile") {
                    hostsManager.switchProfile(to: nil)
                }
                .keyboardShortcut("0", modifiers: .command)
            }
        }

        Settings {
            SettingsView()
                .environment(hostsManager)
                .environment(envManager)
        }

        MenuBarExtra(isInserted: $showMenuBarExtra) {
            MenuBarContentView()
                .environment(hostsManager)
                .environment(envManager)
                .environment(portsManager)
        } label: {
            menuBarLabel
        }
        .menuBarExtraStyle(.window)
    }

    /// Status item per `guidelines/20-menu-bar.md`: `network` when no profile is
    /// active, `network.badge.shield.half.filled` when one is. Template-rendered
    /// so macOS tints it; never animated.
    @ViewBuilder
    private var menuBarLabel: some View {
        Image(systemName: hostsManager.activeProfileID == nil ? "network" : "network.badge.shield.half.filled")
    }

    /// One menu item per profile (⌘1..⌘9). Hidden in production menu via empty title? — kept visible
    /// so users can discover the shortcut. macOS folds it into the Profile menu.
    @ViewBuilder
    private var profileShortcutCommands: some View {
        ForEach(hostsManager.profiles, id: \.id) { profile in
            if let n = profile.shortcutNumber, (1...9).contains(n) {
                Button("Kích hoạt \(profile.name)") {
                    hostsManager.activateProfile(id: profile.id)
                }
                .keyboardShortcut(KeyEquivalent(Character("\(n)")), modifiers: .command)
            }
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        if shouldHideDockAfterLastWindowClosed {
            sender.setActivationPolicy(.accessory)
        }
        return false
    }

    func applicationWillTerminate(_ notification: Notification) {
        releaseAuthorization()
    }

    private var shouldHideDockAfterLastWindowClosed: Bool {
        let defaults = UserDefaults.standard
        if defaults.object(forKey: "showMenuBarExtra") == nil {
            return true
        }
        return defaults.bool(forKey: "showMenuBarExtra")
    }
}
