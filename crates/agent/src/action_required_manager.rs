use collections::HashMap;
use gpui::SharedString;
use std::time::Instant;

pub type ActionRequiredId = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionRequiredKind {
    Permission,
    Confirmation,
    Input,
    Other(SharedString),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionRequiredStatus {
    Pending,
    Resolved,
    Canceled,
}

#[derive(Clone, Debug)]
pub struct ActionRequired {
    pub id: ActionRequiredId,
    pub kind: ActionRequiredKind,
    pub title: SharedString,
    pub message: Option<SharedString>,
    pub status: ActionRequiredStatus,
    pub created_at: Instant,
    pub completed_at: Option<Instant>,
}

#[derive(Default)]
pub struct ActionRequiredManager {
    next_id: ActionRequiredId,
    actions: HashMap<ActionRequiredId, ActionRequired>,
}

impl ActionRequiredManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &mut self,
        kind: ActionRequiredKind,
        title: impl Into<SharedString>,
        message: Option<impl Into<SharedString>>,
    ) -> ActionRequiredId {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.actions.insert(
            id,
            ActionRequired {
                id,
                kind,
                title: title.into(),
                message: message.map(Into::into),
                status: ActionRequiredStatus::Pending,
                created_at: Instant::now(),
                completed_at: None,
            },
        );
        id
    }

    pub fn get(&self, id: ActionRequiredId) -> Option<&ActionRequired> {
        self.actions.get(&id)
    }

    pub fn pending(&self) -> impl Iterator<Item = &ActionRequired> {
        self.actions
            .values()
            .filter(|action| action.status == ActionRequiredStatus::Pending)
    }

    pub fn has_pending(&self) -> bool {
        self.pending().next().is_some()
    }

    pub fn resolve(&mut self, id: ActionRequiredId) -> bool {
        self.complete(id, ActionRequiredStatus::Resolved)
    }

    pub fn cancel(&mut self, id: ActionRequiredId) -> bool {
        self.complete(id, ActionRequiredStatus::Canceled)
    }

    pub fn remove(&mut self, id: ActionRequiredId) -> Option<ActionRequired> {
        self.actions.remove(&id)
    }

    fn complete(&mut self, id: ActionRequiredId, status: ActionRequiredStatus) -> bool {
        let Some(action) = self.actions.get_mut(&id) else {
            return false;
        };
        if action.status != ActionRequiredStatus::Pending {
            return false;
        }
        action.status = status;
        action.completed_at = Some(Instant::now());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_required_manager_tracks_pending_actions() {
        let mut manager = ActionRequiredManager::new();

        let id = manager.add(
            ActionRequiredKind::Permission,
            "Approve command",
            Some("Run tests"),
        );

        assert!(manager.has_pending());
        assert_eq!(manager.pending().count(), 1);
        let action = manager.get(id).unwrap();
        assert_eq!(action.kind, ActionRequiredKind::Permission);
        assert_eq!(action.title.as_ref(), "Approve command");
        assert_eq!(action.message.as_deref(), Some("Run tests"));
    }

    #[test]
    fn test_action_required_manager_resolves_once() {
        let mut manager = ActionRequiredManager::new();
        let id = manager.add(ActionRequiredKind::Confirmation, "Confirm", None::<&str>);

        assert!(manager.resolve(id));
        assert!(!manager.resolve(id));
        assert!(!manager.has_pending());
        let action = manager.get(id).unwrap();
        assert_eq!(action.status, ActionRequiredStatus::Resolved);
        assert!(action.completed_at.is_some());
    }

    #[test]
    fn test_action_required_manager_cancels_and_removes() {
        let mut manager = ActionRequiredManager::new();
        let id = manager.add(ActionRequiredKind::Input, "Input", None::<&str>);

        assert!(manager.cancel(id));
        assert_eq!(
            manager.remove(id).unwrap().status,
            ActionRequiredStatus::Canceled
        );
        assert!(manager.get(id).is_none());
    }
}
