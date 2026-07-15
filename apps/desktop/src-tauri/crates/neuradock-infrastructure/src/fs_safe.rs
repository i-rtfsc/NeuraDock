//! Safe file writing for user-owned config files.
//!
//! External tool configs (`~/.codex/*`, `~/.claude/*`) belong to the user, so
//! every write goes through a backup + atomic rename instead of a plain
//! `fs::write` that could truncate the file on a crash mid-write.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Suffix appended to the sibling backup created before each write.
const BACKUP_SUFFIX: &str = ".neuradock.bak";

/// Copy `path` to a sibling `<name>.neuradock.bak` before overwriting it.
/// Returns the backup path, or `None` when the file does not exist yet.
/// The previous backup is replaced, keeping exactly one generation.
pub fn backup_file(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Config path has no valid file name")?;
    let backup_path = path.with_file_name(format!("{file_name}{BACKUP_SUFFIX}"));

    fs::copy(path, &backup_path)
        .with_context(|| format!("Failed to back up {}", path.display()))?;

    Ok(Some(backup_path))
}

/// Write `contents` to `path` atomically: write a sibling temp file, flush,
/// then rename over the target. `private` forces 0o600 (for key material);
/// otherwise the original file's permissions are preserved.
pub fn atomic_write(path: &Path, contents: &str, private: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Config path has no valid file name")?;
    let tmp_path = path.with_file_name(format!(".{file_name}.neuradock.tmp"));

    let mut tmp = fs::File::create(&tmp_path)
        .with_context(|| format!("Failed to create temp file {}", tmp_path.display()))?;
    tmp.write_all(contents.as_bytes())?;
    tmp.flush()?;
    drop(tmp);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = if private {
            0o600
        } else {
            path.metadata()
                .map(|meta| meta.permissions().mode())
                .unwrap_or(0o644)
        };
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    {
        let _ = private;
    }

    fs::rename(&tmp_path, path).with_context(|| format!("Failed to replace {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file_and_parent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");

        atomic_write(&path, "a = 1\n", false).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "a = 1\n");
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "old").unwrap();

        atomic_write(&path, "new", false).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_private_sets_owner_only_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");

        atomic_write(&path, "{}", true).unwrap();

        let mode = path.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn backup_file_copies_existing_and_skips_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        assert!(backup_file(&path).unwrap().is_none());

        fs::write(&path, "original").unwrap();
        let backup = backup_file(&path).unwrap().unwrap();

        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
        assert!(backup.ends_with("settings.json.neuradock.bak"));
    }
}
