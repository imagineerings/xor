use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{SimProviderCapability, SimProviderId, SimProviderNodeDefinition};

pub const SIM_PROVIDER_POLICY_API_DISABLED_CODE: &str = "world_model.provider_policy.api_disabled";
pub const SIM_PROVIDER_POLICY_OFFLINE_CODE: &str = "world_model.provider_policy.offline";
pub const SIM_PROVIDER_POLICY_EXTERNAL_DATA_CODE: &str =
    "world_model.provider_policy.external_data_approval_required";
pub const SIM_PROVIDER_POLICY_COST_CODE: &str =
    "world_model.provider_policy.cost_approval_required";
pub const SIM_PROVIDER_POLICY_CAPABILITY_UNAVAILABLE_CODE: &str =
    "world_model.provider_policy.capability_unavailable";
pub const SIM_PROVIDER_POLICY_QUOTA_EXCEEDED_CODE: &str =
    "world_model.provider_policy.quota_exceeded";
pub const SIM_PROVIDER_POLICY_MODEL_UNAVAILABLE_CODE: &str =
    "world_model.provider_policy.model_unavailable";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderPolicyContext {
    pub api_nodes_enabled: bool,
    pub offline_mode: bool,
    pub external_data_approved: bool,
    pub cost_approved: bool,
}

impl Default for SimProviderPolicyContext {
    fn default() -> Self {
        Self {
            api_nodes_enabled: true,
            offline_mode: false,
            external_data_approved: false,
            cost_approved: false,
        }
    }
}

impl SimProviderPolicyContext {
    pub fn with_api_nodes_enabled(mut self, enabled: bool) -> Self {
        self.api_nodes_enabled = enabled;
        self
    }

    pub fn with_offline_mode(mut self, enabled: bool) -> Self {
        self.offline_mode = enabled;
        self
    }

    pub fn with_external_data_approved(mut self, approved: bool) -> Self {
        self.external_data_approved = approved;
        self
    }

