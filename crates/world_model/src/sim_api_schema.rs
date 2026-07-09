use serde::{Deserialize, Serialize};

use crate::{ComfyHttpMethod, ComfyRouteCatalog, ComfyRouteHandler};

pub const API_SCHEMA_MISSING_SCHEMA_CODE: &str = "world_model.api_schema.missing_schema";
pub const API_SCHEMA_MISSING_REASON_CODE: &str = "world_model.api_schema.missing_reason";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimApiRouteSupport {
    Implemented,
    Planned { reason: String },
    CloudOnly { reason: String },
    External { reason: String },
    Unsupported { reason: String },
}

impl SimApiRouteSupport {
    pub fn is_implemented(&self) -> bool {
        matches!(self, Self::Implemented)
    }

    fn reason(&self) -> Option<&str> {
        match self {
            Self::Implemented => None,
            Self::Planned { reason }
            | Self::CloudOnly { reason }
            | Self::External { reason }
            | Self::Unsupported { reason } => Some(reason),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimApiSchemaRoute {
    pub method: ComfyHttpMethod,
    pub path: String,
    pub support: SimApiRouteSupport,
    pub native_handler: Option<String>,
    pub schema_ref: Option<String>,
    pub notes: Option<String>,
}

impl SimApiSchemaRoute {
    pub fn implemented(
        method: ComfyHttpMethod,
        path: impl Into<String>,
        handler: ComfyRouteHandler,
        schema_ref: impl Into<String>,
    ) -> Self {
        Self {
            method,
            path: path.into(),
            support: SimApiRouteSupport::Implemented,
            native_handler: Some(handler_name(handler).to_string()),
            schema_ref: Some(schema_ref.into()),
            notes: None,
        }
    }

    pub fn documented(
        method: ComfyHttpMethod,
        path: impl Into<String>,
        support: SimApiRouteSupport,
    ) -> Self {
        Self {
            method,
            path: path.into(),
            support,
            native_handler: None,
            schema_ref: None,
            notes: None,
        }
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimApiSchemaCatalog {
    pub routes: Vec<SimApiSchemaRoute>,
}

impl SimApiSchemaCatalog {
    pub fn new(routes: impl IntoIterator<Item = SimApiSchemaRoute>) -> Self {
        let mut routes = routes.into_iter().collect::<Vec<_>>();
        routes.sort_by(|left, right| {
            (left.method, left.path.as_str()).cmp(&(right.method, right.path.as_str()))
        });
        Self { routes }
    }

    pub fn from_comfy_route_catalog(catalog: &ComfyRouteCatalog) -> Self {
        Self::new(catalog.routes().map(|route| {
            let path = route
                .api_path
                .as_deref()
                .unwrap_or(route.legacy_path.as_str());
            SimApiSchemaRoute::implemented(
                route.method,
                path,
                route.handler,
                schema_ref_for_path(path),
            )
        }))
    }

    pub fn route(&self, method: ComfyHttpMethod, path: &str) -> Option<&SimApiSchemaRoute> {
        self.routes
            .iter()
            .find(|route| route.method == method && route.path == path)
    }

    pub fn with_route(mut self, route: SimApiSchemaRoute) -> Self {
        self.routes
            .retain(|existing| !(existing.method == route.method && existing.path == route.path));
        self.routes.push(route);
        self.routes.sort_by(|left, right| {
            (left.method, left.path.as_str()).cmp(&(right.method, right.path.as_str()))
        });
        self
    }

    pub fn validate(&self) -> Result<(), Vec<SimApiSchemaDiagnostic>> {
        let diagnostics = self
            .routes
            .iter()
            .filter_map(validate_route)
            .collect::<Vec<_>>();
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimApiSchemaDiagnostic {
    pub code: String,
    pub method: ComfyHttpMethod,
    pub path: String,
    pub message: String,
}

fn validate_route(route: &SimApiSchemaRoute) -> Option<SimApiSchemaDiagnostic> {
    if route.support.is_implemented() && route.schema_ref.as_deref().unwrap_or_default().is_empty()
    {
        return Some(SimApiSchemaDiagnostic {
            code: API_SCHEMA_MISSING_SCHEMA_CODE.to_string(),
            method: route.method,
            path: route.path.clone(),
            message: "implemented routes require schema coverage".to_string(),
        });
    }

    if let Some(reason) = route.support.reason()
        && reason.trim().is_empty()
    {
        return Some(SimApiSchemaDiagnostic {
            code: API_SCHEMA_MISSING_REASON_CODE.to_string(),
            method: route.method,
            path: route.path.clone(),
            message: "non-local route statuses require a reason".to_string(),
        });
    }

    None
}

fn schema_ref_for_path(path: &str) -> String {
    let name = path
        .trim_start_matches("/api/")
        .trim_start_matches('/')
        .replace(['/', '{', '}'], "_")
        .trim_matches('_')
        .to_string();
    format!("#/paths/{name}")
}

fn handler_name(handler: ComfyRouteHandler) -> &'static str {
    match handler {
        ComfyRouteHandler::ControlPlane => "control_plane",
        ComfyRouteHandler::JobBridge => "job_bridge",
        ComfyRouteHandler::ModelCatalog => "model_catalog",
        ComfyRouteHandler::ObjectInfo => "object_info",
        ComfyRouteHandler::AssetLibrary => "asset_library",
        ComfyRouteHandler::ExtensionRegistry => "extension_registry",
        ComfyRouteHandler::WorkflowRegistry => "workflow_registry",
        ComfyRouteHandler::UserDataStore => "user_data_store",
    }
}
