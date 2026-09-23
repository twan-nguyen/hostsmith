import AppKit
import SwiftUI

/// Applies the design's four code colours to a hosts or .env `NSTextStorage` in place.
/// Spec: `components/CodeEditor/README.md`. `codeIp` for addresses, `codeKey` for
/// hostnames and variable names, `codeValue` for values, `codeComment` for comments.
/// A commented-out entry keeps its tokens at `Opacity.disabled`.
enum CodeSyntaxHighlighter {
    enum Language { case hosts, env }

    private static var comment: NSColor { NSColor(DS.Colors.codeComment) }
    private static var key: NSColor { NSColor(DS.Colors.codeKey) }
    private static var value: NSColor { NSColor(DS.Colors.codeValue) }
    private static var ip: NSColor { NSColor(DS.Colors.codeIp) }
    private static var punct: NSColor { NSColor(DS.Colors.inkSecondary) }
    private static var ink: NSColor { NSColor(DS.Colors.inkPrimary) }

    private static let tagMarkerPattern = try! NSRegularExpression(pattern: #"^##\s*\[/?tag:[^\]]+\]\s*$"#)
    private static let ipv4Pattern = try! NSRegularExpression(pattern: #"^\d{1,3}(\.\d{1,3}){3}$"#)
    private static let envKeyPattern = try! NSRegularExpression(pattern: #"^(export\s+)?[A-Za-z_][A-Za-z0-9_]*\s*="#)

    static func apply(to storage: NSTextStorage, language: Language, font: NSFont) {
        let fullRange = NSRange(location: 0, length: storage.length)
        guard fullRange.length > 0 else { return }

        storage.beginEditing()
        defer { storage.endEditing() }

        storage.removeAttribute(.foregroundColor, range: fullRange)
        storage.removeAttribute(.font, range: fullRange)
        storage.addAttribute(.foregroundColor, value: ink, range: fullRange)
        storage.addAttribute(.font, value: font, range: fullRange)
        storage.addAttribute(.paragraphStyle, value: paragraphStyle(for: font), range: fullRange)

        let text = storage.string as NSString
        var lineStart = 0
        while lineStart < text.length {
            let lineRange = text.lineRange(for: NSRange(location: lineStart, length: 0))
            switch language {
            case .hosts: highlightHostsLine(in: storage, lineRange: lineRange, font: font)
            case .env: highlightEnvLine(in: storage, lineRange: lineRange, font: font)
            }
            lineStart = NSMaxRange(lineRange)
        }
    }

    // MARK: - Hosts

    private static func highlightHostsLine(in storage: NSTextStorage, lineRange: NSRange, font: NSFont) {
        let nsLine = (storage.string as NSString).substring(with: lineRange)
        let trimmed = nsLine.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return }

        if tagMarkerPattern.firstMatch(in: trimmed, range: NSRange(trimmed.startIndex..., in: trimmed)) != nil {
            storage.addAttribute(.foregroundColor, value: comment, range: lineRange)
            storage.addAttribute(.font, value: bold(font), range: lineRange)
            return
        }

        let disabled = isDisabledHostsLine(trimmed)
        if trimmed.hasPrefix("#") && !disabled {
            storage.addAttribute(.foregroundColor, value: comment, range: lineRange)
            return
        }

        let working = disabled ? String(trimmed.drop(while: { $0 == "#" || $0 == " " })) : trimmed
        let leadingOffset = lineRange.location + (nsLine as NSString).range(of: working).location
        let alpha: CGFloat = disabled ? DS.Opacity.disabled : 1

        for token in tokenizeHosts(working) {
            let range = NSRange(location: leadingOffset + token.offset, length: token.length)
            guard NSMaxRange(range) <= NSMaxRange(lineRange) else { continue }
            let color: NSColor
            switch token.kind {
            case .ip: color = ip
            case .host: color = key
            case .comment: color = comment
            }
            storage.addAttribute(.foregroundColor, value: color.withAlphaComponent(alpha), range: range)
        }
        if disabled {
            let prefixLength = leadingOffset - lineRange.location
            if prefixLength > 0 {
                storage.addAttribute(.foregroundColor, value: comment.withAlphaComponent(alpha), range: NSRange(location: lineRange.location, length: prefixLength))
            }
        }
    }

    private struct HostsToken {
        enum Kind { case ip, host, comment }
        let kind: Kind
        let offset: Int
        let length: Int
    }

    private static func tokenizeHosts(_ line: String) -> [HostsToken] {
        var tokens: [HostsToken] = []
        let ns = line as NSString
        var bodyLength = ns.length
        if let hashIdx = line.firstIndex(of: "#") {
            let pos = line.distance(from: line.startIndex, to: hashIdx)
            bodyLength = pos
            tokens.append(HostsToken(kind: .comment, offset: pos, length: ns.length - pos))
        }
        let chars = Array(ns.substring(with: NSRange(location: 0, length: bodyLength)))
        var idx = 0
        var first = true
        while idx < chars.count {
            while idx < chars.count, chars[idx].isWhitespace { idx += 1 }
            let start = idx
            while idx < chars.count, !chars[idx].isWhitespace { idx += 1 }
            if start == idx { break }
            tokens.append(HostsToken(kind: first ? .ip : .host, offset: start, length: idx - start))
            first = false
        }
        return tokens
    }

    private static func isDisabledHostsLine(_ trimmed: String) -> Bool {
        guard trimmed.hasPrefix("#") else { return false }
        let body = String(trimmed.drop(while: { $0 == "#" || $0 == " " }))
        let parts = body.split(whereSeparator: \.isWhitespace)
        guard parts.count >= 2 else { return false }
        let head = String(parts[0])
        return ipv4Pattern.firstMatch(in: head, range: NSRange(head.startIndex..., in: head)) != nil || head.contains(":")
    }

    // MARK: - Env

    private static func highlightEnvLine(in storage: NSTextStorage, lineRange: NSRange, font: NSFont) {
        let nsLine = (storage.string as NSString).substring(with: lineRange)
        let trimmed = nsLine.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return }

        var body = trimmed
        var disabled = false
        if trimmed.hasPrefix("#") {
            let stripped = String(trimmed.drop(while: { $0 == "#" || $0 == " " }))
            if envKeyPattern.firstMatch(in: stripped, range: NSRange(stripped.startIndex..., in: stripped)) != nil {
                disabled = true
                body = stripped
            } else {
                storage.addAttribute(.foregroundColor, value: comment, range: lineRange)
                return
            }
        }

        let alpha: CGFloat = disabled ? DS.Opacity.disabled : 1
        let bodyOffset = lineRange.location + (nsLine as NSString).range(of: body).location
        let nsBody = body as NSString
        if disabled, bodyOffset > lineRange.location {
            storage.addAttribute(.foregroundColor, value: comment.withAlphaComponent(alpha), range: NSRange(location: lineRange.location, length: bodyOffset - lineRange.location))
        }

        let eq = nsBody.range(of: "=")
        guard eq.location != NSNotFound else {
            storage.addAttribute(.foregroundColor, value: ink.withAlphaComponent(alpha), range: NSRange(location: bodyOffset, length: nsBody.length))
            return
        }
        let keyRange = NSRange(location: bodyOffset, length: eq.location)
        storage.addAttribute(.foregroundColor, value: key.withAlphaComponent(alpha), range: keyRange)
        storage.addAttribute(.font, value: bold(font), range: keyRange)
        storage.addAttribute(.foregroundColor, value: punct.withAlphaComponent(alpha), range: NSRange(location: bodyOffset + eq.location, length: 1))

        let valueStart = eq.location + 1
        var valueLength = nsBody.length - valueStart
        let commentMarker = nsBody.range(of: " #", options: [], range: NSRange(location: valueStart, length: valueLength))
        if commentMarker.location != NSNotFound {
            valueLength = commentMarker.location - valueStart
            storage.addAttribute(.foregroundColor, value: comment.withAlphaComponent(alpha), range: NSRange(location: bodyOffset + commentMarker.location, length: nsBody.length - commentMarker.location))
        }
        if valueLength > 0 {
            storage.addAttribute(.foregroundColor, value: value.withAlphaComponent(alpha), range: NSRange(location: bodyOffset + valueStart, length: valueLength))
        }
    }

    /// Tab stops every 4 columns so `IP<tab>host` always keeps at least one column
    /// of air, and the design's 20/13 line height scaled to the editor font.
    static func paragraphStyle(for font: NSFont) -> NSParagraphStyle {
        let column = ("0" as NSString).size(withAttributes: [.font: font]).width
        let style = NSMutableParagraphStyle()
        style.tabStops = []
        style.defaultTabInterval = column * 4
        let lineHeight = (font.pointSize * 20 / 13).rounded()
        style.minimumLineHeight = lineHeight
        style.maximumLineHeight = lineHeight
        return style
    }

    private static func bold(_ font: NSFont) -> NSFont {
        NSFont.monospacedSystemFont(ofSize: font.pointSize, weight: .semibold)
    }
}
