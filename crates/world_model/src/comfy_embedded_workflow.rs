use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    ArtifactRecord, ArtifactType, ComfyWorkflowDocument, ComfyWorkflowSource, GenerationProvenance,
};

pub const INVALID_EMBEDDED_PROMPT_METADATA_CODE: &str =
    "world_model.comfy_embedded_workflow.invalid_prompt";
pub const INVALID_EMBEDDED_WORKFLOW_METADATA_CODE: &str =
    "world_model.comfy_embedded_workflow.invalid_workflow";
pub const UNSUPPORTED_EMBEDDED_WORKFLOW_FORMAT_CODE: &str =
    "world_model.comfy_embedded_workflow.unsupported_format";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyEmbeddedWorkflowFormat {
    Png,
    WebP,
    Flac,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyEmbeddedWorkflowDiagnostic {
    pub code: String,
    pub metadata_key: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyEmbeddedWorkflowReport {
    pub artifact: ArtifactRecord,
    pub format: ComfyEmbeddedWorkflowFormat,
    pub workflow: Option<ComfyWorkflowDocument>,
    pub prompt_json: Option<serde_json::Value>,
    pub provenance: Option<GenerationProvenance>,
    pub diagnostics: Vec<ComfyEmbeddedWorkflowDiagnostic>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComfyEmbeddedWorkflowExtractor;

impl ComfyEmbeddedWorkflowExtractor {
    pub fn extract(
        &self,
        artifact: &ArtifactRecord,
        provenance: Option<&GenerationProvenance>,
        metadata: &BTreeMap<String, String>,
    ) -> ComfyEmbeddedWorkflowReport {
        let format = embedded_format(artifact);
        let mut diagnostics = Vec::new();
        let mut workflow = None;
        let mut prompt_json = None;

        if format == ComfyEmbeddedWorkflowFormat::Unknown && has_supported_metadata(metadata) {
            diagnostics.push(diagnostic(
                UNSUPPORTED_EMBEDDED_WORKFLOW_FORMAT_CODE,
                None,
                format!(
                    "artifact `{}` is not a supported embedded workflow metadata format",
                    artifact.relative_path.display()
                ),
            ));
        } else {
            workflow = parse_workflow(artifact, metadata, &mut diagnostics);
            prompt_json = parse_prompt(metadata, &mut diagnostics);
        }

        let provenance = workflow.as_ref().and_then(|workflow| {
            provenance.map(|provenance| link_provenance(provenance, artifact, workflow))
        });

        ComfyEmbeddedWorkflowReport {
            artifact: artifact.clone(),
            format,
            workflow,
            prompt_json,
            provenance,
            diagnostics,
        }
    }
}

fn parse_workflow(
    artifact: &ArtifactRecord,
    metadata: &BTreeMap<String, String>,
    diagnostics: &mut Vec<ComfyEmbeddedWorkflowDiagnostic>,
) -> Option<ComfyWorkflowDocument> {
    let (metadata_key, value) = metadata_value(metadata, WORKFLOW_METADATA_KEYS)?;
    match serde_json::from_str::<serde_json::Value>(value) {
        Ok(graph_json) => {
            let name = artifact
                .label
                .clone()
                .or_else(|| {
                    graph_json
                        .get("extra")
                        .and_then(|extra| extra.get("workflow_name"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .unwrap_or_else(|| "embedded workflow".to_string());
            Some(
                ComfyWorkflowDocument::from_graph_json(
                    name,
                    graph_json,
                    ComfyWorkflowSource::Imported {
                        source_path: artifact.relative_path.display().to_string(),
                    },
                )
                .with_provenance_artifact(artifact.relative_path.display().to_string()),
            )
        }
        Err(source) => {
            diagnostics.push(diagnostic(
                INVALID_EMBEDDED_WORKFLOW_METADATA_CODE,
                Some(metadata_key.to_string()),
                format!("embedded workflow metadata is not valid JSON: {source}"),
            ));
            None
        }
    }
}

fn parse_prompt(
    metadata: &BTreeMap<String, String>,
    diagnostics: &mut Vec<ComfyEmbeddedWorkflowDiagnostic>,
) -> Option<serde_json::Value> {
    let (metadata_key, value) = metadata_value(metadata, PROMPT_METADATA_KEYS)?;
    match serde_json::from_str::<serde_json::Value>(value) {
        Ok(prompt) => Some(prompt),
        Err(source) => {
            diagnostics.push(diagnostic(
                INVALID_EMBEDDED_PROMPT_METADATA_CODE,
                Some(metadata_key.to_string()),
                format!("embedded prompt metadata is not valid JSON: {source}"),
            ));
            None
        }
    }
}

fn link_provenance(
    provenance: &GenerationProvenance,
    artifact: &ArtifactRecord,
    workflow: &ComfyWorkflowDocument,
) -> GenerationProvenance {
    let mut provenance = provenance.clone();
    if !provenance
        .artifacts
        .iter()
        .any(|existing| existing.relative_path == artifact.relative_path)
    {
        provenance.artifacts.push(artifact.clone());
    }
    if provenance.workflow_name.is_none() {
        provenance.workflow_name = Some(workflow.name.clone());
    }
    provenance
}

fn has_supported_metadata(metadata: &BTreeMap<String, String>) -> bool {
    WORKFLOW_METADATA_KEYS
        .iter()
        .chain(PROMPT_METADATA_KEYS.iter())
        .any(|key| metadata.contains_key(*key))
}

fn metadata_value<'a>(
    metadata: &'a BTreeMap<String, String>,
    keys: &[&'static str],
) -> Option<(&'static str, &'a str)> {
    keys.iter()
        .find_map(|key| metadata.get(*key).map(|value| (*key, value.as_str())))
}

fn embedded_format(artifact: &ArtifactRecord) -> ComfyEmbeddedWorkflowFormat {
    match artifact
        .relative_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => ComfyEmbeddedWorkflowFormat::Png,
        Some("webp") => ComfyEmbeddedWorkflowFormat::WebP,
        Some("flac") => ComfyEmbeddedWorkflowFormat::Flac,
        _ => match artifact.artifact_type {
            ArtifactType::Image => ComfyEmbeddedWorkflowFormat::Png,
            ArtifactType::Audio => ComfyEmbeddedWorkflowFormat::Flac,
            _ => ComfyEmbeddedWorkflowFormat::Unknown,
        },
    }
}

fn diagnostic(
    code: &str,
    metadata_key: Option<String>,
    message: impl Into<String>,
) -> ComfyEmbeddedWorkflowDiagnostic {
    ComfyEmbeddedWorkflowDiagnostic {
        code: code.to_string(),
        metadata_key,
        message: message.into(),
    }
}

const WORKFLOW_METADATA_KEYS: &[&str] = &[
    "workflow",
    "Workflow",
    "comfy.workflow",
    "extra_pnginfo.workflow",
];
const PROMPT_METADATA_KEYS: &[&str] = &["prompt", "Prompt", "comfy.prompt", "extra_pnginfo.prompt"];
