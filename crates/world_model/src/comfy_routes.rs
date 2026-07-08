use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const MISSING_ROUTE_ALIAS_CODE: &str = "world_model.comfy_routes.missing_alias";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComfyHttpMethod {
    Get,
    Post,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComfyRouteKind {
    PromptSubmission,
    Queue,
    History,
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
    Upload,
    View,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyRouteHandler {
    ControlPlane,
    JobBridge,
    ModelCatalog,
    ObjectInfo,
    AssetLibrary,
    ExtensionRegistry,
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
    };
    use ComfyRouteKind::{
        Embeddings, Extensions, Features, History, HistoryByPromptId, Jobs, Models, ModelsByFolder,
        ObjectInfo as ObjectInfoRoute, ObjectInfoByNodeClass, PromptStatus, PromptSubmission,
        Queue, Upload, View,
    };

    vec![
        route(
            PromptSubmission,
            Post,
            "/prompt",
            Some("/api/prompt"),
            ControlPlane,
        ),
        route(Queue, Get, "/queue", Some("/api/queue"), JobBridge),
        route(History, Get, "/history", Some("/api/history"), JobBridge),
        route(
            HistoryByPromptId,
            Get,
            "/history/{prompt_id}",
            Some("/api/history/{prompt_id}"),
            JobBridge,
        ),
        route(PromptStatus, Get, "/prompt", Some("/api/prompt"), JobBridge),
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
        route(Jobs, Get, "/jobs", Some("/api/jobs"), JobBridge),
        route(
            Upload,
            Post,
            "/upload/image",
            Some("/api/upload/image"),
            AssetLibrary,
        ),
        route(View, Get, "/view", Some("/api/view"), AssetLibrary),
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
        ComfyRouteKind::PromptSubmission
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
