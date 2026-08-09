use comfy_model::{
    NativeModelBackingKind, NativeModelPayload, NativeModelPayloadError, NativeModelResourceRole,
    NativeVae, PatchGraph,
    conditioning::{ConditioningError, ConditioningIdentity},
    controlnet::{
        ControlChain, ControlConditioning, ControlIsolation, ControlModelExecutor, ControlNetError,
        ControlResult, ControlRuntime, ControlTensorBinding,
    },
    generated_native_diffusion::{
        Sd15TinyModel, sd15_latent_format_identity, sd15_model_family_identity,
    },
};
use comfy_tensor::{
    CpuBackend, ExecutionContext, Tensor,
    generated_native_diffusion::{NativeDiffusionTensorError, tensor_from_f32},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, mem, sync::Arc};
use thiserror::Error;

use crate::GUIDANCE_ADAPTER_ID;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeDiffusionResidentAllocationId {
    ModelPayloadArc {
        address: usize,
    },
    ModelBacking {
        kind: NativeModelBackingKind,
        address: usize,
    },
    ConditioningPayloadArc {
        address: usize,
    },
    PatchGraphArc {
        address: usize,
    },
    ControlExecutionArc {
        address: usize,
    },
    ControlChainArc {
        address: usize,
    },
    ControlExecutorArc {
        address: usize,
    },
    TensorStorage {
        storage_id: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDiffusionResidentAllocation {
    id: NativeDiffusionResidentAllocationId,
    resident_bytes: u64,
}

impl NativeDiffusionResidentAllocation {
    pub fn id(&self) -> &NativeDiffusionResidentAllocationId {
        &self.id
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDiffusionResidentParts {
    owned_bytes: u64,
    shared_allocations: Vec<NativeDiffusionResidentAllocation>,
}

impl NativeDiffusionResidentParts {
    fn checked(
        owned_bytes: u64,
        allocations: impl IntoIterator<Item = NativeDiffusionResidentAllocation>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        let mut unique = BTreeMap::new();
        for allocation in allocations {
            if let Some(existing) = unique.insert(allocation.id, allocation.resident_bytes)
                && existing != allocation.resident_bytes
            {
                return Err(NativeDiffusionPayloadError::ResidentAllocationChanged);
            }
        }
        let shared_allocations = unique
            .into_iter()
            .map(|(id, resident_bytes)| NativeDiffusionResidentAllocation { id, resident_bytes })
            .collect::<Vec<_>>();
        let parts = Self {
            owned_bytes,
            shared_allocations,
        };
        parts.resident_bytes()?;
        Ok(parts)
    }

    pub const fn owned_bytes(&self) -> u64 {
        self.owned_bytes
    }

    pub fn shared_allocations(&self) -> &[NativeDiffusionResidentAllocation] {
        &self.shared_allocations
    }

    pub fn resident_bytes(&self) -> Result<u64, NativeDiffusionPayloadError> {
        self.shared_allocations
            .iter()
            .try_fold(self.owned_bytes, |bytes, allocation| {
                bytes
                    .checked_add(allocation.resident_bytes)
                    .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)
            })
    }
}

#[derive(Clone)]
pub struct NativeControlExecution {
    chain: Arc<ControlChain>,
    executor: Arc<dyn ControlModelExecutor>,
    vae: Option<Arc<NativeVae>>,
    vae_execution_digest: Option<String>,
    execution_digest: String,
}

#[derive(Clone)]
pub struct NativeControlPayload {
    role: NativeModelResourceRole,
    execution: Arc<NativeControlExecution>,
    digest_sha256: String,
}

impl NativeControlPayload {
    pub fn checked(
        role: NativeModelResourceRole,
        execution: Arc<NativeControlExecution>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        if !matches!(role, NativeModelResourceRole::ControlNet) {
            return Err(NativeDiffusionPayloadError::RoleMismatch);
        }
        execution.validate()?;
        let digest_sha256 = sha256_tagged(
            "sim.comfy.native-control-payload.v1",
            [
                role.source_type_id().as_bytes(),
                execution.execution_digest().as_bytes(),
            ],
        );
        Ok(Self {
            role,
            execution,
            digest_sha256,
        })
    }

    pub const fn role(&self) -> NativeModelResourceRole {
        self.role
    }

    pub fn execution(&self) -> &Arc<NativeControlExecution> {
        &self.execution
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub fn resident_bytes(&self) -> Result<usize, NativeDiffusionPayloadError> {
        usize::try_from(self.resident_parts()?.resident_bytes()?)
            .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)
    }

    pub fn resident_parts(
        &self,
    ) -> Result<NativeDiffusionResidentParts, NativeDiffusionPayloadError> {
        let owned_bytes = resident_size::<Self>()?
            .checked_add(resident_capacity(&self.digest_sha256)?)
            .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        let mut allocations = vec![NativeDiffusionResidentAllocation {
            id: NativeDiffusionResidentAllocationId::ControlExecutionArc {
                address: arc_address(&self.execution),
            },
            resident_bytes: self.execution.owned_resident_bytes()?,
        }];
        allocations.extend(self.execution.shared_resident_allocations()?);
        NativeDiffusionResidentParts::checked(owned_bytes, allocations)
    }

    pub fn validate(&self) -> Result<(), NativeDiffusionPayloadError> {
        let expected = Self::checked(self.role, self.execution.clone())?;
        if expected.digest_sha256 != self.digest_sha256 {
            return Err(NativeDiffusionPayloadError::Invalid(
                "native ControlNet payload identity changed".to_owned(),
            ));
        }
        Ok(())
    }
}

impl NativeControlExecution {
    pub fn checked(
        chain: Arc<ControlChain>,
        executor: Arc<dyn ControlModelExecutor>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        Self::checked_inner(chain, executor, None)
    }

    pub fn checked_with_vae(
        chain: Arc<ControlChain>,
        executor: Arc<dyn ControlModelExecutor>,
        vae: Arc<NativeVae>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        Self::checked_inner(chain, executor, Some(vae))
    }

    fn checked_inner(
        chain: Arc<ControlChain>,
        executor: Arc<dyn ControlModelExecutor>,
        vae: Option<Arc<NativeVae>>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        let executor_digest = executor.execution_digest();
        validate_sha256("ControlNet executor identity", executor_digest)?;
        chain
            .require_executor_digest(executor_digest)
            .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?;
        let vae_execution_digest = vae.as_ref().map(|vae| vae.execution_digest());
        if let Some(digest) = &vae_execution_digest {
            validate_sha256("ControlNet VAE execution identity", digest)?;
        }
        chain
            .require_vae_execution_digest(vae_execution_digest.as_deref())
            .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?;
        let vae_binding_digest = vae_execution_digest.as_deref().map_or_else(
            || sha256_tagged("sim.comfy.controlnet.vae-binding.absent.v1", []),
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
            vae,
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

    pub fn resident_bytes(&self) -> Result<u64, NativeDiffusionPayloadError> {
        NativeDiffusionResidentParts::checked(
            self.owned_resident_bytes()?,
            self.shared_resident_allocations()?,
        )?
        .resident_bytes()
    }

    fn owned_resident_bytes(&self) -> Result<u64, NativeDiffusionPayloadError> {
        let mut bytes = resident_size::<Self>()?
            .checked_add(resident_capacity(&self.execution_digest)?)
            .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        if let Some(digest) = &self.vae_execution_digest {
            bytes = bytes
                .checked_add(resident_capacity(digest)?)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        }
        Ok(bytes)
    }

    fn embedded_owned_resident_bytes(&self) -> Result<u64, NativeDiffusionPayloadError> {
        self.owned_resident_bytes()?
            .checked_sub(resident_size::<Self>()?)
            .ok_or(NativeDiffusionPayloadError::ResidentAllocationChanged)
    }

    fn shared_resident_allocations(
        &self,
    ) -> Result<Vec<NativeDiffusionResidentAllocation>, NativeDiffusionPayloadError> {
        let chain_parts = self
            .chain
            .resident_parts()
            .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?;
        let mut allocations = vec![
            NativeDiffusionResidentAllocation {
                id: NativeDiffusionResidentAllocationId::ControlChainArc {
                    address: arc_address(&self.chain),
                },
                resident_bytes: chain_parts.owned_bytes(),
            },
            NativeDiffusionResidentAllocation {
                id: NativeDiffusionResidentAllocationId::ControlExecutorArc {
                    address: trait_arc_address(&self.executor),
                },
                resident_bytes: self
                    .executor
                    .resident_bytes()
                    .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
            },
        ];
        allocations.extend(chain_parts.tensor_allocations().iter().map(|allocation| {
            NativeDiffusionResidentAllocation {
                id: NativeDiffusionResidentAllocationId::TensorStorage {
                    storage_id: allocation.storage_id().get(),
                },
                resident_bytes: allocation.resident_bytes(),
            }
        }));
        if let Some(vae) = &self.vae {
            allocations.push(NativeDiffusionResidentAllocation {
                id: NativeDiffusionResidentAllocationId::ModelBacking {
                    kind: NativeModelBackingKind::NativeVae,
                    address: arc_address(vae),
                },
                resident_bytes: vae
                    .resident_bytes()
                    .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
            });
        }
        Ok(allocations)
    }

    pub fn vae_execution_digest(&self) -> Option<&str> {
        self.vae_execution_digest.as_deref()
    }

    pub fn validate(&self) -> Result<(), NativeDiffusionPayloadError> {
        let executor_digest = self.executor.execution_digest();
        validate_sha256("ControlNet executor identity", executor_digest)?;
        if self.execution_digest
            != Self::checked_inner(self.chain.clone(), self.executor.clone(), self.vae.clone())?
                .execution_digest
        {
            return Err(NativeDiffusionPayloadError::Invalid(
                "ControlNet execution identity changed".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct NativeConditioningPayload {
    identity: ConditioningIdentity,
    identity_digest: String,
    patch_graph: Arc<PatchGraph>,
    model_execution_digest: String,
    control: Option<NativeControlExecution>,
    execution_digest: String,
}

impl NativeConditioningPayload {
    pub fn checked_sd15(
        model_digest: &str,
        model: &Sd15TinyModel,
        patch_graph: Arc<PatchGraph>,
        control: Option<NativeControlExecution>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        validate_sha256("model artifact identity", model_digest)?;
        patch_graph
            .identity()
            .validate_for_base(model_digest)
            .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?;
        if &patch_graph.identity() != model.patch_identity() {
            return Err(NativeDiffusionPayloadError::Invalid(
                "model patch identity does not match its prebound graph".to_owned(),
            ));
        }
        validate_sha256("model execution identity", model.patch_execution_digest())?;
        let identity = ConditioningIdentity::new(
            "sd15-native-diffusion",
            sd15_model_family_identity()
                .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
            sd15_latent_format_identity()
                .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
        )?;
        let identity_digest = identity.digest()?;
        let control_digest = control.as_ref().map_or_else(
            || sha256_tagged("sim.comfy.controlnet.absent.v1", []),
            |control| control.execution_digest().to_owned(),
        );
        let execution_digest = sha256_tagged(
            "sim.comfy.conditioning.execution.v1",
            [
                identity_digest.as_bytes(),
                GUIDANCE_ADAPTER_ID.as_bytes(),
                patch_graph.identity().ordered_digest.as_bytes(),
                model.patch_execution_digest().as_bytes(),
                control_digest.as_bytes(),
            ],
        );
        Ok(Self {
            identity,
            identity_digest,
            patch_graph,
            model_execution_digest: model.patch_execution_digest().to_owned(),
            control,
            execution_digest,
        })
    }

    pub fn identity(&self) -> &ConditioningIdentity {
        &self.identity
    }

    pub fn identity_digest(&self) -> &str {
        &self.identity_digest
    }

    pub fn patch_graph(&self) -> &Arc<PatchGraph> {
        &self.patch_graph
    }

    pub fn model_execution_digest(&self) -> &str {
        &self.model_execution_digest
    }

    pub fn control(&self) -> Option<&NativeControlExecution> {
        self.control.as_ref()
    }

    pub fn execution_digest(&self) -> &str {
        &self.execution_digest
    }

    pub fn resident_bytes(&self) -> Result<u64, NativeDiffusionPayloadError> {
        self.resident_parts()?.resident_bytes()
    }

    pub fn resident_parts(
        &self,
    ) -> Result<NativeDiffusionResidentParts, NativeDiffusionPayloadError> {
        let mut owned_bytes = resident_size::<Self>()?;
        let identity_owned_bytes = self
            .identity
            .resident_bytes()?
            .checked_sub(resident_size::<ConditioningIdentity>()?)
            .ok_or(NativeDiffusionPayloadError::ResidentAllocationChanged)?;
        for bytes in [
            identity_owned_bytes,
            resident_capacity(&self.identity_digest)?,
            resident_capacity(&self.model_execution_digest)?,
            resident_capacity(&self.execution_digest)?,
        ] {
            owned_bytes = owned_bytes
                .checked_add(bytes)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        }
        let mut allocations = vec![NativeDiffusionResidentAllocation {
            id: NativeDiffusionResidentAllocationId::PatchGraphArc {
                address: arc_address(&self.patch_graph),
            },
            resident_bytes: self
                .patch_graph
                .resident_bytes()
                .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
        }];
        if let Some(control) = &self.control {
            owned_bytes = owned_bytes
                .checked_add(control.embedded_owned_resident_bytes()?)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
            allocations.extend(control.shared_resident_allocations()?);
        }
        NativeDiffusionResidentParts::checked(owned_bytes, allocations)
    }

    pub fn validate(&self) -> Result<(), NativeDiffusionPayloadError> {
        validate_sha256("conditioning identity", &self.identity_digest)?;
        validate_sha256("conditioning execution identity", &self.execution_digest)?;
        validate_sha256("model execution identity", &self.model_execution_digest)?;
        if let Some(control) = &self.control {
            control.validate()?;
        }
        self.resident_parts()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_control(
        &self,
        backend: &CpuBackend,
        latent: &Tensor,
        model_time: f32,
        cross_attention: &Tensor,
        sampling_percent: f64,
        context: &ExecutionContext<'_>,
    ) -> Result<Option<ControlResult>, ControlNetError> {
        let Some(control) = &self.control else {
            context.check().map_err(|_| ControlNetError::Cancelled)?;
            return Ok(None);
        };
        control
            .validate()
            .map_err(|error| ControlNetError::Invalid(error.to_string()))?;
        let timestep =
            tensor_from_f32(backend, &[1], &[model_time], context).map_err(
                |error| match error {
                    NativeDiffusionTensorError::Tensor(error) => ControlNetError::Tensor(error),
                    error => ControlNetError::CanonicalTensor(Box::new(error)),
                },
            )?;
        let binding = |tensor: Tensor| -> Result<ControlTensorBinding, ControlNetError> {
            let digest = format!("{:x}", Sha256::digest(tensor.contiguous_bytes()?));
            ControlTensorBinding::checked(tensor, digest)
        };
        let batch = latent
            .descriptor()
            .shape()
            .first()
            .copied()
            .ok_or_else(|| {
                ControlNetError::Invalid("canonical ControlNet latent has no batch axis".to_owned())
            })?;
        let conditioning = ControlConditioning::checked(
            binding(latent.clone())?,
            binding(timestep)?,
            binding(cross_attention.clone())?,
            None,
            Vec::new(),
            None,
            BTreeMap::new(),
            sampling_percent as f32,
            batch,
        )?;
        ControlRuntime::new(backend, control.executor.as_ref()).execute(
            &control.chain,
            ControlIsolation::CompleteChain,
            &conditioning,
            control.vae.as_deref(),
            context,
        )
    }
}

#[derive(Clone)]
pub struct NativeDiffusionPayload {
    resource: NativeDiffusionResource,
    digest_sha256: String,
}

#[derive(Clone)]
enum NativeDiffusionResource {
    Model {
        model: Arc<NativeModelPayload>,
        conditioning: Arc<NativeConditioningPayload>,
    },
    Clip {
        clip: Arc<NativeModelPayload>,
    },
    Vae {
        vae: Arc<NativeModelPayload>,
    },
}

impl NativeDiffusionPayload {
    pub fn model(
        model: Arc<NativeModelPayload>,
        conditioning: Arc<NativeConditioningPayload>,
    ) -> Result<Self, NativeDiffusionPayloadError> {
        require_role(&model, NativeModelResourceRole::Model)?;
        model.validate()?;
        conditioning.validate()?;
        let model_resource = model
            .model()
            .ok_or(NativeDiffusionPayloadError::RoleMismatch)?;
        if model_resource.patch_identity() != &conditioning.patch_graph().identity()
            || model_resource.patch_execution_digest() != conditioning.model_execution_digest()
        {
            return Err(NativeDiffusionPayloadError::Invalid(
                "conditioning does not belong to the model payload".to_owned(),
            ));
        }
        let digest_sha256 = sha256_tagged(
            "sim.comfy.native-diffusion-model-payload.v1",
            [
                model.identity().digest_sha256().as_bytes(),
                conditioning.execution_digest().as_bytes(),
            ],
        );
        Ok(Self {
            resource: NativeDiffusionResource::Model {
                model,
                conditioning,
            },
            digest_sha256,
        })
    }

    pub fn clip(clip: Arc<NativeModelPayload>) -> Result<Self, NativeDiffusionPayloadError> {
        require_role(&clip, NativeModelResourceRole::Clip)?;
        clip.validate()?;
        let digest_sha256 = sha256_tagged(
            "sim.comfy.native-diffusion-clip-payload.v1",
            [clip.identity().digest_sha256().as_bytes()],
        );
        Ok(Self {
            resource: NativeDiffusionResource::Clip { clip },
            digest_sha256,
        })
    }

    pub fn vae(vae: Arc<NativeModelPayload>) -> Result<Self, NativeDiffusionPayloadError> {
        require_role(&vae, NativeModelResourceRole::Vae)?;
        vae.validate()?;
        let digest_sha256 = sha256_tagged(
            "sim.comfy.native-diffusion-vae-payload.v1",
            [vae.identity().digest_sha256().as_bytes()],
        );
        Ok(Self {
            resource: NativeDiffusionResource::Vae { vae },
            digest_sha256,
        })
    }

    pub const fn role(&self) -> NativeModelResourceRole {
        match &self.resource {
            NativeDiffusionResource::Model { .. } => NativeModelResourceRole::Model,
            NativeDiffusionResource::Clip { .. } => NativeModelResourceRole::Clip,
            NativeDiffusionResource::Vae { .. } => NativeModelResourceRole::Vae,
        }
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub fn model_payload(&self) -> &Arc<NativeModelPayload> {
        match &self.resource {
            NativeDiffusionResource::Model { model, .. } => model,
            NativeDiffusionResource::Clip { clip } => clip,
            NativeDiffusionResource::Vae { vae } => vae,
        }
    }

    pub fn model_resources(
        &self,
    ) -> Option<(&Arc<NativeModelPayload>, &Arc<NativeConditioningPayload>)> {
        match &self.resource {
            NativeDiffusionResource::Model {
                model,
                conditioning,
            } => Some((model, conditioning)),
            NativeDiffusionResource::Clip { .. } | NativeDiffusionResource::Vae { .. } => None,
        }
    }

    pub fn resident_bytes(&self) -> Result<usize, NativeDiffusionPayloadError> {
        usize::try_from(self.resident_parts()?.resident_bytes()?)
            .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)
    }

    pub fn resident_parts(
        &self,
    ) -> Result<NativeDiffusionResidentParts, NativeDiffusionPayloadError> {
        let owned_bytes = resident_size::<Self>()?
            .checked_add(resident_capacity(&self.digest_sha256)?)
            .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        let mut allocations = Vec::new();
        match &self.resource {
            NativeDiffusionResource::Model {
                model,
                conditioning,
            } => {
                append_model_allocations(&mut allocations, model)?;
                append_conditioning_allocations(&mut allocations, conditioning)?;
            }
            NativeDiffusionResource::Clip { clip } => {
                append_model_allocations(&mut allocations, clip)?;
            }
            NativeDiffusionResource::Vae { vae } => {
                append_model_allocations(&mut allocations, vae)?;
            }
        }
        NativeDiffusionResidentParts::checked(owned_bytes, allocations)
    }

    pub fn validate(&self) -> Result<(), NativeDiffusionPayloadError> {
        let expected = match &self.resource {
            NativeDiffusionResource::Model {
                model,
                conditioning,
                ..
            } => Self::model(model.clone(), conditioning.clone())?,
            NativeDiffusionResource::Clip { clip } => Self::clip(clip.clone())?,
            NativeDiffusionResource::Vae { vae } => Self::vae(vae.clone())?,
        };
        if expected.digest_sha256 != self.digest_sha256 {
            return Err(NativeDiffusionPayloadError::Invalid(
                "native diffusion payload identity changed".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum NativeDiffusionPayloadError {
    #[error(transparent)]
    Model(#[from] NativeModelPayloadError),
    #[error(transparent)]
    Conditioning(#[from] ConditioningError),
    #[error("native diffusion payload role does not match its concrete resource")]
    RoleMismatch,
    #[error("native diffusion payload is invalid: {0}")]
    Invalid(String),
    #[error("native diffusion payload resident byte accounting overflowed")]
    ResidentBytesOverflow,
    #[error("native diffusion payload resident allocation identity changed its byte count")]
    ResidentAllocationChanged,
}

fn resident_size<Value>() -> Result<u64, NativeDiffusionPayloadError> {
    u64::try_from(mem::size_of::<Value>())
        .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)
}

fn resident_capacity(value: &String) -> Result<u64, NativeDiffusionPayloadError> {
    u64::try_from(value.capacity()).map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)
}

fn arc_address<Value>(value: &Arc<Value>) -> usize {
    Arc::as_ptr(value) as usize
}

fn trait_arc_address(value: &Arc<dyn ControlModelExecutor>) -> usize {
    Arc::as_ptr(value) as *const () as usize
}

fn append_model_allocations(
    allocations: &mut Vec<NativeDiffusionResidentAllocation>,
    model: &Arc<NativeModelPayload>,
) -> Result<(), NativeDiffusionPayloadError> {
    let parts = model.resident_parts()?;
    allocations.push(NativeDiffusionResidentAllocation {
        id: NativeDiffusionResidentAllocationId::ModelPayloadArc {
            address: arc_address(model),
        },
        resident_bytes: parts.owned_bytes(),
    });
    allocations.extend(parts.backing_allocations().iter().map(|allocation| {
        NativeDiffusionResidentAllocation {
            id: NativeDiffusionResidentAllocationId::ModelBacking {
                kind: allocation.kind(),
                address: allocation.address(),
            },
            resident_bytes: allocation.resident_bytes(),
        }
    }));
    allocations.extend(parts.tensor_allocations().iter().map(|allocation| {
        NativeDiffusionResidentAllocation {
            id: NativeDiffusionResidentAllocationId::TensorStorage {
                storage_id: allocation.storage_id().get(),
            },
            resident_bytes: allocation.resident_bytes(),
        }
    }));
    Ok(())
}

fn append_conditioning_allocations(
    allocations: &mut Vec<NativeDiffusionResidentAllocation>,
    conditioning: &Arc<NativeConditioningPayload>,
) -> Result<(), NativeDiffusionPayloadError> {
    let parts = conditioning.resident_parts()?;
    allocations.push(NativeDiffusionResidentAllocation {
        id: NativeDiffusionResidentAllocationId::ConditioningPayloadArc {
            address: arc_address(conditioning),
        },
        resident_bytes: parts.owned_bytes(),
    });
    allocations.extend_from_slice(parts.shared_allocations());
    Ok(())
}

fn require_role(
    payload: &NativeModelPayload,
    expected: NativeModelResourceRole,
) -> Result<(), NativeDiffusionPayloadError> {
    if payload.identity().role() == expected {
        Ok(())
    } else {
        Err(NativeDiffusionPayloadError::RoleMismatch)
    }
}

fn validate_sha256(subject: &str, digest: &str) -> Result<(), NativeDiffusionPayloadError> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(NativeDiffusionPayloadError::Invalid(format!(
            "{subject} is not a SHA-256 digest"
        )))
    }
}

fn sha256_tagged<const N: usize>(tag: &str, fields: [&[u8]; N]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tag.as_bytes());
    hasher.update([0]);
    for field in fields {
        hasher.update(field);
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_model::{
        ModelFamilyIdentity, PatchGraphIdentity,
        controlnet::{
            ControlBase, ControlHintPreprocess, ControlModelBinding, ControlModelInput, ControlNet,
            ControlNode, ControlPercentWindow, StrengthType,
        },
    };
    use comfy_tensor::{
        CancellationToken, CpuWorkspaceAuthority, DType, DeviceId, StreamId,
        generated_native_diffusion::tensor_from_f32,
    };
    use std::{
        error::Error,
        sync::atomic::{AtomicBool, Ordering},
    };

    const EXECUTOR_DIGEST: &str =
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const CHANGED_EXECUTOR_DIGEST: &str =
        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    struct MutableIdentityExecutor {
        changed: AtomicBool,
    }

    impl ControlModelExecutor for MutableIdentityExecutor {
        fn execution_digest(&self) -> &str {
            if self.changed.load(Ordering::SeqCst) {
                CHANGED_EXECUTOR_DIGEST
            } else {
                EXECUTOR_DIGEST
            }
        }

        fn resident_bytes(&self) -> Result<u64, ControlNetError> {
            u64::try_from(mem::size_of::<Self>())
                .map_err(|_| ControlNetError::ResidentBytesOverflow)
        }

        fn execute_controlnet(
            &self,
            _binding: &ControlModelBinding,
            _input: &ControlModelInput,
            _context: &ExecutionContext<'_>,
        ) -> Result<ControlResult, ControlNetError> {
            Err(ControlNetError::Invalid("not exercised".to_owned()))
        }

        fn execute_t2i_adapter(
            &self,
            _binding: &ControlModelBinding,
            _hint: &Tensor,
            _context: &ExecutionContext<'_>,
        ) -> Result<ControlResult, ControlNetError> {
            Err(ControlNetError::Invalid("not exercised".to_owned()))
        }
    }

    #[test]
    fn control_payload_binds_executor_identity_and_accounts_retained_resources()
    -> Result<(), Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(1024 * 1024)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let hint = tensor_from_f32(&backend, &[1, 1, 1, 1], &[1.0], &context)?;
        let model_digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let model = ControlModelBinding::checked(
            ModelFamilyIdentity::new("COMFY-MODEL-0001", "control", "v1")?,
            PatchGraphIdentity {
                schema_version: comfy_model::PATCH_GRAPH_SCHEMA_VERSION,
                base_artifact_digest: model_digest.to_owned(),
                ordered_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
            },
            model_digest,
            EXECUTOR_DIGEST,
            DType::F32,
            DeviceId::CPU,
        )?;
        let control = ControlNet::checked(
            ControlBase::checked(
                1.0,
                StrengthType::Constant,
                ControlPercentWindow::checked(0.0, 1.0)?,
                false,
                None,
            )?,
            model,
            ControlTensorBinding::checked(
                hint,
                "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            )?,
            1,
            comfy_tensor::ResizeMode::NearestExact,
            ControlHintPreprocess::Identity,
            None,
            Vec::new(),
            false,
            Vec::new(),
        )?;
        let chain = Arc::new(ControlChain::checked(vec![ControlNode::ControlNet(
            control,
        )])?);
        let executor = Arc::new(MutableIdentityExecutor {
            changed: AtomicBool::new(false),
        });
        let execution = Arc::new(NativeControlExecution::checked(
            chain.clone(),
            executor.clone(),
        )?);
        let payload =
            NativeControlPayload::checked(NativeModelResourceRole::ControlNet, execution.clone())?;
        assert!(payload.resident_bytes()? > usize::try_from(chain.resident_bytes()?)?);
        let aliased =
            NativeControlPayload::checked(NativeModelResourceRole::ControlNet, execution.clone())?;
        assert_eq!(payload.resident_parts()?, aliased.resident_parts()?);

        let distinct_chain = Arc::new(ControlChain::checked(chain.nodes().to_vec())?);
        let distinct_execution = Arc::new(NativeControlExecution::checked(
            distinct_chain,
            executor.clone(),
        )?);
        let distinct =
            NativeControlPayload::checked(NativeModelResourceRole::ControlNet, distinct_execution)?;
        let shared_parts = payload.resident_parts()?;
        let distinct_parts = distinct.resident_parts()?;
        assert_ne!(shared_parts, distinct_parts);
        let shared_ids = shared_parts
            .shared_allocations()
            .iter()
            .map(NativeDiffusionResidentAllocation::id)
            .collect::<Vec<_>>();
        let distinct_ids = distinct_parts
            .shared_allocations()
            .iter()
            .map(NativeDiffusionResidentAllocation::id)
            .collect::<Vec<_>>();
        assert!(shared_ids.iter().any(|identity| {
            matches!(
                identity,
                NativeDiffusionResidentAllocationId::ControlChainArc { .. }
            ) && !distinct_ids.contains(identity)
        }));
        assert!(shared_ids.iter().any(|identity| {
            matches!(
                identity,
                NativeDiffusionResidentAllocationId::ControlExecutorArc { .. }
            ) && distinct_ids.contains(identity)
        }));
        assert!(shared_ids.iter().any(|identity| {
            matches!(
                identity,
                NativeDiffusionResidentAllocationId::TensorStorage { .. }
            ) && distinct_ids.contains(identity)
        }));
        assert!(shared_ids.iter().any(|identity| {
            matches!(
                identity,
                NativeDiffusionResidentAllocationId::ControlExecutionArc { .. }
            ) && !distinct_ids.contains(identity)
        }));
        assert_eq!(
            u64::try_from(payload.resident_bytes()?)?,
            shared_parts.resident_bytes()?
        );
        payload.validate()?;

        executor.changed.store(true, Ordering::SeqCst);
        assert!(matches!(
            execution.validate(),
            Err(NativeDiffusionPayloadError::Invalid(_))
        ));
        Ok(())
    }

    #[test]
    fn diffusion_resident_parts_sort_deduplicate_and_reject_changed_bytes()
    -> Result<(), Box<dyn Error>> {
        let chain = NativeDiffusionResidentAllocationId::ControlChainArc { address: 7 };
        let model = NativeDiffusionResidentAllocationId::ModelPayloadArc { address: 3 };
        let parts = NativeDiffusionResidentParts::checked(
            11,
            [
                NativeDiffusionResidentAllocation {
                    id: chain.clone(),
                    resident_bytes: 13,
                },
                NativeDiffusionResidentAllocation {
                    id: model.clone(),
                    resident_bytes: 17,
                },
                NativeDiffusionResidentAllocation {
                    id: chain.clone(),
                    resident_bytes: 13,
                },
            ],
        )?;
        assert_eq!(parts.shared_allocations().len(), 2);
        assert_eq!(parts.shared_allocations()[0].id(), &model);
        assert_eq!(parts.shared_allocations()[1].id(), &chain);
        assert_eq!(parts.resident_bytes()?, 41);

        assert!(matches!(
            NativeDiffusionResidentParts::checked(
                0,
                [
                    NativeDiffusionResidentAllocation {
                        id: chain.clone(),
                        resident_bytes: 13,
                    },
                    NativeDiffusionResidentAllocation {
                        id: chain,
                        resident_bytes: 14,
                    },
                ],
            ),
            Err(NativeDiffusionPayloadError::ResidentAllocationChanged)
        ));
        Ok(())
    }
}
