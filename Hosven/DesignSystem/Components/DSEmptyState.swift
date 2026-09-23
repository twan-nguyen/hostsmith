import SwiftUI

/// Every pane's tutorial: a 28pt `inkTertiary` symbol, a `textTitle` line that
/// names the thing, a `textCallout` sentence that says why it is empty, and
/// one button that fixes it. Drawn inside a dashed `hairlineStrong` frame at
/// `Radius.md`, as the screens do. Spec: `guidelines/30-interaction.md`.
struct DSEmptyState: View {
    let icon: String
    let title: String
    var message: String? = nil
    var actionLabel: String? = nil
    var action: (() -> Void)? = nil

    @Environment(\.colorSchemeContrast) private var contrast

    var body: some View {
        VStack(spacing: DS.Spacing.space4) {
            Image(systemName: icon)
                .font(.system(size: 28, weight: .medium))
                .foregroundStyle(DS.Colors.inkTertiary)
                .padding(.bottom, DS.Spacing.space2)
            Text(title)
                .font(DS.Text.textTitle)
                .foregroundStyle(DS.Colors.inkPrimary)
            if let message {
                Text(message)
                    .font(DS.Text.textCallout)
                    .foregroundStyle(DS.Colors.inkTertiary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 440)
            }
            if let actionLabel, let action {
                Button(actionLabel, action: action)
                    .buttonStyle(.dsPrimary)
                    .padding(.top, DS.Spacing.space4)
            }
        }
        .padding(DS.Spacing.space8)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .overlay(
            RoundedRectangle(cornerRadius: DS.Radius.md, style: .continuous)
                .strokeBorder(DSHairlineColor.resolve(contrast, strong: true), style: StrokeStyle(lineWidth: DS.Size.hairlineWidth, dash: [4, 4]))
        )
    }
}

/// Loading state: indeterminate spinner beside a `textCallout` label.
struct DSLoadingState: View {
    let label: String

    var body: some View {
        HStack(spacing: DS.Spacing.space4) {
            ProgressView().controlSize(.small)
            Text(label)
                .font(DS.Text.textCallout)
                .foregroundStyle(DS.Colors.inkSecondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

#Preview {
    DSEmptyState(
        icon: "folder",
        title: "Chưa có repo",
        message: "Thêm thư mục của một project và Hosven sẽ tìm các file .env bên trong.",
        actionLabel: "Thêm repo",
        action: {}
    )
    .frame(width: 600, height: 360)
    .padding(24)
    .background(DS.Colors.surfaceWindow)
}
