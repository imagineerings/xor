use crate::{DecisionType, RiskLevel, ToolCall};
use gpui::{
    App, IntoElement, ParentElement, RenderOnce, SharedString, Window, prelude::*, px, rems,
};
use std::sync::Arc;
use ui::{Button, ButtonStyle, Color, Label, LabelSize, TintColor, h_flex, prelude::*, v_flex};

pub type PermissionConfirmationCallback =
    Arc<dyn Fn(PermissionConfirmationAction, &mut Window, &mut App) + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionConfirmationRequest {
    pub tool_call: ToolCall,
    pub risk_level: RiskLevel,
    pub reason: SharedString,
}

impl PermissionConfirmationRequest {
    pub fn new(
        tool_call: ToolCall,
        risk_level: RiskLevel,
        reason: impl Into<SharedString>,
    ) -> Self {
        Self {
            tool_call,
            risk_level,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionConfirmationAction {
    Allow,
    Deny,
    AlwaysAllow,
    AlwaysDeny,
}

impl PermissionConfirmationAction {
    pub fn stored_decision_type(self) -> Option<DecisionType> {
        match self {
            Self::Allow => Some(DecisionType::AllowOnce),
            Self::Deny => Some(DecisionType::DenyOnce),
            Self::AlwaysAllow => Some(DecisionType::AlwaysAllow),
            Self::AlwaysDeny => Some(DecisionType::AlwaysDeny),
        }
    }
}

#[derive(Clone, IntoElement)]
pub struct PermissionConfirmation {
    request: PermissionConfirmationRequest,
    on_action: Option<PermissionConfirmationCallback>,
}

impl PermissionConfirmation {
    pub fn new(request: PermissionConfirmationRequest) -> Self {
        Self {
            request,
            on_action: None,
        }
    }

    pub fn on_action(mut self, callback: PermissionConfirmationCallback) -> Self {
        self.on_action = Some(callback);
        self
    }
}

impl RenderOnce for PermissionConfirmation {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let risk_color = match self.request.risk_level {
            RiskLevel::Low => Color::Success,
            RiskLevel::Medium => Color::Warning,
            RiskLevel::High => Color::Error,
        };
        let risk_label = match self.request.risk_level {
            RiskLevel::Low => "Low risk",
            RiskLevel::Medium => "Medium risk",
            RiskLevel::High => "High risk",
        };

        v_flex()
            .w(px(460.))
            .max_w_full()
            .gap_4()
            .p_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(Label::new("Confirm tool action").size(LabelSize::Large))
                    .child(Label::new(self.request.reason).color(Color::Muted)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(detail_row("Tool", self.request.tool_call.tool_name))
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .child(Label::new("Risk").color(Color::Muted))
                            .child(Label::new(risk_label).color(risk_color)),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("Arguments").color(Color::Muted))
                            .child(
                                v_flex()
                                    .w_full()
                                    .max_h(px(160.))
                                    .overflow_hidden()
                                    .rounded_md()
                                    .border_1()
                                    .p_2()
                                    .text_size(rems(0.8125))
                                    .child(self.request.tool_call.arguments),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(action_button(
                        "permission-deny",
                        "Deny",
                        PermissionConfirmationAction::Deny,
                        ButtonStyle::Tinted(TintColor::Error),
                        self.on_action.clone(),
                    ))
                    .child(action_button(
                        "permission-always-deny",
                        "Always Deny",
                        PermissionConfirmationAction::AlwaysDeny,
                        ButtonStyle::Outlined,
                        self.on_action.clone(),
                    ))
                    .child(action_button(
                        "permission-allow",
                        "Allow",
                        PermissionConfirmationAction::Allow,
                        ButtonStyle::Tinted(TintColor::Success),
                        self.on_action.clone(),
                    ))
                    .child(action_button(
                        "permission-always-allow",
                        "Always Allow",
                        PermissionConfirmationAction::AlwaysAllow,
                        ButtonStyle::Filled,
                        self.on_action,
                    )),
            )
    }
}

fn detail_row(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    h_flex()
        .justify_between()
        .gap_3()
        .child(Label::new(label).color(Color::Muted))
        .child(Label::new(value.into()))
}

fn action_button(
    id: &'static str,
    label: &'static str,
    action: PermissionConfirmationAction,
    style: ButtonStyle,
    callback: Option<PermissionConfirmationCallback>,
) -> impl IntoElement {
    Button::new(id, label)
        .style(style)
        .on_click(move |_event, window, cx| {
            if let Some(callback) = &callback {
                callback(action, window, cx);
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_actions_map_to_stored_decisions() {
        assert_eq!(
            PermissionConfirmationAction::Allow.stored_decision_type(),
            Some(DecisionType::AllowOnce)
        );
        assert_eq!(
            PermissionConfirmationAction::Deny.stored_decision_type(),
            Some(DecisionType::DenyOnce)
        );
        assert_eq!(
            PermissionConfirmationAction::AlwaysAllow.stored_decision_type(),
            Some(DecisionType::AlwaysAllow)
        );
        assert_eq!(
            PermissionConfirmationAction::AlwaysDeny.stored_decision_type(),
            Some(DecisionType::AlwaysDeny)
        );
    }

    #[test]
    fn request_captures_tool_risk_and_reason() {
        let request = PermissionConfirmationRequest::new(
            ToolCall::new("terminal", "cargo test"),
            RiskLevel::Medium,
            "terminal requires confirmation",
        );

        assert_eq!(request.tool_call.tool_name, "terminal");
        assert_eq!(request.risk_level, RiskLevel::Medium);
        assert_eq!(request.reason.as_ref(), "terminal requires confirmation");
    }
}
