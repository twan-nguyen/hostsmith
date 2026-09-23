import SwiftUI
import AppKit

/// Request to move the caret to a line and scroll it into view. A new `id`
/// triggers the jump even when `line` repeats.
struct EditorJump: Equatable {
    let id = UUID()
    let line: Int
}

/// The monospace editor behind the hosts pane and the .env Raw view.
/// Spec: `components/CodeEditor/README.md`. `gutterWidth` gutter on `surfaceSunken`
/// in `inkTertiary` with a hairline on its trailing edge, body in the editor
/// font at the design's 20/13 line height, four token colours, ⌘/ toggles
/// comments on the selected lines.
struct CodeEditor: NSViewRepresentable {
    @Binding var text: String
    var language: CodeSyntaxHighlighter.Language
    var fontSize: CGFloat = 13
    /// Selects and scrolls to the first match when it changes.
    var searchQuery: String = ""
    var jump: EditorJump? = nil

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        let textView = CommentToggleTextView()
        let font = NSFont.monospacedSystemFont(ofSize: fontSize, weight: .regular)
        textView.isEditable = true
        textView.isSelectable = true
        textView.allowsUndo = true
        textView.font = font
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = false
        textView.isRichText = false
        textView.usesFindBar = true
        textView.drawsBackground = true
        textView.backgroundColor = NSColor(DS.Colors.surfaceContent)
        textView.insertionPointColor = NSColor(DS.Colors.accent)
        textView.textContainerInset = NSSize(width: DS.Spacing.space6, height: DS.Spacing.space5)
        textView.string = text
        textView.delegate = context.coordinator
        textView.autoresizingMask = [.width]
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.textContainer?.widthTracksTextView = true
        textView.selectedTextAttributes = [.backgroundColor: NSColor(DS.Colors.accentTint)]
        textView.defaultParagraphStyle = CodeSyntaxHighlighter.paragraphStyle(for: font)
        textView.typingAttributes = [.font: font, .paragraphStyle: CodeSyntaxHighlighter.paragraphStyle(for: font)]

        context.coordinator.font = font
        context.coordinator.language = language
        if let storage = textView.textStorage {
            CodeSyntaxHighlighter.apply(to: storage, language: language, font: font)
        }

        scrollView.hasVerticalScroller = true
        // The hidden-title-bar window reports a safe area; without this the scroll view
        // adds a top inset and the first lines start hidden above the visible rect.
        scrollView.automaticallyAdjustsContentInsets = false
        scrollView.contentInsets = NSEdgeInsetsZero
        scrollView.drawsBackground = true
        scrollView.backgroundColor = NSColor(DS.Colors.surfaceContent)
        scrollView.documentView = textView

        let ruler = LineNumberRulerView(textView: textView)
        scrollView.verticalRulerView = ruler
        scrollView.hasVerticalRuler = true
        scrollView.rulersVisible = true
        context.coordinator.ruler = ruler
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else { return }
        let coordinator = context.coordinator
        let font = NSFont.monospacedSystemFont(ofSize: fontSize, weight: .regular)
        var needsHighlight = false

        if coordinator.font.pointSize != fontSize {
            coordinator.font = font
            textView.font = font
            textView.defaultParagraphStyle = CodeSyntaxHighlighter.paragraphStyle(for: font)
            textView.typingAttributes = [.font: font, .paragraphStyle: CodeSyntaxHighlighter.paragraphStyle(for: font)]
            coordinator.ruler?.needsDisplay = true
            needsHighlight = true
        }
        if coordinator.language != language {
            coordinator.language = language
            needsHighlight = true
        }
        if textView.string != text {
            let selected = textView.selectedRanges
            textView.string = text
            textView.selectedRanges = selected.filter { ($0 as? NSRange).map { NSMaxRange($0) <= (text as NSString).length } ?? false }
            needsHighlight = true
            coordinator.ruler?.needsDisplay = true
        }
        if needsHighlight, let storage = textView.textStorage {
            CodeSyntaxHighlighter.apply(to: storage, language: language, font: font)
        }
        if coordinator.lastQuery != searchQuery {
            coordinator.lastQuery = searchQuery
            coordinator.select(query: searchQuery, in: textView)
        }
        if let jump, coordinator.lastJumpId != jump.id {
            coordinator.lastJumpId = jump.id
            coordinator.scroll(toLine: jump.line, in: textView)
        }
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: CodeEditor
        var font: NSFont = .monospacedSystemFont(ofSize: 13, weight: .regular)
        var language: CodeSyntaxHighlighter.Language = .hosts
        var lastQuery = ""
        var lastJumpId: UUID?
        weak var ruler: LineNumberRulerView?
        private var highlightWorkItem: DispatchWorkItem?

        init(_ parent: CodeEditor) { self.parent = parent }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            parent.text = textView.string
            ruler?.needsDisplay = true

            highlightWorkItem?.cancel()
            let work = DispatchWorkItem { [weak textView, font, language] in
                guard let storage = textView?.textStorage else { return }
                CodeSyntaxHighlighter.apply(to: storage, language: language, font: font)
            }
            highlightWorkItem = work
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.08, execute: work)
        }

        func select(query: String, in textView: NSTextView) {
            guard !query.isEmpty else { return }
            let ns = textView.string as NSString
            let start = NSMaxRange(textView.selectedRange())
            var range = ns.range(of: query, options: .caseInsensitive, range: NSRange(location: start, length: ns.length - start))
            if range.location == NSNotFound {
                range = ns.range(of: query, options: .caseInsensitive)
            }
            guard range.location != NSNotFound else { return }
            textView.setSelectedRange(range)
            textView.scrollRangeToVisible(range)
        }

        func scroll(toLine line: Int, in textView: NSTextView) {
            let ns = textView.string as NSString
            var current = 0
            var location = 0
            while current < line, location < ns.length {
                location = NSMaxRange(ns.lineRange(for: NSRange(location: location, length: 0)))
                current += 1
            }
            let range = ns.lineRange(for: NSRange(location: min(location, ns.length), length: 0))
            textView.setSelectedRange(NSRange(location: range.location, length: 0))
            textView.scrollRangeToVisible(range)
        }
    }
}

