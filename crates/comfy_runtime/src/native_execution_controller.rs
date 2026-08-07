use crate::{
    AssetNamespace, AssetRoots, AttemptEventKind, AttemptState, AuthorizedCapabilities,
    CacheDependencies, CanonicalClipCacheIdentities, CompiledPlan, EffectClass, EffectCoordinator,
    ExecutionActuatorEventInput, ExecutionControlCommand, ExecutionControlCommandKind,
    ExecutionController, ExecutionEngine, ExecutionError, ExecutionEventBus, ExecutionFailure,
    ExecutionFailureOrigin, ExecutionOutput, ExecutionOutputAvailability, ExecutionPreview,
    ExecutionReport, InputBinding, InputMode, MemoryPolicy, NativeCache, NativeNode,
    NativeNodeRegistry, NodeContext, NodeFailure, NodeFailureKind, NodeOutcome, OutputCommitError,
    OutputCommitReceipt, OutputCommitter, OutputExecutionScope, OutputMediaKind, OutputProposal,
    PreparedEffect, PreparedEffectRequest, PreparedOutput, ProfileId, PromptCompileError,
    RuntimeAvailability, RuntimeCachePolicy, RuntimeInputDescriptor, RuntimeNodeDescriptor,
    RuntimeOutputDescriptor, RuntimeSupervisor, RuntimeSupervisorError, SharedAssetService,
    SharedExecutionPresentationService, SharedOutputCommitter, ValueType, WorkerLaunchConfig,
    WorkflowFormatDocument, authorize_native_input_reader, authorize_native_output_committer,
    graph_to_prompt,
};
use chrono::{Local, Utc};
#[cfg(test)]
use comfy_media::encode_png_frame;
use comfy_media::{
    MetadataWritePolicy, PngError, PngLimits, decode_png, encode_png_frame_with_policy_and_context,
};
use comfy_model::{
    clip::{ClipError, LoadedSd1Clip, NativeTokenizer, WeightedText},
    conditioning::{
        ConditioningEntry, ConditioningEntryOptions, ConditioningError, ConditioningIdentity,
        ConditioningSet, ConditioningValue,
    },
    generated_native_diffusion::{
        Sd1Tokenizer, Sd15TinyModel, empty_sd15_latent, sd15_latent_format_identity,
        sd15_model_family_identity,
    },
};
use comfy_nodes::{
    CatalogNodeDescriptor, NativeImageDescriptor, NativeImageDescriptorError, NativeImageEffect,
    NodeDescriptor, NodeRegistry, PortDescriptor, native_diffusion_descriptors,
    native_image_descriptors,
};
use comfy_sampler::{
    DiscreteSamplingProfile, GUIDANCE_ADAPTER_ID, GuidanceDenoiser, GuidanceError,
    GuidanceEvaluation, GuidanceOptions, GuidanceResult, INITIAL_NOISE_PHASE_ID, NoiseRequest,
    SamplingPlan, execute_guidance,
    generated_native_diffusion::{
        checked_native_diffusion_plan, normal_noise, normal_sigmas, sample_euler,
        scale_initial_noise, scale_model_input, sd15_interpret_prediction, sd15_model_time,
    },
};
use comfy_tensor::{
    BackendCapabilityMatrix, CancellationToken, CpuBackend, CpuWorkspaceAuthority,
    ExecutionContext, ImageTensor, ResizeCrop, ResizeMode, ScratchReservation, StreamId,
    TensorDescriptor, TensorError,
};
use comfy_tensor::{Tensor, generated_native_diffusion::tensor_to_f32};
use comfy_types::{AttemptId, BackendUnavailable, CancellationError, NodeId, WorkerOutputProposal};
use futures::future::BoxFuture;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::fs;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;
use uuid::Uuid;

