use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A simple pairing service that maps external platform user IDs
/// to internal sim user identities.
///
/// Pairings can be persisted to a JSON file for durability across
/// application restarts.
pub struct PairingService {
    /// The underlying store: maps `platform_id` → `sim_user`.
    store: HashMap<String, String>,
    /// Optional path to a JSON file used for persistence.
    storage_path: Option<PathBuf>,
}

/// Serializable representation of the pairings for file I/O.
#[derive(Serialize, Deserialize)]
struct PairingData {
    pairings: HashMap<String, String>,
}

impl PairingService {
    /// Create a new, empty pairing service with no persistence.
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            storage_path: None,
        }
    }

    /// Create a new pairing service backed by the given JSON file.
    ///
    /// Existing pairings are loaded from the file if it exists.
    /// All mutations are persisted automatically.
    pub fn with_storage(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let store = if path.exists() {
            Self::load_from_file(&path).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            store,
            storage_path: Some(path),
        }
    }

    /// Link an external platform user to a sim user identity.
    ///
    /// Returns an error if the pairing already exists (use
    /// [`unlink`](Self::unlink) first to change a pairing).
    pub fn pair_platform_user(&mut self, platform_id: &str, sim_user: &str) -> Result<()> {
        if self.store.contains_key(platform_id) {
            anyhow::bail!(
                "platform user '{platform_id}' is already paired to '{}'",
                self.store[platform_id]
            );
        }
        self.store
            .insert(platform_id.to_string(), sim_user.to_string());
        self.persist()
    }

    /// Look up the sim user identity associated with a platform user.
    pub fn lookup_sim_user(&self, platform_id: &str) -> Option<&str> {
        self.store.get(platform_id).map(|s| s.as_str())
    }

    /// Remove an existing pairing for the given platform user.
    pub fn unlink(&mut self, platform_id: &str) -> Result<()> {
        self.store.remove(platform_id);
        self.persist()
    }

    /// Returns `true` if the platform user is currently paired.
    pub fn is_paired(&self, platform_id: &str) -> bool {
        self.store.contains_key(platform_id)
    }

    /// Returns the number of active pairings.
    pub fn count(&self) -> usize {
        self.store.len()
    }

    /// Iterate over all pairings (platform_id, sim_user).
    pub fn store(&self) -> impl Iterator<Item = (&str, &str)> {
        self.store.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    // ------------------------------------------------------------------
    // Persistence helpers
    // ------------------------------------------------------------------

    fn persist(&self) -> Result<()> {
        if let Some(path) = &self.storage_path {
            let data = PairingData {
                pairings: self.store.clone(),
            };
            let json = serde_json::to_string_pretty(&data)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, json)?;
        }
        Ok(())
    }

    fn load_from_file(path: &Path) -> Result<HashMap<String, String>> {
        let json = std::fs::read_to_string(path)?;
        let data: PairingData = serde_json::from_str(&json)?;
        Ok(data.pairings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pair_and_lookup() {
        let mut service = PairingService::new();
        assert!(service.pair_platform_user("tg:12345", "alice").is_ok());
        assert_eq!(service.lookup_sim_user("tg:12345"), Some("alice"));
    }

    #[test]
    fn test_duplicate_pairing_fails() {
        let mut service = PairingService::new();
        service.pair_platform_user("tg:12345", "alice").unwrap();
        let err = service.pair_platform_user("tg:12345", "bob");
        assert!(err.is_err());
        // Original pairing is preserved
        assert_eq!(service.lookup_sim_user("tg:12345"), Some("alice"));
    }

    #[test]
    fn test_unlink() {
        let mut service = PairingService::new();
        service.pair_platform_user("tg:12345", "alice").unwrap();
        assert!(service.is_paired("tg:12345"));
        service.unlink("tg:12345").unwrap();
        assert!(!service.is_paired("tg:12345"));
    }

    #[test]
    fn test_lookup_nonexistent() {
        let service = PairingService::new();
        assert_eq!(service.lookup_sim_user("tg:99999"), None);
    }

    #[test]
    fn test_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pairings.json");

        // Create and add a pairing
        {
            let mut service = PairingService::with_storage(&path);
            service.pair_platform_user("tg:12345", "alice").unwrap();
            assert_eq!(service.count(), 1);
        }

        // Load from the same file and verify
        {
            let service = PairingService::with_storage(&path);
            assert_eq!(service.lookup_sim_user("tg:12345"), Some("alice"));
            assert_eq!(service.count(), 1);
        }
    }

    #[test]
    fn test_empty_persistence_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pairings.json");
        let service = PairingService::with_storage(&path);
        assert_eq!(service.count(), 0);
    }
}
