use crate::BaymaxApp;
use chrono::Local;
use gpui::{AnyElement, App, SharedString, Window};
use ui::prelude::*;

/// A simple clock app that displays the current time.
pub struct ClockApp {
    label: SharedString,
}

impl ClockApp {
    pub fn new() -> Self {
        let now = Local::now();
        Self {
            label: now.format("%Y-%m-%d %H:%M:%S").to_string().into(),
        }
    }

    pub fn tick(&mut self) {
        let now = Local::now();
        self.label = now.format("%Y-%m-%d %H:%M:%S").to_string().into();
    }

    pub fn set_label(&mut self, label: impl Into<SharedString>) {
        self.label = label.into();
    }
}

impl BaymaxApp for ClockApp {
    fn id(&self) -> &str {
        "clock"
    }

    fn name(&self) -> SharedString {
        "Clock".into()
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        v_flex()
            .id("clock-app")
            .size_full()
            .gap_2()
            .p_4()
            .child(
                Headline::new("Clock")
                    .size(HeadlineSize::Small)
                    .color(Color::Default),
            )
            .child(
                h_flex().size_full().justify_center().child(
                    Label::new(self.label.clone())
                        .size(LabelSize::Large)
                        .color(Color::Accent),
                ),
            )
            .into_any_element()
    }

    fn handle_action(&mut self, _action: &dyn gpui::Action, _cx: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_clock_app() {
        let app = ClockApp::new();
        assert_eq!(app.id(), "clock");
        assert_eq!(app.name(), SharedString::from("Clock"));
    }

    #[test]
    fn test_tick_does_not_panic() {
        let mut app = ClockApp::new();
        // Tick should not panic and should update the label.
        app.tick();
        // After ticking, the label should still be a non-empty string.
        assert!(!app.label.is_empty());
    }

    #[test]
    fn test_set_label() {
        let mut app = ClockApp::new();
        app.set_label("custom-label");
        assert_eq!(app.label, SharedString::from("custom-label"));
    }

    #[test]
    fn test_clock_app_trait_methods() {
        let app = ClockApp::new();
        assert_eq!(app.id(), "clock");
        assert_eq!(app.name(), SharedString::from("Clock"));
    }
}
