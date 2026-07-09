use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameDebugProtocol {
    Dap,
    ExternalProcess,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDebugEndpoint {
    pub name: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub protocol: SimGameDebugProtocol,
}

impl SimGameDebugEndpoint {
    pub fn external_process(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            host: None,
            port: None,
            protocol: SimGameDebugProtocol::ExternalProcess,
        }
    }

    pub fn dap(name: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            name: name.into(),
            host: Some(host.into()),
            port: Some(port),
            protocol: SimGameDebugProtocol::Dap,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDebugMetadata {
    pub endpoints: Vec<SimGameDebugEndpoint>,
    pub diagnostics: Vec<String>,
}

impl SimGameDebugMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_endpoint(mut self, endpoint: SimGameDebugEndpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    pub fn validate(mut self) -> Self {
        for endpoint in &self.endpoints {
            if endpoint.protocol == SimGameDebugProtocol::Dap
                && (endpoint.host.as_deref().is_none_or(str::is_empty) || endpoint.port.is_none())
            {
                self.diagnostics.push(format!(
                    "debug endpoint '{}' requires host and port metadata",
                    endpoint.name
                ));
            }
        }
        self
    }
}
