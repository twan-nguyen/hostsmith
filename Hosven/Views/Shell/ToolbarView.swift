import SwiftUI

/// A 60pt strip on the ivory ground with a hairline underneath.
/// Spec: `components/Toolbar/README.md`. Leading: the serif `textTitle` with the
/// full path beside it in `monoCaption`, truncated in the middle. Trailing: the
/// controls straight on the strip, `Apply` always last.
struct ToolbarView<Trailing: View>: View {
    let title: String
    var path: String? = nil
    @ViewBuilder var trailing: () -> Trailing

    @Environment(\.controlActiveState) private var activeState

    var body: some View {
        HStack(spacing: DS.Spacing.space4) {
            HStack(alignment: .firstTextBaseline, spacing: DS.Spacing.space4) {
                Text(title)
                    .font(DS.Text.textTitle)
                    .foregroundStyle(DS.Colors.inkPrimary)
                    .lineLimit(1)
                    .fixedSize(horizontal: true, vertical: false)
                if let path {
                    Text(path)
                        .font(DS.Text.monoCaption)
                        .foregroundStyle(DS.Colors.inkSecondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }

            Spacer(minLength: DS.Spacing.space6)

            HStack(spacing: DS.Spacing.space4) { trailing() }
                .layoutPriority(1)
                .opacity(activeState == .key ? 1 : DS.Opacity.inactiveWindow)
        }
        .frame(height: 60)
        .padding(.horizontal, DS.Spacing.space7)
        .dsHairline(.bottom)
    }
}

/// Status readout for a toolbar: a `StatusDot` with its word, sitting on the strip.
struct DSToolbarStatus: View {
    let state: StatusDot.State
    let label: String

    var body: some View {
        StatusDot(state: state, label: label)
            .fixedSize(horizontal: true, vertical: false)
            .padding(.trailing, DS.Spacing.space2)
    }
}

#Preview("Hosts toolbar") {
    ToolbarView(title: "Hosts hệ thống", path: "/etc/hosts") {
        DSToolbarStatus(state: .warn, label: "3 chưa áp dụng")
        DSSearchField(placeholder: "Tìm entry", text: .constant(""))
        Button("Áp dụng") {}.buttonStyle(.dsPrimary)
    }
    .frame(width: 900)
    .background(DS.Colors.surfaceWindow)
}
