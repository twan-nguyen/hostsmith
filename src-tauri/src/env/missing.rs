//! Variables the repo's `.env.example` defines and the selected file lacks.

use std::collections::HashSet;

use super::files::EXAMPLE_FILE;
use super::parse::EnvEntry;
use super::state::Env;

/// The comment a variable added this way carries into the file.
pub const REQUIRED_COMMENT: &str = "bắt buộc theo .env.example";

/// Keys of `example` that `current` does not define, in the example's order.
/// A key that is present but switched off is not missing.
pub fn missing_keys(current: &[EnvEntry], example: &[EnvEntry]) -> Vec<String> {
    let present: HashSet<&str> = current
        .iter()
        .filter(|e| !e.is_blank_or_comment)
        .map(|e| e.key.as_str())
        .collect();
    let mut missing: Vec<String> = Vec::new();
    for entry in example.iter().filter(|e| !e.is_blank_or_comment) {
        if !present.contains(entry.key.as_str()) && !missing.contains(&entry.key) {
            missing.push(entry.key.clone());
        }
    }
    missing
}

impl Env {
    /// Missing keys of the selected file; none for the example itself.
    pub fn missing_keys(&self) -> Vec<String> {
        if self.selected_file.as_deref() == Some(EXAMPLE_FILE) {
            return Vec::new();
        }
        match (self.selected(), &self.example) {
            (Some(file), Some(example)) => missing_keys(&file.entries, example),
            _ => Vec::new(),
        }
    }

    /// Adds every missing key with an empty value, so none is overlooked.
    /// Returns how many were added.
    pub fn add_missing(&mut self) -> Result<usize, String> {
        if self.selected_file.as_deref() == Some(EXAMPLE_FILE) {
            return Ok(0);
        }
        let example = self.example.clone().unwrap_or_default();
        let file = self.editable()?;
        let keys = missing_keys(&file.entries, &example);
        for key in &keys {
            file.entries
                .push(EnvEntry::variable(key, "", REQUIRED_COMMENT));
        }
        Ok(keys.len())
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::super::state::fixture::Fixture;
    use super::*;

    #[test]
    fn lists_example_keys_the_file_lacks() {
        let current = parse("A=1\n# B=2\n# a comment\n");
        let example = parse("A=\nB=\nC=\n# D=\n# note\nC=again\n");
        assert_eq!(missing_keys(&current, &example), ["C", "D"]);
    }

    #[test]
    fn the_selected_file_is_compared_with_the_example() {
        let mut f = Fixture::new(&[
            (".env", "A=1\n# B=off\n"),
            (".env.example", "A=\nB=\nSECRET_KEY=\nPORT=3000\n"),
        ]);
        assert_eq!(f.env.snapshot().missing_keys, ["SECRET_KEY", "PORT"]);

        assert_eq!(f.env.add_missing(), Ok(2));
        assert!(f.env.snapshot().missing_keys.is_empty());
        f.env.apply(false).unwrap();
        assert_eq!(
            f.disk(".env"),
            "A=1\n# B=off\nSECRET_KEY= # bắt buộc theo .env.example\nPORT= # bắt buộc theo .env.example\n"
        );
        // Read back, the comment stays a comment and the value stays empty.
        f.env.reload_from_disk().unwrap();
        assert_eq!(f.entry("PORT").value, "");
        assert_eq!(f.entry("PORT").comment, REQUIRED_COMMENT);
    }

    #[test]
    fn the_example_itself_has_nothing_missing() {
        let mut f = Fixture::new(&[(".env.example", "A=\n")]);
        assert_eq!(f.env.selected_file.as_deref(), Some(".env.example"));
        assert!(f.env.snapshot().missing_keys.is_empty());
        assert_eq!(f.env.add_missing(), Ok(0));
    }

    #[test]
    fn no_example_means_nothing_missing() {
        let f = Fixture::standard();
        assert!(f.env.snapshot().missing_keys.is_empty());
    }

    #[test]
    fn an_edited_example_is_read_again_on_refresh() {
        let mut f = Fixture::new(&[(".env", "A=1\n"), (".env.example", "A=\n")]);
        assert!(f.env.snapshot().missing_keys.is_empty());
        f.write(".env.example", "A=\nNEW=\n");
        f.env.refresh_from_disk();
        assert_eq!(f.env.snapshot().missing_keys, ["NEW"]);
    }
}
