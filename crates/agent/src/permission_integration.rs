use anyhow::Result;
use permission::{
    PermissionConfirmationAction, PermissionDecision, PermissionInspector, PermissionJudge,
    PermissionStore, StoredDecision, ToolCall,
};
use std::path::Path;

pub struct AgentPermissionSystem {
    store: PermissionStore,
    judge: PermissionJudge,
}

impl AgentPermissionSystem {
    pub fn open_file(path: impl AsRef<Path>, judge: PermissionJudge) -> Result<Self> {
        Ok(Self {
            store: PermissionStore::open_file(path)?,
            judge,
        })
    }

    pub fn open_memory(name: Option<&str>, judge: PermissionJudge) -> Result<Self> {
        Ok(Self {
            store: PermissionStore::open_memory(name)?,
            judge,
        })
    }

    pub fn check_tool_call(
        &self,
        tool_name: impl Into<String>,
        arguments: impl Into<String>,
        now: i64,
    ) -> Result<PermissionDecision> {
        let tool_call = ToolCall::new(tool_name, arguments);
        PermissionInspector::new(&self.store, self.judge.clone()).check_tool_call(&tool_call, now)
    }

    pub fn record_confirmation(
        &self,
        tool_call: &ToolCall,
        action: PermissionConfirmationAction,
        expires_at: Option<i64>,
    ) -> Result<()> {
        if let Some(decision_type) = action.stored_decision_type() {
            self.store.record_decision(StoredDecision::new(
                tool_call.tool_name.clone(),
                tool_call.arguments.clone(),
                decision_type,
                expires_at,
            ))?;
        }
        Ok(())
    }

    pub fn store(&self) -> &PermissionStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use permission::RiskLevel;

    #[test]
    fn checks_tool_calls_with_permission_judge() {
        let permissions = AgentPermissionSystem::open_memory(
            Some("checks_tool_calls_with_permission_judge"),
            PermissionJudge::default(),
        )
        .expect("permission system should open");

        let decision = permissions
            .check_tool_call("terminal", "cargo test", 10)
            .expect("permission check should succeed");

        assert!(matches!(
            decision,
            PermissionDecision::NeedsConfirmation {
                risk_level: RiskLevel::Medium,
                ..
            }
        ));
    }

    #[test]
    fn records_confirmation_for_future_tool_calls() {
        let permissions = AgentPermissionSystem::open_memory(
            Some("records_confirmation_for_future_tool_calls"),
            PermissionJudge::default(),
        )
        .expect("permission system should open");
        let tool_call = ToolCall::new("terminal", "cargo test");

        permissions
            .record_confirmation(&tool_call, PermissionConfirmationAction::AlwaysAllow, None)
            .expect("confirmation should persist");

        let decision = permissions
            .check_tool_call("terminal", "cargo test", 10)
            .expect("permission check should succeed");

        assert_eq!(decision, PermissionDecision::Allowed);
    }
}
