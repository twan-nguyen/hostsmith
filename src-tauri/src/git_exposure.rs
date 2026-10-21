//! Whether a file inside a repo can leak through git. Every failure to ask git
//! counts as "not a git checkout", so the pane stays quiet rather than warning
//! about a folder git does not manage.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Exposure {
    /// git ignores the file, or the folder is not a git checkout.
    Safe,
    /// The file is not ignored, so `git add .` would stage it.
    NotIgnored,
    /// The file is already committed or staged.
    Tracked,
}

/// A path inside the repo, as git and `.gitignore` take it. Anything that
/// could point outside the repo, or add a second line to `.gitignore`, is
/// refused.
fn validate_relative(relative: &str) -> Result<(), String> {
    let escapes = Path::new(relative)
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir));
    if relative.trim().is_empty() || relative.contains(['\n', '\r']) || escapes {
        return Err(format!("Đường dẫn không hợp lệ: {relative}"));
    }
    Ok(())
}

fn validate(repo: &str, relative: &str) -> Result<PathBuf, String> {
    validate_relative(relative)?;
    let repo = PathBuf::from(repo);
    if !repo.is_absolute() || !repo.is_dir() {
        return Err(format!("Không tìm thấy thư mục repo {}", repo.display()));
    }
    Ok(repo)
}

/// git's exit status, or `None` when git could not run.
fn git(repo: &Path, args: &[&str]) -> Option<i32> {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?
        .code()
}

fn check(repo: &Path, relative: &str) -> Exposure {
    if git(repo, &["rev-parse", "--is-inside-work-tree"]) != Some(0) {
        return Exposure::Safe;
    }
    if git(repo, &["ls-files", "--error-unmatch", "--", relative]) == Some(0) {
        return Exposure::Tracked;
    }
    if git(repo, &["check-ignore", "-q", "--", relative]) == Some(0) {
        Exposure::Safe
    } else {
        Exposure::NotIgnored
    }
}

/// `existing` with `relative` on a line of its own at the end. A file that
/// already lists it exactly is left as it is.
fn appended(existing: &str, relative: &str) -> Option<String> {
    if existing.lines().any(|line| line.trim_end() == relative) {
        return None;
    }
    let mut content = existing.to_string();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(relative);
    content.push('\n');
    Some(content)
}

/// Lists the file in the repo's `.gitignore`, creating it if needed, then asks
/// git again: a tracked file stays tracked until it is removed from the index.
fn add_to_gitignore(repo: &Path, relative: &str) -> Result<Exposure, String> {
    let path = repo.join(".gitignore");
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("Không đọc được .gitignore: {error}")),
    };
    if let Some(content) = appended(&existing, relative) {
        config::write_atomic(&path, content.as_bytes())
            .map_err(|e| format!("Không ghi được .gitignore: {e}"))?;
    }
    Ok(check(repo, relative))
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_exposure(repo_path: String, relative_path: String) -> Result<Exposure, String> {
    blocking(move || {
        let repo = validate(&repo_path, &relative_path)?;
        Ok(check(&repo, &relative_path))
    })
    .await
}

#[tauri::command]
pub async fn git_add_to_gitignore(
    repo_path: String,
    relative_path: String,
) -> Result<Exposure, String> {
    blocking(move || {
        let repo = validate(&repo_path, &relative_path)?;
        add_to_gitignore(&repo, &relative_path)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hosven-git-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A fresh repository that ignores the developer's global excludes file,
    /// which may well list `.env` already.
    fn repo() -> PathBuf {
        let dir = temp_dir();
        assert_eq!(git(&dir, &["init", "-q"]), Some(0));
        assert_eq!(
            git(&dir, &["config", "core.excludesFile", "/dev/null"]),
            Some(0)
        );
        dir
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    }

    #[test]
    fn refuses_paths_that_leave_the_repo_or_break_lines() {
        assert!(validate_relative(".env").is_ok());
        assert!(validate_relative("apps/web/.env.local").is_ok());
        assert!(validate_relative("./.env").is_ok());
        assert!(validate_relative("").is_err());
        assert!(validate_relative("/etc/passwd").is_err());
        assert!(validate_relative("../secret").is_err());
        assert!(validate_relative("apps/../../secret").is_err());
        assert!(validate_relative(".env\n!*").is_err());
        assert!(validate("relative/repo", ".env").is_err());
        assert!(validate("/definitely/not/here", ".env").is_err());
    }

    #[test]
    fn appends_on_a_line_of_its_own() {
        assert_eq!(appended("", ".env").as_deref(), Some(".env\n"));
        assert_eq!(
            appended("node_modules", ".env").as_deref(),
            Some("node_modules\n.env\n")
        );
        assert_eq!(
            appended("node_modules\n", ".env").as_deref(),
            Some("node_modules\n.env\n")
        );
        assert_eq!(appended("node_modules\n.env\n", ".env"), None);
        assert_eq!(
            appended(".env.local\n", ".env").as_deref(),
            Some(".env.local\n.env\n")
        );
    }

    #[test]
    fn a_folder_outside_git_is_safe() {
        let dir = temp_dir();
        fs::write(dir.join(".env"), "A=1\n").unwrap();
        assert_eq!(check(&dir, ".env"), Exposure::Safe);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rechecks_after_adding_to_gitignore() {
        if !git_available() {
            eprintln!("git is not installed; skipping");
            return;
        }
        let dir = repo();
        fs::write(dir.join(".env"), "A=1\n").unwrap();
        fs::write(dir.join(".env.local"), "B=2\n").unwrap();
        fs::write(dir.join(".gitignore"), "node_modules").unwrap();

        assert_eq!(check(&dir, ".env"), Exposure::NotIgnored);
        assert_eq!(add_to_gitignore(&dir, ".env"), Ok(Exposure::Safe));
        assert_eq!(
            fs::read_to_string(dir.join(".gitignore")).unwrap(),
            "node_modules\n.env\n"
        );

        // Listing a staged file does not untrack it, and the answer says so.
        assert_eq!(git(&dir, &["add", "--", ".env.local"]), Some(0));
        assert_eq!(check(&dir, ".env.local"), Exposure::Tracked);
        assert_eq!(add_to_gitignore(&dir, ".env.local"), Ok(Exposure::Tracked));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creates_the_gitignore_when_missing() {
        if !git_available() {
            eprintln!("git is not installed; skipping");
            return;
        }
        let dir = repo();
        fs::write(dir.join(".env"), "A=1\n").unwrap();
        assert_eq!(add_to_gitignore(&dir, ".env"), Ok(Exposure::Safe));
        assert_eq!(
            fs::read_to_string(dir.join(".gitignore")).unwrap(),
            ".env\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
