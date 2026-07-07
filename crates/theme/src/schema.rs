#![allow(missing_docs)]

use gpui::Hsla;
use palette::FromColor;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The appearance of a theme in serialisim content.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceContent {
    Light,
    Dark,
}

/// Parses a color string into an [`Hsla`] value.
/// Border radius values for UI element types.
///
/// When present in a theme, these override the default border radius
/// for each element type. When absent, current defaults are used.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BorderRadiusContent {
    /// Border radius for buttons (default: 6)
    pub button: Option<f32>,
    /// Border radius for inputs (default: 4)
    pub input: Option<f32>,
    /// Border radius for panels and sidebars (default: 8)
    pub panel: Option<f32>,
    /// Border radius for modal dialogs (default: 12)
    pub modal: Option<f32>,
    /// Border radius for tooltips (default: 4)
    pub tooltip: Option<f32>,
    /// Border radius for autocomplete menus (default: 6)
    pub autocomplete: Option<f32>,
    /// Border radius for scrollbar thumb (default: 2)
    pub scrollbar_thumb: Option<f32>,
}

pub fn try_parse_color(color: &str) -> anyhow::Result<Hsla> {
    let rgba = gpui::Rgba::try_from(color)?;
    let rgba = palette::rgb::Srgba::from_components((rgba.r, rgba.g, rgba.b, rgba.a));
    let hsla = palette::Hsla::from_color(rgba);

    let hsla = gpui::hsla(
        hsla.hue.into_positive_degrees() / 360.,
        hsla.saturation,
        hsla.lightness,
        hsla.alpha,
    );

    Ok(hsla)
}
