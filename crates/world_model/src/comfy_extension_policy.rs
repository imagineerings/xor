use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    SIM_EXTENSION_POLICY_BLOCKED_CODE, SIM_EXTENSION_POLICY_DISABLED_CODE,
    SIM_EXTENSION_POLICY_INSTALL_DENIED_CODE, SIM_EXTENSION_POLICY_INSTALL_REVIEW_REQUIRED_CODE,
    SIM_EXTENSION_POLICY_NETWORK_DENIED_CODE, SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE,
    SIM_EXTENSION_POLICY_WEB_ASSET_DENIED_CODE, SimExtensionId, SimExtensionPolicyDiagnostic,
    SimExtensionPolicyDiagnosticSeverity, SimExtensionRecord,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimExtensionPolicyDecisionKind {
    Enabled,
    Disabled,
    Whitelisted,
    Blocked,
}

impl SimExtensionPolicyDecisionKind {
    pub fn allows_loading(self) -> bool {
        matches!(self, Self::Enabled | Self::Whitelisted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionPolicyRequest {
    pub requires_script: bool,
    pub requires_web_assets: bool,
    pub requires_network: bool,
    pub requires_install: bool,
}

impl SimExtensionPolicyRequest {
    pub fn metadata_only() -> Self {
        Self {
            requires_script: false,
            requires_web_assets: false,
            requires_network: false,
            requires_install: false,
        }
    }

    pub fn with_script(mut self, required: bool) -> Self {
        self.requires_script = required;
        self
    }

    pub fn with_web_assets(mut self, required: bool) -> Self {
        self.requires_web_assets = required;
        self
    }

    pub fn with_network(mut self, required: bool) -> Self {
        self.requires_network = required;
        self
    }

    pub fn with_install(mut self, required: bool) -> Self {
        self.requires_install = required;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionPermissionReport {
    pub script_allowed: bool,
    pub web_assets_allowed: bool,
    pub network_allowed: bool,
    pub install_allowed: bool,
}

impl SimExtensionPermissionReport {
    fn denied() -> Self {
        Self {
            script_allowed: false,
            web_assets_allowed: false,
            network_allowed: false,
            install_allowed: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionPolicyEvaluation {
    pub extension_id: SimExtensionId,
    pub decision: SimExtensionPolicyDecisionKind,
    pub permissions: SimExtensionPermissionReport,
    pub diagnostics: Vec<SimExtensionPolicyDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionPolicy {
    custom_nodes_enabled: bool,
    whitelist: BTreeSet<SimExtensionId>,
    blocked: BTreeMap<SimExtensionId, String>,
    script_allowlist: BTreeSet<SimExtensionId>,
    web_asset_allowlist: BTreeSet<SimExtensionId>,
    network_allowlist: BTreeSet<SimExtensionId>,
    install_allowlist: BTreeSet<SimExtensionId>,
    dependency_reviewed_installs: BTreeSet<SimExtensionId>,
}

impl Default for SimExtensionPolicy {
    fn default() -> Self {
        Self {
            custom_nodes_enabled: true,
            whitelist: BTreeSet::new(),
            blocked: BTreeMap::new(),
            script_allowlist: BTreeSet::new(),
            web_asset_allowlist: BTreeSet::new(),
            network_allowlist: BTreeSet::new(),
            install_allowlist: BTreeSet::new(),
            dependency_reviewed_installs: BTreeSet::new(),
        }
    }
}

impl SimExtensionPolicy {
    pub fn with_custom_nodes_enabled(mut self, enabled: bool) -> Self {
        self.custom_nodes_enabled = enabled;
        self
    }

    pub fn with_whitelisted_pack(mut self, pack_name: impl AsRef<str>) -> Self {
        self.whitelist.insert(SimExtensionId::new(pack_name));
        self
    }

    pub fn with_blocked_pack(
        mut self,
        pack_name: impl AsRef<str>,
        reason: impl Into<String>,
    ) -> Self {
        self.blocked
            .insert(SimExtensionId::new(pack_name), reason.into());
        self
    }

    pub fn with_script_allowed(mut self, pack_name: impl AsRef<str>) -> Self {
        self.script_allowlist.insert(SimExtensionId::new(pack_name));
        self
    }

    pub fn with_web_assets_allowed(mut self, pack_name: impl AsRef<str>) -> Self {
        self.web_asset_allowlist
            .insert(SimExtensionId::new(pack_name));
        self
    }

    pub fn with_network_allowed(mut self, pack_name: impl AsRef<str>) -> Self {
        self.network_allowlist
            .insert(SimExtensionId::new(pack_name));
        self
    }

    pub fn with_install_allowed(mut self, pack_name: impl AsRef<str>) -> Self {
        self.install_allowlist
            .insert(SimExtensionId::new(pack_name));
        self
    }

    pub fn with_dependency_reviewed_install(mut self, pack_name: impl AsRef<str>) -> Self {
        self.dependency_reviewed_installs
            .insert(SimExtensionId::new(pack_name));
        self
    }

    pub fn evaluate(
        &self,
        extension: &SimExtensionRecord,
        request: &SimExtensionPolicyRequest,
    ) -> SimExtensionPolicyEvaluation {
        let extension_id = extension.id.clone();
        let mut diagnostics = Vec::new();
        let decision = self.decision_for(&extension_id, &mut diagnostics);
        let permissions = if decision.allows_loading() {
            self.permissions_for(&extension_id, request, &mut diagnostics)
        } else {
            SimExtensionPermissionReport::denied()
        };

        SimExtensionPolicyEvaluation {
            extension_id,
            decision,
            permissions,
            diagnostics,
        }
    }

    fn decision_for(
        &self,
        extension_id: &SimExtensionId,
        diagnostics: &mut Vec<SimExtensionPolicyDiagnostic>,
    ) -> SimExtensionPolicyDecisionKind {
        if let Some(reason) = self.blocked.get(extension_id) {
            diagnostics.push(SimExtensionPolicyDiagnostic::new(
                SIM_EXTENSION_POLICY_BLOCKED_CODE,
                extension_id.clone(),
                SimExtensionPolicyDiagnosticSeverity::Error,
                reason.clone(),
            ));
            return SimExtensionPolicyDecisionKind::Blocked;
        }

        if self.whitelist.contains(extension_id) {
            return SimExtensionPolicyDecisionKind::Whitelisted;
        }

        if !self.custom_nodes_enabled {
            diagnostics.push(SimExtensionPolicyDiagnostic::new(
                SIM_EXTENSION_POLICY_DISABLED_CODE,
                extension_id.clone(),
                SimExtensionPolicyDiagnosticSeverity::Warning,
                "custom nodes are disabled and this extension is not whitelisted",
            ));
            return SimExtensionPolicyDecisionKind::Disabled;
        }

        if !self.whitelist.is_empty() {
            diagnostics.push(SimExtensionPolicyDiagnostic::new(
                SIM_EXTENSION_POLICY_DISABLED_CODE,
                extension_id.clone(),
                SimExtensionPolicyDiagnosticSeverity::Warning,
                "extension is not present in the active custom node whitelist",
            ));
            return SimExtensionPolicyDecisionKind::Disabled;
        }

        SimExtensionPolicyDecisionKind::Enabled
    }

    fn permissions_for(
        &self,
        extension_id: &SimExtensionId,
        request: &SimExtensionPolicyRequest,
        diagnostics: &mut Vec<SimExtensionPolicyDiagnostic>,
    ) -> SimExtensionPermissionReport {
        let script_allowed = self.script_allowlist.contains(extension_id);
        let web_assets_allowed = self.web_asset_allowlist.contains(extension_id);
        let network_allowed = self.network_allowlist.contains(extension_id);
        let install_permission_allowed = self.install_allowlist.contains(extension_id);
        let install_reviewed = self.dependency_reviewed_installs.contains(extension_id);
        let install_allowed = install_permission_allowed && install_reviewed;

        if request.requires_script && !script_allowed {
            diagnostics.push(permission_diagnostic(
                SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE,
                extension_id,
                "extension prestartup or import scripts are not allowed by Sim policy",
            ));
        }
        if request.requires_web_assets && !web_assets_allowed {
            diagnostics.push(permission_diagnostic(
                SIM_EXTENSION_POLICY_WEB_ASSET_DENIED_CODE,
                extension_id,
                "extension web assets are not allowed by Sim policy",
            ));
        }
        if request.requires_network && !network_allowed {
            diagnostics.push(permission_diagnostic(
                SIM_EXTENSION_POLICY_NETWORK_DENIED_CODE,
                extension_id,
                "extension network access is not allowed by Sim policy",
            ));
        }
        if request.requires_install && !install_permission_allowed {
            diagnostics.push(permission_diagnostic(
                SIM_EXTENSION_POLICY_INSTALL_DENIED_CODE,
                extension_id,
                "extension install or update actions are not allowed by Sim policy",
            ));
        } else if request.requires_install && !install_reviewed {
            diagnostics.push(permission_diagnostic(
                SIM_EXTENSION_POLICY_INSTALL_REVIEW_REQUIRED_CODE,
                extension_id,
                "extension install or update actions require dependency review approval",
            ));
        }

        SimExtensionPermissionReport {
            script_allowed,
            web_assets_allowed,
            network_allowed,
            install_allowed,
        }
    }
}

fn permission_diagnostic(
    code: &str,
    extension_id: &SimExtensionId,
    message: impl Into<String>,
) -> SimExtensionPolicyDiagnostic {
    SimExtensionPolicyDiagnostic::new(
        code,
        extension_id.clone(),
        SimExtensionPolicyDiagnosticSeverity::Error,
        message,
    )
}
