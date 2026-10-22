//! Profile metadata layered over the `## [tag:Name]` blocks of the hosts file:
//! colour, ⌘-number and whether the profile is an environment. Stored in
//! `profiles.json` in the config directory, in the format Hosven 1.x wrote.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileColor {
    Purple,
    Green,
    Amber,
    Blue,
    Red,
}

impl ProfileColor {
    /// Local, Staging and Production; `green` and `purple` are neutral.
    pub fn names_environment(self) -> bool {
        matches!(self, Self::Blue | Self::Amber | Self::Red)
    }

    /// Unknown values from a newer or hand-edited file read as neutral rather
    /// than making the whole file unreadable.
    fn parse(value: &str) -> Self {
        match value {
            "green" => Self::Green,
            "amber" => Self::Amber,
            "blue" => Self::Blue,
            "red" => Self::Red,
            _ => Self::Purple,
        }
    }

    /// The colour a block found in the hosts file starts with, from its name.
    pub fn guess(name: &str) -> Self {
        let key = name.to_lowercase();
        if key.contains("prod") {
            Self::Red
        } else if ["staging", "test", "uat"].iter().any(|k| key.contains(k)) {
            Self::Amber
        } else if key.contains("dev") || key.contains("local") {
            Self::Blue
        } else {
            Self::Purple
        }
    }
}

/// An environment excludes the others: switching to it turns every other
/// environment's entries off. A block is switched on and off on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub color: ProfileColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut_number: Option<u32>,
    pub is_exclusive: bool,
}

impl Profile {
    pub fn new(name: &str, color: ProfileColor, is_exclusive: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            color,
            shortcut_number: None,
            is_exclusive,
        }
    }
}

/// Non-empty, and free of what would break a `## [tag:Name]` marker.
pub fn is_valid_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && !trimmed.contains(['[', ']', '\n', '\r'])
}

/// A profile as `profiles.json` holds it. Swift wrote UUIDs in uppercase;
/// keeping that leaves the file unchanged for anyone who versions their
/// config. The UI gets the usual lowercase form, like every other id.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiskProfile<'a> {
    id: String,
    name: &'a str,
    color: ProfileColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortcut_number: Option<u32>,
    is_exclusive: bool,
}

impl<'a> From<&'a Profile> for DiskProfile<'a> {
    fn from(profile: &'a Profile) -> Self {
        Self {
            id: profile.id.hyphenated().to_string().to_uppercase(),
            name: &profile.name,
            color: profile.color,
            shortcut_number: profile.shortcut_number,
            is_exclusive: profile.is_exclusive,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProfile {
    id: Uuid,
    name: String,
    color: String,
    shortcut_number: Option<u32>,
    is_exclusive: Option<bool>,
}

impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let stored = StoredProfile::deserialize(deserializer)?;
        let color = ProfileColor::parse(&stored.color);
        // Profiles saved before the flag existed are environments when their
        // colour or their name says so. Hosven 1.1 coloured production green,
        // so the name decides for those.
        let is_exclusive = stored.is_exclusive.unwrap_or_else(|| {
            let name = stored.name.to_lowercase();
            color.names_environment()
                || ["prod", "staging", "dev", "local"]
                    .iter()
                    .any(|k| name.contains(k))
        });
        Ok(Self {
            id: stored.id,
            name: stored.name,
            color,
            shortcut_number: stored.shortcut_number,
            is_exclusive,
        })
    }
}

pub struct Loaded {
    pub profiles: Vec<Profile>,
    /// Something the user should hear about, such as a file set aside.
    pub notice: Option<String>,
}

pub trait ProfileStore: Send {
    fn load(&self) -> Loaded;
    fn save(&self, profiles: &[Profile]) -> Result<(), String>;
}

/// `profiles.json` in the config directory.
pub struct FileProfileStore {
    path: PathBuf,
    /// Set when the file exists but could not be read or moved aside, so a
    /// save would overwrite what the user had.
    read_only: AtomicBool,
}

impl FileProfileStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            read_only: AtomicBool::new(false),
        }
    }

    fn unreadable(&self, notice: String) -> Loaded {
        self.read_only.store(true, Ordering::SeqCst);
        Loaded {
            profiles: Vec::new(),
            notice: Some(notice),
        }
    }

    pub fn default_location() -> Self {
        Self::new(config::config_dir().join("profiles.json"))
    }
}

impl ProfileStore for FileProfileStore {
    fn load(&self) -> Loaded {
        match fs::read(&self.path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(profiles) => Loaded {
                    profiles,
                    notice: None,
                },
                Err(_) => match config::set_aside(&self.path) {
                    Some(moved) => Loaded {
                        profiles: Vec::new(),
                        notice: Some(format!(
                            "Could not read profiles.json, so HostSmith moved it to {}",
                            moved.file_name().unwrap_or_default().to_string_lossy()
                        )),
                    },
                    None => self.unreadable(
                        "Could not read profiles.json. HostSmith will not overwrite it.".into(),
                    ),
                },
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Loaded {
                profiles: Vec::new(),
                notice: None,
            },
            Err(error) => self.unreadable(format!(
                "Could not read profiles.json: {error}. HostSmith will not overwrite it."
            )),
        }
    }

