use std::{error::Error, fmt};

use acp_thread::ThreadStatus;
use gpui::{App, Entity, EntityId, SharedString, SharedUri};
use project::Project;
use workspace::{
    Workspace,
    collaborative_participants::{
        CollaborativeExecutionLocation, CollaborativeExecutionPhase, CollaborativeExecutionStatus,
        CollaborativeParticipant, CollaborativeParticipantPresence,
        CollaborativeParticipantProvider, CollaborativeParticipantProviderState,
        CollaborativeParticipantViewData,
    },
};

use crate::{Agent, AgentPanel, thread_metadata_store::ThreadMetadataStore};

pub struct CollaborativeParticipantAdapter {
    project: Entity<Project>,
    thread_view_id: EntityId,
    view_data: CollaborativeParticipantViewData,
}

impl CollaborativeParticipantAdapter {
    pub fn from_agent_panel(
        agent_panel: &Entity<AgentPanel>,
        workspace: &Entity<Workspace>,
        cx: &App,
    ) -> Result<Self, CollaborativeParticipantAdapterError> {
        let (thread_view, active_thread_id) = agent_panel.read_with(cx, |agent_panel, cx| {
            (
                agent_panel.active_thread_view(cx),
                agent_panel.active_thread_id(cx),
            )
        });
        let thread_view =
            thread_view.ok_or(CollaborativeParticipantAdapterError::ThreadUnavailable)?;
        let project = thread_view
            .read(cx)
            .project
            .upgrade()
            .ok_or(CollaborativeParticipantAdapterError::ProjectUnavailable)?;
        if project.entity_id() != workspace.read(cx).project().entity_id() {
            return Err(CollaborativeParticipantAdapterError::ProjectMismatch);
        }

        let location = active_thread_id
            .and_then(|thread_id| {
                ThreadMetadataStore::try_global(cx).and_then(|store| {
                    store
                        .read(cx)
                        .entry(thread_id)
                        .map(|metadata| metadata.remote_connection.clone())
                })
            })
            .map(|remote_connection| match remote_connection {
                Some(remote_connection) => CollaborativeExecutionLocation::Remote(Some(
                    remote_connection.display_name().into(),
                )),
                None => CollaborativeExecutionLocation::Local,
            })
            .unwrap_or(CollaborativeExecutionLocation::Unknown);

        let snapshot = thread_view.read_with(cx, |thread_view, cx| {
            let agent_id = thread_view.agent_id.clone();
            let agent_server_store = project.read(cx).agent_server_store().clone();
            let display_name = agent_server_store
                .read(cx)
                .agent_display_name(&agent_id)
                .unwrap_or_else(|| Agent::from(agent_id.clone()).label());
            let avatar_uri = thread_view
                .agent_icon_from_external_svg
                .as_ref()
                .filter(|icon| icon.starts_with("https://") || icon.starts_with("http://"))
                .cloned()
                .map(SharedUri::from);
            let model = thread_view.current_model_id(cx).map(SharedString::from);
            let runtime = if thread_view.as_native_connection(cx).is_some() {
                "Zed Agent".into()
            } else {
                "ACP".into()
            };
            let thread = thread_view.thread.read(cx);
            let phase = if thread.is_waiting_for_confirmation() {
                CollaborativeExecutionPhase::WaitingForUser
            } else if thread.status() == ThreadStatus::Generating {
                CollaborativeExecutionPhase::Running
            } else if thread.had_error() {
                CollaborativeExecutionPhase::Failed
            } else {
                CollaborativeExecutionPhase::Idle
            };
            CollaborativeParticipantSnapshot {
                agent_id: agent_id.0,
                display_name,
                avatar_uri,
                model,
                runtime,
                location,
                phase,
            }
        });
        Self::from_snapshot(project, thread_view.entity_id(), Some(snapshot))
    }

    fn from_snapshot(
        project: Entity<Project>,
        thread_view_id: EntityId,
        snapshot: Option<CollaborativeParticipantSnapshot>,
    ) -> Result<Self, CollaborativeParticipantAdapterError> {
        let snapshot = snapshot.ok_or(CollaborativeParticipantAdapterError::ThreadUnavailable)?;
        let participant = CollaborativeParticipant::agent(
            snapshot.agent_id,
            snapshot.display_name,
            snapshot.avatar_uri,
            CollaborativeParticipantPresence::Online,
        );
        let execution = CollaborativeExecutionStatus {
            phase: snapshot.phase,
            model: snapshot.model,
            runtime: Some(snapshot.runtime),
            location: snapshot.location,
        };
        Ok(Self {
            project,
            thread_view_id,
            view_data: CollaborativeParticipantViewData {
                participants: vec![participant],
                execution: Some(execution),
            },
        })
    }

