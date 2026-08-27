use anyhow::Result;
use db::kvp::KeyValueStore;
use serde::{Deserialize, Serialize};
use util::ResultExt;

use crate::WorkspaceId;

const COLLABORATIVE_LAYOUT_NAMESPACE: &str = "collaborative_workspace_layout";
const COLLABORATIVE_LAYOUT_VERSION: u32 = 4;

pub(crate) const DEFAULT_RAIL_WIDTH: f32 = 226.;
pub(crate) const DEFAULT_REVIEW_WIDTH: f32 = 354.5;
pub(crate) const MIN_RAIL_WIDTH: f32 = 180.;
pub(crate) const MAX_RAIL_WIDTH: f32 = 400.;
pub(crate) const MIN_REVIEW_WIDTH: f32 = 280.;
pub(crate) const MAX_REVIEW_WIDTH: f32 = 800.;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct CollaborativeLayoutState {
    #[serde(default = "collaborative_layout_version")]
    version: u32,
    #[serde(default = "default_review_requested")]
    review_requested: bool,
    #[serde(default = "default_review_width")]
    review_width: f32,
    #[serde(default = "default_rail_width")]
    rail_width: f32,
}

impl Default for CollaborativeLayoutState {
    fn default() -> Self {
        Self {
            version: COLLABORATIVE_LAYOUT_VERSION,
            review_requested: true,
            review_width: DEFAULT_REVIEW_WIDTH,
            rail_width: DEFAULT_RAIL_WIDTH,
        }
    }
}

impl CollaborativeLayoutState {
    pub(crate) fn review_requested(self) -> bool {
        self.review_requested
    }

    pub(crate) fn review_width(self) -> f32 {
        self.review_width
    }

    pub(crate) fn rail_width(self) -> f32 {
        self.rail_width
    }

    pub(crate) fn with_review_requested(mut self, review_requested: bool) -> Self {
        self.review_requested = review_requested;
        self
    }

    pub(crate) fn with_review_width(mut self, review_width: f32) -> Self {
        self.review_width = review_width;
        self.normalized()
    }

    pub(crate) fn with_rail_width(mut self, rail_width: f32) -> Self {
        self.rail_width = rail_width;
        self.normalized()
    }

    pub(crate) fn reset_rail_width(mut self) -> Self {
        self.rail_width = DEFAULT_RAIL_WIDTH;
        self
    }

    fn normalized(self) -> Self {
        if self.version != COLLABORATIVE_LAYOUT_VERSION {
            return Self::default();
        }

        Self {
            version: COLLABORATIVE_LAYOUT_VERSION,
            review_requested: self.review_requested,
            review_width: finite_or_default(self.review_width, DEFAULT_REVIEW_WIDTH)
                .clamp(MIN_REVIEW_WIDTH, MAX_REVIEW_WIDTH),
            rail_width: finite_or_default(self.rail_width, DEFAULT_RAIL_WIDTH)
                .clamp(MIN_RAIL_WIDTH, MAX_RAIL_WIDTH),
        }
    }
}

pub(crate) fn read_collaborative_layout_state(
    key_value_store: &KeyValueStore,
    workspace_id: WorkspaceId,
) -> CollaborativeLayoutState {
    key_value_store
        .scoped(COLLABORATIVE_LAYOUT_NAMESPACE)
        .read(&workspace_id.0.to_string())
        .log_err()
        .flatten()
        .and_then(|serialized| {
            serde_json::from_str::<CollaborativeLayoutState>(&serialized).log_err()
        })
        .map(CollaborativeLayoutState::normalized)
        .unwrap_or_default()
}

pub(crate) async fn write_collaborative_layout_state(
    key_value_store: &KeyValueStore,
    workspace_id: WorkspaceId,
    state: CollaborativeLayoutState,
) -> Result<()> {
    let serialized = serde_json::to_string(&state.normalized())?;
    key_value_store
        .scoped(COLLABORATIVE_LAYOUT_NAMESPACE)
        .write(workspace_id.0.to_string(), serialized)
        .await
}

fn collaborative_layout_version() -> u32 {
    COLLABORATIVE_LAYOUT_VERSION
}

fn default_review_requested() -> bool {
    true
}

fn default_review_width() -> f32 {
    DEFAULT_REVIEW_WIDTH
}

fn default_rail_width() -> f32 {
    DEFAULT_RAIL_WIDTH
}

fn finite_or_default(value: f32, default: f32) -> f32 {
    if value.is_finite() { value } else { default }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::init_test;

    #[gpui::test]
    async fn collaborative_layout_restart(cx: &mut gpui::TestAppContext) {
        init_test(cx);

        let key_value_store = cx.update(|cx| KeyValueStore::global(cx));
        let editor_state_key = "editor-layout-window";
        let editor_state = r#"{"sidebar":{"width":517.0},"left_dock_visible":false}"#;
        key_value_store
            .scoped("multi_workspace_state")
            .write(editor_state_key.to_owned(), editor_state.to_owned())
            .await
            .expect("editor layout fixture should write");

        let workspace_id = WorkspaceId(7001);
        let requested = CollaborativeLayoutState::default()
            .with_review_requested(false)
            .with_review_width(2500.)
            .with_rail_width(120.);
        write_collaborative_layout_state(&key_value_store, workspace_id, requested)
            .await
            .expect("collaborative layout should write");

        let restored = read_collaborative_layout_state(&key_value_store, workspace_id);
        assert!(!restored.review_requested());
        assert_eq!(restored.review_width(), MAX_REVIEW_WIDTH);
        assert_eq!(restored.rail_width(), MIN_RAIL_WIDTH);
        assert_eq!(
            key_value_store
                .scoped("multi_workspace_state")
                .read(editor_state_key)
                .expect("editor layout fixture should read"),
            Some(editor_state.to_owned())
        );

        let other_workspace_id = WorkspaceId(7002);
        assert_eq!(
            read_collaborative_layout_state(&key_value_store, other_workspace_id),
            CollaborativeLayoutState::default()
        );

        key_value_store
            .scoped(COLLABORATIVE_LAYOUT_NAMESPACE)
            .write(
                other_workspace_id.0.to_string(),
                r#"{"version":999,"review_requested":false,"review_width":1000,"rail_width":700}"#
                    .to_owned(),
            )
            .await
            .expect("future-version fixture should write");
        assert_eq!(
            read_collaborative_layout_state(&key_value_store, other_workspace_id),
            CollaborativeLayoutState::default()
        );

        key_value_store
            .scoped(COLLABORATIVE_LAYOUT_NAMESPACE)
            .write(other_workspace_id.0.to_string(), "not-json".to_owned())
            .await
            .expect("malformed fixture should write");
        assert_eq!(
            read_collaborative_layout_state(&key_value_store, other_workspace_id),
            CollaborativeLayoutState::default()
        );
    }
}
