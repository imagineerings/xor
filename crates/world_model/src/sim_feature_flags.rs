use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const FEATURE_FLAG_INVALID_VALUE_CODE: &str = "world_model.feature_flags.invalid_value";
pub const FEATURE_FLAG_CORE_OVERRIDE_CODE: &str = "world_model.feature_flags.core_override";
pub const FEATURE_PACKAGE_MISSING_CODE: &str = "world_model.feature_flags.package_missing";
pub const FEATURE_PACKAGE_OUTDATED_CODE: &str = "world_model.feature_flags.package_outdated";

pub const PREVIEW_METADATA_FLAG: &str = "preview_metadata";
pub const UPLOAD_SIZE_FLAG: &str = "upload_size";
pub const MANAGER_SUPPORT_FLAG: &str = "manager_support";
pub const NODE_REPLACEMENTS_FLAG: &str = "node_replacements";
pub const ASSETS_FLAG: &str = "assets";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimFeatureFlagDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimFeatureFlagDiagnostic {
    pub code: String,
    pub severity: SimFeatureFlagDiagnosticSeverity,
    pub name: String,
    pub message: String,
}

impl SimFeatureFlagDiagnostic {
    fn error(code: &str, name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            severity: SimFeatureFlagDiagnosticSeverity::Error,
            name: name.into(),
            message: message.into(),
        }
    }

    fn warning(code: &str, name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            severity: SimFeatureFlagDiagnosticSeverity::Warning,
            name: name.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimFeatureFlags {
    pub flags: BTreeMap<String, bool>,
}

impl SimFeatureFlags {
    pub fn with_flag(mut self, name: impl Into<String>, enabled: bool) -> Self {
        self.flags.insert(name.into(), enabled);
        self
    }

    pub fn enabled(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimClientFeatureNegotiation {
    pub client_id: String,
    pub requested: SimFeatureFlags,
    pub accepted: SimFeatureFlags,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimPackageKind {
    Frontend,
    WorkflowTemplates,
    EmbeddedDocs,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimPackageRequirement {
    pub name: String,
    pub kind: SimPackageKind,
    pub required_version: String,
    pub installed_version: Option<String>,
}

impl SimPackageRequirement {
    pub fn new(
        name: impl Into<String>,
        kind: SimPackageKind,
        required_version: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            required_version: required_version.into(),
            installed_version: None,
        }
    }

    pub fn with_installed_version(mut self, installed_version: impl Into<String>) -> Self {
        self.installed_version = Some(installed_version.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimFeatureFlagRegistry {
    core_flags: SimFeatureFlags,
    cli_flags: SimFeatureFlags,
    client_flags: BTreeMap<String, SimFeatureFlags>,
}

impl Default for SimFeatureFlagRegistry {
    fn default() -> Self {
        Self {
            core_flags: SimFeatureFlags::default()
                .with_flag(PREVIEW_METADATA_FLAG, true)
                .with_flag(UPLOAD_SIZE_FLAG, true)
                .with_flag(MANAGER_SUPPORT_FLAG, false)
                .with_flag(NODE_REPLACEMENTS_FLAG, true)
                .with_flag(ASSETS_FLAG, true),
            cli_flags: SimFeatureFlags::default(),
            client_flags: BTreeMap::new(),
        }
    }
}

impl SimFeatureFlagRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_core_flag(mut self, name: impl Into<String>, enabled: bool) -> Self {
        self.core_flags = self.core_flags.with_flag(name, enabled);
        self
    }

    pub fn apply_cli_flag(
        &mut self,
        name: impl Into<String>,
        value: impl AsRef<str>,
    ) -> Result<(), SimFeatureFlagDiagnostic> {
        let name = name.into();
        if self.core_flags.flags.contains_key(&name) {
            return Err(SimFeatureFlagDiagnostic::warning(
                FEATURE_FLAG_CORE_OVERRIDE_CODE,
                name,
                "CLI-provided feature flags cannot overwrite core server flags",
            ));
        }

        let enabled = coerce_flag_value(value.as_ref()).map_err(|message| {
            SimFeatureFlagDiagnostic::error(FEATURE_FLAG_INVALID_VALUE_CODE, &name, message)
        })?;
        self.cli_flags.flags.insert(name, enabled);
        Ok(())
    }

    pub fn server_features(&self) -> SimFeatureFlags {
        let mut features = self.core_flags.clone();
        for (name, enabled) in &self.cli_flags.flags {
            features.flags.insert(name.clone(), *enabled);
        }
        features
    }

    pub fn negotiate_client_features(
        &mut self,
        client_id: impl Into<String>,
        requested: SimFeatureFlags,
    ) -> SimClientFeatureNegotiation {
        let client_id = client_id.into();
        let server_features = self.server_features();
        let mut accepted = SimFeatureFlags::default();
        for (name, requested_enabled) in &requested.flags {
            accepted = accepted.with_flag(
                name.clone(),
                *requested_enabled && server_features.enabled(name),
            );
        }
        self.client_flags
            .insert(client_id.clone(), accepted.clone());

        SimClientFeatureNegotiation {
            client_id,
            requested,
            accepted,
        }
    }

    pub fn client_features(&self, client_id: &str) -> Option<&SimFeatureFlags> {
        self.client_flags.get(client_id)
    }

    pub fn diagnose_packages(
        &self,
        packages: impl IntoIterator<Item = SimPackageRequirement>,
    ) -> Vec<SimFeatureFlagDiagnostic> {
        packages
            .into_iter()
            .filter_map(|package| diagnose_package(package))
            .collect()
    }
}

pub fn coerce_flag_value(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
        "false" | "0" | "no" | "off" | "disabled" => Ok(false),
        _ => Err("feature flag value must be boolean-like".to_string()),
    }
}

fn diagnose_package(package: SimPackageRequirement) -> Option<SimFeatureFlagDiagnostic> {
    let Some(installed_version) = package.installed_version else {
        return Some(SimFeatureFlagDiagnostic::error(
            FEATURE_PACKAGE_MISSING_CODE,
            package.name,
            package_message(package.kind, "is missing"),
        ));
    };

    if compare_versions(&installed_version, &package.required_version) == Some(Ordering::Less) {
        Some(SimFeatureFlagDiagnostic::error(
            FEATURE_PACKAGE_OUTDATED_CODE,
            package.name,
            package_message(package.kind, "is outdated"),
        ))
    } else {
        None
    }
}

fn package_message(kind: SimPackageKind, state: &str) -> String {
    match kind {
        SimPackageKind::Frontend => format!("frontend package {state}"),
        SimPackageKind::WorkflowTemplates => format!("workflow template package {state}"),
        SimPackageKind::EmbeddedDocs => format!("embedded docs package {state}"),
    }
}

fn compare_versions(installed: &str, required: &str) -> Option<Ordering> {
    let installed = parse_dotted_version(installed)?;
    let required = parse_dotted_version(required)?;
    Some(installed.cmp(&required))
}

fn parse_dotted_version(value: &str) -> Option<Vec<u64>> {
    value
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}
