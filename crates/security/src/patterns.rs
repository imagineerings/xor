use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub category: PatternCategory,
    pub matcher: PatternMatcher,
    pub severity: Severity,
    pub action: PatternAction,
    #[serde(default = "default_active")]
    pub active: bool,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternCategory {
    PromptInjection,
    SensitiveData,
    Pii,
    Credentials,
    HarmfulContent,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PatternMatcher {
    Regex(String),
    Contains(String),
    Exact(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternAction {
    Allow,
    Flag,
    Redact,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub pattern_id: String,
    pub pattern_name: String,
    pub category: PatternCategory,
    pub severity: Severity,
    pub action: PatternAction,
    pub byte_range: Range<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct PatternRegistry {
    patterns: Vec<CompiledPattern>,
}

impl PatternRegistry {
    pub fn new(patterns: impl IntoIterator<Item = Pattern>) -> Result<Self, PatternError> {
        let patterns = patterns
            .into_iter()
            .map(CompiledPattern::compile)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { patterns })
    }

    pub fn from_json_str(json: &str) -> Result<Self, PatternError> {
        let patterns: Vec<Pattern> = serde_json::from_str(json)?;
        Self::new(patterns)
    }

    pub fn from_json_reader(mut reader: impl Read) -> Result<Self, PatternError> {
        let mut json = String::new();
        reader.read_to_string(&mut json)?;
        Self::from_json_str(&json)
    }

    pub fn load_json_file(path: impl AsRef<Path>) -> Result<Self, PatternError> {
        Self::from_json_reader(File::open(path)?)
    }

    pub fn patterns(&self) -> impl ExactSizeIterator<Item = &Pattern> {
        self.patterns.iter().map(|pattern| &pattern.pattern)
    }

    pub fn active_patterns(&self) -> impl Iterator<Item = &Pattern> {
        self.patterns().filter(|pattern| pattern.active)
    }

    pub fn matches(&self, content: &str) -> Vec<PatternMatch> {
        self.patterns
            .iter()
            .filter(|pattern| pattern.pattern.active)
            .filter_map(|pattern| pattern.find(content))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct CompiledPattern {
    pattern: Pattern,
    matcher: CompiledMatcher,
}

impl CompiledPattern {
    fn compile(pattern: Pattern) -> Result<Self, PatternError> {
        let matcher = match &pattern.matcher {
            PatternMatcher::Regex(regex) => {
                let regex = RegexBuilder::new(regex)
                    .case_insensitive(!pattern.case_sensitive)
                    .build()
                    .map_err(|source| PatternError::InvalidRegex {
                        pattern_id: pattern.id.clone(),
                        source,
                    })?;
                CompiledMatcher::Regex(regex)
            }
            PatternMatcher::Contains(value) => CompiledMatcher::Contains(value.clone()),
            PatternMatcher::Exact(value) => CompiledMatcher::Exact(value.clone()),
        };

        Ok(Self { pattern, matcher })
    }

    fn find(&self, content: &str) -> Option<PatternMatch> {
        self.matcher
            .find(content, self.pattern.case_sensitive)
            .map(|byte_range| PatternMatch {
                pattern_id: self.pattern.id.clone(),
                pattern_name: self.pattern.name.clone(),
                category: self.pattern.category.clone(),
                severity: self.pattern.severity,
                action: self.pattern.action,
                byte_range,
            })
    }
}

#[derive(Debug, Clone)]
enum CompiledMatcher {
    Regex(Regex),
    Contains(String),
    Exact(String),
}

impl CompiledMatcher {
    fn find(&self, content: &str, case_sensitive: bool) -> Option<Range<usize>> {
        match self {
            Self::Regex(regex) => regex.find(content).map(|matched| matched.range()),
            Self::Contains(value) => find_contains(content, value, case_sensitive),
            Self::Exact(value) => {
                matches_exact(content, value, case_sensitive).then_some(0..content.len())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("failed to read pattern configuration")]
    Io(#[from] std::io::Error),
    #[error("failed to parse pattern configuration")]
    Json(#[from] serde_json::Error),
    #[error("pattern {pattern_id:?} has an invalid regex")]
    InvalidRegex {
        pattern_id: String,
        source: regex::Error,
    },
}

fn find_contains(content: &str, value: &str, case_sensitive: bool) -> Option<Range<usize>> {
    if value.is_empty() {
        return None;
    }

    if case_sensitive {
        return content.find(value).map(|start| start..start + value.len());
    }

    content.char_indices().find_map(|(start, _)| {
        let end = start.checked_add(value.len())?;
        let candidate = content.get(start..end)?;
        candidate.eq_ignore_ascii_case(value).then_some(start..end)
    })
}

fn matches_exact(content: &str, value: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        content == value
    } else {
        content.eq_ignore_ascii_case(value)
    }
}

fn default_active() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(id: &str, matcher: PatternMatcher) -> Pattern {
        Pattern {
            id: id.into(),
            name: format!("{id} pattern"),
            category: PatternCategory::PromptInjection,
            matcher,
            severity: Severity::High,
            action: PatternAction::Block,
            active: true,
            case_sensitive: false,
        }
    }

    #[test]
    fn compiles_and_matches_regex_patterns() {
        let registry = PatternRegistry::new([pattern(
            "ignore-previous",
            PatternMatcher::Regex(r"ignore\s+previous\s+instructions".into()),
        )])
        .expect("registry should compile");

        let matches = registry.matches("Please IGNORE previous instructions.");
        let first_match = matches.first().expect("expected one match");

        assert_eq!(matches.len(), 1);
        assert_eq!(first_match.pattern_id, "ignore-previous");
        assert_eq!(first_match.action, PatternAction::Block);
    }

    #[test]
    fn supports_contains_and_exact_matchers() {
        let registry = PatternRegistry::new([
            pattern("secret", PatternMatcher::Contains("api_key".into())),
            pattern("exact", PatternMatcher::Exact("root password".into())),
        ])
        .expect("registry should compile");

        assert_eq!(registry.matches("API_KEY=abc").len(), 1);
        assert_eq!(registry.matches("Root Password").len(), 1);
        assert!(registry.matches("root password reset").is_empty());
    }

    #[test]
    fn skips_inactive_patterns() {
        let mut inactive = pattern("inactive", PatternMatcher::Contains("blocked".into()));
        inactive.active = false;

        let registry = PatternRegistry::new([inactive]).expect("registry should compile");

        assert_eq!(registry.patterns().len(), 1);
        assert_eq!(registry.active_patterns().count(), 0);
        assert!(registry.matches("blocked").is_empty());
    }

    #[test]
    fn reports_invalid_regex_with_pattern_id() {
        let error = PatternRegistry::new([pattern("bad", PatternMatcher::Regex("(".into()))])
            .expect_err("invalid regex should fail");

        match error {
            PatternError::InvalidRegex { pattern_id, .. } => assert_eq!(pattern_id, "bad"),
            other => unreachable!("expected invalid regex error, got {other:?}"),
        }
    }

    #[test]
    fn loads_patterns_from_json() {
        let registry = PatternRegistry::from_json_str(
            r#"
            [
              {
                "id": "credential",
                "name": "Credential marker",
                "category": "credentials",
                "matcher": { "type": "contains", "value": "token=" },
                "severity": "critical",
                "action": "redact"
              }
            ]
            "#,
        )
        .expect("json patterns should load");

        let matches = registry.matches("token=secret");
        let first_match = matches.first().expect("expected one match");

        assert_eq!(matches.len(), 1);
        assert_eq!(first_match.category, PatternCategory::Credentials);
        assert_eq!(first_match.severity, Severity::Critical);
        assert_eq!(first_match.action, PatternAction::Redact);
    }
}
