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
        rendered_status_text(self.status.as_ref()).map_or_else(
            || div().into_any_element(),
            |text| {
                Label::new(text)
                    .size(LabelSize::Small)
                    .color(Color::Muted)
                    .into_any_element()
            },
        )
    }
}

fn rendered_status_text(status: Option<&CustomStatus>) -> Option<String> {
    status.map(|status| {
        let emoji = status
            .emoji
            .as_ref()
            .map(|emoji| format!("{emoji} "))
            .unwrap_or_default();
        format!("{emoji}{}", status.text)
    })
}

#[cfg(test)]
mod tests {
    use super::rendered_status_text;
    use client::CustomStatus;

    #[test]
    fn renders_emoji_and_text() {
        let status = CustomStatus {
            emoji: Some("📅".into()),
            text: "In a meeting".into(),
            expires_at: None,
        };
        assert_eq!(
            rendered_status_text(Some(&status)),
            Some("📅 In a meeting".to_string())
        );
    }

    #[test]
    fn omits_empty_status() {
        assert_eq!(rendered_status_text(None), None);
    }
}
