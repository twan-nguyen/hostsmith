import SwiftUI

/// Switches between views of the same content. Spec: `components/SegmentedControl/README.md`.
/// Two to four segments on a `surfaceSunken` track at `Radius.sm`; the active
/// segment is a raised `surfaceRaised` tile with `shadowControl`, never the accent.
struct DSSegmentedControl<Value: Hashable>: View {
    struct Segment: Identifiable {
        let value: Value
        let label: String
        var id: Value { value }
    }

    let segments: [Segment]
    @Binding var selection: Value

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    init(_ segments: [(Value, String)], selection: Binding<Value>) {
        self.segments = segments.map { Segment(value: $0.0, label: $0.1) }
        self._selection = selection
    }

    var body: some View {
        HStack(spacing: 0) {
            ForEach(segments) { segment in
                let active = segment.value == selection
                Button {
                    selection = segment.value
                } label: {
                    Text(segment.label)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(active ? DS.Colors.inkPrimary : DS.Colors.inkSecondary)
                        .fixedSize(horizontal: true, vertical: false)
                        .padding(.horizontal, DS.Spacing.space5)
                        .frame(maxHeight: .infinity)
                        .background(
                            RoundedRectangle(cornerRadius: DS.Radius.sm - 3, style: .continuous)
                                .fill(active ? DS.Colors.surfaceRaised : Color.clear)
                                .modifier(DSShadowModifier(shadow: active ? .control : DS.Shadow(layers: [])))
                        )
                        .contentShape(RoundedRectangle(cornerRadius: DS.Radius.sm - 3, style: .continuous))
                }
                .buttonStyle(.plain)
                .focusEffectDisabled()
                .accessibilityAddTraits(active ? [.isSelected] : [])
            }
        }
        .padding(3)
        .frame(height: DS.Size.controlHeightMd)
        .background(
            RoundedRectangle(cornerRadius: DS.Radius.sm, style: .continuous)
                .fill(DS.Colors.surfaceSunken)
        )
        .animation(reduceMotion ? nil : .dsSwitch, value: selection)
    }
}

#Preview("Table / Raw") {
    struct Host: View {
        @State private var mode = "table"
        var body: some View {
            DSSegmentedControl([("table", "Bảng"), ("raw", "Raw")], selection: $mode)
        }
    }
    return Host().padding(24).background(DS.Colors.surfaceWindow)
}
