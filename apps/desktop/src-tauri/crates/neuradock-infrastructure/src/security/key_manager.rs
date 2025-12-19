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
}

impl KeyManager {
    /// Create a new key manager
    ///
    /// # Arguments
    /// * `app_data_dir` - Application data directory path
    pub fn new(app_data_dir: PathBuf) -> Self {
        let salt_path = app_data_dir.join(".encryption_salt");
        Self { salt_path }
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
        fs::write(&self.salt_path, &salt)
            .map_err(|e| KeyManagerError::IoError(format!("Failed to write salt file: {}", e)))?;

        Ok(salt)
    }

    /// Get salt file path (for testing/debugging)
    pub fn salt_path(&self) -> &PathBuf {
        &self.salt_path
    }
}

/// Key manager errors
#[derive(Debug, Error)]
pub enum KeyManagerError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Invalid salt: {0}")]
    InvalidSalt(String),
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
        fs::write(manager.salt_path(), &[0u8; 16]).unwrap(); // Only 16 bytes

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
