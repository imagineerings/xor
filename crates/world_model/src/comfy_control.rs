use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{ComfyHttpMethod, ComfyRouteCatalog, ComfyRouteHandler, ComfyRouteKind, graph::NodeId};

pub const INVALID_PROMPT_ID_CODE: &str = "world_model.comfy_control.invalid_prompt_id";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyPromptId(String);

impl ComfyPromptId {
    pub fn parse(value: &str) -> Result<Self, ComfyControlDiagnostic> {
        let parsed = Uuid::parse_str(value).map_err(|_| invalid_prompt_id(value))?;
        if parsed.to_string() != value {
            return Err(invalid_prompt_id(value));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyControlDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptSubmission {
    pub prompt_id: Option<ComfyPromptId>,
    pub prompt: serde_json::Value,
    pub queue_number: Option<QueueNumber>,
    pub front: bool,
    pub client_id: Option<String>,
    pub extra_data: PromptExtraData,
    pub partial_execution_targets: Vec<NodeId>,
}

impl PromptSubmission {
    pub fn new(prompt: serde_json::Value) -> Self {
        Self {
            prompt_id: None,
            prompt,
            queue_number: None,
            front: false,
            client_id: None,
            extra_data: PromptExtraData::default(),
            partial_execution_targets: Vec::new(),
        }
    }

    pub fn with_prompt_id(mut self, prompt_id: ComfyPromptId) -> Self {
        self.prompt_id = Some(prompt_id);
        self
    }

    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn with_front(mut self, front: bool) -> Self {
        self.front = front;
        self
    }

    pub fn with_queue_number(mut self, queue_number: QueueNumber) -> Self {
        self.queue_number = Some(queue_number);
        self
    }

    pub fn with_extra_data(mut self, extra_data: PromptExtraData) -> Self {
        self.extra_data = extra_data;
        self
    }

    pub fn with_partial_execution_targets(
        mut self,
        targets: impl IntoIterator<Item = NodeId>,
    ) -> Self {
        self.partial_execution_targets = targets.into_iter().collect();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QueueNumber(pub f64);

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptExtraData {
    pub public: BTreeMap<String, String>,
    pub sensitive_keys: BTreeSet<String>,
}

impl PromptExtraData {
    pub fn with_public(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.public.insert(key.into(), value.into());
        self
    }

    pub fn with_sensitive_key(mut self, key: impl Into<String>) -> Self {
        self.sensitive_keys.insert(key.into());
        self
    }

    pub fn redacted(&self) -> BTreeMap<String, String> {
        self.public
            .iter()
            .filter(|(key, _)| !self.sensitive_keys.contains(*key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptSubmissionResponse {
    pub prompt_id: ComfyPromptId,
    pub number: u64,
    pub node_errors: BTreeMap<NodeId, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueueAction {
    Clear,
    Delete { prompt_ids: BTreeSet<ComfyPromptId> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HistoryAction {
    Clear,
    Delete { prompt_ids: BTreeSet<ComfyPromptId> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ComfyJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ComfyJobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyJobSummary {
    pub prompt_id: ComfyPromptId,
    pub queue_position: Option<u64>,
    pub status: ComfyJobStatus,
    pub client_id: Option<String>,
    pub outputs: Vec<String>,
    pub public_extra_data: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueueStatus {
    pub running: Vec<ComfyJobSummary>,
    pub pending: Vec<ComfyJobSummary>,
    pub history_count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyFeatureFlags {
    pub flags: BTreeMap<String, bool>,
}

impl ComfyFeatureFlags {
    pub fn with_flag(mut self, name: impl Into<String>, enabled: bool) -> Self {
        self.flags.insert(name.into(), enabled);
        self
    }

    pub fn enabled(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimControlPlaneSettingsStore {
    settings: BTreeMap<String, Value>,
}

impl SimControlPlaneSettingsStore {
    pub fn read_all(&self) -> BTreeMap<String, Value> {
        self.settings.clone()
    }

    pub fn read(&self, setting_id: &str) -> Option<Value> {
        self.settings.get(setting_id).cloned()
    }

    pub fn write(&mut self, setting_id: impl Into<String>, value: Value) -> Value {
        let setting_id = setting_id.into();
        self.settings.insert(setting_id, value.clone());
        value
    }

    pub fn replace_all(&mut self, settings: BTreeMap<String, Value>) -> BTreeMap<String, Value> {
        self.settings = settings;
        self.read_all()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlPlaneUser {
    pub user_id: String,
    pub display_name: String,
    pub is_current: bool,
}

impl SimControlPlaneUser {
    pub fn new(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            display_name: display_name.into(),
            is_current: false,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlPlaneUserRegistry {
    users: BTreeMap<String, SimControlPlaneUser>,
    current_user_id: Option<String>,
}

impl SimControlPlaneUserRegistry {
    pub fn upsert_user(&mut self, user: SimControlPlaneUser) {
        let user_id = user.user_id.clone();
        self.users.insert(user_id.clone(), user);
        if self.current_user_id.is_none() {
            self.current_user_id = Some(user_id);
        }
        self.refresh_current_flags();
    }

    pub fn select_user(&mut self, user_id: impl Into<String>) -> Option<SimControlPlaneUser> {
        let user_id = user_id.into();
        if !self.users.contains_key(&user_id) {
            return None;
        }
        self.current_user_id = Some(user_id.clone());
        self.refresh_current_flags();
        self.users.get(&user_id).cloned()
    }

    pub fn users(&self) -> Vec<SimControlPlaneUser> {
        self.users.values().cloned().collect()
    }

    pub fn current_user_id(&self) -> Option<&str> {
        self.current_user_id.as_deref()
    }

    fn refresh_current_flags(&mut self) {
        for user in self.users.values_mut() {
            user.is_current = self.current_user_id.as_ref() == Some(&user.user_id);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlPlaneDeviceStats {
    pub name: String,
    pub device_type: String,
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlPlaneSystemStats {
    pub platform: String,
    pub python_embedded: bool,
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub devices: Vec<SimControlPlaneDeviceStats>,
    pub features: ComfyFeatureFlags,
}

impl SimControlPlaneSystemStats {
    pub fn metadata_only(platform: impl Into<String>) -> Self {
        Self {
            platform: platform.into(),
            python_embedded: false,
            total_memory_bytes: 0,
            free_memory_bytes: 0,
            devices: Vec::new(),
            features: ComfyFeatureFlags::default(),
        }
    }

    pub fn with_device(mut self, device: SimControlPlaneDeviceStats) -> Self {
        self.devices.push(device);
        self
    }

    pub fn with_feature(mut self, feature: impl Into<String>, enabled: bool) -> Self {
        self.features = self.features.with_flag(feature, enabled);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlPlaneRouteCapability {
    pub kind: ComfyRouteKind,
    pub method: ComfyHttpMethod,
    pub path: String,
    pub api_path: Option<String>,
    pub handler: ComfyRouteHandler,
}

impl SimControlPlaneRouteCapability {
    pub fn from_catalog(catalog: &ComfyRouteCatalog) -> Vec<Self> {
        catalog
            .routes()
            .map(|route| Self {
                kind: route.kind,
                method: route.method,
                path: route.legacy_path.clone(),
                api_path: route.api_path.clone(),
                handler: route.handler,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientFeatureNegotiation {
    pub client_id: String,
    pub requested: ComfyFeatureFlags,
    pub accepted: ComfyFeatureFlags,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PreviewPayload {
    LegacyBytes {
        mime_type: String,
        byte_count: u64,
    },
    Metadata {
        artifact_id: String,
        mime_type: String,
        width: Option<u32>,
        height: Option<u32>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyRuntimeEvent {
    Status(QueueStatus),
    Executing {
        prompt_id: ComfyPromptId,
        node_id: Option<NodeId>,
    },
    Progress {
        prompt_id: ComfyPromptId,
        node_id: NodeId,
        value: u64,
        max: u64,
    },
    Preview {
        prompt_id: ComfyPromptId,
        node_id: NodeId,
        payload: PreviewPayload,
    },
    FeatureFlags(ClientFeatureNegotiation),
}

fn invalid_prompt_id(value: &str) -> ComfyControlDiagnostic {
    ComfyControlDiagnostic {
        code: INVALID_PROMPT_ID_CODE.to_string(),
        message: format!("prompt id `{value}` must be a canonical lowercase hyphenated UUID"),
    }
}
