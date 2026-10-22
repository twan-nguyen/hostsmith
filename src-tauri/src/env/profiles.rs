//! Saved env profiles: the content of a repo's dotenv files at one moment,
//! put back on request. Applying one saves the current files first, so the
//! switch can be undone by applying the backup.

use std::path::PathBuf;

use uuid::Uuid;

use super::files;
use super::state::{Env, ToastKind, clean_name};
use super::store::{EnvProfile, FileSnapshot};

const PROFILE_NOT_FOUND: &str = "Không tìm thấy profile";
const DUPLICATE_NAME: &str = "Tên profile đã tồn tại";

/// The name of the profile saved before `profile` is applied.
pub fn backup_name(profile: &str, now: &jiff::Zoned) -> String {
    format!("before-{profile}-{}", now.strftime("%Y%m%d-%H%M%S"))
}

impl Env {
    fn check_profile_name(
        &self,
        repo_index: usize,
        name: &str,
        except: Option<Uuid>,
    ) -> Result<String, String> {
        let name = clean_name(name)?;
        let lower = name.to_lowercase();
        if self.repos[repo_index]
            .profiles
            .iter()
            .any(|p| Some(p.id) != except && p.name.to_lowercase() == lower)
        {
            return Err(DUPLICATE_NAME.into());
        }
        Ok(name)
    }

    /// Records the repo's files as they are on disk, not as Hosven holds
    /// them, and returns the profile's name and file count.
    fn capture(&mut self, repo_index: usize, name: &str) -> Result<(String, usize), String> {
        let name = self.check_profile_name(repo_index, name, None)?;
        let root = PathBuf::from(&self.repos[repo_index].path);
        let snapshots: Vec<FileSnapshot> = files::discover(&root)
            .files()
            .iter()
            .filter_map(|relative_path| {
                let content = std::fs::read_to_string(root.join(relative_path)).ok()?;
                Some(FileSnapshot {
                    relative_path: relative_path.clone(),
                    content,
                })
            })
            .collect();
        let count = snapshots.len();
        self.repos[repo_index]
            .profiles
            .push(EnvProfile::new(&name, snapshots));
        self.persist();
        Ok((name, count))
    }

    pub fn save_profile(&mut self, repo_id: Uuid, name: &str) -> Result<(), String> {
        let index = self.repo_index(repo_id)?;
        let (name, count) = self.capture(index, name)?;
        self.show_toast(
            format!("Đã lưu profile {name} với {count} file"),
            ToastKind::Success,
        );
        Ok(())
    }

    /// Writes each file the profile holds; files it does not hold are left
    /// alone. What Hosven had loaded from the repo is dropped and read again,
    /// so pending edits there are lost: the window asks before calling this.
    pub fn apply_profile(&mut self, repo_id: Uuid, profile_id: Uuid) -> Result<(), String> {
        let index = self.repo_index(repo_id)?;
        let profile = self.repos[index]
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or(PROFILE_NOT_FOUND)?;
        for file in &profile.files {
            files::validate_relative_path(&file.relative_path)?;
        }

        // Best effort: a name taken in the same second only skips the backup.
        let _ = self.capture(index, &backup_name(&profile.name, &jiff::Zoned::now()));

        let root = PathBuf::from(&self.repos[index].path);
        let written = profile.files.iter().try_for_each(|file| {
            files::write_content(&root.join(&file.relative_path), &file.content)
        });

        // Even a partial write changed the disk under what was loaded.
        self.loaded.retain(|(id, _), _| *id != repo_id);
        if self.selected_repo == Some(repo_id) {
            let selected = self.selected_file.clone();
            self.select(repo_id, selected)?;
        } else {
            self.rediscover(repo_id);
        }
        written?;
        self.show_toast(
            format!("Đã áp dụng profile {}", profile.name),
            ToastKind::Success,
        );
        Ok(())
    }

    pub fn rename_profile(
        &mut self,
        repo_id: Uuid,
        profile_id: Uuid,
        name: &str,
    ) -> Result<(), String> {
        let index = self.repo_index(repo_id)?;
        let name = self.check_profile_name(index, name, Some(profile_id))?;
        let profile = self.repos[index]
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or(PROFILE_NOT_FOUND)?;
        profile.name = name;
        self.persist();
        Ok(())
    }

