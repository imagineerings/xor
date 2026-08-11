use crate::{
    AssetNamespace, AssetRoots, AttemptEventKind, AttemptState, AuthorizedCapabilities,
    CacheDependencies, CanonicalClipCacheIdentities, CanonicalConditioningCacheIdentities,
    CanonicalNativeDiffusionCacheIdentities, CanonicalVaeCacheIdentities, CompiledPlan,
    EffectClass, EffectCoordinator, ExecutionActuatorEventInput, ExecutionControlCommand,
    ExecutionControlCommandKind, ExecutionController, ExecutionEngine, ExecutionError,
    ExecutionEventBus, ExecutionFailure, ExecutionFailureOrigin, ExecutionOutput,
    ExecutionOutputAvailability, ExecutionPreview, ExecutionReport, InputBinding, InputMode,
    MemoryPolicy, NativeCache, NativeNode, NativeNodeRegistry, NodeContext, NodeFailure,
    NodeFailureKind, NodeOutcome, OutputCommitError, OutputCommitReceipt, OutputCommitter,
    OutputExecutionScope, OutputMediaKind, OutputProposal, PreparedEffect, PreparedEffectRequest,
    PreparedOutput, ProfileId, PromptCompileError, RuntimeCachePolicy, RuntimeNodeDescriptor,
    RuntimeOutputDescriptor, RuntimeSupervisor, RuntimeSupervisorError, SharedAssetService,
    SharedExecutionPresentationService, SharedOutputCommitter, WorkerLaunchConfig,
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
    AttentionError, ClipTextError, LatentFormatError, ModelStoreError, NativeModelPayload,
    NativeOpsError, NativeVae, PatchGraph, QuantizationError, VaeArchitectureError,
    VaeBoundaryKind, VaeError, VaeKernelProfile,
    clip::{ClipError, LoadedSd1Clip, NativeTokenizer, WeightedText},
    conditioning::{
        ConditioningEntry, ConditioningEntryOptions, ConditioningError, ConditioningIdentity,
        ConditioningSet, ConditioningValue,
    },
    controlnet::{ControlChain, ControlModelExecutor, ControlNetError},
    generated_native_diffusion::{
        NativeDiffusionModelError, Sd1Tokenizer, Sd15TinyModel, empty_sd15_latent,
        sd15_latent_format_identity, sd15_model_family_identity,
    },
};
use comfy_nodes::{
    CatalogNodeDescriptor, NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeHandleKind,
    NativeHandleStoreError, NativeHandleType, NativeImageDescriptor, NativeImageDescriptorError,
    NativeImageEffect, NativeInputDescriptor, NativeNodeBinding, NativeNodeBindingDisposition,
    NativeNodeContractError, NativeNodePresentation, NativeOpaqueHandle, NativePrimitive,
    NativePrimitiveType, NativeResolvedPayload, NativeStoredModelPayload, NativeStoredPayload,
    NativeTypeUnion, NativeValue, NativeValueType, NodeDescriptor, NodeRegistry, PortDescriptor,
    generated_family_node_bindings, native_diffusion_descriptors, native_image_descriptors,
    native_source_type_projection,
};
use comfy_nodes::{
    NativeEffectServiceError, NativeImagePreviewError, NativeNodeServiceIdentity,
    NativeOutputEffectRequest, NativeOutputNamespace, NativeOutputShape, NativePreparedEffectKind,
    NativePreparedEffectService,
};
use comfy_sampler::{
    DiscreteSamplingProfile, GUIDANCE_ADAPTER_ID, GuidanceDenoiser, GuidanceError,
    GuidanceEvaluation, GuidanceOptions, GuidanceResult, INITIAL_NOISE_PHASE_ID,
    NativeConditioningPayload as PortableConditioningPayload,
    NativeControlExecution as PortableControlExecution, NativeDiffusionPayload, NoiseError,
    NoiseRequest, SamplingError, SamplingPlan, SamplingProfileError, SchedulerError,
    execute_guidance,
    generated_native_diffusion::{
        NativeDiffusionSamplerError, checked_native_diffusion_plan, normal_noise, normal_sigmas,
        sample_euler, scale_initial_noise, scale_model_input, sd15_interpret_prediction,
        sd15_model_time,
    },
};
use comfy_tensor::{
    BackendCapabilityMatrix, CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType,
    ExecutionContext, ImageTensor, NativeTensorPayload, NativeTensorRole, ResizeCrop, ResizeMode,
    RngCompatibilityError, RngError, ScratchReservation, StreamId, TensorError,
};
#[cfg(test)]
use comfy_tensor::{DeviceId, TensorDescriptor};
use comfy_tensor::{
    Tensor,
    generated_activation_normalization_functional_01::FunctionalError,
    generated_comfy_operator_indirection_01::OperatorIndirectionError,
    generated_external_tensor_kernel_01::ExternalTensorKernelPartOneError,
    generated_native_diffusion::{NativeDiffusionTensorError, tensor_to_f32},
    generated_neural_network_module_02::NeuralNetworkModulePartTwoError,
    generated_shape_layout_transform_01::ShapeLayoutTransformPartOneError,
    generated_shape_layout_transform_02::ShapeLayoutTransformPartTwoError,
};
use comfy_tensor::{
    generated_elementwise_or_runtime_operation_16::ElementwiseRuntimePartSixteenError,
    generated_indexing_masking_01::IndexingMaskingPartOneError,
};
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
    fmt,
    ops::Deref,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;
use uuid::Uuid;

pub const NATIVE_IMAGE_REGISTRY_VERSION: &str = "native-image-v1";
pub const NATIVE_DIFFUSION_REGISTRY_VERSION: &str = "native-diffusion-v1";
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
    model: Arc<Sd15TinyModel>,
    tokenizer: Arc<Sd1Tokenizer>,
    clip: Arc<LoadedSd1Clip>,
    vae: Arc<NativeVae>,
    conditioning: Arc<NativeConditioningExecution>,
    cache_identities: CanonicalNativeDiffusionCacheIdentities,
}

#[derive(Clone)]
pub struct PreboundControlExecution {
    chain: Arc<ControlChain>,
    executor: Arc<dyn ControlModelExecutor>,
    vae_execution_digest: Option<String>,
    execution_digest: String,
}

impl PreboundControlExecution {
    pub fn checked(
        chain: Arc<ControlChain>,
        executor: Arc<dyn ControlModelExecutor>,
    ) -> Result<Self, NativeImageRuntimeError> {
        Self::checked_inner(chain, executor, None)
    }

    pub fn checked_with_vae(
        chain: Arc<ControlChain>,
        executor: Arc<dyn ControlModelExecutor>,
        vae_execution_digest: impl Into<String>,
    ) -> Result<Self, NativeImageRuntimeError> {
        Self::checked_inner(chain, executor, Some(vae_execution_digest.into()))
    }

    fn checked_inner(
        chain: Arc<ControlChain>,
        executor: Arc<dyn ControlModelExecutor>,
        vae_execution_digest: Option<String>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let executor_digest = executor.execution_digest();
        let validate = |subject: &str, digest: &str| {
            if digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                Ok(())
            } else {
                Err(NativeImageRuntimeError::Registry(format!(
                    "native diffusion {subject} is not a SHA-256 digest"
                )))
            }
        };
        validate("ControlNet executor identity", executor_digest)?;
        chain
            .require_executor_digest(executor_digest)
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        if let Some(digest) = &vae_execution_digest {
            validate("ControlNet VAE execution identity", digest)?;
        }
        chain
            .require_vae_execution_digest(vae_execution_digest.as_deref())
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        let vae_binding_digest = vae_execution_digest.as_deref().map_or_else(
            || {
                sha256_tagged(
                    "sim.comfy.controlnet.vae-binding.absent.v1",
                    std::iter::empty(),
                )
            },
            |digest| {
                sha256_tagged(
                    "sim.comfy.controlnet.vae-binding.exact.v1",
                    [digest.as_bytes()],
                )
            },
        );
        let execution_digest = sha256_tagged(
            "sim.comfy.controlnet.prebound-execution.v1",
            [
                chain.identity().digest().as_bytes(),
                executor_digest.as_bytes(),
                vae_binding_digest.as_bytes(),
            ],
        );
        Ok(Self {
            chain,
            executor,
            vae_execution_digest,
            execution_digest,
        })
    }

    pub fn chain(&self) -> &Arc<ControlChain> {
        &self.chain
    }

    pub fn execution_digest(&self) -> &str {
        &self.execution_digest
    }
}

#[derive(Clone)]
pub struct NativeConditioningExecution {
    identity: ConditioningIdentity,
    patch_graph: Arc<PatchGraph>,
    control: Option<PreboundControlExecution>,
    cache_identities: CanonicalConditioningCacheIdentities,
}