    pub fn thread_view_id(&self) -> EntityId {
        self.thread_view_id
    }

    pub fn view_data(&self) -> &CollaborativeParticipantViewData {
        &self.view_data
    }

    pub fn into_provider(self) -> CollaborativeParticipantProvider {
        CollaborativeParticipantProvider::new(
            self.project,
            self.thread_view_id,
            CollaborativeParticipantProviderState::Ready(self.view_data),
        )
    }
}

struct CollaborativeParticipantSnapshot {
    agent_id: SharedString,
    display_name: SharedString,
    avatar_uri: Option<SharedUri>,
    model: Option<SharedString>,
    runtime: SharedString,
    location: CollaborativeExecutionLocation,
    phase: CollaborativeExecutionPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeParticipantAdapterError {
    ThreadUnavailable,
    ProjectUnavailable,
    ProjectMismatch,
}

impl fmt::Display for CollaborativeParticipantAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThreadUnavailable => formatter.write_str("active agent thread is unavailable"),
            Self::ProjectUnavailable => formatter.write_str("active agent project is unavailable"),
            Self::ProjectMismatch => {
                formatter.write_str("active agent thread belongs to a different project")
            }
        }
    }
}

impl Error for CollaborativeParticipantAdapterError {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fs::FakeFs;
    use gpui::{AppContext as _, Empty, TestAppContext};
    use settings::SettingsStore;
    use workspace::collaborative_participants::CollaborativeParticipantIdentity;

    use super::*;

    fn snapshot(
        agent_id: &str,
        model: Option<&str>,
        location: CollaborativeExecutionLocation,
    ) -> CollaborativeParticipantSnapshot {
        CollaborativeParticipantSnapshot {
            agent_id: agent_id.to_owned().into(),
            display_name: "Review Agent".into(),
            avatar_uri: Some("https://example.test/agent.png".into()),
            model: model.map(SharedString::from),
            runtime: "ACP".into(),
            location,
            phase: CollaborativeExecutionPhase::Running,
        }
    }

    #[gpui::test]
    async fn collaborative_participant_adapter(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            cx.set_global(db::AppDatabase::test_new());
            theme_settings::init(theme::LoadThemes::JustBase, cx);
        });
        let file_system = FakeFs::new(cx.executor());
        let project = Project::test(file_system, [Path::new("/project")], cx).await;
        let first_thread = cx.new(|_| Empty);
        let changed_thread = cx.new(|_| Empty);

        assert!(matches!(
            CollaborativeParticipantAdapter::from_snapshot(
                project.clone(),
                first_thread.entity_id(),
                None
            ),
            Err(CollaborativeParticipantAdapterError::ThreadUnavailable)
        ));

        let local = CollaborativeParticipantAdapter::from_snapshot(
            project.clone(),
            first_thread.entity_id(),
            Some(snapshot(
                "agent:reviewer",
                None,
                CollaborativeExecutionLocation::Local,
            )),
        )
        .expect("active local thread should adapt");
        let changed = CollaborativeParticipantAdapter::from_snapshot(
            project,
            changed_thread.entity_id(),
            Some(snapshot(
                "agent:reviewer",
                Some("claude-sonnet"),
                CollaborativeExecutionLocation::Remote(Some("build-host".into())),
            )),
        )
        .expect("changed remote thread should adapt");

        assert_eq!(local.thread_view_id(), first_thread.entity_id());
        assert_eq!(changed.thread_view_id(), changed_thread.entity_id());
        assert_ne!(local.thread_view_id(), changed.thread_view_id());
        let local_participant = local
            .view_data()
            .participants
            .first()
            .expect("local agent participant should exist");
        let changed_participant = changed
            .view_data()
            .participants
            .first()
            .expect("changed agent participant should exist");
        assert_eq!(
            local_participant.identity,
            CollaborativeParticipantIdentity::Agent("agent:reviewer".into())
        );
        assert_eq!(local_participant.avatar_uri, changed_participant.avatar_uri);

        let local_execution = local
            .view_data()
            .execution
            .as_ref()
            .expect("local execution should exist");
        assert_eq!(local_execution.model_label().as_ref(), "Unknown model");
        assert_eq!(local_execution.runtime_label().as_ref(), "ACP");
        assert_eq!(local_execution.location_label().as_ref(), "Local");
        let changed_execution = changed
            .view_data()
            .execution
            .as_ref()
            .expect("changed execution should exist");
        assert_eq!(changed_execution.model_label().as_ref(), "claude-sonnet");
        assert_eq!(
            changed_execution.location_label().as_ref(),
            "Remote · build-host"
        );
    }
}
