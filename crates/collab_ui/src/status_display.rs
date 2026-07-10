use client::CustomStatus;
use gpui::{App, IntoElement, RenderOnce};
use ui::{Color, Label, LabelSize, prelude::*};

#[derive(IntoElement)]
pub struct StatusDisplay {
    status: Option<CustomStatus>,
}

impl StatusDisplay {
    pub fn new(status: Option<CustomStatus>) -> Self {
        Self { status }
    }
}

impl RenderOnce for StatusDisplay {
    fn render(self, _: &mut gpui::Window, _: &mut App) -> impl IntoElement {
        self.status.map_or_else(
            || div().into_any_element(),
            |status| {
                let emoji = status
                    .emoji
                    .map(|emoji| format!("{emoji} "))
                    .unwrap_or_default();
                Label::new(format!("{emoji}{}", status.text))
                    .size(LabelSize::Small)
                    .color(Color::Muted)
                    .into_any_element()
            },
        )
    }
}
