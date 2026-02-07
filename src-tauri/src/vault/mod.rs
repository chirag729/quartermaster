//! Encrypted credential vault using Argon2id key derivation and AES-256-GCM.
//!
//! Stores sensitive data (SSH passwords, API keys) encrypted at rest.
//! The vault file lives at `~/.config/quartermaster/vault.enc`.
//!
//! # Security model
//!
//! - Master password → Argon2id → 256-bit key
//! - Each entry encrypted with AES-256-GCM using a unique nonce
//! - Salt stored alongside ciphertext (not secret)
//! - No plaintext credentials ever touch disk

use std::collections::HashMap;
use std::path::PathBuf;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use base64::Engine;

use crate::error::AppError;

/// On-disk representation of the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    /// Argon2id salt (base64-encoded).
    salt: String,
    /// Encrypted entries blob (base64-encoded).
    ciphertext: String,
    /// AES-256-GCM nonce (base64-encoded, 12 bytes).
    nonce: String,
}

/// In-memory decrypted vault contents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultData {
    /// Key-value store of secrets. Keys are identifiers like "ssh:myserver",
    /// values are the encrypted secrets (passwords, tokens, etc.).
    pub entries: HashMap<String, String>,
}

/// Manages encrypted credential storage.
pub struct Vault {
    /// Derived encryption key (held in memory only while vault is unlocked).
    key: Option<[u8; 32]>,
    /// Decrypted vault data (held in memory only while unlocked).
    data: Option<VaultData>,
    /// Path to the vault file.
    path: PathBuf,
}

impl Vault {
    /// Creates a new Vault instance pointing to the default vault file.
    pub fn new() -> Self {
        Self {
            key: None,
            data: None,
            path: Self::vault_path(),
        }
    }

    fn vault_path() -> PathBuf {
        crate::dirs::vault_path()
    }

    /// Returns `true` if the vault file exists on disk.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Returns `true` if the vault is currently unlocked (key in memory).
    pub fn is_unlocked(&self) -> bool {
        self.key.is_some()
    }

