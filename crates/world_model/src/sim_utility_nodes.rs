use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SimUserDataDiagnostic, normalize_user_path};

pub const SIM_UTILITY_INVALID_REGEX_CODE: &str = "world_model.utility_nodes.invalid_regex";
pub const SIM_UTILITY_JSON_PATH_CODE: &str = "world_model.utility_nodes.invalid_json_path";
pub const SIM_UTILITY_MATH_EXPRESSION_CODE: &str =
    "world_model.utility_nodes.invalid_math_expression";
pub const SIM_UTILITY_DATASET_PATH_CODE: &str = "world_model.utility_nodes.invalid_dataset_path";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SimUtilityValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Json(Value),
    Seed(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimUtilityLogicOp {
    And,
    Or,
    Xor,
    Not,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDatasetEntry {
    pub source_path: PathBuf,
    pub source_reference: String,
    pub text: Option<String>,
    pub bucket: Option<String>,
    pub attribution: BTreeMap<String, String>,
}

impl SimDatasetEntry {
    pub fn new(
        source_path: impl AsRef<Path>,
        source_reference: impl Into<String>,
    ) -> Result<Self, SimUtilityDiagnostic> {
        Ok(Self {
            source_path: normalize_dataset_path(source_path.as_ref())?,
            source_reference: source_reference.into(),
            text: None,
            bucket: None,
            attribution: BTreeMap::new(),
        })
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn with_bucket(mut self, bucket: impl Into<String>) -> Self {
        self.bucket = Some(bucket.into());
        self
    }

    pub fn with_attribution(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attribution.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDatasetBucket {
    pub key: String,
    pub entries: Vec<SimDatasetEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimUtilityDiagnostic {
    pub code: String,
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimUtilityNodeAdapter;

impl SimUtilityNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn string(&self, value: impl Into<String>) -> SimUtilityValue {
        SimUtilityValue::String(value.into())
    }

    pub fn number(&self, value: f64) -> SimUtilityValue {
        SimUtilityValue::Number(value)
    }

    pub fn boolean(&self, value: bool) -> SimUtilityValue {
        SimUtilityValue::Boolean(value)
    }

    pub fn seed(&self, value: u64) -> SimUtilityValue {
        SimUtilityValue::Seed(value)
    }

    pub fn regex_extract(
        &self,
        pattern: &str,
        text: &str,
    ) -> Result<Vec<String>, SimUtilityDiagnostic> {
        let regex = Regex::new(pattern).map_err(|error| SimUtilityDiagnostic {
            code: SIM_UTILITY_INVALID_REGEX_CODE.to_string(),
            field: "pattern".to_string(),
            message: error.to_string(),
        })?;

        Ok(regex
            .captures_iter(text)
            .filter_map(|captures| {
                captures
                    .get(1)
                    .or_else(|| captures.get(0))
                    .map(|match_| match_.as_str().to_string())
            })
            .collect())
    }

    pub fn json_extract(&self, value: &Value, path: &str) -> Result<Value, SimUtilityDiagnostic> {
        let mut current = value;
        for segment in path.split('.') {
            if segment.trim().is_empty() {
                return Err(json_path_diagnostic("JSON path segments cannot be empty"));
            }
            current = match current {
                Value::Object(map) => map.get(segment).ok_or_else(|| {
                    json_path_diagnostic(format!("JSON path segment `{segment}` was not found"))
                })?,
                Value::Array(items) => {
                    let index = segment.parse::<usize>().map_err(|_| {
                        json_path_diagnostic(format!(
                            "JSON array segment `{segment}` must be a non-negative index"
                        ))
                    })?;
                    items.get(index).ok_or_else(|| {
                        json_path_diagnostic(format!("JSON array index `{segment}` was not found"))
                    })?
                }
                _ => {
                    return Err(json_path_diagnostic(
                        "JSON path cannot descend into scalar value",
                    ));
                }
            };
        }
        Ok(current.clone())
    }

    pub fn math_binary(
        &self,
        left: f64,
        operator: &str,
        right: f64,
    ) -> Result<f64, SimUtilityDiagnostic> {
        match operator {
            "+" => Ok(left + right),
            "-" => Ok(left - right),
            "*" => Ok(left * right),
            "/" if right != 0.0 => Ok(left / right),
            "/" => Err(math_diagnostic("division by zero is not allowed")),
            _ => Err(math_diagnostic(format!(
                "unsupported deterministic math operator `{operator}`"
            ))),
        }
    }

    pub fn logic(&self, op: SimUtilityLogicOp, values: &[bool]) -> bool {
        match op {
            SimUtilityLogicOp::And => values.iter().all(|value| *value),
            SimUtilityLogicOp::Or => values.iter().any(|value| *value),
            SimUtilityLogicOp::Xor => values.iter().filter(|value| **value).count() % 2 == 1,
            SimUtilityLogicOp::Not => !values.first().copied().unwrap_or(false),
        }
    }

    pub fn switch<T: Clone>(&self, condition: bool, when_true: T, when_false: T) -> T {
        if condition { when_true } else { when_false }
    }

    pub fn dataset_shuffle(&self, entries: &[SimDatasetEntry], seed: u64) -> Vec<SimDatasetEntry> {
        let mut entries = entries.to_vec();
        entries.sort_by(|left, right| {
            stable_dataset_key(left, seed)
                .cmp(&stable_dataset_key(right, seed))
                .then_with(|| left.source_path.cmp(&right.source_path))
        });
        entries
    }

    pub fn dataset_deduplicate(&self, entries: &[SimDatasetEntry]) -> Vec<SimDatasetEntry> {
        let mut seen = BTreeSet::new();
        let mut deduplicated = Vec::new();
        for entry in entries {
            let key = (entry.source_path.clone(), entry.text.clone());
            if seen.insert(key) {
                deduplicated.push(entry.clone());
            }
        }
        deduplicated
    }

    pub fn dataset_buckets(&self, entries: &[SimDatasetEntry]) -> Vec<SimDatasetBucket> {
        let mut buckets: BTreeMap<String, Vec<SimDatasetEntry>> = BTreeMap::new();
        for entry in entries {
            let key = entry
                .bucket
                .clone()
                .unwrap_or_else(|| "unbucketed".to_string());
            buckets.entry(key).or_default().push(entry.clone());
        }
        buckets
            .into_iter()
            .map(|(key, entries)| SimDatasetBucket { key, entries })
            .collect()
    }

    pub fn prepare_dataset(
        &self,
        entries: &[SimDatasetEntry],
    ) -> Result<Vec<SimDatasetEntry>, SimUtilityDiagnostic> {
        entries
            .iter()
            .map(|entry| {
                Ok(SimDatasetEntry {
                    source_path: normalize_dataset_path(&entry.source_path)?,
                    source_reference: entry.source_reference.clone(),
                    text: entry.text.clone(),
                    bucket: entry.bucket.clone(),
                    attribution: entry.attribution.clone(),
                })
            })
            .collect()
    }
}

fn normalize_dataset_path(path: &Path) -> Result<PathBuf, SimUtilityDiagnostic> {
    normalize_user_path(path).map_err(|diagnostic| dataset_path_diagnostic(path, diagnostic))
}

fn dataset_path_diagnostic(path: &Path, diagnostic: SimUserDataDiagnostic) -> SimUtilityDiagnostic {
    SimUtilityDiagnostic {
        code: SIM_UTILITY_DATASET_PATH_CODE.to_string(),
        field: path.to_string_lossy().into_owned(),
        message: diagnostic.message,
    }
}

fn stable_dataset_key(entry: &SimDatasetEntry, seed: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    entry.source_path.hash(&mut hasher);
    entry.source_reference.hash(&mut hasher);
    entry.text.hash(&mut hasher);
    hasher.finish()
}

fn json_path_diagnostic(message: impl Into<String>) -> SimUtilityDiagnostic {
    SimUtilityDiagnostic {
        code: SIM_UTILITY_JSON_PATH_CODE.to_string(),
        field: "path".to_string(),
        message: message.into(),
    }
}

fn math_diagnostic(message: impl Into<String>) -> SimUtilityDiagnostic {
    SimUtilityDiagnostic {
        code: SIM_UTILITY_MATH_EXPRESSION_CODE.to_string(),
        field: "operator".to_string(),
        message: message.into(),
    }
}
