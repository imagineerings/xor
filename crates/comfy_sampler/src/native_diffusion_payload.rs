use comfy_model::{
    NativeModelPayload, NativeModelPayloadError, NativeModelResourceRole, NativeVae, PatchGraph,
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
        let wrapper_bytes = u64::try_from(mem::size_of::<Self>())
            .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        usize::try_from(
            wrapper_bytes
                .checked_add(self.execution.resident_bytes()?)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?,
        )
        .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)
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
        let mut bytes = u64::try_from(std::mem::size_of::<Self>())
            .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        let chain_bytes = self
            .chain
            .resident_bytes()
            .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?;
        let executor_bytes = self
            .executor
            .resident_bytes()
            .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?;
        bytes = bytes
            .checked_add(chain_bytes)
            .and_then(|bytes| bytes.checked_add(executor_bytes))
            .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        if let Some(vae) = &self.vae {
            bytes = bytes
                .checked_add(
                    vae.resident_storage_bytes()
                        .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
                )
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        }
        Ok(bytes)
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
        let mut bytes = u64::try_from(std::mem::size_of::<Self>())
            .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)?
            .checked_add(
                self.patch_graph
                    .resident_bytes()
                    .map_err(|error| NativeDiffusionPayloadError::Invalid(error.to_string()))?,
            )
            .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        if let Some(control) = &self.control {
            bytes = bytes
                .checked_add(control.resident_bytes()?)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        }
        Ok(bytes)
    }

    pub fn validate(&self) -> Result<(), NativeDiffusionPayloadError> {
        validate_sha256("conditioning identity", &self.identity_digest)?;
        validate_sha256("conditioning execution identity", &self.execution_digest)?;
        validate_sha256("model execution identity", &self.model_execution_digest)?;
        if let Some(control) = &self.control {
            control.validate()?;
        }
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
        let resource_bytes = match &self.resource {
            NativeDiffusionResource::Model {
                model,
                conditioning,
            } => model
                .resident_bytes()
                .checked_add(conditioning.resident_bytes()?)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?,
            NativeDiffusionResource::Clip { clip } => clip.resident_bytes(),
            NativeDiffusionResource::Vae { vae } => vae.resident_bytes(),
        };
        let wrapper_bytes = u64::try_from(mem::size_of::<Self>())
            .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)?;
        usize::try_from(
            wrapper_bytes
                .checked_add(resource_bytes)
                .ok_or(NativeDiffusionPayloadError::ResidentBytesOverflow)?,
        )
        .map_err(|_| NativeDiffusionPayloadError::ResidentBytesOverflow)
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
        payload.validate()?;

        executor.changed.store(true, Ordering::SeqCst);
        assert!(matches!(
            execution.validate(),
            Err(NativeDiffusionPayloadError::Invalid(_))
        ));
        Ok(())
    }
}
