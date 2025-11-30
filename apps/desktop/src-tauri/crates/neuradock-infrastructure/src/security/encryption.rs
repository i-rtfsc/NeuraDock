use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use std::fmt;

/// Encryption service using AES-256-GCM
/// 
/// # Security Design
/// - Uses AES-256-GCM for authenticated encryption
/// - Key derived from master password using Argon2id
/// - Unique nonce for each encryption
/// - Authenticated encryption prevents tampering
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create encryption service from a master password and salt
    /// 
    /// Uses Argon2id for key derivation which is:
    /// - Memory-hard (resistant to GPU attacks)
    /// - Recommended by OWASP
    /// - Winner of the Password Hashing Competition
    pub fn from_password(password: &str, salt: &[u8; 32]) -> Result<Self, EncryptionError> {
        // Derive 32-byte key using Argon2id
        let mut key = [0u8; 32];
        
        let argon2 = Argon2::default();
        
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
        
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;
        
        Ok(Self { cipher })
    }
    
    /// Encrypt plaintext
    /// 
    /// Returns base64-encoded string containing: nonce (12 bytes) + ciphertext
    /// 
    /// # Security
    /// - Uses random nonce for each encryption (never reuse nonces!)
    /// - Provides authenticated encryption (detects tampering)
    pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
        // Generate random 96-bit nonce (12 bytes)
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt with authentication
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
        
        // Concatenate: nonce || ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        // Encode as base64
        Ok(general_purpose::STANDARD.encode(&result))
    }
    
    /// Decrypt ciphertext
    /// 
    /// Expects base64-encoded string containing: nonce (12 bytes) + ciphertext
    /// 
    /// # Security
    /// - Verifies authentication tag (prevents tampering)
    /// - Returns error if data has been modified
    pub fn decrypt(&self, encrypted: &str) -> Result<String, EncryptionError> {
        // Decode from base64
        let data = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| EncryptionError::InvalidFormat(format!("Base64 decode failed: {}", e)))?;
        
        // Check minimum length (12-byte nonce + 16-byte tag)
        if data.len() < 28 {
            return Err(EncryptionError::InvalidFormat(
                "Data too short (expected at least nonce + tag)".to_string(),
            ));
        }
        
        // Split: nonce || ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt and verify authentication
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Decryption failed (data may be tampered): {}", e)))?;
        
        // Convert to UTF-8 string
        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::InvalidUtf8(e.to_string()))
    }
}

/// Encryption errors
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(String),
}

impl fmt::Display for EncryptionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EncryptionService(AES-256-GCM)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> EncryptionService {
        let salt = [42u8; 32];
        EncryptionService::from_password("test_password_123", &salt).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let service = create_test_service();
        let plaintext = "Hello, World! 你好世界 🌍";
        
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let service = create_test_service();
        let plaintext = "Same plaintext";
        
        let encrypted1 = service.encrypt(plaintext).unwrap();
        let encrypted2 = service.encrypt(plaintext).unwrap();
        
        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to the same plaintext
        assert_eq!(service.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(service.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_invalid_base64_fails() {
        let service = create_test_service();
        let result = service.decrypt("not valid base64!!!");
        
        assert!(result.is_err());
        assert!(matches!(result, Err(EncryptionError::InvalidFormat(_))));
    }

    #[test]
    fn test_decrypt_too_short_data_fails() {
        let service = create_test_service();
        let short_data = general_purpose::STANDARD.encode(&[0u8; 10]);
        let result = service.decrypt(&short_data);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(EncryptionError::InvalidFormat(_))));
    }

    #[test]
    fn test_decrypt_tampered_data_fails() {
        let service = create_test_service();
        let plaintext = "Original message";
        
        let mut encrypted = service.encrypt(plaintext).unwrap();
        
        // Tamper with the encrypted data (flip a bit)
        let mut bytes = general_purpose::STANDARD.decode(&encrypted).unwrap();
        bytes[20] ^= 0x01; // Flip one bit
        encrypted = general_purpose::STANDARD.encode(&bytes);
        
        let result = service.decrypt(&encrypted);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(EncryptionError::DecryptionFailed(_))));
    }

    #[test]
    fn test_different_passwords_produce_different_keys() {
        let salt = [42u8; 32];
        let service1 = EncryptionService::from_password("password1", &salt).unwrap();
        let service2 = EncryptionService::from_password("password2", &salt).unwrap();
        
        let plaintext = "Test message";
        let encrypted = service1.encrypt(plaintext).unwrap();
        
        // Service with different password should not be able to decrypt
        let result = service2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_empty_string() {
        let service = create_test_service();
        let plaintext = "";
        
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_large_text() {
        let service = create_test_service();
        let plaintext = "x".repeat(10000); // 10KB of data
        
        let encrypted = service.encrypt(&plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_unicode_characters() {
        let service = create_test_service();
        let plaintext = "Hello 世界 🌍 Здравствуй مرحبا";
        
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_json_data() {
        let service = create_test_service();
        let json_data = r#"{"username":"test","password":"secret123","token":"abc123xyz"}"#;
        
        let encrypted = service.encrypt(json_data).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(json_data, decrypted);
    }
}
