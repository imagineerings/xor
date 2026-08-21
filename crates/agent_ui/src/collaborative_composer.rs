use std::{error::Error, fmt};

use gpui::{App, Entity, EntityId};
use workspace::{
    Workspace,
    collaborative_composer::{
        CollaborativeComposerActionError, CollaborativeComposerProvider,
        CollaborativeComposerRegistration, CollaborativeComposerRegistrationError,
    },
};

use crate::{AgentPanel, message_editor::MessageEditorEvent};

pub struct CollaborativeComposerAdapter {
    thread_view_id: EntityId,
    provider: CollaborativeComposerProvider,
}

impl CollaborativeComposerAdapter {
    pub fn from_agent_panel(
        agent_panel: &Entity<AgentPanel>,
        workspace: &Entity<Workspace>,
        cx: &App,
    ) -> Result<Self, CollaborativeComposerAdapterError> {
        let thread_view = agent_panel
            .read(cx)
            .active_thread_view(cx)
            .ok_or(CollaborativeComposerAdapterError::ThreadUnavailable)?;
        let (project, message_editor) = thread_view.read_with(cx, |thread_view, _| {
            (
                thread_view.project.upgrade(),
                thread_view.message_editor.clone(),
            )
        });
        let project = project.ok_or(CollaborativeComposerAdapterError::ProjectUnavailable)?;
        if project.entity_id() != workspace.read(cx).project().entity_id() {
            return Err(CollaborativeComposerAdapterError::ProjectMismatch);
        }

        let submit_editor = message_editor.downgrade();
        let cancel_editor = message_editor.downgrade();
        let provider = CollaborativeComposerProvider::new(
            project,
            message_editor.into(),
            move |cx| {
                submit_editor
                    .update(cx, |message_editor, cx| {
                        if message_editor.is_empty(cx) {
                            return Err(CollaborativeComposerActionError::EmptyInput);
                        }
                        message_editor.send(cx);
                        Ok(())
                    })
                    .map_err(|_| CollaborativeComposerActionError::ThreadUnavailable)?
            },
            move |cx| {
                cancel_editor
                    .update(cx, |_, cx| cx.emit(MessageEditorEvent::Cancel))
                    .map_err(|_| CollaborativeComposerActionError::ThreadUnavailable)
            },
        );
        Ok(Self {
            thread_view_id: thread_view.entity_id(),
            provider,
        })
    }

    pub fn thread_view_id(&self) -> EntityId {
        self.thread_view_id
    }

    pub fn register_in_workspace(
        self,
        workspace: &mut Workspace,
        cx: &mut gpui::Context<Workspace>,
    ) -> Result<CollaborativeComposerRegistration, CollaborativeComposerAdapterError> {
        workspace
            .register_collaborative_composer_provider(self.provider, cx)
            .map_err(CollaborativeComposerAdapterError::Registration)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollaborativeComposerAdapterError {
    ThreadUnavailable,
    ProjectUnavailable,
    ProjectMismatch,
    Registration(CollaborativeComposerRegistrationError),
}

impl fmt::Display for CollaborativeComposerAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThreadUnavailable => formatter.write_str("active agent thread is unavailable"),
            Self::ProjectUnavailable => formatter.write_str("active agent project is unavailable"),
            Self::ProjectMismatch => {
                formatter.write_str("active agent thread belongs to a different project")
            }
            Self::Registration(error) => write!(formatter, "composer registration failed: {error}"),
        }
    }
}

impl Error for CollaborativeComposerAdapterError {}