pub const NATIVE_IMAGE_REGISTRY_VERSION: &str = "native-image-v1";
pub const NATIVE_DIFFUSION_REGISTRY_VERSION: &str = "native-diffusion-v1";
pub const NATIVE_TENSOR_HANDLE_SCHEMA_VERSION: u16 = 1;
const MAX_NATIVE_IMAGE_INPUT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_NATIVE_WORKER_INPUT_BYTES: usize = 12 * 1024 * 1024;
const MAX_NATIVE_OUTPUT_PROPOSALS: usize = 4_096;
const MAX_NATIVE_ATTEMPT_PROPOSAL_BYTES: usize = 256 * 1024 * 1024;
const MAX_NATIVE_UI_OUTPUTS: usize = 4_096;
const MAX_NATIVE_UI_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
const NATIVE_CONTROLLER_CAPACITY: usize = 1024;
const NATIVE_WORKER_EVENT_POLL: Duration = Duration::from_millis(100);
const DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn project_effective_native_backend(
    accepted_backend: &BackendCapabilityMatrix,
    configured_memory_limit_bytes: u64,
    memory_policy: MemoryPolicy,
) -> crate::EffectiveNativeBackendState {
    let properties = accepted_backend.device_properties();
    let memory_limit_bytes = properties.map_or(configured_memory_limit_bytes, |value| {
        configured_memory_limit_bytes.min(value.allocation_limit_bytes())
    });
    crate::EffectiveNativeBackendState {
        device: accepted_backend.device(),
        device_name: properties.map_or_else(
            || format!("{:?}", accepted_backend.device().kind()),
            |value| value.name().to_owned(),
        ),
        architecture: properties.and_then(|value| value.architecture().map(str::to_owned)),
        total_memory_bytes: properties.map(|value| value.total_memory_bytes()),
        allocation_limit_bytes: properties.map(|value| value.allocation_limit_bytes()),
        memory_limit_bytes,
        memory_in_use_bytes: 0,
        memory_policy,
        supported_operation_rows: accepted_backend.supported().len(),
        deterministic_operation_rows: accepted_backend.deterministic().len(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTensorKind {
    Image,
    Mask,
    Conditioning,
    Latent,
}

#[derive(Clone)]
pub struct NativeDiffusionBundle {
    fixture_id: String,
    model_digest: String,
    tokenizer_digest: String,
    model: Arc<Sd15TinyModel>,
    tokenizer: Arc<Sd1Tokenizer>,
    clip: Arc<LoadedSd1Clip>,
    clip_cache_identities: CanonicalClipCacheIdentities,
}

impl NativeDiffusionBundle {
    pub fn new(
        fixture_id: impl Into<String>,
        model_digest: impl Into<String>,
        model: Arc<Sd15TinyModel>,
        tokenizer: Arc<Sd1Tokenizer>,
        clip: Arc<LoadedSd1Clip>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let fixture_id = fixture_id.into();
        let model_digest = model_digest.into();
        let tokenizer_identity = tokenizer.identity();
        let invalid_model_digest = model_digest.len() != 64
            || !model_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        let invalid_reason = if fixture_id != "sd15-tiny-v1" {
            Some("fixture")
        } else if invalid_model_digest {
            Some("model artifact")
        } else if tokenizer_identity.descriptor().identifier() != "comfy.sd1.tokenizer" {
            Some("tokenizer descriptor")
        } else if clip.plan().tokenizer_identity() != tokenizer_identity {
            Some("tokenizer execution binding")
        } else if clip.plan().artifact_identity().as_str() != model_digest {
            Some("CLIP artifact binding")
        } else {
            None
        };
        if let Some(reason) = invalid_reason {
            return Err(NativeImageRuntimeError::Registry(format!(
                "native diffusion provider {reason} identity is invalid"
            )));
        }
        let clip_cache_identities = CanonicalClipCacheIdentities::checked(
            tokenizer_identity.digest(),
            clip.architecture().digest(),
            clip.plan().artifact_identity().as_str(),
            clip.plan().model_identity().as_str(),
            clip.plan().patch_identity().as_str(),
            clip.plan().digest(),
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        Ok(Self {
            fixture_id,
            model_digest,
            tokenizer_digest: tokenizer_identity.digest().to_owned(),
            model,
            tokenizer,
            clip,
            clip_cache_identities,
        })
    }

    pub fn model_digest(&self) -> &str {
        &self.model_digest
    }

    pub fn tokenizer_digest(&self) -> &str {
        &self.tokenizer_digest
    }

    pub fn model(&self) -> &Arc<Sd15TinyModel> {
        &self.model
    }

    pub fn clip(&self) -> &Arc<LoadedSd1Clip> {
        &self.clip
    }

    pub fn clip_cache_identities(&self) -> &CanonicalClipCacheIdentities {
        &self.clip_cache_identities
    }

    pub fn encode_text(
        &self,
        text: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<([u32; comfy_model::clip::SD1_CONTEXT_LENGTH], Tensor), NativeImageRuntimeError>
    {
        let prompts = vec![vec![
            WeightedText::checked(text, 1.0).map_err(map_clip_runtime_error)?,
        ]];
        let batch = self
            .tokenizer
            .tokenize_batch(&prompts, context.cancellation)
            .map_err(map_clip_runtime_error)?;
        let tokens = batch
            .rows()
            .first()
            .ok_or_else(|| {
                NativeImageRuntimeError::Execution(
                    "canonical SD1 tokenizer returned no prompt row".to_owned(),
                )
            })?
            .sd1_token_ids()
            .map_err(map_clip_runtime_error)?;
        let encoding = self
            .clip
            .execute(&batch, context)
            .map_err(map_clip_runtime_error)?;
        Ok((tokens, encoding.conditioning().clone()))
    }
}

fn map_clip_runtime_error(error: ClipError) -> NativeImageRuntimeError {
    match error {
        ClipError::Tensor(TensorError::Cancelled) => NativeImageRuntimeError::Cancelled,
        error => NativeImageRuntimeError::Execution(error.to_string()),
    }
}

struct Sd15GuidanceDenoiser<'a> {
    model: &'a Sd15TinyModel,
}

fn map_conditioning_runtime_error(error: ConditioningError) -> NativeImageRuntimeError {
    match error {
        ConditioningError::Cancelled | ConditioningError::Tensor(TensorError::Cancelled) => {
            NativeImageRuntimeError::Cancelled
        }
        error => NativeImageRuntimeError::Execution(error.to_string()),
    }
}

impl GuidanceDenoiser for Sd15GuidanceDenoiser<'_> {
    fn evaluate_batch(
        &mut self,
        evaluations: &[GuidanceEvaluation],
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<Tensor>, GuidanceError> {
        let mut outputs = Vec::new();
        outputs
            .try_reserve_exact(evaluations.len())
            .map_err(|_| GuidanceError::ShapeOverflow("SD15 guidance outputs"))?;
        for evaluation in evaluations {
            context
                .cancellation
                .check()
                .map_err(|_| GuidanceError::Cancelled)?;
            let conditioning = match evaluation.entry().value() {
                ConditioningValue::CrossAttention(conditioning) => conditioning,
                _ => {
                    return Err(GuidanceError::Invalid(
                        "SD15 guidance requires cross-attention conditioning".to_owned(),
                    ));
                }
            };
            let model_time = sd15_model_time(evaluation.sigma())
                .map_err(|error| GuidanceError::Invalid(error.to_string()))?;
            let output = self
                .model
                .denoise_at_model_time(evaluation.latent(), model_time, conditioning, context)
                .map_err(|error| {
                    if context.cancellation.is_cancelled() {
                        GuidanceError::Cancelled
                    } else {
                        GuidanceError::Invalid(format!("SD15 denoiser failed: {error}"))
                    }
                })?;
            outputs.push(output);
        }
        Ok(outputs)
    }
}

pub struct Sd15GuidanceAdapter<'a> {
    profile: DiscreteSamplingProfile,
    positive: ConditioningSet,
    negative: ConditioningSet,
    denoiser: Sd15GuidanceDenoiser<'a>,
}

impl<'a> Sd15GuidanceAdapter<'a> {
    pub fn checked(
        model: &'a Sd15TinyModel,
        positive: &Tensor,
        negative: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let model_family = sd15_model_family_identity()
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        let latent_format = sd15_latent_format_identity()
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        let conditioning = |namespace: &str,
                            identifier: &str,
                            tensor: &Tensor|
         -> Result<ConditioningSet, NativeImageRuntimeError> {
            let identity =
                ConditioningIdentity::new(namespace, model_family.clone(), latent_format.clone())
                    .map_err(map_conditioning_runtime_error)?;
            let entry = ConditioningEntry::checked(
                identifier,
                ConditioningValue::cross_attention(tensor.clone())
                    .map_err(map_conditioning_runtime_error)?,
                ConditioningEntryOptions::default(),
            )
            .map_err(map_conditioning_runtime_error)?;
            ConditioningSet::checked(identity, vec![entry], context.cancellation)
                .map_err(map_conditioning_runtime_error)
        };
        Ok(Self {
            profile: DiscreteSamplingProfile::sd15()
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
            positive: conditioning("sd15-positive", "cross-attention", positive)?,
            negative: conditioning("sd15-negative", "cross-attention", negative)?,
            denoiser: Sd15GuidanceDenoiser { model },
        })
    }

    pub fn execute(
        &mut self,
        backend: &CpuBackend,
        latent: &Tensor,
        sigma: f32,
        plan: &SamplingPlan,
        context: &ExecutionContext<'_>,
    ) -> Result<GuidanceResult, NativeImageRuntimeError> {
        execute_guidance(
            backend,
            latent,
            sigma,
            &self.profile,
            plan,
            &self.positive,
            &self.negative,
            GuidanceOptions::default(),
            &mut self.denoiser,
            &mut [],
            context,
        )
        .map_err(|error| match error {
            GuidanceError::Cancelled => NativeImageRuntimeError::Cancelled,
            error => NativeImageRuntimeError::Execution(error.to_string()),
        })
    }
}

pub trait NativeDiffusionProvider: Send + Sync {
    fn model_digest(&self) -> Result<String, NativeImageRuntimeError>;

    fn tokenizer_digest(&self) -> Result<String, NativeImageRuntimeError>;

    fn clip_cache_identities(
        &self,
    ) -> Result<CanonicalClipCacheIdentities, NativeImageRuntimeError>;

    fn load(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionBundle, NativeImageRuntimeError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "NativeTensorHandleWire")]
pub struct NativeTensorHandle {
    schema_version: u16,
    kind: NativeTensorKind,
    content_digest: String,
    descriptor: TensorDescriptor,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeTensorHandleWire {
    schema_version: u16,
    kind: NativeTensorKind,
    content_digest: String,
    descriptor: TensorDescriptor,
}

impl TryFrom<NativeTensorHandleWire> for NativeTensorHandle {
    type Error = NativeImageRuntimeError;

    fn try_from(value: NativeTensorHandleWire) -> Result<Self, Self::Error> {
        let handle = Self {
            schema_version: value.schema_version,
            kind: value.kind,
            content_digest: value.content_digest,
            descriptor: value.descriptor,
        };
        handle.validate(handle.kind)?;
        Ok(handle)
    }
}

impl NativeTensorHandle {
    pub fn new(
        kind: NativeTensorKind,
        content_digest: impl Into<String>,
        descriptor: TensorDescriptor,
    ) -> Result<Self, NativeImageRuntimeError> {
        let handle = Self {
            schema_version: NATIVE_TENSOR_HANDLE_SCHEMA_VERSION,
            kind,
            content_digest: content_digest.into(),
            descriptor,
        };
        handle.validate(kind)?;
        Ok(handle)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn kind(&self) -> NativeTensorKind {
        self.kind
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub const fn descriptor(&self) -> &TensorDescriptor {
        &self.descriptor
    }

    fn validate(&self, expected_kind: NativeTensorKind) -> Result<(), NativeImageRuntimeError> {
        if self.schema_version != NATIVE_TENSOR_HANDLE_SCHEMA_VERSION
            || self.kind != expected_kind
            || self.content_digest.len() != 64
            || !self
                .content_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(NativeImageRuntimeError::Handle(
                "schema, kind, or digest did not match the typed port".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeImageWorkerPlan {
    pub plan: CompiledPlan,
    pub input_assets: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    pub memory_policy: MemoryPolicy,
    #[serde(default = "metadata_enabled_by_default")]
    pub metadata_enabled: bool,
    #[serde(default)]
    pub injected_delay_millis: u64,
}

impl NativeImageWorkerPlan {
    pub fn from_asset_service(
        plan: CompiledPlan,
        assets: &SharedAssetService,
        authorization: &AuthorizedCapabilities,
        cancellation: &CancellationToken,
        metadata_enabled: bool,
        injected_delay_millis: u64,
    ) -> Result<Self, NativeImageRuntimeError> {
        let input_assets = collect_worker_input_assets(&plan, assets, authorization, cancellation)?;
        Self::new(plan, input_assets, metadata_enabled, injected_delay_millis)
    }

    pub fn new(
        plan: CompiledPlan,
        input_assets: BTreeMap<String, Vec<u8>>,
        metadata_enabled: bool,
        injected_delay_millis: u64,
    ) -> Result<Self, NativeImageRuntimeError> {
        Self::new_with_memory_policy(
            plan,
            input_assets,
            MemoryPolicy::default(),
            metadata_enabled,
            injected_delay_millis,
        )
    }

    pub fn new_with_memory_policy(
        plan: CompiledPlan,
        input_assets: BTreeMap<String, Vec<u8>>,
        memory_policy: MemoryPolicy,
        metadata_enabled: bool,
        injected_delay_millis: u64,
    ) -> Result<Self, NativeImageRuntimeError> {
        let value = Self {
            plan,
            input_assets,
            memory_policy,
            metadata_enabled,
            injected_delay_millis,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), NativeImageRuntimeError> {
        validate_worker_input_assets(&self.input_assets)?;
        let expected = required_worker_input_ids(&self.plan)?;
        let actual = self.input_assets.keys().cloned().collect();
        if expected != actual {
            return Err(NativeImageRuntimeError::Encoding(
                "native worker input assets do not exactly match LoadImage dependencies".to_owned(),
            ));
        }
        Ok(())
    }
}

const fn metadata_enabled_by_default() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct NativeImageOutputProposalMetadata {
    schema_version: u16,
    pub node_id: NodeId,
    pub batch_index: u32,
    pub namespace: AssetNamespace,
    pub filename_prefix: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
}

const NATIVE_IMAGE_OUTPUT_PROPOSAL_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeImageOutputProposal {
    node_id: NodeId,
    output: OutputProposal,
}

impl NativeImageOutputProposal {
    fn new(node_id: NodeId, output: OutputProposal) -> Result<Self, NativeImageRuntimeError> {
        let metadata = NativeImageOutputProposalMetadata {
            schema_version: NATIVE_IMAGE_OUTPUT_PROPOSAL_SCHEMA_VERSION,
            node_id: node_id.clone(),
            batch_index: output.batch_index(),
            namespace: output.namespace(),
            filename_prefix: output.filename_prefix().to_owned(),
            extension: output.extension().to_owned(),
            width: output.width(),
            height: output.height(),
        };
        let output = output
            .with_projection_metadata(
                postcard::to_stdvec(&metadata)
                    .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))?,
            )
            .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))?;
        Ok(Self { node_id, output })
    }

    pub fn proposal_id(&self) -> Uuid {
        self.output.proposal_id()
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn output(&self) -> &OutputProposal {
        &self.output
    }

    pub fn to_worker_proposal(&self) -> Result<WorkerOutputProposal, NativeImageRuntimeError> {
        let metadata = NativeImageOutputProposalMetadata {
            schema_version: NATIVE_IMAGE_OUTPUT_PROPOSAL_SCHEMA_VERSION,
            node_id: self.node_id.clone(),
            batch_index: self.output.batch_index(),
            namespace: self.output.namespace(),
            filename_prefix: self.output.filename_prefix().to_owned(),
            extension: self.output.extension().to_owned(),
            width: self.output.width(),
            height: self.output.height(),
        };
        WorkerOutputProposal::new(
            self.output.proposal_id(),
            postcard::to_stdvec(&metadata)
                .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))?,
            self.output.content().to_vec(),
        )
        .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))
    }

    pub fn from_worker_proposal(
        proposal: WorkerOutputProposal,
    ) -> Result<Self, NativeImageRuntimeError> {
        let (proposal_id, metadata, content) = proposal.into_parts();
        let metadata: NativeImageOutputProposalMetadata = postcard::from_bytes(&metadata)
            .map_err(|error| NativeImageRuntimeError::WorkerEvent(error.to_string()))?;
        if metadata.schema_version != NATIVE_IMAGE_OUTPUT_PROPOSAL_SCHEMA_VERSION {
            return Err(NativeImageRuntimeError::WorkerEvent(format!(
                "unsupported native image output proposal schema {}",
                metadata.schema_version
            )));
        }
        let output = OutputProposal::new(
            proposal_id,
            metadata.namespace,
            metadata.filename_prefix,
            metadata.extension,
            metadata.batch_index,
            metadata.width,
            metadata.height,
            content,
        )
        .map_err(|error| NativeImageRuntimeError::WorkerEvent(error.to_string()))?;
        Self::new(metadata.node_id, output)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeImageExecutionResult {
    pub report: ExecutionReport,
    pub output_proposals: Vec<NativeImageOutputProposal>,
    pub executed_node_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeImageWorkerResult {
    pub report: ExecutionReport,
    pub output_proposal_ids: Vec<Uuid>,
    pub executed_node_count: usize,
    encoded_ui_outputs: Vec<NativeImageWorkerUiOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct NativeImageWorkerUiOutput {
    node_id: NodeId,
    encoded_json: Vec<u8>,
}

impl NativeImageWorkerResult {
    pub fn from_execution_report(
        mut report: ExecutionReport,
        output_proposal_ids: Vec<Uuid>,
        executed_node_count: usize,
    ) -> Result<Self, NativeImageRuntimeError> {
        if report.ui_outputs.len() > MAX_NATIVE_UI_OUTPUTS {
            return Err(NativeImageRuntimeError::Encoding(
                "native worker UI output count exceeds its bound".to_owned(),
            ));
        }
        let mut encoded_byte_count = 0_usize;
        let mut encoded_ui_outputs = Vec::with_capacity(report.ui_outputs.len());
        for (node_id, value) in std::mem::take(&mut report.ui_outputs) {
            let encoded_json = serde_json::to_vec(&value)
                .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))?;
            encoded_byte_count = encoded_byte_count
                .checked_add(encoded_json.len())
                .ok_or_else(|| {
                    NativeImageRuntimeError::Encoding(
                        "native worker UI output byte count overflowed".to_owned(),
                    )
                })?;
            if encoded_byte_count > MAX_NATIVE_UI_OUTPUT_BYTES {
                return Err(NativeImageRuntimeError::Encoding(
                    "native worker UI output bytes exceed their bound".to_owned(),
                ));
            }
            encoded_ui_outputs.push(NativeImageWorkerUiOutput {
                node_id,
                encoded_json,
            });
        }
        Ok(Self {
            report,
            output_proposal_ids,
            executed_node_count,
            encoded_ui_outputs,
        })
    }

    pub fn decode_ui_outputs(&self) -> Result<BTreeMap<NodeId, Value>, NativeImageRuntimeError> {
        if self.encoded_ui_outputs.len() > MAX_NATIVE_UI_OUTPUTS {
            return Err(NativeImageRuntimeError::WorkerEvent(
                "native worker UI output count exceeds its bound".to_owned(),
            ));
        }
        let mut encoded_byte_count = 0_usize;
        let mut ui_outputs = BTreeMap::new();
        for output in &self.encoded_ui_outputs {
            encoded_byte_count = encoded_byte_count
                .checked_add(output.encoded_json.len())
                .ok_or_else(|| {
                    NativeImageRuntimeError::WorkerEvent(
                        "native worker UI output byte count overflowed".to_owned(),
                    )
                })?;
            if encoded_byte_count > MAX_NATIVE_UI_OUTPUT_BYTES {
                return Err(NativeImageRuntimeError::WorkerEvent(
                    "native worker UI output bytes exceed their bound".to_owned(),
                ));
            }
            let value = serde_json::from_slice(&output.encoded_json)
                .map_err(|error| NativeImageRuntimeError::WorkerEvent(error.to_string()))?;
            if ui_outputs.insert(output.node_id.clone(), value).is_some() {
                return Err(NativeImageRuntimeError::WorkerEvent(format!(
                    "native worker repeated UI output for node {:?}",
                    output.node_id
                )));
            }
        }
        Ok(ui_outputs)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NativeImageWorkerProgress {
    pub profile_id: ProfileId,
    pub prompt_id: comfy_types::PromptId,
    pub attempt_id: AttemptId,
    pub sequence: u64,
    pub node_id: Option<NodeId>,
    pub kind: NativeImageWorkerProgressKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NativeImageWorkerProgressKind {
    Started,
    Progress { completed: u64, total: u64 },
    CacheHit,
    OutputPrepared { transaction_id: Uuid },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NativeImageWorkerEvent {
    Progress { progress: NativeImageWorkerProgress },
    Completed { result: NativeImageWorkerResult },
    Failed { message: String, cancelled: bool },
    BackendUnavailable { unavailable: BackendUnavailable },
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum NativeImageRuntimeError {
    #[error("native image descriptors are invalid: {0}")]
    Descriptors(String),
    #[error("native image registry is invalid: {0}")]
    Registry(String),
    #[error("native image asset roots are invalid: {0}")]
    Asset(String),
    #[error("native image execution failed: {0}")]
    Execution(String),
    #[error("native tensor handle is invalid: {0}")]
    Handle(String),
    #[error("native image worker plan encoding failed: {0}")]
    Encoding(String),
    #[error("native image worker event is invalid: {0}")]
    WorkerEvent(String),
    #[error("native image execution was cancelled")]
    Cancelled,
}

impl From<NativeImageDescriptorError> for NativeImageRuntimeError {
    fn from(error: NativeImageDescriptorError) -> Self {
        Self::Descriptors(error.to_string())
    }
}

impl From<PromptCompileError> for NativeImageRuntimeError {
    fn from(error: PromptCompileError) -> Self {
        Self::Registry(error.to_string())
    }
}

impl From<ExecutionError> for NativeImageRuntimeError {
    fn from(error: ExecutionError) -> Self {
        Self::Execution(error.to_string())
    }
}

impl From<TensorError> for NativeImageRuntimeError {
    fn from(error: TensorError) -> Self {
        if error == TensorError::Cancelled {
            Self::Cancelled
        } else {
            Self::Execution(error.to_string())
        }
    }
}

pub fn native_image_frontend_descriptors()
-> Result<BTreeMap<String, NodeDescriptor>, NativeImageRuntimeError> {
    native_image_descriptors()?
        .iter()
        .map(|descriptor| {
            let node = NodeDescriptor {
                type_name: descriptor.class_type.clone(),
                display_name: descriptor.display_name.clone(),
                inputs: descriptor
                    .inputs
                    .iter()
                    .map(|port| PortDescriptor {
                        name: port.name.clone(),
                        type_name: port.type_name.clone(),
                        required: port.required,
                    })
                    .collect(),
                outputs: descriptor
                    .outputs
                    .iter()
                    .map(|port| PortDescriptor {
                        name: port.name.clone(),
                        type_name: port.type_name.clone(),
                        required: port.required,
                    })
                    .collect(),
            };
            Ok((descriptor.class_type.clone(), node))
        })
        .collect()
}

pub fn native_diffusion_frontend_descriptors()
-> Result<BTreeMap<String, NodeDescriptor>, NativeImageRuntimeError> {
    let mut descriptors = native_image_frontend_descriptors()?;
    descriptors.retain(|class_type, _| class_type == "SaveImage");
    for descriptor in native_diffusion_descriptors()? {
        descriptors.insert(
            descriptor.class_type.clone(),
            NodeDescriptor {
                type_name: descriptor.class_type.clone(),
                display_name: descriptor.display_name.clone(),
                inputs: descriptor
                    .inputs
                    .iter()
                    .map(|port| PortDescriptor {
                        name: port.name.clone(),
                        type_name: port.type_name.clone(),
                        required: port.required,
                    })
                    .collect(),
                outputs: descriptor
                    .outputs
                    .iter()
                    .map(|port| PortDescriptor {
                        name: port.name.clone(),
                        type_name: port.type_name.clone(),
                        required: port.required,
                    })
                    .collect(),
            },
        );
    }
    Ok(descriptors)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeImageCatalogBinding {
    pub catalog: CatalogNodeDescriptor,
    pub frontend: NodeDescriptor,
    pub native: NativeImageDescriptor,
}

pub fn native_image_catalog_bindings()
-> Result<BTreeMap<String, NativeImageCatalogBinding>, NativeImageRuntimeError> {
    let catalog = NodeRegistry::built_in()
        .map_err(|error| NativeImageRuntimeError::Descriptors(error.to_string()))?;
    let frontend = native_image_frontend_descriptors()?;
    native_image_descriptors()?
        .iter()
        .map(|descriptor| {
            let class_type = descriptor.class_type.clone();
            let frontend = frontend.get(&class_type).cloned().ok_or_else(|| {
                NativeImageRuntimeError::Descriptors(format!(
                    "native image binding `{class_type}` has no frontend projection"
                ))
            })?;
            let catalog = catalog.descriptor(&class_type).cloned().ok_or_else(|| {
                NativeImageRuntimeError::Descriptors(format!(
                    "native image binding `{class_type}` has no canonical catalog row"
                ))
            })?;
            if catalog.display_name != frontend.display_name {
                return Err(NativeImageRuntimeError::Descriptors(format!(
                    "native image binding `{class_type}` changed canonical display metadata"
                )));
            }
            Ok((
                class_type,
                NativeImageCatalogBinding {
                    catalog,
                    frontend,
                    native: descriptor.clone(),
                },
            ))
        })
        .collect()
}

pub fn native_image_registry_projection() -> Result<NativeNodeRegistry, NativeImageRuntimeError> {
    let cpu_backend = projection_only_cpu_backend()?;
    native_image_registry_with_execution_state(
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(NativeTensorStore::default())),
        cpu_backend,
        true,
    )
}

fn projection_only_cpu_backend() -> Result<Arc<CpuBackend>, NativeImageRuntimeError> {
    let (backend, authority) =
        CpuWorkspaceAuthority::create_backend(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?;
    drop(authority);
    Ok(Arc::new(backend))
}

fn native_image_registry_with_execution_state(
    input_assets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    tensors: Arc<Mutex<NativeTensorStore>>,
    cpu_backend: Arc<CpuBackend>,
    metadata_enabled: bool,
) -> Result<NativeNodeRegistry, NativeImageRuntimeError> {
    let mut registry = NativeNodeRegistry::default();
    for descriptor in native_image_descriptors()? {
        registry.register_descriptor(runtime_descriptor(descriptor)?)?;
    }
    registry.register(Arc::new(LoadImageNode {
        input_assets,
        tensors: tensors.clone(),
        cpu_backend: cpu_backend.clone(),
    }))?;
    registry.register(Arc::new(ImageScaleNode {
        tensors: tensors.clone(),
        cpu_backend: cpu_backend.clone(),
    }))?;
    registry.register(Arc::new(ImageInvertNode {
        tensors: tensors.clone(),
        cpu_backend: cpu_backend.clone(),
    }))?;
    registry.register(Arc::new(SaveImageNode {
        tensors: tensors.clone(),
        cpu_backend: cpu_backend.clone(),
        namespace: AssetNamespace::Temporary,
        metadata_enabled,
    }))?;
    registry.register(Arc::new(SaveImageNode {
        tensors,
        cpu_backend,
        namespace: AssetNamespace::Output,
        metadata_enabled,
    }))?;
    if registry.descriptor_len() != 5
        || registry.node_len() != 5
        || !registry.descriptors_are_fully_bound()
    {
        return Err(NativeImageRuntimeError::Registry(format!(
            "native image registry has {} descriptors and {} implementations, expected 5 of each",
            registry.descriptor_len(),
            registry.node_len(),
        )));
    }
    Ok(registry)
}

fn native_registry_with_diffusion_provider(
    input_assets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    tensors: Arc<Mutex<NativeTensorStore>>,
    cpu_backend: Arc<CpuBackend>,
    metadata_enabled: bool,
    provider: Arc<dyn NativeDiffusionProvider>,
) -> Result<NativeNodeRegistry, NativeImageRuntimeError> {
    let mut registry = native_image_registry_with_execution_state(
        input_assets,
        tensors.clone(),
        cpu_backend.clone(),
        metadata_enabled,
    )?;
    for descriptor in native_diffusion_descriptors()? {
        registry.register_descriptor(runtime_descriptor(descriptor)?)?;
    }
    let state = Arc::new(NativeDiffusionState {
        provider,
        backend: cpu_backend.clone(),
        loaded: Mutex::new(None),
    });
    registry.register(Arc::new(CheckpointLoaderNode {
        state: state.clone(),
    }))?;
    registry.register(Arc::new(ClipTextEncodeNode {
        state: state.clone(),
        tensors: tensors.clone(),
    }))?;
    registry.register(Arc::new(EmptyLatentNode {
        backend: cpu_backend,
        tensors: tensors.clone(),
    }))?;
    registry.register(Arc::new(KSamplerNode {
        state: state.clone(),
        tensors: tensors.clone(),
    }))?;
    registry.register(Arc::new(VaeDecodeNode { state, tensors }))?;
    if registry.descriptor_len() != 10
        || registry.node_len() != 10
        || !registry.descriptors_are_fully_bound()
    {
        return Err(NativeImageRuntimeError::Registry(format!(
            "native early registry has {} descriptors and {} implementations, expected 10 unique bindings",
            registry.descriptor_len(),
            registry.node_len(),
        )));
    }
    Ok(registry)
}

pub fn compile_native_image_workflow(
    workflow_bytes: &[u8],
    selected_output_nodes: &std::collections::BTreeSet<NodeId>,
) -> Result<CompiledPlan, NativeImageRuntimeError> {
    let workflow = WorkflowFormatDocument::parse(workflow_bytes)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
    let descriptors = native_image_frontend_descriptors()?;
    let submission = graph_to_prompt(&workflow, &descriptors, "sim-native-image-v1")
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
    let registry = native_image_registry_projection()?;
    let mut plan = crate::PromptCompiler::new(&registry).compile(submission)?;
    if !selected_output_nodes.is_empty() {
        for node_id in selected_output_nodes {
            let node = plan.nodes.get(node_id).ok_or_else(|| {
                NativeImageRuntimeError::Registry(format!(
                    "selected output node {node_id:?} is not in the compiled plan"
                ))
            })?;
            if !node.descriptor.output_node {
                return Err(NativeImageRuntimeError::Registry(format!(
                    "selected node {node_id:?} is not an output node"
                )));
            }
        }
        plan.output_nodes = selected_output_nodes.iter().cloned().collect();
    }
    Ok(plan)
}

pub fn compile_native_diffusion_workflow(
    workflow_bytes: &[u8],
    selected_output_nodes: &std::collections::BTreeSet<NodeId>,
    provider: Arc<dyn NativeDiffusionProvider>,
) -> Result<CompiledPlan, NativeImageRuntimeError> {
    let workflow = WorkflowFormatDocument::parse(workflow_bytes)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
    let descriptors = native_diffusion_frontend_descriptors()?;
    let submission = graph_to_prompt(&workflow, &descriptors, "sim-native-diffusion-v1")
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
    let backend = projection_only_cpu_backend()?;
    let registry = native_registry_with_diffusion_provider(
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(NativeTensorStore::default())),
        backend,
        true,
        provider,
    )?;
    let mut plan = crate::PromptCompiler::new(&registry).compile(submission)?;
    if !selected_output_nodes.is_empty() {
        for node_id in selected_output_nodes {
            let node = plan.nodes.get(node_id).ok_or_else(|| {
                NativeImageRuntimeError::Registry(format!(
                    "selected output node {node_id:?} is not in the compiled plan"
                ))
            })?;
            if !node.descriptor.output_node {
                return Err(NativeImageRuntimeError::Registry(format!(
                    "selected node {node_id:?} is not an output node"
                )));
            }
        }
        plan.output_nodes = selected_output_nodes.iter().cloned().collect();
    }
    Ok(plan)
}

fn runtime_descriptor(
    descriptor: &NativeImageDescriptor,
) -> Result<RuntimeNodeDescriptor, NativeImageRuntimeError> {
    let inputs = descriptor
        .inputs
        .iter()
        .map(|port| {
            let value_type = value_type(&port.type_name);
            let allows_literal = !matches!(
                value_type,
                ValueType::Image | ValueType::Mask | ValueType::Tensor
            );
            (
                port.name.clone(),
                RuntimeInputDescriptor {
                    value_type,
                    required: port.required,
                    hidden: port.hidden,
                    lazy: false,
                    mode: InputMode::Scalar,
                    allows_literal,
                },
            )
        })
        .collect();
    let outputs = descriptor
        .outputs
        .iter()
        .map(|port| RuntimeOutputDescriptor {
            value_type: value_type(&port.type_name),
            is_list: false,
        })
        .collect();
    Ok(RuntimeNodeDescriptor {
        class_type: descriptor.class_type.clone(),
        implementation_version: descriptor.implementation_version.clone(),
        inputs,
        outputs,
        output_node: descriptor.output_node,
        availability: RuntimeAvailability::Native,
        effect: match descriptor.effect {
            NativeImageEffect::Pure => EffectClass::Pure,
            NativeImageEffect::ReadsArtifact => EffectClass::ReadsArtifact,
            NativeImageEffect::WritesArtifact => EffectClass::WritesArtifact,
        },
        cache: if descriptor.cache_by_input_identity {
            RuntimeCachePolicy::InputIdentity
        } else {
            RuntimeCachePolicy::Never
        },
    })
}

fn value_type(type_name: &str) -> ValueType {
    match type_name {
        "BOOLEAN" => ValueType::Boolean,
        "INT" => ValueType::Integer,
        "FLOAT" => ValueType::Number,
        "STRING" => ValueType::String,
        "IMAGE" => ValueType::Image,
        "MASK" => ValueType::Mask,
        "TENSOR" => ValueType::Tensor,
        "PROMPT" | "EXTRA_PNGINFO" => ValueType::Any,
        other => ValueType::Custom(other.to_owned()),
    }
}

#[derive(Default)]
struct NativeTensorStore {
    images: BTreeMap<String, Arc<ImageTensor>>,
    tensors: BTreeMap<String, Tensor>,
}

impl NativeTensorStore {
    fn insert(
        &mut self,
        kind: NativeTensorKind,
        tensor: ImageTensor,
    ) -> Result<NativeTensorHandle, NativeImageRuntimeError> {
        if !matches!(kind, NativeTensorKind::Image | NativeTensorKind::Mask) {
            return Err(NativeImageRuntimeError::Handle(
                "image tensor handles require image or mask kind".to_owned(),
            ));
        }
        let descriptor = tensor.tensor().descriptor().clone();
        let bytes = tensor.tensor().contiguous_bytes()?;
        let content_digest = native_tensor_digest(kind, &descriptor, &bytes)?;
        self.images
            .entry(content_digest.clone())
            .or_insert_with(|| Arc::new(tensor));
        NativeTensorHandle::new(kind, content_digest, descriptor)
    }

    fn get(
        &self,
        handle: &NativeTensorHandle,
        expected_kind: NativeTensorKind,
    ) -> Result<Arc<ImageTensor>, NativeImageRuntimeError> {
        handle.validate(expected_kind)?;
        if !matches!(
            expected_kind,
            NativeTensorKind::Image | NativeTensorKind::Mask
        ) {
            return Err(NativeImageRuntimeError::Handle(
                "image tensor handle kind or schema is invalid".to_owned(),
            ));
        }
        let tensor = self
            .images
            .get(handle.content_digest())
            .cloned()
            .ok_or_else(|| {
                NativeImageRuntimeError::Handle(format!(
                    "tensor {} is not present in this worker",
                    handle.content_digest()
                ))
            })?;
        if tensor.tensor().descriptor() != handle.descriptor() {
            return Err(NativeImageRuntimeError::Handle(
                "handle descriptor does not match worker storage".to_owned(),
            ));
        }
        let bytes = tensor.tensor().contiguous_bytes()?;
        if native_tensor_digest(expected_kind, tensor.tensor().descriptor(), &bytes)?
            != handle.content_digest()
        {
            return Err(NativeImageRuntimeError::Handle(
                "image tensor content identity changed".to_owned(),
            ));
        }
        Ok(tensor)
    }

    fn insert_tensor(
        &mut self,
        kind: NativeTensorKind,
        tensor: Tensor,
    ) -> Result<NativeTensorHandle, NativeImageRuntimeError> {
        if !matches!(
            kind,
            NativeTensorKind::Conditioning | NativeTensorKind::Latent
        ) {
            return Err(NativeImageRuntimeError::Handle(
                "raw tensor handles require conditioning or latent kind".to_owned(),
            ));
        }
        let descriptor = tensor.descriptor().clone();
        let bytes = tensor.contiguous_bytes()?;
        let content_digest = native_tensor_digest(kind, &descriptor, &bytes)?;
        self.tensors.entry(content_digest.clone()).or_insert(tensor);
        NativeTensorHandle::new(kind, content_digest, descriptor)
    }

    fn get_tensor(
        &self,
        handle: &NativeTensorHandle,
        expected_kind: NativeTensorKind,
    ) -> Result<Tensor, NativeImageRuntimeError> {
        handle.validate(expected_kind)?;
        if !matches!(
            expected_kind,
            NativeTensorKind::Conditioning | NativeTensorKind::Latent
        ) {
            return Err(NativeImageRuntimeError::Handle(
                "raw tensor handle kind or schema is invalid".to_owned(),
            ));
        }
        let tensor = self
            .tensors
            .get(handle.content_digest())
            .cloned()
            .ok_or_else(|| {
                NativeImageRuntimeError::Handle(format!(
                    "tensor {} is not present in this worker",
                    handle.content_digest()
                ))
            })?;
        if tensor.descriptor() != handle.descriptor() {
            return Err(NativeImageRuntimeError::Handle(
                "raw tensor descriptor changed".to_owned(),
            ));
        }
        let bytes = tensor.contiguous_bytes()?;
        if native_tensor_digest(expected_kind, tensor.descriptor(), &bytes)?
            != handle.content_digest()
        {
            return Err(NativeImageRuntimeError::Handle(
                "raw tensor content identity changed".to_owned(),
            ));
        }
        Ok(tensor)
    }
}

fn native_tensor_digest(
    kind: NativeTensorKind,
    descriptor: &TensorDescriptor,
    bytes: &[u8],
) -> Result<String, NativeImageRuntimeError> {
    let mut hasher = Sha256::new();
    hasher.update([match kind {
        NativeTensorKind::Image => 1,
        NativeTensorKind::Mask => 2,
        NativeTensorKind::Conditioning => 3,
        NativeTensorKind::Latent => 4,
    }]);
    hasher.update(
        serde_json::to_vec(descriptor)
            .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))?,
    );
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NativeDiffusionRole {
    Model,
    Clip,
    Vae,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct NativeDiffusionHandle {
    schema_version: u16,
    role: NativeDiffusionRole,
    fixture_id: String,
    model_digest: String,
    tokenizer_digest: String,
    clip_execution_digest: String,
}

struct NativeDiffusionState {
    provider: Arc<dyn NativeDiffusionProvider>,
    backend: Arc<CpuBackend>,
    loaded: Mutex<Option<Arc<NativeDiffusionBundle>>>,
}

impl NativeDiffusionState {
    fn bundle(
        &self,
        context: &ExecutionContext<'_>,
    ) -> Result<Arc<NativeDiffusionBundle>, NativeImageRuntimeError> {
        if let Some(bundle) = self.loaded.lock().clone() {
            context.check()?;
            return Ok(bundle);
        }
        let bundle = Arc::new(self.provider.load(self.backend.clone(), context)?);
        let provider_model_digest = self.provider.model_digest()?;
        let provider_tokenizer_digest = self.provider.tokenizer_digest()?;
        let provider_clip_cache_identities = self.provider.clip_cache_identities()?;
        if provider_model_digest != bundle.model_digest
            || provider_tokenizer_digest != bundle.tokenizer_digest
            || provider_clip_cache_identities != bundle.clip_cache_identities
        {
            return Err(NativeImageRuntimeError::Registry(
                "native diffusion provider cache identity does not match its loaded bundle"
                    .to_owned(),
            ));
        }
        let mut loaded = self.loaded.lock();
        Ok(loaded.get_or_insert_with(|| bundle.clone()).clone())
    }

    fn handle(
        &self,
        role: NativeDiffusionRole,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionHandle, NativeImageRuntimeError> {
        let bundle = self.bundle(context)?;
        Ok(NativeDiffusionHandle {
            schema_version: 3,
            role,
            fixture_id: bundle.fixture_id.clone(),
            model_digest: bundle.model_digest.clone(),
            tokenizer_digest: bundle.tokenizer_digest.clone(),
            clip_execution_digest: bundle.clip_cache_identities.execution().to_owned(),
        })
    }

    fn resolve(
        &self,
        handle: &NativeDiffusionHandle,
        role: NativeDiffusionRole,
        context: &ExecutionContext<'_>,
    ) -> Result<Arc<NativeDiffusionBundle>, NativeImageRuntimeError> {
        let bundle = self.bundle(context)?;
        if handle.schema_version != 3
            || handle.role != role
            || handle.fixture_id != bundle.fixture_id
            || handle.model_digest != bundle.model_digest
            || handle.tokenizer_digest != bundle.tokenizer_digest
            || handle.clip_execution_digest != bundle.clip_cache_identities.execution()
        {
            return Err(NativeImageRuntimeError::Handle(
                "native diffusion handle does not belong to the loaded model".to_owned(),
            ));
        }
        Ok(bundle)
    }
}

struct CheckpointLoaderNode {
    state: Arc<NativeDiffusionState>,
}

impl NativeNode for CheckpointLoaderNode {
    fn class_type(&self) -> &str {
        "CheckpointLoaderSimple"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn cache_dependencies(
        &self,
        _inputs: &BTreeMap<String, Value>,
    ) -> Result<CacheDependencies, NodeFailure> {
        let model_digest = self
            .state
            .provider
            .model_digest()
            .map_err(runtime_failure)?;
        let tokenizer_digest = self
            .state
            .provider
            .tokenizer_digest()
            .map_err(runtime_failure)?;
        let clip_identities = self
            .state
            .provider
            .clip_cache_identities()
            .map_err(runtime_failure)?;
        let mut artifact_digests = clip_identities.artifact_digests();
        artifact_digests.insert("model.safetensors".to_owned(), model_digest);
        artifact_digests.insert("tokenizer.sd1".to_owned(), tokenizer_digest);
        Ok(CacheDependencies {
            artifact_digests,
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            if required_string(&inputs, "ckpt_name")? != "model.safetensors" {
                return Err(invalid_diffusion_input(
                    "only the pinned model.safetensors checkpoint is admitted by this slice",
                ));
            }
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let model = self
                .state
                .handle(NativeDiffusionRole::Model, &tensor_context)
                .map_err(runtime_failure)?;
            let clip = self
                .state
                .handle(NativeDiffusionRole::Clip, &tensor_context)
                .map_err(runtime_failure)?;
            let vae = self
                .state
                .handle(NativeDiffusionRole::Vae, &tensor_context)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![
                    serde_json::to_value(model).map_err(encoding_failure)?,
                    serde_json::to_value(clip).map_err(encoding_failure)?,
                    serde_json::to_value(vae).map_err(encoding_failure)?,
                ],
                ui: Some(json!({"checkpoint": "model.safetensors", "family": "COMFY-MODEL-0117"})),
                effects: Vec::new(),
            })
        })
    }
}

struct ClipTextEncodeNode {
    state: Arc<NativeDiffusionState>,
    tensors: Arc<Mutex<NativeTensorStore>>,
}

impl NativeNode for ClipTextEncodeNode {
    fn class_type(&self) -> &str {
        "CLIPTextEncode"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn cache_dependencies(
        &self,
        _inputs: &BTreeMap<String, Value>,
    ) -> Result<CacheDependencies, NodeFailure> {
        Ok(CacheDependencies {
            artifact_digests: self
                .state
                .provider
                .clip_cache_identities()
                .map_err(runtime_failure)?
                .artifact_digests(),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let handle: NativeDiffusionHandle = required_json(&inputs, "clip")?;
            let bundle = self
                .state
                .resolve(&handle, NativeDiffusionRole::Clip, &tensor_context)
                .map_err(runtime_failure)?;
            let text = required_string(&inputs, "text")?;
            let (tokens, conditioning) = bundle
                .encode_text(text, &tensor_context)
                .map_err(runtime_failure)?;
            let conditioning = self
                .tensors
                .lock()
                .insert_tensor(NativeTensorKind::Conditioning, conditioning)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(conditioning).map_err(encoding_failure)?],
                ui: Some(json!({"tokens": tokens.to_vec()})),
                effects: Vec::new(),
            })
        })
    }
}

struct EmptyLatentNode {
    backend: Arc<CpuBackend>,
    tensors: Arc<Mutex<NativeTensorStore>>,
}

impl NativeNode for EmptyLatentNode {
    fn class_type(&self) -> &str {
        "EmptyLatentImage"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.backend, &context);
            let latent = empty_sd15_latent(
                &self.backend,
                required_u64(&inputs, "batch_size")?,
                required_u64(&inputs, "width")?,
                required_u64(&inputs, "height")?,
                &tensor_context,
            )
            .map_err(diffusion_failure)?;
            let handle = self
                .tensors
                .lock()
                .insert_tensor(NativeTensorKind::Latent, latent)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(handle).map_err(encoding_failure)?],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

struct KSamplerNode {
    state: Arc<NativeDiffusionState>,
    tensors: Arc<Mutex<NativeTensorStore>>,
}

impl NativeNode for KSamplerNode {
    fn class_type(&self) -> &str {
        "KSampler"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn cache_change_token(&self, _inputs: &BTreeMap<String, Value>) -> Result<String, NodeFailure> {
        Ok(GUIDANCE_ADAPTER_ID.to_owned())
    }

    fn cache_dependencies(
        &self,
        inputs: &BTreeMap<String, Value>,
    ) -> Result<CacheDependencies, NodeFailure> {
        Ok(CacheDependencies {
            rng_phase: Some(format!(
                "{INITIAL_NOISE_PHASE_ID}:{}",
                required_u64(inputs, "seed")?
            )),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let model_handle: NativeDiffusionHandle = required_json(&inputs, "model")?;
            let bundle = self
                .state
                .resolve(&model_handle, NativeDiffusionRole::Model, &tensor_context)
                .map_err(runtime_failure)?;
            let steps = u32::try_from(required_u64(&inputs, "steps")?)
                .map_err(|_| invalid_diffusion_input("steps exceed u32"))?;
            let seed = required_u64(&inputs, "seed")?;
            let plan = checked_native_diffusion_plan(
                required_string(&inputs, "sampler_name")?,
                required_string(&inputs, "scheduler")?,
                seed,
                steps,
                required_f32(&inputs, "cfg")?,
                required_f32(&inputs, "denoise")?,
            )
            .map_err(diffusion_failure)?;
            let positive_handle: NativeTensorHandle = required_json(&inputs, "positive")?;
            let negative_handle: NativeTensorHandle = required_json(&inputs, "negative")?;
            let latent_handle: NativeTensorHandle = required_json(&inputs, "latent_image")?;
            let (positive, negative, latent) = {
                let tensors = self.tensors.lock();
                (
                    tensors
                        .get_tensor(&positive_handle, NativeTensorKind::Conditioning)
                        .map_err(runtime_failure)?,
                    tensors
                        .get_tensor(&negative_handle, NativeTensorKind::Conditioning)
                        .map_err(runtime_failure)?,
                    tensors
                        .get_tensor(&latent_handle, NativeTensorKind::Latent)
                        .map_err(runtime_failure)?,
                )
            };
            let stream = NoiseRequest::native_diffusion(
                context.prompt_id.0.to_string(),
                context.node_id.0.clone(),
            )
            .and_then(|request| request.stream(plan.seed(), comfy_tensor::DeviceId::CPU))
            .map_err(diffusion_failure)?;
            let sigmas = normal_sigmas(
                &self.state.backend,
                &tensor_context,
                usize::try_from(plan.steps())
                    .map_err(|_| invalid_diffusion_input("steps exceed usize"))?,
                plan.denoise(),
            )
            .map_err(diffusion_failure)?;
            let noise = normal_noise(
                &self.state.backend,
                latent.descriptor().shape(),
                &stream,
                &tensor_context,
            )
            .map_err(diffusion_failure)?;
            let initial_sigma = sigmas
                .first()
                .copied()
                .ok_or_else(|| invalid_diffusion_input("normal scheduler returned no sigmas"))?;
            let initial = scale_initial_noise(
                &self.state.backend,
                &noise.noise,
                &latent,
                initial_sigma,
                &tensor_context,
            )
            .map_err(diffusion_failure)?;
            let mut guidance =
                Sd15GuidanceAdapter::checked(&bundle.model, &positive, &negative, &tensor_context)
                    .map_err(diffusion_failure)?;
            let trace = sample_euler(
                &self.state.backend,
                initial,
                &sigmas,
                &tensor_context,
                |latent, sigma, _step| {
                    let model_input =
                        scale_model_input(&self.state.backend, latent, sigma, &tensor_context)
                            .map_err(|error| error.to_string())?;
                    let prediction = guidance
                        .execute(
                            &self.state.backend,
                            &model_input,
                            sigma,
                            &plan,
                            &tensor_context,
                        )
                        .map_err(|error| error.to_string())?;
                    sd15_interpret_prediction(
                        &self.state.backend,
                        prediction.guided(),
                        latent,
                        sigma,
                        &tensor_context,
                    )
                    .map_err(|error| error.to_string())
                },
            )
            .map_err(diffusion_failure)?;
            let final_latent = trace
                .latents
                .last()
                .cloned()
                .ok_or_else(|| invalid_diffusion_input("Euler returned no latent"))?;
            let handle = self
                .tensors
                .lock()
                .insert_tensor(NativeTensorKind::Latent, final_latent)
                .map_err(runtime_failure)?;
            let denoiser_sha256 = trace
                .denoiser_evaluations
                .iter()
                .map(tensor_sha256)
                .collect::<Result<Vec<_>, _>>()
                .map_err(runtime_failure)?;
            let latent_sha256 = trace
                .latents
                .iter()
                .map(tensor_sha256)
                .collect::<Result<Vec<_>, _>>()
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(handle).map_err(encoding_failure)?],
                ui: Some(json!({
                    "sigmas": sigmas,
                    "noise_sha256": tensor_sha256(&noise.noise).map_err(runtime_failure)?,
                    "denoiser_sha256": denoiser_sha256,
                    "latent_sha256": latent_sha256,
                })),
                effects: Vec::new(),
            })
        })
    }
}

struct VaeDecodeNode {
    state: Arc<NativeDiffusionState>,
    tensors: Arc<Mutex<NativeTensorStore>>,
}

impl NativeNode for VaeDecodeNode {
    fn class_type(&self) -> &str {
        "VAEDecode"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let vae_handle: NativeDiffusionHandle = required_json(&inputs, "vae")?;
            let bundle = self
                .state
                .resolve(&vae_handle, NativeDiffusionRole::Vae, &tensor_context)
                .map_err(runtime_failure)?;
            let latent_handle: NativeTensorHandle = required_json(&inputs, "samples")?;
            let latent = self
                .tensors
                .lock()
                .get_tensor(&latent_handle, NativeTensorKind::Latent)
                .map_err(runtime_failure)?;
            let decoded = bundle
                .model
                .decode(&latent, &tensor_context)
                .map_err(diffusion_failure)?;
            if decoded.descriptor().shape() != [1, 3, 32, 32] {
                return Err(invalid_diffusion_input(
                    "native SD15 VAE returned an unexpected image shape",
                ));
            }
            let nchw = tensor_to_f32(&self.state.backend, &decoded, &tensor_context)
                .map_err(diffusion_failure)?;
            let mut bhwc = self
                .state
                .backend
                .workspace_vec(&tensor_context, nchw.len())
                .map_err(tensor_failure)?;
            for _ in 0..nchw.len() {
                bhwc.try_push(0.0).map_err(tensor_failure)?;
            }
            for y in 0..32 {
                for x in 0..32 {
                    for channel in 0..3 {
                        bhwc[(y * 32 + x) * 3 + channel] = nchw[(channel * 32 + y) * 32 + x];
                    }
                }
            }
            let image =
                ImageTensor::from_f32(&self.state.backend, &tensor_context, 1, 32, 32, 3, &bhwc)
                    .map_err(tensor_failure)?;
            let handle = self
                .tensors
                .lock()
                .insert(NativeTensorKind::Image, image)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(handle).map_err(encoding_failure)?],
                ui: Some(json!({"width": 32, "height": 32})),
                effects: Vec::new(),
            })
        })
    }
}

#[derive(Clone)]
pub struct NativeImageExecutor {
    profile_id: ProfileId,
    input_assets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    nodes: Arc<NativeNodeRegistry>,
    cache: Arc<Mutex<NativeCache>>,
    cpu_backend: Arc<CpuBackend>,
    metadata_enabled: bool,
    diffusion_enabled: bool,
}

impl NativeImageExecutor {
    #[cfg(test)]
    pub fn new(
        profile_id: ProfileId,
        input_assets: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, NativeImageRuntimeError> {
        Self::new_with_metadata_enabled(profile_id, input_assets, true)
    }

    #[cfg(test)]
    pub fn new_with_metadata_enabled(
        profile_id: ProfileId,
        input_assets: BTreeMap<String, Vec<u8>>,
        metadata_enabled: bool,
    ) -> Result<Self, NativeImageRuntimeError> {
        let cpu_backend = projection_only_cpu_backend()?;
        Self::new_with_cpu_backend(profile_id, input_assets, metadata_enabled, cpu_backend)
    }

    pub fn new_with_cpu_backend(
        profile_id: ProfileId,
        input_assets: BTreeMap<String, Vec<u8>>,
        metadata_enabled: bool,
        cpu_backend: Arc<CpuBackend>,
    ) -> Result<Self, NativeImageRuntimeError> {
        validate_worker_input_assets(&input_assets)?;
        let input_assets = Arc::new(Mutex::new(input_assets));
        let tensors = Arc::new(Mutex::new(NativeTensorStore::default()));
        let nodes = native_image_registry_with_execution_state(
            input_assets.clone(),
            tensors,
            cpu_backend.clone(),
            metadata_enabled,
        )?;
        Ok(Self {
            profile_id,
            input_assets,
            nodes: Arc::new(nodes),
            cache: Arc::new(Mutex::new(NativeCache::new(4096).map_err(|error| {
                NativeImageRuntimeError::Execution(error.to_string())
            })?)),
            cpu_backend,
            metadata_enabled,
            diffusion_enabled: false,
        })
    }

    pub fn new_with_diffusion_provider(
        profile_id: ProfileId,
        input_assets: BTreeMap<String, Vec<u8>>,
        metadata_enabled: bool,
        cpu_backend: Arc<CpuBackend>,
        provider: Arc<dyn NativeDiffusionProvider>,
    ) -> Result<Self, NativeImageRuntimeError> {
        validate_worker_input_assets(&input_assets)?;
        let input_assets = Arc::new(Mutex::new(input_assets));
        let tensors = Arc::new(Mutex::new(NativeTensorStore::default()));
        let nodes = native_registry_with_diffusion_provider(
            input_assets.clone(),
            tensors,
            cpu_backend.clone(),
            metadata_enabled,
            provider,
        )?;
        Ok(Self {
            profile_id,
            input_assets,
            nodes: Arc::new(nodes),
            cache: Arc::new(Mutex::new(NativeCache::new(4096).map_err(|error| {
                NativeImageRuntimeError::Execution(error.to_string())
            })?)),
            cpu_backend,
            metadata_enabled,
            diffusion_enabled: true,
        })
    }

    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    pub fn replace_input_assets(
        &self,
        input_assets: BTreeMap<String, Vec<u8>>,
    ) -> Result<(), NativeImageRuntimeError> {
        validate_worker_input_assets(&input_assets)?;
        *self.input_assets.lock() = input_assets;
        Ok(())
    }

    pub fn metadata_enabled(&self) -> bool {
        self.metadata_enabled
    }

    pub fn diffusion_enabled(&self) -> bool {
        self.diffusion_enabled
    }

    pub fn cpu_backend(&self) -> &Arc<CpuBackend> {
        &self.cpu_backend
    }

    pub fn execute_blocking(
        &self,
        plan: &CompiledPlan,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
        injected_delay_millis: u64,
        workspace: ScratchReservation,
    ) -> Result<NativeImageExecutionResult, NativeImageRuntimeError> {
        self.execute_blocking_with_event_bus(
            plan,
            attempt_id,
            cancellation,
            injected_delay_millis,
            None,
            workspace,
        )
    }

    pub fn execute_blocking_with_event_bus(
        &self,
        plan: &CompiledPlan,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
        injected_delay_millis: u64,
        event_bus: Option<ExecutionEventBus>,
        workspace: ScratchReservation,
    ) -> Result<NativeImageExecutionResult, NativeImageRuntimeError> {
        self.execute_blocking_with_event_bus_and_configuration(
            plan,
            attempt_id,
            cancellation,
            injected_delay_millis,
            event_bus,
            workspace,
            "balanced",
        )
    }

    pub fn execute_blocking_with_event_bus_and_configuration(
        &self,
        plan: &CompiledPlan,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
        injected_delay_millis: u64,
        event_bus: Option<ExecutionEventBus>,
        workspace: ScratchReservation,
        memory_configuration: &str,
    ) -> Result<NativeImageExecutionResult, NativeImageRuntimeError> {
        if memory_configuration.is_empty() {
            return Err(NativeImageRuntimeError::Execution(
                "native memory configuration token is empty".to_owned(),
            ));
        }
        cancellable_delay(injected_delay_millis, &cancellation)?;
        let effects = Arc::new(NativeImageProposalCoordinator::default());
        let registry_version = if self.diffusion_enabled {
            NATIVE_DIFFUSION_REGISTRY_VERSION
        } else {
            NATIVE_IMAGE_REGISTRY_VERSION
        };
        let configuration_token = if self.diffusion_enabled {
            format!("native-diffusion-v1:{memory_configuration}")
        } else {
            format!("native-image-v1:{memory_configuration}")
        };
        let mut engine = ExecutionEngine::new_with_workspace_authorization(
            self.profile_id,
            self.nodes.clone(),
            self.cache.clone(),
            effects.clone(),
            registry_version,
            workspace,
        )?
        .with_backend("cpu")?
        .with_dtype_policy("f32")?
        .with_configuration_token(configuration_token)?;
        if let Some(event_bus) = event_bus {
            engine = engine.with_event_bus(event_bus);
        }
        let report = smol::block_on(engine.execute(plan, attempt_id, cancellation));
        let output_proposals = effects.output_proposals();
        let executed_node_count = report.outputs.len();
        Ok(NativeImageExecutionResult {
            report,
            output_proposals,
            executed_node_count,
        })
    }
}

fn cancellable_delay(
    delay_millis: u64,
    cancellation: &CancellationToken,
) -> Result<(), NativeImageRuntimeError> {
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(delay_millis))
        .ok_or_else(|| NativeImageRuntimeError::Execution("delay overflowed".to_owned()))?;
    while Instant::now() < deadline {
        cancellation
            .check()
            .map_err(|_| NativeImageRuntimeError::Cancelled)?;
        thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(5)),
        );
    }
    cancellation
        .check()
        .map_err(|_| NativeImageRuntimeError::Cancelled)?;
    Ok(())
}

