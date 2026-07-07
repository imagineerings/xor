use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Result;
use serde_json::Value;

/// Manages cached data for sim apps with optional TTL support.
///
/// Each app gets its own cache namespace identified by its app id.
pub struct CacheManager {
    caches: HashMap<String, HashMap<String, CacheEntry>>,
}

struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            caches: HashMap::new(),
        }
    }

    /// Set a cache entry with an optional TTL.
    pub fn set(&mut self, app_id: &str, key: &str, value: Value, ttl: Option<Duration>) {
        let expires_at = ttl.map(|d| Instant::now() + d);
        self.caches
            .entry(app_id.to_string())
            .or_default()
            .insert(key.to_string(), CacheEntry { value, expires_at });
    }

    /// Get a cache entry. Returns `None` if missing or expired.
    pub fn get(&mut self, app_id: &str, key: &str) -> Option<&Value> {
        let app_cache = self.caches.get_mut(app_id)?;

        let is_expired = app_cache.get(key).map_or(false, |entry| {
            entry
                .expires_at
                .map_or(false, |expires_at| Instant::now() > expires_at)
        });

        if is_expired {
            app_cache.remove(key);
            return None;
        }

        app_cache.get(key).map(|entry| &entry.value)
    }

    /// Get a cache entry deserialized to a specific type.
    pub fn get_as<T: serde::de::DeserializeOwned>(
        &mut self,
        app_id: &str,
        key: &str,
    ) -> Result<Option<T>> {
        match self.get(app_id, key) {
            Some(value) => Ok(Some(serde_json::from_value(value.clone())?)),
            None => Ok(None),
        }
    }

    /// Remove a cache entry.
    pub fn remove(&mut self, app_id: &str, key: &str) {
        if let Some(app_cache) = self.caches.get_mut(app_id) {
            app_cache.remove(key);
        }
    }

    /// Clear all cache entries for a given app.
    pub fn clear_app(&mut self, app_id: &str) {
        self.caches.remove(app_id);
    }

    /// Check if a cache entry exists and is not expired.
    pub fn has(&mut self, app_id: &str, key: &str) -> bool {
        self.get(app_id, key).is_some()
    }

    /// Remove all expired entries across all apps.
    pub fn evict_expired(&mut self) {
        let now = Instant::now();
        self.caches.retain(|_, entries| {
            entries.retain(|_, entry| {
                entry
                    .expires_at
                    .map_or(true, |expires_at| now <= expires_at)
            });
            !entries.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_set_and_get() {
        let mut cm = CacheManager::new();
        cm.set("chat", "key", json!("value"), None);
        assert_eq!(cm.get("chat", "key"), Some(&json!("value")));
    }

    #[test]
    fn test_get_missing() {
        let mut cm = CacheManager::new();
        assert_eq!(cm.get("chat", "missing"), None);
    }

    #[test]
    fn test_ttl_expiry() {
        let mut cm = CacheManager::new();
        cm.set(
            "chat",
            "temp",
            json!("expires_soon"),
            Some(Duration::from_millis(1)),
        );
        std::thread::sleep(Duration::from_millis(2));
        assert_eq!(cm.get("chat", "temp"), None);
    }

    #[test]
    fn test_remove() {
        let mut cm = CacheManager::new();
        cm.set("chat", "key", json!("value"), None);
        cm.remove("chat", "key");
        assert_eq!(cm.get("chat", "key"), None);
    }

    #[test]
    fn test_clear_app() {
        let mut cm = CacheManager::new();
        cm.set("app1", "a", json!(1), None);
        cm.set("app2", "b", json!(2), None);
        cm.clear_app("app1");
        assert!(cm.get("app1", "a").is_none());
        assert!(cm.get("app2", "b").is_some());
    }

    #[test]
    fn test_evict_expired() {
        let mut cm = CacheManager::new();
        cm.set(
            "chat",
            "ephemeral",
            json!("gone"),
            Some(Duration::from_millis(1)),
        );
        cm.set("chat", "persistent", json!("stays"), None);
        std::thread::sleep(Duration::from_millis(2));
        cm.evict_expired();
        assert!(cm.get("chat", "ephemeral").is_none());
        assert!(cm.get("chat", "persistent").is_some());
    }
}
