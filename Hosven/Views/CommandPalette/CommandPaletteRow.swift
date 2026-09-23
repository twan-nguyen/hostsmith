import SwiftUI

/// One row in the command palette: glyph well, title, optional subtitle, shortcut hint.
/// Selection is the table rule: `accentTint` fill with `inkPrimary` text.
struct CommandPaletteRow: View {
    let command: PaletteCommand
    let isSelected: Bool

    private var ink: Color { DS.Colors.inkPrimary }
    private var secondary: Color { DS.Colors.inkSecondary }

    var body: some View {
        HStack(spacing: DS.Spacing.space4) {
            Image(systemName: command.icon)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(command.iconTint)
                .frame(width: 28, height: 28)
                .background(
                    RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                        .fill(DS.Colors.surfaceSunken)
                )

            VStack(alignment: .leading, spacing: 1) {
                Text(command.title)
                    .font(DS.Text.textBody)
                    .foregroundStyle(ink)
                    .lineLimit(1)
                if let sub = command.subtitle, !sub.isEmpty {
                    Text(sub)
                        .font(DS.Text.monoCaption)
                        .foregroundStyle(secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }

            Spacer(minLength: DS.Spacing.space4)

            if let hint = command.shortcutHint {
                Text(hint)
                    .font(DS.Text.textCaption)
                    .foregroundStyle(secondary)
            }
        }
        .padding(.horizontal, DS.Spacing.space5)
        .padding(.vertical, DS.Spacing.space2)
        .frame(minHeight: DS.Size.rowHeight)
        .background(
            RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                .fill(isSelected ? DS.Colors.accentTint : Color.clear)
        )
        .contentShape(Rectangle())
    }
}
