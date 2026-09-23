import SwiftUI

/// Labeled field block: `textMenuLabel` label above a `DSTextField`, with an
/// optional helper row. Legacy wrapper around `DSTextField`; keep for sheets.
struct DSField<Helper: View>: View {
    let label: String
    @Binding var text: String
    var prompt: String? = nil
    var monospaced: Bool = false
    var autocorrect: Bool = true
    var invalidReason: String? = nil
    @ViewBuilder var helper: () -> Helper

    init(
        _ label: String,
        text: Binding<String>,
        prompt: String? = nil,
        monospaced: Bool = false,
        autocorrect: Bool = true,
        invalidReason: String? = nil,
        @ViewBuilder helper: @escaping () -> Helper = { EmptyView() }
    ) {
        self.label = label
        self._text = text
        self.prompt = prompt
        self.monospaced = monospaced
        self.autocorrect = autocorrect
        self.invalidReason = invalidReason
        self.helper = helper
    }

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Spacing.space3) {
            Text(label)
                .font(DS.Text.textMenuLabel)
                .foregroundStyle(DS.Colors.inkSecondary)

            DSTextField(
                placeholder: prompt ?? "",
                text: $text,
                mono: monospaced,
                invalidReason: invalidReason,
                autocorrect: autocorrect
            )

            helper()
        }
    }
}
