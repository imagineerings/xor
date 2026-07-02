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
