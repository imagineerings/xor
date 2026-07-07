use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use recipe::{BuiltinRecipeSource, RecipeEngine};
use serde::{Deserialize, Serialize};

use crate::{AuthConfig, ServerConfig, SessionEventBus};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub recipes: Arc<RecipeEngine>,
    pub data: Arc<Mutex<ServerData>>,
    pub events: SessionEventBus,
}

impl AppState {
    pub fn new(config: ServerConfig, recipes: RecipeEngine) -> Self {
        Self {
            config: Arc::new(config),
            recipes: Arc::new(recipes),
            data: Arc::new(Mutex::new(ServerData::default())),
            events: SessionEventBus::default(),
        }
    }

    pub fn for_tests() -> Self {
        Self::new(
            ServerConfig {
                auth: AuthConfig::None,
                ..Default::default()
            },
            RecipeEngine::new().with_source(BuiltinRecipeSource::sim_defaults()),
        )
    }
}

#[derive(Debug, Default)]
pub struct ServerData {
    pub sessions: HashMap<String, SessionDetail>,
    pub schedules: HashMap<String, ScheduleDetail>,
    pub gateways: HashMap<String, GatewayDetail>,
    pub config: serde_json::Value,
    pub setup_complete: bool,
    next_id: usize,
}

impl ServerData {
    pub fn next_id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}-{}", self.next_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerStatus {
    pub status: String,
    pub active_sessions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleDetail {
    pub id: String,
    pub cron: String,
    pub recipe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayDetail {
    pub id: String,
    pub kind: String,
}
