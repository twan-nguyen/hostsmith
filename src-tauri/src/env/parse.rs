//! Reads and writes dotenv files.
//!
//! Blank lines and comments are kept verbatim, so a write changes only the
//! variables the user touched. A commented-out assignment (`# KEY=value`) is a
//! variable that is switched off, which is how one file keeps several values
//! for a key and switches between them.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use uuid::Uuid;

static KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap());
static KEY_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$").unwrap());
static EXPORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^export[ \t]+(.*)$").unwrap());

/// One line of a dotenv file: a variable, on or commented out, or a line kept
/// as it is (blank, comment, or anything that is not an assignment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvEntry {
    pub id: Uuid,
    pub key: String,
    pub value: String,
    pub comment: String,
    pub enabled: bool,
    /// Written back with the `export ` prefix the line had.
    pub exported: bool,
    pub is_blank_or_comment: bool,
    /// The line as it was, for blank and comment lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_line: Option<String>,
}

impl EnvEntry {
    pub fn variable(key: &str, value: &str, comment: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            key: key.to_string(),
            value: value.to_string(),
            comment: comment.to_string(),
            enabled: true,
            exported: false,
            is_blank_or_comment: false,
            raw_line: None,
        }
    }

    fn raw(line: &str) -> Self {
        Self {
            is_blank_or_comment: true,
            raw_line: Some(line.to_string()),
            ..Self::variable("", "", "")
        }
    }

    /// Whether both write the same line, whatever their ids.
    pub fn same_line(&self, other: &Self) -> bool {
        self.key == other.key
            && self.value == other.value
            && self.comment == other.comment
            && self.enabled == other.enabled
            && self.exported == other.exported
            && self.is_blank_or_comment == other.is_blank_or_comment
            && self.raw_line == other.raw_line
    }

    /// A copy with a new id, for duplicating a row.
    pub fn copy(&self) -> Self {
        Self {
            id: Uuid::new_v4(),
            ..self.clone()
        }
    }
}

/// Whether `key` is a name the parser reads back as a variable.
pub fn is_valid_key(key: &str) -> bool {
    KEY.is_match(key)
}

pub fn parse(content: &str) -> Vec<EnvEntry> {
    let mut lines: Vec<&str> = content.split('\n').collect();
    // The newline that ends the last line is not a blank line of its own.
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
        .into_iter()
        .map(|line| parse_line(line.strip_suffix('\r').unwrap_or(line)))
        .collect()
}

fn parse_line(line: &str) -> EnvEntry {
    let trimmed = trim_blank(line);
    if trimmed.is_empty() {
        return EnvEntry::raw(line);
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        return match variable(trim_blank(rest)) {
            Some(entry) => EnvEntry {
                enabled: false,
                ..entry
            },
            None => EnvEntry::raw(line),
        };
    }
    variable(trimmed).unwrap_or_else(|| EnvEntry::raw(line))
}

fn variable(text: &str) -> Option<EnvEntry> {
    if let Some(rest) = EXPORT.captures(text).and_then(|c| c.get(1))
        && let Some(entry) = assignment(rest.as_str())
    {
        return Some(EnvEntry {
            exported: true,
            ..entry
        });
    }
    assignment(text)
}

fn assignment(text: &str) -> Option<EnvEntry> {
    let captures = KEY_VALUE.captures(text)?;
    let (value, comment) = split_value(&captures[2]);
    Some(EnvEntry::variable(&captures[1], &value, &comment))
}

fn split_value(raw: &str) -> (String, String) {
    let value = raw.trim_start_matches([' ', '\t']);
    for quote in ['"', '\''] {
        if value.starts_with(quote)
            && let Some(close) = closing_quote(value, quote)
        {
            let inner = &value[1..close];
            let comment = inline_comment(&value[close + 1..]);
            let inner = if quote == '"' {
                unescape(inner)
            } else {
                inner.to_string()
            };
            return (inner, comment);
        }
    }
    // Searched in `raw`, not `value`: in `KEY= # note` the blank before the
    // `#` was trimmed away above, and the note is still a comment.
    match [" #", "\t#"].iter().filter_map(|m| raw.find(m)).min() {
        Some(at) => (
            trim_blank(&raw[..at]).to_string(),
            trim_blank(&raw[at + 2..]).to_string(),
        ),
        None => (trim_blank(value).to_string(), String::new()),
    }
}

