import SwiftUI

/// Single-line input. Spec: `components/TextField/README.md`.
/// White with a `hairlineStrong` border at `Radius.sm`, `controlHeightMd`
/// (`lg` in a toolbar). `mono` for anything a developer could paste into a
/// terminal. Focus firms the border to `borderControl` under the focus ring;
/// invalid draws `danger` and puts the reason in `textCaption` below the field.
struct DSTextField: View {
    let placeholder: String
    @Binding var text: String
    var mono: Bool = false
    var invalidReason: String? = nil
    var size: DSButtonStyle.Size = .md
    var autocorrect: Bool = false
    var onSubmit: (() -> Void)? = nil

    @FocusState private var focused: Bool
    @Environment(\.isEnabled) private var isEnabled
    @Environment(\.colorSchemeContrast) private var contrast

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous) }

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Spacing.space2) {
            TextField("", text: $text, prompt: Text(placeholder).foregroundStyle(DS.Colors.inkTertiary))
                .textFieldStyle(.plain)
                .font(mono ? DS.Text.monoBody : DS.Text.textBody)
                .foregroundStyle(DS.Colors.inkPrimary)
                .autocorrectionDisabled(!autocorrect)
                .focused($focused)
                .onSubmit { onSubmit?() }
                .padding(.horizontal, DS.Spacing.space5)
                .frame(height: size == .lg ? DS.Size.controlHeightLg : DS.Size.controlHeightMd)
                .background(shape.fill(isEnabled ? DS.Colors.surfaceContent : DS.Colors.surfaceSunken))
                .overlay(shape.strokeBorder(borderColor, lineWidth: DS.Size.hairlineWidth))
                .dsFocusRing(focused)
                .opacity(isEnabled ? 1 : DS.Opacity.disabled)

            if let invalidReason {
                Text(invalidReason)
                    .font(DS.Text.textCaption)
                    .foregroundStyle(DS.Colors.danger)
            }
        }
    }

    private var borderColor: Color {
        if invalidReason != nil { return DS.Colors.danger }
        if focused { return DS.Colors.borderControl }
        return DSHairlineColor.resolve(contrast, strong: true)
    }
}

/// Filter field for a list: `surfaceSidebar` ground, no border, a leading
/// magnifier, filters as the person types.
struct DSSearchField: View {
    let placeholder: String
    @Binding var text: String
    var size: DSButtonStyle.Size = .md
    var isFocused: FocusState<Bool>.Binding? = nil

    @FocusState private var localFocus: Bool

    var body: some View {
        HStack(spacing: DS.Spacing.space3) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(DS.Colors.inkSecondary)
            TextField("", text: $text, prompt: Text(placeholder).foregroundStyle(DS.Colors.inkTertiary))
                .textFieldStyle(.plain)
                .font(DS.Text.textBody)
                .foregroundStyle(DS.Colors.inkPrimary)
                .autocorrectionDisabled()
                .focused(isFocused ?? $localFocus)
            if !text.isEmpty {
                Button {
                    text = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 12))
                        .foregroundStyle(DS.Colors.inkTertiary)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Xoá tìm kiếm")
            }
        }
        .padding(.horizontal, DS.Spacing.space5)
        .frame(height: size == .lg ? DS.Size.controlHeightLg : DS.Size.controlHeightMd)
        .frame(minWidth: 110, idealWidth: 200, maxWidth: 200)
        .background(
            RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                .fill(DS.Colors.surfaceSidebar)
        )
        .dsFocusRing(isFocused?.wrappedValue ?? localFocus)
    }
}

#Preview("Fields") {
    VStack(spacing: 16) {
        DSTextField(placeholder: "Tên profile", text: .constant(""))
        DSTextField(placeholder: "Địa chỉ IP", text: .constant("127.0.0.1"), mono: true)
        DSTextField(placeholder: "Địa chỉ IP", text: .constant("999.1"), mono: true, invalidReason: "Không phải địa chỉ IPv4 hoặc IPv6 hợp lệ")
        DSSearchField(placeholder: "Tìm entry", text: .constant(""))
    }
    .padding(24)
    .frame(width: 360)
    .background(DS.Colors.surfaceWindow)
}
