use crate::{
    Pattern, PatternAction, PatternCategory, PatternError, PatternMatch, PatternMatcher,
    PatternRegistry, Severity,
};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct EgressInspector {
    registry: PatternRegistry,
}

impl EgressInspector {
    pub fn new(registry: PatternRegistry) -> Self {
        Self { registry }
    }

    pub fn with_default_patterns(strategy: RedactionStrategy) -> Result<Self, PatternError> {
        Ok(Self::new(PatternRegistry::new(default_egress_patterns(
            strategy,
        ))?))
    }

    pub fn inspect(&self, content: &str) -> EgressInspection {
        let findings = self
            .registry
            .matches(content)
            .into_iter()
            .map(EgressFinding::from)
            .collect::<Vec<_>>();
        let blocked = findings
            .iter()
            .any(|finding| finding.action == PatternAction::Block);
        let redacted_content = (!blocked)
            .then(|| redact_findings(content, &findings))
            .flatten();

        EgressInspection {
            passed: findings.is_empty(),
            blocked,
            redacted_content,
            findings,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionStrategy {
    Redact,
    Block,
}

impl RedactionStrategy {
    fn action(self) -> PatternAction {
        match self {
            Self::Redact => PatternAction::Redact,
            Self::Block => PatternAction::Block,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgressInspection {
    pub passed: bool,
    pub blocked: bool,
    pub redacted_content: Option<String>,
    pub findings: Vec<EgressFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgressFinding {
    pub pattern_id: String,
    pub pattern_name: String,
    pub category: PatternCategory,
    pub severity: Severity,
    pub action: PatternAction,
    pub byte_range: Range<usize>,
}

impl From<PatternMatch> for EgressFinding {
    fn from(pattern_match: PatternMatch) -> Self {
        Self {
            pattern_id: pattern_match.pattern_id,
            pattern_name: pattern_match.pattern_name,
            category: pattern_match.category,
            severity: pattern_match.severity,
            action: pattern_match.action,
            byte_range: pattern_match.byte_range,
        }
    }
}

pub fn default_egress_patterns(strategy: RedactionStrategy) -> Vec<Pattern> {
    vec![
        egress_pattern(
            "openai-api-key",
            "OpenAI API key",
            PatternCategory::Credentials,
            PatternMatcher::Regex(r"\bsk-[A-Za-z0-9]{20,}\b".into()),
            Severity::Critical,
            strategy,
        ),
        egress_pattern(
            "aws-access-key",
            "AWS access key",
            PatternCategory::Credentials,
            PatternMatcher::Regex(r"\bAKIA[0-9A-Z]{16}\b".into()),
            Severity::Critical,
            strategy,
        ),
        egress_pattern(
            "bearer-token",
            "Bearer token",
            PatternCategory::Credentials,
            PatternMatcher::Regex(r"\bBearer\s+[A-Za-z0-9._~+/=-]{16,}\b".into()),
            Severity::High,
            strategy,
        ),
        egress_pattern(
            "private-key-header",
            "Private key header",
            PatternCategory::SensitiveData,
            PatternMatcher::Contains("-----BEGIN PRIVATE KEY-----".into()),
            Severity::Critical,
            strategy,
        ),
        egress_pattern(
            "email-address",
            "Email address",
            PatternCategory::Pii,
            PatternMatcher::Regex(r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b".into()),
            Severity::Medium,
            strategy,
        ),
        egress_pattern(
            "us-ssn",
            "US social security number",
            PatternCategory::Pii,
            PatternMatcher::Regex(r"\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b".into()),
            Severity::High,
            strategy,
        ),
    ]
}

fn egress_pattern(
    id: &str,
    name: &str,
    category: PatternCategory,
    matcher: PatternMatcher,
    severity: Severity,
    strategy: RedactionStrategy,
) -> Pattern {
    Pattern {
        id: id.into(),
        name: name.into(),
        category,
        matcher,
        severity,
        action: strategy.action(),
        active: true,
        case_sensitive: false,
    }
}

fn redact_findings(content: &str, findings: &[EgressFinding]) -> Option<String> {
    let mut redactions = findings
        .iter()
        .filter(|finding| finding.action == PatternAction::Redact)
        .map(|finding| (finding.byte_range.clone(), finding.pattern_id.as_str()))
        .collect::<Vec<_>>();

    if redactions.is_empty() {
        return None;
    }

    redactions.sort_by_key(|(range, _)| range.start);

    let mut redacted = String::with_capacity(content.len());
    let mut cursor = 0;
    for (range, pattern_id) in redactions {
        if range.start < cursor {
            continue;
        }

        if let Some(prefix) = content.get(cursor..range.start) {
            redacted.push_str(prefix);
        }
        redacted.push_str("[REDACTED:");
        redacted.push_str(pattern_id);
        redacted.push(']');
        cursor = range.end;
    }

    if let Some(suffix) = content.get(cursor..content.len()) {
        redacted.push_str(suffix);
    }

    Some(redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_api_keys_with_default_strategy() {
        let inspector = EgressInspector::with_default_patterns(RedactionStrategy::Redact)
            .expect("default patterns should compile");

        let inspection = inspector.inspect("Use sk-1234567890abcdefghijklmnop for the request.");

        assert!(!inspection.passed);
        assert!(!inspection.blocked);
        assert_eq!(
            inspection.redacted_content.as_deref(),
            Some("Use [REDACTED:openai-api-key] for the request.")
        );
    }

    #[test]
    fn blocks_matches_with_block_strategy() {
        let inspector = EgressInspector::with_default_patterns(RedactionStrategy::Block)
            .expect("default patterns should compile");

        let inspection = inspector.inspect("Contact me at user@example.com.");

        assert!(!inspection.passed);
        assert!(inspection.blocked);
        assert!(inspection.redacted_content.is_none());
        assert_eq!(
            inspection
                .findings
                .first()
                .expect("expected finding")
                .action,
            PatternAction::Block
        );
    }

    #[test]
    fn detects_pii_and_redacts_without_leaking_value() {
        let inspector = EgressInspector::with_default_patterns(RedactionStrategy::Redact)
            .expect("default patterns should compile");

        let inspection = inspector.inspect("SSN: 123-45-6789");

        assert!(!inspection.passed);
        assert_eq!(
            inspection.redacted_content.as_deref(),
            Some("SSN: [REDACTED:us-ssn]")
        );
        assert_eq!(
            inspection
                .findings
                .first()
                .expect("expected finding")
                .category,
            PatternCategory::Pii
        );
    }

    #[test]
    fn custom_blocking_pattern_takes_precedence_over_redaction() {
        let registry = PatternRegistry::new([Pattern {
            id: "internal-project".into(),
            name: "Internal project".into(),
            category: PatternCategory::SensitiveData,
            matcher: PatternMatcher::Contains("Project Nightfall".into()),
            severity: Severity::High,
            action: PatternAction::Block,
            active: true,
            case_sensitive: false,
        }])
        .expect("registry should compile");
        let inspector = EgressInspector::new(registry);

        let inspection = inspector.inspect("Project Nightfall should stay private.");

        assert!(inspection.blocked);
        assert!(inspection.redacted_content.is_none());
    }

    #[test]
    fn benign_output_passes() {
        let inspector = EgressInspector::with_default_patterns(RedactionStrategy::Redact)
            .expect("default patterns should compile");

        let inspection = inspector.inspect("The build completed successfully.");

        assert!(inspection.passed);
        assert!(!inspection.blocked);
        assert!(inspection.redacted_content.is_none());
        assert!(inspection.findings.is_empty());
    }
}
