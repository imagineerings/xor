mod app_registry;
mod cache_manager;
mod chat_app;
mod clock_app;
mod diffusion_graph;
mod game_authoring;
mod resource_manager;

#[cfg(test)]
mod game_authoring_tests;

pub use app_registry::*;
pub use cache_manager::*;
pub use chat_app::*;
pub use clock_app::*;
pub use diffusion_graph::*;
pub use game_authoring::*;
pub use resource_manager::*;

use gpui::{App, SharedString, Window};

/// A sim app is an embedded mini-application that can be
/// registered, launched, and rendered within the workspace.
pub trait SimApp: Send {
    /// Unique identifier for this app (e.g. "chat", "clock").
    fn id(&self) -> &str;
    /// Human-readable name shown in the UI.
    fn name(&self) -> SharedString;
    /// Render the app's UI.
    fn render(&self, window: &mut Window, cx: &mut App) -> gpui::AnyElement;
    /// Handle an action dispatched to this app.
    fn handle_action(&mut self, action: &dyn gpui::Action, cx: &mut App);
}