    pub fn with_cost_approved(mut self, approved: bool) -> Self {
        self.cost_approved = approved;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderPolicyRequest {
    pub provider_id: SimProviderId,
    pub capability: SimProviderCapability,
    pub comfy_node_id: String,
    pub native_handler: String,
    pub model_id: Option<String>,
    pub estimated_quota_units: u64,
    pub transmits_external_data: bool,
    pub may_incur_cost: bool,
}

impl SimProviderPolicyRequest {
    pub fn new(
        provider_id: SimProviderId,
        capability: SimProviderCapability,
        comfy_node_id: impl Into<String>,
        native_handler: impl Into<String>,
    ) -> Self {
        Self {
            provider_id,
            capability,
            comfy_node_id: comfy_node_id.into(),
            native_handler: native_handler.into(),
            model_id: None,
            estimated_quota_units: 0,
            transmits_external_data: false,
            may_incur_cost: false,
        }
    }

    pub fn for_node(node: &SimProviderNodeDefinition) -> Self {
        Self {
            provider_id: node.provider_id.clone(),
            capability: node.capability,
            comfy_node_id: node.comfy_node_id.clone(),
            native_handler: node.native_handler.clone(),
            model_id: None,
            estimated_quota_units: 0,
            transmits_external_data: false,
            may_incur_cost: node.cost.may_incur_cost,
        }
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn with_estimated_quota_units(mut self, units: u64) -> Self {
        self.estimated_quota_units = units;
        self
    }

    pub fn with_external_data(mut self, transmits_external_data: bool) -> Self {
        self.transmits_external_data = transmits_external_data;
        self
    }

    pub fn with_cost(mut self, may_incur_cost: bool) -> Self {
        self.may_incur_cost = may_incur_cost;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderCapabilityPolicy {
    pub provider_id: SimProviderId,
    pub capability: SimProviderCapability,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub remaining_quota_units: Option<u64>,
    pub allowed_models: Vec<String>,
}

impl SimProviderCapabilityPolicy {
    pub fn new(provider_id: SimProviderId, capability: SimProviderCapability) -> Self {
        Self {
            provider_id,
            capability,
            available: true,
            unavailable_reason: None,
            remaining_quota_units: None,
            allowed_models: Vec::new(),
        }
    }

    pub fn unavailable(mut self, reason: impl Into<String>) -> Self {
        self.available = false;
        self.unavailable_reason = Some(reason.into());
        self
    }

    pub fn with_remaining_quota_units(mut self, units: u64) -> Self {
        self.remaining_quota_units = Some(units);
        self
    }

    pub fn with_allowed_model(mut self, model_id: impl Into<String>) -> Self {
        self.allowed_models.push(model_id.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderPolicyDecision {
    pub allowed: bool,
    pub diagnostics: Vec<SimProviderPolicyDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimProviderPolicyDiagnosticSeverity {
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderPolicyDiagnostic {
    pub code: String,
    pub severity: SimProviderPolicyDiagnosticSeverity,
    pub provider_id: SimProviderId,
    pub comfy_node_id: String,
    pub message: String,
}

impl SimProviderPolicyDiagnostic {
    fn error(
        code: impl Into<String>,
        request: &SimProviderPolicyRequest,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: SimProviderPolicyDiagnosticSeverity::Error,
            provider_id: request.provider_id.clone(),
            comfy_node_id: request.comfy_node_id.clone(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderPolicyGate {
    capability_policies:
        BTreeMap<(SimProviderId, SimProviderCapability), SimProviderCapabilityPolicy>,
}

impl SimProviderPolicyGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capability_policy(mut self, policy: SimProviderCapabilityPolicy) -> Self {
        self.capability_policies
            .insert((policy.provider_id.clone(), policy.capability), policy);
        self
    }

    pub fn evaluate(
        &self,
        request: &SimProviderPolicyRequest,
        context: &SimProviderPolicyContext,
    ) -> SimProviderPolicyDecision {
        let mut diagnostics = Vec::new();

        if !context.api_nodes_enabled {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_API_DISABLED_CODE,
                request,
                "API provider nodes are disabled by policy",
            ));
        }

        if context.offline_mode {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_OFFLINE_CODE,
                request,
                "offline mode prevents native Sim provider calls",
            ));
        }

        if request.transmits_external_data && !context.external_data_approved {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_EXTERNAL_DATA_CODE,
                request,
                "external provider data transfer requires approval",
            ));
        }

        if request.may_incur_cost && !context.cost_approved {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_COST_CODE,
                request,
                "provider request may incur cost and requires approval",
            ));
        }

        if let Some(policy) = self.capability_policy(request) {
            self.evaluate_capability_policy(policy, request, &mut diagnostics);
        }

        SimProviderPolicyDecision {
            allowed: diagnostics.is_empty(),
            diagnostics,
        }
    }

    fn capability_policy(
        &self,
        request: &SimProviderPolicyRequest,
    ) -> Option<&SimProviderCapabilityPolicy> {
        self.capability_policies
            .get(&(request.provider_id.clone(), request.capability))
    }

    fn evaluate_capability_policy(
        &self,
        policy: &SimProviderCapabilityPolicy,
        request: &SimProviderPolicyRequest,
        diagnostics: &mut Vec<SimProviderPolicyDiagnostic>,
    ) {
        if !policy.available {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_CAPABILITY_UNAVAILABLE_CODE,
                request,
                policy
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "provider capability is unavailable".to_string()),
            ));
        }

        if let Some(remaining_units) = policy.remaining_quota_units
            && request.estimated_quota_units > remaining_units
        {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_QUOTA_EXCEEDED_CODE,
                request,
                format!(
                    "provider quota requires {} units but only {remaining_units} remain",
                    request.estimated_quota_units
                ),
            ));
        }

        if let Some(model_id) = &request.model_id
            && !policy.allowed_models.is_empty()
            && !policy
                .allowed_models
                .iter()
                .any(|allowed| allowed == model_id)
        {
            diagnostics.push(SimProviderPolicyDiagnostic::error(
                SIM_PROVIDER_POLICY_MODEL_UNAVAILABLE_CODE,
                request,
                format!("provider model {model_id} is not available for this capability"),
            ));
        }
    }
}
