use anyhow::Result;
use gpui::{
    Action, AnyElement, App, AsyncWindowContext, Context, Entity, EventEmitter, FocusHandle,
    Focusable, Render, Subscription, WeakEntity, Window, actions,
};
use sim_apps::{AppRegistry, ChatApp, ClockApp};
use ui::prelude::*;
use workspace::{
    Panel, Workspace,
    dock::{DockPosition, PanelEvent},
};

actions!(
    apps_panel,
    [
        /// Toggles the Sim Apps panel.
        Toggle,
        /// Toggles focus on the Sim Apps panel.
        ToggleFocus
    ]
);

/// A workspace panel that hosts embedded sim apps (chat, clock, etc.).
pub struct AppsPanel {
    registry: AppRegistry,
    focus_handle: FocusHandle,
    _workspace: WeakEntity<Workspace>,
    _subscriptions: Vec<Subscription>,
}

#[allow(dead_code)]
impl AppsPanel {
    pub fn new(workspace: &Workspace, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let mut registry = AppRegistry::new();
        registry.register(Box::new(ChatApp::new()));
        registry.register(Box::new(ClockApp::new()));
        // Launch the chat app by default
        let _ = registry.launch("chat");

        Self {
            registry,
            focus_handle,
            _workspace: workspace.weak_handle(),
            _subscriptions: Vec::new(),
        }
    }

    /// Load the panel asynchronously.
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<AppsPanel>> {
        let panel = workspace.update_in(&mut cx, |workspace, window, cx| {
            cx.new(|cx| AppsPanel::new(workspace, window, cx))
        })?;
        Ok(panel)
    }

    pub fn toggle(
        workspace: &mut Workspace,
        _: &Toggle,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if !workspace.toggle_panel_focus::<Self>(window, cx) {
            workspace.close_panel::<Self>(window, cx);
        }
    }

    pub fn toggle_focus(
        workspace: &mut Workspace,
        _: &ToggleFocus,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        workspace.toggle_panel_focus::<Self>(window, cx);
    }
}

impl Focusable for AppsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for AppsPanel {}

impl Render for AppsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_count = self.registry.list_apps().len();

        v_flex()
            .id("sim-apps-panel")
            .size_full()
            .bg(cx.theme().colors().panel_background)
            .child(
                h_flex()
                    .id("apps-header")
                    .w_full()
                    .p_2()
                    .gap_2()
                    .bg(cx.theme().colors().title_bar_background)
                    .justify_between()
                    .child(
                        Headline::new("Sim Apps")
                            .size(HeadlineSize::Small)
                            .color(Color::Default),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .children(self.render_app_switcher(window, cx))
                            .child(
                                Label::new(format!("{} apps", app_count))
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            ),
                    ),
            )
            .child(
                div()
                    .id("apps-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.render_active_app(window, cx)),
            )
    }
}

impl AppsPanel {
    fn render_app_switcher(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let active_id = self.registry.active_app_id().map(|s| s.to_string());

        self.registry
            .list_apps()
            .into_iter()
            .map(|(id, name)| {
                let is_active = active_id.as_deref() == Some(id.as_ref());
                Label::new(name)
                    .size(LabelSize::Small)
                    .color(if is_active {
                        Color::Accent
                    } else {
                        Color::Muted
                    })
                    .into_any_element()
            })
            .collect()
    }

    fn render_active_app(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if let Some(app) = self.registry.active_app() {
            app.render(window, cx)
        } else {
            v_flex()
                .id("no-app")
                .size_full()
                .justify_center()
                .items_center()
                .child(
                    Label::new("No app selected")
                        .size(LabelSize::Large)
                        .color(Color::Muted),
                )
                .into_any_element()
        }
    }
}

impl Panel for AppsPanel {
    fn persistent_name() -> &'static str {
        "SimAppsPanel"
    }

    fn panel_key() -> &'static str {
        "sim_apps_panel"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Right
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Left | DockPosition::Right | DockPosition::Bottom
        )
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // Position not persisted yet
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(320.0)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::SimAssistant)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Sim Apps")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        100
    }

    fn starts_open(&self, _window: &Window, _cx: &App) -> bool {
        false
    }
}
