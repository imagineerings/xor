use comfy_model::{
    ArtifactIndex, ArtifactKey, ArtifactRecord, ArtifactRoot, ModelStore, ParserLimits,
    clip::NativeTokenizer,
    generated_native_diffusion::{
        NativeDiffusionModelError, Sd1Tokenizer, Sd15DetectorProjection, Sd15TinyModel,
        admit_reduced_fixture, bind_sd15_clip_execution, bind_sd15_vae_execution,
        load_sd15_clip_execution, load_sd15_tokenizer, load_sd15_vae_execution,
    },
};
use comfy_runtime::{
    CanonicalClipCacheIdentities, CanonicalVaeCacheIdentities, NativeDiffusionBundle,
    NativeDiffusionProvider, NativeImageRuntimeError,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, ExecutionContext, StreamId,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

// Cache dependency discovery runs before an execution context exists, so the checked-in fixture
// pins the digest produced by its materialized canonical VAE instead of privately loading it again.
const SD15_TINY_VAE_EXECUTION_DIGEST: &str =
    "31b853b46be2e3335f6d397bdd907ddba0c695ae0b0fe9ccdcf3175a1305bd40";

#[derive(Clone, Debug)]
pub struct NativeDiffusionFixture {
    root: PathBuf,
}

impl NativeDiffusionFixture {
    pub fn checked_in() -> Self {
        Self {
            root: Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/models/sd15-tiny-v1"),
        }
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>, NativeDiffusionFixtureError> {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name == "."
            || name == ".."
        {
            return Err(NativeDiffusionFixtureError::UnsafeName(name.to_owned()));
        }
        Ok(fs::read(self.root.join(name))?)
    }

    pub fn tokenizer(&self) -> Result<Sd1Tokenizer, NativeDiffusionFixtureError> {
        let vocabulary = String::from_utf8(self.read("vocab.json")?)?;
        let merges = String::from_utf8(self.read("merges.txt")?)?;
        Ok(load_sd15_tokenizer(&vocabulary, &merges)?)
    }

    pub fn detector_projection(
        &self,
    ) -> Result<Sd15DetectorProjection, NativeDiffusionFixtureError> {
        Ok(serde_json::from_slice(
            &self.read("sd15-detector-projection.json")?,
        )?)
    }

    pub fn load_model(
        &self,
        memory_limit_bytes: u64,
        cancellation: &CancellationToken,
    ) -> Result<Sd15TinyModel, NativeDiffusionFixtureError> {
        let (backend, workspace_authority) =
            CpuWorkspaceAuthority::create_backend(memory_limit_bytes)?;
        let backend = Arc::new(backend);
        let scratch = workspace_authority.authorize_workspace(memory_limit_bytes)?;
        let context = backend.execution_context(StreamId::DEFAULT, scratch, cancellation);
        self.load_model_with_context(backend, &context)
    }

    pub fn load_model_with_context(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<Sd15TinyModel, NativeDiffusionFixtureError> {
        let (store, index, loaded, _artifact, _projection, admission) =
            self.load_checkpoint(context.cancellation)?;
        Ok(Sd15TinyModel::load_reduced_fixture(
            &store, &index, &loaded, &admission, backend, context,
        )?)
    }

    pub fn load_bundle_with_context(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionBundle, NativeDiffusionFixtureError> {
        let tokenizer = Arc::new(self.tokenizer()?);
        let (store, index, loaded, artifact, projection, admission) =
            self.load_checkpoint(context.cancellation)?;
        let model = Arc::new(Sd15TinyModel::load_reduced_fixture(
            &store,
            &index,
            &loaded,
            &admission,
            backend.clone(),
            context,
        )?);
        let clip = Arc::new(load_sd15_clip_execution(
            &store,
            &index,
            &loaded,
            &projection,
            tokenizer.identity().clone(),
            backend.clone(),
            context,
        )?);
        let vae = Arc::new(load_sd15_vae_execution(
            &store,
            &index,
            loaded.clone(),
            &artifact,
            &projection,
            &backend,
            context,
        )?);
        NativeDiffusionBundle::new_with_vae(
            "sd15-tiny-v1",
            loaded.identity(),
            model,
            tokenizer,
            clip,
            vae,
        )
        .map_err(|error| NativeDiffusionFixtureError::Runtime(error.to_string()))
    }

    pub fn load_clip_with_context(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<comfy_model::clip::LoadedSd1Clip, NativeDiffusionFixtureError> {
        let tokenizer = self.tokenizer()?;
        let (store, index, loaded, _artifact, projection, _admission) =
            self.load_checkpoint(context.cancellation)?;
        Ok(load_sd15_clip_execution(
            &store,
            &index,
            &loaded,
            &projection,
            tokenizer.identity().clone(),
            backend,
            context,
        )?)
    }

    fn load_checkpoint(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<
        (
            ModelStore,
            ArtifactIndex,
            Arc<comfy_model::LoadedModel>,
            ArtifactRecord,
            Sd15DetectorProjection,
            comfy_model::generated_native_diffusion::ReducedFixtureAdmission,
        ),
        NativeDiffusionFixtureError,
    > {
        let detector_bytes = self.read("sd15-detector-projection.json")?;
        let projection: Sd15DetectorProjection = serde_json::from_slice(&detector_bytes)?;
        projection.detect()?;
        let digest = format!("{:x}", Sha256::digest(&detector_bytes));
        let admission = admit_reduced_fixture("sd15-tiny-v1", &digest)?;
        let mut index = ArtifactIndex::default();
        index.add_root(ArtifactRoot::canonical(
            "sd15-tiny-fixture",
            "checkpoint",
            &self.root,
            ["safetensors"],
        )?)?;
        index.refresh(cancellation)?;
        let key = ArtifactKey::new("sd15-tiny-fixture", "model.safetensors")?;
        let artifact = index
            .record(&key)
            .cloned()
            .ok_or_else(|| comfy_model::ArtifactIndexError::Missing(key.clone()))?;
        let mut store = ModelStore::new(ParserLimits::default())?;
        let loaded = store.load(&index, &key, cancellation)?;
        Ok((store, index, loaded, artifact, projection, admission))
    }
}

impl NativeDiffusionProvider for NativeDiffusionFixture {
    fn model_digest(&self) -> Result<String, NativeImageRuntimeError> {
        Ok(format!(
            "{:x}",
            Sha256::digest(
                self.read("model.safetensors")
                    .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?
            )
        ))
    }

    fn tokenizer_digest(&self) -> Result<String, NativeImageRuntimeError> {
        Ok(self
            .tokenizer()
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?
            .identity()
            .digest()
            .to_owned())
    }

    fn clip_cache_identities(
        &self,
    ) -> Result<CanonicalClipCacheIdentities, NativeImageRuntimeError> {
        let tokenizer = self
            .tokenizer()
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        let projection = self
            .detector_projection()
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        let model_digest = self.model_digest()?;
        let (_, binding) =
            bind_sd15_clip_execution(&projection, &model_digest, tokenizer.identity().clone())
                .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        CanonicalClipCacheIdentities::checked(
            tokenizer.identity().digest(),
            binding.architecture().digest(),
            binding.plan().artifact_identity().as_str(),
            binding.plan().model_identity().as_str(),
            binding.plan().patch_identity().as_str(),
            binding.plan().digest(),
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
    }

    fn vae_cache_identities(&self) -> Result<CanonicalVaeCacheIdentities, NativeImageRuntimeError> {
        let cancellation = CancellationToken::default();
        let (_store, _index, _loaded, artifact, projection, _admission) = self
            .load_checkpoint(&cancellation)
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        let descriptor = bind_sd15_vae_execution(&projection, &artifact)
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        let identity = descriptor.identity();
        CanonicalVaeCacheIdentities::checked(
            identity.digest(),
            identity.artifact_sha256(),
            &identity.patch().ordered_digest,
            SD15_TINY_VAE_EXECUTION_DIGEST,
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))
    }

    fn load(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionBundle, NativeImageRuntimeError> {
        self.load_bundle_with_context(backend, context)
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))
    }
}

#[derive(Debug, Error)]
pub enum NativeDiffusionFixtureError {
    #[error("unsafe native diffusion fixture filename {0:?}")]
    UnsafeName(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Model(#[from] NativeDiffusionModelError),
    #[error(transparent)]
    Index(#[from] comfy_model::ArtifactIndexError),
    #[error(transparent)]
    Store(#[from] comfy_model::ModelStoreError),
    #[error(transparent)]
    Tensor(#[from] comfy_tensor::TensorError),
    #[error("native diffusion runtime fixture error: {0}")]
    Runtime(String),
}
