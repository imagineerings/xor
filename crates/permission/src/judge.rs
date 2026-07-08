#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: String,
}

impl ToolCall {
    pub fn new(tool_name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments: arguments.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PermissionJudge {
    policy: PermissionPolicy,
}

impl PermissionJudge {
    pub fn new(policy: PermissionPolicy) -> Self {
        Self { policy }
    }

    pub fn judge(&self, tool_call: &ToolCall) -> PermissionDecision {
        let risk_level = self.risk_level(tool_call);

        match risk_level {
            RiskLevel::Low => PermissionDecision::Allowed,
            RiskLevel::Medium => PermissionDecision::NeedsConfirmation {
                reason: format!("{} requires confirmation", tool_call.tool_name),
                risk_level,
            },
            RiskLevel::High => PermissionDecision::Denied {
                reason: format!("{} is high risk", tool_call.tool_name),
                risk_level,
            },
        }
    }

    pub fn risk_level(&self, tool_call: &ToolCall) -> RiskLevel {
        if self
            .policy
            .high_risk_tools
            .iter()
            .any(|tool_name| tool_name == &tool_call.tool_name)
            || self
                .policy
                .high_risk_argument_markers
                .iter()
                .any(|marker| tool_call.arguments.to_ascii_lowercase().contains(marker))
        {
            RiskLevel::High
        } else if self
            .policy
            .low_risk_tools
            .iter()
            .any(|tool_name| tool_name == &tool_call.tool_name)
        {
            RiskLevel::Low
        } else {
            RiskLevel::Medium
        }
    }
}

impl Default for PermissionJudge {
    fn default() -> Self {
        Self::new(PermissionPolicy::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPolicy {
    pub low_risk_tools: Vec<String>,
    pub high_risk_tools: Vec<String>,
    pub high_risk_argument_markers: Vec<String>,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            low_risk_tools: vec![
                "read_file".into(),
                "list_files".into(),
                "search".into(),
                "grep".into(),
            ],
            high_risk_tools: vec!["delete_file".into(), "write_file".into()],
            high_risk_argument_markers: vec![
                "rm -rf".into(),
                "sudo ".into(),
                "chmod 777".into(),
                "curl ".into(),
                "| sh".into(),
                "| bash".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allowed,
    Denied {
        reason: String,
        risk_level: RiskLevel,
    },
    NeedsConfirmation {
        reason: String,
        risk_level: RiskLevel,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_allows_low_risk_tools() {
        let judge = PermissionJudge::default();

        assert_eq!(
            judge.judge(&ToolCall::new("read_file", "/tmp/example")),
            PermissionDecision::Allowed
        );
    }

    #[test]
    fn auto_blocks_high_risk_tools() {
        let judge = PermissionJudge::default();

        let decision = judge.judge(&ToolCall::new("delete_file", "/tmp/example"));

        assert!(matches!(
            decision,
            PermissionDecision::Denied {
                risk_level: RiskLevel::High,
                ..
            }
        ));
    }

    #[test]
    fn auto_blocks_high_risk_arguments() {
        let judge = PermissionJudge::default();

        let decision = judge.judge(&ToolCall::new("terminal", "rm -rf target"));

        assert!(matches!(
            decision,
            PermissionDecision::Denied {
                risk_level: RiskLevel::High,
                ..
            }
        ));
    }

    #[test]
    fn prompts_for_medium_risk_tools() {
        let judge = PermissionJudge::default();

        let decision = judge.judge(&ToolCall::new("terminal", "cargo test"));

        assert!(matches!(
            decision,
            PermissionDecision::NeedsConfirmation {
                risk_level: RiskLevel::Medium,
                ..
            }
        ));
    }
}
