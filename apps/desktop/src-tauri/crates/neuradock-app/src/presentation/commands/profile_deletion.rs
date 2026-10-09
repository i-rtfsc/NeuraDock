use anyhow::{Context, Result};
use std::path::PathBuf;

/// Rename only explicitly selected files until the profile records are saved.
/// Dropping an uncommitted removal restores the original paths.
pub(super) struct StagedProfileRemoval {
    moved: Vec<(PathBuf, PathBuf)>,
    committed: bool,
}

impl StagedProfileRemoval {
    pub(super) fn stage(paths: Vec<PathBuf>) -> Result<Self> {
        let mut removal = Self {
            moved: Vec::new(),
            committed: false,
        };
        let mut seen = std::collections::HashSet::new();
        for original in paths {
            if !seen.insert(original.clone()) {
                continue;
            }
            let metadata = match std::fs::symlink_metadata(&original) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            anyhow::ensure!(
                metadata.is_file() || metadata.file_type().is_symlink(),
                "Refusing to delete a directory: {}",
                original.display()
            );
            original.file_name().context("Profile file has no name")?;
            let temporary =
                original.with_file_name(format!(".neuradock-delete-{}.tmp", uuid::Uuid::new_v4()));
            std::fs::rename(&original, &temporary)
                .with_context(|| format!("Failed to remove {}", original.display()))?;
            removal.moved.push((original, temporary));
        }
        Ok(removal)
    }

    pub(super) fn commit(mut self) -> Result<()> {
        self.committed = true;
        for (_, temporary) in &self.moved {
            std::fs::remove_file(temporary).with_context(|| {
                format!(
                    "Profile removed, but cleanup failed for {}",
                    temporary.display()
                )
            })?;
        }
        Ok(())
    }
}

impl Drop for StagedProfileRemoval {
    fn drop(&mut self) {
        if !self.committed {
            for (original, temporary) in self.moved.iter().rev() {
                if let Err(error) = std::fs::rename(temporary, original) {
                    log::error!("Failed to restore profile {}: {error}", original.display());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_selected_files_and_commit_preserves_other_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let selected = dir.path().join("any-qq.config.toml");
        let models = dir.path().join("any-qq.models.json");
        let other = dir.path().join("any-gmail.config.toml");
        for path in [&selected, &models, &other] {
            std::fs::write(path, "original").unwrap();
        }
        {
            let _pending =
                StagedProfileRemoval::stage(vec![selected.clone(), models.clone()]).unwrap();
            assert!(!selected.exists());
            assert!(other.exists());
        }
        assert_eq!(std::fs::read_to_string(&selected).unwrap(), "original");
        StagedProfileRemoval::stage(vec![selected.clone(), models.clone()])
            .unwrap()
            .commit()
            .unwrap();
        assert!(!selected.exists() && !models.exists());
        assert!(other.exists());
    }

    #[test]
    fn a_staging_failure_restores_previously_moved_files() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("any-qq.config.toml");
        std::fs::write(&config, "kept").unwrap();
        assert!(
            StagedProfileRemoval::stage(vec![config.clone(), dir.path().to_path_buf()]).is_err()
        );
        assert_eq!(std::fs::read_to_string(config).unwrap(), "kept");
    }

    #[cfg(unix)]
    #[test]
    fn deleting_a_symlink_does_not_delete_its_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("other.config.toml");
        let alias = dir.path().join("any-qq.config.toml");
        std::fs::write(&target, "kept").unwrap();
        std::os::unix::fs::symlink(&target, &alias).unwrap();
        StagedProfileRemoval::stage(vec![alias.clone()])
            .unwrap()
            .commit()
            .unwrap();
        assert!(target.exists());
        assert!(std::fs::symlink_metadata(alias).is_err());
    }
}
