import SwiftUI

/// Inline message at the top of a pane, a sheet or the menu bar panel.
/// Spec: `components/Banner/README.md`. Four levels; the title is the whole
/// message stated as a fact with a number; at most two actions, the
/// rightmost being the recovery. A success banner without actions fades
/// after five seconds through `onAutoDismiss`.
struct DSBanner: View {
    enum Level {
        case info, success, warning, danger

        var tint: Color {
            switch self {
            case .info: return DS.Colors.accentTint
            case .success: return DS.Colors.successTint
            case .warning: return DS.Colors.warningTint
            case .danger: return DS.Colors.dangerTint
            }
        }

        var ink: Color {
            switch self {
            case .info: return DS.Colors.accent
            case .success: return DS.Colors.success
            case .warning: return DS.Colors.warning
            case .danger: return DS.Colors.danger
            }
        }

        var glyph: String {
            switch self {
            case .info: return "info.circle"
            case .success: return "checkmark.circle.fill"
            case .warning: return "exclamationmark.triangle.fill"
            case .danger: return "xmark.octagon.fill"
            }
        }
    }

    struct Action: Identifiable {
        let id = UUID()
        let label: String
        let action: () -> Void
    }

    let level: Level
    let title: String
    var detail: String? = nil
    /// Overrides the level glyph, e.g. `lock.fill` before an administrator write.
    var glyph: String? = nil
    var actions: [Action] = []
    var onAutoDismiss: (() -> Void)? = nil

    var body: some View {
        HStack(alignment: .top, spacing: DS.Spacing.space5) {
            Image(systemName: glyph ?? level.glyph)
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(level.ink)
                .padding(.top, 2)

            VStack(alignment: .leading, spacing: DS.Spacing.space1) {
                Text(title)
                    .font(DS.Text.textHeadline)
                    .foregroundStyle(DS.Colors.inkPrimary)
                if let detail {
                    Text(detail)
                        .font(DS.Text.textCallout)
                        .foregroundStyle(DS.Colors.inkSecondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            if !actions.isEmpty {
                HStack(spacing: DS.Spacing.space4) {
                    ForEach(actions) { action in
                        Button(action.label, action: action.action)
                            .buttonStyle(.dsBordered)
                    }
                }
            }
        }
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space5)
        .fixedSize(horizontal: false, vertical: true)
        .background(
            RoundedRectangle(cornerRadius: DS.Radius.md, style: .continuous)
                .fill(level.tint)
        )
        .accessibilityElement(children: .combine)
        .task(id: title) {
            guard level == .success, actions.isEmpty, let onAutoDismiss else { return }
            try? await Task.sleep(for: .seconds(5))
            if !Task.isCancelled { onAutoDismiss() }
        }
    }
}

#Preview("Levels") {
    VStack(spacing: 12) {
        DSBanner(level: .info, title: "Áp dụng sẽ ghi 3 thay đổi vào /etc/hosts", detail: "Cần quyền quản trị một lần mỗi phiên. ⌘Z hoàn tác sau khi ghi.", glyph: "lock.fill")
        DSBanner(level: .success, title: "Đã ghi 12 entry vào /etc/hosts")
        DSBanner(level: .warning, title: "/etc/hosts đã thay đổi bên ngoài Hosven", detail: "Tải lại để xem nội dung mới nhất.", actions: [.init(label: "Tải lại") {}])
        DSBanner(level: .danger, title: "Không ghi được /etc/hosts vì quyền quản trị bị từ chối", detail: "Thử lại và chấp nhận hộp thoại xác thực.", actions: [.init(label: "Thử lại") {}])
    }
    .padding(24)
    .frame(width: 720)
    .background(DS.Colors.surfaceWindow)
}
