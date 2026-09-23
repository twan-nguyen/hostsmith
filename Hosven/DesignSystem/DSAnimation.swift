import SwiftUI

/// Motion tokens. Brand book: quiet and short, nothing bounces, nothing loops.
/// Reduce Motion callers cut to the end state (`reduceMotion ? nil : .ds*`).
extension Animation {
    /// A popover fades and rises 4pt. 150 ms.
    static let dsPopover = Animation.easeOut(duration: 0.15)

    /// A sheet slides in. 200 ms.
    static let dsSheet = Animation.easeInOut(duration: 0.20)

    /// A switch knob eases across. 160 ms.
    static let dsSwitch = Animation.easeInOut(duration: 0.16)

    /// A status change cross-fades. 80 ms.
    static let dsStatus = Animation.easeInOut(duration: 0.08)
}
