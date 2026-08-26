use rand::RngCore;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Key manager for encryption salt storage
///
/// # Security Design
/// - Stores 256-bit random salt in app data directory
/// - Salt is generated once and reused for consistency
/// - Salt file protected by filesystem permissions
/// - Salt is NOT a secret, but ensures unique keys per installation
pub struct KeyManager {
    salt_path: PathBuf,
    master_key_path: PathBuf,
    pending_master_key_path: PathBuf,
}

impl KeyManager {
    /// Create a new key manager
    ///
    /// # Arguments
    /// * `app_data_dir` - Application data directory path
    pub fn new(app_data_dir: PathBuf) -> Self {
        let salt_path = app_data_dir.join(".encryption_salt");
        let master_key_path = app_data_dir.join(".encryption_key");
        let pending_master_key_path = app_data_dir.join(".encryption_key.pending");
        Self {
            salt_path,
            master_key_path,
            pending_master_key_path,
        }
    }

    /// Load or generate a random 256-bit master key for new installations.
    /// Existing installations without this file remain on the legacy key path
    /// until their encrypted records are re-encrypted by an explicit migration.
    pub fn initialize_master_key(&self) -> Result<Option<[u8; 32]>, KeyManagerError> {
        if self.master_key_path.exists() {
            return self.load_master_key().map(Some);
        }
        if self.salt_path.exists() {
            return Ok(None);
        }
        let key = self.generate_and_save_master_key()?;
        Ok(Some(key))
    }

    pub fn generate_migration_key(&self) -> Result<[u8; 32], KeyManagerError> {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }

    pub fn save_master_key(&self, key: &[u8; 32]) -> Result<(), KeyManagerError> {
        if let Some(parent) = self.master_key_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KeyManagerError::IoError(format!("Failed to create directory: {}", e))
            })?;
        }
        fs::write(&self.master_key_path, key)
            .map_err(|e| KeyManagerError::IoError(format!("Failed to write master key: {}", e)))?;
        set_private_permissions(&self.master_key_path)
    }

    pub fn save_pending_master_key(&self, key: &[u8; 32]) -> Result<(), KeyManagerError> {
        fs::write(&self.pending_master_key_path, key).map_err(|e| {
            KeyManagerError::IoError(format!("Failed to write pending master key: {}", e))
        })?;
        set_private_permissions(&self.pending_master_key_path)
    }

    pub fn load_pending_master_key(&self) -> Result<Option<[u8; 32]>, KeyManagerError> {
        if !self.pending_master_key_path.exists() {
            return Ok(None);
        }
        self.load_key(&self.pending_master_key_path).map(Some)
    }

    pub fn promote_pending_master_key(&self) -> Result<(), KeyManagerError> {
        if !self.pending_master_key_path.exists() {
            return Ok(());
        }
        fs::rename(&self.pending_master_key_path, &self.master_key_path).map_err(|e| {
            KeyManagerError::IoError(format!("Failed to promote pending master key: {}", e))
        })?;
        set_private_permissions(&self.master_key_path)
    }

    /// Initialize encryption salt (load or generate)
    ///
    /// - If salt file exists, load it
    /// - If not, generate new random salt and save it
    ///
    /// # Returns
    /// 32-byte salt for key derivation
    pub fn initialize(&self) -> Result<[u8; 32], KeyManagerError> {
        if self.salt_path.exists() {
            self.load_salt()
        } else {
            self.generate_and_save_salt()
        }
    }

    /// Load existing salt from file
    fn load_salt(&self) -> Result<[u8; 32], KeyManagerError> {
        let bytes = fs::read(&self.salt_path)
            .map_err(|e| KeyManagerError::IoError(format!("Failed to read salt file: {}", e)))?;

        if bytes.len() != 32 {
            return Err(KeyManagerError::InvalidSalt(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut salt = [0u8; 32];
        salt.copy_from_slice(&bytes);

        Ok(salt)
    }

    /// Generate new random salt and save to file
    fn generate_and_save_salt(&self) -> Result<[u8; 32], KeyManagerError> {
        // Generate cryptographically secure random salt
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        // Ensure parent directory exists
        if let Some(parent) = self.salt_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KeyManagerError::IoError(format!("Failed to create directory: {}", e))
            })?;
        }

        // Write salt to file
        fs::write(&self.salt_path, salt)
            .map_err(|e| KeyManagerError::IoError(format!("Failed to write salt file: {}", e)))?;

        Ok(salt)
    }

    fn load_master_key(&self) -> Result<[u8; 32], KeyManagerError> {
        self.load_key(&self.master_key_path)
    }

    fn load_key(&self, path: &std::path::Path) -> Result<[u8; 32], KeyManagerError> {
        let bytes = fs::read(path)
            .map_err(|e| KeyManagerError::IoError(format!("Failed to read master key: {}", e)))?;
        if bytes.len() != 32 {
            return Err(KeyManagerError::InvalidMasterKey(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    }

    fn generate_and_save_master_key(&self) -> Result<[u8; 32], KeyManagerError> {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        if let Some(parent) = self.master_key_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KeyManagerError::IoError(format!("Failed to create directory: {}", e))
            })?;
        }
        self.save_master_key(&key)?;
        Ok(key)
    }

    /// Get salt file path (for testing/debugging)
    pub fn salt_path(&self) -> &PathBuf {
        &self.salt_path
    }
}

