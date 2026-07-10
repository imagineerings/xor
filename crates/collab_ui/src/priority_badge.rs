use client::MessagePriority;
use gpui::{App, IntoElement, RenderOnce, Window};
use ui::{Color, Icon, IconName, IconSize, Label, LabelSize, prelude::*};

#[derive(IntoElement)]
pub struct PriorityBadge {
    priority: MessagePriority,
}

impl PriorityBadge {
    pub fn new(priority: MessagePriority) -> Self {
        Self { priority }
    }
}

impl RenderOnce for PriorityBadge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (label, icon, color) = match self.priority {
            MessagePriority::Normal => return div().into_any_element(),
            MessagePriority::Important => ("Important", IconName::Warning, Color::Warning),
            MessagePriority::Urgent => ("Urgent", IconName::Triangle, Color::Error),
        };

        h_flex()
            .gap_1()
            .items_center()
            .child(Icon::new(icon).size(IconSize::XSmall).color(color))
            .child(Label::new(label).size(LabelSize::XSmall).color(color))
            .into_any_element()
    }
}
