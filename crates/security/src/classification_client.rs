use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ClassificationClient<P = NoopClassificationProvider> {
    config: ClassificationConfig,
    provider: P,
}

impl<P> ClassificationClient<P>
where
    P: ClassificationProvider,
{
    pub fn new(config: ClassificationConfig, provider: P) -> Self {
        Self { config, provider }
    }

    pub fn classify(&self, content: &str) -> ClassificationDecision {
        if !self.config.enabled {
            return ClassificationDecision::allowed(None);
        }

        match self.provider.classify(content) {
            Ok(rating) => self.classify_rating(rating),
            Err(ClassificationError::Unavailable(reason)) => match self.config.unavailable_policy {
                ClassificationUnavailablePolicy::FailOpen => ClassificationDecision {
                    allowed: true,
                    action: ClassificationAction::Allow,
                    reason: Some(reason),
                    rating: None,
                },
                ClassificationUnavailablePolicy::FailClosed => ClassificationDecision {
                    allowed: false,
                    action: ClassificationAction::Block,
                    reason: Some(reason),
                    rating: None,
                },
            },
            Err(ClassificationError::Provider(reason)) => ClassificationDecision {
                allowed: false,
                action: ClassificationAction::Block,
                reason: Some(reason),
                rating: None,
            },
        }
    }

    pub fn config(&self) -> &ClassificationConfig {
        &self.config
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    fn classify_rating(&self, rating: ClassificationRating) -> ClassificationDecision {
        let highest_score = rating.highest_score();
        let exceeds_threshold = rating.flagged || highest_score >= self.config.threshold;

        if exceeds_threshold {
            ClassificationDecision {
                allowed: self.config.action != ClassificationAction::Block,
                action: self.config.action,
                reason: rating.highest_category().map(|category| {
                    format!(
                        "classification category {:?} scored {:.3}",
                        category.name, category.score
                    )
                }),
                rating: Some(rating),
            }
        } else {
            ClassificationDecision::allowed(Some(rating))
        }
    }
}

impl ClassificationClient<NoopClassificationProvider> {
    pub fn disabled() -> Self {
        Self::new(ClassificationConfig::disabled(), NoopClassificationProvider)
    }
}

pub trait ClassificationProvider: Clone + Send + Sync + 'static {
    fn classify(&self, content: &str) -> Result<ClassificationRating, ClassificationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopClassificationProvider;

impl ClassificationProvider for NoopClassificationProvider {
    fn classify(&self, _content: &str) -> Result<ClassificationRating, ClassificationError> {
        Ok(ClassificationRating::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassificationConfig {
    pub enabled: bool,
    pub threshold: f32,
    pub action: ClassificationAction,
    pub unavailable_policy: ClassificationUnavailablePolicy,
}

impl ClassificationConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.7,
            action: ClassificationAction::Block,
            unavailable_policy: ClassificationUnavailablePolicy::FailOpen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationAction {
    Allow,
    Flag,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationUnavailablePolicy {
    FailOpen,
    FailClosed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClassificationRating {
    pub flagged: bool,
    pub categories: Vec<SafetyCategory>,
}

impl ClassificationRating {
    pub fn highest_score(&self) -> f32 {
        self.highest_category()
            .map(|category| category.score)
            .unwrap_or_default()
    }

    pub fn highest_category(&self) -> Option<&SafetyCategory> {
        self.categories
            .iter()
            .max_by(|left, right| left.score.total_cmp(&right.score))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyCategory {
    pub name: String,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationDecision {
    pub allowed: bool,
    pub action: ClassificationAction,
    pub reason: Option<String>,
    pub rating: Option<ClassificationRating>,
}

impl ClassificationDecision {
    fn allowed(rating: Option<ClassificationRating>) -> Self {
        Self {
            allowed: true,
            action: ClassificationAction::Allow,
            reason: None,
            rating,
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ClassificationError {
    #[error("classification provider unavailable: {0}")]
    Unavailable(String),
    #[error("classification provider failed: {0}")]
    Provider(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestProvider {
        result: Result<ClassificationRating, ClassificationError>,
    }

    impl ClassificationProvider for TestProvider {
        fn classify(&self, _content: &str) -> Result<ClassificationRating, ClassificationError> {
            self.result.clone()
        }
    }

    fn client_with_rating(
        config: ClassificationConfig,
        rating: ClassificationRating,
    ) -> ClassificationClient<TestProvider> {
        ClassificationClient::new(config, TestProvider { result: Ok(rating) })
    }

    #[test]
    fn disabled_client_allows_without_provider_signal() {
        let client = ClassificationClient::disabled();

        let decision = client.classify("content");

        assert!(decision.allowed);
        assert_eq!(decision.action, ClassificationAction::Allow);
        assert!(decision.rating.is_none());
    }

    #[test]
    fn blocks_when_score_exceeds_threshold() {
        let client = client_with_rating(
            ClassificationConfig {
                threshold: 0.4,
                ..ClassificationConfig::default()
            },
            ClassificationRating {
                flagged: false,
                categories: vec![SafetyCategory {
                    name: "violence".into(),
                    score: 0.8,
                }],
            },
        );

        let decision = client.classify("unsafe content");

        assert!(!decision.allowed);
        assert_eq!(decision.action, ClassificationAction::Block);
        assert!(
            decision
                .reason
                .as_deref()
                .is_some_and(|reason| { reason.contains("violence") && reason.contains("0.800") })
        );
    }

    #[test]
    fn can_flag_instead_of_blocking() {
        let client = client_with_rating(
            ClassificationConfig {
                threshold: 0.5,
                action: ClassificationAction::Flag,
                ..ClassificationConfig::default()
            },
            ClassificationRating {
                flagged: true,
                categories: vec![SafetyCategory {
                    name: "self_harm".into(),
                    score: 0.1,
                }],
            },
        );

        let decision = client.classify("flagged content");

        assert!(decision.allowed);
        assert_eq!(decision.action, ClassificationAction::Flag);
        assert!(decision.rating.is_some());
    }

    #[test]
    fn allows_scores_below_threshold() {
        let client = client_with_rating(
            ClassificationConfig::default(),
            ClassificationRating {
                flagged: false,
                categories: vec![SafetyCategory {
                    name: "harassment".into(),
                    score: 0.1,
                }],
            },
        );

        let decision = client.classify("benign content");

        assert!(decision.allowed);
        assert_eq!(decision.action, ClassificationAction::Allow);
    }

    #[test]
    fn provider_unavailable_can_fail_open() {
        let client = ClassificationClient::new(
            ClassificationConfig {
                unavailable_policy: ClassificationUnavailablePolicy::FailOpen,
                ..ClassificationConfig::default()
            },
            TestProvider {
                result: Err(ClassificationError::Unavailable("timeout".into())),
            },
        );

        let decision = client.classify("content");

        assert!(decision.allowed);
        assert_eq!(decision.action, ClassificationAction::Allow);
        assert_eq!(decision.reason.as_deref(), Some("timeout"));
    }

    #[test]
    fn provider_unavailable_can_fail_closed() {
        let client = ClassificationClient::new(
            ClassificationConfig {
                unavailable_policy: ClassificationUnavailablePolicy::FailClosed,
                ..ClassificationConfig::default()
            },
            TestProvider {
                result: Err(ClassificationError::Unavailable("timeout".into())),
            },
        );

        let decision = client.classify("content");

        assert!(!decision.allowed);
        assert_eq!(decision.action, ClassificationAction::Block);
        assert_eq!(decision.reason.as_deref(), Some("timeout"));
    }
}