/// Line-number gutter: `gutterWidth`, `surfaceSunken`, `inkTertiary`, a hairline
/// on its trailing edge, not selectable.
final class LineNumberRulerView: NSRulerView {
    private weak var textView: NSTextView?
    private var observers: [NSObjectProtocol] = []

    init(textView: NSTextView) {
        self.textView = textView
        super.init(scrollView: textView.enclosingScrollView, orientation: .verticalRuler)
        clientView = textView
        ruleThickness = DS.Size.gutterWidth
        let center = NotificationCenter.default
        observers.append(center.addObserver(forName: NSText.didChangeNotification, object: textView, queue: .main) { [weak self] _ in
            self?.needsDisplay = true
        })
        if let clip = textView.enclosingScrollView?.contentView {
            clip.postsBoundsChangedNotifications = true
            observers.append(center.addObserver(forName: NSView.boundsDidChangeNotification, object: clip, queue: .main) { [weak self] _ in
                self?.needsDisplay = true
            })
        }
    }

    required init(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    deinit { observers.forEach(NotificationCenter.default.removeObserver) }

    override func drawHashMarksAndLabels(in rect: NSRect) {
        guard let textView, let layoutManager = textView.layoutManager, let container = textView.textContainer else { return }

        NSColor(DS.Colors.surfaceSunken).setFill()
        bounds.fill()
        NSColor(DS.Colors.hairline).setFill()
        NSRect(x: bounds.maxX - DS.Size.hairlineWidth, y: bounds.minY, width: DS.Size.hairlineWidth, height: bounds.height).fill()

        let font = NSFont.monospacedSystemFont(ofSize: max(10, (textView.font?.pointSize ?? 13) - 1), weight: .regular)
        let attributes: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: NSColor(DS.Colors.inkTertiary)]
        let text = textView.string as NSString
        let visibleRect = textView.visibleRect
        let visibleGlyphs = layoutManager.glyphRange(forBoundingRect: visibleRect, in: container)
        let visibleChars = layoutManager.characterRange(forGlyphRange: visibleGlyphs, actualGlyphRange: nil)

        var lineNumber = 1
        var index = 0
        while index < visibleChars.location {
            index = NSMaxRange(text.lineRange(for: NSRange(location: index, length: 0)))
            lineNumber += 1
        }

        let inset = textView.textContainerInset.height
        while index < NSMaxRange(visibleChars) || (index == text.length && text.length == 0) {
            let lineRange = text.lineRange(for: NSRange(location: index, length: 0))
            let glyphIndex = layoutManager.glyphIndexForCharacter(at: lineRange.location)
            let lineRect = layoutManager.lineFragmentRect(forGlyphAt: glyphIndex, effectiveRange: nil)
            let y = lineRect.minY + inset - visibleRect.minY
            let label = "\(lineNumber)" as NSString
            let size = label.size(withAttributes: attributes)
            label.draw(at: NSPoint(x: ruleThickness - size.width - DS.Spacing.space4, y: y + (lineRect.height - size.height) / 2), withAttributes: attributes)
            lineNumber += 1
            if lineRange.length == 0 { break }
            index = NSMaxRange(lineRange)
            if index >= text.length { break }
        }

        if text.length > 0, text.hasSuffix("\n") == false { return }
        if text.length > 0 {
            // Trailing empty line after the final newline.
            let glyphCount = layoutManager.numberOfGlyphs
            let extra = layoutManager.extraLineFragmentRect
            if extra.height > 0, glyphCount > 0 {
                let y = extra.minY + inset - visibleRect.minY
                let label = "\(lineNumber)" as NSString
                let size = label.size(withAttributes: attributes)
                label.draw(at: NSPoint(x: ruleThickness - size.width - DS.Spacing.space4, y: y + (extra.height - size.height) / 2), withAttributes: attributes)
            }
        }
    }
}

/// `⌘/` comments or uncomments the selected lines.
final class CommentToggleTextView: NSTextView {
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if event.modifierFlags.contains(.command) && event.charactersIgnoringModifiers == "/" {
            toggleCommentOnSelectedLines()
            return true
        }
        return super.performKeyEquivalent(with: event)
    }

    private func toggleCommentOnSelectedLines() {
        let nsString = string as NSString
        let lineRange = nsString.lineRange(for: selectedRange())
        let lines = nsString.substring(with: lineRange).components(separatedBy: "\n")
        let trimmedLines = lines.last == "" ? Array(lines.dropLast()) : lines
        let allCommented = trimmedLines.allSatisfy { $0.isEmpty || $0.hasPrefix("#") }

        let newLines: [String] = trimmedLines.map { line in
            if line.isEmpty { return line }
            if allCommented {
                if line.hasPrefix("# ") { return String(line.dropFirst(2)) }
                if line.hasPrefix("#") { return String(line.dropFirst(1)) }
                return line
            }
            return "# " + line
        }

        var replacement = newLines.joined(separator: "\n")
        if lines.last == "" { replacement += "\n" }

        if shouldChangeText(in: lineRange, replacementString: replacement) {
            replaceCharacters(in: lineRange, with: replacement)
            didChangeText()
            setSelectedRange(NSRange(location: lineRange.location, length: (replacement as NSString).length))
        }
    }
}
