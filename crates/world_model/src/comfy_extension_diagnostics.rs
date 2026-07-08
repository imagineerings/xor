use serde::{Deserialize, Serialize};

use crate::SimExtensionId;

pub const SIM_EXTENSION_POLICY_BLOCKED_CODE: &str = "world_model.extensions.policy.blocked";
pub const SIM_EXTENSION_POLICY_DISABLED_CODE: &str = "world_model.extensions.policy.disabled";
pub const SIM_EXTENSION_POLICY_INSTALL_DENIED_CODE: &str =
    "world_model.extensions.policy.install_denied";
pub const SIM_EXTENSION_POLICY_INSTALL_REVIEW_REQUIRED_CODE: &str =
    "world_model.extensions.policy.install_review_required";
pub const SIM_EXTENSION_POLICY_NETWORK_DENIED_CODE: &str =
    "world_model.extensions.policy.network_denied";
pub const SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE: &str =
    "world_model.extensions.policy.script_denied";
pub const SIM_EXTENSION_POLICY_WEB_ASSET_DENIED_CODE: &str =
    "world_model.extensions.policy.web_asset_denied";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimExtensionPolicyDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionPolicyDiagnostic {
    pub code: String,
    pub extension_id: SimExtensionId,
    pub severity: SimExtensionPolicyDiagnosticSeverity,
    pub message: String,
}

impl SimExtensionPolicyDiagnostic {
    pub fn new(
        code: impl Into<String>,
        extension_id: SimExtensionId,
        severity: SimExtensionPolicyDiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            extension_id,
            severity,
            message: message.into(),
        }
    }
}
