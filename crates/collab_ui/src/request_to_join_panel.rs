use channel::ChannelStore;
use client::ChannelId;
use editor::Editor;
use gpui::{Context, Entity, Render, SharedString, Window, prelude::*};
use rpc::proto;
use ui::{Button, Label, LabelSize, prelude::*};

pub struct RequestToJoinPanel {
    channel_id: ChannelId,
    reason_editor: Entity<Editor>,
    channel_store: Entity<ChannelStore>,
    state: RequestState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestState {
    Idle,
    Sending,
    Sent,
    AlreadyRequested,
    Error(SharedString),
}

impl RequestToJoinPanel {
    pub fn new(
        channel_id: ChannelId,
        channel_store: Entity<ChannelStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let reason_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Optional message to channel admins", window, cx);
            editor
        });
        Self {
            channel_id,
            reason_editor,
            channel_store,
            state: RequestState::Idle,
        }
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state == RequestState::Sending {
            return;
        }

        self.state = RequestState::Sending;
        let reason = self.reason_editor.read(cx).text(cx).trim().to_string();
        let reason = (!reason.is_empty()).then_some(reason);
        let channel_id = self.channel_id;
        let client = self.channel_store.read(cx).client();
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let result = client
                .request(proto::RequestJoinChannel {
                    channel_id: channel_id.0,
                    reason,
                })
                .await;
            this.update_in(cx, |this, _, cx| {
                this.state = match result {
                    Ok(_) => RequestState::Sent,
                    Err(error)
                        if error.to_string().contains("already")
                            || error.to_string().contains("unique") =>
                    {
                        RequestState::AlreadyRequested
                    }
                    Err(error) => RequestState::Error(SharedString::from(error.to_string())),
                };
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }
}

impl Render for RequestToJoinPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(Label::new("Request to Join").size(LabelSize::Large))
            .child(match &self.state {
                RequestState::Idle | RequestState::Sending | RequestState::Error(_) => v_flex()
                    .gap_2()
                    .child(self.reason_editor.clone())
                    .when_some(
                        match &self.state {
                            RequestState::Error(error) => Some(error.clone()),
                            _ => None,
                        },
                        |this, error| this.child(Label::new(error).color(Color::Error)),
                    )
                    .child(
                        Button::new("request-to-join", "Request to Join")
                            .disabled(self.state == RequestState::Sending)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit(window, cx);
                            })),
                    )
                    .into_any_element(),
                RequestState::Sent => Label::new(
                    "Join request sent. You'll be notified when a channel admin responds.",
                )
                .into_any_element(),
                RequestState::AlreadyRequested => {
                    Label::new("You have already requested to join this channel.")
                        .into_any_element()
                }
            })
    }
}
