use super::model::ToastKind;
use super::parse::{is_valid_ip, parse_entry};
use super::state::Hosts;

impl Hosts {
    /// Adds the `ip hostname` lines of pasted text as enabled entries outside
    /// every block, skipping comments and hostnames the file already has.
    /// The whole import is one undo step. Returns how many were added.
    pub fn import(&mut self, text: &str) -> Result<usize, String> {
        self.prepare_edit()?;
        let mut added = 0;
        for line in text.split('\n').map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(entry) = parse_entry(line, None) else {
                continue;
            };
            if !is_valid_ip(&entry.ip) || entry.hostnames().any(|h| self.hostname_exists(h)) {
                continue;
            }
            if added == 0 {
                self.history.push(&self.entries);
            }
            self.entries.push(entry);
            added += 1;
        }
        if added > 0 {
            self.show_toast(format!("Đã nhập {added} entry mới"), ToastKind::Success);
        } else {
            self.show_toast("Không có entry mới để nhập".into(), ToastKind::Info);
        }
        Ok(added)
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::tests::{BASE, hosts};

    #[test]
    fn imports_new_address_lines_as_one_undo_step() {
        let (mut h, _) = hosts(BASE, vec![]);
        let added = h
            .import(
                "# a comment\n\n127.0.0.1 api.test www.test # api\nnot-an-ip web.test\n127.0.0.1 localhost\n10.0.0.2\tdb.test\n",
            )
            .unwrap();
        assert_eq!(added, 2);
        let api = h.entries.iter().find(|e| e.hostname == "api.test").unwrap();
        assert_eq!(api.aliases, ["www.test"]);
        assert_eq!(api.comment, "api");
        assert!(api.enabled && api.tag.is_none());
        assert_eq!(h.history.depth(), 1);
        assert_eq!(h.toast().unwrap().message, "Đã nhập 2 entry mới");
    }

    #[test]
    fn says_so_when_nothing_is_new() {
        let (mut h, _) = hosts(BASE, vec![]);
        assert_eq!(h.import("127.0.0.1 LOCALHOST\n").unwrap(), 0);
        assert_eq!(h.toast().unwrap().message, "Không có entry mới để nhập");
        assert!(!h.history.can_undo());
    }
}
