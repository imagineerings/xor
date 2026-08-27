use std::{error::Error, fmt};

use gpui::{App, Entity, EntityId, Focusable};
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
        let focus_handle = message_editor.read(cx).focus_handle(cx);

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
        )
        .with_focus_handle(focus_handle);
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

#[cfg(test)]
mod tests {
    use acp_thread::{StubAgentConnection, ThreadStatus};
    use fs::FakeFs;
    use gpui::{AppContext as _, TestAppContext, VisualTestContext, px, size};
    use project::Project;
    use workspace::MultiWorkspace;

    use crate::thread_metadata_store::ThreadMetadataStore;

    use super::*;

    #[gpui::test]
    async fn collaborative_composer_routes_submit_and_cancel_to_the_active_acp_thread(
        cx: &mut TestAppContext,
    ) {
        crate::test_support::init_test(cx);
        cx.update(|cx| {
            agent::ThreadStore::init_global(cx);
            language_model::LanguageModelRegistry::test(cx);
            ThreadMetadataStore::init_global(cx);
        });

        let project = Project::test(FakeFs::new(cx.executor()), [], cx).await;
        let multi_workspace =
            cx.add_window(|window, cx| MultiWorkspace::test_new(project, window, cx));
        let workspace = multi_workspace
            .read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone())
            .expect("test workspace should exist");
        let cx = &mut VisualTestContext::from_window(multi_workspace.into(), cx);
        cx.simulate_resize(size(px(900.), px(700.)));

        let panel = workspace.update_in(cx, |workspace, window, cx| {
            let panel = cx.new(|cx| AgentPanel::new(workspace, window, cx));
            workspace.add_panel(panel.clone(), window, cx);
            workspace.focus_panel::<AgentPanel>(window, cx);
            panel
        });
        crate::test_support::open_thread_with_connection(&panel, StubAgentConnection::new(), cx);

        let adapter = cx
            .update(|_, cx| CollaborativeComposerAdapter::from_agent_panel(&panel, &workspace, cx));
        let adapter = adapter.expect("active ACP composer should adapt");
        workspace
            .update(cx, |workspace, cx| {
                adapter.register_in_workspace(workspace, cx)
            })
            .expect("active ACP composer should register");
        assert_eq!(
            workspace.update(cx, |workspace, cx| {
                workspace.submit_collaborative_composer(cx)
            }),
            Err(CollaborativeComposerActionError::EmptyInput)
        );

        let thread_view = panel.read_with(cx, |panel, cx| {
            panel
                .active_thread_view(cx)
                .expect("active thread view should exist")
        });
        let message_editor =
            thread_view.read_with(cx, |thread_view, _| thread_view.message_editor.clone());
        let message_editor_focus =
            message_editor.read_with(cx, |message_editor, cx| message_editor.focus_handle(cx));
        cx.dispatch_action(workspace::SwitchToCollaborativeWorkspace);
        cx.run_until_parked();
        cx.dispatch_action(workspace::collaborative_focus::RestoreCollaborativeFocus);
        cx.dispatch_action(workspace::collaborative_focus::FocusNextCollaborativeRegion);
        cx.run_until_parked();
        assert!(cx.update(|window, _| message_editor_focus.is_focused(window)));

        message_editor.update_in(cx, |message_editor, window, cx| {
            message_editor.set_text("Review the project", window, cx);
        });
        workspace
            .update(cx, |workspace, cx| {
                workspace.submit_collaborative_composer(cx)
            })
            .expect("composer submit should reach the active message editor");
        cx.run_until_parked();

        let thread = panel.read_with(cx, |panel, cx| {
            panel
                .active_agent_thread(cx)
                .expect("active ACP thread should exist")
        });
        assert_eq!(
            thread.read_with(cx, |thread, _| thread.status()),
            ThreadStatus::Generating
        );
        assert_eq!(thread.read_with(cx, |thread, _| thread.entries().len()), 1);

        workspace
            .update(cx, |workspace, cx| {
                workspace.cancel_collaborative_composer(cx)
            })
            .expect("composer cancel should reach the active message editor");
        cx.run_until_parked();
        assert_eq!(
            thread.read_with(cx, |thread, _| thread.status()),
            ThreadStatus::Idle
        );
    }
}
