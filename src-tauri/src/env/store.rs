//! The repo list and its saved env profiles, kept in `env.json` in the config
//! directory in the format Hosven 1.x wrote: `{ "version": 1, "repos": [...] }`,
//! sorted keys, uppercase UUIDs, ISO 8601 dates in whole seconds.

use std::fs;
use std::path::PathBuf;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config;

pub const CORRUPT_NOTICE: &str = "Không đọc được cấu hình env, Hosven khởi tạo lại danh sách repo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub profiles: Vec<EnvProfile>,
}

impl Repo {
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            path: path.to_string(),
            profiles: Vec::new(),
        }
    }
}

/// The content of a repo's dotenv files at one moment, to put back later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvProfile {
    pub id: Uuid,
    pub name: String,
    pub captured_at: Timestamp,
    pub files: Vec<FileSnapshot>,
}

impl EnvProfile {
    pub fn new(name: &str, files: Vec<FileSnapshot>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            // Whole seconds, so the date reads back equal from the file.
            captured_at: Timestamp::from_second(Timestamp::now().as_second())
                .unwrap_or(Timestamp::UNIX_EPOCH),
            files,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    pub relative_path: String,
    pub content: String,
}

/// `2026-10-21T06:05:00Z`, as Swift's `.iso8601` strategy wrote it.
pub fn iso8601(timestamp: Timestamp) -> String {
    timestamp.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn disk_id(id: Uuid) -> String {
    id.hyphenated().to_string().to_uppercase()
}

#[derive(Serialize, Deserialize)]
struct DiskConfig {
    version: u32,
    repos: Vec<DiskRepo>,
}

#[derive(Serialize, Deserialize)]
struct DiskRepo {
    id: String,
    name: String,
    path: String,
    #[serde(default)]
    profiles: Vec<DiskProfile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiskProfile {
    id: String,
    name: String,
    captured_at: String,
    files: Vec<FileSnapshot>,
}

fn decode(bytes: &[u8]) -> Result<Vec<Repo>, String> {
    let config: DiskConfig = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let id = |text: &str| Uuid::parse_str(text).map_err(|e| e.to_string());
    config
        .repos
        .into_iter()
        .map(|repo| {
            let profiles = repo
                .profiles
                .into_iter()
                .map(|p| {
                    Ok(EnvProfile {
                        id: id(&p.id)?,
                        name: p.name,
                        captured_at: p
                            .captured_at
                            .parse::<Timestamp>()
                            .map_err(|e| e.to_string())?,
                        files: p.files,
                    })
                })
                .collect::<Result<_, String>>()?;
            Ok(Repo {
                id: id(&repo.id)?,
                name: repo.name,
                path: repo.path,
                profiles,
            })
        })
        .collect()
}

fn encode(repos: &[Repo]) -> DiskConfig {
    DiskConfig {
        version: 1,
        repos: repos
            .iter()
            .map(|repo| DiskRepo {
                id: disk_id(repo.id),
                name: repo.name.clone(),
                path: repo.path.clone(),
                profiles: repo
                    .profiles
                    .iter()
                    .map(|p| DiskProfile {
                        id: disk_id(p.id),
                        name: p.name.clone(),
                        captured_at: iso8601(p.captured_at),
                        files: p.files.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub struct Loaded {
    pub repos: Vec<Repo>,
    /// Something the user should hear about, such as a file set aside.
    pub notice: Option<String>,
}

pub struct EnvStore {
    path: PathBuf,
    /// Where Hosven 1.x kept the same data, copied once when `path` is missing.
    legacy: Option<PathBuf>,
    /// Set when the file exists but could not be read or moved aside, so a
    /// save would overwrite what the user had.
    read_only: bool,
}

impl EnvStore {
    pub fn new(path: PathBuf, legacy: Option<PathBuf>) -> Self {
        Self {
            path,
            legacy,
            read_only: false,
        }
    }

    pub fn default_location() -> Self {
        Self::new(
            config::config_dir().join("env.json"),
            Some(
                config::home_dir()
                    .join("Library/Application Support/com.twannguyen.hosven/env-config.json"),
            ),
        )
    }

    pub fn load(&mut self) -> Loaded {
        match fs::read(&self.path) {
            Ok(bytes) => match decode(&bytes) {
                Ok(repos) => Loaded {
                    repos,
                    notice: None,
                },
                Err(_) => match config::set_aside(&self.path) {
                    Some(_) => Loaded {
                        repos: Vec::new(),
                        notice: Some(CORRUPT_NOTICE.into()),
                    },
                    None => self.unreadable("Không đọc được env.json. Hosven sẽ không ghi đè nó."),
                },
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => self.migrate(),
            Err(error) => self.unreadable(&format!(
                "Không đọc được env.json: {error}. Hosven sẽ không ghi đè nó."
            )),
        }
    }

    fn unreadable(&mut self, notice: &str) -> Loaded {
        self.read_only = true;
        Loaded {
            repos: Vec::new(),
            notice: Some(notice.to_string()),
        }
    }

    /// Copies the Application Support file once, byte for byte. The old file
    /// stays where it is, so a downgrade still finds it.
    fn migrate(&self) -> Loaded {
        let Some(bytes) = self.legacy.as_deref().and_then(|p| fs::read(p).ok()) else {
            return Loaded {
                repos: Vec::new(),
                notice: None,
            };
        };
        match decode(&bytes) {
            Ok(repos) => {
                // A failed copy is not fatal: the next save writes the file.
                let _ = config::write_atomic(&self.path, &bytes);
                Loaded {
                    repos,
                    notice: None,
                }
            }
            Err(_) => Loaded {
                repos: Vec::new(),
                notice: Some(CORRUPT_NOTICE.into()),
            },
        }
    }

    pub fn save(&self, repos: &[Repo]) -> Result<(), String> {
        if self.read_only {
            return Err("file hiện có không đọc được, Hosven không ghi đè nó".into());
        }
        config::write_json_atomic(&self.path, &encode(repos))
    }

    #[cfg(test)]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

/// Where a store for tests lives: a fresh directory under the system temp dir.
#[cfg(test)]
pub fn temp_store() -> (EnvStore, PathBuf) {
    let dir = std::env::temp_dir().join(format!("hosven-envstore-{}", Uuid::new_v4()));
    (EnvStore::new(dir.join("env.json"), None), dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SWIFT_FIXTURE: &str = include_str!("../../tests/fixtures/env/env-config.swift.json");

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn reads_the_file_hosven_1_wrote() {
        let repos = decode(SWIFT_FIXTURE.as_bytes()).unwrap();
        assert_eq!(repos.len(), 2);
        let api = &repos[0];
        assert_eq!(
            api.id,
            Uuid::parse_str("6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7A").unwrap()
        );
        assert_eq!(api.name, "api");
        assert_eq!(api.path, "/Users/twan/code/api");
        assert_eq!(api.profiles.len(), 1);
        let staging = &api.profiles[0];
        assert_eq!(staging.name, "staging");
        assert_eq!(staging.captured_at, ts("2026-10-21T06:05:00Z"));
        assert_eq!(staging.files.len(), 2);
        assert_eq!(staging.files[0].relative_path, ".env");
        assert_eq!(
            staging.files[0].content,
            "DB_HOST=staging.db\nDEBUG=false\n"
        );
        assert!(repos[1].profiles.is_empty());
    }

    #[test]
    fn writes_the_format_hosven_1_wrote_and_reads_it_back() {
        let (store, dir) = temp_store();
        let repos = decode(SWIFT_FIXTURE.as_bytes()).unwrap();
        store.save(&repos).unwrap();

        let text = fs::read_to_string(store.path()).unwrap();
        assert!(text.contains("\"id\": \"6F1C2A52-6D0E-4D43-9A2B-3C1F0E9D8B7A\""));
        assert!(text.contains("\"capturedAt\": \"2026-10-21T06:05:00Z\""));
        assert!(text.contains("\"relativePath\": \".env\""));
        assert!(text.contains("\"version\": 1"));
        assert!(text.find("\"repos\"").unwrap() < text.find("\"version\"").unwrap());
        assert!(text.find("\"content\"").unwrap() < text.find("\"relativePath\"").unwrap());

        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let swift: serde_json::Value = serde_json::from_str(SWIFT_FIXTURE).unwrap();
        assert_eq!(value, swift, "the same data as the Swift file");

        let mut reread = EnvStore::new(store.path().to_path_buf(), None);
        assert_eq!(reread.load().repos, repos);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn new_profiles_keep_whole_seconds() {
        let (store, dir) = temp_store();
        let mut repo = Repo::new("web", "/tmp/web");
        repo.profiles.push(EnvProfile::new(
            "local",
            vec![FileSnapshot {
                relative_path: ".env".into(),
                content: "A=1\n".into(),
            }],
        ));
        store.save(std::slice::from_ref(&repo)).unwrap();
        let mut reread = EnvStore::new(store.path().to_path_buf(), None);
        assert_eq!(reread.load().repos, vec![repo]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_file_starts_empty() {
        let (mut store, _) = temp_store();
        let loaded = store.load();
        assert!(loaded.repos.is_empty());
        assert!(loaded.notice.is_none());
    }

    #[test]
    fn copies_the_application_support_file_once() {
        let (_, dir) = temp_store();
        let legacy = dir.join("Application Support/com.twannguyen.hosven/env-config.json");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, SWIFT_FIXTURE).unwrap();
        let path = dir.join("config/hosven/env.json");

        let mut store = EnvStore::new(path.clone(), Some(legacy.clone()));
        let loaded = store.load();
        assert_eq!(loaded.repos.len(), 2);
        assert!(loaded.notice.is_none());
        assert_eq!(fs::read_to_string(&path).unwrap(), SWIFT_FIXTURE);
        assert!(legacy.exists(), "the old file stays in place");

        // Once env.json exists, the old file is no longer read.
        fs::write(&legacy, "{ broken").unwrap();
        let mut again = EnvStore::new(path, Some(legacy));
        assert_eq!(again.load().repos.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_unreadable_old_file_starts_empty_and_stays_in_place() {
        let (_, dir) = temp_store();
        let legacy = dir.join("env-config.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&legacy, "not json").unwrap();
        let path = dir.join("env.json");
        let mut store = EnvStore::new(path.clone(), Some(legacy.clone()));
        let loaded = store.load();
        assert!(loaded.repos.is_empty());
        assert_eq!(loaded.notice.as_deref(), Some(CORRUPT_NOTICE));
        assert!(!path.exists());
        assert_eq!(fs::read_to_string(legacy).unwrap(), "not json");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_corrupt_file_is_set_aside_not_overwritten() {
        let (mut store, dir) = temp_store();
        fs::create_dir_all(&dir).unwrap();
        fs::write(store.path(), r#"{"version":1,"repos":[{"id":"nope"}]}"#).unwrap();

        let loaded = store.load();
        assert!(loaded.repos.is_empty());
        assert_eq!(loaded.notice.as_deref(), Some(CORRUPT_NOTICE));
        assert!(!store.path().exists());
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 1);
        assert!(names[0].starts_with("env.json.corrupt-"));

        store.save(&[Repo::new("api", "/tmp/api")]).unwrap();
        assert!(store.path().exists(), "saving after a set-aside is safe");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_that_cannot_be_read_is_never_written_over() {
        let (mut store, dir) = temp_store();
        // A directory where the file should be: the read fails, but not with "not found".
        fs::create_dir_all(store.path()).unwrap();
        let loaded = store.load();
        assert!(loaded.repos.is_empty());
        assert!(loaded.notice.unwrap().contains("không ghi đè"));
        assert!(store.save(&[Repo::new("api", "/tmp/api")]).is_err());
        assert!(store.path().is_dir());
        fs::remove_dir_all(dir).unwrap();
    }
}
