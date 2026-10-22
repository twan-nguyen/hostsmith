//! Reads and writes the hosts file format.
//!
//! The text is normalised on the way through: blank lines go, comment lines
//! move to the top, and every entry is written `ip<TAB>hostname[ alias…][ # comment]`
//! inside its `## [tag:Name]` block. HostSmith compares against this normalised
//! form, so formatting differences on disk never count as changes.

use std::sync::LazyLock;

use regex::Regex;

use super::model::HostEntry;

static TAG_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^##\s*\[tag:(.+)\]\s*$").unwrap());
static TAG_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^##\s*\[/tag:(.+)\]\s*$").unwrap());

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub entries: Vec<HostEntry>,
    /// Lines kept verbatim at the top of the file: comments, and anything that
    /// is not an entry, such as a bare IP.
    pub comment_lines: Vec<String>,
    /// Block names in the order their markers first appear.
    pub tags: Vec<String>,
}

pub fn parse(content: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let mut current_tag: Option<String> = None;

    for line in content.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(name) = marker_name(&TAG_START, trimmed) {
            // A second spelling of a block joins the first, or its entries
            // would belong to no block that gets written.
            let known = parsed
                .tags
                .iter()
                .find(|t| t.to_lowercase() == name.to_lowercase());
            let name = match known {
                Some(existing) => existing.clone(),
                None => {
                    parsed.tags.push(name.clone());
                    name
                }
            };
            current_tag = Some(name);
            continue;
        }
        if marker_name(&TAG_END, trimmed).is_some() {
            current_tag = None;
            continue;
        }

        let (body, enabled) = match trimmed.strip_prefix('#') {
            Some(rest) => (rest.trim(), false),
            None => (trimmed, true),
        };
        match parse_entry(body, current_tag.clone()) {
            // A commented line is an entry only when it starts with an address.
            Some(mut entry) if enabled || is_valid_ip(&entry.ip) => {
                entry.enabled = enabled;
                parsed.entries.push(entry);
            }
            _ => parsed.comment_lines.push(line.to_string()),
        }
    }
    parsed
}

fn marker_name(pattern: &Regex, line: &str) -> Option<String> {
    pattern.captures(line).map(|c| c[1].trim().to_string())
}

/// `ip hostname [alias…] [# comment]`, or `None` when there are fewer than two fields.
pub fn parse_entry(text: &str, tag: Option<String>) -> Option<HostEntry> {
    let (body, comment) = match text.split_once('#') {
        Some((body, comment)) => (body, comment.trim()),
        None => (text, ""),
    };
    let mut fields = body.split_whitespace();
    let ip = fields.next()?;
    let hostname = fields.next()?;
    let mut entry = HostEntry::new(ip, hostname, comment, tag);
    entry.aliases = fields.map(str::to_string).collect();
    Some(entry)
}

#[cfg(test)]
pub fn serialize(parsed: &Parsed) -> String {
    serialize_parts(&parsed.entries, &parsed.comment_lines, &parsed.tags)
}

pub fn serialize_parts(entries: &[HostEntry], comment_lines: &[String], tags: &[String]) -> String {
    let mut lines: Vec<String> = comment_lines.to_vec();
    if !comment_lines.is_empty() {
        lines.push(String::new());
    }

    lines.extend(entries.iter().filter(|e| e.tag.is_none()).map(format_entry));

    // Blocks in marker order, then any block only entries name, so an entry
    // added to a new profile is never left out of the file.
    let mut order: Vec<&str> = tags.iter().map(String::as_str).collect();
    for tag in entries.iter().filter_map(|e| e.tag.as_deref()) {
        if !order.contains(&tag) {
            order.push(tag);
        }
    }
    for tag in order {
        let group: Vec<&HostEntry> = entries
            .iter()
            .filter(|e| e.tag.as_deref() == Some(tag))
            .collect();
        if group.is_empty() {
            continue;
        }
        if lines.last().is_some_and(|last| !last.is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("## [tag:{tag}]"));
        lines.extend(group.into_iter().map(format_entry));
        lines.push(format!("## [/tag:{tag}]"));
    }

    let mut text = lines.join("\n");
    text.push('\n');
    text
}

