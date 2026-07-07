use std::collections::HashMap;

use anyhow::Result;
use serde_json::Value;

/// Manages resources (persistent data) for sim apps.
///
/// Each app gets its own key-value namespace identified by its app id.
pub struct ResourceManager {
    resources: HashMap<String, HashMap<String, Value>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    /// Set a resource value for a given app.
    pub fn set(&mut self, app_id: &str, key: &str, value: Value) {
        self.resources
            .entry(app_id.to_string())
            .or_default()
            .insert(key.to_string(), value);
    }

    /// Get a resource value for a given app.
    pub fn get(&self, app_id: &str, key: &str) -> Option<&Value> {
        self.resources.get(app_id)?.get(key)
    }

    /// Get a resource value deserialized to a specific type.
    pub fn get_as<T: serde::de::DeserializeOwned>(
        &self,
        app_id: &str,
        key: &str,
    ) -> Result<Option<T>> {
        match self.get(app_id, key) {
            Some(value) => Ok(Some(serde_json::from_value(value.clone())?)),
            None => Ok(None),
        }
    }

    /// Remove a resource for a given app.
    pub fn remove(&mut self, app_id: &str, key: &str) {
        if let Some(app_resources) = self.resources.get_mut(app_id) {
            app_resources.remove(key);
        }
    }

    /// Clear all resources for a given app.
    pub fn clear_app(&mut self, app_id: &str) {
        self.resources.remove(app_id);
    }

    /// Check if a resource exists for a given app.
    pub fn has(&self, app_id: &str, key: &str) -> bool {
        self.resources
            .get(app_id)
            .is_some_and(|r| r.contains_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_set_and_get() {
        let mut rm = ResourceManager::new();
        rm.set("chat", "greeting", json!("hello"));
        assert_eq!(rm.get("chat", "greeting"), Some(&json!("hello")));
    }

    #[test]
    fn test_get_missing() {
        let rm = ResourceManager::new();
        assert_eq!(rm.get("chat", "nonexistent"), None);
    }

    #[test]
    fn test_get_as() {
        let mut rm = ResourceManager::new();
        rm.set("clock", "format", json!("24h"));
        let format: Option<String> = rm.get_as("clock", "format").unwrap();
        assert_eq!(format, Some("24h".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut rm = ResourceManager::new();
        rm.set("chat", "key", json!("value"));
        rm.remove("chat", "key");
        assert_eq!(rm.get("chat", "key"), None);
    }

    #[test]
    fn test_clear_app() {
        let mut rm = ResourceManager::new();
        rm.set("app1", "a", json!(1));
        rm.set("app2", "b", json!(2));
        rm.clear_app("app1");
        assert!(rm.get("app1", "a").is_none());
        assert!(rm.get("app2", "b").is_some());
    }

    #[test]
    fn test_has() {
        let mut rm = ResourceManager::new();
        rm.set("chat", "present", json!(true));
        assert!(rm.has("chat", "present"));
        assert!(!rm.has("chat", "missing"));
    }
}
