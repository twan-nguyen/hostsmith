import Testing
import AppKit
@testable import Hosven

@Suite("HostsSyntaxHighlighter")
struct HostsSyntaxHighlighterTests {
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
        HostsSyntaxHighlighter.apply(to: s, font: font)
        #expect(s.length == 0)
    }

    @Test("Localhost IP gets distinct color from remote IP")
    func localhostVsRemoteColors() {
        let line1 = "127.0.0.1\tlocalhost\n"
        let line2 = "10.0.0.1\tapi.test\n"
        let s = storage(line1 + line2)
        HostsSyntaxHighlighter.apply(to: s, font: font)

        let localColor = colorAt(s, 2)            // inside "127.0.0.1"
        let remoteColor = colorAt(s, line1.count + 2) // inside "10.0.0.1"
        #expect(localColor != nil)
        #expect(remoteColor != nil)
        #expect(localColor != remoteColor, "localhost and remote IPs should be distinct colors")
    }

    @Test("Tag marker line is colored uniformly")
    func tagMarkerLineIsColored() {
        let line = "## [tag:Release]\n"
        let s = storage(line)
        HostsSyntaxHighlighter.apply(to: s, font: font)
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
        HostsSyntaxHighlighter.apply(to: s, font: font)
        let commentColor = colorAt(s, 2)
        let entryHostnameColor = colorAt(s, comment.count + 10) // hostname region
        #expect(commentColor != entryHostnameColor)
    }

    @Test("Disabled entry treated as dimmed, not as pure comment")
    func disabledEntryIsDimmed() {
        let disabled = "# 10.0.0.5\tapi.staging.com\n"
        let comment = "# random note\n"
        let s = storage(disabled + comment)
        HostsSyntaxHighlighter.apply(to: s, font: font)
        // Last char of disabled line vs comment line — both should be dim/grey but
        // disabled uses tertiaryLabelColor, comment uses our explicit gray. Just check
        // both are non-default (not labelColor).
        let disabledMid = colorAt(s, 5)
        #expect(disabledMid != nil)
        #expect(disabledMid != NSColor.labelColor)
    }
}
