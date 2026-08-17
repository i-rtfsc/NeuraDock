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

    let backup_path = backup_path_for(path)?;

    fs::copy(path, &backup_path)
        .with_context(|| format!("Failed to back up {}", path.display()))?;

    Ok(Some(backup_path))
}

/// The sibling backup path this module would use for `path`.
pub fn backup_path_for(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Config path has no valid file name")?;
    Ok(path.with_file_name(format!("{file_name}{BACKUP_SUFFIX}")))
}

/// Remove the sibling backup for `path`, if one exists.
///
/// Backups are taken *before* a write, so after an operation whose purpose is
/// to remove credentials (e.g. "clear config") the backup still holds a live,
/// usable token. Nothing else in the codebase deletes these files, so they
/// would otherwise sit indefinitely in a location the UI never shows.
pub fn remove_backup(path: &Path) -> Result<()> {
    let backup_path = backup_path_for(path)?;
    match fs::remove_file(&backup_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("Failed to remove {}", backup_path.display())),
    }
}

/// Write `contents` to `path` atomically: write a sibling temp file, fsync,
/// then rename over the target and fsync the parent directory. `private`
/// forces 0o600 (for key material); otherwise the original file's permissions
/// are preserved.
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
    // `flush()` is a no-op on std::fs::File (it has no userspace buffer) and
    // contributes nothing to durability. Without an explicit fsync the data
    // blocks sit in the page cache while the rename's metadata may already be
    // durable, so a crash can leave a committed rename pointing at an empty or
    // partial file -- which is how a snapshot meant to protect the user's
    // config ended up destroying it.
    tmp.sync_all()
        .with_context(|| format!("Failed to flush {} to disk", tmp_path.display()))?;
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

    // Persist the rename itself. Without this the directory entry can still be
    // lost on power failure even though the file contents are durable.
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }

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

    #[test]
    fn remove_backup_deletes_credential_copy_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"ANTHROPIC_AUTH_TOKEN":"sk-live"}"#).unwrap();

        let backup = backup_file(&path).unwrap().unwrap();
        assert!(backup.exists());

        remove_backup(&path).unwrap();
        assert!(
            !backup.exists(),
            "backup still holds a usable token after clear"
        );

        // Clearing twice, or clearing a config that was never backed up, must
        // not surface an error.
        remove_backup(&path).unwrap();
    }

    #[test]
    fn atomic_write_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        atomic_write(&path, "a = 1\n", false).unwrap();

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains(".neuradock.tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
    }
}
