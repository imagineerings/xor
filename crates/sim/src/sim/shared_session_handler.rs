use std::sync::Arc;

use agent::{SharedSessionData, ThreadStore};
use agent_client_protocol::schema as acp;
use agent_ui::AgentPanel;
use anyhow::{Context as _, Result};
use gpui::{AsyncApp, Entity, Focusable};
use uuid::Uuid;
use workspace::{AppState, Toast, notifications::NotificationId};

pub async fn import_shared_session_from_link(
    app_state: Arc<AppState>,
    data: String,
    cx: &mut AsyncApp,
) -> Result<()> {
    let multi_workspace =
        workspace::get_any_active_multi_workspace(app_state.clone(), cx.clone()).await?;

    let workspace =
        multi_workspace.read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone())?;

    let import_state = multi_workspace.update(cx, |_, window, cx| {
        workspace.update(cx, |workspace, cx| {
            if workspace.root_paths(cx).is_empty() {
                workspace.focus_panel::<AgentPanel>(window, cx);

                struct OpenProjectForSharedSessionToast;
                workspace.show_toast(
                    Toast::new(
                        NotificationId::unique::<OpenProjectForSharedSessionToast>(),
                        "Open a project to import shared sessions",
                    )
                    .autohide(),
                    cx,
                );

                return anyhow::Ok(None);
            }

            let thread_store: Option<Entity<ThreadStore>> = workspace
                .panel::<AgentPanel>(cx)
                .map(|panel| panel.read(cx).thread_store().clone());
            anyhow::Ok(Some(thread_store))
        })
    })??;

    let Some(thread_store) = import_state else {
        return Ok(());
    };

    let Some(thread_store) = thread_store else {
        anyhow::bail!("Agent panel not available");
    };

    let shared_session =
        SharedSessionData::from_share_code(&data).context("Failed to parse shared session link")?;
    let db_thread = shared_session.to_db_thread();
    let title = db_thread.title.clone();
    let session_id = acp::SessionId::new(Uuid::new_v4().to_string());
    let save_session_id = session_id.clone();

    thread_store
        .update(&mut cx.clone(), |store, cx| {
            store.save_thread(save_session_id.clone(), db_thread, Default::default(), cx)
        })
        .await?;

    multi_workspace.update(cx, |_, window, cx| {
        workspace.update(cx, |workspace, cx| {
            if let Some(panel) = workspace.panel::<AgentPanel>(cx) {
                panel.update(cx, |panel, cx| {
                    panel.open_thread(session_id.clone(), None, Some(title.clone()), window, cx);
                });
                panel.focus_handle(cx).focus(window, cx);
            }

            struct ImportedSharedSessionToast;
            workspace.show_toast(
                Toast::new(
                    NotificationId::unique::<ImportedSharedSessionToast>(),
                    "Imported shared session",
                )
                .autohide(),
                cx,
            );
        });
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use agent::{DbThread, SharedSessionData};
    use chrono::{TimeZone, Utc};

    fn make_thread() -> DbThread {
        DbThread {
            title: "Imported".into(),
            messages: Vec::new(),
            updated_at: Utc.with_ymd_and_hms(2026, 7, 8, 12, 0, 0).unwrap(),
            detailed_summary: None,
            initial_project_snapshot: None,
            cumulative_token_usage: Default::default(),
            request_token_usage: Default::default(),
            model: None,
            profile: None,
            imported: false,
            subagent_context: None,
            speed: None,
            thinking_enabled: false,
            thinking_effort: None,
            draft_prompt: None,
            ui_scroll_position: None,
            sandboxed_terminal_temp_dir: None,
        }
    }

    #[test]
    fn shared_session_link_payload_decodes_to_imported_thread() {
        let data = SharedSessionData::from_db_thread(
            &make_thread(),
            Utc.with_ymd_and_hms(2026, 7, 8, 12, 30, 0).unwrap(),
        );
        let code = data.to_share_code().expect("encode shared session");

        let decoded = SharedSessionData::from_share_code(&code).expect("decode shared session");
        let thread = decoded.to_db_thread();

        assert!(thread.imported);
        assert_eq!(thread.title.as_ref(), "🔗 Imported");
    }
}
