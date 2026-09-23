import SwiftUI

/// Create or rename a hosts profile and pick its environment colour.
struct ProfileFormSheet: View {
    enum Mode: Identifiable {
        case create
        case rename(Profile)

        var id: String {
            switch self {
            case .create: return "create"
            case .rename(let p): return "rename-\(p.id.uuidString)"
            }
        }
    }

    let mode: Mode
    @Environment(HostsFileManager.self) private var hostsManager
    @Environment(\.dismiss) private var dismiss

    @State private var name = ""
    @State private var color: ProfileColor = .blue
    @State private var invalidReason: String?

    private var isRename: Bool {
        if case .rename = mode { return true }
        return false
    }

    var body: some View {
        DSSheetContainer(
            title: isRename ? "Đổi tên profile" : "Profile mới",
            subtitle: "Một profile nhóm các entry trong /etc/hosts để bật tắt cùng nhau.",
            bodyContent: {
                VStack(alignment: .leading, spacing: DS.Spacing.space6) {
                    DSField("Tên profile", text: $name, prompt: "Tên profile", autocorrect: false, invalidReason: invalidReason)
                    VStack(alignment: .leading, spacing: DS.Spacing.space3) {
                        Text("Môi trường")
                            .font(DS.Text.textMenuLabel)
                            .foregroundStyle(DS.Colors.inkSecondary)
                        ProfileColorPicker(selection: $color)
                    }
                }
            },
            footer: {
                Button("Huỷ") { dismiss() }
                    .buttonStyle(.ds(.bordered, size: .lg))
                    .keyboardShortcut(.escape)
                Button(isRename ? "Đổi tên" : "Tạo profile") { save() }
                    .buttonStyle(.ds(.primary, size: .lg))
                    .keyboardShortcut(.return)
            }
        )
        .frame(width: 420)
        .onAppear {
            if case .rename(let profile) = mode {
                name = profile.name
                color = profile.color
            }
        }
    }

    private func save() {
        invalidReason = nil
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard Profile(name: trimmed, color: color).isNameValid else {
            invalidReason = "Tên không được để trống và không chứa [ hoặc ]"
            return
        }
        switch mode {
        case .create:
            guard hostsManager.addProfile(name: trimmed, color: color) != nil else {
                invalidReason = "Đã có profile tên \(trimmed)"
                return
            }
        case .rename(let profile):
            if profile.name != trimmed, !hostsManager.renameProfile(id: profile.id, to: trimmed) {
                invalidReason = "Đã có profile tên \(trimmed)"
                return
            }
            if profile.color != color {
                hostsManager.updateProfileColor(id: profile.id, color: color)
            }
        }
        dismiss()
    }
}

/// The three environment hues plus neutral, as selectable chips.
struct ProfileColorPicker: View {
    @Binding var selection: ProfileColor

    var body: some View {
        HStack(spacing: DS.Spacing.space3) {
            ForEach(ProfileColor.pickerCases) { color in
                Button {
                    selection = color
                } label: {
                    DSChip(text: color.displayName, color: color, style: selection == color ? .solid : .tint)
                        .overlay(
                            Capsule(style: .continuous)
                                .strokeBorder(DS.Colors.focusRing, lineWidth: selection == color ? 2 : 0)
                                .padding(-3)
                        )
                }
                .buttonStyle(.plain)
                .focusEffectDisabled()
                .accessibilityLabel(color.displayName)
                .accessibilityAddTraits(selection == color ? [.isSelected] : [])
            }
        }
    }
}
