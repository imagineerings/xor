use crate::{
    DecisionType, PermissionDecision, PermissionJudge, PermissionStore, RiskLevel, ToolCall,
};
use anyhow::Result;

pub struct PermissionInspector<'a> {
    store: &'a PermissionStore,
    judge: PermissionJudge,
}

impl<'a> PermissionInspector<'a> {
    pub fn new(store: &'a PermissionStore, judge: PermissionJudge) -> Self {
        Self { store, judge }
    }

    pub fn check_tool_call(&self, tool_call: &ToolCall, now: i64) -> Result<PermissionDecision> {
        if let Some(stored) =
            self.store
                .find_decision_for_args(&tool_call.tool_name, &tool_call.arguments, now)?
        {
            let decision = match stored.decision_type {
                DecisionType::AlwaysAllow | DecisionType::AllowOnce => PermissionDecision::Allowed,
                DecisionType::AlwaysDeny | DecisionType::DenyOnce => PermissionDecision::Denied {
                    reason: format!(
                        "stored permission decision {:?} matched {}",
                        stored.decision_type, stored.args_pattern
                    ),
                    risk_level: RiskLevel::High,
                },
            };

            if matches!(
                stored.decision_type,
                DecisionType::AllowOnce | DecisionType::DenyOnce
            ) {
                self.store
                    .delete_decision(&stored.tool_name, &stored.args_pattern)?;
            }

            Ok(decision)
        } else {
            Ok(self.judge.judge(tool_call))
        }
    }

    pub fn judge(&self) -> &PermissionJudge {
        &self.judge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StoredDecision, store::PermissionStore};

    #[test]
    fn uses_stored_always_allow_decision() {
        let store = PermissionStore::open_memory(Some("uses_stored_always_allow_decision"))
            .expect("store should open");
        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "rm -rf target".into(),
                decision_type: DecisionType::AlwaysAllow,
                created_at: 1,
                expires_at: None,
            })
            .expect("decision should write");
        let inspector = PermissionInspector::new(&store, PermissionJudge::default());

        let decision = inspector
            .check_tool_call(&ToolCall::new("terminal", "rm -rf target"), 10)
            .expect("inspection should succeed");

        assert_eq!(decision, PermissionDecision::Allowed);
    }

    #[test]
    fn uses_stored_always_deny_decision() {
        let store = PermissionStore::open_memory(Some("uses_stored_always_deny_decision"))
            .expect("store should open");
        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "cargo *".into(),
                decision_type: DecisionType::AlwaysDeny,
                created_at: 1,
                expires_at: None,
            })
            .expect("decision should write");
        let inspector = PermissionInspector::new(&store, PermissionJudge::default());

        let decision = inspector
            .check_tool_call(&ToolCall::new("terminal", "cargo test"), 10)
            .expect("inspection should succeed");

        assert!(matches!(
            decision,
            PermissionDecision::Denied {
                risk_level: RiskLevel::High,
                ..
            }
        ));
    }

    #[test]
    fn consumes_once_decisions() {
        let store = PermissionStore::open_memory(Some("consumes_once_decisions"))
            .expect("store should open");
        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "cargo test".into(),
                decision_type: DecisionType::AllowOnce,
                created_at: 1,
                expires_at: None,
            })
            .expect("decision should write");
        let inspector = PermissionInspector::new(&store, PermissionJudge::default());

        assert_eq!(
            inspector
                .check_tool_call(&ToolCall::new("terminal", "cargo test"), 10)
                .expect("inspection should succeed"),
            PermissionDecision::Allowed
        );
        assert!(
            store
                .get_decision("terminal", "cargo test")
                .expect("decision should read")
                .is_none()
        );
    }

    #[test]
    fn expired_decision_falls_back_to_judge() {
        let store = PermissionStore::open_memory(Some("expired_decision_falls_back_to_judge"))
            .expect("store should open");
        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "rm -rf target".into(),
                decision_type: DecisionType::AlwaysAllow,
                created_at: 1,
                expires_at: Some(5),
            })
            .expect("decision should write");
        let inspector = PermissionInspector::new(&store, PermissionJudge::default());

        let decision = inspector
            .check_tool_call(&ToolCall::new("terminal", "rm -rf target"), 10)
            .expect("inspection should succeed");

        assert!(matches!(
            decision,
            PermissionDecision::Denied {
                risk_level: RiskLevel::High,
                ..
            }
        ));
    }
}
