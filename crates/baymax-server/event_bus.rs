use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SessionEvent {
    MessageAdded { message: String },
    ToolCalled { tool_name: String },
    ToolCompleted { tool_name: String, result: String },
    StatusChanged { old: String, new: String },
}

#[derive(Debug, Clone, Default)]
pub struct SessionEventBus {
    events: Arc<Mutex<HashMap<String, Vec<SessionEvent>>>>,
}

impl SessionEventBus {
    pub fn publish(&self, session_id: &str, event: SessionEvent) {
        if let Ok(mut events) = self.events.lock() {
            events
                .entry(session_id.to_string())
                .or_default()
                .push(event);
        }
    }

    pub fn events_for(&self, session_id: &str) -> Vec<SessionEvent> {
        self.events
            .lock()
            .ok()
            .and_then(|events| events.get(session_id).cloned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_events_by_session() {
        let bus = SessionEventBus::default();
        bus.publish(
            "one",
            SessionEvent::MessageAdded {
                message: "hello".into(),
            },
        );

        assert_eq!(bus.events_for("one").len(), 1);
        assert!(bus.events_for("two").is_empty());
    }
}
