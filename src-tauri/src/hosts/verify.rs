//! After a write, asks the resolver what it now returns for the hostnames the
//! write changed, so the banner reports a counted fact instead of assuming
//! the write took.

use serde::Serialize;

use super::model::HostEntry;
use super::parse::parse_entry;

const LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub hostname: String,
    pub expected_ip: String,
    pub resolved_ips: Vec<String>,
    pub matches: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub results: Vec<Check>,
}

/// Enabled `(hostname, ip)` pairs on lines of `new` that `old` does not have,
/// capped so a bulk import does not turn into a hundred resolver calls.
pub fn changed_pairs(old: &str, new: &str) -> Vec<(String, String)> {
    let before: std::collections::HashSet<&str> = old.split('\n').map(str::trim).collect();
    new.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !before.contains(line))
        .filter_map(|line| parse_entry(line, None))
        .flat_map(|entry: HostEntry| {
            let ip = entry.ip.clone();
            entry
                .hostnames()
                .map(|host| (host.to_string(), ip.clone()))
                .collect::<Vec<_>>()
        })
        .take(LIMIT)
        .collect()
}

pub fn run(pairs: &[(String, String)], resolve: impl Fn(&str) -> Vec<String>) -> Verification {
    let results = pairs
        .iter()
        .map(|(hostname, ip)| {
            let resolved_ips = resolve(hostname);
            Check {
                matches: resolved_ips.contains(ip),
                hostname: hostname.clone(),
                expected_ip: ip.clone(),
                resolved_ips,
            }
        })
        .collect();
    Verification { results }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_hostnames_on_new_enabled_lines() {
        let old = "127.0.0.1\tlocalhost\n# 10.0.0.1\toff.test\n";
        let new = "127.0.0.1\tlocalhost\n10.0.0.1\toff.test\n127.0.0.1\tapi.test www.test # note\n# 10.0.0.2\tstill-off.test\n";
        assert_eq!(
            changed_pairs(old, new),
            [
                ("off.test".to_string(), "10.0.0.1".to_string()),
                ("api.test".to_string(), "127.0.0.1".to_string()),
                ("www.test".to_string(), "127.0.0.1".to_string()),
            ]
        );
    }

    #[test]
    fn caps_the_number_of_checks() {
        let new: String = (0..30).map(|i| format!("127.0.0.1\th{i}.test\n")).collect();
        assert_eq!(changed_pairs("", &new).len(), LIMIT);
    }

    #[test]
    fn a_check_matches_when_the_resolver_returns_the_address() {
        let pairs = vec![
            ("api.test".to_string(), "127.0.0.1".to_string()),
            ("db.test".to_string(), "10.0.0.5".to_string()),
        ];
        let verification = run(&pairs, |_| vec!["127.0.0.1".to_string()]);
        assert_eq!(
            verification
                .results
                .iter()
                .map(|c| c.matches)
                .collect::<Vec<_>>(),
            [true, false]
        );
    }
}
