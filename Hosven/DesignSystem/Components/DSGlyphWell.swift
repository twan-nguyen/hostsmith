import SwiftUI

/// A 30pt `Radius.sm` tile on `surfaceSunken` holding an SF Symbol in `inkSecondary`.
struct DSGlyphWell: View {
    let systemName: String

    var body: some View {
        Image(systemName: systemName)
            .font(.system(size: 14, weight: .medium))
            .foregroundStyle(DS.Colors.inkSecondary)
            .frame(width: 30, height: 30)
            .background(
                RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                    .fill(DS.Colors.surfaceSunken)
            )
    }
}
