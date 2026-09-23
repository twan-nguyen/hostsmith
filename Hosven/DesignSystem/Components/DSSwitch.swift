import SwiftUI

/// On/off control for one reversible thing. Spec: `components/Switch/README.md`.
/// Track 36×22, knob 18 with `shadowControl`; on is `accentBright`, the off
/// state carries the `borderControl` ring. Optional `requiresAdmin` shows
/// `lock.fill` after the label.
struct DSSwitch: View {
    @Binding var isOn: Bool
    var label: String? = nil
    var sublabel: String? = nil
    var requiresAdmin: Bool = false
    var disabled: Bool = false
    var onChange: ((Bool) -> Void)? = nil

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.isFocused) private var isFocused

    private let trackWidth: CGFloat = 36
    private let trackHeight: CGFloat = 22
    private let knobSize: CGFloat = 18

    var body: some View {
        Button(action: toggle) {
            HStack(spacing: DS.Spacing.space4) {
                if label != nil || sublabel != nil {
                    VStack(alignment: .leading, spacing: 2) {
                        if let label {
                            HStack(spacing: DS.Spacing.space2) {
                                Text(label)
                                    .font(DS.Text.textBody)
                                    .foregroundStyle(DS.Colors.inkPrimary)
                                if requiresAdmin {
                                    Image(systemName: "lock.fill")
                                        .font(.system(size: 10, weight: .medium))
                                        .foregroundStyle(DS.Colors.inkSecondary)
                                }
                            }
                        }
                        if let sublabel {
                            Text(sublabel)
                                .font(DS.Text.monoCaption)
                                .foregroundStyle(DS.Colors.inkSecondary)
                                .lineLimit(1)
                                .truncationMode(.tail)
                        }
                    }
                    Spacer(minLength: DS.Spacing.space4)
                }
                track
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .focusEffectDisabled()
        .disabled(disabled)
        .opacity(disabled ? DS.Opacity.disabled : 1)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityText)
        .accessibilityAddTraits(.isToggle)
    }

    private var track: some View {
        ZStack(alignment: isOn ? .trailing : .leading) {
            Capsule(style: .continuous)
                .fill(isOn ? DS.Colors.accentBright : DS.Colors.controlTrack)
                .overlay(
                    Capsule(style: .continuous)
                        .strokeBorder(DS.Colors.borderControl, lineWidth: isOn ? 0 : 1)
                )
                .frame(width: trackWidth, height: trackHeight)

            Circle()
                .fill(Color.white)
                .frame(width: knobSize, height: knobSize)
                .dsShadow(.control)
                .padding(.horizontal, 2)
        }
        .dsFocusRing(isFocused, radius: DS.Radius.pill)
        .animation(reduceMotion ? nil : .dsSwitch, value: isOn)
    }

    private var accessibilityText: String {
        let thing = label ?? "Công tắc"
        return "\(thing), \(isOn ? "bật" : "tắt")"
    }

    private func toggle() {
        isOn.toggle()
        onChange?(isOn)
    }
}

/// Legacy name kept for older call sites.
typealias DSToggle = DSSwitch

#Preview("States") {
    VStack(alignment: .leading, spacing: 16) {
        DSSwitch(isOn: .constant(true), label: "Acme services", sublabel: "14 entries · 127.0.0.1 → *.acme.test")
        DSSwitch(isOn: .constant(false), label: "Route CDN to localhost", sublabel: "127.0.0.1 → cdn.acme.test", requiresAdmin: true)
        DSSwitch(isOn: .constant(true), disabled: true)
        DSSwitch(isOn: .constant(false))
    }
    .padding(24)
    .frame(width: 360)
    .background(DS.Colors.surfaceContent)
}
