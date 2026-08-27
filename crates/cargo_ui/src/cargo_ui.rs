mod cargo_actions;
mod cargo_coverage;
mod cargo_panel;
mod cargo_panel_settings;
mod cargo_preset;
mod cargo_preset_editor;
mod cargo_profile;
mod dependency_insight;

pub use cargo_actions::*;
pub use cargo_coverage::*;
pub use cargo_panel::{CargoPanel, CargoTreeProvider, ToggleCargoPanel};
pub use cargo_panel_settings::{CargoPanelSettings, CargoPanelSide};
pub use cargo_preset::*;
pub use cargo_preset_editor::*;
pub use cargo_profile::*;
pub use dependency_insight::*;

use gpui::{App, KeyBinding, UpdateGlobal as _};
use language_tools::language_tool_tree::{
    ActivateSelected, CollapseAll, ExpandAll, Refresh, SelectFirst, SelectFirstChild, SelectLast,
    SelectNext, SelectParent, SelectPrevious, ToggleExpanded,
};
use settings::{CargoPanelSettingsContent, CargoSettingsContent, DockSide, SettingsStore};
use workspace::Workspace;

pub fn init(cx: &mut App) {
    SettingsStore::update_global(cx, |store, cx| {
        store.update_default_settings(cx, |settings| {
            settings.cargo_panel = Some(CargoPanelSettingsContent {
                button: Some(true),
                default_width: Some(280.),
                dock: Some(DockSide::Right),
                starts_open: Some(false),
            });
            settings.cargo = Some(CargoSettingsContent {
                schema_version: Some(CARGO_PRESET_SCHEMA_VERSION),
                presets: Default::default(),
            });
        });
        store.register_setting::<CargoPanelSettings>();
        store.register_setting::<CargoPresetSettings>();
    });
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(CargoPanel::toggle_focus);
    })
    .detach();
    cx.bind_keys([
        KeyBinding::new("up", SelectPrevious, Some("CargoPanel")),
        KeyBinding::new("down", SelectNext, Some("CargoPanel")),
        KeyBinding::new("home", SelectFirst, Some("CargoPanel")),
        KeyBinding::new("end", SelectLast, Some("CargoPanel")),
        KeyBinding::new("left", SelectParent, Some("CargoPanel")),
        KeyBinding::new("right", SelectFirstChild, Some("CargoPanel")),
        KeyBinding::new("space", ToggleExpanded, Some("CargoPanel")),
        KeyBinding::new("enter", ActivateSelected, Some("CargoPanel")),
        KeyBinding::new("cmd-shift-e", ExpandAll, Some("CargoPanel")),
        KeyBinding::new("cmd-shift-c", CollapseAll, Some("CargoPanel")),
        KeyBinding::new("cmd-r", Refresh, Some("CargoPanel")),
    ]);
}
