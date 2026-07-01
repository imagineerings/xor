//! OAuth token persistence layer.
//!
//! Provides the [`TokenStore`] trait and two implementations:
//! - [`KeyringTokenStore`] — stores tokens in the OS keychain via the `keyring` crate
//! - [`EncryptedFileTokenStore`] — AES-GCM encrypted file storage, used as a fallback
//!   in headless environments where the OS keychain is unavailable.

use aes_gcm::{
    KeyInit,
    aead::rand_core::RngCore,
    aead::{Aead, OsRng},
};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

// ── OAuthTokens ────────────────────────────────────────────────────────────

/// Represents a set of OAuth 2.0 tokens returned from the token endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Lifetime of the access token, measured from `obtained_at`.
    pub expires_in: Option<Duration>,
    pub token_type: String,
    pub scope: Option<String>,
    /// When these tokens were obtained (or last refreshed).
    /// Used by [`TokenManager`](crate::TokenManager) to detect expiry.
    pub obtained_at: Option<DateTime<Utc>>,
}

// ── TokenStore trait ───────────────────────────────────────────────────────

/// A store for persisting OAuth tokens.
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait TokenStore: Send + Sync {
    /// Persist tokens under the given key (e.g. `"anthropic"` or `"user_42"`).
    fn store(&self, key: &str, tokens: &OAuthTokens) -> Result<()>;

    /// Load previously stored tokens for the given key.
    fn load(&self, key: &str) -> Result<Option<OAuthTokens>>;

    /// Delete stored tokens for the given key.
    fn delete(&self, key: &str) -> Result<()>;
}

// ── KeyringTokenStore ──────────────────────────────────────────────────────

/// Token store backed by the OS keyring.
///
/// Uses the `keyring` crate which supports:
/// - macOS: Keychain
/// - Linux: Secret Service (DBus) / libsecret
/// - Windows: Windows Credential Manager
pub struct KeyringTokenStore {
    service_name: String,
}

impl KeyringTokenStore {
    /// Create a new store with the given service name.
    ///
    /// The service name identifies the application in the keyring (e.g.
    /// `"baymax-oauth"`).
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }
}

impl TokenStore for KeyringTokenStore {
    fn store(&self, key: &str, tokens: &OAuthTokens) -> Result<()> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| anyhow!("failed to create keyring entry: {e}"))?;
        let json = serde_json::to_string(tokens).context("failed to serialize tokens")?;
        entry
            .set_password(&json)
            .map_err(|e| anyhow!("failed to store token in keyring: {e}"))?;
        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<OAuthTokens>> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| anyhow!("failed to create keyring entry: {e}"))?;
        match entry.get_password() {
            Ok(json) => {
                let tokens = serde_json::from_str(&json).context("failed to deserialize tokens")?;
                Ok(Some(tokens))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("failed to read token from keyring: {e}")),
        }
    }

    fn delete(&self, key: &str) -> Result<()> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| anyhow!("failed to create keyring entry: {e}"))?;
        entry
            .delete_credential()
            .map_err(|e| anyhow!("failed to delete token from keyring: {e}"))?;
        Ok(())
    }
}

// ── EncryptedFileTokenStore ────────────────────────────────────────────────

/// Token store backed by an AES-256-GCM encrypted file.
///
/// Used as a fallback when the OS keyring is unavailable (e.g. headless CI,
/// SSH sessions, or containers).
pub struct EncryptedFileTokenStore {
    file_path: PathBuf,
    key: [u8; 32],
}