pub fn format_entry(entry: &HostEntry) -> String {
    let mut line = String::new();
    if !entry.enabled {
        line.push_str("# ");
    }
    line.push_str(&entry.ip);
    line.push('\t');
    line.push_str(&entry.hostname);
    for alias in &entry.aliases {
        line.push(' ');
        line.push_str(alias);
    }
    if !entry.comment.is_empty() {
        line.push_str(" # ");
        line.push_str(&entry.comment);
    }
    line
}

/// Four dotted numbers 0-255, or an IPv6 address with an optional `%zone`.
/// Hosven 1.x took anything with a colon, which turned comments such as
/// `# Note: fix this` into disabled entries.
pub fn is_valid_ip(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        return true;
    }
    let address = text.split_once('%').map_or(text, |(address, _)| address);
    address.parse::<std::net::Ipv6Addr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "# Sample /etc/hosts fixture for parser round-trip tests.
##
# Hostfile
127.0.0.1\tlocalhost
::1\tlocalhost

## [tag:Release]
127.0.0.1\tapi.dev.example.com # local API
127.0.0.1\tweb.dev.example.com
## [/tag:Release]

## [tag:Production]
10.0.0.5\tapi.prod.example.com
# 10.0.0.6\tapi.staging.example.com # disabled staging
## [/tag:Production]
";

    fn hostnames(parsed: &Parsed, tag: Option<&str>) -> Vec<String> {
        parsed
            .entries
            .iter()
            .filter(|e| e.tag.as_deref() == tag)
            .map(|e| e.hostname.clone())
            .collect()
    }

    #[test]
    fn groups_entries_by_tag_marker() {
        let parsed = parse(FIXTURE);
        assert_eq!(parsed.tags, ["Release", "Production"]);
        assert_eq!(
            hostnames(&parsed, Some("Release")),
            ["api.dev.example.com", "web.dev.example.com"]
        );
    }

    #[test]
    fn a_commented_address_line_is_a_disabled_entry() {
        let parsed = parse(FIXTURE);
        let staging = parsed
            .entries
            .iter()
            .find(|e| e.hostname == "api.staging.example.com")
            .unwrap();
        assert!(!staging.enabled);
        assert_eq!(staging.ip, "10.0.0.6");
        assert_eq!(staging.tag.as_deref(), Some("Production"));
        assert_eq!(staging.comment, "disabled staging");
    }

    #[test]
    fn localhost_lines_are_enabled_and_untagged() {
        let parsed = parse(FIXTURE);
        let localhost: Vec<_> = parsed
            .entries
            .iter()
            .filter(|e| e.hostname == "localhost")
            .collect();
        assert_eq!(localhost.len(), 2);
        assert!(localhost.iter().all(|e| e.enabled && e.tag.is_none()));
    }

    #[test]
    fn empty_content_has_nothing() {
        let parsed = parse("");
        assert!(
            parsed.entries.is_empty() && parsed.tags.is_empty() && parsed.comment_lines.is_empty()
        );
        assert_eq!(serialize(&parsed), "\n");
    }

    #[test]
    fn a_commented_line_needs_an_address_to_be_an_entry() {
        let parsed = parse("# not a valid host line\n# 127.0.0.1 valid.example.com");
        assert_eq!(parsed.entries.len(), 1);
        assert!(!parsed.entries[0].enabled);
        assert_eq!(parsed.entries[0].hostname, "valid.example.com");
        assert_eq!(parsed.comment_lines, ["# not a valid host line"]);
    }

    #[test]
    fn serializes_the_fixture_in_normal_form() {
        let expected = "# Sample /etc/hosts fixture for parser round-trip tests.
##
# Hostfile

127.0.0.1\tlocalhost
::1\tlocalhost

## [tag:Release]
127.0.0.1\tapi.dev.example.com # local API
127.0.0.1\tweb.dev.example.com
## [/tag:Release]

## [tag:Production]
10.0.0.5\tapi.prod.example.com
# 10.0.0.6\tapi.staging.example.com # disabled staging
## [/tag:Production]
";
        assert_eq!(serialize(&parse(FIXTURE)), expected);
    }

    #[test]
    fn round_trip_is_stable() {
        let once = serialize(&parse(FIXTURE));
        let twice = serialize(&parse(&once));
        let thrice = serialize(&parse(&twice));
        assert_eq!(once, twice);
        assert_eq!(twice, thrice);
    }

    #[test]
    fn a_disabled_entry_comment_does_not_grow() {
        let text = "# 10.0.0.6\th # note\n";
        let mut current = text.to_string();
        for _ in 0..3 {
            current = serialize(&parse(&current));
        }
        assert_eq!(current, text);
    }

    #[test]
    fn aliases_survive_a_write() {
        let text = "127.0.0.1\tapi.test admin.test www.test # three names\n# 10.0.0.1\told.test older.test\n";
        let parsed = parse(text);
        assert_eq!(parsed.entries[0].aliases, ["admin.test", "www.test"]);
        assert_eq!(parsed.entries[1].aliases, ["older.test"]);
        assert_eq!(serialize(&parsed), text);
    }

    #[test]
    fn a_bare_address_line_is_kept_verbatim() {
        let parsed = parse("10.0.0.9\n127.0.0.1\tlocalhost\n");
        assert_eq!(parsed.comment_lines, ["10.0.0.9"]);
        assert_eq!(serialize(&parsed), "10.0.0.9\n\n127.0.0.1\tlocalhost\n");
    }

    #[test]
    fn crlf_line_endings_leave_no_carriage_returns() {
        let parsed = parse("# top\r\n127.0.0.1\tapi.test # note\r\n");
        assert_eq!(parsed.comment_lines, ["# top"]);
        assert_eq!(parsed.entries[0].hostname, "api.test");
        assert_eq!(parsed.entries[0].comment, "note");
    }

    #[test]
    fn a_second_spelling_of_a_block_joins_the_first() {
        let parsed = parse(
            "## [tag:Dev]\n127.0.0.1\ta.test\n## [/tag:Dev]\n## [tag:dev]\n127.0.0.1\tb.test\n## [/tag:dev]\n",
        );
        assert_eq!(parsed.tags, ["Dev"]);
        assert_eq!(hostnames(&parsed, Some("Dev")), ["a.test", "b.test"]);
        assert!(serialize(&parsed).contains("127.0.0.1\tb.test"));
    }

    #[test]
    fn entries_of_a_block_without_a_marker_are_still_written() {
        let mut parsed = parse("127.0.0.1\tlocalhost\n");
        parsed.entries.push(HostEntry::new(
            "127.0.0.1",
            "new.test",
            "",
            Some("QA".into()),
        ));
        assert_eq!(
            serialize(&parsed),
            "127.0.0.1\tlocalhost\n\n## [tag:QA]\n127.0.0.1\tnew.test\n## [/tag:QA]\n"
        );
    }

    /// A hosts file with aliases, comments after an address, a bare address,
    /// a block spelled twice and a scoped IPv6 address.
    const SAMPLE: &str = include_str!("../../tests/fixtures/hosts-sample");

    #[test]
    fn a_real_file_keeps_every_line_and_settles_after_one_pass() {
        let ours = serialize(&parse(SAMPLE));
        for line in [
            "10.0.0.9",
            "127.0.0.1\tweb.local.test admin.local.test",
            "127.0.0.1\textra.local.test",
            "# 10.0.0.5\tapi.local.test # staging API",
            "fe80::1%lo0\tlocalhost",
        ] {
            assert!(ours.lines().any(|l| l == line), "{line} was lost");
        }
        assert_eq!(
            serialize(&parse(&ours)),
            ours,
            "a second pass changes nothing"
        );
    }

    #[test]
    fn validates_addresses() {
        assert!(is_valid_ip("127.0.0.1"));
        assert!(is_valid_ip("::1"));
        assert!(is_valid_ip("fe80::1%lo0"));
        assert!(!is_valid_ip("256.0.0.1"));
        assert!(!is_valid_ip("not"));
        assert!(!is_valid_ip("1.2.3"));
        assert!(!is_valid_ip("Note:"));
    }

    #[test]
    fn a_comment_with_a_colon_stays_a_comment() {
        let parsed = parse("# Note: fix this later\n");
        assert!(parsed.entries.is_empty());
        assert_eq!(parsed.comment_lines, ["# Note: fix this later"]);
    }
}