/// The byte index of the quote that closes the one `s` starts with. A
/// backslash skips the character after it, in either kind of quote.
fn closing_quote(s: &str, quote: char) -> Option<usize> {
    let bytes = s.as_bytes();
    let quote = quote as u8;
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == quote {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The comment after a closing quote. Any other text there is dropped.
fn inline_comment(after: &str) -> String {
    trim_blank(after)
        .strip_prefix('#')
        .map(|rest| trim_blank(rest).to_string())
        .unwrap_or_default()
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn trim_blank(s: &str) -> &str {
    s.trim_matches([' ', '\t'])
}

pub fn format(entries: &[EnvEntry]) -> String {
    let lines: Vec<String> = entries.iter().map(format_line).collect();
    lines.join("\n") + "\n"
}

fn format_line(entry: &EnvEntry) -> String {
    if entry.is_blank_or_comment {
        return entry.raw_line.clone().unwrap_or_default();
    }
    let mut line = String::new();
    if !entry.enabled {
        line.push_str("# ");
    }
    if entry.exported {
        line.push_str("export ");
    }
    line.push_str(&entry.key);
    line.push('=');
    line.push_str(&format_value(&entry.value));
    if !entry.comment.is_empty() {
        line.push_str(" # ");
        line.push_str(&entry.comment);
    }
    line
}

/// Quotes a value that would not read back as it is. A carriage return is
/// escaped too: left bare at the end of a value, the reader would take it for
/// a CRLF line ending and drop it.
fn format_value(value: &str) -> String {
    let needs_quotes = value.contains([' ', '\t', '#', '"', '\n', '\r']) || value.starts_with('\'');
    if !needs_quotes {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (line, key, value, comment, enabled, exported)
    type Expect = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        bool,
        bool,
    );

    #[test]
    fn reads_every_kind_of_variable_line() {
        let cases: &[Expect] = &[
            ("FOO=bar", "FOO", "bar", "", true, false),
            ("  FOO = bar  ", "FOO", "bar", "", true, false),
            ("_A1=x", "_A1", "x", "", true, false),
            ("EMPTY=", "EMPTY", "", "", true, false),
            ("FOO=bar # note", "FOO", "bar", "note", true, false),
            ("FOO=bar\t#note", "FOO", "bar", "note", true, false),
            ("FOO=a\t#one #two", "FOO", "a", "one #two", true, false),
            ("FOO=a#b", "FOO", "a#b", "", true, false),
            ("FOO= # note", "FOO", "", "note", true, false),
            ("URL=http://x?a=b", "URL", "http://x?a=b", "", true, false),
            (r#"FOO="a b""#, "FOO", "a b", "", true, false),
            (r#"FOO="a # b" # c"#, "FOO", "a # b", "c", true, false),
            (
                r#"FOO="x\ny\t\"z\"\\\q""#,
                "FOO",
                "x\ny\t\"z\"\\q",
                "",
                true,
                false,
            ),
            (r#"FOO="a\rb""#, "FOO", "a\rb", "", true, false),
            (r#"FOO="v" trailing"#, "FOO", "v", "", true, false),
            ("FOO='a \\n b' # c", "FOO", "a \\n b", "c", true, false),
            ("FOO='it\\'s'", "FOO", "it\\'s", "", true, false),
            (r#"FOO="unclosed"#, "FOO", "\"unclosed", "", true, false),
            ("FOO='unclosed # c", "FOO", "'unclosed", "c", true, false),
            ("# FOO=bar", "FOO", "bar", "", false, false),
            ("#FOO=bar # old", "FOO", "bar", "old", false, false),
            ("export FOO=bar", "FOO", "bar", "", true, true),
            ("export\tFOO=\"a b\"", "FOO", "a b", "", true, true),
            ("# export FOO=bar", "FOO", "bar", "", false, true),
            ("export=1", "export", "1", "", true, false),
        ];
        for (line, key, value, comment, enabled, exported) in cases {
            let entries = parse(line);
            assert_eq!(entries.len(), 1, "{line}");
            let e = &entries[0];
            assert!(!e.is_blank_or_comment, "{line} is a variable");
            assert_eq!(
                (
                    e.key.as_str(),
                    e.value.as_str(),
                    e.comment.as_str(),
                    e.enabled,
                    e.exported
                ),
                (*key, *value, *comment, *enabled, *exported),
                "{line}"
            );
        }
    }

    #[test]
    fn keeps_lines_that_are_not_variables_verbatim() {
        for line in [
            "\t",
            "   ",
            "# a comment",
            "## Section",
            "# the API key = something",
            "not a variable",
            "1KEY=x",
            "KÉY=x",
            "export FOO",
            "  # indented comment",
        ] {
            let entries = parse(line);
            assert_eq!(entries.len(), 1, "{line:?}");
            assert!(entries[0].is_blank_or_comment, "{line:?}");
            assert_eq!(entries[0].raw_line.as_deref(), Some(line));
        }
    }

    #[test]
    fn the_final_newline_is_not_a_blank_line() {
        assert!(parse("").is_empty());
        assert_eq!(parse("A=1\n").len(), 1);
        assert_eq!(parse("A=1\n\n").len(), 2);
        assert_eq!(parse("\n").len(), 1);
    }

    #[test]
    fn strips_crlf_line_endings() {
        let entries = parse("A=1\r\n# note\r\n\r\nB=\"x y\"\r\n");
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].value, "1");
        assert_eq!(entries[1].raw_line.as_deref(), Some("# note"));
        assert_eq!(entries[2].raw_line.as_deref(), Some(""));
        assert_eq!(entries[3].value, "x y");
        assert_eq!(format(&entries), "A=1\n# note\n\nB=\"x y\"\n");
    }

    #[test]
    fn formats_each_kind_of_line() {
        let mut off = EnvEntry::variable("OLD", "1", "");
        off.enabled = false;
        let mut exported = EnvEntry::variable("PATHS", "a b", "shell");
        exported.exported = true;
        let entries = vec![
            EnvEntry::raw("# header"),
            EnvEntry::variable("A", "1", ""),
            EnvEntry::variable("B", "", "bắt buộc theo .env.example"),
            off,
            exported,
            EnvEntry::raw(""),
        ];
        assert_eq!(
            format(&entries),
            "# header\nA=1\nB= # bắt buộc theo .env.example\n# OLD=1\nexport PATHS=\"a b\" # shell\n\n"
        );
        assert_eq!(format(&[]), "\n");
    }

    #[test]
    fn quotes_values_that_would_not_read_back() {
        let cases = [
            ("plain", "plain"),
            ("a b", "\"a b\""),
            ("a\tb", "\"a\\tb\""),
            ("a#b", "\"a#b\""),
            ("say \"hi\"", "\"say \\\"hi\\\"\""),
            ("line\nbreak", "\"line\\nbreak\""),
            ("back\\slash quoted", "\"back\\\\slash quoted\""),
            ("back\\slash", "back\\slash"),
            ("cr\r", "\"cr\\r\""),
            ("'single'", "\"'single'\""),
        ];
        for (value, written) in cases {
            assert_eq!(format_value(value), written, "{value:?}");
        }
    }

    #[test]
    fn every_value_round_trips() {
        let values = [
            "",
            "plain",
            "a b",
            " lead",
            "trail ",
            "a#b",
            "a #b",
            "\"",
            "'x'",
            "x'",
            "multi\nline",
            "tab\there",
            "cr\r",
            "back\\slash",
            "back\\slash and space",
            "ünïcode värde",
            "=",
        ];
        for value in values {
            for comment in ["", "note", "a # b"] {
                let mut entry = EnvEntry::variable("KEY", value, comment);
                for (enabled, exported) in [(true, false), (false, false), (true, true)] {
                    entry.enabled = enabled;
                    entry.exported = exported;
                    let back = parse(&format(std::slice::from_ref(&entry)));
                    assert_eq!(back.len(), 1, "{value:?} {comment:?}");
                    assert!(back[0].same_line(&entry), "{value:?} {comment:?}");
                }
            }
        }
    }

    #[test]
    fn a_file_round_trips_unchanged() {
        let text = "# Database\nDB_HOST=localhost\nDB_PASS=\"p@ss word\" # secret\n\n# DB_HOST=prod.example.com\nexport PATH_EXTRA=/opt/bin\nnot a variable\n";
        assert_eq!(format(&parse(text)), text);
    }

    #[test]
    fn validates_keys_like_the_parser() {
        assert!(is_valid_key("FOO"));
        assert!(is_valid_key("_foo_1"));
        assert!(!is_valid_key(""));
        assert!(!is_valid_key("1FOO"));
        assert!(!is_valid_key("FOO-BAR"));
        assert!(!is_valid_key("KÉY"));
        assert!(!is_valid_key("FOO BAR"));
        assert!(!is_valid_key("FOO\n"));
    }
}