    pub fn delete_profile(&mut self, repo_id: Uuid, profile_id: Uuid) -> Result<(), String> {
        let index = self.repo_index(repo_id)?;
        let profiles = &mut self.repos[index].profiles;
        let at = profiles
            .iter()
            .position(|p| p.id == profile_id)
            .ok_or(PROFILE_NOT_FOUND)?;
        let removed = profiles.remove(at);
        if self.persist() {
            self.show_toast(format!("Đã xoá profile {}", removed.name), ToastKind::Info);
        }
        Ok(())
    }

    /// Applies a profile named by a `hosven://` link, without asking: a
    /// script cannot answer a dialog. Names match regardless of case. The
    /// repo is selected even when the profile is not found, as in Hosven 1.x.
    pub fn apply_profile_by_names(
        &mut self,
        repo_name: &str,
        profile_name: &str,
    ) -> Result<(), String> {
        let repo_id = self
            .repo_id_by_name(repo_name)
            .ok_or_else(|| format!("Không có repo tên {repo_name}"))?;
        let repo = self.repo(repo_id)?;
        let wanted = profile_name.to_lowercase();
        let profile = repo
            .profiles
            .iter()
            .find(|p| p.name.to_lowercase() == wanted)
            .map(|p| p.id);
        let repo_label = repo.name.clone();
        self.select(repo_id, None)?;
        let profile = profile
            .ok_or_else(|| format!("Repo {repo_label} không có profile tên {profile_name}"))?;
        // A link cannot ask before dropping edits, so it leaves them alone.
        if self
            .loaded
            .iter()
            .any(|((id, _), file)| *id == repo_id && file.dirty())
        {
            return Err(format!(
                "Repo {repo_label} có thay đổi chưa áp dụng nên chưa đổi profile"
            ));
        }
        self.apply_profile(repo_id, profile)
    }

    #[cfg(test)]
    fn profile_names(&self, repo_id: Uuid) -> Vec<String> {
        self.repo(repo_id)
            .map(|r| r.profiles.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::fixture::Fixture;
    use super::*;

    fn repo_with_two_files() -> Fixture {
        Fixture::new(&[(".env", "DB=staging\n"), (".env.local", "PORT=1\n")])
    }

    #[test]
    fn saving_reads_the_files_from_disk_not_the_pending_edits() {
        let mut f = repo_with_two_files();
        f.edit_db("unsaved");
        f.env.save_profile(f.repo, "  staging ").unwrap();
        assert_eq!(f.toast(), "Đã lưu profile staging với 2 file");
        let profile = &f.env.repos[0].profiles[0];
        assert_eq!(profile.name, "staging");
        assert_eq!(profile.files[0].content, "DB=staging\n");
        assert_eq!(profile.files[1].relative_path, ".env.local");
    }

    #[test]
    fn profile_names_are_unique_regardless_of_case() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "Staging").unwrap();
        assert_eq!(
            f.env.save_profile(f.repo, "staging"),
            Err(DUPLICATE_NAME.into())
        );
        assert_eq!(
            f.env.save_profile(f.repo, "   "),
            Err("Tên không hợp lệ".into())
        );
        f.env.save_profile(f.repo, "prod").unwrap();

        let prod = f.env.repos[0].profiles[1].id;
        assert_eq!(
            f.env.rename_profile(f.repo, prod, "STAGING"),
            Err(DUPLICATE_NAME.into())
        );
        f.env.rename_profile(f.repo, prod, "Prod").unwrap();
        assert_eq!(f.env.profile_names(f.repo), ["Staging", "Prod"]);
    }

