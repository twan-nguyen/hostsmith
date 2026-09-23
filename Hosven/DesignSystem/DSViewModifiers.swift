import SwiftUI

// MARK: - Shadow

/// Draws the layers of a `DS.Shadow` under `content`. Shadows are rare in this
/// design: a switch knob, the active segment tile, and whatever opens over the window.
struct DSShadowModifier: ViewModifier {
    let shadow: DS.Shadow

    func body(content: Content) -> some View {
        var view = AnyView(content)
        for layer in shadow.layers {
            view = AnyView(view.shadow(color: layer.color, radius: layer.radius, y: layer.y))
        }
        return view
    }
}

// MARK: - Structural line colour

/// `hairline` normally; `borderControl` under Increase Contrast, so every
/// structural line stays findable.
struct DSHairlineColor {
    static func resolve(_ contrast: ColorSchemeContrast, strong: Bool = false) -> Color {
        if contrast == .increased { return DS.Colors.borderControl }
        return strong ? DS.Colors.hairlineStrong : DS.Colors.hairline
    }
}

// MARK: - Content card (level 0, bordered)

/// A white card on the ivory ground: `surfaceContent` with a `hairline` border
/// at `Radius.md`. No shadow; structure is drawn with the line.
struct DSContentCardModifier: ViewModifier {
    var radius: CGFloat = DS.Radius.md
    @Environment(\.colorSchemeContrast) private var contrast

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: radius, style: .continuous)
        content
            .background(shape.fill(DS.Colors.surfaceContent))
            .clipShape(shape)
            .overlay(shape.strokeBorder(DSHairlineColor.resolve(contrast), lineWidth: DS.Size.hairlineWidth))
    }
}

// MARK: - Popover surface (level 1)

/// Anything that opens over the window: popovers, sheets, the command palette
/// and the menu bar panel. `surfaceRaised` (or `surfaceMenubar` when `menubar`
/// is set) with a `hairline` edge and `shadowPopover`. Opaque under Reduce Transparency.
struct DSPopoverSurfaceModifier: ViewModifier {
    let radius: CGFloat
    var menubar: Bool = false
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.colorSchemeContrast) private var contrast

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: radius, style: .continuous)
        content
            .background(shape.fill(menubar && !reduceTransparency ? DS.Colors.surfaceMenubar : DS.Colors.surfaceRaised))
            .clipShape(shape)
            .overlay(shape.strokeBorder(DSHairlineColor.resolve(contrast), lineWidth: DS.Size.hairlineWidth))
            .modifier(DSShadowModifier(shadow: .popover))
    }
}

// MARK: - Hairline

/// The structural line: around every card, under the toolbar, between rows,
/// along the sidebar's trailing edge. Becomes `borderControl` under Increase Contrast.
struct DSHairlineModifier: ViewModifier {
    let edge: Edge
    var inset: CGFloat = 0
    @Environment(\.colorSchemeContrast) private var contrast

    func body(content: Content) -> some View {
        content.overlay(alignment: alignment) {
            Rectangle()
                .fill(DSHairlineColor.resolve(contrast))
                .frame(
                    width: isHorizontal ? nil : DS.Size.hairlineWidth,
                    height: isHorizontal ? DS.Size.hairlineWidth : nil
                )
                .padding(isHorizontal ? .horizontal : .vertical, inset)
        }
    }

    private var isHorizontal: Bool { edge == .top || edge == .bottom }
    private var alignment: Alignment {
        switch edge {
        case .top: return .top
        case .bottom: return .bottom
        case .leading: return .leading
        case .trailing: return .trailing
        }
    }
}

// MARK: - Focus ring

/// Solid `focusRing`, `focusRingWidth` thick, drawn 2pt outside the control
/// and following its own radius. Never suppressed.
struct DSFocusRingModifier: ViewModifier {
    let isFocused: Bool
    var radius: CGFloat = DS.Radius.sm

    func body(content: Content) -> some View {
        content.overlay {
            if isFocused {
                RoundedRectangle(cornerRadius: radius + DS.Size.focusRingWidth + 1, style: .continuous)
                    .strokeBorder(DS.Colors.focusRing, lineWidth: DS.Size.focusRingWidth)
                    .padding(-(DS.Size.focusRingWidth + 1))
                    .allowsHitTesting(false)
            }
        }
    }
}

// MARK: - Row hover

/// Hover fill for a list row: `surfaceHover` at `Radius.sm`. Text colour does not change.
struct DSRowHoverModifier: ViewModifier {
    @State private var isHovering = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            .background(
                RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                    .fill(isHovering ? DS.Colors.surfaceHover : Color.clear)
            )
            .animation(reduceMotion ? nil : .dsSwitch, value: isHovering)
            .onHover { isHovering = $0 }
    }
}

// MARK: - Pane content

/// The content area under a pane's toolbar: `paneInset` on top, `space8` on
/// the sides and bottom, as every screen in the design draws it.
struct DSPaneContentModifier: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding(.top, DS.Size.paneInset)
            .padding([.horizontal, .bottom], DS.Spacing.space8)
    }
}

// MARK: - Convenience

extension View {
    func dsPopoverSurface(radius: CGFloat = DS.Radius.lg, menubar: Bool = false) -> some View {
        modifier(DSPopoverSurfaceModifier(radius: radius, menubar: menubar))
    }

    func dsContentCard(radius: CGFloat = DS.Radius.md) -> some View {
        modifier(DSContentCardModifier(radius: radius))
    }

    func dsShadow(_ shadow: DS.Shadow) -> some View {
        modifier(DSShadowModifier(shadow: shadow))
    }

    func dsHairline(_ edge: Edge = .bottom, inset: CGFloat = 0) -> some View {
        modifier(DSHairlineModifier(edge: edge, inset: inset))
    }

    func dsFocusRing(_ isFocused: Bool, radius: CGFloat = DS.Radius.sm) -> some View {
        modifier(DSFocusRingModifier(isFocused: isFocused, radius: radius))
    }

    func dsRowHover() -> some View {
        modifier(DSRowHoverModifier())
    }

    func dsPaneContent() -> some View {
        modifier(DSPaneContentModifier())
    }
}
