use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const MISSING_ROUTE_ALIAS_CODE: &str = "world_model.comfy_routes.missing_alias";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComfyHttpMethod {
    Get,
    Post,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComfyRouteKind {
    AppSettingsRead,
    AppSettingsWrite,
    AppSettingRead,
    AppSettingWrite,
    PromptSubmission,
    Queue,
    QueueAction,
    History,
    HistoryAction,
    HistoryByPromptId,
    PromptStatus,
    Features,
    ObjectInfo,
    ObjectInfoByNodeClass,
    Models,
    ModelsByFolder,
    Embeddings,
    Extensions,
    Jobs,
    JobById,
    JobCancel,
    JobCancelById,
    Upload,
    UploadMask,
    View,
    ViewMetadataByFolder,
    Root,
    SystemStats,
    FreeResources,
    Interrupt,
    WebSocket,
    UserDataList,
    UserDataRead,
    UserDataWrite,
    UserDataDelete,
    UsersRead,
    UsersWrite,
    V2UserData,
    UserDataMove,
    I18n,
    WorkflowTemplates,
    ExperimentModels,
    ExperimentModelsByFolder,
    ExperimentModelPreview,
    NodeReplacements,
    GlobalSubgraphs,
    GlobalSubgraphById,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyRouteHandler {
    ControlPlane,
    JobBridge,
    ModelCatalog,
    ObjectInfo,
    AssetLibrary,
    ExtensionRegistry,
    WorkflowRegistry,
    UserDataStore,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyRouteDefinition {
    pub kind: ComfyRouteKind,
    pub method: ComfyHttpMethod,
    pub legacy_path: String,
    pub api_path: Option<String>,
    pub handler: ComfyRouteHandler,
}

impl ComfyRouteDefinition {
    pub fn aliases(&self) -> Vec<&str> {
        let mut aliases = vec![self.legacy_path.as_str()];
        if let Some(api_path) = &self.api_path {
            aliases.push(api_path.as_str());
        }
        aliases
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyRouteCatalog {
    routes: BTreeMap<ComfyRouteKind, ComfyRouteDefinition>,
    aliases: BTreeMap<(ComfyHttpMethod, String), ComfyRouteKind>,
}

impl ComfyRouteCatalog {
    pub fn new(routes: impl IntoIterator<Item = ComfyRouteDefinition>) -> Self {
        let mut catalog = Self::default();
        for route in routes {
            for alias in route.aliases() {
                catalog
                    .aliases
                    .insert((route.method, alias.to_string()), route.kind);
            }
            catalog.routes.insert(route.kind, route);
        }
        catalog
    }

    pub fn default_comfy_routes() -> Self {
        Self::new(default_routes())
    }

    pub fn route_for_path(
        &self,
        method: ComfyHttpMethod,
        path: &str,
    ) -> Option<&ComfyRouteDefinition> {
        self.aliases
            .get(&(method, path.to_string()))
            .and_then(|kind| self.routes.get(kind))
    }

    pub fn route(&self, kind: ComfyRouteKind) -> Option<&ComfyRouteDefinition> {
        self.routes.get(&kind)
    }

    pub fn routes(&self) -> impl Iterator<Item = &ComfyRouteDefinition> {
        self.routes.values()
    }

    pub fn aliases_for_kind(&self, kind: ComfyRouteKind) -> BTreeSet<&str> {
        self.route(kind)
            .map(|route| route.aliases().into_iter().collect())
            .unwrap_or_default()
    }

    pub fn validate_required_aliases(&self) -> Result<(), Vec<ComfyRouteDiagnostic>> {
        let diagnostics = self
            .routes
            .values()
            .filter(|route| route.api_path.is_none() && requires_api_alias(route.kind))
            .map(|route| ComfyRouteDiagnostic {
                code: MISSING_ROUTE_ALIAS_CODE.to_string(),
                path: route.legacy_path.clone(),
                message: "route requires a legacy and /api alias".to_string(),
            })
            .collect::<Vec<_>>();
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyRouteDiagnostic {
    pub code: String,
    pub path: String,
    pub message: String,
}

fn default_routes() -> Vec<ComfyRouteDefinition> {
    use ComfyHttpMethod::{Get, Post};
    use ComfyRouteHandler::{
        AssetLibrary, ControlPlane, ExtensionRegistry, JobBridge, ModelCatalog, ObjectInfo,
        UserDataStore, WorkflowRegistry,
    };
    use ComfyRouteKind::{
        AppSettingRead, AppSettingWrite, AppSettingsRead, AppSettingsWrite, Embeddings,
        ExperimentModelPreview, ExperimentModels, ExperimentModelsByFolder, Extensions, Features,
        FreeResources, GlobalSubgraphById, GlobalSubgraphs, History, HistoryAction,
        HistoryByPromptId, I18n, Interrupt, JobById, JobCancel, JobCancelById, Jobs, Models,
        ModelsByFolder, NodeReplacements, ObjectInfo as ObjectInfoRoute, ObjectInfoByNodeClass,
        PromptStatus, PromptSubmission, Queue, QueueAction, Root, SystemStats, Upload, UploadMask,
        UserDataDelete, UserDataList, UserDataMove, UserDataRead, UserDataWrite, UsersRead,
        UsersWrite, V2UserData, View, ViewMetadataByFolder, WebSocket, WorkflowTemplates,
    };

    vec![
        route(Root, Get, "/", None, ControlPlane),
        route(
            PromptSubmission,
            Post,
            "/prompt",
            Some("/api/prompt"),
            ControlPlane,
        ),
        route(PromptStatus, Get, "/prompt", Some("/api/prompt"), JobBridge),
        route(Queue, Get, "/queue", Some("/api/queue"), JobBridge),
        route(QueueAction, Post, "/queue", Some("/api/queue"), JobBridge),
        route(History, Get, "/history", Some("/api/history"), JobBridge),
        route(
            HistoryAction,
            Post,
            "/history",
            Some("/api/history"),
            JobBridge,
        ),
        route(
            HistoryByPromptId,
            Get,
            "/history/{prompt_id}",
            Some("/api/history/{prompt_id}"),
            JobBridge,
        ),
        route(
            Features,
            Get,
            "/features",
            Some("/api/features"),
            ControlPlane,
        ),
        route(
            ObjectInfoRoute,
            Get,
            "/object_info",
            Some("/api/object_info"),
            ObjectInfo,
        ),
        route(
            ObjectInfoByNodeClass,
            Get,
            "/object_info/{node_class}",
            Some("/api/object_info/{node_class}"),
            ObjectInfo,
        ),
        route(Models, Get, "/models", Some("/api/models"), ModelCatalog),
        route(
            ModelsByFolder,
            Get,
            "/models/{folder}",
            Some("/api/models/{folder}"),
            ModelCatalog,
        ),
        route(
            Embeddings,
            Get,
            "/embeddings",
            Some("/api/embeddings"),
            ModelCatalog,
        ),
        route(
            Extensions,
            Get,
            "/extensions",
            Some("/api/extensions"),
            ExtensionRegistry,
        ),
        route(SystemStats, Get, "/system_stats", None, ControlPlane),
        route(WebSocket, Get, "/ws", None, ControlPlane),
        route(Jobs, Get, "/jobs", Some("/api/jobs"), JobBridge),
        route(
            JobById,
            Get,
            "/jobs/{job_id}",
            Some("/api/jobs/{job_id}"),
            JobBridge,
        ),
        route(
            JobCancel,
            Post,
            "/jobs/cancel",
            Some("/api/jobs/cancel"),
            JobBridge,
        ),
        route(
            JobCancelById,
            Post,
            "/jobs/{job_id}/cancel",
            Some("/api/jobs/{job_id}/cancel"),
            JobBridge,
        ),
        route(
            Interrupt,
            Post,
            "/interrupt",
            Some("/api/interrupt"),
            JobBridge,
        ),
        route(
            FreeResources,
            Post,
            "/free",
            Some("/api/free"),
            ControlPlane,
        ),
        route(
            Upload,
            Post,
            "/upload/image",
            Some("/api/upload/image"),
            AssetLibrary,
        ),
        route(
            UploadMask,
            Post,
            "/upload/mask",
            Some("/api/upload/mask"),
            AssetLibrary,
        ),
        route(View, Get, "/view", Some("/api/view"), AssetLibrary),
        route(
            ViewMetadataByFolder,
            Get,
            "/view_metadata/{folder_name}",
            Some("/api/view_metadata/{folder_name}"),
            AssetLibrary,
        ),
        route(
            AppSettingsRead,
            Get,
            "/settings",
            Some("/api/settings"),
            ControlPlane,
        ),
        route(
            AppSettingsWrite,
            Post,
            "/settings",
            Some("/api/settings"),
            ControlPlane,
        ),
        route(
            AppSettingRead,
            Get,
            "/settings/{id}",
            Some("/api/settings/{id}"),
            ControlPlane,
        ),
        route(
            AppSettingWrite,
            Post,
            "/settings/{id}",
            Some("/api/settings/{id}"),
            ControlPlane,
        ),
        route(I18n, Get, "/i18n", Some("/api/i18n"), ExtensionRegistry),
        route(
            WorkflowTemplates,
            Get,
            "/workflow_templates",
            Some("/api/workflow_templates"),
            WorkflowRegistry,
        ),
        route(
            ExperimentModels,
            Get,
            "/experiment/models",
            Some("/api/experiment/models"),
            ModelCatalog,
        ),
        route(
            ExperimentModelsByFolder,
            Get,
            "/experiment/models/{folder}",
            Some("/api/experiment/models/{folder}"),
            ModelCatalog,
        ),
        route(
            ExperimentModelPreview,
            Get,
            "/experiment/models/preview/{folder}/{path_index}/{filename}",
            Some("/api/experiment/models/preview/{folder}/{path_index}/{filename}"),
            ModelCatalog,
        ),
        route(
            NodeReplacements,
            Get,
            "/node_replacements",
            Some("/api/node_replacements"),
            WorkflowRegistry,
        ),
        route(
            GlobalSubgraphs,
            Get,
            "/global_subgraphs",
            Some("/api/global_subgraphs"),
            WorkflowRegistry,
        ),
        route(
            GlobalSubgraphById,
            Get,
            "/global_subgraphs/{id}",
            Some("/api/global_subgraphs/{id}"),
            WorkflowRegistry,
        ),
        route(
            UserDataList,
            Get,
            "/userdata",
            Some("/api/userdata"),
            UserDataStore,
        ),
        route(
            V2UserData,
            Get,
            "/v2/userdata",
            Some("/api/v2/userdata"),
            UserDataStore,
        ),
        route(
            UserDataRead,
            Get,
            "/userdata/{file}",
            Some("/api/userdata/{file}"),
            UserDataStore,
        ),
        route(
            UserDataWrite,
            Post,
            "/userdata/{file}",
            Some("/api/userdata/{file}"),
            UserDataStore,
        ),
        route(
            UserDataDelete,
            ComfyHttpMethod::Delete,
            "/userdata/{file}",
            Some("/api/userdata/{file}"),
            UserDataStore,
        ),
        route(
            UserDataMove,
            Post,
            "/userdata/{file}/move/{dest}",
            Some("/api/userdata/{file}/move/{dest}"),
            UserDataStore,
        ),
        route(UsersRead, Get, "/users", Some("/api/users"), UserDataStore),
        route(
            UsersWrite,
            Post,
            "/users",
            Some("/api/users"),
            UserDataStore,
        ),
    ]
}

fn route(
    kind: ComfyRouteKind,
    method: ComfyHttpMethod,
    legacy_path: &str,
    api_path: Option<&str>,
    handler: ComfyRouteHandler,
) -> ComfyRouteDefinition {
    ComfyRouteDefinition {
        kind,
        method,
        legacy_path: legacy_path.to_string(),
        api_path: api_path.map(str::to_string),
        handler,
    }
}

fn requires_api_alias(kind: ComfyRouteKind) -> bool {
    matches!(
        kind,
        ComfyRouteKind::AppSettingsRead
            | ComfyRouteKind::AppSettingsWrite
            | ComfyRouteKind::AppSettingRead
            | ComfyRouteKind::AppSettingWrite
            | ComfyRouteKind::FreeResources
            | ComfyRouteKind::Interrupt
            | ComfyRouteKind::UserDataList
            | ComfyRouteKind::UserDataRead
            | ComfyRouteKind::UserDataWrite
            | ComfyRouteKind::UserDataDelete
            | ComfyRouteKind::UsersRead
            | ComfyRouteKind::UsersWrite
            | ComfyRouteKind::V2UserData
            | ComfyRouteKind::UserDataMove
            | ComfyRouteKind::I18n
            | ComfyRouteKind::WorkflowTemplates
            | ComfyRouteKind::ExperimentModels
            | ComfyRouteKind::ExperimentModelsByFolder
            | ComfyRouteKind::ExperimentModelPreview
            | ComfyRouteKind::NodeReplacements
            | ComfyRouteKind::GlobalSubgraphs
            | ComfyRouteKind::GlobalSubgraphById
            | ComfyRouteKind::JobById
            | ComfyRouteKind::JobCancel
            | ComfyRouteKind::JobCancelById
            | ComfyRouteKind::UploadMask
            | ComfyRouteKind::ViewMetadataByFolder
            | ComfyRouteKind::QueueAction
            | ComfyRouteKind::HistoryAction
            | ComfyRouteKind::PromptSubmission
            | ComfyRouteKind::Queue
            | ComfyRouteKind::History
            | ComfyRouteKind::HistoryByPromptId
            | ComfyRouteKind::PromptStatus
            | ComfyRouteKind::Features
            | ComfyRouteKind::ObjectInfo
            | ComfyRouteKind::ObjectInfoByNodeClass
            | ComfyRouteKind::Models
            | ComfyRouteKind::ModelsByFolder
            | ComfyRouteKind::Embeddings
            | ComfyRouteKind::Extensions
            | ComfyRouteKind::Jobs
            | ComfyRouteKind::Upload
            | ComfyRouteKind::View
    )
}
