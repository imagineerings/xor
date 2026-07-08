use serde::{Deserialize, Serialize};

use crate::{
    SimProviderCapability, SimProviderId, SimProviderPolicyRequest, SimProviderRemoteTaskHandle,
    SimProviderRemoteTaskStatus,
};

pub const SIM_PROVIDER_CONNECTOR_START_FAILED_CODE: &str =
    "world_model.provider_connector.start_failed";
pub const SIM_PROVIDER_CONNECTOR_POLL_FAILED_CODE: &str =
    "world_model.provider_connector.poll_failed";
pub const SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE: &str =
    "world_model.provider_connector.cancel_unsupported";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderConnectorError {
    pub code: String,
    pub provider_id: SimProviderId,
    pub message: String,
}

impl SimProviderConnectorError {
    pub fn new(
        code: impl Into<String>,
        provider_id: SimProviderId,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            provider_id,
            message: message.into(),
        }
    }
}

pub trait SimProviderConnector {
    fn provider_id(&self) -> &SimProviderId;
    fn capabilities(&self) -> &[SimProviderCapability];

    fn start(
        &mut self,
        request: SimProviderPolicyRequest,
    ) -> Result<SimProviderRemoteTaskHandle, SimProviderConnectorError>;

    fn poll(
        &mut self,
        handle: &SimProviderRemoteTaskHandle,
    ) -> Result<SimProviderRemoteTaskStatus, SimProviderConnectorError>;

    fn cancel(
        &mut self,
        handle: &SimProviderRemoteTaskHandle,
    ) -> Result<SimProviderRemoteTaskStatus, SimProviderConnectorError>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimMockProviderConnector {
    provider_id: SimProviderId,
    capabilities: Vec<SimProviderCapability>,
    next_task_index: u64,
    queued_statuses: Vec<SimProviderRemoteTaskStatus>,
    cancellation_supported: bool,
}

impl SimMockProviderConnector {
    pub fn new(provider_id: SimProviderId, capabilities: Vec<SimProviderCapability>) -> Self {
        Self {
            provider_id,
            capabilities,
            next_task_index: 0,
            queued_statuses: Vec::new(),
            cancellation_supported: true,
        }
    }

    pub fn with_status(mut self, status: SimProviderRemoteTaskStatus) -> Self {
        self.queued_statuses.push(status);
        self
    }

    pub fn with_cancellation_supported(mut self, supported: bool) -> Self {
        self.cancellation_supported = supported;
        self
    }
}

impl SimProviderConnector for SimMockProviderConnector {
    fn provider_id(&self) -> &SimProviderId {
        &self.provider_id
    }

    fn capabilities(&self) -> &[SimProviderCapability] {
        &self.capabilities
    }

    fn start(
        &mut self,
        request: SimProviderPolicyRequest,
    ) -> Result<SimProviderRemoteTaskHandle, SimProviderConnectorError> {
        if request.provider_id != self.provider_id {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
                self.provider_id.clone(),
                "request provider does not match connector provider",
            ));
        }

        if !self.capabilities.contains(&request.capability) {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
                self.provider_id.clone(),
                "connector does not support requested provider capability",
            ));
        }

        self.next_task_index += 1;
        Ok(SimProviderRemoteTaskHandle::new(
            request.provider_id,
            format!("sim-remote-task-{}", self.next_task_index),
            request.comfy_node_id,
            request.native_handler,
        ))
    }

    fn poll(
        &mut self,
        _handle: &SimProviderRemoteTaskHandle,
    ) -> Result<SimProviderRemoteTaskStatus, SimProviderConnectorError> {
        if self.queued_statuses.is_empty() {
            return Ok(SimProviderRemoteTaskStatus::Running {
                progress: None,
                message: None,
            });
        }
        Ok(self.queued_statuses.remove(0))
    }

    fn cancel(
        &mut self,
        _handle: &SimProviderRemoteTaskHandle,
    ) -> Result<SimProviderRemoteTaskStatus, SimProviderConnectorError> {
        if !self.cancellation_supported {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE,
                self.provider_id.clone(),
                "provider does not support remote cancellation",
            ));
        }
        Ok(SimProviderRemoteTaskStatus::Cancelled {
            message: "provider task cancelled".to_string(),
        })
    }
}