impl EncryptedFileTokenStore {
    /// Create a new store backed by `file_path`.
    ///
    /// The encryption key is derived from `raw_key` (SHA-256 hashed to get 32
    /// bytes). If `raw_key` is empty, a random key is generated and returned
    /// alongside the store so callers can persist it.
    pub fn new(file_path: impl Into<PathBuf>, raw_key: &[u8]) -> (Self, Vec<u8>) {
        let file_path = file_path.into();
        let key = if raw_key.is_empty() {
            let mut k = [0u8; 32];
            aes_gcm::aead::OsRng.fill_bytes(&mut k);
            k
        } else {
            let mut k = [0u8; 32];
            let len = raw_key.len().min(32);
            k[..len].copy_from_slice(&raw_key[..len]);
            k
        };
        let store = Self { file_path, key };
        (store, key.to_vec())
    }

    /// The path to the encrypted token file.
    pub fn path(&self) -> &Path {
        &self.file_path
    }
}

impl TokenStore for EncryptedFileTokenStore {
    fn store(&self, key: &str, tokens: &OAuthTokens) -> Result<()> {
        // Serialize the entry: a map of key -> token JSON.
        let mut store: std::collections::HashMap<String, OAuthTokens> = if self.file_path.exists() {
            // Read existing store
            let encrypted = std::fs::read(&self.file_path).context("failed to read token file")?;
            if encrypted.is_empty() {
                std::collections::HashMap::new()
            } else {
                decrypt_map(&encrypted, &self.key).context("failed to decrypt token file")?
            }
        } else {
            std::collections::HashMap::new()
        };

        store.insert(key.to_string(), tokens.clone());

        // Serialize and encrypt
        let plaintext = serde_json::to_vec(&store).context("failed to serialize token store")?;
        let encrypted =
            encrypt_blob(&plaintext, &self.key).context("failed to encrypt token store")?;

        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).context("failed to create token store directory")?;
        }
        std::fs::write(&self.file_path, &encrypted).context("failed to write token file")?;

        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<OAuthTokens>> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let encrypted = std::fs::read(&self.file_path).context("failed to read token file")?;
        if encrypted.is_empty() {
            return Ok(None);
        }

        let store: std::collections::HashMap<String, OAuthTokens> =
            decrypt_map(&encrypted, &self.key).context("failed to decrypt token file")?;

        Ok(store.get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<()> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let encrypted = std::fs::read(&self.file_path).context("failed to read token file")?;
        let mut store: std::collections::HashMap<String, OAuthTokens> = if encrypted.is_empty() {
            std::collections::HashMap::new()
        } else {
            decrypt_map(&encrypted, &self.key).context("failed to decrypt token file")?
        };

        store.remove(key);

        let plaintext = serde_json::to_vec(&store).context("failed to serialize token store")?;
        let encrypted =
            encrypt_blob(&plaintext, &self.key).context("failed to encrypt token store")?;

        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).context("failed to create token store directory")?;
        }
        std::fs::write(&self.file_path, &encrypted).context("failed to write token file")?;

        Ok(())
    }
}

// ── Encryption helpers ─────────────────────────────────────────────────────

type Aes256Gcm = aes_gcm::Aes256Gcm;
type Nonce = aes_gcm::Nonce<aes_gcm::aead::generic_array::typenum::U12>;

/// Encrypt a plaintext blob with AES-256-GCM.
fn encrypt_blob(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    use aes_gcm::aead::AeadCore;

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("invalid AES key: {e}"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("encryption failed: {e}"))?;

    // Output format: nonce (12 bytes) || ciphertext || tag (16 bytes)
    let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a blob encrypted with `encrypt_blob`.
fn decrypt_blob(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("invalid AES key: {e}"))?;

    let nonce_size = std::mem::size_of::<Nonce>();
    if data.len() < nonce_size {
        return Err(anyhow!("encrypted data too short"));
    }

    let (nonce_bytes, ciphertext) = data.split_at(nonce_size);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("decryption failed: {e}"))?;

    Ok(plaintext)
}

