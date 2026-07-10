use channel::ChannelStore;
use client::ChannelId;
use editor::{Editor, EditorEvent};
use gpui::{Context, Entity, Render, SharedString, Subscription, Window, prelude::*};
use rpc::proto;
use ui::{Button, Label, LabelSize, prelude::*};

pub struct RequestToJoinPanel {
    channel_id: ChannelId,
    reason_editor: Entity<Editor>,
    channel_store: Entity<ChannelStore>,
    state: RequestState,
    _reason_subscription: Subscription,
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
    const MAX_REASON_CHARS: usize = 500;

    fn normalize_reason(reason: &str) -> String {
        reason.trim().chars().take(Self::MAX_REASON_CHARS).collect()
    }

    fn state_from_result(result: std::result::Result<(), String>) -> RequestState {
        match result {
            Ok(()) => RequestState::Sent,
            Err(error) if error.contains("already") || error.contains("unique") => {
                RequestState::AlreadyRequested
            }
            Err(error) => RequestState::Error(error.into()),
        }
    }

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
        let reason_subscription = cx.subscribe(&reason_editor, |_this, _, event, cx| {
            if matches!(event, EditorEvent::BufferEdited) {
                cx.notify();
            }
        });
        Self {
            channel_id,
            reason_editor,
            channel_store,
            state: RequestState::Idle,
            _reason_subscription: reason_subscription,
        }
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state == RequestState::Sending {
            return;
        }

        self.state = RequestState::Sending;
        let reason = Self::normalize_reason(&self.reason_editor.read(cx).text(cx));
        self.reason_editor.update(cx, |editor, cx| {
            editor.set_text(reason.clone(), window, cx);
        });
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
                this.state =
                    Self::state_from_result(result.map(|_| ()).map_err(|error| error.to_string()));
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_normalization_trims_and_limits_input() {
        assert_eq!(RequestToJoinPanel::normalize_reason("  hello  "), "hello");
        assert_eq!(
            RequestToJoinPanel::normalize_reason(&"x".repeat(501)).len(),
            RequestToJoinPanel::MAX_REASON_CHARS
        );
    }

    #[test]
    fn request_state_maps_rpc_outcomes() {
        assert_eq!(
            RequestToJoinPanel::state_from_result(Ok(())),
            RequestState::Sent
        );
        assert_eq!(
            RequestToJoinPanel::state_from_result(Err("already requested".to_string())),
            RequestState::AlreadyRequested
        );
        assert!(matches!(
            RequestToJoinPanel::state_from_result(Err("server unavailable".to_string())),
            RequestState::Error(_)
        ));
    }
}

impl Render for RequestToJoinPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let reason_length = self.reason_editor.read(cx).text(cx).chars().count();
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(Label::new("Request to Join").size(LabelSize::Large))
            .child(match &self.state {
                RequestState::Idle | RequestState::Sending | RequestState::Error(_) => v_flex()
                    .gap_2()
                    .child(self.reason_editor.clone())
                    .child(
                        Label::new(format!("{reason_length}/{}", Self::MAX_REASON_CHARS))
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
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