impl NativeConditioningExecution {
    pub fn cache_identities_for_sd15(
        patch_identity: &comfy_model::PatchGraphIdentity,
        model_execution_digest: &str,
        control_execution_digest: Option<&str>,
    ) -> Result<CanonicalConditioningCacheIdentities, NativeImageRuntimeError> {
        let identity = ConditioningIdentity::new(
            "sd15-native-diffusion",
            sd15_model_family_identity()
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
            sd15_latent_format_identity()
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
        )
        .map_err(map_conditioning_runtime_error)?;
        let conditioning_digest = sha256_serialized("conditioning ABI", &identity)?;
        let guidance_digest = sha256_tagged(
            "sim.comfy.guidance-adapter.identity.v1",
            [GUIDANCE_ADAPTER_ID.as_bytes()],
        );
        let control_digest = control_execution_digest.map_or_else(
            || sha256_tagged("sim.comfy.controlnet.absent.v1", std::iter::empty()),
            str::to_owned,
        );
        CanonicalConditioningCacheIdentities::checked(
            conditioning_digest,
            guidance_digest,
            &patch_identity.ordered_digest,
            model_execution_digest,
            control_digest,
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
    }

    pub fn checked(
        model_digest: &str,
        model: &Sd15TinyModel,
        patch_graph: Arc<PatchGraph>,
        control: Option<PreboundControlExecution>,
    ) -> Result<Self, NativeImageRuntimeError> {
        patch_graph
            .identity()
            .validate_for_base(model_digest)
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        if &patch_graph.identity() != model.patch_identity() {
            return Err(NativeImageRuntimeError::Registry(
                "native diffusion model patch identity does not match its prebound graph"
                    .to_owned(),
            ));
        }
        let identity = ConditioningIdentity::new(
            "sd15-native-diffusion",
            sd15_model_family_identity()
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
            sd15_latent_format_identity()
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
        )
        .map_err(map_conditioning_runtime_error)?;
        let cache_identities = Self::cache_identities_for_sd15(
            &patch_graph.identity(),
            model.patch_execution_digest(),
            control
                .as_ref()
                .map(PreboundControlExecution::execution_digest),
        )?;
        Ok(Self {
            identity,
            patch_graph,
            control,
            cache_identities,
        })
    }

    pub fn identity(&self) -> &ConditioningIdentity {
        &self.identity
    }

    pub fn patch_graph(&self) -> &Arc<PatchGraph> {
        &self.patch_graph
    }

    pub fn control(&self) -> Option<&PreboundControlExecution> {
        self.control.as_ref()
    }

    pub fn cache_identities(&self) -> &CanonicalConditioningCacheIdentities {
        &self.cache_identities
    }
}

fn sha256_serialized(
    subject: &str,
    value: &impl Serialize,
) -> Result<String, NativeImageRuntimeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| NativeImageRuntimeError::Encoding(format!("{subject}: {error}")))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn sha256_tagged<'a>(tag: &str, fields: impl IntoIterator<Item = &'a [u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tag.as_bytes());
    hasher.update([0]);
    for field in fields {
        hasher.update(field);
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn validate_native_diffusion_bundle_inputs(
    fixture_id: &str,
    model_digest: &str,
    tokenizer: &Sd1Tokenizer,
    clip: &LoadedSd1Clip,
) -> Result<CanonicalClipCacheIdentities, NativeImageRuntimeError> {
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
    CanonicalClipCacheIdentities::checked(
        tokenizer_identity.digest(),
        clip.architecture().digest(),
        clip.plan().artifact_identity().as_str(),
        clip.plan().model_identity().as_str(),
        clip.plan().patch_identity().as_str(),
        clip.plan().digest(),
    )
    .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
}

impl NativeDiffusionBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_vae(
        fixture_id: impl Into<String>,
        model_digest: impl Into<String>,
        model: Arc<Sd15TinyModel>,
        tokenizer: Arc<Sd1Tokenizer>,
        clip: Arc<LoadedSd1Clip>,
        vae: Arc<NativeVae>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let patch_graph = Arc::new(
            PatchGraph::checked_semantic(model_digest.into(), Vec::new())
                .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?,
        );
        let model_digest = patch_graph.identity().base_artifact_digest;
        let conditioning = Arc::new(NativeConditioningExecution::checked(
            &model_digest,
            &model,
            patch_graph,
            None,
        )?);
        Self::new_prebound(
            fixture_id,
            model_digest,
            model,
            tokenizer,
            clip,
            vae,
            conditioning,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_prebound(
        fixture_id: impl Into<String>,
        model_digest: impl Into<String>,
        model: Arc<Sd15TinyModel>,
        tokenizer: Arc<Sd1Tokenizer>,
        clip: Arc<LoadedSd1Clip>,
        vae: Arc<NativeVae>,
        conditioning: Arc<NativeConditioningExecution>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let fixture_id = fixture_id.into();
        let model_digest = model_digest.into();
        let clip_cache_identities =
            validate_native_diffusion_bundle_inputs(&fixture_id, &model_digest, &tokenizer, &clip)?;
        let tokenizer_identity = tokenizer.identity();
        let identity = vae.descriptor().identity();
        let family = sd15_model_family_identity()
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        let latent = sd15_latent_format_identity()
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        let invalid_vae = identity.artifact_sha256() != model_digest
            || identity.family() != &family
            || identity.latent_format() != &latent
            || identity.architecture().as_str()
                != "comfy.ldm.models.autoencoder.AutoencoderKL.reduced.v1"
            || identity.profile() != &VaeKernelProfile::Sd15AutoencoderKlReducedV1
            || identity.dtype() != DType::F32
            || identity.device() != comfy_tensor::DeviceId::CPU
            || identity.boundary().kind() != VaeBoundaryKind::Image
            || identity.boundary().channels() != 3
            || vae.descriptor().decode_clamp() != [0.0, 1.0];
        if invalid_vae {
            return Err(NativeImageRuntimeError::Registry(
                "native diffusion provider canonical VAE identity is invalid".to_owned(),
            ));
        }
        if conditioning.patch_graph().identity().base_artifact_digest != model_digest
            || conditioning.patch_graph().identity() != *model.patch_identity()
            || conditioning.cache_identities().model_execution() != model.patch_execution_digest()
            || conditioning.control().is_some_and(|control| {
                control
                    .vae_execution_digest
                    .as_deref()
                    .is_some_and(|digest| digest != vae.execution_digest())
            })
        {
            return Err(NativeImageRuntimeError::Registry(
                "native diffusion provider conditioning identity is invalid".to_owned(),
            ));
        }
        let vae_cache_identities = CanonicalVaeCacheIdentities::checked(
            identity.digest(),
            identity.artifact_sha256(),
            &identity.patch().ordered_digest,
            vae.execution_digest(),
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        let cache_identities = CanonicalNativeDiffusionCacheIdentities::checked(
            &model_digest,
            tokenizer_identity.digest(),
            clip_cache_identities,
            vae_cache_identities,
            conditioning.cache_identities().clone(),
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        Ok(Self {
            model,
            tokenizer,
            clip,
            vae,
            conditioning,
            cache_identities,
        })
    }

    pub fn model_digest(&self) -> &str {
        self.cache_identities.model_digest()
    }

    pub fn tokenizer_digest(&self) -> &str {
        self.cache_identities.tokenizer_digest()
    }

    pub fn model(&self) -> &Arc<Sd15TinyModel> {
        &self.model
    }

    pub fn clip(&self) -> &Arc<LoadedSd1Clip> {
        &self.clip
    }

    pub fn clip_cache_identities(&self) -> &CanonicalClipCacheIdentities {
        self.cache_identities.clip()
    }

    pub fn vae(&self) -> &Arc<NativeVae> {
        &self.vae
    }

    pub fn vae_cache_identities(&self) -> &CanonicalVaeCacheIdentities {
        self.cache_identities.vae()
    }

    pub fn conditioning(&self) -> &Arc<NativeConditioningExecution> {
        &self.conditioning
    }

    pub fn conditioning_cache_identities(&self) -> &CanonicalConditioningCacheIdentities {
        self.cache_identities.conditioning()
    }

    pub fn cache_identities(&self) -> &CanonicalNativeDiffusionCacheIdentities {
        &self.cache_identities
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
    let failure = match error {
        ClipError::Allocation(_) => resource_diffusion_failure(error.to_string()),
        ClipError::Tensor(error) => classified_diffusion_failure(error),
        ClipError::TensorOperation(error) => native_diffusion_tensor_failure(error),
        ClipError::Attention(error) => attention_failure(error),
        ClipError::ModelStore(error) => model_store_failure(error),
        ClipError::NativeModule(error) => native_ops_failure(error),
        ClipError::TextTransformer(error) => clip_text_failure(error),
        error => classified_diffusion_failure(error),
    };
    node_failure_runtime_error(failure)
}

struct Sd15GuidanceDenoiser<'a> {
    model: &'a Sd15TinyModel,
    conditioning: Option<&'a PortableConditioningPayload>,
    runtime_failure: Option<NativeImageRuntimeError>,
}

fn map_conditioning_runtime_error(error: ConditioningError) -> NativeImageRuntimeError {
    match error {
        ConditioningError::Cancelled => NativeImageRuntimeError::Cancelled,
        ConditioningError::Tensor(error) => classified_runtime_error(error),
        ConditioningError::Narrow(IndexingMaskingPartOneError::Cancelled) => {
            NativeImageRuntimeError::Cancelled
        }
        ConditioningError::Narrow(IndexingMaskingPartOneError::Tensor(error)) => {
            classified_runtime_error(error)
        }
        ConditioningError::ShapeView(ShapeLayoutTransformPartOneError::Cancelled)
        | ConditioningError::TensorResize(ExternalTensorKernelPartOneError::Cancelled) => {
            NativeImageRuntimeError::Cancelled
        }
        ConditioningError::ShapeOperation(ShapeLayoutTransformPartTwoError::Cancelled)
        | ConditioningError::TensorCast(OperatorIndirectionError::Cancelled) => {
            NativeImageRuntimeError::Cancelled
        }
        error => classified_runtime_error(error),
    }
}

fn map_guidance_runtime_error(error: GuidanceError) -> NativeImageRuntimeError {
    match error {
        GuidanceError::Cancelled => NativeImageRuntimeError::Cancelled,
        GuidanceError::Tensor(error) => classified_runtime_error(error),
        GuidanceError::Conditioning(error) => map_conditioning_runtime_error(error),
        GuidanceError::NativeDiffusion(NativeDiffusionTensorError::Tensor(error)) => {
            classified_runtime_error(error)
        }
        GuidanceError::Profile(SamplingProfileError::OutOfMemory(_))
        | GuidanceError::BatchMemoryLimit { .. } => {
            NativeImageRuntimeError::ResourceExhausted(error.to_string())
        }
        GuidanceError::Indexing(IndexingMaskingPartOneError::Cancelled)
        | GuidanceError::Elementwise(ElementwiseRuntimePartSixteenError::Cancelled) => {
            NativeImageRuntimeError::Cancelled
        }
        GuidanceError::Indexing(IndexingMaskingPartOneError::Tensor(error))
        | GuidanceError::Elementwise(ElementwiseRuntimePartSixteenError::Tensor(error)) => {
            classified_runtime_error(error)
        }
        error => classified_runtime_error(error),
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
            let control = match self.conditioning {
                Some(execution) => execution
                    .execute_control(
                        self.model.backend(),
                        evaluation.latent(),
                        model_time,
                        conditioning,
                        evaluation.sampling_percent(),
                        context,
                    )
                    .map_err(|error| {
                        let message = error.to_string();
                        self.runtime_failure = Some(control_runtime_error(error));
                        GuidanceError::Invalid(message)
                    })?,
                None => None,
            };
            let output = self
                .model
                .denoise_at_model_time_with_control(
                    evaluation.latent(),
                    model_time,
                    conditioning,
                    control.as_ref(),
                    context,
                )
                .map_err(|error| {
                    let message = format!("SD15 denoiser failed: {error}");
                    self.runtime_failure = Some(native_diffusion_model_runtime_error(error));
                    GuidanceError::Invalid(message)
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
            denoiser: Sd15GuidanceDenoiser {
                model,
                conditioning: None,
                runtime_failure: None,
            },
        })
    }

    pub fn checked_prebound(
        model: &'a Sd15TinyModel,
        execution: &'a PortableConditioningPayload,
        positive: &Tensor,
        negative: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let mut adapter = Self::checked(model, positive, negative, context)?;
        adapter.denoiser.conditioning = Some(execution);
        Ok(adapter)
    }

    pub fn checked_prebound_conditioning_sets(
        model: &'a Sd15TinyModel,
        execution: &'a PortableConditioningPayload,
        positive: &ConditioningSet,
        negative: &ConditioningSet,
    ) -> Result<Self, NativeImageRuntimeError> {
        positive
            .validate()
            .map_err(map_conditioning_runtime_error)?;
        negative
            .validate()
            .map_err(map_conditioning_runtime_error)?;
        Ok(Self {
            profile: DiscreteSamplingProfile::sd15()
                .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?,
            positive: positive.clone(),
            negative: negative.clone(),
            denoiser: Sd15GuidanceDenoiser {
                model,
                conditioning: Some(execution),
                runtime_failure: None,
            },
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
        self.denoiser.runtime_failure = None;
        let result = execute_guidance(
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
        );
        match result {
            Ok(result) => Ok(result),
            Err(error) => {
                if let Some(runtime_failure) = self.denoiser.runtime_failure.take() {
                    Err(runtime_failure)
                } else {
                    Err(map_guidance_runtime_error(error))
                }
            }
        }
    }
}

pub trait NativeDiffusionProvider: Send + Sync {
    fn cache_identities(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<CanonicalNativeDiffusionCacheIdentities, NativeImageRuntimeError>;

    fn load(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionBundle, NativeImageRuntimeError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeProviderRegistryPin {
    generation: u64,
    registry_digest_sha256: String,
    binding_digests_sha256: Vec<String>,
}

impl NativeProviderRegistryPin {
    pub fn checked(
        generation: u64,
        registry_digest_sha256: impl Into<String>,
        binding_digests_sha256: Vec<String>,
    ) -> Result<Self, NativeImageRuntimeError> {
        let pin = Self {
            generation,
            registry_digest_sha256: registry_digest_sha256.into(),
            binding_digests_sha256,
        };
        pin.validate()?;
        Ok(pin)
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn registry_digest_sha256(&self) -> &str {
        &self.registry_digest_sha256
    }

    pub fn binding_digests_sha256(&self) -> &[String] {
        &self.binding_digests_sha256
    }

    pub fn identity_sha256(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(b"sim-native-provider-registry-pin-v1\0");
        digest.update(self.generation.to_le_bytes());
        digest.update(self.registry_digest_sha256.as_bytes());
        for binding_digest in &self.binding_digests_sha256 {
            digest.update([0]);
            digest.update(binding_digest.as_bytes());
        }
        format!("{:x}", digest.finalize())
    }

    pub fn validate(&self) -> Result<(), NativeImageRuntimeError> {
        if self.generation == 0
            || !valid_provider_registry_digest(&self.registry_digest_sha256)
            || self.binding_digests_sha256.is_empty()
            || self
                .binding_digests_sha256
                .iter()
                .any(|digest| !valid_provider_registry_digest(digest))
            || self
                .binding_digests_sha256
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(NativeImageRuntimeError::Encoding(
                "native provider registry pin is invalid".to_owned(),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_registry: Option<NativeProviderRegistryPin>,
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
        plan.validate_provider_execution_identity()?;
        let provider_registry = plan.provider_registry_pin().cloned();
        let value = Self {
            plan,
            input_assets,
            memory_policy,
            metadata_enabled,
            injected_delay_millis,
            provider_registry,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), NativeImageRuntimeError> {
        self.plan.validate_provider_execution_identity()?;
        if let Some(provider_registry) = &self.provider_registry {
            provider_registry.validate()?;
        }
        if self.provider_registry.as_ref() != self.plan.provider_registry_pin() {
            return Err(NativeImageRuntimeError::Encoding(
                "native worker provider registry must be derived from the compiled plan".to_owned(),
            ));
        }
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

fn valid_provider_registry_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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

fn validate_native_worker_ui_value(value: &Value) -> Result<(), String> {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => {
                for (key, value) in values {
                    if [
                        "service_id",
                        "reference_id",
                        "request_digest_sha256",
                        "scratch_binding",
                        "backend_id",
                        "authority_id",
                    ]
                    .iter()
                    .any(|reserved| key.eq_ignore_ascii_case(reserved))
                    {
                        return Err(format!(
                            "native worker UI output contains private capability key `{key}`"
                        ));
                    }
                    pending.push(value);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    Ok(())
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
            validate_native_worker_ui_value(&value).map_err(NativeImageRuntimeError::Encoding)?;
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
        report.outputs.clear();
        report.events.clear();
        drop(report.handle_lease.take());
        Ok(Self {
            report,
            output_proposal_ids,
            executed_node_count,
            encoded_ui_outputs,
        })
    }

    pub fn decode_ui_outputs(&self) -> Result<BTreeMap<NodeId, Value>, NativeImageRuntimeError> {
        if !self.report.outputs.is_empty() || !self.report.events.is_empty() {
            return Err(NativeImageRuntimeError::WorkerEvent(
                "native worker result carried process-local outputs or events".to_owned(),
            ));
        }
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
            validate_native_worker_ui_value(&value)
                .map_err(NativeImageRuntimeError::WorkerEvent)?;
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
    #[error("native image execution exhausted resources: {0}")]
    ResourceExhausted(String),
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

impl From<NativeNodeContractError> for NativeImageRuntimeError {
    fn from(error: NativeNodeContractError) -> Self {
        Self::Registry(error.to_string())
    }
}

impl From<NativeHandleStoreError> for NativeImageRuntimeError {
    fn from(error: NativeHandleStoreError) -> Self {
        Self::Handle(error.to_string())
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
        cpu_backend,
        true,
    )
}

pub fn generated_native_node_registry_projection(
    diffusion_provider: Option<Arc<dyn NativeDiffusionProvider>>,
) -> Result<NativeNodeRegistry, NativeImageRuntimeError> {
    let cpu_backend = projection_only_cpu_backend()?;
    let mut registry = if let Some(provider) = diffusion_provider {
        native_registry_with_diffusion_provider(
            Arc::new(Mutex::new(BTreeMap::new())),
            cpu_backend,
            true,
            provider,
        )?
    } else {
        native_image_registry_with_execution_state(
            Arc::new(Mutex::new(BTreeMap::new())),
            cpu_backend,
            true,
        )?
    };
    register_generated_family_bindings(&mut registry)?;
    Ok(registry)
}

fn register_generated_family_bindings(
    registry: &mut NativeNodeRegistry,
) -> Result<(), NativeImageRuntimeError> {
    registry
        .register_native_bindings(generated_family_node_bindings()?)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
    registry
        .validate_comprehensive_bindings()
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
}

pub fn compile_generated_native_prompt(
    submission: comfy_types::PromptSubmission,
    diffusion_provider: Option<Arc<dyn NativeDiffusionProvider>>,
) -> Result<CompiledPlan, NativeImageRuntimeError> {
    let registry = generated_native_node_registry_projection(diffusion_provider)?;
    Ok(crate::PromptCompiler::new(&registry).compile(submission)?)
}

pub fn generated_native_frontend_descriptors(
    diffusion_provider: Option<Arc<dyn NativeDiffusionProvider>>,
) -> Result<BTreeMap<String, NodeDescriptor>, NativeImageRuntimeError> {
    Ok(generated_native_frontend_contracts(diffusion_provider)?
        .into_iter()
        .map(|(class_type, contract)| (class_type, contract.graph))
        .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedNativeFrontendDescriptor {
    pub graph: NodeDescriptor,
    pub runtime: RuntimeNodeDescriptor,
    pub presentation: NativeNodePresentation,
    pub disposition: NativeNodeBindingDisposition,
    pub unavailable_reason: Option<String>,
}

pub fn generated_native_frontend_contracts(
    diffusion_provider: Option<Arc<dyn NativeDiffusionProvider>>,
) -> Result<BTreeMap<String, GeneratedNativeFrontendDescriptor>, NativeImageRuntimeError> {
    let registry = generated_native_node_registry_projection(diffusion_provider)?;
    registry
        .descriptors()
        .map(|(class_type, descriptor)| {
            descriptor.validate_exact_schema_v2().map_err(|error| {
                NativeImageRuntimeError::Registry(format!(
                    "native frontend node `{class_type}` lacks exact schema v2: {error}"
                ))
            })?;
            let presentation = registry.presentation(class_type).ok_or_else(|| {
                NativeImageRuntimeError::Registry(format!(
                    "native node `{class_type}` has no presentation projection"
                ))
            })?;
            let inputs = descriptor
                .inputs
                .iter()
                .map(|input| {
                    Ok(PortDescriptor {
                        name: input.name.clone(),
                        type_name: frontend_type_name(input.accepted_types.members())?,
                        required: input.required,
                    })
                })
                .collect::<Result<Vec<_>, NativeImageRuntimeError>>()?;
            let outputs = descriptor
                .outputs
                .iter()
                .map(|output| {
                    Ok(PortDescriptor {
                        name: output.name.clone(),
                        type_name: frontend_type_name(std::slice::from_ref(&output.produced_type))?,
                        required: true,
                    })
                })
                .collect::<Result<Vec<_>, NativeImageRuntimeError>>()?;
            Ok((
                class_type.to_owned(),
                GeneratedNativeFrontendDescriptor {
                    graph: NodeDescriptor {
                        type_name: class_type.to_owned(),
                        display_name: presentation.display_name.clone(),
                        inputs,
                        outputs,
                    },
                    runtime: descriptor.clone(),
                    presentation: presentation.clone(),
                    disposition: registry.binding_disposition(class_type).ok_or_else(|| {
                        NativeImageRuntimeError::Registry(format!(
                            "native node `{class_type}` has no binding disposition"
                        ))
                    })?,
                    unavailable_reason: registry
                        .unavailable_reason(class_type)
                        .map(ToOwned::to_owned),
                },
            ))
        })
        .collect()
}

fn frontend_type_name(types: &[NativeValueType]) -> Result<String, NativeImageRuntimeError> {
    if types.is_empty() {
        return Err(NativeImageRuntimeError::Registry(
            "frontend projection received an empty runtime type union".to_owned(),
        ));
    }
    let project = |value_type: &NativeValueType| match value_type {
        NativeValueType::Any => "ANY".to_owned(),
        NativeValueType::Primitive(NativePrimitiveType::Null) => "NULL".to_owned(),
        NativeValueType::Primitive(NativePrimitiveType::Boolean) => "BOOLEAN".to_owned(),
        NativeValueType::Primitive(NativePrimitiveType::Integer) => "INT".to_owned(),
        NativeValueType::Primitive(NativePrimitiveType::Number) => "FLOAT".to_owned(),
        NativeValueType::Primitive(NativePrimitiveType::String) => "STRING".to_owned(),
        NativeValueType::Handle(handle_type) => handle_type.type_id.clone(),
        NativeValueType::PreservedUnknown => "UNKNOWN".to_owned(),
        NativeValueType::NamedPreservedUnknown(type_name) => type_name.clone(),
    };
    let projected = types.iter().map(project).collect::<Vec<_>>();
    Ok(projected.join("|"))
}

fn projection_only_cpu_backend() -> Result<Arc<CpuBackend>, NativeImageRuntimeError> {
    let (backend, authority) =
        CpuWorkspaceAuthority::create_backend(DEFAULT_NATIVE_IMAGE_MEMORY_LIMIT_BYTES)?;
    drop(authority);
    Ok(Arc::new(backend))
}

fn native_image_registry_with_execution_state(
    input_assets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    cpu_backend: Arc<CpuBackend>,
    metadata_enabled: bool,
) -> Result<NativeNodeRegistry, NativeImageRuntimeError> {
    let mut registry = NativeNodeRegistry::default();
    let nodes = BTreeMap::<String, Arc<dyn NativeNode>>::from([
        (
            "LoadImage".to_owned(),
            Arc::new(LoadImageNode {
                input_assets,
                cpu_backend: cpu_backend.clone(),
            }) as Arc<dyn NativeNode>,
        ),
        (
            "ImageScale".to_owned(),
            Arc::new(ImageScaleNode {
                cpu_backend: cpu_backend.clone(),
            }) as Arc<dyn NativeNode>,
        ),
        (
            "ImageInvert".to_owned(),
            Arc::new(ImageInvertNode {
                cpu_backend: cpu_backend.clone(),
            }) as Arc<dyn NativeNode>,
        ),
        (
            "PreviewImage".to_owned(),
            Arc::new(SaveImageNode {
                cpu_backend: cpu_backend.clone(),
                namespace: AssetNamespace::Temporary,
                metadata_enabled,
            }) as Arc<dyn NativeNode>,
        ),
        (
            "SaveImage".to_owned(),
            Arc::new(SaveImageNode {
                cpu_backend,
                namespace: AssetNamespace::Output,
                metadata_enabled,
            }) as Arc<dyn NativeNode>,
        ),
    ]);
    let bindings = native_image_descriptors()?
        .iter()
        .map(|descriptor| {
            let node = nodes.get(&descriptor.class_type).cloned().ok_or_else(|| {
                NativeImageRuntimeError::Registry(format!(
                    "native image implementation `{}` is absent",
                    descriptor.class_type
                ))
            })?;
            native_executable_binding(descriptor, node)
        })
        .collect::<Result<Vec<_>, _>>()?;
    registry
        .register_native_bindings(bindings)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
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
    cpu_backend: Arc<CpuBackend>,
    metadata_enabled: bool,
    provider: Arc<dyn NativeDiffusionProvider>,
) -> Result<NativeNodeRegistry, NativeImageRuntimeError> {
    let mut registry = native_image_registry_with_execution_state(
        input_assets,
        cpu_backend.clone(),
        metadata_enabled,
    )?;
    let cancellation = CancellationToken::default();
    let state = Arc::new(NativeDiffusionState::checked(
        provider,
        cpu_backend.clone(),
        &cancellation,
    )?);
    let nodes = BTreeMap::<String, Arc<dyn NativeNode>>::from([
        (
            "CheckpointLoaderSimple".to_owned(),
            Arc::new(CheckpointLoaderNode {
                state: state.clone(),
            }) as Arc<dyn NativeNode>,
        ),
        (
            "CLIPTextEncode".to_owned(),
            Arc::new(ClipTextEncodeNode {
                state: state.clone(),
            }) as Arc<dyn NativeNode>,
        ),
        (
            "EmptyLatentImage".to_owned(),
            Arc::new(EmptyLatentNode {
                backend: cpu_backend,
            }) as Arc<dyn NativeNode>,
        ),
        (
            "KSampler".to_owned(),
            Arc::new(KSamplerNode {
                state: state.clone(),
            }) as Arc<dyn NativeNode>,
        ),
        (
            "VAEDecode".to_owned(),
            Arc::new(VaeDecodeNode { state }) as Arc<dyn NativeNode>,
        ),
    ]);
    let bindings = native_diffusion_descriptors()?
        .iter()
        .map(|descriptor| {
            let node = nodes.get(&descriptor.class_type).cloned().ok_or_else(|| {
                NativeImageRuntimeError::Registry(format!(
                    "native diffusion implementation `{}` is absent",
                    descriptor.class_type
                ))
            })?;
            native_executable_binding(descriptor, node)
        })
        .collect::<Result<Vec<_>, _>>()?;
    registry
        .register_native_bindings(bindings)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
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
            let value_type = native_value_type(&port.type_name)?;
            let allows_literal = matches!(
                value_type,
                NativeValueType::Any | NativeValueType::Primitive(_)
            );
            Ok(NativeInputDescriptor {
                name: port.name.clone(),
                accepted_types: NativeTypeUnion::new([value_type])?,
                required: port.required,
                hidden: port.hidden,
                lazy: false,
                cardinality: InputMode::Scalar,
                allows_literal,
            })
        })
        .collect::<Result<Vec<_>, NativeImageRuntimeError>>()?;
    let outputs = descriptor
        .outputs
        .iter()
        .map(|port| {
            Ok(RuntimeOutputDescriptor {
                name: port.name.clone(),
                produced_type: native_value_type(&port.type_name)?,
                is_list: false,
            })
        })
        .collect::<Result<Vec<_>, NativeImageRuntimeError>>()?;
    let input_names = descriptor
        .inputs
        .iter()
        .map(|port| port.name.clone())
        .collect::<Vec<_>>();
    let output_names = descriptor
        .outputs
        .iter()
        .map(|port| port.name.clone())
        .collect::<Vec<_>>();
    let source_schema = comfy_nodes::built_in_source_schema(&descriptor.class_type)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?
        .bind_execution_ports(&input_names, &[], &output_names)
        .map_err(|error| {
            NativeImageRuntimeError::Registry(format!(
                "{} source schema: {error}",
                descriptor.class_type
            ))
        })?;
    let runtime = RuntimeNodeDescriptor {
        schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
        class_type: descriptor.class_type.clone(),
        implementation_version: descriptor.implementation_version.clone(),
        source_schema: Some(source_schema),
        inputs,
        dynamic_inputs: Vec::new(),
        outputs,
        output_node: descriptor.output_node,
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
    };
    runtime.validate()?;
    Ok(runtime)
}

fn native_executable_binding(
    descriptor: &NativeImageDescriptor,
    node: Arc<dyn NativeNode>,
) -> Result<NativeNodeBinding, NativeImageRuntimeError> {
    let catalog = NodeRegistry::built_in()
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
    let catalog = catalog.descriptor(&descriptor.class_type).ok_or_else(|| {
        NativeImageRuntimeError::Registry(format!(
            "native node `{}` has no catalog descriptor",
            descriptor.class_type
        ))
    })?;
    Ok(NativeNodeBinding::Executable {
        feature_id: catalog.feature_id.clone(),
        descriptor: runtime_descriptor(descriptor)?,
        presentation: NativeNodePresentation {
            display_name: descriptor.display_name.clone(),
            category: descriptor.category.clone(),
            description: descriptor.description.clone(),
            output_names: descriptor
                .outputs
                .iter()
                .map(|output| output.name.clone())
                .collect(),
            search_aliases: descriptor.search_aliases.clone(),
            is_deprecated: false,
            is_experimental: false,
        },
        node,
    })
}

fn native_value_type(type_name: &str) -> Result<NativeValueType, NativeImageRuntimeError> {
    if type_name == "TENSOR" {
        return Ok(NativeValueType::Handle(NativeHandleType::new(
            NativeHandleKind::Tensor,
            type_name,
        )?));
    }
    native_source_type_projection(type_name)
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?
        .value_type()
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
}

fn native_tensor_role(kind: NativeTensorKind) -> NativeTensorRole {
    match kind {
        NativeTensorKind::Image => NativeTensorRole::Image,
        NativeTensorKind::Mask => NativeTensorRole::Mask,
        NativeTensorKind::Conditioning => NativeTensorRole::Conditioning,
        NativeTensorKind::Latent => NativeTensorRole::Latent,
    }
}

fn native_tensor_handle_type(
    kind: NativeTensorKind,
) -> Result<NativeHandleType, NativeImageRuntimeError> {
    let (kind, type_id) = match kind {
        NativeTensorKind::Image => (NativeHandleKind::Image, "IMAGE"),
        NativeTensorKind::Mask => (NativeHandleKind::Mask, "MASK"),
        NativeTensorKind::Conditioning => (NativeHandleKind::Conditioning, "CONDITIONING"),
        NativeTensorKind::Latent => (NativeHandleKind::Latent, "LATENT"),
    };
    Ok(NativeHandleType::new(kind, type_id)?)
}

fn publish_image(
    context: &NodeContext,
    kind: NativeTensorKind,
    tensor: ImageTensor,
) -> Result<NativeValue, NodeFailure> {
    let payload = NativeTensorPayload::from_image(native_tensor_role(kind), tensor)
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    let handle = context
        .handle_store()
        .publish(
            NativeStoredPayload::Tensor(Arc::new(payload)),
            &context.cancellation,
        )
        .map_err(|error| runtime_failure(error.into()))?;
    Ok(NativeValue::Handle { value: handle })
}

fn required_opaque_handle<'a>(
    inputs: &'a BTreeMap<String, NativeValue>,
    name: &str,
) -> Result<&'a NativeOpaqueHandle, NodeFailure> {
    match inputs.get(name) {
        Some(NativeValue::Handle { value }) => Ok(value),
        Some(_) => Err(NodeFailure {
            code: "invalid_native_handle".to_owned(),
            message: format!("`{name}` must be an opaque native handle"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
        None => Err(NodeFailure {
            code: "missing_native_handle".to_owned(),
            message: format!("`{name}` is missing"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
    }
}

struct ResolvedNative<Value> {
    value: Value,
    _resolved_payload: NativeResolvedPayload,
}

impl<Value> Deref for ResolvedNative<Value> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

fn resolve_image(
    context: &NodeContext,
    inputs: &BTreeMap<String, NativeValue>,
    name: &str,
    expected_kind: NativeTensorKind,
) -> Result<ResolvedNative<Arc<ImageTensor>>, NodeFailure> {
    let handle = required_opaque_handle(inputs, name)?;
    let expected_type = native_tensor_handle_type(expected_kind).map_err(runtime_failure)?;
    let stored = context
        .handle_store()
        .resolve(handle, &expected_type, &context.cancellation)
        .map_err(|error| runtime_failure(error.into()))?;
    let NativeStoredPayload::Tensor(payload) = stored.as_ref() else {
        return Err(invalid_diffusion_input(
            "native image handle stored a raw tensor object",
        ));
    };
    if payload.role() != native_tensor_role(expected_kind) {
        return Err(runtime_failure(NativeImageRuntimeError::Handle(
            "image tensor handle kind or schema is invalid".to_owned(),
        )));
    }
    let image = payload.image().ok_or_else(|| {
        invalid_diffusion_input("native image handle stored a non-image tensor payload")
    })?;
    Ok(ResolvedNative {
        value: Arc::new(image.clone()),
        _resolved_payload: stored,
    })
}

fn publish_tensor(
    context: &NodeContext,
    kind: NativeTensorKind,
    tensor: Tensor,
) -> Result<NativeValue, NodeFailure> {
    let payload = NativeTensorPayload::from_tensor(native_tensor_role(kind), tensor)
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    let handle = context
        .handle_store()
        .publish(
            NativeStoredPayload::Tensor(Arc::new(payload)),
            &context.cancellation,
        )
        .map_err(|error| runtime_failure(error.into()))?;
    Ok(NativeValue::Handle { value: handle })
}

fn publish_conditioning(
    context: &NodeContext,
    conditioning: ConditioningSet,
) -> Result<NativeValue, NodeFailure> {
    conditioning
        .validate()
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    let handle = context
        .handle_store()
        .publish(
            NativeStoredPayload::Conditioning(Arc::new(conditioning)),
            &context.cancellation,
        )
        .map_err(|error| runtime_failure(error.into()))?;
    Ok(NativeValue::Handle { value: handle })
}

fn resolve_conditioning(
    context: &NodeContext,
    inputs: &BTreeMap<String, NativeValue>,
    name: &str,
) -> Result<ResolvedNative<Arc<ConditioningSet>>, NodeFailure> {
    let handle = required_opaque_handle(inputs, name)?;
    let expected_type = NativeHandleType::new(NativeHandleKind::Conditioning, "CONDITIONING")
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    let stored = context
        .handle_store()
        .resolve(handle, &expected_type, &context.cancellation)
        .map_err(|error| runtime_failure(error.into()))?;
    let NativeStoredPayload::Conditioning(payload) = stored.as_ref() else {
        return Err(invalid_diffusion_input(
            "native CONDITIONING handle stored a non-conditioning payload",
        ));
    };
    payload
        .validate()
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    Ok(ResolvedNative {
        value: payload.clone(),
        _resolved_payload: stored,
    })
}

fn resolve_tensor(
    context: &NodeContext,
    inputs: &BTreeMap<String, NativeValue>,
    name: &str,
    expected_kind: NativeTensorKind,
) -> Result<ResolvedNative<Tensor>, NodeFailure> {
    let handle = required_opaque_handle(inputs, name)?;
    let expected_type = native_tensor_handle_type(expected_kind).map_err(runtime_failure)?;
    let stored = context
        .handle_store()
        .resolve(handle, &expected_type, &context.cancellation)
        .map_err(|error| runtime_failure(error.into()))?;
    let NativeStoredPayload::Tensor(payload) = stored.as_ref() else {
        return Err(invalid_diffusion_input(
            "native tensor handle stored an image object",
        ));
    };
    if payload.role() != native_tensor_role(expected_kind) || payload.image().is_some() {
        return Err(runtime_failure(NativeImageRuntimeError::Handle(
            "raw tensor handle kind or schema is invalid".to_owned(),
        )));
    }
    Ok(ResolvedNative {
        value: payload.tensor().clone(),
        _resolved_payload: stored,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NativeDiffusionRole {
    Model,
    Clip,
    Vae,
}

struct NativeDiffusionState {
    provider: Arc<dyn NativeDiffusionProvider>,
    backend: Arc<CpuBackend>,
    admitted_identities: CanonicalNativeDiffusionCacheIdentities,
}

fn native_diffusion_handle_type(
    role: NativeDiffusionRole,
) -> Result<NativeHandleType, NativeImageRuntimeError> {
    let (kind, type_id) = match role {
        NativeDiffusionRole::Model => (NativeHandleKind::Model, "MODEL"),
        NativeDiffusionRole::Clip => (NativeHandleKind::Clip, "CLIP"),
        NativeDiffusionRole::Vae => (NativeHandleKind::Vae, "VAE"),
    };
    Ok(NativeHandleType::new(kind, type_id)?)
}

fn portable_diffusion_payload(
    bundle: &NativeDiffusionBundle,
    role: NativeDiffusionRole,
) -> Result<NativeDiffusionPayload, NativeImageRuntimeError> {
    let payload = match role {
        NativeDiffusionRole::Model => {
            let model = Arc::new(
                NativeModelPayload::sd15_model(bundle.model.clone())
                    .map_err(|error| NativeImageRuntimeError::Handle(error.to_string()))?,
            );
            let control = bundle
                .conditioning
                .control
                .as_ref()
                .map(|control| {
                    if control.vae_execution_digest.is_some() {
                        PortableControlExecution::checked_with_vae(
                            control.chain.clone(),
                            control.executor.clone(),
                            bundle.vae.clone(),
                        )
                    } else {
                        PortableControlExecution::checked(
                            control.chain.clone(),
                            control.executor.clone(),
                        )
                    }
                })
                .transpose()
                .map_err(|error| NativeImageRuntimeError::Handle(error.to_string()))?;
            let conditioning = Arc::new(
                PortableConditioningPayload::checked_sd15(
                    bundle.model_digest(),
                    bundle.model.as_ref(),
                    bundle.conditioning.patch_graph.clone(),
                    control,
                )
                .map_err(|error| NativeImageRuntimeError::Handle(error.to_string()))?,
            );
            NativeDiffusionPayload::model(model, conditioning)
        }
        NativeDiffusionRole::Clip => NativeDiffusionPayload::clip(Arc::new(
            NativeModelPayload::sd1_clip(bundle.tokenizer.clone(), bundle.clip.clone())
                .map_err(|error| NativeImageRuntimeError::Handle(error.to_string()))?,
        )),
        NativeDiffusionRole::Vae => NativeDiffusionPayload::vae(Arc::new(
            NativeModelPayload::native_vae(bundle.vae.clone())
                .map_err(|error| NativeImageRuntimeError::Handle(error.to_string()))?,
        )),
    };
    payload.map_err(|error| NativeImageRuntimeError::Handle(error.to_string()))
}

fn publish_diffusion_bundle(
    bundle: &NativeDiffusionBundle,
    context: &NodeContext,
    role: NativeDiffusionRole,
) -> Result<NativeValue, NodeFailure> {
    let payload = portable_diffusion_payload(bundle, role).map_err(runtime_failure)?;
    let payload = NativeStoredModelPayload::native_diffusion(Arc::new(payload))
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    let handle = context
        .handle_store()
        .publish(
            NativeStoredPayload::Model(Arc::new(payload)),
            &context.cancellation,
        )
        .map_err(|error| runtime_failure(error.into()))?;
    Ok(NativeValue::Handle { value: handle })
}

fn resolve_diffusion_bundle(
    context: &NodeContext,
    inputs: &BTreeMap<String, NativeValue>,
    name: &str,
    role: NativeDiffusionRole,
) -> Result<ResolvedNative<Arc<NativeDiffusionPayload>>, NodeFailure> {
    let handle = required_opaque_handle(inputs, name)?;
    let expected_type = native_diffusion_handle_type(role).map_err(runtime_failure)?;
    let stored = context
        .handle_store()
        .resolve(handle, &expected_type, &context.cancellation)
        .map_err(|error| runtime_failure(error.into()))?;
    let NativeStoredPayload::Model(payload) = stored.as_ref() else {
        return Err(invalid_diffusion_input(
            "native diffusion handle stored the wrong object type",
        ));
    };
    let payload = payload.diffusion().ok_or_else(|| {
        invalid_diffusion_input("native diffusion handle stored a non-diffusion model payload")
    })?;
    if payload.role().source_type_id() != expected_type.type_id {
        return Err(invalid_diffusion_input(
            "native diffusion handle stored the wrong resource role",
        ));
    }
    payload
        .validate()
        .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
    Ok(ResolvedNative {
        value: payload.clone(),
        _resolved_payload: stored,
    })
}

impl NativeDiffusionState {
    fn checked(
        provider: Arc<dyn NativeDiffusionProvider>,
        backend: Arc<CpuBackend>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeImageRuntimeError> {
        let admitted_identities = provider.cache_identities(cancellation)?;
        Ok(Self {
            provider,
            backend,
            admitted_identities,
        })
    }

    fn validate_current_provider_identity(
        &self,
        current: &CanonicalNativeDiffusionCacheIdentities,
    ) -> Result<(), NativeImageRuntimeError> {
        if current.model_digest() != self.admitted_identities.model_digest()
            || current.tokenizer_digest() != self.admitted_identities.tokenizer_digest()
            || current.clip() != self.admitted_identities.clip()
            || current.vae() != self.admitted_identities.vae()
        {
            return Err(NativeImageRuntimeError::Registry(
                "native diffusion provider cache identity does not match its admitted identity"
                    .to_owned(),
            ));
        }
        current
            .conditioning()
            .require_exact_match(self.admitted_identities.conditioning())
            .map_err(|_| {
                NativeImageRuntimeError::Registry(
                    "native diffusion provider conditioning identity does not match its loaded bundle"
                        .to_owned(),
                )
            })
    }

    fn validate_provider_bundle(
        &self,
        bundle: &NativeDiffusionBundle,
    ) -> Result<(), NativeImageRuntimeError> {
        if self.admitted_identities.model_digest() != bundle.model_digest()
            || self.admitted_identities.tokenizer_digest() != bundle.tokenizer_digest()
            || self.admitted_identities.clip() != bundle.clip_cache_identities()
            || self.admitted_identities.vae() != bundle.vae_cache_identities()
        {
            return Err(NativeImageRuntimeError::Registry(
                "native diffusion provider cache identity does not match its loaded bundle"
                    .to_owned(),
            ));
        }
        self.admitted_identities
            .conditioning()
            .require_exact_match(bundle.conditioning_cache_identities())
            .map_err(|_| {
                NativeImageRuntimeError::Registry(
                    "native diffusion provider conditioning identity does not match its loaded bundle"
                        .to_owned(),
                )
            })?;
        Ok(())
    }

    fn load_bundle(
        &self,
        context: &ExecutionContext<'_>,
    ) -> Result<Arc<NativeDiffusionBundle>, NativeImageRuntimeError> {
        let current = self.provider.cache_identities(context.cancellation)?;
        self.validate_current_provider_identity(&current)?;
        let bundle = Arc::new(self.provider.load(self.backend.clone(), context)?);
        context.check()?;
        self.validate_provider_bundle(&bundle)?;
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
        context: &NodeContext,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<CacheDependencies, NodeFailure> {
        let identities = self
            .state
            .provider
            .cache_identities(&context.cancellation)
            .map_err(runtime_failure)?;
        Ok(CacheDependencies {
            artifact_digests: identities.artifact_digests(),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            if required_string(&inputs, "ckpt_name")? != "model.safetensors" {
                return Err(invalid_diffusion_input(
                    "only the pinned model.safetensors checkpoint is admitted by this slice",
                ));
            }
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let bundle = self
                .state
                .load_bundle(&tensor_context)
                .map_err(runtime_failure)?;
            let model = publish_diffusion_bundle(&bundle, &context, NativeDiffusionRole::Model)?;
            let clip = publish_diffusion_bundle(&bundle, &context, NativeDiffusionRole::Clip)?;
            let vae = publish_diffusion_bundle(&bundle, &context, NativeDiffusionRole::Vae)?;
            Ok(NodeOutcome::Values {
                outputs: vec![model, clip, vae],
                ui: Some(json!({"checkpoint": "model.safetensors", "family": "COMFY-MODEL-0117"})),
                effects: Vec::new(),
            })
        })
    }
}

struct ClipTextEncodeNode {
    state: Arc<NativeDiffusionState>,
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
        context: &NodeContext,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<CacheDependencies, NodeFailure> {
        let identities = self
            .state
            .provider
            .cache_identities(&context.cancellation)
            .map_err(runtime_failure)?;
        Ok(CacheDependencies {
            artifact_digests: identities.clip().artifact_digests(),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let payload =
                resolve_diffusion_bundle(&context, &inputs, "clip", NativeDiffusionRole::Clip)?;
            let text = required_string(&inputs, "text")?;
            let (tokenizer, clip) = payload.model_payload().clip().ok_or_else(|| {
                invalid_diffusion_input("native CLIP handle has no canonical CLIP resource")
            })?;
            let prompts =
                vec![vec![WeightedText::checked(text, 1.0).map_err(|error| {
                    runtime_failure(map_clip_runtime_error(error))
                })?]];
            let batch = tokenizer
                .tokenize_batch(&prompts, tensor_context.cancellation)
                .map_err(|error| runtime_failure(map_clip_runtime_error(error)))?;
            let tokens = batch
                .rows()
                .first()
                .ok_or_else(|| {
                    invalid_diffusion_input("canonical SD1 tokenizer returned no prompt row")
                })?
                .sd1_token_ids()
                .map_err(|error| runtime_failure(map_clip_runtime_error(error)))?;
            let conditioning = clip
                .execute(&batch, &tensor_context)
                .map_err(|error| runtime_failure(map_clip_runtime_error(error)))?
                .conditioning()
                .clone();
            let identity = ConditioningIdentity::new(
                "sd15-clip-text",
                sd15_model_family_identity()
                    .map_err(|error| invalid_diffusion_input(&error.to_string()))?,
                sd15_latent_format_identity()
                    .map_err(|error| invalid_diffusion_input(&error.to_string()))?,
            )
            .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
            let entry = ConditioningEntry::checked(
                "cross-attention",
                ConditioningValue::cross_attention(conditioning)
                    .map_err(|error| invalid_diffusion_input(&error.to_string()))?,
                ConditioningEntryOptions::default(),
            )
            .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
            let conditioning =
                ConditioningSet::checked(identity, vec![entry], tensor_context.cancellation)
                    .map_err(|error| invalid_diffusion_input(&error.to_string()))?;
            let output = publish_conditioning(&context, conditioning)?;
            let ui = json!({"tokens": tokens.to_vec()});
            tensor_context.check().map_err(tensor_failure)?;
            Ok(NodeOutcome::Values {
                outputs: vec![output],
                ui: Some(ui),
                effects: Vec::new(),
            })
        })
    }
}

struct EmptyLatentNode {
    backend: Arc<CpuBackend>,
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
        inputs: BTreeMap<String, NativeValue>,
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
            .map_err(native_diffusion_model_failure)?;
            tensor_context.check().map_err(tensor_failure)?;
            let output = publish_tensor(&context, NativeTensorKind::Latent, latent)?;
            Ok(NodeOutcome::Values {
                outputs: vec![output],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

struct KSamplerNode {
    state: Arc<NativeDiffusionState>,
}

impl NativeNode for KSamplerNode {
    fn class_type(&self) -> &str {
        "KSampler"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn cache_dependencies(
        &self,
        context: &NodeContext,
        inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<CacheDependencies, NodeFailure> {
        let identities = self
            .state
            .provider
            .cache_identities(&context.cancellation)
            .map_err(runtime_failure)?;
        Ok(CacheDependencies {
            artifact_digests: identities.conditioning().artifact_digests(),
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
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let payload =
                resolve_diffusion_bundle(&context, &inputs, "model", NativeDiffusionRole::Model)?;
            let (model_payload, conditioning) = payload.model_resources().ok_or_else(|| {
                invalid_diffusion_input("native MODEL handle has no canonical model resources")
            })?;
            let model = model_payload.model().ok_or_else(|| {
                invalid_diffusion_input("native MODEL handle has no canonical SD15 model")
            })?;
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
            .map_err(native_diffusion_sampler_failure)?;
            let positive = resolve_conditioning(&context, &inputs, "positive")?;
            let negative = resolve_conditioning(&context, &inputs, "negative")?;
            let latent =
                resolve_tensor(&context, &inputs, "latent_image", NativeTensorKind::Latent)?;
            let stream = NoiseRequest::native_diffusion(
                context.prompt_id.0.to_string(),
                context.node_id.0.clone(),
            )
            .and_then(|request| request.stream(plan.seed(), comfy_tensor::DeviceId::CPU))
            .map_err(noise_failure)?;
            let sigmas = normal_sigmas(
                &self.state.backend,
                &tensor_context,
                usize::try_from(plan.steps())
                    .map_err(|_| invalid_diffusion_input("steps exceed usize"))?,
                plan.denoise(),
            )
            .map_err(native_diffusion_sampler_failure)?;
            let noise = normal_noise(
                &self.state.backend,
                latent.descriptor().shape(),
                &stream,
                &tensor_context,
            )
            .map_err(native_diffusion_sampler_failure)?;
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
            .map_err(native_diffusion_sampler_failure)?;
            let mut guidance = Sd15GuidanceAdapter::checked_prebound_conditioning_sets(
                model,
                conditioning,
                &positive,
                &negative,
            )
            .map_err(runtime_diffusion_failure)?;
            let mut typed_denoiser_failure = None;
            let trace_result = sample_euler(
                &self.state.backend,
                initial,
                &sigmas,
                &tensor_context,
                |latent, sigma, _step| {
                    let model_input = match scale_model_input(
                        &self.state.backend,
                        latent,
                        sigma,
                        &tensor_context,
                    ) {
                        Ok(model_input) => model_input,
                        Err(error) => {
                            let message = error.to_string();
                            typed_denoiser_failure = Some(native_diffusion_sampler_failure(error));
                            return Err(message);
                        }
                    };
                    let prediction = match guidance.execute(
                        &self.state.backend,
                        &model_input,
                        sigma,
                        &plan,
                        &tensor_context,
                    ) {
                        Ok(prediction) => prediction,
                        Err(error) => {
                            let message = error.to_string();
                            typed_denoiser_failure = Some(runtime_diffusion_failure(error));
                            return Err(message);
                        }
                    };
                    match sd15_interpret_prediction(
                        &self.state.backend,
                        prediction.guided(),
                        latent,
                        sigma,
                        &tensor_context,
                    ) {
                        Ok(prediction) => Ok(prediction),
                        Err(error) => {
                            let message = error.to_string();
                            typed_denoiser_failure = Some(native_diffusion_sampler_failure(error));
                            Err(message)
                        }
                    }
                },
            );
            let trace = match trace_result {
                Ok(trace) => trace,
                Err(error) if typed_denoiser_failure.is_some() => {
                    if let Some(failure) = typed_denoiser_failure.take() {
                        return Err(failure);
                    }
                    return Err(native_diffusion_sampler_failure(error));
                }
                Err(error) if tensor_context.cancellation.is_cancelled() => {
                    return Err(cancelled_diffusion_failure(error.to_string()));
                }
                Err(error) => return Err(native_diffusion_sampler_failure(error)),
            };
            let final_latent = trace
                .latents
                .last()
                .cloned()
                .ok_or_else(|| invalid_diffusion_input("Euler returned no latent"))?;
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
            let ui = json!({
                "sigmas": sigmas,
                "noise_sha256": tensor_sha256(&noise.noise).map_err(runtime_failure)?,
                "denoiser_sha256": denoiser_sha256,
                "latent_sha256": latent_sha256,
            });
            tensor_context.check().map_err(tensor_failure)?;
            let output = publish_tensor(&context, NativeTensorKind::Latent, final_latent)?;
            Ok(NodeOutcome::Values {
                outputs: vec![output],
                ui: Some(ui),
                effects: Vec::new(),
            })
        })
    }
}

struct VaeDecodeNode {
    state: Arc<NativeDiffusionState>,
}

impl NativeNode for VaeDecodeNode {
    fn class_type(&self) -> &str {
        "VAEDecode"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_DIFFUSION_REGISTRY_VERSION
    }

    fn cache_dependencies(
        &self,
        context: &NodeContext,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<CacheDependencies, NodeFailure> {
        let identities = self
            .state
            .provider
            .cache_identities(&context.cancellation)
            .map_err(runtime_failure)?;
        Ok(CacheDependencies {
            artifact_digests: identities.vae().artifact_digests(),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let tensor_context = native_image_tensor_context(&self.state.backend, &context);
            let payload =
                resolve_diffusion_bundle(&context, &inputs, "vae", NativeDiffusionRole::Vae)?;
            let vae = payload.model_payload().vae().ok_or_else(|| {
                invalid_diffusion_input("native VAE handle has no canonical VAE resource")
            })?;
            let latent = resolve_tensor(&context, &inputs, "samples", NativeTensorKind::Latent)?;
            let decoded = vae
                .decode(self.state.backend.as_ref(), &latent, &tensor_context)
                .map_err(vae_failure)?;
            if decoded.descriptor().shape() != [1, 3, 32, 32] {
                return Err(invalid_diffusion_input(
                    "native SD15 VAE returned an unexpected image shape",
                ));
            }
            let nchw = tensor_to_f32(&self.state.backend, &decoded, &tensor_context)
                .map_err(native_diffusion_tensor_failure)?;
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
            let ui = json!({"width": 32, "height": 32});
            tensor_context.check().map_err(tensor_failure)?;
            let output = publish_image(&context, NativeTensorKind::Image, image)?;
            Ok(NodeOutcome::Values {
                outputs: vec![output],
                ui: Some(ui),
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
    handle_store_generation: crate::NativeHandleStoreGeneration,
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
        let nodes = native_image_registry_with_execution_state(
            input_assets.clone(),
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
            handle_store_generation: crate::NativeHandleStoreGeneration::new()?,
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
        let nodes = native_registry_with_diffusion_provider(
            input_assets.clone(),
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
            handle_store_generation: crate::NativeHandleStoreGeneration::new()?,
            cpu_backend,
            metadata_enabled,
            diffusion_enabled: true,
        })
    }

    pub fn new_with_generated_registry(
        profile_id: ProfileId,
        input_assets: BTreeMap<String, Vec<u8>>,
        metadata_enabled: bool,
        cpu_backend: Arc<CpuBackend>,
    ) -> Result<Self, NativeImageRuntimeError> {
        validate_worker_input_assets(&input_assets)?;
        let input_assets = Arc::new(Mutex::new(input_assets));
        let mut nodes = native_image_registry_with_execution_state(
            input_assets.clone(),
            cpu_backend.clone(),
            metadata_enabled,
        )?;
        register_generated_family_bindings(&mut nodes)?;
        Ok(Self {
            profile_id,
            input_assets,
            nodes: Arc::new(nodes),
            cache: Arc::new(Mutex::new(NativeCache::new(4096).map_err(|error| {
                NativeImageRuntimeError::Execution(error.to_string())
            })?)),
            handle_store_generation: crate::NativeHandleStoreGeneration::new()?,
            cpu_backend,
            metadata_enabled,
            diffusion_enabled: false,
        })
    }

    pub fn new_with_generated_registry_and_diffusion_provider(
        profile_id: ProfileId,
        input_assets: BTreeMap<String, Vec<u8>>,
        metadata_enabled: bool,
        cpu_backend: Arc<CpuBackend>,
        provider: Arc<dyn NativeDiffusionProvider>,
    ) -> Result<Self, NativeImageRuntimeError> {
        validate_worker_input_assets(&input_assets)?;
        let input_assets = Arc::new(Mutex::new(input_assets));
        let mut nodes = native_registry_with_diffusion_provider(
            input_assets.clone(),
            cpu_backend.clone(),
            metadata_enabled,
            provider,
        )?;
        register_generated_family_bindings(&mut nodes)?;
        Ok(Self {
            profile_id,
            input_assets,
            nodes: Arc::new(nodes),
            cache: Arc::new(Mutex::new(NativeCache::new(4096).map_err(|error| {
                NativeImageRuntimeError::Execution(error.to_string())
            })?)),
            handle_store_generation: crate::NativeHandleStoreGeneration::new()?,
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
        plan.validate_provider_execution_identity()?;
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
        let provider_identity = plan
            .provider_registry_pin()
            .map(NativeProviderRegistryPin::identity_sha256)
            .unwrap_or_else(|| "local".to_owned());
        let configuration_token = format!("{configuration_token}:provider={provider_identity}");
        let mut engine = ExecutionEngine::new_with_handle_store_generation(
            self.profile_id,
            self.nodes.clone(),
            self.cache.clone(),
            effects.clone(),
            registry_version,
            workspace,
            self.handle_store_generation.clone(),
        )?
        .with_compute_backend(self.cpu_backend.clone())?
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
    cpu_backend: Arc<CpuBackend>,
}

impl NativeNode for LoadImageNode {
    fn class_type(&self) -> &str {
        "LoadImage"
    }

    fn implementation_version(&self) -> &str {
        NATIVE_IMAGE_REGISTRY_VERSION
    }

    fn cache_dependencies(
        &self,
        context: &NodeContext,
        inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<CacheDependencies, NodeFailure> {
        context.cancellation.check().map_err(cancellation_failure)?;
        let image = required_string(inputs, "image")?;
        let digest = self.input_digest(image, &context.cancellation)?;
        Ok(CacheDependencies {
            artifact_digests: BTreeMap::from([(image.to_owned(), digest)]),
            ..CacheDependencies::default()
        })
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, NativeValue>,
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
            tensor_context.check().map_err(tensor_failure)?;
            let image_output = publish_image(&context, NativeTensorKind::Image, image)?;
            let mask_output = publish_image(&context, NativeTensorKind::Mask, mask)?;
            Ok(NodeOutcome::Values {
                outputs: vec![image_output, mask_output],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

impl LoadImageNode {
    fn input_digest(
        &self,
        logical_id: &str,
        cancellation: &CancellationToken,
    ) -> Result<String, NodeFailure> {
        cancellation.check().map_err(cancellation_failure)?;
        let input_assets = self.input_assets.lock();
        let bytes = input_assets.get(logical_id).ok_or_else(|| NodeFailure {
            code: "native_image_input_missing".to_owned(),
            message: format!("worker input `{logical_id}` was not supplied by the host"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        })?;
        let mut hasher = Sha256::new();
        for chunk in bytes.chunks(64 * 1024) {
            cancellation.check().map_err(cancellation_failure)?;
            hasher.update(chunk);
        }
        cancellation.check().map_err(cancellation_failure)?;
        Ok(format!("{:x}", hasher.finalize()))
    }
}

struct ImageScaleNode {
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
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let image = resolve_image(&context, &inputs, "image", NativeTensorKind::Image)?;
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
            tensor_context.check().map_err(tensor_failure)?;
            let output = publish_image(&context, NativeTensorKind::Image, resized)?;
            Ok(NodeOutcome::Values {
                outputs: vec![output],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

struct ImageInvertNode {
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
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let image = resolve_image(&context, &inputs, "image", NativeTensorKind::Image)?;
            let tensor_context = native_image_tensor_context(&self.cpu_backend, &context);
            let inverted = image
                .invert(&self.cpu_backend, &tensor_context)
                .map_err(tensor_failure)?;
            tensor_context.check().map_err(tensor_failure)?;
            let output = publish_image(&context, NativeTensorKind::Image, inverted)?;
            Ok(NodeOutcome::Values {
                outputs: vec![output],
                ui: None,
                effects: Vec::new(),
            })
        })
    }
}

struct SaveImageNode {
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
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let image = resolve_image(&context, &inputs, "images", NativeTensorKind::Image)?;
            if self.namespace == AssetNamespace::Temporary {
                let preview = context
                    .prepare_image_preview(&image, "ComfyUI_temp")
                    .map_err(image_preview_failure)?;
                let (effects, ui) = preview.into_parts();
                return Ok(NodeOutcome::Values {
                    outputs: Vec::new(),
                    ui: Some(ui),
                    effects,
                });
            }
            let (batch, height, width, channels) = image.dimensions().map_err(tensor_failure)?;
            let pixels = image.as_f32_slice().map_err(tensor_failure)?;
            let prefix = optional_string(&inputs, "filename_prefix")
                .unwrap_or("ComfyUI")
                .to_owned();
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
                let request = NativeOutputEffectRequest::checked(
                    match self.namespace {
                        AssetNamespace::Output => NativeOutputNamespace::Output,
                        AssetNamespace::Temporary => NativeOutputNamespace::Temporary,
                        _ => return Err(invalid_output_namespace()),
                    },
                    prefix.clone(),
                    "png",
                    batch_index,
                    NativeOutputShape::Image {
                        width: u32::try_from(width).map_err(|_| dimension_failure())?,
                        height: u32::try_from(height).map_err(|_| dimension_failure())?,
                    },
                    Arc::from(encoded),
                    context
                        .prepared_effects()
                        .map_err(effect_service_failure)?
                        .maximum_output_bytes(),
                )
                .map_err(effect_service_failure)?;
                let effect = context
                    .prepared_effects()
                    .map_err(effect_service_failure)?
                    .prepare_output(request, &context.cancellation)
                    .map_err(effect_service_failure)?;
                let transaction_id = effect.transaction_id();
                effects.push(effect);
                ui_images.push(json!({
                    "transaction_id": transaction_id,
                    "batch_index": batch_index,
                    "type": self.namespace.locator_type(),
                }));
            }
            let outputs = vec![
                inputs
                    .get("images")
                    .cloned()
                    .ok_or_else(|| invalid_diffusion_input("`images` is missing"))?,
            ];
            Ok(NodeOutcome::Values {
                outputs,
                ui: Some(json!({"images": ui_images})),
                effects,
            })
        })
    }
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

#[derive(Clone, Default)]
struct NativeImageProposalCoordinator {
    state: Arc<Mutex<NativeImageEffectState>>,
}

impl NativeImageProposalCoordinator {
    fn output_proposals(&self) -> Vec<NativeImageOutputProposal> {
        self.state.lock().proposed.clone()
    }
}

struct NativeImagePreparedEffectService {
    identity: NativeNodeServiceIdentity,
    prompt_id: comfy_types::PromptId,
    state: Arc<Mutex<NativeImageEffectState>>,
    ordinal: AtomicU64,
}

impl fmt::Debug for NativeImagePreparedEffectService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeImagePreparedEffectService")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

impl NativePreparedEffectService for NativeImagePreparedEffectService {
    fn identity(&self) -> &NativeNodeServiceIdentity {
        &self.identity
    }

    fn maximum_output_bytes(&self) -> u64 {
        2 * 1024 * 1024 * 1024
    }

    fn prepare_output(
        &self,
        request: NativeOutputEffectRequest,
        cancellation: &CancellationToken,
    ) -> Result<PreparedEffectRequest, NativeEffectServiceError> {
        cancellation
            .check()
            .map_err(|_| NativeEffectServiceError::Cancelled)?;
        let ordinal = self
            .ordinal
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| NativeEffectServiceError::Rejected)?;
        let transaction_id =
            native_output_transaction_id(&self.identity, ordinal, request.request_digest_sha256());
        let (width, height) = match request.shape() {
            NativeOutputShape::File => (0, 0),
            NativeOutputShape::Image { width, height } => (width, height),
        };
        let namespace = match request.namespace() {
            NativeOutputNamespace::Output => AssetNamespace::Output,
            NativeOutputNamespace::Temporary => AssetNamespace::Temporary,
        };
        let output = OutputProposal::new(
            transaction_id,
            namespace,
            request.filename_prefix(),
            request.extension(),
            request.batch_index(),
            width,
            height,
            request.content().to_vec(),
        )
        .map_err(|_| NativeEffectServiceError::InvalidRequest)?;
        let ticket = PreparedEffectRequest::checked(
            self.identity.service_id(),
            transaction_id,
            NativePreparedEffectKind::Output,
            request.request_digest_sha256(),
        )
        .map_err(|_| NativeEffectServiceError::Rejected)?;
        let effect = PreparedEffect {
            prompt_id: self.prompt_id,
            attempt_id: self.identity.attempt_id(),
            node_id: self.identity.node_id().clone(),
            service_id: self.identity.service_id(),
            transaction_id,
            kind: NativePreparedEffectKind::Output,
            request_digest_sha256: request.request_digest_sha256().to_owned(),
        };
        let prepared = PreparedNativeImage {
            effect: effect.clone(),
            proposal: NativeImageOutputProposal::new(effect.node_id, output)
                .map_err(|_| NativeEffectServiceError::Rejected)?,
        };
        {
            let mut state = self.state.lock();
            if state.prepared.insert(transaction_id, prepared).is_some() {
                return Err(NativeEffectServiceError::InvalidTicket);
            }
        }
        if cancellation.check().is_err() {
            self.state.lock().prepared.remove(&transaction_id);
            return Err(NativeEffectServiceError::Cancelled);
        }
        Ok(ticket)
    }

    fn rollback_prepared(
        &self,
        request: &PreparedEffectRequest,
    ) -> Result<(), NativeEffectServiceError> {
        if request.service_id() != self.identity.service_id() {
            return Err(NativeEffectServiceError::InvalidTicket);
        }
        let mut state = self.state.lock();
        let Some(prepared) = state.prepared.get(&request.transaction_id()) else {
            return Err(NativeEffectServiceError::InvalidTicket);
        };
        if prepared.effect.service_id != request.service_id()
            || prepared.effect.kind != request.kind()
            || prepared.effect.request_digest_sha256 != request.request_digest_sha256()
        {
            return Err(NativeEffectServiceError::InvalidTicket);
        }
        state.prepared.remove(&request.transaction_id());
        Ok(())
    }

    fn rollback_all_prepared(&self) -> Result<(), NativeEffectServiceError> {
        self.state.lock().prepared.retain(|_, prepared| {
            prepared.effect.service_id != self.identity.service_id()
                || prepared.effect.attempt_id != self.identity.attempt_id()
                || prepared.effect.node_id != *self.identity.node_id()
        });
        Ok(())
    }
}

impl EffectCoordinator for NativeImageProposalCoordinator {
    fn node_service(
        &self,
        identity: NativeNodeServiceIdentity,
        prompt_id: comfy_types::PromptId,
    ) -> Result<Arc<dyn NativePreparedEffectService>, String> {
        Ok(Arc::new(NativeImagePreparedEffectService {
            identity,
            prompt_id,
            state: self.state.clone(),
            ordinal: AtomicU64::new(0),
        }))
    }

    fn prepared_effect(
        &self,
        ticket: &PreparedEffectRequest,
        prompt_id: comfy_types::PromptId,
        attempt_id: AttemptId,
        node_id: &NodeId,
    ) -> Result<PreparedEffect, String> {
        let state = self.state.lock();
        let prepared = state
            .prepared
            .get(&ticket.transaction_id())
            .ok_or_else(|| "native output effect ticket was not prepared".to_owned())?;
        if prepared.effect.prompt_id != prompt_id
            || prepared.effect.attempt_id != attempt_id
            || &prepared.effect.node_id != node_id
            || prepared.effect.service_id != ticket.service_id()
            || prepared.effect.kind != ticket.kind()
            || prepared.effect.request_digest_sha256 != ticket.request_digest_sha256()
        {
            return Err("native output effect ticket belongs to another node session".to_owned());
        }
        Ok(prepared.effect.clone())
    }

    fn commit_batch(
        &self,
        effects: &[PreparedEffect],
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        cancellation
            .check()
            .map_err(|_| "native image effect commit was cancelled".to_owned())?;
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
        cancellation
            .check()
            .map_err(|_| "native image effect commit was cancelled".to_owned())?;
        for (effect, prepared) in effects.iter().zip(prepared_batch) {
            state.prepared.remove(&effect.transaction_id);
            state.proposed.push(prepared.proposal);
        }
        Ok(())
    }

    fn rollback_batch(&self, effects: &[PreparedEffect]) -> Result<(), String> {
        let mut state = self.state.lock();
        let transaction_ids = effects
            .iter()
            .map(|effect| effect.transaction_id)
            .collect::<BTreeSet<_>>();
        for effect in effects {
            state.prepared.remove(&effect.transaction_id);
        }
        state
            .proposed
            .retain(|proposal| !transaction_ids.contains(&proposal.proposal_id()));
        Ok(())
    }
}

fn required_string<'a>(
    inputs: &'a BTreeMap<String, NativeValue>,
    name: &str,
) -> Result<&'a str, NodeFailure> {
    optional_string(inputs, name).ok_or_else(|| NodeFailure {
        code: "invalid_native_image_input".to_owned(),
        message: format!("`{name}` must be a string"),
        kind: NodeFailureKind::Failure,
        retryable: false,
    })
}

fn optional_string<'a>(inputs: &'a BTreeMap<String, NativeValue>, name: &str) -> Option<&'a str> {
    match inputs.get(name) {
        Some(NativeValue::Primitive {
            value: NativePrimitive::String(value),
        }) => Some(value),
        _ => None,
    }
}

fn required_u64(inputs: &BTreeMap<String, NativeValue>, name: &str) -> Result<u64, NodeFailure> {
    match inputs.get(name) {
        Some(NativeValue::Primitive {
            value: NativePrimitive::UnsignedInteger(value),
        }) => Ok(*value),
        Some(NativeValue::Primitive {
            value: NativePrimitive::Integer(value),
        }) => u64::try_from(*value).map_err(|_| NodeFailure {
            code: "invalid_native_image_input".to_owned(),
            message: format!("`{name}` must be a non-negative integer"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
        _ => Err(NodeFailure {
            code: "invalid_native_image_input".to_owned(),
            message: format!("`{name}` must be a non-negative integer"),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
    }
}

fn required_f32(inputs: &BTreeMap<String, NativeValue>, name: &str) -> Result<f32, NodeFailure> {
    let value = match inputs.get(name) {
        Some(NativeValue::Primitive {
            value: NativePrimitive::Number(value),
        }) => Some(*value as f32),
        Some(NativeValue::Primitive {
            value: NativePrimitive::Integer(value),
        }) => Some(*value as f32),
        Some(NativeValue::Primitive {
            value: NativePrimitive::UnsignedInteger(value),
        }) => Some(*value as f32),
        _ => None,
    }
    .filter(|value| value.is_finite())
    .ok_or_else(|| invalid_diffusion_input(&format!("`{name}` must be a finite number")))?;
    Ok(value)
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
    NodeFailure {
        code: "native_diffusion_failed".to_owned(),
        message: error.to_string(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypedFailureClass {
    Cancelled,
    ResourceExhausted,
}

fn typed_failure_class(error: &(dyn std::error::Error + 'static)) -> Option<TypedFailureClass> {
    let mut current = Some(error);
    for _ in 0..32 {
        let Some(error) = current else {
            return None;
        };
        if let Some(error) = error.downcast_ref::<TensorError>() {
            if error == &TensorError::Cancelled {
                return Some(TypedFailureClass::Cancelled);
            }
            if tensor_error_is_resource_exhaustion(error) {
                return Some(TypedFailureClass::ResourceExhausted);
            }
        }
        if let Some(error) = error.downcast_ref::<RngError>() {
            match error {
                RngError::Cancelled => return Some(TypedFailureClass::Cancelled),
                RngError::AllocationFailed { .. } => {
                    return Some(TypedFailureClass::ResourceExhausted);
                }
                _ => {}
            }
        }
        if let Some(error) = error.downcast_ref::<RngCompatibilityError>() {
            match error {
                RngCompatibilityError::Cancelled => return Some(TypedFailureClass::Cancelled),
                RngCompatibilityError::AllocationFailed { .. } => {
                    return Some(TypedFailureClass::ResourceExhausted);
                }
                _ => {}
            }
        }
        if let Some(error) = error.downcast_ref::<SamplingError>() {
            match error {
                SamplingError::Cancelled => return Some(TypedFailureClass::Cancelled),
                SamplingError::OutOfMemory(_) => {
                    return Some(TypedFailureClass::ResourceExhausted);
                }
                _ => {}
            }
        }
        if matches!(
            error.downcast_ref::<SamplingProfileError>(),
            Some(SamplingProfileError::OutOfMemory(_))
        ) {
            return Some(TypedFailureClass::ResourceExhausted);
        }
        if let Some(error) = error.downcast_ref::<SchedulerError>() {
            match error {
                SchedulerError::Cancelled => return Some(TypedFailureClass::Cancelled),
                SchedulerError::OutOfMemory(_) => {
                    return Some(TypedFailureClass::ResourceExhausted);
                }
                _ => {}
            }
        }
        if matches!(
            error.downcast_ref::<NoiseError>(),
            Some(NoiseError::Cancelled)
        ) {
            return Some(TypedFailureClass::Cancelled);
        }
        if let Some(error) = error.downcast_ref::<AttentionError>() {
            match error {
                AttentionError::Cancelled => return Some(TypedFailureClass::Cancelled),
                AttentionError::AllocationFailed { .. }
                | AttentionError::WorkspaceTooSmall { .. } => {
                    return Some(TypedFailureClass::ResourceExhausted);
                }
                _ => {}
            }
        }
        if let Some(error) = error.downcast_ref::<ModelStoreError>() {
            match error {
                ModelStoreError::Cancelled => return Some(TypedFailureClass::Cancelled),
                ModelStoreError::AllocationFailed { .. } => {
                    return Some(TypedFailureClass::ResourceExhausted);
                }
                _ => {}
            }
        }
        if matches!(
            error.downcast_ref::<NativeOpsError>(),
            Some(NativeOpsError::Cancelled)
        ) {
            return Some(TypedFailureClass::Cancelled);
        }
        if matches!(
            error.downcast_ref::<IndexingMaskingPartOneError>(),
            Some(IndexingMaskingPartOneError::Cancelled)
        ) || matches!(
            error.downcast_ref::<ElementwiseRuntimePartSixteenError>(),
            Some(ElementwiseRuntimePartSixteenError::Cancelled)
        ) || matches!(
            error.downcast_ref::<ConditioningError>(),
            Some(ConditioningError::Cancelled)
        ) || matches!(
            error.downcast_ref::<NativeDiffusionModelError>(),
            Some(NativeDiffusionModelError::Cancelled)
        ) || matches!(
            error.downcast_ref::<GuidanceError>(),
            Some(GuidanceError::Cancelled)
        ) || matches!(
            error.downcast_ref::<NativeImageRuntimeError>(),
            Some(NativeImageRuntimeError::Cancelled)
        ) {
            return Some(TypedFailureClass::Cancelled);
        }
        if matches!(
            error.downcast_ref::<VaeError>(),
            Some(VaeError::Allocation(_))
        ) || matches!(
            error.downcast_ref::<GuidanceError>(),
            Some(GuidanceError::BatchMemoryLimit { .. })
        ) || matches!(
            error.downcast_ref::<NativeImageRuntimeError>(),
            Some(NativeImageRuntimeError::ResourceExhausted(_))
        ) {
            return Some(TypedFailureClass::ResourceExhausted);
        }
        current = error.source();
    }
    None
}

fn classified_runtime_error(error: impl std::error::Error + 'static) -> NativeImageRuntimeError {
    let message = error.to_string();
    match typed_failure_class(&error) {
        Some(TypedFailureClass::Cancelled) => NativeImageRuntimeError::Cancelled,
        Some(TypedFailureClass::ResourceExhausted) => {
            NativeImageRuntimeError::ResourceExhausted(message)
        }
        None => NativeImageRuntimeError::Execution(message),
    }
}

fn classified_diffusion_failure(error: impl std::error::Error + 'static) -> NodeFailure {
    let message = error.to_string();
    match typed_failure_class(&error) {
        Some(TypedFailureClass::Cancelled) => cancelled_diffusion_failure(message),
        Some(TypedFailureClass::ResourceExhausted) => resource_diffusion_failure(message),
        None => diffusion_failure(message),
    }
}

fn tensor_error_is_resource_exhaustion(error: &TensorError) -> bool {
    matches!(
        error,
        TensorError::AllocationFailed { .. }
            | TensorError::ResourceLimitExceeded { .. }
            | TensorError::WorkspaceAuthorizationExceeded { .. }
    )
}

fn cancelled_diffusion_failure(message: impl Into<String>) -> NodeFailure {
    NodeFailure {
        code: "native_diffusion_cancelled".to_owned(),
        message: message.into(),
        kind: NodeFailureKind::Interrupted,
        retryable: true,
    }
}

fn resource_diffusion_failure(message: impl Into<String>) -> NodeFailure {
    NodeFailure {
        code: "native_diffusion_resource_exhausted".to_owned(),
        message: message.into(),
        kind: NodeFailureKind::Failure,
        retryable: true,
    }
}

fn runtime_diffusion_failure(error: NativeImageRuntimeError) -> NodeFailure {
    match error {
        NativeImageRuntimeError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        NativeImageRuntimeError::ResourceExhausted(message) => resource_diffusion_failure(message),
        error => diffusion_failure(error),
    }
}

fn native_diffusion_tensor_failure(error: NativeDiffusionTensorError) -> NodeFailure {
    match error {
        NativeDiffusionTensorError::Tensor(error) => classified_diffusion_failure(error),
        NativeDiffusionTensorError::Functional(error) => functional_failure(error),
        NativeDiffusionTensorError::Operator(error) => operator_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn functional_failure(error: FunctionalError) -> NodeFailure {
    match error {
        FunctionalError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        FunctionalError::AllocationFailed { .. } => resource_diffusion_failure(error.to_string()),
        FunctionalError::Tensor(error) => classified_diffusion_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn operator_failure(error: OperatorIndirectionError) -> NodeFailure {
    match error {
        OperatorIndirectionError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        OperatorIndirectionError::Tensor(error) => classified_diffusion_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn attention_failure(error: AttentionError) -> NodeFailure {
    match error {
        AttentionError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        AttentionError::Tensor(error) => classified_diffusion_failure(error),
        AttentionError::AllocationFailed { .. } | AttentionError::WorkspaceTooSmall { .. } => {
            resource_diffusion_failure(error.to_string())
        }
        error => classified_diffusion_failure(error),
    }
}

fn model_store_failure(error: ModelStoreError) -> NodeFailure {
    match error {
        ModelStoreError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        ModelStoreError::AllocationFailed { .. } => resource_diffusion_failure(error.to_string()),
        error => classified_diffusion_failure(error),
    }
}

fn native_ops_failure(error: NativeOpsError) -> NodeFailure {
    match error {
        NativeOpsError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        NativeOpsError::Tensor(error) => operator_failure(error),
        NativeOpsError::Functional(error) => functional_failure(error),
        NativeOpsError::Quantization(error) => quantization_failure(error),
        NativeOpsError::Workspace(error) => classified_diffusion_failure(error),
        NativeOpsError::ModulePartTwo(NeuralNetworkModulePartTwoError::Cancelled) => {
            cancelled_diffusion_failure(error.to_string())
        }
        NativeOpsError::ModulePartTwo(NeuralNetworkModulePartTwoError::Tensor(error)) => {
            classified_diffusion_failure(error)
        }
        NativeOpsError::ModulePartTwo(NeuralNetworkModulePartTwoError::Functional(error)) => {
            functional_failure(error)
        }
        NativeOpsError::ModulePartTwo(NeuralNetworkModulePartTwoError::Operator(error)) => {
            operator_failure(error)
        }
        error => classified_diffusion_failure(error),
    }
}

fn quantization_failure(error: QuantizationError) -> NodeFailure {
    match error {
        QuantizationError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        QuantizationError::AllocationFailed { .. }
        | QuantizationError::MaterializationCapacity { .. } => {
            resource_diffusion_failure(error.to_string())
        }
        error => classified_diffusion_failure(error),
    }
}

fn clip_text_failure(error: ClipTextError) -> NodeFailure {
    match error {
        ClipTextError::Tensor(error) => classified_diffusion_failure(error),
        ClipTextError::Module(error) => native_ops_failure(error),
        ClipTextError::Attention(error) => attention_failure(error),
        ClipTextError::NativeDiffusion(error) => native_diffusion_tensor_failure(error),
        ClipTextError::ShapeLayout(ShapeLayoutTransformPartTwoError::Cancelled) => {
            cancelled_diffusion_failure(error.to_string())
        }
        ClipTextError::Allocation(_) => resource_diffusion_failure(error.to_string()),
        error => classified_diffusion_failure(error),
    }
}

fn native_diffusion_model_failure(error: NativeDiffusionModelError) -> NodeFailure {
    match error {
        NativeDiffusionModelError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        NativeDiffusionModelError::ResourceExhausted(message) => {
            resource_diffusion_failure(message)
        }
        NativeDiffusionModelError::Tensor(error) => native_diffusion_tensor_failure(error),
        NativeDiffusionModelError::TensorBackend(error) => classified_diffusion_failure(error),
        NativeDiffusionModelError::Attention(error) => attention_failure(error),
        NativeDiffusionModelError::Store(error) => model_store_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn native_diffusion_model_runtime_error(
    error: NativeDiffusionModelError,
) -> NativeImageRuntimeError {
    node_failure_runtime_error(native_diffusion_model_failure(error))
}

fn control_runtime_error(error: ControlNetError) -> NativeImageRuntimeError {
    let failure = match error {
        ControlNetError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        ControlNetError::Tensor(error) => classified_diffusion_failure(error),
        ControlNetError::Vae(error) => vae_failure(error),
        error => classified_diffusion_failure(error),
    };
    node_failure_runtime_error(failure)
}

fn node_failure_runtime_error(failure: NodeFailure) -> NativeImageRuntimeError {
    match (failure.kind, failure.retryable) {
        (NodeFailureKind::Interrupted, true) => NativeImageRuntimeError::Cancelled,
        (NodeFailureKind::Failure, true) => {
            NativeImageRuntimeError::ResourceExhausted(failure.message)
        }
        (NodeFailureKind::Interrupted | NodeFailureKind::Failure, false) => {
            NativeImageRuntimeError::Execution(failure.message)
        }
    }
}

fn rng_compatibility_failure(error: RngCompatibilityError) -> NodeFailure {
    match error {
        RngCompatibilityError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        RngCompatibilityError::AllocationFailed { .. } => {
            resource_diffusion_failure(error.to_string())
        }
        RngCompatibilityError::Canonical(error) => classified_diffusion_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn sampling_failure(error: SamplingError) -> NodeFailure {
    match error {
        SamplingError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        SamplingError::OutOfMemory(_) => resource_diffusion_failure(error.to_string()),
        SamplingError::Tensor(error) => classified_diffusion_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn sampling_profile_failure(error: SamplingProfileError) -> NodeFailure {
    match error {
        SamplingProfileError::OutOfMemory(_) => resource_diffusion_failure(error.to_string()),
        error => classified_diffusion_failure(error),
    }
}

fn scheduler_failure(error: SchedulerError) -> NodeFailure {
    match error {
        SchedulerError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        SchedulerError::OutOfMemory(_) => resource_diffusion_failure(error.to_string()),
        SchedulerError::Tensor(error) => classified_diffusion_failure(error),
        SchedulerError::Profile(error) => sampling_profile_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn noise_failure(error: NoiseError) -> NodeFailure {
    match error {
        NoiseError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        NoiseError::Rng(error) => classified_diffusion_failure(error),
        NoiseError::Tensor(error) => classified_diffusion_failure(error),
        NoiseError::TensorKernel(error) => native_diffusion_tensor_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn native_diffusion_sampler_failure(error: NativeDiffusionSamplerError) -> NodeFailure {
    match error {
        NativeDiffusionSamplerError::Tensor(error) => classified_diffusion_failure(error),
        NativeDiffusionSamplerError::TensorKernel(error) => native_diffusion_tensor_failure(error),
        NativeDiffusionSamplerError::Rng(error) => classified_diffusion_failure(error),
        NativeDiffusionSamplerError::RngCompatibility(error) => rng_compatibility_failure(error),
        NativeDiffusionSamplerError::Sampling(error) => sampling_failure(error),
        NativeDiffusionSamplerError::Scheduler(error) => scheduler_failure(error),
        NativeDiffusionSamplerError::Profile(error) => sampling_profile_failure(error),
        NativeDiffusionSamplerError::Noise(error) => noise_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn vae_failure(error: VaeError) -> NodeFailure {
    match error {
        VaeError::Allocation(_) => resource_diffusion_failure(error.to_string()),
        VaeError::Architecture(VaeArchitectureError::Cancelled(_)) => {
            cancelled_diffusion_failure(error.to_string())
        }
        VaeError::LatentFormat(LatentFormatError::Tensor(error)) | VaeError::Tensor(error) => {
            classified_diffusion_failure(error)
        }
        VaeError::NativeTensor(error) => native_diffusion_tensor_failure(error),
        VaeError::Attention(error) => attention_failure(error),
        VaeError::ModelStore(error) => model_store_failure(error),
        VaeError::NativeOps(error) => native_ops_failure(error),
        error => classified_diffusion_failure(error),
    }
}

fn tensor_sha256(tensor: &Tensor) -> Result<String, NativeImageRuntimeError> {
    Ok(format!("{:x}", Sha256::digest(tensor.contiguous_bytes()?)))
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

fn native_value_json(value: &NativeValue) -> Result<Value, NodeFailure> {
    match value {
        NativeValue::Primitive { value } => Ok(match value {
            NativePrimitive::Null => Value::Null,
            NativePrimitive::Boolean(value) => Value::Bool(*value),
            NativePrimitive::Integer(value) => Value::from(*value),
            NativePrimitive::UnsignedInteger(value) => Value::from(*value),
            NativePrimitive::Number(value) => Value::from(*value),
            NativePrimitive::String(value) => Value::String(value.clone()),
        }),
        NativeValue::List { values } => values
            .iter()
            .map(native_value_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        NativeValue::PreservedUnknown { value, .. } => Ok(value.clone()),
        NativeValue::Handle { .. } => Err(NodeFailure {
            code: "opaque_handle_serialization_rejected".to_owned(),
            message: "opaque native handles cannot be projected into persisted metadata".to_owned(),
            kind: NodeFailureKind::Failure,
            retryable: false,
        }),
    }
}

fn png_metadata(
    inputs: &BTreeMap<String, NativeValue>,
) -> Result<BTreeMap<String, String>, NodeFailure> {
    let mut metadata = BTreeMap::new();
    if let Some(prompt) = inputs.get("prompt")
        && !matches!(
            prompt,
            NativeValue::Primitive {
                value: NativePrimitive::Null
            }
        )
    {
        metadata.insert(
            "prompt".to_owned(),
            serde_json::to_string(&native_value_json(prompt)?).map_err(encoding_failure)?,
        );
    }
    if let Some(extra) = inputs
        .get("extra_pnginfo")
        .map(native_value_json)
        .transpose()?
        .and_then(|value| value.as_object().cloned())
    {
        for (key, value) in extra {
            metadata.insert(
                key,
                serde_json::to_string(&value).map_err(encoding_failure)?,
            );
        }
    }
    Ok(metadata)
}

fn native_output_transaction_id(
    identity: &NativeNodeServiceIdentity,
    ordinal: u64,
    request_digest_sha256: &str,
) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"sim.comfy.native-output-transaction.v1");
    hasher.update(identity.service_id().as_bytes());
    hasher.update(identity.attempt_id().0.as_bytes());
    hasher.update(identity.node_id().0.as_bytes());
    hasher.update(ordinal.to_le_bytes());
    hasher.update(request_digest_sha256.as_bytes());
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
        let NativeValue::Primitive {
            value: NativePrimitive::String(logical_id),
        } = value
        else {
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
    match error {
        TensorError::Cancelled => NodeFailure {
            code: "native_image_cancelled".to_owned(),
            message: error.to_string(),
            kind: NodeFailureKind::Interrupted,
            retryable: true,
        },
        error if tensor_error_is_resource_exhaustion(&error) => NodeFailure {
            code: "native_image_resource_exhausted".to_owned(),
            message: error.to_string(),
            kind: NodeFailureKind::Failure,
            retryable: true,
        },
        error => NodeFailure {
            code: "native_image_tensor_failed".to_owned(),
            message: error.to_string(),
            kind: NodeFailureKind::Failure,
            retryable: false,
        },
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

fn image_preview_failure(error: NativeImagePreviewError) -> NodeFailure {
    match error {
        NativeImagePreviewError::Tensor(error) => tensor_failure(error),
        NativeImagePreviewError::Png(error) => media_failure(error),
        NativeImagePreviewError::Effect(error) => effect_service_failure(error),
        NativeImagePreviewError::Contract(error) => NodeFailure {
            code: "native_image_preview_context_invalid".to_owned(),
            message: error.to_string(),
            kind: NodeFailureKind::Failure,
            retryable: false,
        },
        NativeImagePreviewError::DimensionOverflow => dimension_failure(),
    }
}

fn runtime_failure(error: NativeImageRuntimeError) -> NodeFailure {
    match error {
        NativeImageRuntimeError::Cancelled => cancelled_diffusion_failure(error.to_string()),
        NativeImageRuntimeError::ResourceExhausted(message) => resource_diffusion_failure(message),
        error => NodeFailure {
            code: "native_tensor_handle_failed".to_owned(),
            message: error.to_string(),
            kind: NodeFailureKind::Failure,
            retryable: false,
        },
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

fn invalid_output_namespace() -> NodeFailure {
    NodeFailure {
        code: "native_output_namespace_invalid".to_owned(),
        message: "native output effects require output or temporary namespace".to_owned(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

fn effect_service_failure(error: NativeEffectServiceError) -> NodeFailure {
    NodeFailure {
        code: match &error {
            NativeEffectServiceError::Cancelled => "native_effect_cancelled",
            NativeEffectServiceError::Unavailable => "native_effect_unavailable",
            NativeEffectServiceError::InvalidRequest => "native_effect_invalid_request",
            NativeEffectServiceError::InvalidTicket => "native_effect_invalid_ticket",
            NativeEffectServiceError::Rejected => "native_effect_rejected",
        }
        .to_owned(),
        message: error.to_string(),
        kind: if matches!(error, NativeEffectServiceError::Cancelled) {
            NodeFailureKind::Interrupted
        } else {
            NodeFailureKind::Failure
        },
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
    pub provider_registry: Option<NativeProviderRegistryPin>,
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
            provider_registry: None,
        })
    }

    pub fn with_memory_policy(mut self, memory_policy: MemoryPolicy) -> Self {
        self.memory_policy = memory_policy;
        self
    }

    pub fn with_provider_registry(
        mut self,
        provider_registry: NativeProviderRegistryPin,
    ) -> Result<Self, NativeImageRuntimeError> {
        provider_registry.validate()?;
        self.provider_registry = Some(provider_registry);
        self.validate()?;
        Ok(self)
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
        if let Some(provider_registry) = &self.provider_registry {
            provider_registry.validate()?;
            let deployment = self.worker.registry_deployment.as_ref().ok_or_else(|| {
                NativeImageRuntimeError::Registry(
                    "provider registry pin requires a worker registry deployment".to_owned(),
                )
            })?;
            if deployment.begin().generation().get() != provider_registry.generation()
                || deployment.begin().registry_digest_sha256().as_str()
                    != provider_registry.registry_digest_sha256()
            {
                return Err(NativeImageRuntimeError::Registry(
                    "provider registry pin differs from the worker registry deployment".to_owned(),
                ));
            }
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
    provider_registry: Option<NativeProviderRegistryPin>,
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
        let provider_registry = config.provider_registry.clone();
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
            provider_registry,
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
        match &command.kind {
            ExecutionControlCommandKind::Queue { plan, .. } => {
                validate_plan_provider_registry(plan, self.provider_registry.as_ref())?;
            }
            ExecutionControlCommandKind::Retry {
                replacement_plan: Some(plan),
                ..
            } => {
                validate_plan_provider_registry(plan, self.provider_registry.as_ref())?;
            }
            _ => {}
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

fn validate_plan_provider_registry(
    plan: &CompiledPlan,
    current: Option<&NativeProviderRegistryPin>,
) -> Result<(), ExecutionFailure> {
    plan.validate_provider_execution_identity()
        .map_err(|error| {
            ExecutionFailure::new("native_provider_plan_invalid", error.to_string())
                .with_origin(ExecutionFailureOrigin::Validation)
        })?;
    if let Some(planned) = plan.provider_registry_pin()
        && current != Some(planned)
    {
        return Err(ExecutionFailure::new(
            "native_provider_registry_stale",
            "the compiled plan provider registry is unavailable or has changed",
        )
        .with_origin(ExecutionFailureOrigin::Validation));
    }
    Ok(())
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
        if let Err(failure) =
            validate_plan_provider_registry(&lease.plan, self.config.provider_registry.as_ref())
        {
            self.publish_kind(
                lease.profile_id,
                lease.prompt_id,
                lease.attempt_id,
                None,
                AttemptEventKind::Started,
                None,
            )?;
            self.publish_kind(
                lease.profile_id,
                lease.prompt_id,
                lease.attempt_id,
                None,
                AttemptEventKind::Failed { failure },
                None,
            )?;
            return Ok(());
        }
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
            handle_lease: None,
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
        AttemptEvent, AttemptState, CacheKey, ExecutionAttemptPersistence, ExecutionCommandAck,
        ExecutionCommandOutcome, ExecutionDataSource, ExecutionPresentationService,
        ExecutionSnapshotStatus, NativeHandleStoreGeneration, PersistedExecutionAttempt,
        PersistedExecutionProfile, PromptCompiler,
    };
    use comfy_types::{ApiPrompt, PromptId, PromptNode, PromptSubmission, RequestId, WorkerId};
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn native(value: Value) -> NativeValue {
        match value {
            Value::Null => NativeValue::Primitive {
                value: NativePrimitive::Null,
            },
            Value::Bool(value) => NativeValue::Primitive {
                value: NativePrimitive::Boolean(value),
            },
            Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    NativeValue::Primitive {
                        value: NativePrimitive::Integer(value),
                    }
                } else if let Some(value) = value.as_u64() {
                    NativeValue::Primitive {
                        value: NativePrimitive::UnsignedInteger(value),
                    }
                } else if let Some(value) = value.as_f64() {
                    NativeValue::Primitive {
                        value: NativePrimitive::Number(value),
                    }
                } else {
                    NativeValue::PreservedUnknown {
                        type_name: "sim.json-number@1".to_owned(),
                        value: Value::Number(value),
                    }
                }
            }
            Value::String(value) => NativeValue::Primitive {
                value: NativePrimitive::String(value),
            },
            value => NativeValue::PreservedUnknown {
                type_name: "sim.json@1".to_owned(),
                value,
            },
        }
    }

    #[test]
    fn provider_registry_pin_is_canonical_and_tamper_evident() {
        let pin = NativeProviderRegistryPin::checked(
            7,
            "a".repeat(64),
            vec!["b".repeat(64), "c".repeat(64)],
        )
        .expect("sorted provider registry pin is valid");
        let encoded = serde_json::to_vec(&pin).expect("provider registry pin serializes");
        let decoded: NativeProviderRegistryPin =
            serde_json::from_slice(&encoded).expect("provider registry pin deserializes");
        assert_eq!(decoded, pin);

        for invalid in [
            NativeProviderRegistryPin::checked(0, "a".repeat(64), vec!["b".repeat(64)]),
            NativeProviderRegistryPin::checked(1, "A".repeat(64), vec!["b".repeat(64)]),
            NativeProviderRegistryPin::checked(1, "a".repeat(64), Vec::new()),
            NativeProviderRegistryPin::checked(
                1,
                "a".repeat(64),
                vec!["c".repeat(64), "b".repeat(64)],
            ),
            NativeProviderRegistryPin::checked(
                1,
                "a".repeat(64),
                vec!["b".repeat(64), "b".repeat(64)],
            ),
        ] {
            assert!(matches!(invalid, Err(NativeImageRuntimeError::Encoding(_))));
        }

        let unknown_field = serde_json::json!({
            "generation": 7,
            "registry_digest_sha256": "a".repeat(64),
            "binding_digests_sha256": ["b".repeat(64)],
            "store_generation": 99,
        });
        assert!(
            serde_json::from_value::<NativeProviderRegistryPin>(unknown_field).is_err(),
            "provider registry pins reject unknown fields"
        );
        let changed = NativeProviderRegistryPin::checked(
            8,
            "a".repeat(64),
            vec!["b".repeat(64), "c".repeat(64)],
        )
        .expect("changed provider registry pin is valid");
        assert_ne!(pin.identity_sha256(), changed.identity_sha256());
        assert!(valid_provider_registry_digest(&pin.identity_sha256()));
    }

    fn compiled_provider_plan(
        pin: NativeProviderRegistryPin,
    ) -> Result<CompiledPlan, Box<dyn std::error::Error>> {
        let descriptor = comfy_nodes::NativeNodeDescriptor {
            schema_version: comfy_nodes::LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "ControllerProvider".to_owned(),
            implementation_version: "1".to_owned(),
            source_schema: None,
            inputs: Vec::new(),
            dynamic_inputs: Vec::new(),
            outputs: vec![comfy_nodes::NativeOutputDescriptor {
                name: "value".to_owned(),
                produced_type: NativeValueType::Primitive(NativePrimitiveType::Number),
                is_list: false,
            }],
            output_node: true,
            effect: comfy_nodes::NativeEffectClass::Provider,
            cache: comfy_nodes::NativeCachePolicy::Never,
        };
        let mut registry = NativeNodeRegistry::default();
        registry.register_descriptor(descriptor)?;
        Ok(PromptCompiler::new(&registry)
            .with_provider_registry_pin(pin)?
            .compile(PromptSubmission {
                prompt: ApiPrompt(BTreeMap::from([(
                    NodeId::from("provider"),
                    PromptNode {
                        class_type: "ControllerProvider".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )])),
                prompt_id: Some(PromptId(Uuid::from_u128(0x3710))),
                client_id: None,
                number: None,
                extra_data: BTreeMap::new(),
                unknown: BTreeMap::new(),
            })?)
    }

    #[test]
    fn controller_rejects_stale_provider_plan_before_dispatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let planned = NativeProviderRegistryPin::checked(7, "a".repeat(64), vec!["b".repeat(64)])?;
        let plan = compiled_provider_plan(planned.clone())?;
        assert!(validate_plan_provider_registry(&plan, Some(&planned)).is_ok());
        let current = NativeProviderRegistryPin::checked(8, "c".repeat(64), vec!["d".repeat(64)])?;
        let stale = validate_plan_provider_registry(&plan, Some(&current))
            .expect_err("changed provider deployment rejects queued plan");
        assert_eq!(stale.code, "native_provider_registry_stale");
        assert_eq!(
            validate_plan_provider_registry(&plan, None)
                .expect_err("missing provider deployment rejects queued plan")
                .code,
            "native_provider_registry_stale"
        );
        let mut worker_plan = NativeImageWorkerPlan::new(plan, BTreeMap::new(), true, 0)?;
        assert_eq!(worker_plan.provider_registry.as_ref(), Some(&planned));
        worker_plan.provider_registry = None;
        assert!(matches!(
            worker_plan.validate(),
            Err(NativeImageRuntimeError::Encoding(message))
                if message.contains("derived from the compiled plan")
        ));
        Ok(())
    }

    fn empty_worker_registry_deployment(
        generation: u64,
        registry_digest_sha256: &str,
    ) -> Result<crate::WorkerRegistryDeploymentPlan, Box<dyn std::error::Error>> {
        let begin = comfy_types::WorkerRegistryDeploymentBegin::new(
            comfy_types::WorkerRegistryGeneration::new(generation)?,
            comfy_types::WorkerSha256Digest::new(registry_digest_sha256.to_owned())?,
            Vec::new(),
        )?;
        let authorization_verifier = crate::PluginAuthorizationSealer::from_seed(
            [0x71; 32],
            crate::PermissionPolicyGeneration::new(1)?,
        )?
        .verifier()?;
        Ok(crate::WorkerRegistryDeploymentPlan::new(
            begin,
            Vec::new(),
            authorization_verifier,
        )?)
    }

    #[test]
    fn native_controller_requires_the_exact_provider_registry_deployment()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temporary, roots) = fixture_roots()?;
        let profile_id = ProfileId(Uuid::parse_str(&roots.profile_id)?);
        let mut presentation_service = ExecutionPresentationService::new(8)?;
        presentation_service.initialize_profile(
            profile_id,
            ExecutionDataSource::Live,
            ExecutionSnapshotStatus::Ready,
        )?;
        let presentation = crate::ExecutionPresentationOwner::ephemeral(presentation_service);
        let pin = NativeProviderRegistryPin::checked(7, "a".repeat(64), vec!["b".repeat(64)])?;
        let worker = || {
            WorkerLaunchConfig::new(
                PathBuf::from("unused-native-image-worker"),
                profile_id,
                WorkerId(Uuid::from_u128(0x1977)),
                NATIVE_IMAGE_REGISTRY_VERSION,
                1024,
            )
        };

        let config = NativeExecutionControllerConfig::new(
            fixture_asset_service(&roots)?,
            presentation.clone(),
            worker(),
            true,
        )?;
        assert!(matches!(
            config.with_provider_registry(pin.clone()),
            Err(NativeImageRuntimeError::Registry(message))
                if message.contains("requires a worker registry deployment")
        ));

        let mismatched_deployment = empty_worker_registry_deployment(8, &"a".repeat(64))?;
        let config = NativeExecutionControllerConfig::new(
            fixture_asset_service(&roots)?,
            presentation.clone(),
            worker().with_registry_deployment(mismatched_deployment),
            true,
        )?;
        assert!(matches!(
            config.with_provider_registry(pin.clone()),
            Err(NativeImageRuntimeError::Registry(message))
                if message.contains("differs from the worker registry deployment")
        ));

        let matching_deployment = empty_worker_registry_deployment(7, &"a".repeat(64))?;
        let config = NativeExecutionControllerConfig::new(
            fixture_asset_service(&roots)?,
            presentation,
            worker().with_registry_deployment(matching_deployment),
            true,
        )?
        .with_provider_registry(pin.clone())?;
        assert_eq!(config.provider_registry, Some(pin));
        Ok(())
    }

    struct FailOnceExecutionPersistence {
        database: crate::ComfyRuntimeDb,
        fail_next: Arc<AtomicBool>,
    }

    struct WorkspaceProbeDiffusionProvider {
        observed: Arc<AtomicBool>,
        cache_identity_calls: Arc<AtomicUsize>,
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
        let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1 << 20)?;
        let cache_identity_calls = Arc::new(AtomicUsize::new(0));
        let cancellation = CancellationToken::default();
        let state = NativeDiffusionState::checked(
            Arc::new(WorkspaceProbeDiffusionProvider {
                observed: Arc::new(AtomicBool::new(false)),
                cache_identity_calls: cache_identity_calls.clone(),
            }),
            Arc::new(backend),
            &cancellation,
        )?;
        cache_identity_calls.store(0, Ordering::SeqCst);
        let node = KSamplerNode {
            state: Arc::new(state),
        };
        let inputs = BTreeMap::from([
            ("model".to_owned(), native(json!("model-digest-a"))),
            (
                "positive".to_owned(),
                native(json!("conditioning-positive")),
            ),
            (
                "negative".to_owned(),
                native(json!("conditioning-negative")),
            ),
            ("cfg".to_owned(), native(json!(7.0))),
            ("seed".to_owned(), native(json!(1))),
        ]);
        let attempt_id = AttemptId(Uuid::from_u128(0x5101));
        let handle_generation = crate::NativeHandleStoreGeneration::new()?;
        let scratch = workspace_authority.authorize_workspace(1 << 20)?;
        let context = NodeContext::new(
            PromptId(Uuid::from_u128(0x5100)),
            attempt_id,
            NodeId("5".to_owned()),
            cancellation,
            scratch.clone(),
            handle_generation.handle_store_for_attempt(attempt_id),
        )?;
        let dependencies = node.cache_dependencies(&context, &inputs)?;
        assert_eq!(cache_identity_calls.load(Ordering::SeqCst), 1);
        assert_eq!(dependencies.artifact_digests.len(), 6);
        assert_eq!(
            dependencies.artifact_digests.get("conditioning.guidance"),
            Some(&"a".repeat(64))
        );
        let expected_rng_phase = format!("{INITIAL_NOISE_PHASE_ID}:1");
        assert_eq!(
            dependencies.rng_phase.as_deref(),
            Some(expected_rng_phase.as_str())
        );
        let change_token = node.cache_change_token(&inputs)?;
        assert_eq!(change_token, "stable");
        let key = |inputs: &BTreeMap<String, NativeValue>, token: &str| {
            CacheKey::from_inputs_with_dependencies(
                "KSampler",
                NATIVE_DIFFUSION_REGISTRY_VERSION,
                inputs,
                BTreeMap::new(),
                dependencies.artifact_digests.clone(),
                "cpu",
                "f32",
                dependencies.plugin_digest.clone(),
                dependencies.rng_phase.clone(),
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
                outputs: vec![native(json!("latent"))],
                ui: None,
            },
        );
        assert!(cache.get(&key(&inputs, &change_token)?).is_some());
        assert!(cache.get(&key(&inputs, "sim.comfy.guidance.v0")?).is_none());

        for (name, value) in [
            ("model", native(json!("model-digest-b"))),
            ("positive", native(json!("conditioning-other"))),
            ("negative", native(json!("conditioning-other"))),
            ("cfg", native(json!(1.0))),
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

        let mut changed_artifacts = dependencies.artifact_digests.clone();
        changed_artifacts.insert("conditioning.guidance".to_owned(), "f".repeat(64));
        assert_ne!(
            CacheKey::from_inputs_with_dependencies(
                "KSampler",
                NATIVE_DIFFUSION_REGISTRY_VERSION,
                &inputs,
                BTreeMap::new(),
                changed_artifacts,
                "cpu",
                "f32",
                dependencies.plugin_digest.clone(),
                dependencies.rng_phase.clone(),
                "configuration-v1",
                NATIVE_DIFFUSION_REGISTRY_VERSION,
                &change_token,
            )?,
            canonical
        );

        let cancelled = CancellationToken::default();
        cancelled.cancel();
        let cancelled_context = NodeContext::new(
            context.prompt_id,
            context.attempt_id,
            context.node_id,
            cancelled,
            scratch,
            handle_generation.handle_store_for_attempt(attempt_id),
        )?;
        let error = node
            .cache_dependencies(&cancelled_context, &inputs)
            .expect_err("cancelled cache identity discovery must fail");
        assert_eq!(cache_identity_calls.load(Ordering::SeqCst), 2);
        assert_eq!(error.code, "native_diffusion_cancelled");
        assert_eq!(error.kind, NodeFailureKind::Interrupted);
        assert!(error.retryable);
        Ok(())
    }

    #[test]
    fn load_image_cache_dependencies_hash_exact_bytes_and_preserve_typed_cancellation()
    -> Result<(), Box<dyn std::error::Error>> {
        let bytes = vec![0x5a; 128 * 1024 + 17];
        let logical_id = "input/cancellation-probe.png";
        let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1 << 20)?;
        let node = LoadImageNode {
            input_assets: Arc::new(Mutex::new(BTreeMap::from([(
                logical_id.to_owned(),
                bytes.clone(),
            )]))),
            cpu_backend: Arc::new(backend),
        };
        let inputs = BTreeMap::from([("image".to_owned(), native(json!(logical_id)))]);
        let cancellation = CancellationToken::default();
        let attempt_id = AttemptId(Uuid::from_u128(0x1a6f));
        let handle_generation = crate::NativeHandleStoreGeneration::new()?;
        let context = NodeContext::new(
            PromptId(Uuid::from_u128(0x1a6e)),
            attempt_id,
            NodeId("load-image".to_owned()),
            cancellation.clone(),
            workspace_authority.authorize_workspace(1 << 20)?,
            handle_generation.handle_store_for_attempt(attempt_id),
        )?;

        let dependencies = node.cache_dependencies(&context, &inputs)?;
        assert_eq!(
            dependencies.artifact_digests,
            BTreeMap::from([(
                logical_id.to_owned(),
                format!("{:x}", Sha256::digest(&bytes)),
            )])
        );

        cancellation.cancel();
        let error = node
            .cache_dependencies(&context, &inputs)
            .expect_err("pre-cancelled input identity discovery must fail");
        assert_eq!(error, cancellation_failure(CancellationError));
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
        fn cache_identities(
            &self,
            cancellation: &CancellationToken,
        ) -> Result<CanonicalNativeDiffusionCacheIdentities, NativeImageRuntimeError> {
            self.cache_identity_calls.fetch_add(1, Ordering::SeqCst);
            if cancellation.is_cancelled() {
                return Err(NativeImageRuntimeError::Cancelled);
            }
            let clip = CanonicalClipCacheIdentities::checked(
                "1".repeat(64),
                "2".repeat(64),
                "0".repeat(64),
                "3".repeat(64),
                "4".repeat(64),
                "5".repeat(64),
            )
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
            let vae = CanonicalVaeCacheIdentities::checked(
                "6".repeat(64),
                "0".repeat(64),
                "7".repeat(64),
                "8".repeat(64),
            )
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
            let conditioning = CanonicalConditioningCacheIdentities::checked(
                "9".repeat(64),
                "a".repeat(64),
                "b".repeat(64),
                "c".repeat(64),
                "d".repeat(64),
            )
            .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
            CanonicalNativeDiffusionCacheIdentities::checked(
                "0".repeat(64),
                "1".repeat(64),
                clip,
                vae,
                conditioning,
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
            descriptor.validate()?;
            assert_eq!(
                descriptor.implementation_version,
                NATIVE_IMAGE_REGISTRY_VERSION
            );
            assert!(registry.node(class_type).is_some());
            assert_eq!(
                registry.implementation_namespace(class_type),
                Some("sim.native_rust")
            );
            assert!(registry.binding_source(class_type).is_some());
        }
        Ok(())
    }

    #[test]
    fn generated_registry_is_comprehensive_and_preserves_union_frontend_types()
    -> Result<(), Box<dyn std::error::Error>> {
        let bindings = generated_family_node_bindings()?;
        let registry = generated_native_node_registry_projection(None)?;
        registry.validate_comprehensive_bindings()?;
        for class_type in [
            "LoadImage",
            "ImageScale",
            "ImageInvert",
            "PreviewImage",
            "SaveImage",
        ] {
            assert!(registry.descriptor(class_type).is_some());
            assert!(registry.node(class_type).is_some());
        }
        for binding in &bindings {
            let class_type = &binding.descriptor().class_type;
            assert_eq!(
                registry.binding_declared_disposition(class_type),
                Some(binding.disposition())
            );
            assert!(
                registry
                    .binding_source(class_type)
                    .is_some_and(|source| !source.is_empty())
            );
            assert!(registry.presentation(class_type).is_some());
        }

        let cpu_backend = projection_only_cpu_backend()?;
        let executor = NativeImageExecutor::new_with_generated_registry(
            ProfileId(Uuid::new_v4()),
            BTreeMap::new(),
            true,
            cpu_backend,
        )?;
        executor.nodes.validate_comprehensive_bindings()?;
        assert_eq!(executor.nodes.descriptor_len(), registry.descriptor_len());
        assert_eq!(executor.nodes.node_len(), registry.node_len());

        let frontend = generated_native_frontend_descriptors(None)?;
        assert_eq!(frontend.len(), registry.descriptor_len());
        let contracts = generated_native_frontend_contracts(None)?;
        assert_eq!(contracts.len(), registry.descriptor_len());
        for (class_type, contract) in &contracts {
            contract.runtime.validate_exact_schema_v2()?;
            assert_eq!(contract.graph.type_name, *class_type);
            assert_eq!(
                contract.runtime.source_schema.as_ref().map(|schema| schema
                    .inputs
                    .iter()
                    .map(|input| &input.name)
                    .collect::<Vec<_>>()),
                Some(
                    contract
                        .graph
                        .inputs
                        .iter()
                        .map(|input| &input.name)
                        .collect::<Vec<_>>()
                )
            );
            assert_eq!(
                contract.presentation,
                registry
                    .presentation(class_type)
                    .ok_or("frontend presentation is absent from the registry")?
                    .clone()
            );
            assert_eq!(
                Some(contract.disposition),
                registry.binding_disposition(class_type)
            );
        }
        if let Some((class_type, descriptor, input)) =
            registry.descriptors().find_map(|(class_type, descriptor)| {
                descriptor
                    .inputs
                    .iter()
                    .find(|input| input.accepted_types.members().len() > 1)
                    .map(|input| (class_type, descriptor, input))
            })
        {
            let frontend_descriptor = frontend
                .get(class_type)
                .ok_or("union descriptor was absent from frontend projection")?;
            let frontend_input = frontend_descriptor
                .inputs
                .iter()
                .find(|candidate| candidate.name == input.name)
                .ok_or("union input was absent from frontend projection")?;
            assert!(frontend_input.type_name.contains('|'));
            assert_eq!(frontend_descriptor.outputs.len(), descriptor.outputs.len());
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
        let state = NativeDiffusionState::checked(
            Arc::new(WorkspaceProbeDiffusionProvider {
                observed: observed.clone(),
                cache_identity_calls: Arc::new(AtomicUsize::new(0)),
            }),
            backend,
            &cancellation,
        )?;

        assert!(matches!(
            state.load_bundle(&context),
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
        assert_eq!(allocation.code, "native_image_resource_exhausted");
        assert_eq!(allocation.kind, NodeFailureKind::Failure);
        assert!(allocation.retryable);

        let codec = media_failure(PngError::Codec("injected codec failure".to_owned()));
        assert_eq!(codec.code, "native_image_codec_failed");
        assert_eq!(codec.kind, NodeFailureKind::Failure);
        assert!(!codec.retryable);
    }

    #[test]
    fn native_diffusion_failures_preserve_typed_cancellation_and_resource_exhaustion() {
        let cancelled = runtime_failure(NativeImageRuntimeError::Cancelled);
        assert_eq!(cancelled.code, "native_diffusion_cancelled");
        assert_eq!(cancelled.kind, NodeFailureKind::Interrupted);
        assert!(cancelled.retryable);

        let resource = runtime_failure(NativeImageRuntimeError::ResourceExhausted(
            "injected resource limit".to_owned(),
        ));
        assert_eq!(resource.code, "native_diffusion_resource_exhausted");
        assert_eq!(resource.kind, NodeFailureKind::Failure);
        assert!(resource.retryable);

        let model_resource = native_diffusion_model_failure(
            NativeDiffusionModelError::TensorBackend(TensorError::AllocationFailed {
                requested: 32,
                reason: "injected model allocation failure".to_owned(),
            }),
        );
        assert_eq!(model_resource.code, "native_diffusion_resource_exhausted");
        assert!(model_resource.retryable);

        let sampler_cancelled = native_diffusion_sampler_failure(
            NativeDiffusionSamplerError::Tensor(TensorError::Cancelled),
        );
        assert_eq!(sampler_cancelled.code, "native_diffusion_cancelled");
        assert_eq!(sampler_cancelled.kind, NodeFailureKind::Interrupted);

        let sampler_noise_resource = native_diffusion_sampler_failure(
            NativeDiffusionSamplerError::Noise(NoiseError::Tensor(TensorError::AllocationFailed {
                requested: 48,
                reason: "injected noise allocation failure".to_owned(),
            })),
        );
        assert_eq!(
            sampler_noise_resource.code,
            "native_diffusion_resource_exhausted"
        );
        assert!(sampler_noise_resource.retryable);

        let vae_resource = vae_failure(VaeError::Tensor(
            TensorError::WorkspaceAuthorizationExceeded {
                requested: 64,
                authorized: 32,
                in_use: 0,
            },
        ));
        assert_eq!(vae_resource.code, "native_diffusion_resource_exhausted");
        assert!(vae_resource.retryable);

        let vae_architecture_cancelled = vae_failure(VaeError::Architecture(
            VaeArchitectureError::Cancelled(CancellationError),
        ));
        assert_eq!(
            vae_architecture_cancelled.code,
            "native_diffusion_cancelled"
        );
        assert_eq!(
            vae_architecture_cancelled.kind,
            NodeFailureKind::Interrupted
        );
        assert!(vae_architecture_cancelled.retryable);

        let guidance_resource =
            runtime_diffusion_failure(map_guidance_runtime_error(GuidanceError::Conditioning(
                ConditioningError::Tensor(TensorError::ResourceLimitExceeded {
                    resource: "injected guidance resource",
                    limit: 1,
                }),
            )));
        assert_eq!(
            guidance_resource.code,
            "native_diffusion_resource_exhausted"
        );
        assert!(guidance_resource.retryable);

        assert!(matches!(
            map_conditioning_runtime_error(ConditioningError::ShapeView(
                ShapeLayoutTransformPartOneError::Cancelled,
            )),
            NativeImageRuntimeError::Cancelled
        ));
        assert!(matches!(
            map_conditioning_runtime_error(ConditioningError::TensorResize(
                ExternalTensorKernelPartOneError::Cancelled,
            )),
            NativeImageRuntimeError::Cancelled
        ));
        assert!(matches!(
            map_conditioning_runtime_error(ConditioningError::ShapeOperation(
                ShapeLayoutTransformPartTwoError::Cancelled,
            )),
            NativeImageRuntimeError::Cancelled
        ));
        assert!(matches!(
            map_conditioning_runtime_error(ConditioningError::TensorCast(
                OperatorIndirectionError::Cancelled,
            )),
            NativeImageRuntimeError::Cancelled
        ));

        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::Module(
                NativeOpsError::Tensor(OperatorIndirectionError::Cancelled),
            ))),
            NativeImageRuntimeError::Cancelled
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::Module(
                NativeOpsError::Functional(FunctionalError::AllocationFailed {
                    name: "clip layer norm",
                }),
            ))),
            NativeImageRuntimeError::ResourceExhausted(_)
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::Module(
                NativeOpsError::ModulePartTwo(NeuralNetworkModulePartTwoError::Tensor(
                    TensorError::WorkspaceAuthorizationExceeded {
                        requested: 64,
                        authorized: 32,
                        in_use: 0,
                    },
                )),
            ))),
            NativeImageRuntimeError::ResourceExhausted(_)
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::Module(
                NativeOpsError::Quantization(QuantizationError::MaterializationCapacity {
                    requested: 4096,
                }),
            ))),
            NativeImageRuntimeError::ResourceExhausted(_)
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::Module(
                NativeOpsError::Quantization(QuantizationError::Cancelled),
            ))),
            NativeImageRuntimeError::Cancelled
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::NativeDiffusion(
                NativeDiffusionTensorError::Functional(FunctionalError::Cancelled,)
            ),)),
            NativeImageRuntimeError::Cancelled
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::Allocation(
                "intermediate captures",
            ))),
            NativeImageRuntimeError::ResourceExhausted(_)
        ));
        assert!(matches!(
            map_clip_runtime_error(ClipError::TextTransformer(ClipTextError::ShapeLayout(
                ShapeLayoutTransformPartTwoError::Cancelled,
            ))),
            NativeImageRuntimeError::Cancelled
        ));

        let model_cancelled = native_diffusion_model_failure(NativeDiffusionModelError::Cancelled);
        assert_eq!(model_cancelled.code, "native_diffusion_cancelled");
        assert_eq!(model_cancelled.kind, NodeFailureKind::Interrupted);
        assert!(model_cancelled.retryable);

        let model_functional_resource =
            native_diffusion_model_failure(NativeDiffusionModelError::Tensor(
                NativeDiffusionTensorError::Functional(FunctionalError::AllocationFailed {
                    name: "UNet normalization",
                }),
            ));
        assert_eq!(
            model_functional_resource.code,
            "native_diffusion_resource_exhausted"
        );
        assert!(model_functional_resource.retryable);

        let vae_cancelled = vae_failure(VaeError::NativeOps(NativeOpsError::Cancelled));
        assert_eq!(vae_cancelled.code, "native_diffusion_cancelled");
        assert_eq!(vae_cancelled.kind, NodeFailureKind::Interrupted);
        assert!(vae_cancelled.retryable);

        let vae_native_resource = vae_failure(VaeError::NativeOps(NativeOpsError::ModulePartTwo(
            NeuralNetworkModulePartTwoError::Tensor(TensorError::AllocationFailed {
                requested: 128,
                reason: "injected VAE module allocation failure".to_owned(),
            }),
        )));
        assert_eq!(
            vae_native_resource.code,
            "native_diffusion_resource_exhausted"
        );
        assert!(vae_native_resource.retryable);

        let control_cancelled = control_runtime_error(ControlNetError::Cancelled);
        assert!(matches!(
            control_cancelled,
            NativeImageRuntimeError::Cancelled
        ));
        let control_cancelled = runtime_diffusion_failure(control_cancelled);
        assert_eq!(control_cancelled.code, "native_diffusion_cancelled");
        assert_eq!(control_cancelled.kind, NodeFailureKind::Interrupted);
        assert!(control_cancelled.retryable);

        let control_resource =
            control_runtime_error(ControlNetError::Tensor(TensorError::AllocationFailed {
                requested: 96,
                reason: "injected ControlNet allocation failure".to_owned(),
            }));
        assert!(matches!(
            control_resource,
            NativeImageRuntimeError::ResourceExhausted(_)
        ));
        let control_resource = runtime_diffusion_failure(control_resource);
        assert_eq!(control_resource.code, "native_diffusion_resource_exhausted");
        assert_eq!(control_resource.kind, NodeFailureKind::Failure);
        assert!(control_resource.retryable);

        let wrapped_control_resource = control_runtime_error(ControlNetError::CanonicalTensor(
            Box::new(TensorError::WorkspaceAuthorizationExceeded {
                requested: 128,
                authorized: 64,
                in_use: 0,
            }),
        ));
        assert!(matches!(
            wrapped_control_resource,
            NativeImageRuntimeError::ResourceExhausted(_)
        ));

        let mut renamed_cancelled = cancelled_diffusion_failure("renamed cancellation");
        renamed_cancelled.code = "renamed_code".to_owned();
        assert!(matches!(
            node_failure_runtime_error(renamed_cancelled),
            NativeImageRuntimeError::Cancelled
        ));
        let mut renamed_resource = resource_diffusion_failure("renamed resource failure");
        renamed_resource.code = "renamed_code".to_owned();
        assert!(matches!(
            node_failure_runtime_error(renamed_resource),
            NativeImageRuntimeError::ResourceExhausted(_)
        ));

        let invalid = runtime_failure(NativeImageRuntimeError::Execution(
            "injected invalid request".to_owned(),
        ));
        assert_eq!(invalid.code, "native_tensor_handle_failed");
        assert_eq!(invalid.kind, NodeFailureKind::Failure);
        assert!(!invalid.retryable);
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
        let first = NativeTensorPayload::from_image(NativeTensorRole::Image, tensor.clone())?;
        let second = NativeTensorPayload::from_image(NativeTensorRole::Image, tensor)?;
        assert_eq!(first.projection(), second.projection());
        let mask_tensor = ImageTensor::from_f32(&cpu_backend, &tensor_context, 1, 1, 1, 1, &[0.5])?;
        let mask = NativeTensorPayload::from_image(NativeTensorRole::Mask, mask_tensor)?;
        assert_ne!(first.projection(), mask.projection());
        assert!(
            NativeTensorPayload::from_tensor(NativeTensorRole::Image, raw_tensor.clone()).is_err()
        );
        let conditioning_descriptor = TensorDescriptor::contiguous(
            vec![1, 1, 3],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let (conditioning_tensor, _) =
            cpu_backend.upload_f32(conditioning_descriptor, &[0.0, 0.5, 1.0], &tensor_context)?;
        let raw_payload =
            NativeTensorPayload::from_tensor(NativeTensorRole::Conditioning, conditioning_tensor)?;
        let latent_payload =
            NativeTensorPayload::from_tensor(NativeTensorRole::Latent, raw_tensor)?;
        assert_ne!(raw_payload.projection(), latent_payload.projection());
        let mut invalid_raw_digest = serde_json::to_value(raw_payload.projection())?;
        invalid_raw_digest["content_digest"] = json!("g".repeat(64));
        assert!(
            serde_json::from_value::<comfy_tensor::NativeTensorProjection>(invalid_raw_digest)
                .is_err()
        );

        let mut invalid_wire = serde_json::to_value(first.projection())?;
        let strides = invalid_wire
            .pointer_mut("/descriptor/strides")
            .ok_or("native tensor handle descriptor strides are unavailable")?;
        *strides = json!([0, 0, 0, 0]);
        assert!(
            serde_json::from_value::<comfy_tensor::NativeTensorProjection>(invalid_wire).is_err()
        );

        let mut invalid_schema = serde_json::to_value(first.projection())?;
        invalid_schema["schema_version"] =
            json!(comfy_tensor::NATIVE_TENSOR_PROJECTION_SCHEMA_VERSION + 1);
        assert!(
            serde_json::from_value::<comfy_tensor::NativeTensorProjection>(invalid_schema).is_err()
        );

        let mut unknown_field = serde_json::to_value(first.projection())?;
        unknown_field["future"] = json!(true);
        assert!(
            serde_json::from_value::<comfy_tensor::NativeTensorProjection>(unknown_field).is_err()
        );
        Ok(())
    }

    #[test]
    fn prepared_tensor_handles_remain_unpublished_when_final_cancellation_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (cpu_backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024)?;
        let workspace = workspace_authority.authorize_workspace(1024)?;
        let tensor_context =
            cpu_backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);
        let image =
            ImageTensor::from_f32(&cpu_backend, &tensor_context, 1, 1, 1, 3, &[0.0, 0.5, 1.0])?;
        let image_payload = NativeTensorPayload::from_image(NativeTensorRole::Image, image)?;
        let conditioning_descriptor = TensorDescriptor::contiguous(
            vec![1, 1, 3],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let (conditioning_tensor, _) =
            cpu_backend.upload_f32(conditioning_descriptor, &[0.0, 0.5, 1.0], &tensor_context)?;
        let raw_payload =
            NativeTensorPayload::from_tensor(NativeTensorRole::Conditioning, conditioning_tensor)?;
        serde_json::to_value(image_payload.projection())?;
        serde_json::to_value(raw_payload.projection())?;

        cancellation.cancel();
        assert!(tensor_context.check().is_err());

        let generation = NativeHandleStoreGeneration::new()?;
        let attempt_id = AttemptId(Uuid::from_u128(1));
        let store = generation.handle_store_for_attempt(attempt_id);
        let result = store.publish(
            NativeStoredPayload::Tensor(Arc::new(image_payload)),
            &cancellation,
        );
        assert!(matches!(result, Err(NativeHandleStoreError::Cancelled)));
        assert!(generation.is_empty());
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
                    handle_lease: None,
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

        let profile_id = ProfileId(Uuid::from_u128(31));
        let prompt_id = PromptId(Uuid::from_u128(32));
        let attempt_id = AttemptId(Uuid::from_u128(33));
        let event = AttemptEvent {
            profile_id,
            prompt_id,
            attempt_id,
            sequence: 0,
            node_id: Some(NodeId("worker".to_owned())),
            at: Utc::now(),
            kind: AttemptEventKind::Started,
            data: None,
        };
        let mut result = NativeImageWorkerResult::from_execution_report(
            ExecutionReport {
                profile_id,
                prompt_id,
                attempt_id,
                state: AttemptState::Succeeded,
                outputs: BTreeMap::from([(NodeId("worker".to_owned()), vec![native(json!(1))])]),
                ui_outputs: BTreeMap::new(),
                events: vec![event.clone()],
                cache_hits: 0,
                error: None,
                handle_lease: None,
            },
            Vec::new(),
            1,
        )?;
        assert!(result.report.outputs.is_empty());
        assert!(result.report.events.is_empty());

        result.report.events.push(event);
        assert!(matches!(
            result.decode_ui_outputs(),
            Err(NativeImageRuntimeError::WorkerEvent(message))
                if message.contains("process-local outputs or events")
        ));

        for key in [
            "service_id",
            "reference_id",
            "request_digest_sha256",
            "scratch_binding",
            "backend_id",
            "authority_id",
        ] {
            let report = ExecutionReport {
                profile_id,
                prompt_id,
                attempt_id,
                state: AttemptState::Succeeded,
                outputs: BTreeMap::new(),
                ui_outputs: BTreeMap::from([(
                    NodeId("worker".to_owned()),
                    json!({"nested": [{(key): "private"}]}),
                )]),
                events: Vec::new(),
                cache_hits: 0,
                error: None,
                handle_lease: None,
            };
            assert!(matches!(
                NativeImageWorkerResult::from_execution_report(report, Vec::new(), 1),
                Err(NativeImageRuntimeError::Encoding(message))
                    if message.contains("private capability key")
            ));
        }
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
        assert_eq!(
            first.report.ui_outputs.get(&NodeId("4".to_owned())),
            Some(&json!({
                "images": [{
                    "transaction_id": first
                        .output_proposals
                        .iter()
                        .find(|proposal| {
                            proposal.output.namespace() == AssetNamespace::Temporary
                        })
                        .ok_or("preview output")?
                        .proposal_id(),
                    "batch_index": 0,
                    "type": "temp",
                }],
                "animated": [false],
            }))
        );
        assert!(
            fs::read_dir(roots.test_root_path(AssetNamespace::Output)?)?
                .next()
                .is_none(),
            "worker-side execution must not publish a final file"
        );
        let preview = first
            .output_proposals
            .iter()
            .find(|proposal| proposal.output.namespace() == AssetNamespace::Temporary)
            .ok_or("preview output")?;
        let preview_png = decode_png(preview.output.content(), PngLimits::default())?;
        assert_eq!((preview_png.width, preview_png.height), (4, 2));
        assert_eq!(preview_png.metadata.comfy_metadata(), Default::default());
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