/// Decrypt the token map from an encrypted blob.
fn decrypt_map(
    encrypted: &[u8],
    key: &[u8; 32],
) -> Result<std::collections::HashMap<String, OAuthTokens>> {
    let plaintext = decrypt_blob(encrypted, key)?;
    serde_json::from_slice(&plaintext).context("failed to deserialize token map")
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tokens() -> OAuthTokens {
        OAuthTokens {
            access_token: "test-access-token".to_string(),
            refresh_token: Some("test-refresh-token".to_string()),
            expires_in: Some(Duration::from_secs(3600)),
            token_type: "Bearer".to_string(),
            scope: Some("read write".to_string()),
            obtained_at: None,
        }
    }

    /// Helper: returns `true` if the OS keyring supports full CRUD.
    fn keyring_available() -> bool {
        let store = KeyringTokenStore::new("baymax-test-probe");
        let key = "__probe__";
        let tokens = sample_tokens();
        // Clean up any stale entry first.
        let _ = store.delete(key);
        if store.store(key, &tokens).is_err() {
            return false;
        }
        let loaded = store.load(key).ok().flatten();
        let _ = store.delete(key);
        loaded.is_some()
    }

    #[test]
    fn test_keyring_store() {
        if !keyring_available() {
            eprintln!("skipping keyring test: OS keychain not available in this environment");
            return;
        }
        let store = KeyringTokenStore::new("baymax-test");
        let key = "test_keyring_key";
        let tokens = sample_tokens();

        // Clean up any stale entry from a previous run.
        let _ = store.delete(key);

        store.store(key, &tokens).unwrap();
        let loaded = store.load(key).unwrap().unwrap();
        assert_eq!(loaded.access_token, tokens.access_token);
        assert_eq!(loaded.refresh_token, tokens.refresh_token);

        store.delete(key).unwrap();
        assert!(store.load(key).unwrap().is_none());
    }

    #[test]
    fn test_encrypted_file_store() {
        let dir =
            std::env::temp_dir().join(format!("baymax-test-tokens-{}", rand::random::<u64>()));
        let file_path = dir.join("tokens.json.enc");

        let (store, key) = EncryptedFileTokenStore::new(&file_path, b"test-key-12345");
        let store2 = EncryptedFileTokenStore::new(&file_path, &key).0;

        let key1 = "provider:anthropic";
        let tokens = sample_tokens();

        store.store(key1, &tokens).unwrap();
        let loaded = store2.load(key1).unwrap().unwrap();
        assert_eq!(loaded.access_token, tokens.access_token);

        // Delete and verify gone
        store.delete(key1).unwrap();
        assert!(store2.load(key1).unwrap().is_none());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_encrypted_file_store_multiple_keys() {
        let dir =
            std::env::temp_dir().join(format!("baymax-test-tokens-{}", rand::random::<u64>()));
        let file_path = dir.join("tokens.json.enc");

        let (store, key) = EncryptedFileTokenStore::new(&file_path, b"multi-key-test");

        store.store("user1", &sample_tokens()).unwrap();
        store.store("user2", &sample_tokens()).unwrap();

        let store2 = EncryptedFileTokenStore::new(&file_path, &key).0;
        assert!(store2.load("user1").unwrap().is_some());
        assert!(store2.load("user2").unwrap().is_some());

        store2.delete("user1").unwrap();
        assert!(store2.load("user1").unwrap().is_none());
        assert!(store2.load("user2").unwrap().is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut key = [0u8; 32];
        aes_gcm::aead::OsRng.fill_bytes(&mut key);

        let plaintext = b"hello, world! this is a secret message.";
        let encrypted = encrypt_blob(plaintext, &key).unwrap();
        let decrypted = decrypt_blob(&encrypted, &key).unwrap();

        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let mut key = [0u8; 32];
        let mut wrong_key = [0u8; 32];
        aes_gcm::aead::OsRng.fill_bytes(&mut key);
        aes_gcm::aead::OsRng.fill_bytes(&mut wrong_key);

        let plaintext = b"secret data";
        let encrypted = encrypt_blob(plaintext, &key).unwrap();
        let result = decrypt_blob(&encrypted, &wrong_key);
        assert!(result.is_err());
    }
}