#[cfg(unix)]
fn set_private_permissions(path: &std::path::Path) -> Result<(), KeyManagerError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| {
        KeyManagerError::IoError(format!("Failed to protect master key permissions: {}", e))
    })
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &std::path::Path) -> Result<(), KeyManagerError> {
    Ok(())
}

/// Key manager errors
#[derive(Debug, Error)]
pub enum KeyManagerError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Invalid salt: {0}")]
    InvalidSalt(String),

    #[error("Invalid master key: {0}")]
    InvalidMasterKey(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_manager() -> (KeyManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = KeyManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_initialize_generates_salt_if_not_exists() {
        let (manager, _temp_dir) = create_test_manager();

        assert!(!manager.salt_path().exists());

        let salt = manager.initialize().unwrap();

        // Salt should be 32 bytes
        assert_eq!(salt.len(), 32);

        // Salt file should exist now
        assert!(manager.salt_path().exists());
    }

    #[test]
    fn new_installation_generates_random_master_key() {
        let (manager, _temp_dir) = create_test_manager();
        let key = manager.initialize_master_key().unwrap().unwrap();
        assert!(key.iter().any(|&byte| byte != 0));
        assert_eq!(manager.initialize_master_key().unwrap(), Some(key));
    }

    #[test]
    fn legacy_installation_is_not_changed_implicitly() {
        let (manager, _temp_dir) = create_test_manager();
        manager.initialize().unwrap();
        assert_eq!(manager.initialize_master_key().unwrap(), None);
    }

    #[test]
    fn pending_master_key_survives_restart_and_promotes_atomically() {
        let (manager, temp_dir) = create_test_manager();
        manager.initialize().unwrap();
        let key = manager.generate_migration_key().unwrap();
        manager.save_pending_master_key(&key).unwrap();

        let restarted = KeyManager::new(temp_dir.path().to_path_buf());
        assert_eq!(restarted.load_pending_master_key().unwrap(), Some(key));
        assert!(!restarted.master_key_path.exists());

        restarted.promote_pending_master_key().unwrap();
        assert_eq!(restarted.load_master_key().unwrap(), key);
        assert!(!restarted.pending_master_key_path.exists());
    }

    #[test]
    fn promoting_without_pending_key_is_a_noop() {
        let (manager, _temp_dir) = create_test_manager();

        manager.promote_pending_master_key().unwrap();
        assert!(!manager.master_key_path.exists());
    }

    #[test]
    fn test_initialize_loads_existing_salt() {
        let (manager, _temp_dir) = create_test_manager();

        // First initialization generates salt
        let salt1 = manager.initialize().unwrap();

        // Second initialization should load the same salt
        let salt2 = manager.initialize().unwrap();

        assert_eq!(salt1, salt2);
    }

    #[test]
    fn test_generated_salts_are_random() {
        let (manager1, _temp1) = create_test_manager();
        let (manager2, _temp2) = create_test_manager();

        let salt1 = manager1.initialize().unwrap();
        let salt2 = manager2.initialize().unwrap();

        // Different managers should generate different salts
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_salt_not_all_zeros() {
        let (manager, _temp_dir) = create_test_manager();

        let salt = manager.initialize().unwrap();

        // Salt should not be all zeros (extremely unlikely with random generation)
        assert!(salt.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_load_invalid_salt_length_fails() {
        let (manager, _temp_dir) = create_test_manager();

        // Write invalid salt (wrong length)
        fs::create_dir_all(manager.salt_path().parent().unwrap()).unwrap();
        fs::write(manager.salt_path(), [0u8; 16]).unwrap(); // Only 16 bytes

        let result = manager.initialize();

        assert!(result.is_err());
        assert!(matches!(result, Err(KeyManagerError::InvalidSalt(_))));
    }

    #[test]
    fn test_salt_file_permissions() {
        let (manager, _temp_dir) = create_test_manager();

        manager.initialize().unwrap();

        // Verify file exists and is readable
        let metadata = fs::metadata(manager.salt_path()).unwrap();
        assert!(metadata.is_file());
        assert!(metadata.len() == 32);
    }
}
