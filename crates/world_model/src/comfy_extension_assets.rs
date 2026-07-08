use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{ComfyCacheClass, ComfyPathRoots, SimExtensionId, SimExtensionRecord};

pub const SIM_EXTENSION_ASSET_DEPRECATED_PATH_CODE: &str =
    "world_model.extension_assets.deprecated_path";
pub const SIM_EXTENSION_ASSET_PATH_ESCAPE_CODE: &str = "world_model.extension_assets.path_escape";
pub const SIM_EXTENSION_ASSET_UNKNOWN_ROOT_CODE: &str = "world_model.extension_assets.unknown_root";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimExtensionAssetKind {
    Web,
    Template,
}

impl SimExtensionAssetKind {
    fn route_segment(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Template => "templates",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimExtensionAssetRootId(String);

impl SimExtensionAssetRootId {
    pub fn new(extension_id: &SimExtensionId, kind: SimExtensionAssetKind) -> Self {
        Self(format!(
            "{}:{}",
            extension_id.as_str(),
            kind.route_segment()
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionAssetRoot {
    pub id: SimExtensionAssetRootId,
    pub extension_id: SimExtensionId,
    pub kind: SimExtensionAssetKind,
    pub root_path: PathBuf,
    pub route_prefix: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionAssetResponse {
    pub extension_id: SimExtensionId,
    pub kind: SimExtensionAssetKind,
    pub relative_path: String,
    pub file_path: PathBuf,
    pub route_path: String,
    pub content_type: String,
    pub attachment: bool,
    pub cache_control: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionAssetDiagnostic {
    pub code: String,
    pub extension_id: Option<SimExtensionId>,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionAssetService {
    roots: BTreeMap<SimExtensionAssetRootId, SimExtensionAssetRoot>,
    diagnostics: Vec<SimExtensionAssetDiagnostic>,
}

impl SimExtensionAssetService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_web_root(
        &mut self,
        extension: &SimExtensionRecord,
        root_path: impl Into<PathBuf>,
    ) -> SimExtensionAssetRootId {
        self.register_root(extension, SimExtensionAssetKind::Web, root_path)
    }

    pub fn register_template_root(
        &mut self,
        extension: &SimExtensionRecord,
        root_path: impl Into<PathBuf>,
    ) -> SimExtensionAssetRootId {
        self.register_root(extension, SimExtensionAssetKind::Template, root_path)
    }

    pub fn resolve(
        &mut self,
        root_id: &SimExtensionAssetRootId,
        relative_path: impl AsRef<Path>,
    ) -> Result<SimExtensionAssetResponse, SimExtensionAssetDiagnostic> {
        let roots = self
            .roots
            .values()
            .fold(ComfyPathRoots::new(), |roots, root| {
                roots.with_root(root.id.as_str(), root.root_path.clone())
            });
        let root = self.roots.get(root_id).ok_or_else(|| {
            diagnostic(
                SIM_EXTENSION_ASSET_UNKNOWN_ROOT_CODE,
                None,
                None,
                format!(
                    "extension asset root `{}` is not registered",
                    root_id.as_str()
                ),
            )
        })?;
        let relative_path = relative_path.as_ref();
        let resolved_path = roots
            .resolve(root_id.as_str(), relative_path)
            .map_err(|error| {
                let code = match error.code.as_str() {
                    crate::PATH_ESCAPE_CODE => SIM_EXTENSION_ASSET_PATH_ESCAPE_CODE,
                    crate::UNKNOWN_ROOT_CODE => SIM_EXTENSION_ASSET_UNKNOWN_ROOT_CODE,
                    _ => error.code.as_str(),
                };
                diagnostic(
                    code,
                    Some(root.extension_id.clone()),
                    Some(relative_path.display().to_string()),
                    error.message,
                )
            })?;
        let relative_path = relative_path.display().to_string();
        let (content_type, attachment) = content_type_for_path(&resolved_path);
        Ok(SimExtensionAssetResponse {
            extension_id: root.extension_id.clone(),
            kind: root.kind,
            route_path: format!("{}/{}", root.route_prefix, route_escape(&relative_path)),
            relative_path,
            file_path: resolved_path,
            content_type,
            attachment,
            cache_control: ComfyCacheClass::StaticAsset.cache_control().to_string(),
        })
    }

    pub fn resolve_deprecated_path(
        &mut self,
        root_id: &SimExtensionAssetRootId,
        deprecated_path: impl AsRef<Path>,
    ) -> Result<SimExtensionAssetResponse, SimExtensionAssetDiagnostic> {
        let deprecated_path = deprecated_path.as_ref();
        let extension_id = self
            .roots
            .get(root_id)
            .map(|root| root.extension_id.clone());
        self.diagnostics.push(diagnostic(
            SIM_EXTENSION_ASSET_DEPRECATED_PATH_CODE,
            extension_id,
            Some(deprecated_path.display().to_string()),
            "deprecated extension asset path resolved through native Sim asset service",
        ));
        self.resolve(root_id, deprecated_path)
    }

    pub fn roots(&self) -> impl Iterator<Item = &SimExtensionAssetRoot> {
        self.roots.values()
    }

    pub fn diagnostics(&self) -> &[SimExtensionAssetDiagnostic] {
        &self.diagnostics
    }

    fn register_root(
        &mut self,
        extension: &SimExtensionRecord,
        kind: SimExtensionAssetKind,
        root_path: impl Into<PathBuf>,
    ) -> SimExtensionAssetRootId {
        let id = SimExtensionAssetRootId::new(&extension.id, kind);
        let route_prefix = format!(
            "/sim/extensions/{}/{}/{}",
            extension.id.as_str(),
            kind.route_segment(),
            stable_route_suffix(extension.source_path.as_path(), kind)
        );
        self.roots.insert(
            id.clone(),
            SimExtensionAssetRoot {
                id: id.clone(),
                extension_id: extension.id.clone(),
                kind,
                root_path: root_path.into(),
                route_prefix,
            },
        );
        id
    }
}

fn content_type_for_path(path: &Path) -> (String, bool) {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "css" => ("text/css".to_string(), false),
        "gif" => ("image/gif".to_string(), false),
        "jpg" | "jpeg" => ("image/jpeg".to_string(), false),
        "js" | "mjs" => ("application/javascript".to_string(), false),
        "json" | "map" => ("application/json".to_string(), false),
        "png" => ("image/png".to_string(), false),
        "wasm" => ("application/wasm".to_string(), false),
        "webp" => ("image/webp".to_string(), false),
        "woff" => ("font/woff".to_string(), false),
        "woff2" => ("font/woff2".to_string(), false),
        _ => ("application/octet-stream".to_string(), true),
    }
}

fn route_escape(path: &str) -> String {
    path.replace('\\', "/")
}

fn stable_route_suffix(path: &Path, kind: SimExtensionAssetKind) -> String {
    let mut hash = match kind {
        SimExtensionAssetKind::Web => 0xcbf29ce484222325,
        SimExtensionAssetKind::Template => 0x9e3779b185ebca87,
    };
    for byte in path.display().to_string().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn diagnostic(
    code: impl Into<String>,
    extension_id: Option<SimExtensionId>,
    path: Option<String>,
    message: impl Into<String>,
) -> SimExtensionAssetDiagnostic {
    SimExtensionAssetDiagnostic {
        code: code.into(),
        extension_id,
        path,
        message: message.into(),
    }
}