    /// Derives a 256-bit key from the master password using Argon2id.
    fn derive_key(password: &str, salt: &SaltString) -> Result<[u8; 32], AppError> {
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), salt)
            .map_err(|e| AppError::Vault(format!("Key derivation failed: {}", e)))?;

        let hash_bytes = hash.hash.ok_or_else(|| {
            AppError::Vault("Argon2 produced no hash output".to_string())
        })?;

        let bytes = hash_bytes.as_bytes();
        if bytes.len() < 32 {
            return Err(AppError::Vault("Derived key too short".to_string()));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[..32]);
        Ok(key)
    }

    /// Creates a new vault with the given master password.
    ///
    /// Overwrites any existing vault file.
    pub fn create(&mut self, master_password: &str) -> Result<(), AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let key = Self::derive_key(master_password, &salt)?;

        self.key = Some(key);
        self.data = Some(VaultData::default());

        self.save(&salt.to_string())?;
        Ok(())
    }

    /// Unlocks an existing vault with the master password.
    ///
    /// Reads the vault file, derives the key, and decrypts the contents.
    pub fn unlock(&mut self, master_password: &str) -> Result<(), AppError> {
        if !self.exists() {
            return Err(AppError::Vault("Vault file does not exist".to_string()));
        }

        let content = std::fs::read_to_string(&self.path)?;
        let vault_file: VaultFile = serde_json::from_str(&content)
            .map_err(|e| AppError::Vault(format!("Corrupt vault file: {}", e)))?;

        let salt = SaltString::from_b64(&vault_file.salt)
            .map_err(|e| AppError::Vault(format!("Invalid salt: {}", e)))?;
        let key = Self::derive_key(master_password, &salt)?;

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(&vault_file.ciphertext)
            .map_err(|e| AppError::Vault(format!("Invalid ciphertext encoding: {}", e)))?;

        let nonce_bytes = base64::engine::general_purpose::STANDARD
            .decode(&vault_file.nonce)
            .map_err(|e| AppError::Vault(format!("Invalid nonce encoding: {}", e)))?;

        if nonce_bytes.len() != 12 {
            return Err(AppError::Vault("Invalid nonce length".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::Vault(format!("Cipher init failed: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| AppError::Vault("Decryption failed — wrong password?".to_string()))?;

        let data: VaultData = serde_json::from_slice(&plaintext)
            .map_err(|e| AppError::Vault(format!("Corrupt vault data: {}", e)))?;

        self.key = Some(key);
        self.data = Some(data);
        Ok(())
    }

    /// Locks the vault, clearing the key and decrypted data from memory.
    pub fn lock(&mut self) {
        // Zero out the key before dropping
        if let Some(ref mut key) = self.key {
            key.fill(0);
        }
        self.key = None;
        self.data = None;
    }

    /// Saves the current vault data to disk, encrypted.
    fn save(&self, salt_str: &str) -> Result<(), AppError> {
        let key = self.key.as_ref().ok_or_else(|| {
            AppError::Vault("Vault is locked".to_string())
        })?;
        let data = self.data.as_ref().ok_or_else(|| {
            AppError::Vault("No vault data".to_string())
        })?;

        let plaintext = serde_json::to_vec(data)
            .map_err(|e| AppError::Vault(format!("Serialization failed: {}", e)))?;

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| AppError::Vault(format!("Cipher init failed: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| AppError::Vault(format!("Encryption failed: {}", e)))?;

        let vault_file = VaultFile {
            salt: salt_str.to_string(),
            ciphertext: base64::engine::general_purpose::STANDARD.encode(&ciphertext),
            nonce: base64::engine::general_purpose::STANDARD.encode(&nonce_bytes),
        };

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&vault_file)
            .map_err(|e| AppError::Vault(format!("Serialization failed: {}", e)))?;
        std::fs::write(&self.path, content)?;

        Ok(())
    }

    /// Saves the vault using the salt from the existing vault file.
    fn save_with_existing_salt(&self) -> Result<(), AppError> {
        let content = std::fs::read_to_string(&self.path)?;
        let vault_file: VaultFile = serde_json::from_str(&content)
            .map_err(|e| AppError::Vault(format!("Corrupt vault file: {}", e)))?;
        self.save(&vault_file.salt)
    }

    /// Retrieves a secret by key. Returns `None` if the key doesn't exist.
    ///
    /// The vault must be unlocked.
    pub fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let data = self.data.as_ref().ok_or_else(|| {
            AppError::Vault("Vault is locked".to_string())
        })?;
        Ok(data.entries.get(key).cloned())
    }

    /// Stores a secret. Overwrites any existing value for the key.
    ///
    /// The vault must be unlocked. Changes are persisted to disk immediately.
    pub fn set(&mut self, key: String, value: String) -> Result<(), AppError> {
        let data = self.data.as_mut().ok_or_else(|| {
            AppError::Vault("Vault is locked".to_string())
        })?;
        data.entries.insert(key, value);
        self.save_with_existing_salt()
    }

    /// Removes a secret by key. Returns `true` if the key existed.
    ///
    /// The vault must be unlocked. Changes are persisted to disk immediately.
    pub fn remove(&mut self, key: &str) -> Result<bool, AppError> {
        let data = self.data.as_mut().ok_or_else(|| {
            AppError::Vault("Vault is locked".to_string())
        })?;
        let existed = data.entries.remove(key).is_some();
        if existed {
            self.save_with_existing_salt()?;
        }
        Ok(existed)
    }

    /// Lists all secret keys (not values) in the vault.
    pub fn list_keys(&self) -> Result<Vec<String>, AppError> {
        let data = self.data.as_ref().ok_or_else(|| {
            AppError::Vault("Vault is locked".to_string())
        })?;
        Ok(data.entries.keys().cloned().collect())
    }

    /// Changes the master password. Re-encrypts the vault with the new key.
    pub fn change_password(
        &mut self,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        // Verify current password by re-unlocking
        if !self.is_unlocked() {
            self.unlock(current_password)?;
        }

        // Derive new key
        let new_salt = SaltString::generate(&mut OsRng);
        let new_key = Self::derive_key(new_password, &new_salt)?;

        self.key = Some(new_key);
        self.save(&new_salt.to_string())
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_vault(dir: &std::path::Path) -> Vault {
        Vault {
            key: None,
            data: None,
            path: dir.join("vault.enc"),
        }
    }

    #[test]
    fn create_and_unlock_vault() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("test-password-123").unwrap();
        assert!(vault.is_unlocked());
        assert!(vault.exists());

        vault.lock();
        assert!(!vault.is_unlocked());

        vault.unlock("test-password-123").unwrap();
        assert!(vault.is_unlocked());
    }

    #[test]
    fn wrong_password_fails() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("correct-password").unwrap();
        vault.lock();

        let result = vault.unlock("wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn store_and_retrieve_secrets() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("password").unwrap();
        vault
            .set("ssh:myserver".to_string(), "s3cret".to_string())
            .unwrap();
        vault
            .set("api:github".to_string(), "ghp_token123".to_string())
            .unwrap();

        assert_eq!(
            vault.get("ssh:myserver").unwrap(),
            Some("s3cret".to_string())
        );
        assert_eq!(
            vault.get("api:github").unwrap(),
            Some("ghp_token123".to_string())
        );
        assert_eq!(vault.get("nonexistent").unwrap(), None);
    }

    #[test]
    fn secrets_persist_across_lock_unlock() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("password").unwrap();
        vault
            .set("key1".to_string(), "value1".to_string())
            .unwrap();

        vault.lock();
        vault.unlock("password").unwrap();

        assert_eq!(vault.get("key1").unwrap(), Some("value1".to_string()));
    }

    #[test]
    fn remove_secret() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("password").unwrap();
        vault.set("key".to_string(), "value".to_string()).unwrap();

        assert!(vault.remove("key").unwrap());
        assert!(!vault.remove("key").unwrap()); // already removed
        assert_eq!(vault.get("key").unwrap(), None);
    }

    #[test]
    fn list_keys() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("password").unwrap();
        vault.set("a".to_string(), "1".to_string()).unwrap();
        vault.set("b".to_string(), "2".to_string()).unwrap();
        vault.set("c".to_string(), "3".to_string()).unwrap();

        let mut keys = vault.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn operations_on_locked_vault_fail() {
        let dir = tempdir().unwrap();
        let vault = test_vault(dir.path());

        assert!(vault.get("key").is_err());
        assert!(vault.list_keys().is_err());
    }

    #[test]
    fn change_password() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("old-password").unwrap();
        vault.set("key".to_string(), "value".to_string()).unwrap();

        vault
            .change_password("old-password", "new-password")
            .unwrap();
        vault.lock();

        // Old password should fail
        let result = vault.unlock("old-password");
        assert!(result.is_err());

        // New password should work
        vault.unlock("new-password").unwrap();
        assert_eq!(vault.get("key").unwrap(), Some("value".to_string()));
    }

    #[test]
    fn unlock_nonexistent_vault_fails() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());
        let result = vault.unlock("password");
        assert!(result.is_err());
    }

    #[test]
    fn overwrite_existing_secret() {
        let dir = tempdir().unwrap();
        let mut vault = test_vault(dir.path());

        vault.create("password").unwrap();
        vault
            .set("key".to_string(), "original".to_string())
            .unwrap();
        vault
            .set("key".to_string(), "updated".to_string())
            .unwrap();

        assert_eq!(vault.get("key").unwrap(), Some("updated".to_string()));
    }
}
