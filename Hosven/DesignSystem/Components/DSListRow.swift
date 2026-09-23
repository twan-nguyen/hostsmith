import SwiftUI

/// The workhorse row: a glyph, a title, a monospace sublabel, one trailing control.
/// Spec: `components/ListRow/README.md`. Rows live inside one card and are
/// separated by a hairline; hover fills `surfaceHover`.
struct DSListRow<Trailing: View>: View {
    var glyph: String? = nil
    var glyphInWell: Bool = true
    let title: String
    var sublabel: String? = nil
    var disabled: Bool = false
    var showsHairline: Bool = true
    @ViewBuilder var trailing: () -> Trailing

    var body: some View {
        HStack(spacing: DS.Spacing.space5) {
            if let glyph {
                if glyphInWell {
                    DSGlyphWell(systemName: glyph)
                } else {
                    Image(systemName: glyph)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(DS.Colors.inkSecondary)
                        .frame(width: 20)
                }
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(DS.Colors.inkPrimary)
                    .lineLimit(1)
                    .truncationMode(.tail)
                if let sublabel {
                    Text(sublabel)
                        .font(DS.Text.monoCaption)
                        .foregroundStyle(DS.Colors.inkSecondary)
                        .lineLimit(1)
                        .truncationMode(.tail)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            HStack(spacing: DS.Spacing.space4) { trailing() }
        }
        .padding(.horizontal, DS.Spacing.space6)
        .padding(.vertical, DS.Spacing.space5)
        .frame(minHeight: DS.Size.rowHeight)
        .contentShape(Rectangle())
        .dsRowHover()
        .opacity(disabled ? DS.Opacity.disabled : 1)
        .modifier(DSConditionalHairline(enabled: showsHairline))
    }
}

private struct DSConditionalHairline: ViewModifier {
    let enabled: Bool
    func body(content: Content) -> some View {
        if enabled { content.dsHairline(.bottom) } else { content }
    }
}

#Preview("Rows") {
    VStack(spacing: 0) {
        DSListRow(glyph: "network", title: "Acme services", sublabel: "14 entries · 127.0.0.1 → *.acme.test") {
            DSSwitch(isOn: .constant(true))
        }
        DSListRow(glyph: "network", title: "Ad blocking", sublabel: "180 entries · 0.0.0.0 → trackers", showsHairline: false) {
            DSSwitch(isOn: .constant(false))
        }
    }
    .dsContentCard(radius: DS.Radius.md)
    .padding(24)
    .frame(width: 560)
    .background(DS.Colors.surfaceWindow)
}
