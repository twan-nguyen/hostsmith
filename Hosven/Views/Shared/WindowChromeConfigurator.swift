import AppKit
import SwiftUI

/// Reaches up to the hosting NSWindow once it exists and applies our custom chrome:
/// transparent title bar, full-size content view, traffic lights centred in the
/// `titleStripHeight` strip at the top of the sidebar. This must run from inside the SwiftUI view tree (not
/// AppDelegate) because NSApp.windows iteration at didFinishLaunching can miss
/// the WindowGroup window — viewDidMoveToWindow is the reliable hook.
struct WindowChromeConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView { ProbeView() }
    func updateNSView(_ nsView: NSView, context: Context) {}

    /// Apply our custom chrome to `window`. Exposed for tests.
    /// Only sets fields whose value differs from desired — re-applying otherwise
    /// drops pending click events.
    /// The strip at the top of the sidebar that holds the traffic lights.
    static let titleStripHeight: CGFloat = 40
    /// Traffic-light centre from the window's top-left: vertically centred in the strip,
    /// the close button starting at the sidebar's `space6` inset as the screens draw it.
    static let trafficLightCenter = NSPoint(x: DS.Spacing.space6 + 6, y: titleStripHeight / 2)
    private static let trafficLightSpacing: CGFloat = 20

    static func configure(_ window: NSWindow) {
        let desiredMask: NSWindow.StyleMask = [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
        if window.styleMask != desiredMask {
            window.styleMask = desiredMask
        }
        if !window.titlebarAppearsTransparent { window.titlebarAppearsTransparent = true }
        if window.titleVisibility != .hidden { window.titleVisibility = .hidden }
        // Drag only from the titlebar (where traffic lights live) — not the body.
        // With fullSizeContentView the titlebar is still hit-tested for window
        // dragging even though our custom TitleBarView paints over it.
        if window.isMovableByWindowBackground { window.isMovableByWindowBackground = false }
        if window.toolbar != nil { window.toolbar = nil }
        ensureTitlebarVisible(window)
        layoutTrafficLights(window)
    }

    /// Moves the three standard buttons to `trafficLightCenter`. AppKit resets their
    /// frames on layout, so callers re-run this from resize and frame observers.
    static func layoutTrafficLights(_ window: NSWindow) {
        let types: [NSWindow.ButtonType] = [.closeButton, .miniaturizeButton, .zoomButton]
        for (index, type) in types.enumerated() {
            guard let button = window.standardWindowButton(type), let superview = button.superview else { continue }
            let size = button.frame.size
            let x = trafficLightCenter.x + CGFloat(index) * trafficLightSpacing - size.width / 2
            let yFromTop = trafficLightCenter.y - size.height / 2
            let y = superview.isFlipped ? yFromTop : superview.bounds.height - yFromTop - size.height
            let origin = NSPoint(x: x, y: y)
            if abs(button.frame.origin.x - origin.x) > 0.5 || abs(button.frame.origin.y - origin.y) > 0.5 {
                button.setFrameOrigin(origin)
            }
        }
    }

    /// Lightweight re-assertion of titlebar visibility (no styleMask reset).
    /// Used by KVO callback when NavigationSplitView hides the container.
    static func ensureTitlebarVisible(_ window: NSWindow) {
        let close = window.standardWindowButton(.closeButton)
        if let titlebarContainer = close?.superview?.superview {
            if titlebarContainer.isHidden { titlebarContainer.isHidden = false }
            if titlebarContainer.alphaValue != 1.0 { titlebarContainer.alphaValue = 1.0 }
        }
        if close?.isHidden == true { close?.isHidden = false }
        if window.standardWindowButton(.miniaturizeButton)?.isHidden == true {
            window.standardWindowButton(.miniaturizeButton)?.isHidden = false
        }
        if window.standardWindowButton(.zoomButton)?.isHidden == true {
            window.standardWindowButton(.zoomButton)?.isHidden = false
        }
    }

    private final class ProbeView: NSView {
        private var keyObserver: NSObjectProtocol?
        private var hiddenKVO: NSKeyValueObservation?
        private var alphaKVO: NSKeyValueObservation?
        private var frameKVO: NSKeyValueObservation?
        private var resizeObserver: NSObjectProtocol?

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            detachObservers()
            guard let window = self.window, !(window is NSPanel) else { return }
            WindowChromeConfigurator.configure(window)

            // Re-apply when window becomes key.
            keyObserver = NotificationCenter.default.addObserver(
                forName: NSWindow.didBecomeKeyNotification,
                object: window,
                queue: .main
            ) { [weak window] _ in
                guard let window else { return }
                WindowChromeConfigurator.configure(window)
            }

            resizeObserver = NotificationCenter.default.addObserver(
                forName: NSWindow.didResizeNotification,
                object: window,
                queue: .main
            ) { [weak window] _ in
                guard let window else { return }
                WindowChromeConfigurator.layoutTrafficLights(window)
            }
            if let close = window.standardWindowButton(.closeButton) {
                frameKVO = close.observe(\.frame, options: [.new]) { [weak window] _, _ in
                    guard let window else { return }
                    WindowChromeConfigurator.layoutTrafficLights(window)
                }
            }
            DispatchQueue.main.async { [weak window] in
                guard let window else { return }
                WindowChromeConfigurator.layoutTrafficLights(window)
            }

            // KVO on NSTitlebarContainerView's isHidden + alphaValue: NavigationSplitView
            // (used by EnvView) hides the container when it installs its toolbar.
            // Catch every hide attempt and immediately revert.
            if let container = window.standardWindowButton(.closeButton)?.superview?.superview {
                hiddenKVO = container.observe(\.isHidden, options: [.new]) { view, change in
                    if change.newValue == true {
                        view.isHidden = false
                    }
                }
                alphaKVO = container.observe(\.alphaValue, options: [.new]) { view, change in
                    if let v = change.newValue, v < 1.0 {
                        view.alphaValue = 1.0
                    }
                }
            }
        }

        private func detachObservers() {
            if let keyObserver {
                NotificationCenter.default.removeObserver(keyObserver)
                self.keyObserver = nil
            }
            if let resizeObserver {
                NotificationCenter.default.removeObserver(resizeObserver)
                self.resizeObserver = nil
            }
            hiddenKVO?.invalidate(); hiddenKVO = nil
            alphaKVO?.invalidate(); alphaKVO = nil
            frameKVO?.invalidate(); frameKVO = nil
        }

        deinit { detachObservers() }
    }
}