    #[test]
    fn applying_backs_up_the_disk_then_writes_the_snapshot() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "staging").unwrap();
        let staging = f.env.repos[0].profiles[0].id;

        f.write(".env", "DB=prod\n");
        f.write(".env.example", "DB=\n");
        std::fs::remove_file(f.path(".env.local")).unwrap();
        f.env.refresh_from_disk();
        assert_eq!(f.entry("DB").value, "prod");

        f.env.apply_profile(f.repo, staging).unwrap();
        assert_eq!(f.toast(), "Đã áp dụng profile staging");
        assert_eq!(f.disk(".env"), "DB=staging\n");
        assert_eq!(
            f.disk(".env.local"),
            "PORT=1\n",
            "a missing file is created"
        );
        assert_eq!(
            f.disk(".env.example"),
            "DB=\n",
            "other files are left alone"
        );
        assert_eq!(
            f.entry("DB").value,
            "staging",
            "the selected file is read again"
        );
        assert!(!f.file().dirty());

        let names = f.env.profile_names(f.repo);
        assert_eq!(names.len(), 2);
        assert!(names[1].starts_with("before-staging-"), "{names:?}");
        let backup = &f.env.repos[0].profiles[1];
        let saved: Vec<_> = backup
            .files
            .iter()
            .map(|s| (s.relative_path.as_str(), s.content.as_str()))
            .collect();
        assert_eq!(saved, [(".env", "DB=prod\n"), (".env.example", "DB=\n")]);
    }

    #[test]
    fn applying_drops_pending_edits_of_the_repo() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "base").unwrap();
        let base = f.env.repos[0].profiles[0].id;
        f.edit_db("pending");
        f.env.apply_profile(f.repo, base).unwrap();
        assert_eq!(f.entry("DB").value, "staging");
        assert!(!f.env.snapshot().dirty);
    }

    #[test]
    fn the_backup_name_carries_the_local_time() {
        let now: jiff::Zoned = "2026-10-21T13:05:09+07:00[Asia/Saigon]".parse().unwrap();
        assert_eq!(backup_name("prod", &now), "before-prod-20261021-130509");
    }

    #[test]
    fn deleting_a_profile_keeps_the_files() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "staging").unwrap();
        let id = f.env.repos[0].profiles[0].id;
        f.env.delete_profile(f.repo, id).unwrap();
        assert!(f.env.repos[0].profiles.is_empty());
        assert_eq!(f.toast(), "Đã xoá profile staging");
        assert_eq!(
            f.env.delete_profile(f.repo, id),
            Err(PROFILE_NOT_FOUND.into())
        );
        assert!(f.path(".env").exists());
    }

    #[test]
    fn profiles_survive_a_restart() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "staging").unwrap();
        let store_path = f.env.store_path().to_path_buf();
        let reread = Env::new(super::super::store::EnvStore::new(store_path, None));
        assert_eq!(reread.repos[0].profiles, f.env.repos[0].profiles);
    }

    #[test]
    fn applies_a_profile_named_by_a_link() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "Staging").unwrap();
        f.write(".env", "DB=prod\n");
        let repo_name = f.env.repos[0].name.to_uppercase();

        assert_eq!(
            f.env.apply_profile_by_names("nope", "staging"),
            Err("Không có repo tên nope".into())
        );
        let missing = f.env.apply_profile_by_names(&repo_name, "qa");
        assert_eq!(
            missing,
            Err(format!(
                "Repo {} không có profile tên qa",
                f.env.repos[0].name
            ))
        );
        f.env.apply_profile_by_names(&repo_name, "STAGING").unwrap();
        assert_eq!(f.disk(".env"), "DB=staging\n");
    }

    #[test]
    fn a_link_leaves_unapplied_edits_alone() {
        let mut f = repo_with_two_files();
        f.env.save_profile(f.repo, "Staging").unwrap();
        f.edit_db("typed");
        let before = f.disk(".env");
        let name = f.env.repos[0].name.clone();
        let result = f.env.apply_profile_by_names(&name, "staging");
        assert!(result.unwrap_err().contains("chưa áp dụng"));
        assert_eq!(f.disk(".env"), before);
        assert!(f.file().dirty());
    }

    impl Fixture {
        fn edit_db(&mut self, value: &str) {
            let id = self.entry("DB").id;
            self.env
                .update_entry(
                    id,
                    super::super::state::EntryInput {
                        key: "DB".into(),
                        value: value.into(),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
    }
}
