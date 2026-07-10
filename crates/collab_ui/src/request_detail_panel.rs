use crate::channel_join_requests::PendingJoinRequest;
use channel::ChannelStore;
use client::ChannelId;
use editor::Editor;
use gpui::{Context, Entity, EventEmitter, Render, SharedString, Window, prelude::*};
use rpc::proto;
use ui::{Button, Label, LabelSize, prelude::*};

pub struct RequestDetailPanel {
    channel_id: ChannelId,
    request: PendingJoinRequest,
    channel_store: Entity<ChannelStore>,
    denial_reason_editor: Option<Entity<Editor>>,
    response_error: Option<SharedString>,
    responding: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestDetailPanelEvent {
    Responded,
}

impl EventEmitter<RequestDetailPanelEvent> for RequestDetailPanel {}

impl RequestDetailPanel {
    pub fn new(
        channel_id: ChannelId,
        request: PendingJoinRequest,
        channel_store: Entity<ChannelStore>,
    ) -> Self {
        Self {
            channel_id,
            request,
            channel_store,
            denial_reason_editor: None,
            response_error: None,
            responding: false,
        }
    }

    fn show_denial_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.denial_reason_editor.is_none() {
            self.denial_reason_editor = Some(cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Optional reason", window, cx);
                editor
            }));
            cx.notify();
        }
    }

    fn respond(&mut self, approve: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.responding {
            return;
        }

        let denial_reason = (!approve)
            .then(|| {
                self.denial_reason_editor
                    .as_ref()
                    .map(|editor| editor.read(cx).text(cx).trim().to_string())
                    .filter(|reason| !reason.is_empty())
            })
            .flatten();
        self.responding = true;
        self.response_error = None;
        let client = self.channel_store.read(cx).client();
        let channel_id = self.channel_id;
        let requesting_user_id = self.request.user_id;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let result = client
                .request(proto::RespondToJoinRequest {
                    channel_id: channel_id.0,
                    requesting_user_id,
                    approve,
                    denial_reason,
                })
                .await;
            this.update_in(cx, |this, _, cx| {
                this.responding = false;
                match result {
                    Ok(_) => cx.emit(RequestDetailPanelEvent::Responded),
                    Err(error) => this.response_error = Some(SharedString::from(error.to_string())),
                }
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }
}

impl Render for RequestDetailPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let username = self.request.user.github_login.clone();
        let reason = self
            .request
            .reason
            .clone()
            .unwrap_or_else(|| "No reason provided".into());
        v_flex()
            .size_full()
            .gap_3()
            .child(Label::new(username).size(LabelSize::Large))
            .child(Label::new(self.request.created_at.to_string()).color(Color::Muted))
            .child(Label::new(reason))
            .when_some(self.denial_reason_editor.clone(), |this, editor| {
                this.child(editor)
            })
            .when_some(self.response_error.clone(), |this, error| {
                this.child(Label::new(error).color(Color::Error))
            })
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("approve-join-request", "Approve")
                            .disabled(self.responding)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.respond(true, window, cx);
                            })),
                    )
                    .child(
                        Button::new("deny-join-request", "Deny")
                            .disabled(self.responding)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.denial_reason_editor.is_none() {
                                    this.show_denial_input(window, cx);
                                } else {
                                    this.respond(false, window, cx);
                                }
                            })),
                    ),
            )
    }
}
