use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use language_model::{LanguageModelToolResult, LanguageModelToolUse, MessageContent, Role};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub conversation: Vec<SnapshotMessage>,
    pub tool_state: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMessage {
    pub role: Role,
    pub content: Vec<MessageContent>,
    #[serde(default)]
    pub tool_uses: Vec<LanguageModelToolUse>,
    #[serde(default)]
    pub tool_results: Vec<LanguageModelToolResult>,
}

impl AgentSnapshot {
    pub fn new(conversation: Vec<SnapshotMessage>) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            conversation,
            tool_state: BTreeMap::new(),
        }
    }

    pub fn with_tool_state(mut self, key: impl Into<String>, value: Value) -> Self {
        self.tool_state.insert(key.into(), value);
        self
    }

    pub fn set_tool_state(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.tool_state.insert(key.into(), value)
    }

    pub fn tool_state(&self, key: &str) -> Option<&Value> {
        self.tool_state.get(key)
    }

    pub fn restore(self) -> RestoredAgentSnapshot {
        RestoredAgentSnapshot {
            conversation: self.conversation,
            tool_state: self.tool_state,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RestoredAgentSnapshot {
    pub conversation: Vec<SnapshotMessage>,
    pub tool_state: BTreeMap<String, Value>,
}

impl SnapshotMessage {
    pub fn new(role: Role, content: Vec<MessageContent>) -> Self {
        Self {
            role,
            content,
            tool_uses: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    pub fn with_tool_uses(mut self, tool_uses: Vec<LanguageModelToolUse>) -> Self {
        self.tool_uses = tool_uses;
        self
    }

    pub fn with_tool_results(mut self, tool_results: Vec<LanguageModelToolResult>) -> Self {
        self.tool_results = tool_results;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use language_model::MessageContent;
    use pretty_assertions::assert_eq;

    #[test]
    fn snapshot_round_trips_conversation_and_tool_state() {
        let snapshot = AgentSnapshot::new(vec![SnapshotMessage::new(
            Role::User,
            vec![MessageContent::Text("hello".into())],
        )])
        .with_tool_state("todo", serde_json::json!({ "items": ["one"] }));

        let serialized = serde_json::to_string(&snapshot).expect("serialize snapshot");
        let deserialized =
            serde_json::from_str::<AgentSnapshot>(&serialized).expect("deserialize snapshot");
        let restored = deserialized.restore();

        assert_eq!(restored.conversation.len(), 1);
        assert_eq!(
            restored.tool_state.get("todo"),
            Some(&serde_json::json!({ "items": ["one"] }))
        );
    }
}
