use serde::{Deserialize, Serialize};

use crate::{
    SimDependencyProposal, SimDependencyReviewDiagnostic, SimDependencyReviewGate, SimExtensionId,
    SimExtensionPolicy, SimExtensionPolicyDecisionKind, SimExtensionPolicyRequest,
    SimExtensionRecord,
};

pub const SIM_MANAGER_ACTION_DENIED_CODE: &str = "world_model.manager.action_denied";
pub const SIM_MANAGER_APPROVAL_REQUIRED_CODE: &str = "world_model.manager.approval_required";
pub const SIM_MANAGER_BACKGROUND_DENIED_CODE: &str = "world_model.manager.background_denied";
pub const SIM_MANAGER_DEPENDENCY_REVIEW_DENIED_CODE: &str =
    "world_model.manager.dependency_review_denied";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimManagerActionKind {
    Status,
    Install,
    Update,
    Disable,
    BackgroundOperation,
}

impl SimManagerActionKind {
    fn requires_write(self) -> bool {
        matches!(self, Self::Install | Self::Update | Self::Disable)
    }

    fn requires_install_policy(self) -> bool {
        matches!(self, Self::Install | Self::Update)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerApproval {
    pub approved_by: String,
    pub approved_at: String,
    pub reason: String,
}

impl SimManagerApproval {
    pub fn new(
        approved_by: impl Into<String>,
        approved_at: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            approved_by: approved_by.into(),
            approved_at: approved_at.into(),
            reason: reason.into(),
        }
    }

    fn is_complete(&self) -> bool {
        !self.approved_by.trim().is_empty()
            && !self.approved_at.trim().is_empty()
            && !self.reason.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerStatus {
    pub manager_routes_enabled: bool,
    pub background_operations_enabled: bool,
    pub managed_extensions: Vec<SimExtensionId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerActionRequest {
    pub action: SimManagerActionKind,
    pub extension: SimExtensionRecord,
    pub requires_network: bool,
    pub requires_filesystem_write: bool,
    pub dependency_proposals: Vec<SimDependencyProposal>,
    pub approval: Option<SimManagerApproval>,
}

impl SimManagerActionRequest {
    pub fn new(action: SimManagerActionKind, extension: SimExtensionRecord) -> Self {
        Self {
            action,
            extension,
            requires_network: false,
            requires_filesystem_write: action.requires_write(),
            dependency_proposals: Vec::new(),
            approval: None,
        }
    }

    pub fn with_network(mut self, required: bool) -> Self {
        self.requires_network = required;
        self
    }

    pub fn with_filesystem_write(mut self, required: bool) -> Self {
        self.requires_filesystem_write = required;
        self
    }

    pub fn with_dependency(mut self, proposal: SimDependencyProposal) -> Self {
        self.dependency_proposals.push(proposal);
        self
    }

    pub fn with_approval(mut self, approval: SimManagerApproval) -> Self {
        self.approval = Some(approval);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerDiagnostic {
    pub code: String,
    pub extension_id: SimExtensionId,
    pub action: SimManagerActionKind,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerActionEvaluation {
    pub action: SimManagerActionKind,
    pub extension_id: SimExtensionId,
    pub allowed: bool,
    pub policy_decision: SimExtensionPolicyDecisionKind,
    pub diagnostics: Vec<SimManagerDiagnostic>,
    pub dependency_diagnostics: Vec<SimDependencyReviewDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerBoundary {
    manager_routes_enabled: bool,
    background_operations_enabled: bool,
    policy: SimExtensionPolicy,
    dependency_review_gate: SimDependencyReviewGate,
    managed_extensions: Vec<SimExtensionId>,
}

impl Default for SimManagerBoundary {
    fn default() -> Self {
        Self {
            manager_routes_enabled: false,
            background_operations_enabled: false,
            policy: SimExtensionPolicy::default(),
            dependency_review_gate: SimDependencyReviewGate::new(),
            managed_extensions: Vec::new(),
        }
    }
}

impl SimManagerBoundary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_manager_routes_enabled(mut self, enabled: bool) -> Self {
        self.manager_routes_enabled = enabled;
        self
    }

    pub fn with_background_operations_enabled(mut self, enabled: bool) -> Self {
        self.background_operations_enabled = enabled;
        self
    }

    pub fn with_policy(mut self, policy: SimExtensionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_dependency_review_gate(mut self, gate: SimDependencyReviewGate) -> Self {
        self.dependency_review_gate = gate;
        self
    }

    pub fn with_managed_extension(mut self, extension_id: SimExtensionId) -> Self {
        self.managed_extensions.push(extension_id);
        self
    }

    pub fn status(&self) -> SimManagerStatus {
        SimManagerStatus {
            manager_routes_enabled: self.manager_routes_enabled,
            background_operations_enabled: self.background_operations_enabled,
            managed_extensions: self.managed_extensions.clone(),
        }
    }

    pub fn evaluate(&self, request: SimManagerActionRequest) -> SimManagerActionEvaluation {
        let extension_id = request.extension.id.clone();
        let mut diagnostics = Vec::new();
        let policy_request = SimExtensionPolicyRequest::metadata_only()
            .with_network(request.requires_network)
            .with_install(request.action.requires_install_policy());
        let policy_evaluation = self.policy.evaluate(&request.extension, &policy_request);

        if request.action != SimManagerActionKind::Status && !self.manager_routes_enabled {
            diagnostics.push(manager_diagnostic(
                SIM_MANAGER_ACTION_DENIED_CODE,
                extension_id.clone(),
                request.action,
                "manager routes are disabled by Sim policy",
            ));
        }

        if request.action == SimManagerActionKind::BackgroundOperation
            && !self.background_operations_enabled
        {
            diagnostics.push(manager_diagnostic(
                SIM_MANAGER_BACKGROUND_DENIED_CODE,
                extension_id.clone(),
                request.action,
                "manager background operations are disabled by Sim policy",
            ));
        }

        if (request.requires_filesystem_write || request.requires_network)
            && !request
                .approval
                .as_ref()
                .is_some_and(SimManagerApproval::is_complete)
        {
            diagnostics.push(manager_diagnostic(
                SIM_MANAGER_APPROVAL_REQUIRED_CODE,
                extension_id.clone(),
                request.action,
                "manager action requires explicit user approval before network or filesystem writes",
            ));
        }

        for diagnostic in policy_evaluation.diagnostics {
            diagnostics.push(manager_diagnostic(
                SIM_MANAGER_ACTION_DENIED_CODE,
                extension_id.clone(),
                request.action,
                diagnostic.message,
            ));
        }

        let dependency_review = self
            .dependency_review_gate
            .evaluate(request.dependency_proposals);
        let dependency_diagnostics = dependency_review.diagnostics().cloned().collect::<Vec<_>>();
        if !dependency_diagnostics.is_empty() {
            diagnostics.push(manager_diagnostic(
                SIM_MANAGER_DEPENDENCY_REVIEW_DENIED_CODE,
                extension_id.clone(),
                request.action,
                "manager dependency proposals require approved Sim dependency review",
            ));
        }

        SimManagerActionEvaluation {
            action: request.action,
            extension_id,
            allowed: diagnostics.is_empty() && dependency_review.is_allowed(),
            policy_decision: policy_evaluation.decision,
            diagnostics,
            dependency_diagnostics,
        }
    }
}

fn manager_diagnostic(
    code: impl Into<String>,
    extension_id: SimExtensionId,
    action: SimManagerActionKind,
    message: impl Into<String>,
) -> SimManagerDiagnostic {
    SimManagerDiagnostic {
        code: code.into(),
        extension_id,
        action,
        message: message.into(),
    }
}
