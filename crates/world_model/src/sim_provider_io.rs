use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    SIM_PROVIDER_SIGNED_URL_PLACEHOLDER, SimProviderId, SimProviderRedactor,
    SimProviderRemoteTaskHandle, SimProviderRemoteTaskId,
};

pub const SIM_PROVIDER_IO_MISSING_OUTPUT_CODE: &str = "world_model.provider_io.missing_output";
pub const SIM_PROVIDER_IO_MISSING_MIME_CODE: &str = "world_model.provider_io.missing_mime";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderSourceMedia {
    pub asset_ref: String,
    pub mime_type: String,
    pub signed_upload_url: Option<String>,
}

impl SimProviderSourceMedia {
    pub fn new(asset_ref: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            asset_ref: asset_ref.into(),
            mime_type: mime_type.into(),
            signed_upload_url: None,
        }
    }

    pub fn with_signed_upload_url(mut self, signed_upload_url: impl Into<String>) -> Self {
        self.signed_upload_url = Some(signed_upload_url.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderUploadRecord {
    pub provider_id: SimProviderId,
    pub source_asset_ref: String,
    pub upload_ref: String,
    pub mime_type: String,
    pub redacted_upload_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimProviderOutputKind {
    Image,
    Video,
    Audio,
    Text,
    Vector,
    ThreeD,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderOutput {
    pub output_ref: String,
    pub kind: SimProviderOutputKind,
    pub mime_type: String,
    pub remote_url: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl SimProviderOutput {
    pub fn new(
        output_ref: impl Into<String>,
        kind: SimProviderOutputKind,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            output_ref: output_ref.into(),
            kind,
            mime_type: mime_type.into(),
            remote_url: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_remote_url(mut self, remote_url: impl Into<String>) -> Self {
        self.remote_url = Some(remote_url.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderOutputProvenance {
    pub provider_id: SimProviderId,
    pub remote_task_id: SimProviderRemoteTaskId,
    pub comfy_node_id: String,
    pub native_handler: String,
    pub source_asset_refs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderImportedAsset {
    pub asset_ref: String,
    pub kind: SimProviderOutputKind,
    pub mime_type: String,
    pub redacted_remote_url: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub provenance: SimProviderOutputProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderIoDiagnostic {
    pub code: String,
    pub provider_id: SimProviderId,
    pub remote_task_id: SimProviderRemoteTaskId,
    pub message: String,
}

impl SimProviderIoDiagnostic {
    fn new(
        code: impl Into<String>,
        handle: &SimProviderRemoteTaskHandle,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            provider_id: handle.provider_id.clone(),
            remote_task_id: handle.remote_task_id.clone(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderImportReport {
    pub assets: Vec<SimProviderImportedAsset>,
    pub diagnostics: Vec<SimProviderIoDiagnostic>,
}

impl SimProviderImportReport {
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderIoService {
    redactor: SimProviderRedactor,
}

impl Default for SimProviderIoService {
    fn default() -> Self {
        Self {
            redactor: SimProviderRedactor::new(),
        }
    }
}

impl SimProviderIoService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_redactor(mut self, redactor: SimProviderRedactor) -> Self {
        self.redactor = redactor;
        self
    }

    pub fn prepare_upload(
        &self,
        provider_id: SimProviderId,
        source: SimProviderSourceMedia,
    ) -> SimProviderUploadRecord {
        let redacted_upload_url = source
            .signed_upload_url
            .as_deref()
            .map(|url| self.redactor.redact_string(url));
        SimProviderUploadRecord {
            provider_id,
            upload_ref: format!("sim-upload:{}", source.asset_ref),
            source_asset_ref: source.asset_ref,
            mime_type: source.mime_type,
            redacted_upload_url,
        }
    }

    pub fn import_outputs(
        &self,
        handle: &SimProviderRemoteTaskHandle,
        source_asset_refs: Vec<String>,
        outputs: Vec<SimProviderOutput>,
    ) -> SimProviderImportReport {
        if outputs.is_empty() {
            return SimProviderImportReport {
                assets: Vec::new(),
                diagnostics: vec![SimProviderIoDiagnostic::new(
                    SIM_PROVIDER_IO_MISSING_OUTPUT_CODE,
                    handle,
                    "provider task completed without importable outputs",
                )],
            };
        }

        let mut assets = Vec::new();
        let mut diagnostics = Vec::new();
        for output in outputs {
            if output.mime_type.trim().is_empty() {
                diagnostics.push(SimProviderIoDiagnostic::new(
                    SIM_PROVIDER_IO_MISSING_MIME_CODE,
                    handle,
                    "provider output must include MIME metadata",
                ));
                continue;
            }

            let redacted_remote_url = output
                .remote_url
                .as_deref()
                .map(|url| self.redactor.redact_string(url));
            assets.push(SimProviderImportedAsset {
                asset_ref: format!("sim-asset:{}", output.output_ref),
                kind: output.kind,
                mime_type: output.mime_type,
                redacted_remote_url,
                metadata: output.metadata,
                provenance: SimProviderOutputProvenance {
                    provider_id: handle.provider_id.clone(),
                    remote_task_id: handle.remote_task_id.clone(),
                    comfy_node_id: handle.comfy_node_id.clone(),
                    native_handler: handle.native_handler.clone(),
                    source_asset_refs: source_asset_refs.clone(),
                },
            });
        }

        SimProviderImportReport {
            assets,
            diagnostics,
        }
    }
}

pub fn is_signed_url_placeholder(value: &str) -> bool {
    value == SIM_PROVIDER_SIGNED_URL_PLACEHOLDER
}
