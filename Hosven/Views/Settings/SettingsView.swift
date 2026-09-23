import SwiftUI

/// Settings scene: General (appearance, menu bar, editor) and Profiles.
struct SettingsView: View {
    @Environment(HostsFileManager.self) private var hostsManager

    var body: some View {
        TabView {
            generalTab
                .tabItem { Label("Chung", systemImage: "gearshape") }
            profilesTab
                .tabItem { Label("Profiles", systemImage: "square.stack.3d.up") }
        }
        .frame(width: 480, height: 380)
    }

    // MARK: - General

    private var generalTab: some View {
        Form {
            Section("Giao diện") {
                @AppStorage("appearanceMode") var raw: String = AppearanceMode.system.rawValue
                Picker("Chế độ", selection: $raw) {
                    ForEach(AppearanceMode.allCases) { mode in
                        Label(mode.label, systemImage: mode.icon).tag(mode.rawValue)
                    }
                }
                .pickerStyle(.menu)
            }

            Section("Trình soạn thảo") {
                @AppStorage("editorFontSize") var editorFontSize: Double = 13
                Stepper(value: $editorFontSize, in: 10...20, step: 1) {
                    HStack {
                        Text("Cỡ chữ")
                        Spacer()
                        Text("\(Int(editorFontSize)) pt")
                            .font(DS.Text.monoCaption)
                            .foregroundStyle(DS.Colors.inkSecondary)
                    }
                }
                Text("Cỡ chữ của trình soạn thảo hosts và .env, độc lập với cỡ chữ hệ thống.")
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkSecondary)
            }

            Section("Menu bar") {
                @AppStorage("showMenuBarExtra") var showMenuBar: Bool = true
                Toggle("Hiện ở menu bar", isOn: $showMenuBar)
                Text("Bật tắt block và kích hoạt profile từ menu bar mà không cần mở cửa sổ.")
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkSecondary)
            }

            Section("Hệ thống") {
                Text("Hosven \(appVersion)")
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkSecondary)
            }
        }
        .formStyle(.grouped)
    }

    // MARK: - Profiles

    private var profilesTab: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Profiles")
                    .font(DS.Text.textHeadline)
                    .foregroundStyle(DS.Colors.inkPrimary)
                Spacer()
                Text("\(hostsManager.profiles.count) profile")
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.inkSecondary)
            }
            .padding(.horizontal, DS.Spacing.space6)
            .padding(.vertical, DS.Spacing.space5)

            if hostsManager.profiles.isEmpty {
                DSEmptyState(icon: "square.stack.3d.up", title: "Chưa có profile", message: "Một profile nhóm các entry trong /etc/hosts để bật tắt cùng nhau. Thêm profile từ nhóm Profiles trong cửa sổ chính.")
            } else {
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach(Array(hostsManager.profiles.enumerated()), id: \.element.id) { index, profile in
                            DSListRow(
                                glyph: "square.stack.3d.up",
                                title: profile.name,
                                sublabel: profile.shortcutNumber.map { "⌘\($0) · \(hostsManager.tagEntryCount(name: profile.name)) entry" } ?? "\(hostsManager.tagEntryCount(name: profile.name)) entry",
                                showsHairline: index < hostsManager.profiles.count - 1
                            ) {
                                DSChip(text: profile.color.displayName, color: profile.color)
                                Button("Xoá") { hostsManager.removeProfile(id: profile.id) }
                                    .buttonStyle(.ds(.danger, size: .sm))
                            }
                        }
                    }
                    .dsContentCard(radius: DS.Radius.md)
                    .padding(.horizontal, DS.Spacing.space6)
                    .padding(.bottom, DS.Spacing.space6)
                }
            }
        }
        .background(DS.Colors.surfaceWindow)
    }

    private var appVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "?"
    }
}
