import SwiftUI

/// Names an environment wherever one appears. Spec: `components/EnvChip/README.md`.
/// Fixed mapping: Local green, Staging amber, Production red; anything else is
/// neutral and relies on its name. Always shows the word.
struct DSChip: View {
    enum Style { case tint, solid }

    let text: String
    var color: ProfileColor? = nil
    var style: Style = .tint

    var body: some View {
        Text(text)
            .font(DS.Text.textMenuLabel)
            .lineLimit(1)
            .foregroundStyle(foreground)
            .padding(.horizontal, DS.Spacing.space4)
            .frame(height: 22)
            .background(Capsule(style: .continuous).fill(background))
    }

    private var isNeutral: Bool {
        guard let color else { return true }
        return color == .green || color == .purple
    }

    private var foreground: Color {
        guard let color, !isNeutral else { return DS.Colors.inkSecondary }
        switch style {
        case .tint: return Color.ds(color)
        case .solid: return DS.Colors.inkOnStatus
        }
    }

    private var background: Color {
        guard let color, !isNeutral else { return DS.Colors.surfaceSunken }
        switch style {
        case .tint: return Color.dsTint(color)
        case .solid: return Color.ds(color)
        }
    }
}

#Preview("Chips") {
    HStack(spacing: 8) {
        DSChip(text: "Local", color: .blue)
        DSChip(text: "Staging", color: .amber)
        DSChip(text: "Production", color: .red)
        DSChip(text: "Preview", color: .purple)
        DSChip(text: "Staging", color: .amber, style: .solid)
    }
    .padding(24)
    .background(DS.Colors.surfaceContent)
}
