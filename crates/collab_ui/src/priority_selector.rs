use client::MessagePriority;
use gpui::{App, ClickEvent, IntoElement, Window};
use ui::{Button, ButtonStyle, LabelSize, SelectableButton, TintColor, h_flex, prelude::*};

pub struct PrioritySelector {
    selected: MessagePriority,
}

impl PrioritySelector {
    pub fn new(selected: MessagePriority) -> Self {
        Self { selected }
    }

    pub fn render<Normal, Important, Urgent>(
        self,
        normal: Normal,
        important: Important,
        urgent: Urgent,
    ) -> impl IntoElement
    where
        Normal: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        Important: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        Urgent: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        h_flex()
            .id("channel-message-priority")
            .gap_1()
            .items_center()
            .child(
                Button::new("channel-message-priority-normal", "Normal")
                    .label_size(LabelSize::XSmall)
                    .toggle_state(self.selected == MessagePriority::Normal)
                    .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                    .on_click(normal),
            )
            .child(
                Button::new("channel-message-priority-important", "Important")
                    .label_size(LabelSize::XSmall)
                    .toggle_state(self.selected == MessagePriority::Important)
                    .selected_style(ButtonStyle::Tinted(TintColor::Warning))
                    .on_click(important),
            )
            .child(
                Button::new("channel-message-priority-urgent", "Urgent")
                    .label_size(LabelSize::XSmall)
                    .toggle_state(self.selected == MessagePriority::Urgent)
                    .selected_style(ButtonStyle::Tinted(TintColor::Error))
                    .on_click(urgent),
            )
    }
}
