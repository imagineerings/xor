use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub const ORIGIN_MISMATCH_CODE: &str = "world_model.comfy_http.origin_mismatch";
pub const PATH_ESCAPE_CODE: &str = "world_model.comfy_http.path_escape";
pub const UNKNOWN_ROOT_CODE: &str = "world_model.comfy_http.unknown_root";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyHttpSafetyDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyApiNodeMode {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyCacheClass {
    StaticAsset,
    Dynamic,
    Sensitive,
}

impl ComfyCacheClass {
    pub fn cache_control(self) -> &'static str {
        match self {
            Self::StaticAsset => "public, max-age=31536000, immutable",
            Self::Dynamic => "no-cache",
            Self::Sensitive => "no-store",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyContentDisposition {
    pub content_type: String,
    pub attachment: bool,
}

impl ComfyContentDisposition {
    pub fn safe_for_view(content_type: &str) -> Self {
        if is_executable_content_type(content_type) {
            Self {
                content_type: "application/octet-stream".to_string(),
                attachment: true,
            }
        } else {
            Self {
                content_type: content_type.to_string(),
                attachment: false,
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyContentSecurityPolicy {
    pub value: String,
}

impl ComfyContentSecurityPolicy {
    pub fn for_api_node_mode(mode: ComfyApiNodeMode) -> Self {
        let value = match mode {
            ComfyApiNodeMode::Enabled => {
                "default-src 'self'; connect-src 'self' http: https: ws: wss:"
            }
            ComfyApiNodeMode::Disabled => {
                "default-src 'self'; connect-src 'self'; script-src 'self'"
            }
        };
        Self {
            value: value.to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyPathRoots {
    roots: BTreeMap<String, PathBuf>,
}

impl ComfyPathRoots {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(mut self, id: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        self.roots.insert(id.into(), root.into());
        self
    }

    pub fn resolve(
        &self,
        root_id: &str,
        relative_path: impl AsRef<Path>,
    ) -> Result<PathBuf, ComfyHttpSafetyDiagnostic> {
        let root = self.roots.get(root_id).ok_or_else(|| {
            diagnostic(
                UNKNOWN_ROOT_CODE,
                format!("path root `{root_id}` is not registered"),
            )
        })?;
        let relative_path = relative_path.as_ref();
        if relative_path.is_absolute() || has_escaping_component(relative_path) {
            return Err(diagnostic(
                PATH_ESCAPE_CODE,
                "path must remain relative to the configured Comfy root",
            ));
        }
        Ok(root.join(relative_path))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyOriginCheck {
    pub request_host: String,
    pub origin: Option<String>,
}

impl ComfyOriginCheck {
    pub fn validate_loopback_origin(&self) -> Result<(), ComfyHttpSafetyDiagnostic> {
        let request_host = host_without_port(&self.request_host);
        if !is_loopback_host(request_host) {
            return Ok(());
        }

        let Some(origin) = self.origin.as_deref() else {
            return Ok(());
        };
        let Some(origin_host) = origin_host(origin) else {
            return Err(diagnostic(
                ORIGIN_MISMATCH_CODE,
                "origin header is not a valid HTTP origin",
            ));
        };
        if host_without_port(origin_host) == request_host {
            Ok(())
        } else {
            Err(diagnostic(
                ORIGIN_MISMATCH_CODE,
                "cross-site browser request to loopback Comfy route was rejected",
            ))
        }
    }
}

fn is_executable_content_type(content_type: &str) -> bool {
    let content_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    matches!(
        content_type.as_str(),
        "text/html"
            | "application/xhtml+xml"
            | "application/javascript"
            | "text/javascript"
            | "image/svg+xml"
    )
}

fn has_escaping_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost" || host == "::1" || host == "[::1]" || host.starts_with("127.")
}

fn origin_host(origin: &str) -> Option<&str> {
    let (_, rest) = origin.split_once("://")?;
    Some(rest.split('/').next().unwrap_or(rest))
}

fn host_without_port(host: &str) -> &str {
    if host.starts_with('[') {
        return host
            .split(']')
            .next()
            .map(|host| host.trim_start_matches('['))
            .unwrap_or(host);
    }
    host.split(':').next().unwrap_or(host)
}

fn diagnostic(code: &str, message: impl Into<String>) -> ComfyHttpSafetyDiagnostic {
    ComfyHttpSafetyDiagnostic {
        code: code.to_string(),
        message: message.into(),
    }
}
