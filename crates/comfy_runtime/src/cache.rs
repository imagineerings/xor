use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use thiserror::Error;

pub const CACHE_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CacheKey {
    pub node_class: String,
    pub implementation_version: String,
    pub demanded_inputs: Vec<(String, String)>,
    #[serde(default)]
    pub demanded_dependencies: Vec<(String, String)>,
    pub artifact_digests: Vec<(String, String)>,
    pub backend: String,
    pub dtype_policy: String,
    pub plugin_digest: Option<String>,
    pub rng_phase: Option<String>,
    pub configuration_token: String,
    pub registry_version: String,
    pub change_token: String,
}

impl CacheKey {
    #[allow(clippy::too_many_arguments)]
    pub fn from_inputs(
        node_class: impl Into<String>,
        implementation_version: impl Into<String>,
        inputs: &BTreeMap<String, Value>,
        artifact_digests: BTreeMap<String, String>,
        backend: impl Into<String>,
        dtype_policy: impl Into<String>,
        plugin_digest: Option<String>,
        rng_phase: Option<String>,
        configuration_token: impl Into<String>,
        registry_version: impl Into<String>,
        change_token: impl Into<String>,
    ) -> Result<Self, NativeCacheError> {
        Self::from_inputs_with_dependencies(
            node_class,
            implementation_version,
            inputs,
            BTreeMap::new(),
            artifact_digests,
            backend,
            dtype_policy,
            plugin_digest,
            rng_phase,
            configuration_token,
            registry_version,
            change_token,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_inputs_with_dependencies(
        node_class: impl Into<String>,
        implementation_version: impl Into<String>,
        inputs: &BTreeMap<String, Value>,
        demanded_dependencies: BTreeMap<String, String>,
        artifact_digests: BTreeMap<String, String>,
        backend: impl Into<String>,
        dtype_policy: impl Into<String>,
        plugin_digest: Option<String>,
        rng_phase: Option<String>,
        configuration_token: impl Into<String>,
        registry_version: impl Into<String>,
        change_token: impl Into<String>,
    ) -> Result<Self, NativeCacheError> {
        let demanded_inputs = inputs
            .iter()
            .map(|(name, value)| Ok((name.clone(), canonical_json(value)?)))
            .collect::<Result<Vec<_>, NativeCacheError>>()?;
        let node_class = node_class.into();
        let implementation_version = implementation_version.into();
        let backend = backend.into();
        let dtype_policy = dtype_policy.into();
        let configuration_token = configuration_token.into();
        let registry_version = registry_version.into();
        let change_token = change_token.into();
        for (name, value) in [
            ("node class", node_class.as_str()),
            ("implementation version", implementation_version.as_str()),
            ("backend", backend.as_str()),
            ("dtype policy", dtype_policy.as_str()),
            ("configuration token", configuration_token.as_str()),
            ("registry version", registry_version.as_str()),
            ("change token", change_token.as_str()),
        ] {
            if value.is_empty() {
                return Err(NativeCacheError::InvalidDimension(name));
            }
        }
        if demanded_dependencies
            .iter()
            .chain(artifact_digests.iter())
            .any(|(name, identity)| name.is_empty() || identity.is_empty())
            || plugin_digest.as_deref().is_some_and(str::is_empty)
            || rng_phase.as_deref().is_some_and(str::is_empty)
        {
            return Err(NativeCacheError::InvalidDependencyIdentity);
        }
        Ok(Self {
            node_class,
            implementation_version,
            demanded_inputs,
            demanded_dependencies: demanded_dependencies.into_iter().collect(),
            artifact_digests: artifact_digests.into_iter().collect(),
            backend,
            dtype_policy,
            plugin_digest,
            rng_phase,
            configuration_token,
            registry_version,
            change_token,
        })
    }

    pub fn identity(&self) -> Result<String, NativeCacheError> {
        let value = serde_json::to_value(self)
            .map_err(|error| NativeCacheError::Canonicalization(error.to_string()))?;
        let canonical = canonical_json(&value)?;
        Ok(format!("{:x}", Sha256::digest(canonical.as_bytes())))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub outputs: Vec<Value>,
    pub ui: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub schema_version: u16,
    pub entry_count: usize,
    pub registry_versions: Vec<String>,
    pub artifact_digests: Vec<(String, String)>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum NativeCacheError {
    #[error("cache capacity must be non-zero")]
    ZeroCapacity,
    #[error("cache value could not be canonicalized: {0}")]
    Canonicalization(String),
    #[error("cache {0} must be non-empty")]
    InvalidDimension(&'static str),
    #[error("cache dependency identities must have non-empty names and values")]
    InvalidDependencyIdentity,
}

#[derive(Clone, Debug)]
struct CacheRecord {
    entry: CacheEntry,
}

#[derive(Clone, Debug)]
pub struct NativeCache {
    maximum_entries: usize,
    entries: BTreeMap<CacheKey, CacheRecord>,
    least_recently_used: VecDeque<CacheKey>,
}

impl NativeCache {
    pub fn new(maximum_entries: usize) -> Result<Self, NativeCacheError> {
        if maximum_entries == 0 {
            return Err(NativeCacheError::ZeroCapacity);
        }
        Ok(Self {
            maximum_entries,
            entries: BTreeMap::new(),
            least_recently_used: VecDeque::new(),
        })
    }

    pub fn get(&mut self, key: &CacheKey) -> Option<CacheEntry> {
        let entry = self.entries.get(key)?.entry.clone();
        self.touch(key);
        Some(entry)
    }

    pub fn insert(&mut self, key: CacheKey, entry: CacheEntry) {
        self.entries.insert(key.clone(), CacheRecord { entry });
        self.touch(&key);
        while self.entries.len() > self.maximum_entries {
            let Some(oldest) = self.least_recently_used.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }

    pub fn invalidate_node(&mut self, node_class: &str) -> usize {
        self.retain(|key| key.node_class != node_class)
    }

    pub fn invalidate_registry(&mut self, registry_version: &str) -> usize {
        self.retain(|key| key.registry_version == registry_version)
    }

    pub fn invalidate_artifact(&mut self, path: &str, current_digest: &str) -> usize {
        self.retain(|key| {
            key.artifact_digests
                .iter()
                .all(|(dependency_path, digest)| {
                    dependency_path != path || digest == current_digest
                })
        })
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.least_recently_used.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn metadata(&self) -> CacheMetadata {
        let registry_versions = self
            .entries
            .keys()
            .map(|key| key.registry_version.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let artifact_digests = self
            .entries
            .keys()
            .flat_map(|key| key.artifact_digests.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        CacheMetadata {
            schema_version: CACHE_SCHEMA_VERSION,
            entry_count: self.entries.len(),
            registry_versions,
            artifact_digests,
        }
    }

    fn touch(&mut self, key: &CacheKey) {
        self.least_recently_used
            .retain(|candidate| candidate != key);
        self.least_recently_used.push_back(key.clone());
    }

    fn retain(&mut self, mut keep: impl FnMut(&CacheKey) -> bool) -> usize {
        let before = self.entries.len();
        self.entries.retain(|key, _| keep(key));
        self.least_recently_used
            .retain(|key| self.entries.contains_key(key));
        before.saturating_sub(self.entries.len())
    }
}

pub fn canonical_json(value: &Value) -> Result<String, NativeCacheError> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => serde_json::to_string(value)
            .map_err(|error| NativeCacheError::Canonicalization(error.to_string())),
        Value::Array(values) => {
            let values = values
                .iter()
                .map(canonical_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", values.join(",")))
        }
        Value::Object(values) => {
            let mut fields = values
                .iter()
                .map(|(key, value)| {
                    Ok(format!(
                        "{}:{}",
                        serde_json::to_string(key).map_err(|error| {
                            NativeCacheError::Canonicalization(error.to_string())
                        })?,
                        canonical_json(value)?
                    ))
                })
                .collect::<Result<Vec<_>, NativeCacheError>>()?;
            fields.sort();
            Ok(format!("{{{}}}", fields.join(",")))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn key(node: &str, input: Value) -> Result<CacheKey, NativeCacheError> {
        CacheKey::from_inputs(
            node,
            "1",
            &BTreeMap::from([("input".to_owned(), input)]),
            BTreeMap::new(),
            "cpu",
            "f32",
            None,
            None,
            "config-v1",
            "registry-v1",
            "stable",
        )
    }

    #[test]
    fn val_domain_004_canonical_inputs_ignore_object_insertion_order()
    -> Result<(), NativeCacheError> {
        let first = json!({"b": 2, "a": 1});
        let second: Value = serde_json::from_str(r#"{"a":1,"b":2}"#)
            .map_err(|error| NativeCacheError::Canonicalization(error.to_string()))?;
        assert_eq!(key("Node", first)?, key("Node", second)?);
        Ok(())
    }

    #[test]
    fn val_domain_004_lru_and_targeted_invalidation_are_deterministic()
    -> Result<(), NativeCacheError> {
        let mut cache = NativeCache::new(2)?;
        let first = key("First", json!(1))?;
        let second = key("Second", json!(2))?;
        let third = key("Third", json!(3))?;
        cache.insert(
            first.clone(),
            CacheEntry {
                outputs: vec![json!(1)],
                ui: None,
            },
        );
        cache.insert(
            second.clone(),
            CacheEntry {
                outputs: vec![json!(2)],
                ui: None,
            },
        );
        assert!(cache.get(&first).is_some());
        cache.insert(
            third.clone(),
            CacheEntry {
                outputs: vec![json!(3)],
                ui: None,
            },
        );
        assert!(cache.get(&second).is_none());
        assert_eq!(cache.invalidate_node("First"), 1);
        assert!(cache.get(&third).is_some());
        Ok(())
    }

    #[test]
    fn val_domain_004_dependency_identities_are_canonical_and_validated()
    -> Result<(), NativeCacheError> {
        let inputs = BTreeMap::from([("value".to_owned(), json!(7))]);
        let first = CacheKey::from_inputs_with_dependencies(
            "Output",
            "1",
            &inputs,
            BTreeMap::from([("value".to_owned(), "source-v1".to_owned())]),
            BTreeMap::new(),
            "cpu",
            "f32",
            None,
            None,
            "config-v1",
            "registry-v1",
            "stable",
        )?;
        let second = CacheKey::from_inputs_with_dependencies(
            "Output",
            "1",
            &inputs,
            BTreeMap::from([("value".to_owned(), "source-v2".to_owned())]),
            BTreeMap::new(),
            "cpu",
            "f32",
            None,
            None,
            "config-v1",
            "registry-v1",
            "stable",
        )?;
        assert_ne!(first, second);
        assert_ne!(first.identity()?, second.identity()?);
        assert!(matches!(
            CacheKey::from_inputs_with_dependencies(
                "Output",
                "1",
                &inputs,
                BTreeMap::from([("value".to_owned(), String::new())]),
                BTreeMap::new(),
                "cpu",
                "f32",
                None,
                None,
                "config-v1",
                "registry-v1",
                "stable",
            ),
            Err(NativeCacheError::InvalidDependencyIdentity)
        ));
        Ok(())
    }

    pub(crate) fn val_domain_004_cache_case_results()
    -> Result<Vec<(&'static str, bool)>, NativeCacheError> {
        val_domain_004_canonical_inputs_ignore_object_insertion_order()?;
        val_domain_004_lru_and_targeted_invalidation_are_deterministic()?;
        val_domain_004_dependency_identities_are_canonical_and_validated()?;
        Ok(vec![
            ("cache_canonical_declared_inputs", true),
            ("cache_lru_targeted_invalidation", true),
            ("cache_demanded_dependency_identity", true),
        ])
    }
}
