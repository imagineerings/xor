use comfy_model::{
    ArtifactIndex, ArtifactKey, ArtifactRoot, ModelStore, ParserLimits,
    clip::NativeTokenizer,
    generated_native_diffusion::{
        NativeDiffusionModelError, Sd1Tokenizer, Sd15DetectorProjection, Sd15TinyModel,
        admit_reduced_fixture, load_sd15_tokenizer,
    },
};
use comfy_runtime::{NativeDiffusionBundle, NativeDiffusionProvider, NativeImageRuntimeError};
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
        index.refresh(context.cancellation)?;
        let key = ArtifactKey::new("sd15-tiny-fixture", "model.safetensors")?;
        let mut store = ModelStore::new(ParserLimits::default())?;
        let loaded = store.load(&index, &key, context.cancellation)?;
        Ok(Sd15TinyModel::load_reduced_fixture(
            &store, &index, &loaded, &admission, backend, context,
        )?)
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

    fn load(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionBundle, NativeImageRuntimeError> {
        let model_digest = self.model_digest()?;
        let tokenizer = self
            .tokenizer()
            .map_err(|error| NativeImageRuntimeError::Asset(error.to_string()))?;
        let model = self
            .load_model_with_context(backend, context)
            .map_err(|error| NativeImageRuntimeError::Execution(error.to_string()))?;
        NativeDiffusionBundle::new(
            "sd15-tiny-v1",
            model_digest,
            Arc::new(model),
            Arc::new(tokenizer),
        )
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
}
