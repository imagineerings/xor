use std::collections::HashMap;

use gpui::SharedString;

use crate::BaymaxApp;

/// Registry for managing and launching baymax apps.
pub struct AppRegistry {
    apps: HashMap<String, Box<dyn BaymaxApp>>,
    active_app: Option<String>,
}

impl AppRegistry {
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
            active_app: None,
        }
    }

    /// Register an app with the registry.
    pub fn register(&mut self, app: Box<dyn BaymaxApp>) {
        self.apps.insert(app.id().to_string(), app);
    }

    /// Get a reference to a registered app by id.
    pub fn get(&self, id: &str) -> Option<&dyn BaymaxApp> {
        self.apps.get(id).map(|b| b.as_ref())
    }

    /// Get a mutable reference to a registered app by id.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut dyn BaymaxApp> {
        match self.apps.get_mut(id) {
            Some(b) => Some(b.as_mut()),
            None => None,
        }
    }

    /// Launch (activate) an app by id.
    pub fn launch(&mut self, id: &str) -> Result<(), &'static str> {
        if self.apps.contains_key(id) {
            self.active_app = Some(id.to_string());
            Ok(())
        } else {
            Err("app not found")
        }
    }

    /// Returns the id of the currently active app, if any.
    pub fn active_app_id(&self) -> Option<&str> {
        self.active_app.as_deref()
    }

    /// Returns the currently active app, if any.
    pub fn active_app(&self) -> Option<&dyn BaymaxApp> {
        self.active_app
            .as_ref()
            .and_then(|id| self.apps.get(id))
            .map(|b| b.as_ref())
    }

    /// Returns the active app mutably, if any.
    pub fn active_app_mut(&mut self) -> Option<&mut dyn BaymaxApp> {
        let id = self.active_app.as_ref()?;
        match self.apps.get_mut(id) {
            Some(b) => Some(b.as_mut()),
            None => None,
        }
    }

    /// List all registered app ids and names.
    pub fn list_apps(&self) -> Vec<(SharedString, SharedString)> {
        self.apps
            .values()
            .map(|app| (app.id().into(), app.name()))
            .collect()
    }

    /// Close the active app.
    pub fn close_active(&mut self) {
        self.active_app = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::IntoElement;
    use gpui::SharedString;

    struct TestApp {
        id: &'static str,
        name: &'static str,
    }

    impl BaymaxApp for TestApp {
        fn id(&self) -> &str {
            self.id
        }
        fn name(&self) -> SharedString {
            self.name.into()
        }
        fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> gpui::AnyElement {
            gpui::div().into_any_element()
        }
        fn handle_action(&mut self, _action: &dyn gpui::Action, _cx: &mut gpui::App) {}
    }

    fn test_app(id: &'static str, name: &'static str) -> Box<dyn BaymaxApp> {
        Box::new(TestApp { id, name })
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("chat", "Chat"));
        let app = registry.get("chat").unwrap();
        assert_eq!(app.id(), "chat");
        assert_eq!(app.name(), SharedString::from("Chat"));
    }

    #[test]
    fn test_get_missing_app() {
        let registry = AppRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("config", "Config"));
        let app = registry.get_mut("config").unwrap();
        assert_eq!(app.id(), "config");
    }

    #[test]
    fn test_launch_and_active_app() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("clock", "Clock"));
        registry.launch("clock").unwrap();
        assert_eq!(registry.active_app_id(), Some("clock"));
        assert!(registry.active_app().is_some());
    }

    #[test]
    fn test_launch_missing_app_returns_error() {
        let mut registry = AppRegistry::new();
        assert_eq!(registry.launch("missing"), Err("app not found"));
    }

    #[test]
    fn test_active_app_is_none_when_no_app_launched() {
        let registry = AppRegistry::new();
        assert!(registry.active_app_id().is_none());
        assert!(registry.active_app().is_none());
    }

    #[test]
    fn test_active_app_mut() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("term", "Terminal"));
        registry.launch("term").unwrap();
        assert!(registry.active_app_mut().is_some());
    }

    #[test]
    fn test_close_active() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("chat", "Chat"));
        registry.launch("chat").unwrap();
        assert!(registry.active_app().is_some());
        registry.close_active();
        assert!(registry.active_app().is_none());
    }

    #[test]
    fn test_list_apps() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("a", "App A"));
        registry.register(test_app("b", "App B"));
        let apps = registry.list_apps();
        assert_eq!(apps.len(), 2);
        assert!(apps.contains(&("a".into(), "App A".into())));
        assert!(apps.contains(&("b".into(), "App B".into())));
    }

    #[test]
    fn test_register_overwrites_existing() {
        let mut registry = AppRegistry::new();
        registry.register(test_app("chat", "Chat"));
        registry.register(test_app("chat", "Chat v2"));
        let app = registry.get("chat").unwrap();
        assert_eq!(app.name(), SharedString::from("Chat v2"));
    }
}