fn native_image_tensor_context<'a>(
    backend: &CpuBackend,
    context: &'a NodeContext,
) -> ExecutionContext<'a> {
    backend.execution_context(
        StreamId::DEFAULT,
        context.scratch.clone(),
        &context.cancellation,
    )
}

struct LoadImageNode {
    input_assets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    tensors: Arc<Mutex<NativeTensorStore>>,
    cpu_backend: Arc<CpuBackend>,
}

impl NativeNode for LoadImageNode {
    fn class_type(&self) -> &str {
        "LoadImage"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_IMAGE_REGISTRY_VERSION
    }

    fn cache_change_token(&self, inputs: &BTreeMap<String, Value>) -> Result<String, NodeFailure> {
        self.input_digest(inputs)
    }

    fn cache_dependencies(
        &self,
        inputs: &BTreeMap<String, Value>,
    ) -> Result<CacheDependencies, NodeFailure> {
        let image = required_string(inputs, "image")?;
        let digest = self.input_digest(inputs)?;
        Ok(CacheDependencies {
            artifact_digests: BTreeMap::from([(image.to_owned(), digest)]),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            context.cancellation.check().map_err(cancellation_failure)?;
            let logical_id = required_string(&inputs, "image")?;
            let input_assets = self.input_assets.lock();
            let bytes = input_assets.get(logical_id).ok_or_else(|| NodeFailure {
                code: "native_image_input_missing".to_owned(),
                message: format!("worker input `{logical_id}` was not supplied by the host"),
                kind: NodeFailureKind::Failure,
                retryable: false,
            })?;
            let decoded = decode_png(&bytes, PngLimits::default()).map_err(media_failure)?;
            let tensor_context = native_image_tensor_context(&self.cpu_backend, &context);
            let image = ImageTensor::from_f32(
                &self.cpu_backend,
                &tensor_context,
                1,
                u64::from(decoded.height),
                u64::from(decoded.width),
                3,
                &decoded.pixels_bhwc,
            )
            .map_err(tensor_failure)?;
            let mask = ImageTensor::from_f32(
                &self.cpu_backend,
                &tensor_context,
                1,
                u64::from(decoded.mask_height),
                u64::from(decoded.mask_width),
                1,
                &decoded.mask_bhw,
            )
            .map_err(tensor_failure)?;
            let mut tensors = self.tensors.lock();
            let image = tensors
                .insert(NativeTensorKind::Image, image)
                .map_err(runtime_failure)?;
            let mask = tensors
                .insert(NativeTensorKind::Mask, mask)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![
                    serde_json::to_value(image).map_err(encoding_failure)?,
                    serde_json::to_value(mask).map_err(encoding_failure)?,
                ],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

impl LoadImageNode {
    fn input_digest(&self, inputs: &BTreeMap<String, Value>) -> Result<String, NodeFailure> {
        let logical_id = required_string(inputs, "image")?;
        let input_assets = self.input_assets.lock();
        let bytes = input_assets.get(logical_id).ok_or_else(|| NodeFailure {
            code: "native_image_input_missing".to_owned(),
            message: format!("worker input `{logical_id}` was not supplied by the host"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        })?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

struct ImageScaleNode {
    tensors: Arc<Mutex<NativeTensorStore>>,
    cpu_backend: Arc<CpuBackend>,
}

impl NativeNode for ImageScaleNode {
    fn class_type(&self) -> &str {
        "ImageScale"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_IMAGE_REGISTRY_VERSION
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let handle = required_handle(&inputs, "image")?;
            let image = self
                .tensors
                .lock()
                .get(&handle, NativeTensorKind::Image)
                .map_err(runtime_failure)?;
            let mode = parse_resize_mode(required_string(&inputs, "upscale_method")?)?;
            let crop = parse_resize_crop(required_string(&inputs, "crop")?)?;
            let width = required_u64(&inputs, "width")?;
            let height = required_u64(&inputs, "height")?;
            let tensor_context = native_image_tensor_context(&self.cpu_backend, &context);
            let resized = image
                .resize(
                    width,
                    height,
                    mode,
                    crop,
                    &self.cpu_backend,
                    &tensor_context,
                )
                .map_err(tensor_failure)?;
            let handle = self
                .tensors
                .lock()
                .insert(NativeTensorKind::Image, resized)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(handle).map_err(encoding_failure)?],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

struct ImageInvertNode {
    tensors: Arc<Mutex<NativeTensorStore>>,
    cpu_backend: Arc<CpuBackend>,
}

impl NativeNode for ImageInvertNode {
    fn class_type(&self) -> &str {
        "ImageInvert"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_IMAGE_REGISTRY_VERSION
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let handle = required_handle(&inputs, "image")?;
            let image = self
                .tensors
                .lock()
                .get(&handle, NativeTensorKind::Image)
                .map_err(runtime_failure)?;
            let tensor_context = native_image_tensor_context(&self.cpu_backend, &context);
            let inverted = image
                .invert(&self.cpu_backend, &tensor_context)
                .map_err(tensor_failure)?;
            let handle = self
                .tensors
                .lock()
                .insert(NativeTensorKind::Image, inverted)
                .map_err(runtime_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(handle).map_err(encoding_failure)?],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

struct SaveImageNode {
    tensors: Arc<Mutex<NativeTensorStore>>,
    cpu_backend: Arc<CpuBackend>,
    namespace: AssetNamespace,
    metadata_enabled: bool,
}

impl NativeNode for SaveImageNode {
    fn class_type(&self) -> &str {
        match self.namespace {
            AssetNamespace::Temporary => "PreviewImage",
            _ => "SaveImage",
        }
    }

    fn implementation_version(&self) -> &str {
        NATIVE_IMAGE_REGISTRY_VERSION
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let handle = required_handle(&inputs, "images")?;
            let image = self
                .tensors
                .lock()
                .get(&handle, NativeTensorKind::Image)
                .map_err(runtime_failure)?;
            let (batch, height, width, channels) = image.dimensions().map_err(tensor_failure)?;
            let pixels = image.as_f32_slice().map_err(tensor_failure)?;
            let prefix = if self.namespace == AssetNamespace::Temporary {
                "ComfyUI_temp".to_owned()
            } else {
                inputs
                    .get("filename_prefix")
                    .and_then(Value::as_str)
                    .unwrap_or("ComfyUI")
                    .to_owned()
            };
            let metadata = png_metadata(&inputs)?;
            let tensor_context = native_image_tensor_context(&self.cpu_backend, &context);
            let mut effects = Vec::new();
            let mut ui_images = Vec::new();
            for batch_index in 0..batch {
                context.cancellation.check().map_err(cancellation_failure)?;
                let encoded = encode_png_frame_with_policy_and_context(
                    &self.cpu_backend,
                    &tensor_context,
                    pixels,
                    batch,
                    height,
                    width,
                    channels,
                    batch_index,
                    &metadata,
                    MetadataWritePolicy {
                        metadata_enabled: self.metadata_enabled,
                    },
                    PngLimits::default(),
                )
                .map_err(media_failure)?;
                let batch_index = u32::try_from(batch_index).map_err(|_| NodeFailure {
                    code: "image_batch_too_large".to_owned(),
                    message: "image batch index does not fit the output contract".to_owned(),
                    kind: NodeFailureKind::Failure,
                    retryable: false,
                })?;
                let effect = NativeImageEffectRequest {
                    namespace: self.namespace,
                    filename_prefix: prefix.clone(),
                    batch_index,
                    width: u32::try_from(width).map_err(|_| dimension_failure())?,
                    height: u32::try_from(height).map_err(|_| dimension_failure())?,
                    encoded_png: encoded,
                };
                let metadata = serde_json::to_vec(&effect).map_err(encoding_failure)?;
                let transaction_id =
                    transaction_id(&context, self.namespace, batch_index, &metadata);
                effects.push(PreparedEffectRequest {
                    transaction_id,
                    metadata,
                });
                ui_images.push(json!({
                    "transaction_id": transaction_id,
                    "batch_index": batch_index,
                    "type": self.namespace.locator_type(),
                }));
            }
            Ok(NodeOutcome::Values {
                outputs: vec![serde_json::to_value(handle).map_err(encoding_failure)?],
                ui: Some(json!({"images": ui_images})),
                effects,
            })
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NativeImageEffectRequest {
    namespace: AssetNamespace,
    filename_prefix: String,
    batch_index: u32,
    width: u32,
    height: u32,
    encoded_png: Vec<u8>,
}

#[derive(Clone)]
struct PreparedNativeImage {
    effect: PreparedEffect,
    proposal: NativeImageOutputProposal,
}

#[derive(Default)]
struct NativeImageEffectState {
    prepared: BTreeMap<Uuid, PreparedNativeImage>,
    proposed: Vec<NativeImageOutputProposal>,
}

#[derive(Default)]
struct NativeImageProposalCoordinator {
    state: Mutex<NativeImageEffectState>,
}

impl NativeImageProposalCoordinator {
    fn output_proposals(&self) -> Vec<NativeImageOutputProposal> {
        self.state.lock().proposed.clone()
    }
}

impl EffectCoordinator for NativeImageProposalCoordinator {
    fn prepare(&self, effect: PreparedEffect) -> Result<PreparedEffect, String> {
        let request: NativeImageEffectRequest =
            serde_json::from_slice(&effect.metadata).map_err(|error| error.to_string())?;
        let output = OutputProposal::new(
            effect.transaction_id,
            request.namespace,
            request.filename_prefix,
            "png",
            request.batch_index,
            request.width,
            request.height,
            request.encoded_png,
        )
        .map_err(|error| error.to_string())?;
        let mut state = self.state.lock();
        if state.prepared.contains_key(&effect.transaction_id) {
            return Err(format!(
                "duplicate native image transaction {}",
                effect.transaction_id
            ));
        }
        state.prepared.insert(
            effect.transaction_id,
            PreparedNativeImage {
                effect: effect.clone(),
                proposal: NativeImageOutputProposal::new(effect.node_id.clone(), output)
                    .map_err(|error| error.to_string())?,
            },
        );
        Ok(effect)
    }

    fn commit_batch(&self, effects: &[PreparedEffect]) -> Result<(), String> {
        let mut state = self.state.lock();
        let mut prepared_batch = Vec::with_capacity(effects.len());
        for effect in effects {
            let prepared = state
                .prepared
                .get(&effect.transaction_id)
                .cloned()
                .ok_or_else(|| format!("unprepared transaction {}", effect.transaction_id))?;
            if prepared.effect != *effect {
                return Err(format!(
                    "prepared transaction {} changed before commit",
                    effect.transaction_id
                ));
            }
            prepared_batch.push(prepared);
        }
        for (effect, prepared) in effects.iter().zip(prepared_batch) {
            state.prepared.remove(&effect.transaction_id);
            state.proposed.push(prepared.proposal);
        }
        Ok(())
    }

    fn rollback_batch(&self, effects: &[PreparedEffect]) -> Result<(), String> {
        let mut state = self.state.lock();
        for effect in effects {
            state.prepared.remove(&effect.transaction_id);
        }
        Ok(())
    }
}

fn required_string<'a>(
    inputs: &'a BTreeMap<String, Value>,
    name: &str,
) -> Result<&'a str, NodeFailure> {
    inputs
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| NodeFailure {
            code: "invalid_native_image_input".to_owned(),
            message: format!("`{name}` must be a string"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        })
}

fn required_u64(inputs: &BTreeMap<String, Value>, name: &str) -> Result<u64, NodeFailure> {
    inputs
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| NodeFailure {
            code: "invalid_native_image_input".to_owned(),
            message: format!("`{name}` must be a non-negative integer"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        })
}

fn required_f32(inputs: &BTreeMap<String, Value>, name: &str) -> Result<f32, NodeFailure> {
    let value = inputs
        .get(name)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid_diffusion_input(&format!("`{name}` must be a finite number")))?;
    Ok(value)
}

fn required_json<T: serde::de::DeserializeOwned>(
    inputs: &BTreeMap<String, Value>,
    name: &str,
) -> Result<T, NodeFailure> {
    serde_json::from_value(
        inputs
            .get(name)
            .cloned()
            .ok_or_else(|| invalid_diffusion_input(&format!("`{name}` is missing")))?,
    )
    .map_err(encoding_failure)
}

fn invalid_diffusion_input(message: &str) -> NodeFailure {
    NodeFailure {
        code: "invalid_native_diffusion_input".to_owned(),
        message: message.to_owned(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

fn diffusion_failure(error: impl std::fmt::Display) -> NodeFailure {
    let message = error.to_string();
    let cancelled = message.to_ascii_lowercase().contains("cancelled");
    NodeFailure {
        code: "native_diffusion_failed".to_owned(),
        message,
        kind: if cancelled {
            NodeFailureKind::Interrupted
        } else {
            NodeFailureKind::Failure
        },
        retryable: cancelled,
    }
}

fn tensor_sha256(tensor: &Tensor) -> Result<String, NativeImageRuntimeError> {
    Ok(format!("{:x}", Sha256::digest(tensor.contiguous_bytes()?)))
}

fn required_handle(
    inputs: &BTreeMap<String, Value>,
    name: &str,
) -> Result<NativeTensorHandle, NodeFailure> {
    let value = inputs.get(name).cloned().ok_or_else(|| NodeFailure {
        code: "missing_native_tensor_handle".to_owned(),
        message: format!("`{name}` is missing"),
        kind: NodeFailureKind::Failure,
        retryable: false,
    })?;
    serde_json::from_value(value).map_err(encoding_failure)
}

fn parse_resize_mode(value: &str) -> Result<ResizeMode, NodeFailure> {
    match value {
        "nearest-exact" => Ok(ResizeMode::NearestExact),
        "bilinear" => Ok(ResizeMode::Bilinear),
        "area" => Ok(ResizeMode::Area),
        "bicubic" => Ok(ResizeMode::Bicubic),
        "lanczos" => Ok(ResizeMode::Lanczos),
        _ => Err(NodeFailure {
            code: "unsupported_image_scale_method".to_owned(),
            message: format!("unsupported ImageScale method `{value}`"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
    }
}

fn parse_resize_crop(value: &str) -> Result<ResizeCrop, NodeFailure> {
    match value {
        "disabled" => Ok(ResizeCrop::Disabled),
        "center" => Ok(ResizeCrop::Center),
        _ => Err(NodeFailure {
            code: "unsupported_image_scale_crop".to_owned(),
            message: format!("unsupported ImageScale crop `{value}`"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
    }
}

fn png_metadata(inputs: &BTreeMap<String, Value>) -> Result<BTreeMap<String, String>, NodeFailure> {
    let mut metadata = BTreeMap::new();
    if let Some(prompt) = inputs.get("prompt")
        && !prompt.is_null()
    {
        metadata.insert(
            "prompt".to_owned(),
            serde_json::to_string(prompt).map_err(encoding_failure)?,
        );
    }
    if let Some(extra) = inputs.get("extra_pnginfo").and_then(Value::as_object) {
        for (key, value) in extra {
            metadata.insert(
                key.clone(),
                serde_json::to_string(value).map_err(encoding_failure)?,
            );
        }
    }
    Ok(metadata)
}

fn transaction_id(
    context: &NodeContext,
    namespace: AssetNamespace,
    batch_index: u32,
    metadata: &[u8],
) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(context.prompt_id.0.as_bytes());
    hasher.update(context.attempt_id.0.as_bytes());
    hasher.update(context.node_id.0.as_bytes());
    hasher.update([namespace as u8]);
    hasher.update(batch_index.to_le_bytes());
    hasher.update(metadata);
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

fn required_worker_input_ids(
    plan: &CompiledPlan,
) -> Result<BTreeSet<String>, NativeImageRuntimeError> {
    let mut input_ids = BTreeSet::new();
    for node in plan
        .nodes
        .values()
        .filter(|node| node.class_type == "LoadImage")
    {
        let Some(InputBinding::Literal { value }) = node.inputs.get("image") else {
            return Err(NativeImageRuntimeError::Encoding(format!(
                "LoadImage node {:?} has no literal image identity",
                node.id
            )));
        };
        let Some(logical_id) = value.as_str() else {
            return Err(NativeImageRuntimeError::Encoding(format!(
                "LoadImage node {:?} has a non-string image identity",
                node.id
            )));
        };
        if logical_id.is_empty() {
            return Err(NativeImageRuntimeError::Encoding(
                "LoadImage identity is empty".to_owned(),
            ));
        }
        input_ids.insert(logical_id.to_owned());
    }
    Ok(input_ids)
}

fn validate_worker_input_assets(
    input_assets: &BTreeMap<String, Vec<u8>>,
) -> Result<(), NativeImageRuntimeError> {
    let total_bytes = input_assets.values().try_fold(0_usize, |total, bytes| {
        total.checked_add(bytes.len()).ok_or_else(|| {
            NativeImageRuntimeError::Encoding(
                "native worker input byte count overflowed".to_owned(),
            )
        })
    })?;
    if total_bytes > MAX_NATIVE_WORKER_INPUT_BYTES {
        return Err(NativeImageRuntimeError::Encoding(format!(
            "native worker inputs contain {total_bytes} bytes, exceeding the {MAX_NATIVE_WORKER_INPUT_BYTES}-byte private-frame budget"
        )));
    }
    Ok(())
}

fn collect_worker_input_assets(
    plan: &CompiledPlan,
    shared_assets: &SharedAssetService,
    authorization: &AuthorizedCapabilities,
    cancellation: &CancellationToken,
) -> Result<BTreeMap<String, Vec<u8>>, NativeImageRuntimeError> {
    let mut assets = shared_assets.lock().map_err(|error| {
        NativeImageRuntimeError::Asset(format!("native asset service is unavailable: {error}"))
    })?;
    assets
        .scan_namespaces(&[AssetNamespace::Input], authorization, cancellation)
        .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
    let mut input_assets = BTreeMap::new();
    for logical_id in required_worker_input_ids(plan)? {
        let identity = assets
            .roots()
            .identity(AssetNamespace::Input, PathBuf::from(&logical_id))
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        let bytes = assets
            .read_verified(
                &identity,
                authorization,
                cancellation,
                MAX_NATIVE_IMAGE_INPUT_BYTES,
            )
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        input_assets.insert(logical_id, bytes);
    }
    validate_worker_input_assets(&input_assets)?;
    Ok(input_assets)
}

fn tensor_failure(error: TensorError) -> NodeFailure {
    NodeFailure {
        code: if error == TensorError::Cancelled {
            "native_image_cancelled"
        } else {
            "native_image_tensor_failed"
        }
        .to_owned(),
        message: error.to_string(),
        kind: if error == TensorError::Cancelled {
            NodeFailureKind::Interrupted
        } else {
            NodeFailureKind::Failure
        },
        retryable: error == TensorError::Cancelled,
    }
}

fn cancellation_failure(_: CancellationError) -> NodeFailure {
    tensor_failure(TensorError::Cancelled)
}

fn media_failure(error: PngError) -> NodeFailure {
    match error {
        PngError::Tensor(error) => tensor_failure(error),
        error => NodeFailure {
            code: "native_image_codec_failed".to_owned(),
            message: error.to_string(),
            kind: NodeFailureKind::Failure,
            retryable: false,
        },
    }
}

fn runtime_failure(error: NativeImageRuntimeError) -> NodeFailure {
    NodeFailure {
        code: "native_tensor_handle_failed".to_owned(),
        message: error.to_string(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

fn encoding_failure(error: impl std::fmt::Display) -> NodeFailure {
    NodeFailure {
        code: "native_image_encoding_failed".to_owned(),
        message: error.to_string(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

fn dimension_failure() -> NodeFailure {
    NodeFailure {
        code: "native_image_dimension_overflow".to_owned(),
        message: "image dimensions exceed the PNG contract".to_owned(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

#[derive(Clone)]
pub struct NativeExecutionControllerConfig {
    pub assets: SharedAssetService,
    pub presentation: SharedExecutionPresentationService,
    pub output_committer: SharedOutputCommitter,
    pub worker: WorkerLaunchConfig,
    pub memory_policy: MemoryPolicy,
    pub metadata_enabled: bool,
}

impl NativeExecutionControllerConfig {
    pub fn new(
        assets: SharedAssetService,
        presentation: SharedExecutionPresentationService,
        worker: WorkerLaunchConfig,
        metadata_enabled: bool,
    ) -> Result<Self, NativeImageRuntimeError> {
        let roots = assets
            .lock()
            .map_err(|error| {
                NativeImageRuntimeError::Asset(format!(
                    "native asset service is unavailable: {error}"
                ))
            })?
            .roots()
            .clone();
        let output_committer = Arc::new(std::sync::Mutex::new(
            OutputCommitter::open(roots)
                .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?,
        ));
        Ok(Self {
            assets,
            presentation,
            output_committer,
            worker,
            memory_policy: MemoryPolicy::default(),
            metadata_enabled,
        })
    }

    pub fn with_memory_policy(mut self, memory_policy: MemoryPolicy) -> Self {
        self.memory_policy = memory_policy;
        self
    }

    pub fn validate(&self) -> Result<(), NativeImageRuntimeError> {
        let assets = self.assets.lock().map_err(|error| {
            NativeImageRuntimeError::Asset(format!("native asset service is unavailable: {error}"))
        })?;
        let root_profile = Uuid::parse_str(&assets.roots().profile_id)
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        if self.worker.profile_id != ProfileId(root_profile) {
            return Err(NativeImageRuntimeError::Asset(
                "worker and asset-root profiles differ".to_owned(),
            ));
        }
        self.presentation
            .snapshot(self.worker.profile_id)
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        if self.worker.registry_version != NATIVE_IMAGE_REGISTRY_VERSION {
            return Err(NativeImageRuntimeError::Registry(format!(
                "worker registry version is `{}`, expected `{NATIVE_IMAGE_REGISTRY_VERSION}`",
                self.worker.registry_version
            )));
        }
        Ok(())
    }

    fn roots(&self) -> Result<AssetRoots, NativeImageRuntimeError> {
        self.assets
            .lock()
            .map(|assets| assets.roots().clone())
            .map_err(|error| {
                NativeImageRuntimeError::Asset(format!(
                    "native asset service is unavailable: {error}"
                ))
            })
    }
}

pub struct NativeExecutionController {
    profile_id: ProfileId,
    commands: async_channel::Sender<PreparedNativeCommand>,
    runner: Mutex<Option<thread::JoinHandle<()>>>,
}

struct PreparedNativeCommand {
    command: ExecutionControlCommand,
    activation: async_channel::Receiver<bool>,
}

struct NativePreparedExecutionActivation {
    activation: Option<async_channel::Sender<bool>>,
}

impl crate::PreparedExecutionActivation for NativePreparedExecutionActivation {
    fn commit(mut self: Box<Self>) {
        if let Some(activation) = self.activation.take()
            && let Err(error) = activation.try_send(true)
        {
            eprintln!("native prepared execution activation could not commit: {error}");
        }
    }
}

impl Drop for NativePreparedExecutionActivation {
    fn drop(&mut self) {
        if let Some(activation) = self.activation.take()
            && let Err(error) = activation.try_send(false)
        {
            eprintln!("native prepared execution activation could not abort: {error}");
        }
    }
}

impl NativeExecutionController {
    pub fn start(
        config: NativeExecutionControllerConfig,
        event_bus: ExecutionEventBus,
    ) -> Result<Arc<Self>, NativeImageRuntimeError> {
        config.validate()?;
        let supervisor = smol::block_on(RuntimeSupervisor::start(config.worker.clone()))
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        let (commands, receiver) = async_channel::bounded(NATIVE_CONTROLLER_CAPACITY);
        let profile_id = config.worker.profile_id;
        let runner = thread::Builder::new()
            .name("native-image-controller".to_owned())
            .spawn(move || {
                if let Err(error) = smol::block_on(run_native_controller(
                    config, event_bus, receiver, supervisor,
                )) {
                    eprintln!("native image execution controller stopped: {error}");
                }
            })
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        Ok(Arc::new(Self {
            profile_id,
            commands,
            runner: Mutex::new(Some(runner)),
        }))
    }

    fn shutdown_and_join(&self) -> Result<(), ExecutionFailure> {
        self.commands.close();
        let Some(runner) = self.runner.lock().take() else {
            return Ok(());
        };
        runner.join().map_err(|_| {
            ExecutionFailure::new(
                "native_controller_join_failed",
                "the native execution controller thread panicked during shutdown",
            )
            .with_origin(ExecutionFailureOrigin::Transport)
        })
    }
}

impl ExecutionController for NativeExecutionController {
    fn prepare<'a>(
        &'a self,
        command: &ExecutionControlCommand,
        _assigned_attempt_id: Option<AttemptId>,
    ) -> Result<Box<dyn crate::PreparedExecutionActivation + 'a>, ExecutionFailure> {
        if command.profile_id != self.profile_id {
            return Err(ExecutionFailure::new(
                "native_profile_unavailable",
                "the native image worker is scoped to a different profile",
            )
            .with_origin(ExecutionFailureOrigin::Transport));
        }
        let (activation, receiver) = async_channel::bounded(1);
        self.commands
            .try_send(PreparedNativeCommand {
                command: command.clone(),
                activation: receiver,
            })
            .map_err(|error| {
                ExecutionFailure::new(
                    "native_controller_backpressure",
                    format!("the native execution command queue is unavailable: {error}"),
                )
                .with_origin(ExecutionFailureOrigin::Transport)
                .retryable(true)
            })?;
        Ok(Box::new(NativePreparedExecutionActivation {
            activation: Some(activation),
        }))
    }

    fn accept(
        &self,
        command: &ExecutionControlCommand,
        assigned_attempt_id: Option<AttemptId>,
    ) -> Result<(), ExecutionFailure> {
        let activation = self.prepare(command, assigned_attempt_id)?;
        activation.commit();
        Ok(())
    }

    fn shutdown(&self) -> Result<(), ExecutionFailure> {
        self.shutdown_and_join()
    }
}

impl Drop for NativeExecutionController {
    fn drop(&mut self) {
        self.commands.close();
        if let Some(runner) = self.runner.get_mut().take()
            && runner.join().is_err()
        {
            eprintln!("native execution controller thread panicked during drop");
        }
    }
}

struct ActiveNativeExecution {
    profile_id: ProfileId,
    prompt_id: comfy_types::PromptId,
    attempt_id: AttemptId,
    cancellation: CancellationToken,
    output_proposals: BTreeMap<Uuid, NativeImageOutputProposal>,
    output_proposal_bytes: usize,
}

struct NativeControllerState {
    config: NativeExecutionControllerConfig,
    event_bus: ExecutionEventBus,
    supervisor: Option<RuntimeSupervisor>,
    active: Option<ActiveNativeExecution>,
    output_committer: SharedOutputCommitter,
    input_authorization: AuthorizedCapabilities,
    output_authorization: AuthorizedCapabilities,
}

enum ControllerInput {
    Command(Result<PreparedNativeCommand, async_channel::RecvError>),
    Worker(Result<comfy_types::WorkerEnvelope, RuntimeSupervisorError>),
}

async fn run_native_controller(
    config: NativeExecutionControllerConfig,
    event_bus: ExecutionEventBus,
    commands: async_channel::Receiver<PreparedNativeCommand>,
    supervisor: RuntimeSupervisor,
) -> Result<(), NativeImageRuntimeError> {
    let roots = config.roots()?;
    let output_committer = config.output_committer.clone();
    let input_authorization = authorize_native_input_reader(&roots.profile_id)
        .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
    let output_authorization = authorize_native_output_committer(&roots.profile_id)
        .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
    let mut state = NativeControllerState {
        config,
        event_bus,
        supervisor: Some(supervisor),
        active: None,
        output_committer,
        input_authorization,
        output_authorization,
    };
    state.reconcile_committed_output_receipts().await?;
    loop {
        state.start_next().await?;
        let input = if state.active.is_some() {
            let supervisor = state.supervisor.as_ref().ok_or_else(|| {
                NativeImageRuntimeError::Execution(
                    "active execution has no worker supervisor".to_owned(),
                )
            })?;
            smol::future::race(
                async { ControllerInput::Command(commands.recv().await) },
                async {
                    ControllerInput::Worker(supervisor.next_event(NATIVE_WORKER_EVENT_POLL).await)
                },
            )
            .await
        } else {
            ControllerInput::Command(commands.recv().await)
        };
        match input {
            ControllerInput::Command(Ok(command)) => {
                if command.activation.recv().await != Ok(true) {
                    continue;
                }
                if let Err(error) = state.apply_command(command.command).await {
                    eprintln!("native image command could not be applied: {error}");
                }
            }
            ControllerInput::Command(Err(_)) => {
                state.shutdown_for_restart().await?;
                return Ok(());
            }
            ControllerInput::Worker(Ok(envelope)) => {
                let mut command_channel_closed = false;
                loop {
                    match commands.try_recv() {
                        Ok(command) => {
                            if command.activation.recv().await != Ok(true) {
                                continue;
                            }
                            if let Err(error) = state.apply_command(command.command).await {
                                eprintln!("native image command could not be applied: {error}");
                            }
                        }
                        Err(async_channel::TryRecvError::Empty) => break,
                        Err(async_channel::TryRecvError::Closed) => {
                            command_channel_closed = true;
                            break;
                        }
                    }
                }
                if command_channel_closed {
                    state.shutdown_for_restart().await?;
                    return Ok(());
                }
                match state.apply_worker_event(envelope).await {
                    Ok(()) => {}
                    Err(NativeImageRuntimeError::WorkerEvent(message)) => {
                        state
                            .recover_worker(RuntimeSupervisorError::Protocol(message))
                            .await?;
                    }
                    Err(error) => return Err(error),
                }
            }
            ControllerInput::Worker(Err(RuntimeSupervisorError::Timeout { .. })) => {}
            ControllerInput::Worker(Err(error)) => state.recover_worker(error).await?,
        }
    }
}

impl NativeControllerState {
    async fn reconcile_committed_output_receipts(&mut self) -> Result<(), NativeImageRuntimeError> {
        let scopes = self
            .output_committer
            .lock()
            .map_err(|error| {
                NativeImageRuntimeError::Asset(format!(
                    "native output committer is unavailable: {error}"
                ))
            })?
            .committed_execution_scopes();
        for scope in scopes {
            let receipts = self
                .output_committer
                .lock()
                .map_err(|error| {
                    NativeImageRuntimeError::Asset(format!(
                        "native output committer is unavailable: {error}"
                    ))
                })?
                .committed_receipts_for_scope(&scope)
                .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
            let snapshot = self
                .config
                .presentation
                .snapshot(scope.profile_id)
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
            let Some(attempt) = snapshot
                .attempts
                .iter()
                .find(|attempt| attempt.attempt_id == scope.attempt_id)
            else {
                continue;
            };
            let missing = receipts
                .into_iter()
                .filter(|receipt| {
                    !attempt
                        .outputs
                        .iter()
                        .any(|output| output.output_id == receipt.proposal_id())
                })
                .collect::<Vec<_>>();
            if missing.is_empty() && attempt.state == AttemptState::Succeeded {
                continue;
            }
            if attempt.state.is_terminal() {
                continue;
            }
            let at = Utc::now();
            let mut inputs = recovered_committed_event_inputs(&missing, at)?;
            inputs.push(ExecutionActuatorEventInput {
                node_id: None,
                kind: AttemptEventKind::Succeeded,
                data: Some(json!({"recovered_output_receipts": missing.len()})),
                at,
            });
            let events = self
                .config
                .presentation
                .apply_actuator_event_batch_durable(
                    scope.profile_id,
                    scope.prompt_id,
                    scope.attempt_id,
                    &inputs,
                )
                .await
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
            self.publish_projection_events(events);
        }
        Ok(())
    }

    async fn shutdown_for_restart(&mut self) -> Result<(), NativeImageRuntimeError> {
        if let Some(active) = self.active.take() {
            active.cancellation.cancel();
            self.publish_kind(
                active.profile_id,
                active.prompt_id,
                active.attempt_id,
                None,
                AttemptEventKind::RecoveryInterrupted {
                    reason: crate::ExecutionRecoveryInterruptionReason::RuntimeRestart,
                },
                Some(json!({"reason": "native controller stopped"})),
            )?;
        }
        if let Some(mut supervisor) = self.supervisor.take()
            && let Err(error) = supervisor.shutdown().await
        {
            eprintln!("native image worker shutdown failed: {error}");
        }
        Ok(())
    }

    async fn apply_command(
        &mut self,
        command: ExecutionControlCommand,
    ) -> Result<(), NativeImageRuntimeError> {
        match command.kind {
            ExecutionControlCommandKind::Cancel { attempt_id, reason } => {
                self.publish_termination_request(command.profile_id, attempt_id)?;
                self.terminate(attempt_id, reason).await?;
            }
            ExecutionControlCommandKind::Interrupt { attempt_id, reason } => {
                self.publish_termination_request(command.profile_id, attempt_id)?;
                self.terminate(attempt_id, reason).await?;
            }
            ExecutionControlCommandKind::Queue { .. }
            | ExecutionControlCommandKind::Retry { .. }
            | ExecutionControlCommandKind::Reorder { .. }
            | ExecutionControlCommandKind::ClearPending { .. }
            | ExecutionControlCommandKind::ClearHistory
            | ExecutionControlCommandKind::RemoveHistory { .. } => {}
        }
        Ok(())
    }

    fn publish_termination_request(
        &self,
        profile_id: ProfileId,
        attempt_id: AttemptId,
    ) -> Result<(), NativeImageRuntimeError> {
        let event = self
            .config
            .presentation
            .latest_termination_request_event(profile_id, attempt_id)
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        if let Some(event) = event {
            self.publish_projection_events(vec![event]);
        }
        Ok(())
    }

    async fn terminate(
        &mut self,
        attempt_id: AttemptId,
        reason: String,
    ) -> Result<(), NativeImageRuntimeError> {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.attempt_id == attempt_id)
        {
            let active = self.active.as_mut().ok_or_else(|| {
                NativeImageRuntimeError::Execution(
                    "active execution disappeared during termination".to_owned(),
                )
            })?;
            active.cancellation.cancel();
            let prompt_id = active.prompt_id;
            let cancel_result = self
                .supervisor
                .as_mut()
                .ok_or_else(|| {
                    NativeImageRuntimeError::Execution(
                        "active termination has no worker".to_owned(),
                    )
                })?
                .cancel(prompt_id, attempt_id, reason)
                .await;
            if let Err(error) = cancel_result {
                let active = self.active.take().ok_or_else(|| {
                    NativeImageRuntimeError::Execution(
                        "active execution disappeared after cancellation failure".to_owned(),
                    )
                })?;
                let kind = {
                    canonical_termination_kind(
                        self.config
                            .presentation
                            .termination_intent(active.profile_id, active.attempt_id)
                            .map_err(|error| {
                                NativeImageRuntimeError::Execution(error.to_string())
                            })?,
                        &active,
                    )
                    .unwrap_or_else(|| AttemptEventKind::Failed {
                        failure: ExecutionFailure::new(
                            "native_worker_cancel_without_canonical_intent",
                            error.to_string(),
                        )
                        .with_origin(ExecutionFailureOrigin::Transport)
                        .retryable(true),
                    })
                };
                self.publish_kind(
                    active.profile_id,
                    active.prompt_id,
                    active.attempt_id,
                    None,
                    kind,
                    Some(json!({"worker_cancel_error": error.to_string()})),
                )?;
                if let Some(supervisor) = self.supervisor.take() {
                    match supervisor.recover().await {
                        Ok(supervisor) => self.supervisor = Some(supervisor),
                        Err(recovery_error) => eprintln!(
                            "native image worker recovery failed after cancellation error {error}: {recovery_error}"
                        ),
                    }
                }
            }
        }
        Ok(())
    }

    async fn start_next(&mut self) -> Result<(), NativeImageRuntimeError> {
        if self.active.is_some() {
            return Ok(());
        }
        let lease = self
            .config
            .presentation
            .next_queued_attempt(self.config.worker.profile_id)
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        let Some(lease) = lease else {
            return Ok(());
        };
        self.active = Some(ActiveNativeExecution {
            profile_id: lease.profile_id,
            prompt_id: lease.prompt_id,
            attempt_id: lease.attempt_id,
            cancellation: lease.cancellation.clone(),
            output_proposals: BTreeMap::new(),
            output_proposal_bytes: 0,
        });
        if self.supervisor.is_none() {
            match RuntimeSupervisor::start(self.config.worker.clone()).await {
                Ok(supervisor) => self.supervisor = Some(supervisor),
                Err(error) => {
                    self.active = None;
                    self.publish_kind(
                        lease.profile_id,
                        lease.prompt_id,
                        lease.attempt_id,
                        None,
                        AttemptEventKind::Started,
                        None,
                    )?;
                    self.publish_terminal_failure(
                        lease.profile_id,
                        lease.prompt_id,
                        lease.attempt_id,
                        "native_worker_start_failed",
                        error.to_string(),
                    )?;
                    return Ok(());
                }
            }
        }
        let accepted_backend = self
            .supervisor
            .as_ref()
            .and_then(RuntimeSupervisor::accepted_backend)
            .ok_or_else(|| {
                NativeImageRuntimeError::WorkerEvent(
                    "ready native worker omitted its accepted backend matrix".to_owned(),
                )
            })?;
        let effective_backend = project_effective_native_backend(
            &accepted_backend,
            self.config.worker.memory_limit_bytes,
            self.config.memory_policy,
        );
        self.publish_kind(
            lease.profile_id,
            lease.prompt_id,
            lease.attempt_id,
            None,
            AttemptEventKind::Started,
            Some(json!({"effective_native_backend": effective_backend})),
        )?;
        let input_assets = collect_worker_input_assets(
            &lease.plan,
            &self.config.assets,
            &self.input_authorization,
            &lease.cancellation,
        )?;
        let worker_plan = NativeImageWorkerPlan::new_with_memory_policy(
            lease.plan.clone(),
            input_assets,
            self.config.memory_policy,
            self.config.metadata_enabled,
            lease
                .plan
                .extra_data
                .get("sim_native_delay_millis")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        )?;
        let encoded = serde_json::to_vec(&worker_plan)
            .map_err(|error| NativeImageRuntimeError::Encoding(error.to_string()))?;
        let supervisor = self.supervisor.as_mut().ok_or_else(|| {
            NativeImageRuntimeError::Execution(
                "native worker disappeared before execute".to_owned(),
            )
        })?;
        if let Err(error) = supervisor
            .execute(lease.prompt_id, lease.attempt_id, encoded)
            .await
        {
            self.active = None;
            self.publish_terminal_failure(
                lease.profile_id,
                lease.prompt_id,
                lease.attempt_id,
                "native_worker_execute_failed",
                error.to_string(),
            )?;
            return Ok(());
        }
        Ok(())
    }
}

fn decode_native_image_worker_event(
    encoded: &[u8],
) -> Result<NativeImageWorkerEvent, NativeImageRuntimeError> {
    postcard::from_bytes(encoded).map_err(|error| {
        NativeImageRuntimeError::WorkerEvent(format!(
            "event payload could not be decoded as the native image schema: {error}"
        ))
    })
}

#[cfg(test)]
pub(crate) fn val_domain_004_worker_ui_wire_adapter_case()
-> Result<bool, Box<dyn std::error::Error>> {
    let node_id = NodeId("ui".to_owned());
    let expected = BTreeMap::from([(node_id, json!({"images": [{"filename": "fixture.png"}]}))]);
    let result = NativeImageWorkerResult::from_execution_report(
        ExecutionReport {
            profile_id: ProfileId(Uuid::from_u128(1)),
            prompt_id: comfy_types::PromptId(Uuid::from_u128(2)),
            attempt_id: AttemptId(Uuid::from_u128(3)),
            state: AttemptState::Succeeded,
            outputs: BTreeMap::new(),
            ui_outputs: expected.clone(),
            events: Vec::new(),
            cache_hits: 0,
            error: None,
        },
        Vec::new(),
        1,
    )?;
    if !result.report.ui_outputs.is_empty() {
        return Ok(false);
    }
    let event = NativeImageWorkerEvent::Completed { result };
    let encoded = postcard::to_stdvec(&event)?;
    let decoded: NativeImageWorkerEvent = postcard::from_bytes(&encoded)?;
    let NativeImageWorkerEvent::Completed { result } = decoded else {
        return Ok(false);
    };
    Ok(result.decode_ui_outputs()? == expected)
}

impl NativeControllerState {
    async fn apply_worker_event(
        &mut self,
        envelope: comfy_types::WorkerEnvelope,
    ) -> Result<(), NativeImageRuntimeError> {
        let event = match envelope.message {
            comfy_types::WorkerMessage::OutputProposal { proposal } => {
                let Some(active) = self.active.as_mut() else {
                    return Ok(());
                };
                if envelope.prompt_id != Some(active.prompt_id)
                    || envelope.attempt_id != Some(active.attempt_id)
                {
                    return Err(NativeImageRuntimeError::WorkerEvent(
                        "worker output proposal has stale attempt identity".to_owned(),
                    ));
                }
                let proposal = NativeImageOutputProposal::from_worker_proposal(proposal)?;
                let proposal_id = proposal.proposal_id();
                let output_bytes = proposal.output.content().len();
                let next_bytes = active
                    .output_proposal_bytes
                    .checked_add(output_bytes)
                    .ok_or_else(|| {
                        NativeImageRuntimeError::WorkerEvent(
                            "worker output proposal byte count overflowed".to_owned(),
                        )
                    })?;
                if active.output_proposals.len() >= MAX_NATIVE_OUTPUT_PROPOSALS
                    || next_bytes > MAX_NATIVE_ATTEMPT_PROPOSAL_BYTES
                {
                    return Err(NativeImageRuntimeError::WorkerEvent(
                        "worker output proposal batch exceeds its count or byte bound".to_owned(),
                    ));
                }
                if active
                    .output_proposals
                    .insert(proposal_id, proposal)
                    .is_some()
                {
                    return Err(NativeImageRuntimeError::WorkerEvent(format!(
                        "worker repeated output proposal {proposal_id}"
                    )));
                }
                active.output_proposal_bytes = next_bytes;
                return Ok(());
            }
            comfy_types::WorkerMessage::Event { event } => event,
            _ => return Ok(()),
        };
        match decode_native_image_worker_event(&event)? {
            NativeImageWorkerEvent::Progress { progress } => {
                self.publish_worker_progress(progress)?;
            }
            NativeImageWorkerEvent::Completed { result } => {
                let ui_outputs = result.decode_ui_outputs()?;
                let Some(active) = self.active.as_ref() else {
                    return Ok(());
                };
                if result.report.profile_id != active.profile_id
                    || result.report.prompt_id != active.prompt_id
                    || result.report.attempt_id != active.attempt_id
                {
                    return Ok(());
                }
                let mut active = self.active.take().ok_or_else(|| {
                    NativeImageRuntimeError::Execution(
                        "active execution disappeared while applying completion".to_owned(),
                    )
                })?;
                let expected_ids = result
                    .output_proposal_ids
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>();
                let actual_ids = active
                    .output_proposals
                    .keys()
                    .copied()
                    .collect::<BTreeSet<_>>();
                if expected_ids.len() != result.output_proposal_ids.len()
                    || expected_ids != actual_ids
                {
                    return Err(NativeImageRuntimeError::WorkerEvent(
                        "worker completion does not exactly reference its output proposals"
                            .to_owned(),
                    ));
                }
                let proposals = result
                    .output_proposal_ids
                    .iter()
                    .map(|proposal_id| {
                        active.output_proposals.remove(proposal_id).ok_or_else(|| {
                            NativeImageRuntimeError::WorkerEvent(format!(
                                "worker output proposal {proposal_id} disappeared"
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if result.report.state != AttemptState::Succeeded && !proposals.is_empty() {
                    return Err(NativeImageRuntimeError::WorkerEvent(
                        "failed worker completion referenced output proposals".to_owned(),
                    ));
                }
                let completed_at = Utc::now();
                let transaction = self
                    .config
                    .presentation
                    .apply_actuator_event_transaction_durable(
                        active.profile_id,
                        active.prompt_id,
                        active.attempt_id,
                        |termination_intent, validator| {
                            let mut event_inputs = Vec::new();
                            let canonical_termination =
                                canonical_termination_kind(termination_intent, &active);
                            let terminal_kind =
                                canonical_termination.clone().unwrap_or_else(|| {
                                    terminal_event(result.report.state, result.report.error)
                                });
                            if result.report.state == AttemptState::Succeeded
                                && canonical_termination.is_none()
                            {
                                let outputs = proposals
                                    .iter()
                                    .map(|proposal| proposal.output.clone())
                                    .collect::<Vec<_>>();
                                let mut assets = self.config.assets.lock().map_err(|error| {
                                    NativeImageRuntimeError::Asset(format!(
                                        "native asset service is unavailable: {error}"
                                    ))
                                })?;
                                let committed = self
                                    .output_committer
                                    .lock()
                                    .map_err(|error| {
                                        NativeImageRuntimeError::Asset(format!(
                                            "native output committer is unavailable: {error}"
                                        ))
                                    })?
                                    .commit_scoped_proposal_batch_and_register_with_precommit(
                                        &OutputExecutionScope {
                                            profile_id: active.profile_id,
                                            prompt_id: active.prompt_id,
                                            attempt_id: active.attempt_id,
                                        },
                                        &outputs,
                                        Local::now().fixed_offset(),
                                        &mut assets,
                                        &self.output_authorization,
                                        &active.cancellation,
                                        |prepared| {
                                            event_inputs = committed_event_inputs(
                                                &proposals,
                                                prepared,
                                                completed_at,
                                            )
                                            .map_err(|error| {
                                                OutputCommitError::PrecommitValidation(
                                                    error.to_string(),
                                                )
                                            })?;
                                            event_inputs.push(ExecutionActuatorEventInput {
                                                node_id: None,
                                                kind: terminal_kind.clone(),
                                                data: (!ui_outputs.is_empty())
                                                    .then(|| json!({"ui_outputs": ui_outputs})),
                                                at: completed_at,
                                            });
                                            validator.validate(&event_inputs).map_err(|error| {
                                                OutputCommitError::PrecommitValidation(
                                                    error.to_string(),
                                                )
                                            })
                                        },
                                    )
                                    .map_err(|error| {
                                        NativeImageRuntimeError::Asset(error.to_string())
                                    })?;
                                if committed.len() != proposals.len() {
                                    return Err(NativeImageRuntimeError::Execution(
                                        "native output commit omitted a prepared proposal"
                                            .to_owned(),
                                    ));
                                }
                            } else {
                                event_inputs.push(ExecutionActuatorEventInput {
                                    node_id: None,
                                    kind: terminal_kind,
                                    data: (!ui_outputs.is_empty())
                                        .then(|| json!({"ui_outputs": ui_outputs})),
                                    at: completed_at,
                                });
                                validator.validate(&event_inputs).map_err(|error| {
                                    NativeImageRuntimeError::Execution(error.to_string())
                                })?;
                            }
                            Ok((event_inputs, ()))
                        },
                    )
                    .await;
                let (applied, ()) = match transaction {
                    Ok(transaction) => transaction,
                    Err(crate::ExecutionActuatorTransactionError::Presentation(error)) => {
                        return Err(NativeImageRuntimeError::Execution(error.to_string()));
                    }
                    Err(crate::ExecutionActuatorTransactionError::Operation(error)) => {
                        return Err(error);
                    }
                };
                self.publish_projection_events(applied);
            }
            NativeImageWorkerEvent::BackendUnavailable { unavailable } => {
                let Some(active) = self.active.take() else {
                    return Ok(());
                };
                self.publish_kind(
                    active.profile_id,
                    active.prompt_id,
                    active.attempt_id,
                    None,
                    AttemptEventKind::Failed {
                        failure: backend_unavailable_failure(&unavailable),
                    },
                    Some(json!({
                        "device": format!("{:?}", unavailable.device()).to_ascii_lowercase(),
                    })),
                )?;
            }
            NativeImageWorkerEvent::Failed { message, cancelled } => {
                let Some(active) = self.active.take() else {
                    return Ok(());
                };
                let canonical_termination = canonical_termination_kind(
                    self.config
                        .presentation
                        .termination_intent(active.profile_id, active.attempt_id)
                        .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
                    &active,
                );
                let worker_observation_was_unowned = cancelled && canonical_termination.is_none();
                let kind = canonical_termination.unwrap_or_else(|| AttemptEventKind::Failed {
                    failure: ExecutionFailure::new("native_worker_failed", message)
                        .with_origin(ExecutionFailureOrigin::Transport)
                        .retryable(true),
                });
                self.publish_kind(
                    active.profile_id,
                    active.prompt_id,
                    active.attempt_id,
                    None,
                    kind,
                    worker_observation_was_unowned
                        .then(|| json!({"worker_reported_cancelled": true})),
                )?;
            }
        }
        Ok(())
    }

    fn publish_worker_progress(
        &mut self,
        progress: NativeImageWorkerProgress,
    ) -> Result<(), NativeImageRuntimeError> {
        let (profile_id, prompt_id, attempt_id) = {
            let Some(active) = self.active.as_ref() else {
                return Ok(());
            };
            if progress.profile_id != active.profile_id
                || progress.prompt_id != active.prompt_id
                || progress.attempt_id != active.attempt_id
            {
                return Ok(());
            }
            (active.profile_id, active.prompt_id, active.attempt_id)
        };
        let kind = match progress.kind {
            NativeImageWorkerProgressKind::Started => AttemptEventKind::Started,
            NativeImageWorkerProgressKind::Progress { completed, total } => {
                AttemptEventKind::Progress { completed, total }
            }
            NativeImageWorkerProgressKind::CacheHit => AttemptEventKind::CacheHit,
            NativeImageWorkerProgressKind::OutputPrepared { transaction_id } => {
                AttemptEventKind::OutputPrepared { transaction_id }
            }
        };
        if matches!(kind, AttemptEventKind::Started) {
            return Ok(());
        }
        self.publish_kind(
            profile_id,
            prompt_id,
            attempt_id,
            progress.node_id,
            kind,
            None,
        )
    }

    async fn recover_worker(
        &mut self,
        error: RuntimeSupervisorError,
    ) -> Result<(), NativeImageRuntimeError> {
        let worker_error = error.to_string();
        if let Some(active) = self.active.take() {
            self.publish_kind(
                active.profile_id,
                active.prompt_id,
                active.attempt_id,
                None,
                AttemptEventKind::RecoveryInterrupted {
                    reason: crate::ExecutionRecoveryInterruptionReason::RuntimeRestart,
                },
                Some(json!({"worker_error": worker_error})),
            )?;
        }
        if let Some(supervisor) = self.supervisor.take() {
            match supervisor.recover().await {
                Ok(supervisor) => self.supervisor = Some(supervisor),
                Err(recovery_error) => {
                    eprintln!(
                        "native image worker recovery failed after {error}: {recovery_error}"
                    );
                }
            }
        }
        Ok(())
    }

    fn publish_terminal_failure(
        &self,
        profile_id: ProfileId,
        prompt_id: comfy_types::PromptId,
        attempt_id: AttemptId,
        code: &str,
        message: String,
    ) -> Result<(), NativeImageRuntimeError> {
        self.publish_kind(
            profile_id,
            prompt_id,
            attempt_id,
            None,
            AttemptEventKind::Failed {
                failure: ExecutionFailure::new(code, message)
                    .with_origin(ExecutionFailureOrigin::Transport)
                    .retryable(true),
            },
            None,
        )
    }

    fn publish_kind(
        &self,
        profile_id: ProfileId,
        prompt_id: comfy_types::PromptId,
        attempt_id: AttemptId,
        node_id: Option<NodeId>,
        kind: AttemptEventKind,
        data: Option<Value>,
    ) -> Result<(), NativeImageRuntimeError> {
        let event = smol::block_on(self.config.presentation.apply_actuator_event_batch_durable(
            profile_id,
            prompt_id,
            attempt_id,
            &[ExecutionActuatorEventInput {
                node_id,
                kind,
                data,
                at: Utc::now(),
            }],
        ))
        .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        self.publish_projection_events(event);
        Ok(())
    }

    fn publish_projection_events(&self, events: Vec<crate::AttemptEvent>) {
        for event in events {
            if let Err(error) = self.event_bus.publish(event) {
                eprintln!("native execution projection event was dropped: {error}");
            }
        }
    }
}

fn backend_unavailable_failure(unavailable: &BackendUnavailable) -> ExecutionFailure {
    ExecutionFailure::new("native_backend_unavailable", unavailable.to_string())
        .with_origin(ExecutionFailureOrigin::Validation)
}

fn canonical_termination_kind(
    intent: Option<crate::ExecutionTerminationIntent>,
    active: &ActiveNativeExecution,
) -> Option<AttemptEventKind> {
    match intent {
        Some(crate::ExecutionTerminationIntent::Cancel) => Some(AttemptEventKind::Cancelled),
        Some(crate::ExecutionTerminationIntent::Interrupt { reason }) => {
            Some(AttemptEventKind::Interrupted { reason })
        }
        None if active.cancellation.is_cancelled() => Some(AttemptEventKind::Cancelled),
        None => None,
    }
}

fn committed_event_inputs(
    proposals: &[NativeImageOutputProposal],
    prepared: &[PreparedOutput],
    at: chrono::DateTime<Utc>,
) -> Result<Vec<ExecutionActuatorEventInput>, NativeImageRuntimeError> {
    if proposals.len() != prepared.len() {
        return Err(NativeImageRuntimeError::Execution(
            "prepared output count does not match its native proposals".to_owned(),
        ));
    }
    let mut inputs = Vec::with_capacity(proposals.len().saturating_mul(2));
    for (proposal, prepared) in proposals.iter().zip(prepared) {
        let reference = prepared
            .identity
            .to_reference()
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        if proposal.output.namespace() == AssetNamespace::Temporary {
            inputs.push(ExecutionActuatorEventInput {
                node_id: Some(proposal.node_id.clone()),
                kind: AttemptEventKind::Preview {
                    preview: ExecutionPreview {
                        preview_id: proposal.proposal_id(),
                        node_id: proposal.node_id.clone(),
                        revision: 1,
                        frame_index: Some(u64::from(proposal.output.batch_index())),
                        output_index: Some(0),
                        media_kind: OutputMediaKind::Image,
                        media_type: "image/png".to_owned(),
                        width: Some(proposal.output.width()),
                        height: Some(proposal.output.height()),
                        encoded_bytes: proposal.output.content().to_vec(),
                    },
                },
                data: None,
                at,
            });
        }
        inputs.push(ExecutionActuatorEventInput {
            node_id: Some(proposal.node_id.clone()),
            kind: AttemptEventKind::OutputAvailable {
                output: ExecutionOutput {
                    output_id: proposal.proposal_id(),
                    node_id: proposal.node_id.clone(),
                    output_index: usize::try_from(proposal.output.batch_index()).map_err(|_| {
                        NativeImageRuntimeError::Execution(
                            "output batch index does not fit this platform".to_owned(),
                        )
                    })?,
                    name: prepared
                        .identity
                        .filename()
                        .unwrap_or("native-image.png")
                        .to_owned(),
                    media_kind: OutputMediaKind::Image,
                    media_type: "image/png".to_owned(),
                    subfolder: Some(prepared.identity.subfolder().to_string_lossy().into_owned()),
                    storage_type: Some(prepared.identity.namespace.locator_type().to_owned()),
                    metadata: BTreeMap::from([("sha256".to_owned(), json!(prepared.sha256))]),
                    view_reference: Some(reference.clone()),
                    download_reference: Some(reference.clone()),
                    availability: ExecutionOutputAvailability::Ready {
                        reference,
                        byte_length: prepared.byte_size,
                    },
                    created_at: at,
                },
            },
            data: None,
            at,
        });
    }
    Ok(inputs)
}

fn recovered_committed_event_inputs(
    receipts: &[OutputCommitReceipt],
    at: chrono::DateTime<Utc>,
) -> Result<Vec<ExecutionActuatorEventInput>, NativeImageRuntimeError> {
    receipts
        .iter()
        .map(|receipt| {
            let operation = receipt.operation();
            let metadata: NativeImageOutputProposalMetadata =
                postcard::from_bytes(&operation.projection_metadata).map_err(|error| {
                    NativeImageRuntimeError::Execution(format!(
                        "committed output projection metadata is invalid: {error}"
                    ))
                })?;
            if metadata.schema_version != NATIVE_IMAGE_OUTPUT_PROPOSAL_SCHEMA_VERSION
                || metadata.namespace != operation.identity.namespace
            {
                return Err(NativeImageRuntimeError::Execution(
                    "committed output projection metadata does not match its journal operation"
                        .to_owned(),
                ));
            }
            let reference = operation
                .identity
                .to_reference()
                .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
            Ok(ExecutionActuatorEventInput {
                node_id: Some(metadata.node_id.clone()),
                kind: AttemptEventKind::OutputAvailable {
                    output: ExecutionOutput {
                        output_id: receipt.proposal_id(),
                        node_id: metadata.node_id,
                        output_index: usize::try_from(metadata.batch_index).map_err(|_| {
                            NativeImageRuntimeError::Execution(
                                "output batch index does not fit this platform".to_owned(),
                            )
                        })?,
                        name: operation
                            .identity
                            .filename()
                            .unwrap_or("native-image.png")
                            .to_owned(),
                        media_kind: OutputMediaKind::Image,
                        media_type: "image/png".to_owned(),
                        subfolder: Some(
                            operation
                                .identity
                                .subfolder()
                                .to_string_lossy()
                                .into_owned(),
                        ),
                        storage_type: Some(operation.identity.namespace.locator_type().to_owned()),
                        metadata: BTreeMap::from([
                            ("sha256".to_owned(), json!(operation.sha256)),
                            ("width".to_owned(), json!(metadata.width)),
                            ("height".to_owned(), json!(metadata.height)),
                        ]),
                        view_reference: Some(reference.clone()),
                        download_reference: Some(reference.clone()),
                        availability: ExecutionOutputAvailability::Ready {
                            reference,
                            byte_length: operation.byte_size,
                        },
                        created_at: at,
                    },
                },
                data: Some(json!({"recovered_output_receipt": true})),
                at,
            })
        })
        .collect()
}

fn terminal_event(state: AttemptState, error: Option<String>) -> AttemptEventKind {
    match state {
        AttemptState::Succeeded => AttemptEventKind::Succeeded,
        AttemptState::Cancelled => AttemptEventKind::Cancelled,
        AttemptState::Interrupted => AttemptEventKind::Interrupted {
            reason: error.unwrap_or_else(|| "native image execution was interrupted".to_owned()),
        },
        AttemptState::Failed
        | AttemptState::Queued
        | AttemptState::Running
        | AttemptState::Cancelling => AttemptEventKind::Failed {
            failure: ExecutionFailure::new(
                "native_image_execution_failed",
                error.unwrap_or_else(|| format!("native image execution ended in {state:?}")),
            )
            .with_origin(ExecutionFailureOrigin::Node)
            .retryable(true),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AttemptState, CacheKey, ExecutionAttemptPersistence, ExecutionCommandAck,
        ExecutionCommandOutcome, ExecutionDataSource, ExecutionPresentationService,
        ExecutionSnapshotStatus, PersistedExecutionAttempt, PersistedExecutionProfile,
        PromptCompiler,
    };
    use comfy_types::{ApiPrompt, PromptId, PromptNode, PromptSubmission, RequestId, WorkerId};
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::TempDir;

    struct FailOnceExecutionPersistence {
        database: crate::ComfyRuntimeDb,
        fail_next: Arc<AtomicBool>,
    }

    struct WorkspaceProbeDiffusionProvider {
        observed: Arc<AtomicBool>,
    }

    #[test]
    fn native_output_worker_adapter_round_trips_the_canonical_proposal()
    -> Result<(), Box<dyn std::error::Error>> {
        let canonical = OutputProposal::new(
            Uuid::from_u128(0xabba),
            AssetNamespace::Output,
            "adapter/round-trip",
            "png",
            3,
            17,
            19,
            b"native image bytes".to_vec(),
        )?;
        let native = NativeImageOutputProposal::new(NodeId("save".to_owned()), canonical)?;
        let recovered =
            NativeImageOutputProposal::from_worker_proposal(native.to_worker_proposal()?)?;
        assert_eq!(recovered, native);
        assert_eq!(recovered.output().proposal_id(), Uuid::from_u128(0xabba));

        let invalid_metadata = NativeImageOutputProposalMetadata {
            schema_version: NATIVE_IMAGE_OUTPUT_PROPOSAL_SCHEMA_VERSION + 1,
            node_id: NodeId("save".to_owned()),
            batch_index: 0,
            namespace: AssetNamespace::Output,
            filename_prefix: "adapter/rejected".to_owned(),
            extension: "png".to_owned(),
            width: 1,
            height: 1,
        };
        let invalid = WorkerOutputProposal::new(
            Uuid::from_u128(0xabb9),
            postcard::to_stdvec(&invalid_metadata)?,
            vec![0],
        )?;
        assert!(matches!(
            NativeImageOutputProposal::from_worker_proposal(invalid),
            Err(NativeImageRuntimeError::WorkerEvent(message))
                if message.contains("unsupported native image output proposal schema")
        ));
        Ok(())
    }

    #[test]
    fn ksampler_cache_identity_binds_guidance_adapter_and_inputs()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, _workspace_authority) = CpuWorkspaceAuthority::create_backend(1 << 20)?;
        let node = KSamplerNode {
            state: Arc::new(NativeDiffusionState {
                provider: Arc::new(WorkspaceProbeDiffusionProvider {
                    observed: Arc::new(AtomicBool::new(false)),
                }),
                backend: Arc::new(backend),
                loaded: Mutex::new(None),
            }),
            tensors: Arc::new(Mutex::new(NativeTensorStore::default())),
        };
        let inputs = BTreeMap::from([
            ("model".to_owned(), json!("model-digest-a")),
            ("positive".to_owned(), json!("conditioning-positive")),
            ("negative".to_owned(), json!("conditioning-negative")),
            ("cfg".to_owned(), json!(7.0)),
        ]);
        let change_token = node.cache_change_token(&inputs)?;
        assert_eq!(change_token, GUIDANCE_ADAPTER_ID);
        let key = |inputs: &BTreeMap<String, Value>, token: &str| {
            CacheKey::from_inputs(
                "KSampler",
                NATIVE_DIFFUSION_REGISTRY_VERSION,
                inputs,
                BTreeMap::new(),
                "cpu",
                "f32",
                None,
                Some(format!("{INITIAL_NOISE_PHASE_ID}:1")),
                "configuration-v1",
                NATIVE_DIFFUSION_REGISTRY_VERSION,
                token,
            )
        };
        let canonical = key(&inputs, &change_token)?;
        let mut cache = NativeCache::new(8)?;
        cache.insert(
            canonical.clone(),
            crate::CacheEntry {
                outputs: vec![json!("latent")],
                ui: None,
            },
        );
        assert!(cache.get(&key(&inputs, &change_token)?).is_some());
        assert!(cache.get(&key(&inputs, "sim.comfy.guidance.v0")?).is_none());

        for (name, value) in [
            ("model", json!("model-digest-b")),
            ("positive", json!("conditioning-other")),
            ("negative", json!("conditioning-other")),
            ("cfg", json!(1.0)),
        ] {
            let mut changed = inputs.clone();
            changed.insert(name.to_owned(), value);
            assert_ne!(key(&changed, &change_token)?, canonical);
        }
        let mut swapped = inputs.clone();
        let positive = inputs.get("positive").cloned().ok_or("positive input")?;
        let negative = inputs.get("negative").cloned().ok_or("negative input")?;
        swapped.insert("positive".to_owned(), negative);
        swapped.insert("negative".to_owned(), positive);
        assert_ne!(key(&swapped, &change_token)?, canonical);
        Ok(())
    }

    #[test]
    fn certified_worker_properties_project_exact_effective_backend_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let device = comfy_tensor::DeviceId::new(comfy_types::DeviceKind::Mlu, 2);
        let properties = comfy_tensor::NativeDeviceProperties::new_with_allocation_limit(
            device,
            "Cambricon MLU fixture",
            24 * 1024 * 1024 * 1024,
            20 * 1024 * 1024 * 1024,
            11,
            0,
            Some("Neuware 1.20".to_owned()),
            true,
        )?;
        let support = comfy_tensor::OperationSupport::allocation(
            comfy_tensor::DType::F16,
            comfy_tensor::Layout::Contiguous,
        );
        let matrix = BackendCapabilityMatrix::new_with_properties(
            device,
            vec![support],
            vec![support],
            Some(properties),
        )?;
        let worker = matrix.to_worker_capabilities()?;
        let accepted = BackendCapabilityMatrix::try_from(worker)?;
        let effective = project_effective_native_backend(
            &accepted,
            16 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );

        assert_eq!(effective.device, device);
        assert_eq!(effective.device_name, "Cambricon MLU fixture");
        assert_eq!(effective.architecture.as_deref(), Some("Neuware 1.20"));
        assert_eq!(effective.total_memory_bytes, Some(24 * 1024 * 1024 * 1024));
        assert_eq!(
            effective.allocation_limit_bytes,
            Some(20 * 1024 * 1024 * 1024)
        );
        assert_eq!(effective.memory_limit_bytes, 16 * 1024 * 1024 * 1024);
        assert_eq!(effective.memory_in_use_bytes, 0);
        assert_eq!(effective.memory_policy, MemoryPolicy::Balanced);
        assert_eq!(effective.supported_operation_rows, 1);
        assert_eq!(effective.deterministic_operation_rows, 1);

        let device_limited = project_effective_native_backend(
            &accepted,
            22 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );
        assert_eq!(device_limited.memory_limit_bytes, 20 * 1024 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn certified_directml_properties_keep_all_three_memory_limits_distinct()
    -> Result<(), Box<dyn std::error::Error>> {
        let device = comfy_tensor::DeviceId::new(comfy_types::DeviceKind::DirectMl, 0);
        let properties = comfy_tensor::NativeDeviceProperties::new_with_allocation_limit(
            device,
            "DirectML certified adapter",
            24 * 1024 * 1024 * 1024,
            18 * 1024 * 1024 * 1024,
            1,
            13,
            Some("DXGI adapter LUID 0x1122334455667788".to_owned()),
            true,
        )?;
        let support = comfy_tensor::OperationSupport::allocation(
            comfy_tensor::DType::F16,
            comfy_tensor::Layout::Contiguous,
        );
        let matrix = BackendCapabilityMatrix::new_with_properties(
            device,
            vec![support],
            vec![support],
            Some(properties),
        )?;
        let accepted = BackendCapabilityMatrix::try_from(matrix.to_worker_capabilities()?)?;
        let effective = project_effective_native_backend(
            &accepted,
            12 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );

        assert_eq!(effective.device, device);
        assert_eq!(effective.device_name, "DirectML certified adapter");
        assert_eq!(
            effective.architecture.as_deref(),
            Some("DXGI adapter LUID 0x1122334455667788")
        );
        assert_eq!(effective.total_memory_bytes, Some(24 * 1024 * 1024 * 1024));
        assert_eq!(
            effective.allocation_limit_bytes,
            Some(18 * 1024 * 1024 * 1024)
        );
        assert_eq!(effective.memory_limit_bytes, 12 * 1024 * 1024 * 1024);

        let allocation_limited = project_effective_native_backend(
            &accepted,
            20 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );
        assert_eq!(
            allocation_limited.memory_limit_bytes,
            18 * 1024 * 1024 * 1024
        );
        Ok(())
    }

    #[test]
    fn certified_npu_properties_keep_validated_device_and_effective_limits_distinct()
    -> Result<(), Box<dyn std::error::Error>> {
        let device = comfy_tensor::DeviceId::new(comfy_types::DeviceKind::Npu, 2);
        let properties = comfy_tensor::NativeDeviceProperties::new_with_allocation_limit(
            device,
            "Huawei Ascend certified device",
            32 * 1024 * 1024 * 1024,
            24 * 1024 * 1024 * 1024,
            8,
            0,
            Some("AscendCL 8.0.0".to_owned()),
            true,
        )?;
        let support = comfy_tensor::OperationSupport::allocation(
            comfy_tensor::DType::F16,
            comfy_tensor::Layout::Contiguous,
        );
        let matrix = BackendCapabilityMatrix::new_with_properties(
            device,
            vec![support],
            vec![support],
            Some(properties),
        )?;
        let accepted = BackendCapabilityMatrix::try_from(matrix.to_worker_capabilities()?)?;
        let effective = project_effective_native_backend(
            &accepted,
            12 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );

        assert_eq!(effective.device, device);
        assert_eq!(effective.device_name, "Huawei Ascend certified device");
        assert_eq!(effective.architecture.as_deref(), Some("AscendCL 8.0.0"));
        assert_eq!(effective.total_memory_bytes, Some(32 * 1024 * 1024 * 1024));
        assert_eq!(
            effective.allocation_limit_bytes,
            Some(24 * 1024 * 1024 * 1024)
        );
        assert_eq!(effective.memory_limit_bytes, 12 * 1024 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn certified_xpu_properties_keep_validated_device_and_effective_limits_distinct()
    -> Result<(), Box<dyn std::error::Error>> {
        let device = comfy_tensor::DeviceId::new(comfy_types::DeviceKind::Xpu, 3);
        let properties = comfy_tensor::NativeDeviceProperties::new_with_allocation_limit(
            device,
            "Intel XPU certified device",
            16 * 1024 * 1024 * 1024,
            6 * 1024 * 1024 * 1024,
            1,
            11,
            Some("Intel 0x8086:0x56a0; oneDNN 3.5.0".to_owned()),
            true,
        )?;
        let support = comfy_tensor::OperationSupport::allocation(
            comfy_tensor::DType::F16,
            comfy_tensor::Layout::Contiguous,
        );
        let matrix = BackendCapabilityMatrix::new_with_properties(
            device,
            vec![support],
            vec![support],
            Some(properties),
        )?;
        let accepted = BackendCapabilityMatrix::try_from(matrix.to_worker_capabilities()?)?;
        let effective = project_effective_native_backend(
            &accepted,
            8 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );

        assert_eq!(effective.device, device);
        assert_eq!(effective.device_name, "Intel XPU certified device");
        assert_eq!(
            effective.architecture.as_deref(),
            Some("Intel 0x8086:0x56a0; oneDNN 3.5.0")
        );
        assert_eq!(effective.total_memory_bytes, Some(16 * 1024 * 1024 * 1024));
        assert_eq!(
            effective.allocation_limit_bytes,
            Some(6 * 1024 * 1024 * 1024)
        );
        assert_eq!(effective.memory_limit_bytes, 6 * 1024 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn certified_cuda_properties_keep_validated_device_and_effective_limits_distinct()
    -> Result<(), Box<dyn std::error::Error>> {
        let device = comfy_tensor::DeviceId::new(comfy_types::DeviceKind::Cuda, 3);
        let properties = comfy_tensor::NativeDeviceProperties::new_with_allocation_limit(
            device,
            "NVIDIA CUDA certified device",
            24 * 1024 * 1024 * 1024,
            18 * 1024 * 1024 * 1024,
            12,
            8,
            Some("CUDA driver 12080; NVRTC 12.8".to_owned()),
            true,
        )?;
        let support = comfy_tensor::OperationSupport::allocation(
            comfy_tensor::DType::F16,
            comfy_tensor::Layout::Contiguous,
        );
        let matrix = BackendCapabilityMatrix::new_with_properties(
            device,
            vec![support],
            vec![support],
            Some(properties),
        )?;
        let accepted = BackendCapabilityMatrix::try_from(matrix.to_worker_capabilities()?)?;
        let effective = project_effective_native_backend(
            &accepted,
            12 * 1024 * 1024 * 1024,
            MemoryPolicy::Balanced,
        );

        assert_eq!(effective.device, device);
        assert_eq!(effective.device_name, "NVIDIA CUDA certified device");
        assert_eq!(
            effective.architecture.as_deref(),
            Some("CUDA driver 12080; NVRTC 12.8")
        );
        assert_eq!(effective.total_memory_bytes, Some(24 * 1024 * 1024 * 1024));
        assert_eq!(
            effective.allocation_limit_bytes,
            Some(18 * 1024 * 1024 * 1024)
        );
        assert_eq!(effective.memory_limit_bytes, 12 * 1024 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn controller_construction_negotiates_worker_readiness_before_returning() {
        let source = include_str!("native_execution_controller.rs");
        let start = source
            .find("impl NativeExecutionController")
            .expect("controller implementation");
        let source = source
            .get(start..)
            .expect("controller implementation slice");
        let readiness = source
            .find("smol::block_on(RuntimeSupervisor::start(config.worker.clone()))")
            .expect("eager worker readiness");
        let thread = source
            .find("thread::Builder::new()")
            .expect("controller runner construction");
        let returned = source
            .find("Ok(Arc::new(Self")
            .expect("ready controller return");
        assert!(readiness < thread);
        assert!(thread < returned);
    }

    impl NativeDiffusionProvider for WorkspaceProbeDiffusionProvider {
        fn model_digest(&self) -> Result<String, NativeImageRuntimeError> {
            Ok("0".repeat(64))
        }

        fn tokenizer_digest(&self) -> Result<String, NativeImageRuntimeError> {
            Ok("1".repeat(64))
        }

        fn clip_cache_identities(
            &self,
        ) -> Result<CanonicalClipCacheIdentities, NativeImageRuntimeError> {
            CanonicalClipCacheIdentities::checked(
                "1".repeat(64),
                "2".repeat(64),
                "0".repeat(64),
                "3".repeat(64),
                "4".repeat(64),
                "5".repeat(64),
            )
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
        }

        fn load(
            &self,
            backend: Arc<CpuBackend>,
            context: &ExecutionContext<'_>,
        ) -> Result<NativeDiffusionBundle, NativeImageRuntimeError> {
            let mut staging = backend.workspace_vec::<u8>(context, 128)?;
            for value in 0..128_u8 {
                staging.try_push(value)?;
            }
            self.observed.store(true, Ordering::SeqCst);
            Err(NativeImageRuntimeError::Execution(
                "workspace authority probe completed".to_owned(),
            ))
        }
    }

    impl ExecutionAttemptPersistence for FailOnceExecutionPersistence {
        fn replace_execution_state(
            &self,
            profile: PersistedExecutionProfile,
            attempts: Vec<PersistedExecutionAttempt>,
        ) -> BoxFuture<'static, anyhow::Result<()>> {
            let database = self.database.clone();
            let fail_next = self.fail_next.clone();
            Box::pin(async move {
                if fail_next.swap(false, Ordering::SeqCst) {
                    anyhow::bail!("injected output projection persistence failure");
                }
                database.replace_execution_profile(profile, attempts).await
            })
        }

        fn load_execution_state(
            &self,
            profile_id: ProfileId,
        ) -> anyhow::Result<(
            Option<PersistedExecutionProfile>,
            Vec<PersistedExecutionAttempt>,
        )> {
            Ok((
                self.database.load_execution_profile(profile_id)?,
                self.database
                    .load_execution_attempts_for_profile(profile_id)?,
            ))
        }
    }

    #[test]
    fn runtime_registry_contains_exact_native_image_slice() -> Result<(), NativeImageRuntimeError> {
        let registry = native_image_registry_projection()?;
        let class_types = [
            "LoadImage",
            "ImageScale",
            "ImageInvert",
            "PreviewImage",
            "SaveImage",
        ];
        assert_eq!(registry.descriptor_len(), class_types.len());
        for class_type in class_types {
            let descriptor = registry.descriptor(class_type).ok_or_else(|| {
                NativeImageRuntimeError::Registry(format!("missing {class_type}"))
            })?;
            assert_eq!(descriptor.availability, RuntimeAvailability::Native);
            assert_eq!(
                descriptor.implementation_version,
                NATIVE_IMAGE_REGISTRY_VERSION
            );
            assert!(registry.node(class_type).is_some());
            assert_eq!(
                registry.implementation_namespace(class_type),
                Some("sim.native_rust")
            );
        }
        Ok(())
    }

    #[test]
    fn native_image_executor_preserves_the_injected_backend_owner()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temporary, roots) = fixture_roots()?;
        let (cpu_backend, _workspace_authority) = CpuWorkspaceAuthority::create_backend(1024)?;
        let cpu_backend = Arc::new(cpu_backend);
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let executor = NativeImageExecutor::new_with_cpu_backend(
            profile_id,
            BTreeMap::new(),
            true,
            cpu_backend.clone(),
        )?;
        assert!(Arc::ptr_eq(executor.cpu_backend(), &cpu_backend));
        Ok(())
    }

    #[test]
    fn native_diffusion_provider_uses_the_node_workspace_authority_without_reauthorization()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024)?;
        let backend = Arc::new(backend);
        let scratch = workspace_authority.authorize_workspace(256)?;
        let cancellation = CancellationToken::default();
        let context = backend.execution_context(StreamId::DEFAULT, scratch.clone(), &cancellation);
        let observed = Arc::new(AtomicBool::new(false));
        let state = NativeDiffusionState {
            provider: Arc::new(WorkspaceProbeDiffusionProvider {
                observed: observed.clone(),
            }),
            backend,
            loaded: Mutex::new(None),
        };

        assert!(matches!(
            state.bundle(&context),
            Err(NativeImageRuntimeError::Execution(message))
                if message == "workspace authority probe completed"
        ));
        assert!(observed.load(Ordering::SeqCst));
        assert_eq!(scratch.peak_bytes(), 128);
        assert_eq!(scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn save_image_preserves_typed_media_cancellation() {
        let cancelled = media_failure(PngError::Tensor(TensorError::Cancelled));
        assert_eq!(cancelled.code, "native_image_cancelled");
        assert_eq!(cancelled.kind, NodeFailureKind::Interrupted);
        assert!(cancelled.retryable);

        let allocation = media_failure(PngError::Tensor(TensorError::AllocationFailed {
            requested: 16,
            reason: "injected allocation failure".to_owned(),
        }));
        assert_eq!(allocation.code, "native_image_tensor_failed");
        assert_eq!(allocation.kind, NodeFailureKind::Failure);
        assert!(!allocation.retryable);

        let codec = media_failure(PngError::Codec("injected codec failure".to_owned()));
        assert_eq!(codec.code, "native_image_codec_failed");
        assert_eq!(codec.kind, NodeFailureKind::Failure);
        assert!(!codec.retryable);
    }

    #[test]
    fn typed_tensor_handles_are_content_addressed_and_kind_checked()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (cpu_backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024)?;
        let workspace = workspace_authority.authorize_workspace(1024)?;
        let tensor_context =
            cpu_backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);
        let tensor =
            ImageTensor::from_f32(&cpu_backend, &tensor_context, 1, 1, 1, 3, &[0.0, 0.5, 1.0])?;
        let raw_tensor = tensor.tensor().clone();
        let mut store = NativeTensorStore::default();
        let first = store.insert(NativeTensorKind::Image, tensor.clone())?;
        let second = store.insert(NativeTensorKind::Image, tensor)?;
        assert_eq!(first, second);
        assert!(store.get(&first, NativeTensorKind::Image).is_ok());
        assert!(store.get(&first, NativeTensorKind::Mask).is_err());
        let mut forged_image_kind = serde_json::to_value(&first)?;
        forged_image_kind["kind"] = json!("mask");
        let forged_image_kind = serde_json::from_value::<NativeTensorHandle>(forged_image_kind)?;
        assert!(
            store
                .get(&forged_image_kind, NativeTensorKind::Mask)
                .is_err()
        );
        let raw_handle = store.insert_tensor(NativeTensorKind::Conditioning, raw_tensor)?;
        assert!(
            store
                .get_tensor(&raw_handle, NativeTensorKind::Conditioning)
                .is_ok()
        );
        let mut forged_raw_kind = serde_json::to_value(&raw_handle)?;
        forged_raw_kind["kind"] = json!("latent");
        let forged_raw_kind = serde_json::from_value::<NativeTensorHandle>(forged_raw_kind)?;
        assert!(
            store
                .get_tensor(&forged_raw_kind, NativeTensorKind::Latent)
                .is_err()
        );
        let mut invalid_raw_digest = serde_json::to_value(&raw_handle)?;
        invalid_raw_digest["content_digest"] = json!("g".repeat(64));
        assert!(serde_json::from_value::<NativeTensorHandle>(invalid_raw_digest).is_err());

        let non_hex_digest = NativeTensorHandle::new(
            NativeTensorKind::Image,
            "g".repeat(64),
            first.descriptor().clone(),
        );
        assert!(matches!(
            non_hex_digest,
            Err(NativeImageRuntimeError::Handle(message))
                if message == "schema, kind, or digest did not match the typed port"
        ));

        let mismatched = NativeTensorHandle::new(
            first.kind(),
            first.content_digest(),
            TensorDescriptor::contiguous(
                vec![1, 1, 1, 3],
                first.descriptor().dtype(),
                first.descriptor().device(),
                comfy_tensor::StreamId::new(1),
            )?,
        )?;
        assert!(store.get(&mismatched, NativeTensorKind::Image).is_err());

        let mut invalid_wire = serde_json::to_value(&first)?;
        let strides = invalid_wire
            .pointer_mut("/descriptor/strides")
            .ok_or("native tensor handle descriptor strides are unavailable")?;
        *strides = json!([0, 0, 0, 0]);
        assert!(serde_json::from_value::<NativeTensorHandle>(invalid_wire).is_err());

        let mut invalid_schema = serde_json::to_value(&first)?;
        invalid_schema["schema_version"] = json!(NATIVE_TENSOR_HANDLE_SCHEMA_VERSION + 1);
        assert!(serde_json::from_value::<NativeTensorHandle>(invalid_schema).is_err());

        let mut unknown_field = serde_json::to_value(&first)?;
        unknown_field["future"] = json!(true);
        assert!(serde_json::from_value::<NativeTensorHandle>(unknown_field).is_err());
        Ok(())
    }

    #[test]
    fn native_worker_progress_wire_round_trips_without_self_describing_serde() {
        let event = NativeImageWorkerEvent::Progress {
            progress: NativeImageWorkerProgress {
                profile_id: ProfileId(Uuid::from_u128(1)),
                prompt_id: PromptId(Uuid::from_u128(2)),
                attempt_id: AttemptId(Uuid::from_u128(3)),
                sequence: 4,
                node_id: Some(NodeId("5".to_owned())),
                kind: NativeImageWorkerProgressKind::OutputPrepared {
                    transaction_id: Uuid::from_u128(6),
                },
            },
        };
        let encoded = postcard::to_stdvec(&event).expect("encode native worker progress");
        assert_eq!(
            postcard::from_bytes::<NativeImageWorkerEvent>(&encoded)
                .expect("decode native worker progress"),
            event
        );
    }

    #[test]
    fn native_worker_event_discriminants_are_append_only() -> Result<(), Box<dyn std::error::Error>>
    {
        let progress = NativeImageWorkerEvent::Progress {
            progress: NativeImageWorkerProgress {
                profile_id: ProfileId(Uuid::from_u128(1)),
                prompt_id: PromptId(Uuid::from_u128(2)),
                attempt_id: AttemptId(Uuid::from_u128(3)),
                sequence: 0,
                node_id: None,
                kind: NativeImageWorkerProgressKind::Started,
            },
        };
        let completed = NativeImageWorkerEvent::Completed {
            result: NativeImageWorkerResult::from_execution_report(
                ExecutionReport {
                    profile_id: ProfileId(Uuid::from_u128(1)),
                    prompt_id: PromptId(Uuid::from_u128(2)),
                    attempt_id: AttemptId(Uuid::from_u128(3)),
                    state: AttemptState::Succeeded,
                    outputs: BTreeMap::new(),
                    ui_outputs: BTreeMap::new(),
                    events: Vec::new(),
                    cache_hits: 0,
                    error: None,
                },
                Vec::new(),
                1,
            )?,
        };
        let failed = NativeImageWorkerEvent::Failed {
            message: "failure".to_owned(),
            cancelled: false,
        };
        let unavailable = NativeImageWorkerEvent::BackendUnavailable {
            unavailable: BackendUnavailable::new(comfy_types::DeviceKind::Rocm, "unavailable"),
        };
        for (event, discriminant) in [
            (progress, 0_u8),
            (completed, 1),
            (failed, 2),
            (unavailable, 3),
        ] {
            assert_eq!(postcard::to_stdvec(&event)?.first(), Some(&discriminant));
        }
        Ok(())
    }

    #[test]
    fn backend_unavailable_projects_as_non_retryable_validation_failure() {
        let failure = backend_unavailable_failure(&BackendUnavailable::new(
            comfy_types::DeviceKind::Rocm,
            "exact graph rows are unavailable",
        ));
        assert_eq!(failure.code, "native_backend_unavailable");
        assert_eq!(failure.origin, ExecutionFailureOrigin::Validation);
        assert!(!failure.retryable);
    }

    #[test]
    fn native_worker_result_maps_ui_outputs_through_a_bounded_wire_dto()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(super::val_domain_004_worker_ui_wire_adapter_case()?);
        Ok(())
    }

    #[test]
    fn malformed_native_worker_events_are_typed_protocol_errors() {
        assert!(matches!(
            decode_native_image_worker_event(&[]),
            Err(NativeImageRuntimeError::WorkerEvent(message))
                if message.contains("could not be decoded as the native image schema")
        ));
    }

    #[test]
    fn native_controller_has_no_queue_state_and_leases_shared_presentation_work()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temporary, roots) = fixture_roots()?;
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let registry = native_image_registry_projection()?;
        let mut plan = PromptCompiler::new(&registry).compile(fixture_prompt())?;
        plan.prompt_id = PromptId(Uuid::from_u128(0x1911));
        let attempt_id = AttemptId(Uuid::from_u128(0x1901));
        let mut presentation_service =
            ExecutionPresentationService::new_with_first_attempt_id(16, attempt_id)?;
        presentation_service.initialize_profile(
            profile_id,
            ExecutionDataSource::Live,
            ExecutionSnapshotStatus::Ready,
        )?;
        let queue_command = ExecutionControlCommand {
            request_id: RequestId(Uuid::from_u128(0x1902)),
            profile_id,
            expected_revision: None,
            kind: ExecutionControlCommandKind::Queue {
                plan: plan.clone(),
                priority: 7,
                front: true,
            },
        };
        presentation_service.submit(queue_command.clone())?;
        presentation_service.apply_ack(ExecutionCommandAck {
            request_id: queue_command.request_id,
            profile_id,
            outcome: ExecutionCommandOutcome::Accepted {
                assigned_attempt_id: Some(attempt_id),
            },
        })?;
        let presentation = crate::ExecutionPresentationOwner::ephemeral(presentation_service);
        let config = NativeExecutionControllerConfig::new(
            fixture_asset_service(&roots)?,
            presentation.clone(),
            WorkerLaunchConfig::new(
                PathBuf::from("unused-native-image-worker"),
                profile_id,
                WorkerId(Uuid::from_u128(0x1900)),
                NATIVE_IMAGE_REGISTRY_VERSION,
                1024,
            ),
            true,
        )?;
        let roots = config.roots()?;
        let output_committer = config.output_committer.clone();
        let input_authorization = authorize_native_input_reader(&roots.profile_id)?;
        let output_authorization = authorize_native_output_committer(&roots.profile_id)?;
        let mut state = NativeControllerState {
            config,
            event_bus: ExecutionEventBus::new(32)?,
            supervisor: None,
            active: None,
            output_committer,
            input_authorization,
            output_authorization,
        };
        let before = presentation.snapshot(profile_id)?;
        smol::block_on(state.apply_command(ExecutionControlCommand {
            request_id: RequestId(Uuid::from_u128(0x1903)),
            profile_id,
            expected_revision: None,
            kind: ExecutionControlCommandKind::Queue {
                plan: plan.clone(),
                priority: -100,
                front: false,
            },
        }))?;
        smol::block_on(state.apply_command(ExecutionControlCommand {
            request_id: RequestId(Uuid::from_u128(0x1904)),
            profile_id,
            expected_revision: None,
            kind: ExecutionControlCommandKind::Reorder {
                attempt_id,
                position: 0,
            },
        }))?;
        assert_eq!(presentation.snapshot(profile_id)?, before);
        let lease = presentation
            .next_queued_attempt(profile_id)?
            .ok_or("shared presentation omitted queued work")?;
        assert_eq!(lease.profile_id, profile_id);
        assert_eq!(lease.prompt_id, plan.prompt_id);
        assert_eq!(lease.attempt_id, attempt_id);
        assert_eq!(lease.plan, plan);
        Ok(())
    }

    #[test]
    fn output_committer_is_the_durable_owner_of_scoped_recovery_receipts()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temporary, roots) = fixture_roots()?;
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let prompt_id = PromptId(Uuid::from_u128(0x1961));
        let attempt_id = AttemptId(Uuid::from_u128(0x1951));
        let proposal_id = Uuid::from_u128(0x1971);
        let proposal = OutputProposal::new(
            proposal_id,
            AssetNamespace::Output,
            "recovery/scoped-receipt",
            "bin",
            0,
            0,
            0,
            b"scoped receipt".to_vec(),
        )?;
        let scope = OutputExecutionScope {
            profile_id,
            prompt_id,
            attempt_id,
        };
        let authorization = authorize_native_output_committer(&roots.profile_id)?;
        let cancellation = CancellationToken::default();
        let mut committer = OutputCommitter::open(roots.clone())?;
        let receipts = committer.commit_scoped_proposal_batch(
            &scope,
            &[proposal],
            Local::now().fixed_offset(),
            &authorization,
            &cancellation,
        )?;
        let receipt = receipts
            .into_iter()
            .next()
            .ok_or("missing scoped commit receipt")?;
        assert_eq!(receipt.operation().execution_scope.as_ref(), Some(&scope));
        assert_eq!(receipt.operation().proposal_id, Some(proposal_id));

        let mut recovery_projection = crate::RecoveryJournal::default();
        recovery_projection.record_output_receipt(profile_id, prompt_id, attempt_id, &receipt)?;
        let encoded = recovery_projection.encode()?;
        let decoded = crate::RecoveryJournal::decode(&encoded)?;
        let recorded = decoded
            .receipts_for_attempt(profile_id, prompt_id, attempt_id)
            .collect::<Vec<_>>();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].proposal_id(), receipt.proposal_id());
        assert_eq!(recorded[0].operation_id(), receipt.operation().operation_id);
        assert_eq!(recorded[0].identity(), &receipt.operation().identity);
        assert_eq!(recorded[0].sha256(), receipt.operation().sha256.as_str());
        assert_eq!(
            committer.committed_receipts_for_scope(&scope)?,
            vec![receipt.clone()]
        );
        drop(committer);
        let reopened = OutputCommitter::open(roots)?;
        assert_eq!(
            reopened.committed_receipts_for_scope(&scope)?,
            vec![receipt]
        );
        Ok(())
    }

    #[test]
    fn committed_output_receipts_reconcile_exactly_once_after_projection_failure()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_temporary, roots) = fixture_roots()?;
            let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
            let registry = native_image_registry_projection()?;
            let mut plan = PromptCompiler::new(&registry).compile(fixture_prompt())?;
            plan.prompt_id = PromptId(Uuid::from_u128(0x1972));
            let attempt_id = AttemptId(Uuid::from_u128(0x1973));
            let mut service =
                ExecutionPresentationService::new_with_first_attempt_id(16, attempt_id)?;
            service.initialize_profile(
                profile_id,
                ExecutionDataSource::Live,
                ExecutionSnapshotStatus::Ready,
            )?;
            let command = ExecutionControlCommand {
                request_id: RequestId(Uuid::from_u128(0x1974)),
                profile_id,
                expected_revision: None,
                kind: ExecutionControlCommandKind::Queue {
                    plan: plan.clone(),
                    priority: 0,
                    front: false,
                },
            };
            service.submit(command.clone())?;
            service.apply_ack(ExecutionCommandAck {
                request_id: command.request_id,
                profile_id,
                outcome: ExecutionCommandOutcome::Accepted {
                    assigned_attempt_id: Some(attempt_id),
                },
            })?;
            service.apply_actuator_event(
                profile_id,
                plan.prompt_id,
                attempt_id,
                None,
                AttemptEventKind::Started,
                None,
                Utc::now(),
            )?;
            let database =
                crate::ComfyRuntimeDb::open_test_db("native_output_receipt_projection_recovery")
                    .await;
            database
                .replace_execution_profile(
                    service.persisted_profile(profile_id)?,
                    service.persisted_attempts(profile_id)?,
                )
                .await?;
            let fail_next = Arc::new(AtomicBool::new(true));
            let presentation = crate::ExecutionPresentationOwner::persistent(
                service,
                Arc::new(FailOnceExecutionPersistence {
                    database,
                    fail_next: fail_next.clone(),
                }),
            );
            let assets = fixture_asset_service(&roots)?;
            let authorization = authorize_native_output_committer(&roots.profile_id)?;
            let proposal = NativeImageOutputProposal::new(
                NodeId("5".to_owned()),
                OutputProposal::new(
                    Uuid::from_u128(0x1975),
                    AssetNamespace::Output,
                    "recovery/projected-output",
                    "png",
                    0,
                    1,
                    1,
                    b"committed output".to_vec(),
                )?,
            )?;
            let scope = OutputExecutionScope {
                profile_id,
                prompt_id: plan.prompt_id,
                attempt_id,
            };
            let mut output_committer = OutputCommitter::open(roots.clone())?;
            {
                let mut asset_service = assets
                    .lock()
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                output_committer.commit_scoped_proposal_batch_and_register(
                    &scope,
                    &[proposal.output().clone()],
                    Local::now().fixed_offset(),
                    &mut asset_service,
                    &authorization,
                    &CancellationToken::default(),
                )?;
            }
            let mut config = NativeExecutionControllerConfig::new(
                assets,
                presentation.clone(),
                WorkerLaunchConfig::new(
                    PathBuf::from("unused-native-image-worker"),
                    profile_id,
                    WorkerId(Uuid::from_u128(0x1976)),
                    NATIVE_IMAGE_REGISTRY_VERSION,
                    1024,
                ),
                true,
            )?;
            config.output_committer = Arc::new(std::sync::Mutex::new(output_committer));
            let mut state = NativeControllerState {
                output_committer: config.output_committer.clone(),
                config,
                event_bus: ExecutionEventBus::new(16)?,
                supervisor: None,
                active: None,
                input_authorization: authorize_native_input_reader(&roots.profile_id)?,
                output_authorization: authorization,
            };

            assert!(state.reconcile_committed_output_receipts().await.is_err());
            assert!(!fail_next.load(Ordering::SeqCst));
            let after_failure = presentation.snapshot(profile_id)?;
            let attempt = after_failure
                .attempts
                .iter()
                .find(|attempt| attempt.attempt_id == attempt_id)
                .ok_or("running attempt disappeared after persistence failure")?;
            assert_eq!(attempt.state, AttemptState::Running);
            assert!(attempt.outputs.is_empty());
            assert_eq!(
                state
                    .output_committer
                    .lock()
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?
                    .committed_receipts_for_scope(&scope)?
                    .len(),
                1
            );

            state.reconcile_committed_output_receipts().await?;
            let recovered = presentation.snapshot(profile_id)?;
            let attempt = recovered
                .attempts
                .iter()
                .find(|attempt| attempt.attempt_id == attempt_id)
                .ok_or("recovered attempt disappeared")?;
            assert_eq!(attempt.state, AttemptState::Succeeded);
            assert_eq!(attempt.outputs.len(), 1);
            let recovered_revision = recovered.revision;
            drop(recovered);

            state.reconcile_committed_output_receipts().await?;
            assert_eq!(
                presentation.snapshot(profile_id)?.revision,
                recovered_revision
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn worker_loss_applies_one_shared_recovery_interrupted_event()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temporary, roots) = fixture_roots()?;
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let registry = native_image_registry_projection()?;
        let mut plan = PromptCompiler::new(&registry).compile(fixture_prompt())?;
        plan.prompt_id = PromptId(Uuid::from_u128(0x1981));
        let attempt_id = AttemptId(Uuid::from_u128(0x1983));
        let mut presentation_service =
            ExecutionPresentationService::new_with_first_attempt_id(16, attempt_id)?;
        presentation_service.initialize_profile(
            profile_id,
            ExecutionDataSource::Live,
            ExecutionSnapshotStatus::Ready,
        )?;
        let queue_command = ExecutionControlCommand {
            request_id: RequestId(Uuid::from_u128(0x1984)),
            profile_id,
            expected_revision: None,
            kind: ExecutionControlCommandKind::Queue {
                plan: plan.clone(),
                priority: 0,
                front: false,
            },
        };
        presentation_service.submit(queue_command.clone())?;
        presentation_service.apply_ack(ExecutionCommandAck {
            request_id: queue_command.request_id,
            profile_id,
            outcome: ExecutionCommandOutcome::Accepted {
                assigned_attempt_id: Some(attempt_id),
            },
        })?;
        let lease = presentation_service
            .next_queued_attempt(profile_id)?
            .ok_or("missing queued execution lease")?;
        presentation_service.apply_actuator_event(
            profile_id,
            plan.prompt_id,
            attempt_id,
            None,
            AttemptEventKind::Started,
            None,
            Utc::now(),
        )?;
        let presentation = crate::ExecutionPresentationOwner::ephemeral(presentation_service);
        let config = NativeExecutionControllerConfig::new(
            fixture_asset_service(&roots)?,
            presentation.clone(),
            WorkerLaunchConfig::new(
                PathBuf::from("unused-native-image-worker"),
                profile_id,
                WorkerId(Uuid::from_u128(0x1980)),
                NATIVE_IMAGE_REGISTRY_VERSION,
                1024,
            ),
            true,
        )?;
        let event_bus = ExecutionEventBus::new(8)?;
        let events = event_bus.subscribe();
        let roots = config.roots()?;
        let output_committer = config.output_committer.clone();
        let input_authorization = authorize_native_input_reader(&roots.profile_id)?;
        let output_authorization = authorize_native_output_committer(&roots.profile_id)?;
        let mut state = NativeControllerState {
            config,
            event_bus,
            supervisor: None,
            active: Some(ActiveNativeExecution {
                profile_id,
                prompt_id: plan.prompt_id,
                attempt_id,
                cancellation: lease.cancellation,
                output_proposals: BTreeMap::new(),
                output_proposal_bytes: 0,
            }),
            output_committer,
            input_authorization,
            output_authorization,
        };

        smol::block_on(state.recover_worker(RuntimeSupervisorError::ChannelClosed))?;
        assert!(state.active.is_none());
        let event = events.try_recv()?;
        assert_eq!(event.profile_id, profile_id);
        assert_eq!(event.prompt_id, plan.prompt_id);
        assert_eq!(event.attempt_id, attempt_id);
        assert_eq!(event.sequence, 1);
        assert_eq!(
            event.kind,
            AttemptEventKind::RecoveryInterrupted {
                reason: crate::ExecutionRecoveryInterruptionReason::RuntimeRestart,
            }
        );
        assert_eq!(
            event.data,
            Some(json!({"worker_error": RuntimeSupervisorError::ChannelClosed.to_string()}))
        );
        assert!(events.try_recv().is_err());
        let snapshot = presentation.snapshot(profile_id)?;
        let attempt = snapshot
            .attempts
            .iter()
            .find(|attempt| attempt.attempt_id == attempt_id)
            .ok_or("shared presentation omitted interrupted attempt")?;
        assert_eq!(attempt.state, AttemptState::Interrupted);
        assert_eq!(attempt.last_sequence, Some(1));
        assert_eq!(attempt.canonical_event_count, 2);
        assert_eq!(
            attempt.recovery_interruption_reason,
            Some(crate::ExecutionRecoveryInterruptionReason::RuntimeRestart)
        );
        Ok(())
    }

    #[test]
    fn worker_cancellation_observation_cannot_choose_the_canonical_terminal_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temporary, roots) = fixture_roots()?;
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let registry = native_image_registry_projection()?;
        let mut plan = PromptCompiler::new(&registry).compile(fixture_prompt())?;
        plan.prompt_id = PromptId(Uuid::from_u128(0x1991));
        let attempt_id = AttemptId(Uuid::from_u128(0x1992));
        let mut presentation_service =
            ExecutionPresentationService::new_with_first_attempt_id(16, attempt_id)?;
        presentation_service.initialize_profile(
            profile_id,
            ExecutionDataSource::Live,
            ExecutionSnapshotStatus::Ready,
        )?;
        let queue_command = ExecutionControlCommand {
            request_id: RequestId(Uuid::from_u128(0x1993)),
            profile_id,
            expected_revision: None,
            kind: ExecutionControlCommandKind::Queue {
                plan: plan.clone(),
                priority: 0,
                front: false,
            },
        };
        presentation_service.submit(queue_command.clone())?;
        presentation_service.apply_ack(ExecutionCommandAck {
            request_id: queue_command.request_id,
            profile_id,
            outcome: ExecutionCommandOutcome::Accepted {
                assigned_attempt_id: Some(attempt_id),
            },
        })?;
        let lease = presentation_service
            .next_queued_attempt(profile_id)?
            .ok_or("missing queued execution lease")?;
        presentation_service.apply_actuator_event(
            profile_id,
            plan.prompt_id,
            attempt_id,
            None,
            AttemptEventKind::Started,
            None,
            Utc::now(),
        )?;
        let presentation = crate::ExecutionPresentationOwner::ephemeral(presentation_service);
        let config = NativeExecutionControllerConfig::new(
            fixture_asset_service(&roots)?,
            presentation.clone(),
            WorkerLaunchConfig::new(
                PathBuf::from("unused-native-image-worker"),
                profile_id,
                WorkerId(Uuid::from_u128(0x1994)),
                NATIVE_IMAGE_REGISTRY_VERSION,
                1024,
            ),
            true,
        )?;
        let roots = config.roots()?;
        let canonical_cancellation = lease.cancellation.clone();
        let event_bus = ExecutionEventBus::new(8)?;
        let events = event_bus.subscribe();
        let mut state = NativeControllerState {
            output_committer: config.output_committer.clone(),
            config,
            event_bus,
            supervisor: None,
            active: Some(ActiveNativeExecution {
                profile_id,
                prompt_id: plan.prompt_id,
                attempt_id,
                cancellation: lease.cancellation,
                output_proposals: BTreeMap::new(),
                output_proposal_bytes: 0,
            }),
            input_authorization: authorize_native_input_reader(&roots.profile_id)?,
            output_authorization: authorize_native_output_committer(&roots.profile_id)?,
        };
        let event = NativeImageWorkerEvent::Failed {
            message: "untrusted worker cancellation observation".to_owned(),
            cancelled: true,
        };
        let envelope = comfy_types::WorkerEnvelope {
            version: comfy_types::WORKER_PROTOCOL_VERSION,
            profile_id,
            worker_id: WorkerId(Uuid::from_u128(0x1994)),
            request_id: RequestId(Uuid::from_u128(0x1995)),
            prompt_id: Some(plan.prompt_id),
            attempt_id: Some(attempt_id),
            sequence: 1,
            registry_version: NATIVE_IMAGE_REGISTRY_VERSION.to_owned(),
            message: comfy_types::WorkerMessage::Event {
                event: postcard::to_stdvec(&event)?,
            },
            extensions: BTreeMap::new(),
        };

        smol::block_on(state.apply_worker_event(envelope))?;
        let snapshot = presentation.snapshot(profile_id)?;
        let attempt = snapshot
            .attempts
            .iter()
            .find(|attempt| attempt.attempt_id == attempt_id)
            .ok_or("failed attempt disappeared")?;
        assert_eq!(attempt.state, AttemptState::Failed);
        assert_eq!(
            attempt
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("native_worker_failed")
        );
        let projected = events.try_recv()?;
        assert_eq!(
            projected.data.as_ref(),
            Some(&json!({"worker_reported_cancelled": true}))
        );
        assert!(!canonical_cancellation.is_cancelled());
        Ok(())
    }

    #[test]
    fn val_cancel_001_native_image_adapter_cancels_before_dispatch_and_commit()
    -> Result<(), Box<dyn std::error::Error>> {
        let (temporary, roots) = fixture_roots()?;
        let input_path = roots
            .test_root_path(AssetNamespace::Input)?
            .join("fixture.png");
        fs::write(
            &input_path,
            encode_png_frame(
                &[0.0, 0.25, 0.5, 0.75, 1.0, 0.125],
                1,
                1,
                2,
                3,
                0,
                &BTreeMap::new(),
                PngLimits::default(),
            )?,
        )?;
        let registry = native_image_registry_projection()?;
        let plan = PromptCompiler::new(&registry).compile(fixture_prompt())?;
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let input_assets = collect_fixture_worker_input_assets(&plan, &roots)?;
        let (cpu_backend, workspace_authority) =
            CpuWorkspaceAuthority::create_backend(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?;
        let executor = NativeImageExecutor::new_with_cpu_backend(
            profile_id,
            input_assets,
            true,
            Arc::new(cpu_backend),
        )?;

        let first = executor.execute_blocking(
            &plan,
            AttemptId(Uuid::from_u128(1)),
            CancellationToken::default(),
            0,
            workspace_authority.authorize_workspace(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?,
        )?;
        assert_eq!(first.report.state, AttemptState::Succeeded);
        assert_eq!(first.report.cache_hits, 0);
        assert_eq!(first.output_proposals.len(), 2);
        assert!(
            fs::read_dir(roots.test_root_path(AssetNamespace::Output)?)?
                .next()
                .is_none(),
            "worker-side execution must not publish a final file"
        );
        let saved = first
            .output_proposals
            .iter()
            .find(|proposal| proposal.output.namespace() == AssetNamespace::Output)
            .ok_or("saved output")?;
        let mut output_committer = OutputCommitter::open(roots.clone())?;
        let authorization = authorize_native_output_committer(&roots.profile_id)?;
        let outputs = first
            .output_proposals
            .iter()
            .map(|proposal| proposal.output.clone())
            .collect::<Vec<_>>();
        let receipts = output_committer.commit_proposal_batch(
            &outputs,
            Local::now().fixed_offset(),
            &authorization,
            &CancellationToken::default(),
        )?;
        let saved_receipt = receipts
            .iter()
            .find(|receipt| receipt.proposal_id() == saved.proposal_id())
            .ok_or("saved receipt")?;
        let saved_bytes =
            fs::read(roots.test_resolve_existing(&saved_receipt.operation().identity)?)?;
        let decoded = decode_png(&saved_bytes, PngLimits::default())?;
        assert_eq!((decoded.width, decoded.height), (4, 2));
        let comfy_metadata = decoded.metadata.comfy_metadata();
        let prompt_metadata = comfy_metadata
            .prompt
            .as_deref()
            .ok_or("saved prompt metadata")?;
        assert!(serde_json::from_str::<Value>(prompt_metadata)?["1"].is_object());
        assert_eq!(
            comfy_metadata.workflow.as_deref(),
            Some("{\"version\":0.4}")
        );
        assert_eq!(
            decoded.pixels_bhwc,
            vec![
                1.0,
                192.0 / 255.0,
                127.0 / 255.0,
                1.0,
                192.0 / 255.0,
                127.0 / 255.0,
                63.0 / 255.0,
                0.0,
                224.0 / 255.0,
                63.0 / 255.0,
                0.0,
                224.0 / 255.0,
                1.0,
                192.0 / 255.0,
                127.0 / 255.0,
                1.0,
                192.0 / 255.0,
                127.0 / 255.0,
                63.0 / 255.0,
                0.0,
                224.0 / 255.0,
                63.0 / 255.0,
                0.0,
                224.0 / 255.0,
            ]
        );

        let second = executor.execute_blocking(
            &plan,
            AttemptId(Uuid::from_u128(2)),
            CancellationToken::default(),
            0,
            workspace_authority.authorize_workspace(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?,
        )?;
        assert_eq!(second.report.state, AttemptState::Succeeded);
        assert_eq!(second.report.cache_hits, 3);

        fs::write(
            &input_path,
            encode_png_frame(
                &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                1,
                1,
                2,
                3,
                0,
                &BTreeMap::new(),
                PngLimits::default(),
            )?,
        )?;
        let changed_input_assets = collect_fixture_worker_input_assets(&plan, &roots)?;
        let (changed_cpu_backend, changed_workspace_authority) =
            CpuWorkspaceAuthority::create_backend(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?;
        let changed_executor = NativeImageExecutor::new_with_cpu_backend(
            profile_id,
            changed_input_assets.clone(),
            true,
            Arc::new(changed_cpu_backend),
        )?;
        let invalidated = changed_executor.execute_blocking(
            &plan,
            AttemptId(Uuid::from_u128(3)),
            CancellationToken::default(),
            0,
            changed_workspace_authority
                .authorize_workspace(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?,
        )?;
        assert_eq!(invalidated.report.state, AttemptState::Succeeded);
        assert_eq!(invalidated.report.cache_hits, 0);

        let cancellation = CancellationToken::default();
        cancellation.cancel();
        assert_eq!(
            executor.execute_blocking(
                &plan,
                AttemptId(Uuid::from_u128(4)),
                cancellation,
                25,
                workspace_authority.authorize_workspace(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?,
            ),
            Err(NativeImageRuntimeError::Cancelled)
        );

        let (metadata_cpu_backend, metadata_workspace_authority) =
            CpuWorkspaceAuthority::create_backend(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?;
        let metadata_disabled_executor = NativeImageExecutor::new_with_cpu_backend(
            profile_id,
            changed_input_assets,
            false,
            Arc::new(metadata_cpu_backend),
        )?;
        let metadata_disabled = metadata_disabled_executor.execute_blocking(
            &plan,
            AttemptId(Uuid::from_u128(5)),
            CancellationToken::default(),
            0,
            metadata_workspace_authority
                .authorize_workspace(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?,
        )?;
        let metadata_disabled_output = metadata_disabled
            .output_proposals
            .iter()
            .find(|proposal| proposal.output.namespace() == AssetNamespace::Output)
            .ok_or("metadata-disabled saved output")?;
        let metadata_disabled_png = decode_png(
            metadata_disabled_output.output.content(),
            PngLimits::default(),
        )?;
        assert_eq!(
            metadata_disabled_png.metadata.comfy_metadata(),
            Default::default()
        );
        assert!(temporary.path().exists());
        Ok(())
    }

    fn fixture_roots() -> Result<(TempDir, AssetRoots), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let profile_id = Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_1901).to_string();
        let mut typed = Vec::new();
        for (namespace, name) in [
            (AssetNamespace::Input, "input"),
            (AssetNamespace::Output, "output"),
            (AssetNamespace::Temporary, "temporary"),
            (AssetNamespace::Model, "model"),
            (AssetNamespace::Plugin, "plugin"),
        ] {
            let path = temporary.path().join(name);
            fs::create_dir(&path)?;
            typed.push((namespace, path));
        }
        Ok((temporary, AssetRoots::new(profile_id, typed)?))
    }

    fn fixture_asset_service(
        roots: &AssetRoots,
    ) -> Result<SharedAssetService, Box<dyn std::error::Error>> {
        Ok(Arc::new(std::sync::Mutex::new(crate::AssetService::open(
            roots.clone(),
        )?)))
    }

    fn collect_fixture_worker_input_assets(
        plan: &CompiledPlan,
        roots: &AssetRoots,
    ) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn std::error::Error>> {
        let assets = fixture_asset_service(roots)?;
        let authorization = authorize_native_input_reader(&roots.profile_id)?;
        Ok(collect_worker_input_assets(
            plan,
            &assets,
            &authorization,
            &CancellationToken::default(),
        )?)
    }

    fn fixture_prompt() -> PromptSubmission {
        let link = |node: &str, output: usize| json!([node, output]);
        PromptSubmission {
            prompt: ApiPrompt(BTreeMap::from([
                (
                    NodeId("1".to_owned()),
                    PromptNode {
                        class_type: "LoadImage".to_owned(),
                        inputs: BTreeMap::from([("image".to_owned(), json!("fixture.png"))]),
                        unknown: BTreeMap::new(),
                    },
                ),
                (
                    NodeId("2".to_owned()),
                    PromptNode {
                        class_type: "ImageScale".to_owned(),
                        inputs: BTreeMap::from([
                            ("image".to_owned(), link("1", 0)),
                            ("upscale_method".to_owned(), json!("nearest-exact")),
                            ("width".to_owned(), json!(4)),
                            ("height".to_owned(), json!(0)),
                            ("crop".to_owned(), json!("disabled")),
                        ]),
                        unknown: BTreeMap::new(),
                    },
                ),
                (
                    NodeId("3".to_owned()),
                    PromptNode {
                        class_type: "ImageInvert".to_owned(),
                        inputs: BTreeMap::from([("image".to_owned(), link("2", 0))]),
                        unknown: BTreeMap::new(),
                    },
                ),
                (
                    NodeId("4".to_owned()),
                    PromptNode {
                        class_type: "PreviewImage".to_owned(),
                        inputs: BTreeMap::from([("images".to_owned(), link("3", 0))]),
                        unknown: BTreeMap::new(),
                    },
                ),
                (
                    NodeId("5".to_owned()),
                    PromptNode {
                        class_type: "SaveImage".to_owned(),
                        inputs: BTreeMap::from([
                            ("images".to_owned(), link("3", 0)),
                            ("filename_prefix".to_owned(), json!("native-image")),
                        ]),
                        unknown: BTreeMap::new(),
                    },
                ),
            ])),
            prompt_id: Some(PromptId(Uuid::from_u128(1901))),
            client_id: Some("native-image-test".to_owned()),
            number: Some(1.0),
            extra_data: BTreeMap::from([(
                "extra_pnginfo".to_owned(),
                json!({
                    "workflow": {"version": 0.4},
                    "frontendVersion": "native-image-test",
                }),
            )]),
            unknown: BTreeMap::new(),
        }
    }
}
