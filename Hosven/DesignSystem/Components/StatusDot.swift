import SwiftUI

/// An 8pt dot plus the word for the state. Spec: `components/StatusDot/README.md`.
/// The word is not optional; the accessibility label is the word.
struct StatusDot: View {
    enum State {
        case ok, warn, danger, idle

        var color: Color {
            switch self {
            case .ok: return DS.Colors.success
            case .warn: return DS.Colors.warning
            case .danger: return DS.Colors.danger
            case .idle: return DS.Colors.controlTrack
            }
        }
    }

    let state: State
    let label: String
    var labelColor: Color = DS.Colors.inkPrimary

    var body: some View {
        HStack(spacing: DS.Spacing.space3) {
            dot
            Text(label)
                .font(DS.Text.textCallout)
                .foregroundStyle(labelColor)
                .lineLimit(1)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
    }

    private var dot: some View {
        Circle()
            .fill(state.color)
            .overlay(
                Circle().strokeBorder(DS.Colors.borderControl, lineWidth: state == .idle ? 1 : 0)
            )
            .frame(width: 8, height: 8)
    }
}

#Preview("States") {
    VStack(alignment: .leading, spacing: 12) {
        StatusDot(state: .ok, label: "Đã áp dụng 12 entry")
        StatusDot(state: .warn, label: "3 chưa áp dụng")
        StatusDot(state: .danger, label: "Ghi thất bại")
        StatusDot(state: .idle, label: "Không hoạt động")
    }
    .padding(24)
    .background(DS.Colors.surfaceContent)
}
