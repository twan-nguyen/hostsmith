import SwiftUI

/// Rounded push button. Spec: `components/Button/README.md`.
/// White with a `hairlineStrong` border by default; `primary` is the one clay
/// action in a view; `plain` sits inside a row; `danger` is danger text on the
/// bordered button; `dangerFill` only confirms inside a sheet. Height follows
/// position: `lg` in a sheet, `md` everywhere, `sm` inside a table row.
struct DSButtonStyle: ButtonStyle {
    enum Emphasis { case bordered, primary, plain, danger, dangerFill }
    enum Size { case sm, md, lg }

    var emphasis: Emphasis = .bordered
    var size: Size = .md

    func makeBody(configuration: Configuration) -> some View {
        DSButtonBody(configuration: configuration, emphasis: emphasis, size: size)
    }
}

private struct DSButtonBody: View {
    let configuration: ButtonStyleConfiguration
    let emphasis: DSButtonStyle.Emphasis
    let size: DSButtonStyle.Size

    @State private var isHovering = false
    @Environment(\.isEnabled) private var isEnabled
    @Environment(\.isFocused) private var isFocused
    @Environment(\.colorSchemeContrast) private var contrast

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous) }

    var body: some View {
        configuration.label
            .font(font)
            .foregroundStyle(foreground)
            .lineLimit(1)
            .fixedSize(horizontal: true, vertical: false)
            .padding(.horizontal, horizontalPadding)
            .frame(height: height)
            .background(shape.fill(fill))
            .overlay(shape.strokeBorder(border, lineWidth: DS.Size.hairlineWidth))
            .opacity(isEnabled ? 1 : DS.Opacity.disabled)
            .opacity(configuration.isPressed ? 0.85 : 1)
            .dsFocusRing(isFocused)
            .contentShape(shape)
            .onHover { isHovering = $0 }
    }

    private var height: CGFloat {
        switch size {
        case .sm: return DS.Size.controlHeightSm
        case .md: return DS.Size.controlHeightMd
        case .lg: return DS.Size.controlHeightLg
        }
    }

    private var horizontalPadding: CGFloat {
        switch size {
        case .sm: return DS.Spacing.space5
        case .md: return DS.Spacing.space6
        case .lg: return DS.Spacing.space7
        }
    }

    private var font: Font {
        .system(size: size == .sm ? 13 : 14, weight: .medium)
    }

    private var foreground: Color {
        switch emphasis {
        case .primary: return DS.Colors.inkOnAccent
        case .dangerFill: return DS.Colors.inkOnStatus
        case .danger: return DS.Colors.danger
        case .bordered, .plain: return DS.Colors.inkPrimary
        }
    }

    private var fill: Color {
        switch emphasis {
        case .primary: return isHovering ? DS.Colors.accentHover : DS.Colors.accent
        case .dangerFill: return DS.Colors.danger
        case .plain: return isHovering ? DS.Colors.surfaceHover : Color.clear
        case .bordered, .danger: return isHovering ? DS.Colors.surfaceHover : DS.Colors.surfaceContent
        }
    }

    private var border: Color {
        switch emphasis {
        case .primary: return isHovering ? DS.Colors.accentHover : DS.Colors.accent
        case .dangerFill: return DS.Colors.danger
        case .plain: return Color.clear
        case .bordered, .danger: return DSHairlineColor.resolve(contrast, strong: true)
        }
    }
}

extension ButtonStyle where Self == DSButtonStyle {
    static var dsBordered: DSButtonStyle { DSButtonStyle() }
    static var dsPrimary: DSButtonStyle { DSButtonStyle(emphasis: .primary) }
    static var dsPlain: DSButtonStyle { DSButtonStyle(emphasis: .plain) }
    static var dsDanger: DSButtonStyle { DSButtonStyle(emphasis: .danger) }
    static var dsDangerFill: DSButtonStyle { DSButtonStyle(emphasis: .dangerFill) }

    static func ds(_ emphasis: DSButtonStyle.Emphasis = .bordered, size: DSButtonStyle.Size = .md) -> DSButtonStyle {
        DSButtonStyle(emphasis: emphasis, size: size)
    }
}

#Preview("Emphases") {
    VStack(spacing: 12) {
        HStack {
            Button("Áp dụng") {}.buttonStyle(.ds(.primary, size: .lg))
            Button("Huỷ") {}.buttonStyle(.dsBordered)
            Button("Hiện") {}.buttonStyle(.ds(.plain, size: .sm))
            Button("Xoá entry") {}.buttonStyle(.dsDanger)
            Button("Xoá profile") {}.buttonStyle(.dsDangerFill)
            Button("Tắt") {}.buttonStyle(.dsBordered).disabled(true)
        }
    }
    .padding(24)
    .background(DS.Colors.surfaceWindow)
}
