import Testing
import AppKit
@testable import Hosven

@Suite("CodeSyntaxHighlighter")
struct CodeSyntaxHighlighterTests {
    private let font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)

    private func storage(_ text: String) -> NSTextStorage {
        NSTextStorage(string: text)
    }

    private func colorAt(_ storage: NSTextStorage, _ offset: Int) -> NSColor? {
        guard offset >= 0, offset < storage.length else { return nil }
        return storage.attribute(.foregroundColor, at: offset, effectiveRange: nil) as? NSColor
    }

    @Test("Empty input is a no-op (no crash)")
    func emptyInputDoesNotCrash() {
        let s = storage("")
        CodeSyntaxHighlighter.apply(to: s, language: .hosts, font: font)
        #expect(s.length == 0)
    }

    @Test("IP and hostname get distinct colours")
    func ipVsHostnameColors() {
        let line = "127.0.0.1\tlocalhost\n"
        let s = storage(line)
        CodeSyntaxHighlighter.apply(to: s, language: .hosts, font: font)

        let ipColor = colorAt(s, 2)        // inside "127.0.0.1"
        let hostColor = colorAt(s, 12)     // inside "localhost"
        #expect(ipColor != nil)
        #expect(hostColor != nil)
        #expect(ipColor != hostColor, "IP and hostname use code-ip and code-key")
    }

    @Test("Env key, value and comment get distinct colours")
    func envTokens() {
        let line = "API_URL=https://acme.test # prod\n"
        let s = storage(line)
        CodeSyntaxHighlighter.apply(to: s, language: .env, font: font)
        let keyColor = colorAt(s, 1)
        let valueColor = colorAt(s, 10)
        let commentColor = colorAt(s, line.count - 3)
        #expect(keyColor != valueColor)
        #expect(valueColor != commentColor)
    }

    @Test("Disabled env entry is dimmed rather than treated as a comment")
    func envDisabledEntry() {
        let disabled = "# FEATURE=off\n"
        let s = storage(disabled)
        CodeSyntaxHighlighter.apply(to: s, language: .env, font: font)
        let keyColor = colorAt(s, 3)
        #expect(keyColor != nil)
        #expect((keyColor?.alphaComponent ?? 1) < 1, "disabled entry tokens carry Opacity.disabled")
    }

    @Test("Tag marker line is colored uniformly")
    func tagMarkerLineIsColored() {
        let line = "## [tag:Release]\n"
        let s = storage(line)
        CodeSyntaxHighlighter.apply(to: s, language: .hosts, font: font)
        let c0 = colorAt(s, 0)
        let cMid = colorAt(s, 8)
        #expect(c0 != nil)
        #expect(c0 == cMid, "entire tag marker should share one color")
    }

    @Test("Pure comment is colored differently than active entry")
    func commentVsEntry() {
        let comment = "# This is a comment\n"
        let entry = "127.0.0.1\tlocalhost\n"
        let s = storage(comment + entry)
        CodeSyntaxHighlighter.apply(to: s, language: .hosts, font: font)
        let commentColor = colorAt(s, 2)
        let entryHostnameColor = colorAt(s, comment.count + 10) // hostname region
        #expect(commentColor != entryHostnameColor)
    }

    @Test("Disabled entry treated as dimmed, not as pure comment")
    func disabledEntryIsDimmed() {
        let disabled = "# 10.0.0.5\tapi.staging.com\n"
        let comment = "# random note\n"
        let s = storage(disabled + comment)
        CodeSyntaxHighlighter.apply(to: s, language: .hosts, font: font)
        let disabledMid = colorAt(s, 5)
        #expect(disabledMid != nil)
        #expect((disabledMid?.alphaComponent ?? 1) < 1, "disabled entry tokens carry Opacity.disabled")
    }
}