    fn save(&self, profiles: &[Profile]) -> Result<(), String> {
        if self.read_only.load(Ordering::SeqCst) {
            return Err(
                "the existing file could not be read, so HostSmith will not overwrite it".into(),
            );
        }
        let disk: Vec<DiskProfile> = profiles.iter().map(DiskProfile::from).collect();
        config::write_json_atomic(&self.path, &disk)
    }
}

/// Keeps profiles in memory, for tests.
#[cfg(test)]
#[derive(Default, Clone)]
pub struct MemoryStore(pub std::sync::Arc<std::sync::Mutex<Vec<Profile>>>);

#[cfg(test)]
impl MemoryStore {
    pub fn with(profiles: Vec<Profile>) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(profiles)))
    }

    pub fn stored(&self) -> Vec<Profile> {
        self.0.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl ProfileStore for MemoryStore {
    fn load(&self) -> Loaded {
        Loaded {
            profiles: self.stored(),
            notice: None,
        }
    }

    fn save(&self, profiles: &[Profile]) -> Result<(), String> {
        *self.0.lock().unwrap() = profiles.to_vec();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (FileProfileStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("hostsmith-profiles-{}", Uuid::new_v4()));
        let path = dir.join("profiles.json");
        (FileProfileStore::new(path.clone()), path)
    }

    fn sample() -> Vec<Profile> {
        let mut release = Profile::new("Release", ProfileColor::Purple, true);
        release.shortcut_number = Some(1);
        vec![release, Profile::new("Docker", ProfileColor::Green, false)]
    }

    #[test]
    fn an_empty_store_has_no_profiles() {
        let (store, _) = temp_store();
        assert!(store.load().profiles.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let (store, _) = temp_store();
        let profiles = sample();
        store.save(&profiles).unwrap();
        assert_eq!(store.load().profiles, profiles);
        store.save(&profiles[1..]).unwrap();
        assert_eq!(store.load().profiles, profiles[1..]);
    }

    #[test]
    fn writes_the_format_hosven_1_wrote() {
        let (store, path) = temp_store();
        let mut profile = Profile::new("Release", ProfileColor::Purple, true);
        profile.id = Uuid::parse_str("6f1c2a52-6d0e-4d43-9a2b-3c1f0e9d8b7a").unwrap();
        store.save(&[profile]).unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("\"id\": \"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7A\""));
        assert!(text.contains("\"isExclusive\": true"));
        assert!(
            !text.contains("shortcutNumber"),
            "a missing shortcut is left out"
        );
        assert!(text.find("\"color\"").unwrap() < text.find("\"name\"").unwrap());
    }

    #[test]
    fn profiles_without_the_flag_migrate_by_colour_or_name() {
        let json = r#"[
            {"id":"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7A","name":"Staging","color":"amber"},
            {"id":"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7B","name":"Production","color":"green","shortcutNumber":2},
            {"id":"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7C","name":"Mock","color":"purple"},
            {"id":"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7D","name":"Docker","color":"blue","isExclusive":false}
        ]"#;
        let profiles: Vec<Profile> = serde_json::from_str(json).unwrap();
        assert_eq!(
            profiles.iter().map(|p| p.is_exclusive).collect::<Vec<_>>(),
            [true, true, false, false]
        );
        assert_eq!(profiles[1].shortcut_number, Some(2));
    }

    #[test]
    fn an_unknown_colour_reads_as_neutral() {
        let json = r#"[{"id":"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7A","name":"QA","color":"teal","isExclusive":true}]"#;
        let profiles: Vec<Profile> = serde_json::from_str(json).unwrap();
        assert_eq!(profiles[0].color, ProfileColor::Purple);
    }

    #[test]
    fn a_corrupt_file_is_set_aside_not_overwritten() {
        let (store, path) = temp_store();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, [0xFF, 0x00, 0xAB]).unwrap();

        let loaded = store.load();
        assert!(loaded.profiles.is_empty());
        assert!(loaded.notice.unwrap().contains("profiles.json.corrupt-"));
        assert!(!path.exists());
        let moved: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(moved.len(), 1);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_never_written_over() {
        let (store, path) = temp_store();
        // A directory where the file should be: the read fails, but not with "not found".
        fs::create_dir_all(&path).unwrap();
        let loaded = store.load();
        assert!(loaded.profiles.is_empty());
        assert!(loaded.notice.unwrap().contains("will not overwrite"));
        assert!(store.save(&sample()).is_err());
        assert!(path.is_dir());
    }

    #[test]
    fn the_ui_gets_lowercase_ids_like_every_other_id() {
        let profile = Profile::new("Local", ProfileColor::Blue, true);
        let value = serde_json::to_value(&profile).unwrap();
        assert_eq!(value["id"], profile.id.to_string());
        assert_eq!(value["isExclusive"], true);
    }

    #[test]
    fn validates_names() {
        assert!(is_valid_name("Release"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("  "));
        assert!(!is_valid_name("Bad[Name"));
        assert!(!is_valid_name("Multi\nLine"));
    }

    #[test]
    fn guesses_colours_from_names() {
        assert_eq!(ProfileColor::guess("Production"), ProfileColor::Red);
        assert_eq!(ProfileColor::guess("UAT"), ProfileColor::Amber);
        assert_eq!(ProfileColor::guess("local-dev"), ProfileColor::Blue);
        assert_eq!(ProfileColor::guess("Docker"), ProfileColor::Purple);
    }
}
