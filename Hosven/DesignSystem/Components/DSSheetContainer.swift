import SwiftUI

/// Shell for modal sheets: `surfaceRaised`, header (title in the serif
/// `textTitle`, optional subtitle in `textCallout`), body slot, footer slot.
/// Footer convention: `Huỷ` (bordered, cancel) first, the one default action
/// (`primary` or `dangerFill`, `lg`) last.
struct DSSheetContainer<Body: View, Footer: View>: View {
    let title: String
    var subtitle: String? = nil
    @ViewBuilder let bodyContent: () -> Body
    @ViewBuilder let footer: () -> Footer

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            bodyContent()
                .padding(.horizontal, DS.Spacing.space8)
                .padding(.vertical, DS.Spacing.space6)
            HStack(spacing: DS.Spacing.space4) {
                Spacer(minLength: 0)
                footer()
            }
            .padding(.horizontal, DS.Spacing.space8)
            .padding(.bottom, DS.Spacing.space8)
            .padding(.top, DS.Spacing.space4)
        }
        .background(DS.Colors.surfaceRaised)
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: DS.Spacing.space2) {
            Text(title)
                .font(DS.Text.textTitle)
                .foregroundStyle(DS.Colors.inkPrimary)
            if let subtitle, !subtitle.isEmpty {
                Text(subtitle)
                    .font(DS.Text.textCallout)
                    .foregroundStyle(DS.Colors.inkSecondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, DS.Spacing.space8)
        .padding(.top, DS.Spacing.space8)
        .padding(.bottom, DS.Spacing.space4)
    }
}
