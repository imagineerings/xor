use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    net::IpAddr,
    time::{Duration, Instant},
};

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::{Host, Url};
use zeroize::Zeroizing;

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
use comfy_model::ArtifactRoot;
use comfy_plugin_sdk::{
    ED25519_PUBLIC_KEY_BYTES, ED25519_SIGNATURE_BYTES, PLUGIN_SIGNATURE_ALGORITHM,
    PluginContractError, PluginManifest, TypeRegistry,
};
use comfy_tensor::CancellationToken;
#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
use comfy_types::CancellationError;

#[cfg(any(target_os = "linux", target_os = "windows", test))]
use std::io::{Seek, SeekFrom, Write};

use crate::{
    AuthorizedCapabilities, CapabilitySet, PermissionError, PermissionPolicy,
    PermissionPolicyGeneration,
};

pub const SEALED_PLUGIN_AUTHORIZATION_VERSION: u16 = 2;
pub const MAX_SEALED_PLUGIN_AUTHORIZATION_BYTES: usize = 2 * 1024 * 1024;
pub const CUDART_LIBRARY_ID: &str = "nvidia-cudart";
const PLUGIN_AUTHORIZATION_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-plugin-authorization-v2\0";
const PROVIDER_COST_ACCEPTANCE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-provider-cost-acceptance-v2\0";
const MAX_PROVIDER_COST_ACCEPTANCE_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
const ROCM_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-rocm-package-v1\0";
const METAL_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-metal-package-v1\0";
const MLU_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-mlu-package-v1\0";
const NPU_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-npu-package-v1\0";
const CUDA_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-cuda-package-v1\0";
const XPU_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-xpu-package-v1\0";
const DIRECTML_PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"sim-comfy-directml-package-v1\0";
const NATIVE_PACKAGE_SIGNATURE_ALGORITHM: &str = "ed25519";
const MAX_NATIVE_PACKAGE_SIGNATURE_RECEIPT_BYTES: usize = 1_024;
#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
const MAX_NATIVE_LIBRARY_IMAGE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
const NATIVE_LIBRARY_IMAGE_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Restricted,
    Blocked,
}

#[derive(Clone, Eq, PartialEq)]
pub struct PluginVerificationKey {
    key_id: String,
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl PluginVerificationKey {
    pub fn new(key_id: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        let key_id = key_id.into();
        let public_key = key
            .as_ref()
            .try_into()
            .map_err(|_| TrustError::InvalidVerificationKey)?;
        if !valid_ascii_identifier(&key_id, 1_024) {
            return Err(TrustError::InvalidVerificationKey);
        }
        Ok(Self { key_id, public_key })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.public_key
    }
}

impl fmt::Debug for PluginVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PluginVerificationKey")
            .field("key_id", &self.key_id)
            .field("public_key", &encode_hex(&self.public_key))
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct RocmPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NativePackageSignatureReceipt {
    schema_version: u16,
    algorithm: String,
    signature: String,
}

#[derive(Clone, Eq, PartialEq)]
pub struct MetalPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Clone, Eq, PartialEq)]
pub struct MluPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Clone, Eq, PartialEq)]
pub struct NpuPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Clone, Eq, PartialEq)]
pub struct CudaPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Clone, Eq, PartialEq)]
pub struct XpuPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Clone, Eq, PartialEq)]
pub struct DirectMlPackageVerificationKey {
    authority: NativePackageVerificationAuthority,
}

#[derive(Clone, Eq, PartialEq)]
struct NativePackageVerificationAuthority {
    signer: String,
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl NativePackageVerificationAuthority {
    fn new(
        signer: impl Into<String>,
        key: impl AsRef<[u8]>,
        invalid_key: TrustError,
    ) -> Result<Self, TrustError> {
        let signer = signer.into();
        let public_key = key.as_ref().try_into().map_err(|_| invalid_key.clone())?;
        if !valid_ascii_identifier(&signer, 256) {
            return Err(invalid_key);
        }
        Ok(Self { signer, public_key })
    }

    fn verify(
        &self,
        domain: &[u8],
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
        unknown_signer: TrustError,
        invalid_signature: TrustError,
    ) -> Result<(), TrustError> {
        if signer != self.signer {
            return Err(unknown_signer);
        }
        if receipt_bytes.is_empty()
            || receipt_bytes.len() > MAX_NATIVE_PACKAGE_SIGNATURE_RECEIPT_BYTES
        {
            return Err(invalid_signature);
        }
        let receipt_value =
            parse_strict_json_value(receipt_bytes).map_err(|_| invalid_signature.clone())?;
        let receipt: NativePackageSignatureReceipt =
            serde_json::from_value(receipt_value).map_err(|_| invalid_signature.clone())?;
        if receipt.schema_version != 1 || receipt.algorithm != NATIVE_PACKAGE_SIGNATURE_ALGORITHM {
            return Err(invalid_signature);
        }
        let signature = decode_hex_exact::<ED25519_SIGNATURE_BYTES>(&receipt.signature)
            .ok_or_else(|| invalid_signature.clone())?;
        let mut canonical_receipt =
            serde_json::to_vec(&receipt).map_err(|_| invalid_signature.clone())?;
        canonical_receipt.push(b'\n');
        if receipt_bytes != canonical_receipt {
            return Err(invalid_signature);
        }
        let signing_payload =
            package_signing_payload(domain, signer, coverage, invalid_signature.clone())?;
        UnparsedPublicKey::new(&ED25519, self.public_key)
            .verify(&signing_payload, &signature)
            .map_err(|_| invalid_signature)
    }
}

impl RocmPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidRocmPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            ROCM_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownRocmPackageSigner,
            TrustError::InvalidRocmPackageSignature,
        )
    }
}

impl MetalPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidMetalPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            METAL_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownMetalPackageSigner,
            TrustError::InvalidMetalPackageSignature,
        )
    }
}

impl MluPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidMluPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            MLU_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownMluPackageSigner,
            TrustError::InvalidMluPackageSignature,
        )
    }
}

impl NpuPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidNpuPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            NPU_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownNpuPackageSigner,
            TrustError::InvalidNpuPackageSignature,
        )
    }
}

impl XpuPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidXpuPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            XPU_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownXpuPackageSigner,
            TrustError::InvalidXpuPackageSignature,
        )
    }
}

impl CudaPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidCudaPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            CUDA_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownCudaPackageSigner,
            TrustError::InvalidCudaPackageSignature,
        )
    }
}

impl DirectMlPackageVerificationKey {
    pub fn new(signer: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, TrustError> {
        Ok(Self {
            authority: NativePackageVerificationAuthority::new(
                signer,
                key,
                TrustError::InvalidDirectMlPackageVerificationKey,
            )?,
        })
    }

    pub fn signer(&self) -> &str {
        &self.authority.signer
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.authority.public_key
    }

    pub fn verify_package(
        &self,
        signer: &str,
        coverage: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<(), TrustError> {
        self.authority.verify(
            DIRECTML_PACKAGE_SIGNATURE_DOMAIN,
            signer,
            coverage,
            receipt_bytes,
            TrustError::UnknownDirectMlPackageSigner,
            TrustError::InvalidDirectMlPackageSignature,
        )
    }
}

impl fmt::Debug for RocmPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RocmPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

impl fmt::Debug for MetalPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MetalPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

impl fmt::Debug for MluPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MluPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

impl fmt::Debug for NpuPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NpuPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

impl fmt::Debug for XpuPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("XpuPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

impl fmt::Debug for CudaPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CudaPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

impl fmt::Debug for DirectMlPackageVerificationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectMlPackageVerificationKey")
            .field("signer", &self.authority.signer)
            .field("public_key", &encode_hex(&self.authority.public_key))
            .finish()
    }
}

#[cfg(test)]
pub(crate) fn rocm_package_signing_payload(
    signer: &str,
    coverage: &[u8],
) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        ROCM_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidRocmPackageSignature,
    )
}

#[cfg(any(test, feature = "signing-tooling"))]
pub fn metal_package_signing_payload(signer: &str, coverage: &[u8]) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        METAL_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidMetalPackageSignature,
    )
}

#[cfg(any(test, feature = "signing-tooling"))]
pub fn mlu_package_signing_payload(signer: &str, coverage: &[u8]) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        MLU_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidMluPackageSignature,
    )
}

#[cfg(any(test, feature = "signing-tooling"))]
pub fn npu_package_signing_payload(signer: &str, coverage: &[u8]) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        NPU_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidNpuPackageSignature,
    )
}

#[cfg(any(test, feature = "signing-tooling"))]
pub fn xpu_package_signing_payload(signer: &str, coverage: &[u8]) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        XPU_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidXpuPackageSignature,
    )
}

#[cfg(any(test, feature = "signing-tooling"))]
pub fn cuda_package_signing_payload(signer: &str, coverage: &[u8]) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        CUDA_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidCudaPackageSignature,
    )
}

#[cfg(any(test, feature = "signing-tooling"))]
pub fn directml_package_signing_payload(
    signer: &str,
    coverage: &[u8],
) -> Result<Vec<u8>, TrustError> {
    package_signing_payload(
        DIRECTML_PACKAGE_SIGNATURE_DOMAIN,
        signer,
        coverage,
        TrustError::InvalidDirectMlPackageSignature,
    )
}

fn package_signing_payload(
    domain: &[u8],
    signer: &str,
    coverage: &[u8],
    invalid_signature: TrustError,
) -> Result<Vec<u8>, TrustError> {
    let signer_length = u64::try_from(signer.len()).map_err(|_| invalid_signature.clone())?;
    let coverage_length = u64::try_from(coverage.len()).map_err(|_| invalid_signature.clone())?;
    let capacity = domain
        .len()
        .checked_add(16)
        .and_then(|length| length.checked_add(signer.len()))
        .and_then(|length| length.checked_add(coverage.len()))
        .ok_or_else(|| invalid_signature.clone())?;
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(capacity)
        .map_err(|_| invalid_signature)?;
    payload.extend_from_slice(domain);
    payload.extend_from_slice(&signer_length.to_be_bytes());
    payload.extend_from_slice(signer.as_bytes());
    payload.extend_from_slice(&coverage_length.to_be_bytes());
    payload.extend_from_slice(coverage);
    Ok(payload)
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativePackagePayloadLimit {
    path: &'static str,
    maximum_bytes: usize,
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
#[derive(Debug, Error)]
pub(crate) enum NativeLibraryImageError {
    #[error("native-library image capture was cancelled")]
    Cancelled,
    #[error("native-library image capture or sealing is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("native-library image is invalid: {0}")]
    Invalid(String),
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
impl From<CancellationError> for NativeLibraryImageError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
pub(crate) struct CapturedNativeLibraryImage {
    bytes: Vec<u8>,
    digest_sha256: String,
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
impl CapturedNativeLibraryImage {
    #[cfg(any(test, feature = "rocm"))]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub(crate) fn seal(
        self,
        snapshot_name: &str,
        cancellation: &CancellationToken,
    ) -> Result<RetainedNativeLibraryImage, NativeLibraryImageError> {
        self.seal_with_check(snapshot_name, || cancellation.check())
    }

    pub(crate) fn seal_with_check(
        self,
        snapshot_name: &str,
        mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
    ) -> Result<RetainedNativeLibraryImage, NativeLibraryImageError> {
        seal_native_library_image(&self.bytes, snapshot_name, &mut check_cancellation)
    }
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
#[cfg_attr(
    not(all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )),
    allow(dead_code)
)]
pub(crate) struct RetainedNativeLibraryImage {
    _file: File,
    loader_path: PathBuf,
    #[cfg(all(target_os = "windows", any(test, feature = "cuda", feature = "xpu")))]
    _temporary_directory: tempfile::TempDir,
    #[cfg(all(test, not(any(target_os = "linux", target_os = "windows"))))]
    _temporary_path: tempfile::TempPath,
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
#[cfg_attr(
    not(all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )),
    allow(dead_code)
)]
impl RetainedNativeLibraryImage {
    pub(crate) fn loader_path(&self) -> &Path {
        &self.loader_path
    }

    #[cfg(test)]
    pub(crate) fn file(&self) -> &File {
        &self._file
    }
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
pub(crate) fn capture_native_library_image(
    path: &Path,
    cancellation: &CancellationToken,
) -> Result<CapturedNativeLibraryImage, NativeLibraryImageError> {
    capture_native_library_image_with_check(path, || cancellation.check())
}

#[cfg(any(
    test,
    feature = "mlu",
    feature = "npu",
    feature = "rocm",
    feature = "cuda",
    feature = "xpu"
))]
pub(crate) fn capture_native_library_image_with_check(
    path: &Path,
    mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
) -> Result<CapturedNativeLibraryImage, NativeLibraryImageError> {
    capture_native_library_image_with_limit(
        path,
        MAX_NATIVE_LIBRARY_IMAGE_BYTES,
        &mut check_cancellation,
    )
}

#[cfg(all(
    unix,
    any(
        test,
        feature = "mlu",
        feature = "npu",
        feature = "rocm",
        feature = "cuda",
        feature = "xpu"
    )
))]
fn capture_native_library_image_with_limit(
    path: &Path,
    maximum_bytes: u64,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<CapturedNativeLibraryImage, NativeLibraryImageError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    check_cancellation()?;
    if maximum_bytes == 0 {
        return Err(NativeLibraryImageError::Invalid(
            "native-library byte bound must be nonzero".to_owned(),
        ));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let before = file
        .metadata()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    if !before.is_file() || before.len() == 0 || before.len() > maximum_bytes {
        return Err(NativeLibraryImageError::Invalid(format!(
            "image must be a nonempty regular file no larger than {maximum_bytes} bytes"
        )));
    }
    let expected_length = usize::try_from(before.len()).map_err(|_| {
        NativeLibraryImageError::Invalid(
            "image length exceeds the process address space".to_owned(),
        )
    })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected_length.min(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES))
        .map_err(|error| {
            NativeLibraryImageError::Invalid(format!("image buffer allocation failed: {error}"))
        })?;
    let mut hasher = Sha256::new();
    let mut chunk = [0_u8; NATIVE_LIBRARY_IMAGE_CHUNK_BYTES];
    loop {
        check_cancellation()?;
        let remaining = maximum_bytes
            .checked_add(1)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(usize::MAX)
            .saturating_sub(bytes.len());
        if remaining == 0 {
            break;
        }
        let chunk_limit = remaining.min(chunk.len());
        let read = file
            .read(&mut chunk[..chunk_limit])
            .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
        if read == 0 {
            break;
        }
        bytes.try_reserve_exact(read).map_err(|error| {
            NativeLibraryImageError::Invalid(format!("image buffer allocation failed: {error}"))
        })?;
        bytes.extend_from_slice(&chunk[..read]);
        hasher.update(&chunk[..read]);
    }
    check_cancellation()?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != before.len()
        || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes
    {
        return Err(NativeLibraryImageError::Invalid(
            "image length changed while it was captured".to_owned(),
        ));
    }
    let after = file
        .metadata()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    if before.dev() != after.dev()
        || before.ino() != after.ino()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return Err(NativeLibraryImageError::Invalid(
            "image metadata changed while it was captured".to_owned(),
        ));
    }
    check_cancellation()?;
    Ok(CapturedNativeLibraryImage {
        bytes,
        digest_sha256: format!("{:x}", hasher.finalize()),
    })
}

#[cfg(all(target_os = "windows", any(test, feature = "cuda", feature = "xpu")))]
fn capture_native_library_image_with_limit(
    path: &Path,
    maximum_bytes: u64,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<CapturedNativeLibraryImage, NativeLibraryImageError> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;

    check_cancellation()?;
    if maximum_bytes == 0 {
        return Err(NativeLibraryImageError::Invalid(
            "native-library byte bound must be nonzero".to_owned(),
        ));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_SEQUENTIAL_SCAN)
        .open(path)
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let before = file
        .metadata()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    if !before.is_file()
        || before.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || before.len() == 0
        || before.len() > maximum_bytes
    {
        return Err(NativeLibraryImageError::Invalid(format!(
            "image must be a nonempty non-reparse regular file no larger than {maximum_bytes} bytes"
        )));
    }
    let first = read_bounded_native_library_image(
        &mut file,
        before.len(),
        maximum_bytes,
        check_cancellation,
    )?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let second = read_bounded_native_library_image(
        &mut file,
        before.len(),
        maximum_bytes,
        check_cancellation,
    )?;
    if first != second {
        return Err(NativeLibraryImageError::Invalid(
            "image changed while it was captured".to_owned(),
        ));
    }
    let after = file
        .metadata()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    if before.volume_serial_number() != after.volume_serial_number()
        || before.file_index() != after.file_index()
        || before.len() != after.len()
        || before.last_write_time() != after.last_write_time()
        || before.creation_time() != after.creation_time()
    {
        return Err(NativeLibraryImageError::Invalid(
            "image metadata changed while it was captured".to_owned(),
        ));
    }
    check_cancellation()?;
    Ok(CapturedNativeLibraryImage {
        digest_sha256: format!("{:x}", Sha256::digest(&first)),
        bytes: first,
    })
}

#[cfg(all(target_os = "windows", any(test, feature = "cuda", feature = "xpu")))]
fn read_bounded_native_library_image(
    file: &mut File,
    expected_bytes: u64,
    maximum_bytes: u64,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<Vec<u8>, NativeLibraryImageError> {
    let capacity = usize::try_from(expected_bytes).map_err(|_| {
        NativeLibraryImageError::Invalid("image length exceeds addressable memory".to_owned())
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|error| {
        NativeLibraryImageError::Invalid(format!("image buffer allocation failed: {error}"))
    })?;
    let mut chunk = [0_u8; NATIVE_LIBRARY_IMAGE_CHUNK_BYTES];
    loop {
        check_cancellation()?;
        let count = file
            .read(&mut chunk)
            .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
        if count == 0 {
            break;
        }
        bytes.try_reserve_exact(count).map_err(|error| {
            NativeLibraryImageError::Invalid(format!("image buffer allocation failed: {error}"))
        })?;
        bytes.extend_from_slice(&chunk[..count]);
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
            return Err(NativeLibraryImageError::Invalid(
                "image exceeds the native-library byte bound".to_owned(),
            ));
        }
    }
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != expected_bytes {
        return Err(NativeLibraryImageError::Invalid(
            "image length changed while it was captured".to_owned(),
        ));
    }
    Ok(bytes)
}

#[cfg(all(
    not(unix),
    not(target_os = "windows"),
    any(
        test,
        feature = "mlu",
        feature = "npu",
        feature = "rocm",
        feature = "cuda",
        feature = "xpu"
    )
))]
fn capture_native_library_image_with_limit(
    _path: &Path,
    _maximum_bytes: u64,
    _check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<CapturedNativeLibraryImage, NativeLibraryImageError> {
    Err(NativeLibraryImageError::UnsupportedPlatform)
}

#[cfg(all(
    target_os = "linux",
    any(
        test,
        feature = "mlu",
        feature = "npu",
        feature = "rocm",
        feature = "cuda",
        feature = "xpu"
    )
))]
fn seal_native_library_image(
    bytes: &[u8],
    snapshot_name: &str,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<RetainedNativeLibraryImage, NativeLibraryImageError> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};

    check_cancellation()?;
    let name = CString::new(format!("sim-native-{snapshot_name}")).map_err(|error| {
        NativeLibraryImageError::Invalid(format!("snapshot name is invalid: {error}"))
    })?;
    let descriptor =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if descriptor < 0 {
        return Err(NativeLibraryImageError::Invalid(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    for chunk in bytes.chunks(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES) {
        check_cancellation()?;
        file.write_all(chunk)
            .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    }
    check_cancellation()?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let required_seals =
        libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, required_seals) } != 0 {
        return Err(NativeLibraryImageError::Invalid(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    let actual_seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    if actual_seals < 0 || actual_seals & required_seals != required_seals {
        return Err(NativeLibraryImageError::Invalid(
            "snapshot does not carry every required immutable seal".to_owned(),
        ));
    }
    let loader_path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    Ok(RetainedNativeLibraryImage {
        _file: file,
        loader_path,
    })
}

#[cfg(all(target_os = "windows", any(test, feature = "cuda", feature = "xpu")))]
fn seal_native_library_image(
    bytes: &[u8],
    snapshot_name: &str,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<RetainedNativeLibraryImage, NativeLibraryImageError> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_ATTRIBUTE_TEMPORARY: u32 = 0x0000_0100;
    const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;

    if snapshot_name.is_empty()
        || snapshot_name.len() > 128
        || !snapshot_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(NativeLibraryImageError::Invalid(
            "snapshot name is invalid".to_owned(),
        ));
    }
    check_cancellation()?;
    let temporary_directory = tempfile::Builder::new()
        .prefix("sim-native-")
        .tempdir()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let loader_path = temporary_directory
        .path()
        .join(format!("{snapshot_name}.dll"));
    let mut writer = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_SEQUENTIAL_SCAN)
        .open(&loader_path)
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    for chunk in bytes.chunks(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES) {
        check_cancellation()?;
        writer
            .write_all(chunk)
            .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    }
    writer
        .sync_all()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    drop(writer);
    let mut permissions = std::fs::metadata(&loader_path)
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&loader_path, permissions)
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
        .open(&loader_path)
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    check_cancellation()?;
    Ok(RetainedNativeLibraryImage {
        _file: file,
        loader_path,
        _temporary_directory: temporary_directory,
    })
}

#[cfg(all(test, not(any(target_os = "linux", target_os = "windows"))))]
fn seal_native_library_image(
    bytes: &[u8],
    _snapshot_name: &str,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<RetainedNativeLibraryImage, NativeLibraryImageError> {
    let mut named_file = tempfile::NamedTempFile::new()
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    for chunk in bytes.chunks(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES) {
        check_cancellation()?;
        named_file
            .as_file_mut()
            .write_all(chunk)
            .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    }
    check_cancellation()?;
    named_file
        .as_file_mut()
        .seek(SeekFrom::Start(0))
        .map_err(|error| NativeLibraryImageError::Invalid(error.to_string()))?;
    let (file, temporary_path) = named_file.into_parts();
    let loader_path = temporary_path.to_path_buf();
    Ok(RetainedNativeLibraryImage {
        _file: file,
        loader_path,
        _temporary_path: temporary_path,
    })
}

#[cfg(all(
    not(target_os = "linux"),
    not(target_os = "windows"),
    not(test),
    any(
        feature = "mlu",
        feature = "npu",
        feature = "rocm",
        feature = "cuda",
        feature = "xpu"
    )
))]
fn seal_native_library_image(
    _bytes: &[u8],
    _snapshot_name: &str,
    _check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<RetainedNativeLibraryImage, NativeLibraryImageError> {
    Err(NativeLibraryImageError::UnsupportedPlatform)
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
impl NativePackagePayloadLimit {
    pub(crate) const fn new(path: &'static str, maximum_bytes: usize) -> Self {
        Self {
            path,
            maximum_bytes,
        }
    }

    #[cfg(all(
        test,
        any(
            feature = "directml",
            feature = "metal",
            feature = "mlu",
            feature = "npu",
            feature = "cuda",
            feature = "xpu"
        )
    ))]
    pub(crate) const fn path(&self) -> &'static str {
        self.path
    }
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub(crate) enum NativePackageAdmissionError {
    #[error("native package admission was cancelled")]
    Cancelled,
    #[error("native package tree is unsafe or incomplete: {0}")]
    UnsafePackage(String),
    #[error("native package signature coverage is invalid: {0}")]
    InvalidCoverage(String),
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
pub(crate) fn capture_native_package(
    root: &ArtifactRoot,
    limits: &[NativePackagePayloadLimit],
    maximum_entries: usize,
    maximum_total_bytes: usize,
    cancellation: &CancellationToken,
) -> Result<BTreeMap<String, Vec<u8>>, NativePackageAdmissionError> {
    capture_native_package_with_hook(
        root,
        limits,
        maximum_entries,
        maximum_total_bytes,
        cancellation,
        || Ok(()),
    )
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
fn capture_native_package_with_hook(
    root: &ArtifactRoot,
    limits: &[NativePackagePayloadLimit],
    maximum_entries: usize,
    maximum_total_bytes: usize,
    cancellation: &CancellationToken,
    after_capture: impl FnOnce() -> Result<(), NativePackageAdmissionError>,
) -> Result<BTreeMap<String, Vec<u8>>, NativePackageAdmissionError> {
    check_package_admission_cancellation(cancellation)?;
    if limits.is_empty() || maximum_entries == 0 || maximum_total_bytes == 0 {
        return Err(NativePackageAdmissionError::UnsafePackage(
            "package admission bounds and payload set must be nonzero".to_owned(),
        ));
    }
    let expected_paths = limits
        .iter()
        .map(|limit| limit.path.to_owned())
        .collect::<Vec<_>>();
    if limits
        .iter()
        .any(|limit| limit.maximum_bytes == 0 || !valid_native_package_path(limit.path))
        || expected_paths
            .windows(2)
            .any(|pair| matches!(pair, [left, right] if left >= right))
    {
        return Err(NativePackageAdmissionError::UnsafePackage(
            "package payload limits are not a strict sorted canonical set".to_owned(),
        ));
    }

    let observed_paths = list_native_package_paths(root, maximum_entries, cancellation)?;
    if observed_paths != expected_paths {
        return Err(NativePackageAdmissionError::UnsafePackage(
            "payload membership differs from the exact signed package contract".to_owned(),
        ));
    }

    let mut payloads = BTreeMap::new();
    let mut total_bytes = 0_usize;
    for limit in limits {
        check_package_admission_cancellation(cancellation)?;
        let bytes = root
            .read_private_file(limit.path, limit.maximum_bytes)
            .map_err(|error| NativePackageAdmissionError::UnsafePackage(error.to_string()))?
            .ok_or_else(|| {
                NativePackageAdmissionError::UnsafePackage(format!(
                    "required payload is missing: {}",
                    limit.path
                ))
            })?;
        check_package_admission_cancellation(cancellation)?;
        if bytes.is_empty() {
            return Err(NativePackageAdmissionError::UnsafePackage(format!(
                "required payload is empty: {}",
                limit.path
            )));
        }
        total_bytes = total_bytes.checked_add(bytes.len()).ok_or_else(|| {
            NativePackageAdmissionError::UnsafePackage(
                "package byte accounting overflowed".to_owned(),
            )
        })?;
        if total_bytes > maximum_total_bytes {
            return Err(NativePackageAdmissionError::UnsafePackage(format!(
                "package exceeds the {maximum_total_bytes}-byte aggregate bound"
            )));
        }
        payloads.insert(limit.path.to_owned(), bytes);
    }

    after_capture()?;
    check_package_admission_cancellation(cancellation)?;
    if list_native_package_paths(root, maximum_entries, cancellation)? != observed_paths {
        return Err(NativePackageAdmissionError::UnsafePackage(
            "package tree changed while it was captured".to_owned(),
        ));
    }
    for limit in limits {
        check_package_admission_cancellation(cancellation)?;
        let current = root
            .read_private_file(limit.path, limit.maximum_bytes)
            .map_err(|error| NativePackageAdmissionError::UnsafePackage(error.to_string()))?
            .ok_or_else(|| {
                NativePackageAdmissionError::UnsafePackage(format!(
                    "required payload disappeared after capture: {}",
                    limit.path
                ))
            })?;
        if payloads.get(limit.path) != Some(&current) {
            return Err(NativePackageAdmissionError::UnsafePackage(format!(
                "payload changed after capture: {}",
                limit.path
            )));
        }
    }
    check_package_admission_cancellation(cancellation)?;
    Ok(payloads)
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
pub(crate) fn validate_native_package_coverage(
    coverage: &[u8],
    payloads: &BTreeMap<String, Vec<u8>>,
    excludes: &[&str],
    maximum_bytes: usize,
) -> Result<(), NativePackageAdmissionError> {
    if maximum_bytes == 0
        || coverage.is_empty()
        || coverage.len() > maximum_bytes
        || coverage.last() != Some(&b'\n')
        || !coverage.is_ascii()
    {
        return Err(NativePackageAdmissionError::InvalidCoverage(
            "coverage must be bounded nonempty newline-terminated ASCII".to_owned(),
        ));
    }
    if excludes
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left >= right))
        || excludes
            .iter()
            .any(|path| !valid_native_package_path(path) || !payloads.contains_key(*path))
    {
        return Err(NativePackageAdmissionError::InvalidCoverage(
            "coverage exclusions are not a strict sorted payload subset".to_owned(),
        ));
    }
    let expected_paths = payloads
        .keys()
        .filter(|path| excludes.binary_search(&path.as_str()).is_err())
        .cloned()
        .collect::<Vec<_>>();
    let text = std::str::from_utf8(coverage).map_err(|_| {
        NativePackageAdmissionError::InvalidCoverage("coverage must be UTF-8".to_owned())
    })?;
    let mut observed_paths = Vec::new();
    for line in text.lines() {
        let (digest_and_size, path) = line.split_once("  ").ok_or_else(|| {
            NativePackageAdmissionError::InvalidCoverage(
                "coverage row must contain digest, size, and path".to_owned(),
            )
        })?;
        let (digest, size) = digest_and_size.split_once(' ').ok_or_else(|| {
            NativePackageAdmissionError::InvalidCoverage(
                "coverage row has an invalid digest/size separator".to_owned(),
            )
        })?;
        if !valid_lower_hex_sha256(digest)
            || size.is_empty()
            || size.starts_with('0') && size != "0"
            || !size.bytes().all(|byte| byte.is_ascii_digit())
            || !valid_native_package_path(path)
        {
            return Err(NativePackageAdmissionError::InvalidCoverage(
                "coverage row is not canonical".to_owned(),
            ));
        }
        let payload = payloads.get(path).ok_or_else(|| {
            NativePackageAdmissionError::InvalidCoverage(
                "coverage references an unknown payload".to_owned(),
            )
        })?;
        if size != payload.len().to_string() || digest != format!("{:x}", Sha256::digest(payload)) {
            return Err(NativePackageAdmissionError::InvalidCoverage(format!(
                "coverage does not match payload {path}"
            )));
        }
        observed_paths.push(path.to_owned());
    }
    if observed_paths != expected_paths {
        return Err(NativePackageAdmissionError::InvalidCoverage(
            "coverage paths must be exact, strictly sorted, and unique".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
fn list_native_package_paths(
    root: &ArtifactRoot,
    maximum_entries: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<String>, NativePackageAdmissionError> {
    root.list_contained_regular_files_recursive(maximum_entries, cancellation)
        .map_err(|error| {
            if matches!(error, comfy_model::ArtifactIndexError::Cancelled) {
                NativePackageAdmissionError::Cancelled
            } else {
                NativePackageAdmissionError::UnsafePackage(error.to_string())
            }
        })?
        .into_iter()
        .map(|path| {
            path.components()
                .map(|component| {
                    component.as_os_str().to_str().ok_or_else(|| {
                        NativePackageAdmissionError::UnsafePackage(
                            "package path is not UTF-8".to_owned(),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|components| components.join("/"))
        })
        .collect()
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
fn check_package_admission_cancellation(
    cancellation: &CancellationToken,
) -> Result<(), NativePackageAdmissionError> {
    cancellation
        .check()
        .map_err(|_| NativePackageAdmissionError::Cancelled)
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
fn valid_native_package_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains('\\')
        && path.split('/').all(|component| {
            !component.is_empty()
                && !matches!(component, "." | "..")
                && !component.chars().any(char::is_control)
        })
}

#[cfg(any(
    test,
    feature = "directml",
    feature = "metal",
    feature = "mlu",
    feature = "npu",
    feature = "cuda",
    feature = "xpu"
))]
fn valid_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginTrustPolicy {
    keys: BTreeMap<String, PluginVerificationKey>,
}

impl PluginTrustPolicy {
    pub fn new(keys: impl IntoIterator<Item = PluginVerificationKey>) -> Result<Self, TrustError> {
        let mut checked_keys = BTreeMap::new();
        for key in keys {
            let key_id = key.key_id.clone();
            if checked_keys.insert(key_id.clone(), key).is_some() {
                return Err(TrustError::DuplicateVerificationKey(key_id));
            }
        }
        Ok(Self { keys: checked_keys })
    }

    pub fn authorize_manifest(
        &self,
        manifest: &PluginManifest,
        permissions: &PermissionPolicy,
    ) -> Result<PluginAuthorization, TrustError> {
        validate_plugin_manifest(manifest)?;
        let verification_key = self
            .keys
            .get(&manifest.signature.key_id)
            .ok_or_else(|| TrustError::UnknownVerificationKey(manifest.signature.key_id.clone()))?;
        let signature = decode_hex_exact::<ED25519_SIGNATURE_BYTES>(&manifest.signature.value)
            .ok_or(TrustError::InvalidPluginSignature)?;
        UnparsedPublicKey::new(&ED25519, verification_key.public_key)
            .verify(&manifest.signing_payload(), &signature)
            .map_err(|_| TrustError::InvalidPluginSignature)?;
        let requested_capabilities = manifest
            .capabilities
            .iter()
            .map(crate::Capability::from_plugin_request)
            .collect::<Result<Vec<_>, _>>()?;
        let capabilities = permissions.authorize(
            &manifest.identifier,
            &CapabilitySet::new(requested_capabilities),
        )?;
        Ok(PluginAuthorization {
            plugin_id: manifest.identifier.clone(),
            digest_sha256: manifest.digest_sha256.clone(),
            signing_payload_sha256: Sha256::digest(manifest.signing_payload()).into(),
            policy_generation: permissions.generation(),
            level: TrustLevel::Restricted,
            capabilities,
        })
    }
}

fn validate_plugin_manifest(manifest: &PluginManifest) -> Result<(), TrustError> {
    let registry = TypeRegistry::built_in().map_err(PluginContractError::from)?;
    manifest.validate(&registry)?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginAuthorization {
    plugin_id: String,
    digest_sha256: String,
    signing_payload_sha256: [u8; 32],
    policy_generation: PermissionPolicyGeneration,
    level: TrustLevel,
    capabilities: AuthorizedCapabilities,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SealedPluginAuthorization {
    payload: SealedPluginAuthorizationPayload,
    signature_algorithm: String,
    signature: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SealedPluginAuthorizationPayload {
    version: u16,
    plugin_id: String,
    digest_sha256: String,
    signing_payload_sha256: [u8; 32],
    policy_generation: PermissionPolicyGeneration,
    level: TrustLevel,
    capabilities: AuthorizedCapabilities,
}

pub struct PluginAuthorizationSealer {
    seed: Zeroizing<[u8; 32]>,
    policy_generation: PermissionPolicyGeneration,
}

impl PluginAuthorizationSealer {
    pub fn generate(policy_generation: PermissionPolicyGeneration) -> Result<Self, TrustError> {
        let mut seed = [0_u8; 32];
        SystemRandom::new()
            .fill(&mut seed)
            .map_err(|_| TrustError::AuthorizationSealingUnavailable)?;
        Self::from_seed(seed, policy_generation)
    }

    pub fn from_seed(
        seed: [u8; 32],
        policy_generation: PermissionPolicyGeneration,
    ) -> Result<Self, TrustError> {
        Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|_| TrustError::AuthorizationSealingUnavailable)?;
        Ok(Self {
            seed: Zeroizing::new(seed),
            policy_generation,
        })
    }

    pub fn verifier(&self) -> Result<PluginAuthorizationVerifier, TrustError> {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(self.seed.as_ref())
            .map_err(|_| TrustError::AuthorizationSealingUnavailable)?;
        let public_key = key_pair
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TrustError::AuthorizationSealingUnavailable)?;
        Ok(PluginAuthorizationVerifier {
            public_key,
            policy_generation: self.policy_generation,
        })
    }

    fn sign(&self, payload: &[u8]) -> Result<[u8; ED25519_SIGNATURE_BYTES], TrustError> {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(self.seed.as_ref())
            .map_err(|_| TrustError::AuthorizationSealingUnavailable)?;
        key_pair
            .sign(payload)
            .as_ref()
            .try_into()
            .map_err(|_| TrustError::AuthorizationSealingUnavailable)
    }
}

impl fmt::Debug for PluginAuthorizationSealer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PluginAuthorizationSealer([REDACTED])")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginAuthorizationVerifier {
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
    policy_generation: PermissionPolicyGeneration,
}

impl PluginAuthorizationVerifier {
    pub fn new(
        public_key: impl AsRef<[u8]>,
        policy_generation: PermissionPolicyGeneration,
    ) -> Result<Self, TrustError> {
        let public_key = public_key
            .as_ref()
            .try_into()
            .map_err(|_| TrustError::InvalidAuthorizationVerificationKey)?;
        Ok(Self {
            public_key,
            policy_generation,
        })
    }

    pub fn public_key_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.public_key
    }

    pub fn from_hex(
        value: &str,
        policy_generation: PermissionPolicyGeneration,
    ) -> Result<Self, TrustError> {
        let public_key = decode_hex_exact::<ED25519_PUBLIC_KEY_BYTES>(value)
            .ok_or(TrustError::InvalidAuthorizationVerificationKey)?;
        Self::new(public_key, policy_generation)
    }

    pub fn to_hex(&self) -> String {
        encode_hex(&self.public_key)
    }

    pub const fn policy_generation(&self) -> PermissionPolicyGeneration {
        self.policy_generation
    }

    pub fn to_token(&self) -> String {
        format!("{}:{}", self.policy_generation.get(), self.to_hex())
    }

    pub fn from_token(value: &str) -> Result<Self, TrustError> {
        let (generation, public_key) = value
            .split_once(':')
            .ok_or(TrustError::InvalidAuthorizationVerificationKey)?;
        if generation.is_empty() || generation.starts_with('0') {
            return Err(TrustError::InvalidAuthorizationVerificationKey);
        }
        let generation = generation
            .parse::<u64>()
            .map_err(|_| TrustError::InvalidAuthorizationVerificationKey)?;
        let generation = PermissionPolicyGeneration::new(generation)
            .map_err(|_| TrustError::InvalidAuthorizationVerificationKey)?;
        Self::from_hex(public_key, generation)
    }
}

impl PluginAuthorization {
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub fn level(&self) -> TrustLevel {
        self.level
    }

    pub fn capabilities(&self) -> &AuthorizedCapabilities {
        &self.capabilities
    }

    pub const fn policy_generation(&self) -> PermissionPolicyGeneration {
        self.policy_generation
    }

    pub fn sealed_bytes(&self, sealer: &PluginAuthorizationSealer) -> Result<Vec<u8>, TrustError> {
        if self.policy_generation != sealer.policy_generation
            || self.capabilities.policy_generation() != self.policy_generation
        {
            return Err(TrustError::InvalidSealedPluginAuthorization);
        }
        let payload = SealedPluginAuthorizationPayload {
            version: SEALED_PLUGIN_AUTHORIZATION_VERSION,
            plugin_id: self.plugin_id.clone(),
            digest_sha256: self.digest_sha256.clone(),
            signing_payload_sha256: self.signing_payload_sha256,
            policy_generation: self.policy_generation,
            level: self.level,
            capabilities: self.capabilities.clone(),
        };
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|_| TrustError::InvalidSealedPluginAuthorization)?;
        let sealed = SealedPluginAuthorization {
            payload,
            signature_algorithm: PLUGIN_SIGNATURE_ALGORITHM.to_owned(),
            signature: encode_hex(&sealer.sign(&authorization_signing_payload(&payload_bytes))?),
        };
        let bytes = serde_json::to_vec(&sealed)
            .map_err(|_| TrustError::InvalidSealedPluginAuthorization)?;
        if bytes.len() > MAX_SEALED_PLUGIN_AUTHORIZATION_BYTES {
            return Err(TrustError::SealedPluginAuthorizationTooLarge);
        }
        Ok(bytes)
    }

    pub fn from_sealed_bytes(
        bytes: &[u8],
        manifest: &PluginManifest,
        verifier: &PluginAuthorizationVerifier,
        expected_policy_generation: PermissionPolicyGeneration,
        expected_profile_id: &str,
    ) -> Result<Self, TrustError> {
        if bytes.is_empty() || bytes.len() > MAX_SEALED_PLUGIN_AUTHORIZATION_BYTES {
            return Err(TrustError::SealedPluginAuthorizationTooLarge);
        }
        let sealed: SealedPluginAuthorization = serde_json::from_slice(bytes)
            .map_err(|_| TrustError::InvalidSealedPluginAuthorization)?;
        if sealed.signature_algorithm != PLUGIN_SIGNATURE_ALGORITHM {
            return Err(TrustError::InvalidSealedPluginAuthorization);
        }
        let signature = decode_hex_exact::<ED25519_SIGNATURE_BYTES>(&sealed.signature)
            .ok_or(TrustError::InvalidSealedPluginAuthorization)?;
        let payload_bytes = serde_json::to_vec(&sealed.payload)
            .map_err(|_| TrustError::InvalidSealedPluginAuthorization)?;
        UnparsedPublicKey::new(&ED25519, verifier.public_key)
            .verify(&authorization_signing_payload(&payload_bytes), &signature)
            .map_err(|_| TrustError::InvalidSealedPluginAuthorization)?;
        let sealed = sealed.payload;
        if sealed.version != SEALED_PLUGIN_AUTHORIZATION_VERSION
            || sealed.level != TrustLevel::Restricted
            || sealed.capabilities.subject_id() != sealed.plugin_id
            || sealed.capabilities.profile_id() != expected_profile_id
            || sealed.policy_generation != expected_policy_generation
            || sealed.policy_generation != verifier.policy_generation
            || sealed.capabilities.policy_generation() != sealed.policy_generation
        {
            return Err(TrustError::InvalidSealedPluginAuthorization);
        }
        sealed.capabilities.validate_sealed()?;
        let authorization = Self {
            plugin_id: sealed.plugin_id,
            digest_sha256: sealed.digest_sha256,
            signing_payload_sha256: sealed.signing_payload_sha256,
            policy_generation: sealed.policy_generation,
            level: sealed.level,
            capabilities: sealed.capabilities,
        };
        authorization.require_manifest(manifest)?;
        Ok(authorization)
    }

    pub fn require_manifest(&self, manifest: &PluginManifest) -> Result<(), TrustError> {
        validate_plugin_manifest(manifest)?;
        let signing_payload_sha256: [u8; 32] = Sha256::digest(manifest.signing_payload()).into();
        if manifest.identifier != self.plugin_id
            || manifest.digest_sha256 != self.digest_sha256
            || !constant_time_equal(&signing_payload_sha256, &self.signing_payload_sha256)
        {
            return Err(TrustError::AuthorizationManifestMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SecretId(String);

impl SecretId {
    pub fn new(value: impl Into<String>) -> Result<Self, TrustError> {
        let value = value.into();
        if !valid_ascii_identifier(&value, 1_024) {
            return Err(TrustError::InvalidSecretId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new(value: impl Into<String>) -> Result<Self, TrustError> {
        let value = value.into();
        if !valid_ascii_identifier(&value, 256) {
            return Err(TrustError::InvalidProviderId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SecretId {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

pub struct SecretValue(Zeroizing<Vec<u8>>);

impl SecretValue {
    pub fn new(value: Vec<u8>) -> Self {
        Self(Zeroizing::new(value))
    }

    pub fn expose_to<R>(&self, operation: impl FnOnce(&[u8]) -> R) -> R {
        operation(self.0.as_slice())
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue([REDACTED])")
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProviderEndpoint {
    provider: ProviderId,
    endpoint: String,
}

impl ProviderEndpoint {
    pub fn new(
        provider: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Result<Self, TrustError> {
        let provider = ProviderId::new(provider)?;
        let endpoint = endpoint.into();
        if endpoint.len() > 2_048
            || endpoint != endpoint.trim()
            || endpoint.chars().any(char::is_control)
        {
            return Err(TrustError::InvalidProviderEndpoint);
        }
        let parsed = Url::parse(&endpoint).map_err(|_| TrustError::InvalidProviderEndpoint)?;
        let host = parsed.host().ok_or(TrustError::InvalidProviderEndpoint)?;
        let loopback_http = parsed.scheme() == "http"
            && match host {
                Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
                Host::Ipv4(address) => address.is_loopback(),
                Host::Ipv6(address) => address.is_loopback(),
            };
        if (parsed.scheme() != "https" && !loopback_http)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.to_string() != endpoint
        {
            return Err(TrustError::InvalidProviderEndpoint);
        }
        Ok(Self { provider, endpoint })
    }

    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CredentialScope {
    profile_id: String,
    subject_id: String,
    provider: ProviderId,
    secret_id: SecretId,
}

impl CredentialScope {
    pub fn new(
        profile_id: impl Into<String>,
        subject_id: impl Into<String>,
        provider: impl Into<String>,
        secret_id: SecretId,
    ) -> Result<Self, TrustError> {
        let profile_id = profile_id.into();
        let subject_id = subject_id.into();
        if !valid_ascii_identifier(&profile_id, 1_024)
            || !valid_ascii_identifier(&subject_id, 1_024)
        {
            return Err(TrustError::InvalidCredentialScope);
        }
        Ok(Self {
            profile_id,
            subject_id,
            provider: ProviderId::new(provider)?,
            secret_id,
        })
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    pub fn secret_id(&self) -> &SecretId {
        &self.secret_id
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProviderMode {
    #[default]
    Disabled,
    Offline,
    Enabled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPolicy {
    profile_id: Option<String>,
    mode: ProviderMode,
    allowed_endpoints: BTreeSet<ProviderEndpoint>,
    credential_scopes: BTreeSet<CredentialScope>,
}

impl Default for ProviderPolicy {
    fn default() -> Self {
        Self {
            profile_id: None,
            mode: ProviderMode::Disabled,
            allowed_endpoints: BTreeSet::new(),
            credential_scopes: BTreeSet::new(),
        }
    }
}

impl ProviderPolicy {
    pub fn new(
        profile_id: impl Into<String>,
        mode: ProviderMode,
        allowed_endpoints: impl IntoIterator<Item = ProviderEndpoint>,
        credential_scopes: impl IntoIterator<Item = CredentialScope>,
    ) -> Result<Self, TrustError> {
        let profile_id = profile_id.into();
        if !valid_ascii_identifier(&profile_id, 1_024) {
            return Err(TrustError::InvalidProviderProfile);
        }
        let mut allowed_endpoint_set = BTreeSet::new();
        for endpoint in allowed_endpoints {
            if !allowed_endpoint_set.insert(endpoint) {
                return Err(TrustError::InvalidProviderEndpoint);
            }
        }
        let mut credential_scope_set = BTreeSet::new();
        for scope in credential_scopes {
            if !credential_scope_set.insert(scope) {
                return Err(TrustError::InvalidCredentialScope);
            }
        }
        if credential_scope_set
            .iter()
            .any(|scope| scope.profile_id() != profile_id)
            || credential_scope_set.iter().any(|scope| {
                !allowed_endpoint_set
                    .iter()
                    .any(|endpoint| endpoint.provider() == scope.provider())
            })
        {
            return Err(TrustError::InvalidCredentialScope);
        }
        Ok(Self {
            profile_id: Some(profile_id),
            mode,
            allowed_endpoints: allowed_endpoint_set,
            credential_scopes: credential_scope_set,
        })
    }

    pub fn authorize(
        &self,
        profile_id: &str,
        subject_id: &str,
        provider: &str,
        endpoint: &str,
        secret_id: Option<&SecretId>,
    ) -> Result<AuthorizedProviderRequest, TrustError> {
        match self.mode {
            ProviderMode::Disabled => return Err(TrustError::ProviderDisabled),
            ProviderMode::Offline => return Err(TrustError::Offline),
            ProviderMode::Enabled => {}
        }
        if self.profile_id.as_deref() != Some(profile_id) {
            return Err(TrustError::ProviderProfileMismatch);
        }
        let requested_endpoint = ProviderEndpoint::new(provider, endpoint)?;
        if !self.allowed_endpoints.contains(&requested_endpoint) {
            return Err(TrustError::ProviderEndpointDenied);
        }
        let provider_id = ProviderId::new(provider)?;
        if secret_id.is_some_and(|secret_id| {
            CredentialScope::new(
                profile_id,
                subject_id,
                provider_id.as_str(),
                secret_id.clone(),
            )
            .map_or(true, |scope| !self.credential_scopes.contains(&scope))
        }) {
            return Err(TrustError::MissingSecretGrant);
        }
        Ok(AuthorizedProviderRequest {
            profile_id: profile_id.to_owned(),
            subject_id: subject_id.to_owned(),
            endpoint: requested_endpoint,
            secret_id: secret_id.cloned(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedProviderRequest {
    profile_id: String,
    subject_id: String,
    endpoint: ProviderEndpoint,
    secret_id: Option<SecretId>,
}

impl AuthorizedProviderRequest {
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn provider(&self) -> &str {
        self.endpoint.provider.as_str()
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint.endpoint
    }

    pub fn secret_id(&self) -> Option<&SecretId> {
        self.secret_id.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPriceBound {
    currency_code: String,
    maximum_microunits: u64,
}

impl ProviderPriceBound {
    pub fn new(
        currency_code: impl Into<String>,
        maximum_microunits: u64,
    ) -> Result<Self, TrustError> {
        let currency_code = currency_code.into();
        if currency_code.len() != 3
            || !currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
            || maximum_microunits == 0
        {
            return Err(TrustError::InvalidProviderPriceBound);
        }
        Ok(Self {
            currency_code,
            maximum_microunits,
        })
    }

    pub fn currency_code(&self) -> &str {
        &self.currency_code
    }

    pub fn maximum_microunits(&self) -> u64 {
        self.maximum_microunits
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProviderCostNonce([u8; 32]);

impl ProviderCostNonce {
    pub fn new(bytes: [u8; 32]) -> Result<Self, TrustError> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(TrustError::InvalidProviderCostAcceptance);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCostAcceptanceScope {
    principal_id: String,
    profile_id: String,
    prompt_id: String,
    prompt_sha256: String,
    attempt_id: String,
    node_id: String,
    request_ordinal: u32,
    request_sha256: String,
    plugin_id: String,
    plugin_digest_sha256: String,
    provider_binding_sha256: String,
    endpoint: ProviderEndpoint,
    price_bound: ProviderPriceBound,
}

impl ProviderCostAcceptanceScope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        principal_id: impl Into<String>,
        profile_id: impl Into<String>,
        prompt_id: impl Into<String>,
        prompt_sha256: impl Into<String>,
        attempt_id: impl Into<String>,
        node_id: impl Into<String>,
        request_ordinal: u32,
        request_sha256: impl Into<String>,
        plugin_id: impl Into<String>,
        plugin_digest_sha256: impl Into<String>,
        provider_binding_sha256: impl Into<String>,
        provider: impl Into<String>,
        endpoint: impl Into<String>,
        price_bound: ProviderPriceBound,
    ) -> Result<Self, TrustError> {
        let principal_id = principal_id.into();
        let profile_id = profile_id.into();
        let prompt_id = prompt_id.into();
        let prompt_sha256 = prompt_sha256.into();
        let attempt_id = attempt_id.into();
        let node_id = node_id.into();
        let request_sha256 = request_sha256.into();
        let plugin_id = plugin_id.into();
        let plugin_digest_sha256 = plugin_digest_sha256.into();
        let provider_binding_sha256 = provider_binding_sha256.into();
        if principal_id.is_empty()
            || principal_id.len() > 1_024
            || principal_id != principal_id.trim()
            || principal_id.chars().any(char::is_control)
            || !valid_ascii_identifier(&profile_id, 1_024)
            || !valid_ascii_identifier(&prompt_id, 1_024)
            || validate_sha256(&prompt_sha256).is_err()
            || !valid_ascii_identifier(&attempt_id, 1_024)
            || !valid_ascii_identifier(&node_id, 1_024)
            || validate_sha256(&request_sha256).is_err()
            || !valid_ascii_identifier(&plugin_id, 1_024)
            || validate_sha256(&plugin_digest_sha256).is_err()
            || validate_sha256(&provider_binding_sha256).is_err()
        {
            return Err(TrustError::InvalidProviderCostAcceptance);
        }
        Ok(Self {
            principal_id,
            profile_id,
            prompt_id,
            prompt_sha256,
            attempt_id,
            node_id,
            request_ordinal,
            request_sha256,
            plugin_id,
            plugin_digest_sha256,
            provider_binding_sha256,
            endpoint: ProviderEndpoint::new(provider, endpoint)?,
            price_bound,
        })
    }

    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn prompt_id(&self) -> &str {
        &self.prompt_id
    }

    pub fn prompt_sha256(&self) -> &str {
        &self.prompt_sha256
    }

    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn request_ordinal(&self) -> u32 {
        self.request_ordinal
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn plugin_digest_sha256(&self) -> &str {
        &self.plugin_digest_sha256
    }

    pub fn provider_binding_sha256(&self) -> &str {
        &self.provider_binding_sha256
    }

    pub fn provider(&self) -> &str {
        self.endpoint.provider().as_str()
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.endpoint()
    }

    pub fn price_bound(&self) -> &ProviderPriceBound {
        &self.price_bound
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProviderCostAcceptanceClaims {
    scope: ProviderCostAcceptanceScope,
    issued_at_milliseconds: u64,
    expires_at_milliseconds: u64,
    nonce: ProviderCostNonce,
}

pub struct ProviderCostAcceptance {
    claims: ProviderCostAcceptanceClaims,
    signature: [u8; ED25519_SIGNATURE_BYTES],
}

impl ProviderCostAcceptance {
    pub fn nonce(&self) -> ProviderCostNonce {
        self.claims.nonce
    }
}

impl fmt::Debug for ProviderCostAcceptance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderCostAcceptance([SEALED])")
    }
}

pub struct ProviderCostAcceptanceIssuer {
    seed: Zeroizing<[u8; 32]>,
    clock_origin: Instant,
}

impl ProviderCostAcceptanceIssuer {
    pub fn generate(clock_origin: Instant) -> Result<Self, TrustError> {
        let mut seed = [0_u8; 32];
        SystemRandom::new()
            .fill(&mut seed)
            .map_err(|_| TrustError::ProviderCostAcceptanceSealingUnavailable)?;
        Self::from_seed(seed, clock_origin)
    }

    pub fn from_seed(seed: [u8; 32], clock_origin: Instant) -> Result<Self, TrustError> {
        Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|_| TrustError::ProviderCostAcceptanceSealingUnavailable)?;
        Ok(Self {
            seed: Zeroizing::new(seed),
            clock_origin,
        })
    }

    pub fn verifier(&self) -> Result<ProviderCostAcceptanceVerifier, TrustError> {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(self.seed.as_ref())
            .map_err(|_| TrustError::ProviderCostAcceptanceSealingUnavailable)?;
        let public_key = key_pair
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TrustError::ProviderCostAcceptanceSealingUnavailable)?;
        Ok(ProviderCostAcceptanceVerifier {
            public_key,
            clock_origin: self.clock_origin,
        })
    }

    pub fn issue(
        &self,
        scope: ProviderCostAcceptanceScope,
        issued_at: Instant,
        expires_at: Instant,
        nonce: ProviderCostNonce,
    ) -> Result<ProviderCostAcceptance, TrustError> {
        let issued_at_milliseconds = provider_cost_milliseconds(self.clock_origin, issued_at)?;
        let expires_at_milliseconds = provider_cost_milliseconds(self.clock_origin, expires_at)?;
        if expires_at <= issued_at
            || expires_at
                .checked_duration_since(issued_at)
                .is_none_or(|duration| duration > MAX_PROVIDER_COST_ACCEPTANCE_LIFETIME)
        {
            return Err(TrustError::InvalidProviderCostAcceptance);
        }
        let claims = ProviderCostAcceptanceClaims {
            scope,
            issued_at_milliseconds,
            expires_at_milliseconds,
            nonce,
        };
        let key_pair = Ed25519KeyPair::from_seed_unchecked(self.seed.as_ref())
            .map_err(|_| TrustError::ProviderCostAcceptanceSealingUnavailable)?;
        let signature = key_pair
            .sign(&provider_cost_acceptance_signing_payload(&claims))
            .as_ref()
            .try_into()
            .map_err(|_| TrustError::ProviderCostAcceptanceSealingUnavailable)?;
        Ok(ProviderCostAcceptance { claims, signature })
    }
}

impl fmt::Debug for ProviderCostAcceptanceIssuer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderCostAcceptanceIssuer([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedProviderCostAcceptance {
    nonce: ProviderCostNonce,
    expires_at: Instant,
}

impl VerifiedProviderCostAcceptance {
    pub fn nonce(self) -> ProviderCostNonce {
        self.nonce
    }

    pub fn expires_at(self) -> Instant {
        self.expires_at
    }
}

#[derive(Clone, Debug)]
pub struct ProviderCostAcceptanceVerifier {
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
    clock_origin: Instant,
}

impl ProviderCostAcceptanceVerifier {
    pub fn verify(
        &self,
        acceptance: &ProviderCostAcceptance,
        expected_scope: &ProviderCostAcceptanceScope,
        now: Instant,
    ) -> Result<VerifiedProviderCostAcceptance, TrustError> {
        UnparsedPublicKey::new(&ED25519, self.public_key)
            .verify(
                &provider_cost_acceptance_signing_payload(&acceptance.claims),
                &acceptance.signature,
            )
            .map_err(|_| TrustError::InvalidProviderCostAcceptance)?;
        if acceptance.claims.scope != *expected_scope {
            return Err(TrustError::InvalidProviderCostAcceptance);
        }
        let now_milliseconds = provider_cost_milliseconds(self.clock_origin, now)?;
        if now_milliseconds < acceptance.claims.issued_at_milliseconds
            || now_milliseconds >= acceptance.claims.expires_at_milliseconds
        {
            return Err(TrustError::ExpiredProviderCostAcceptance);
        }
        let expires_at = self
            .clock_origin
            .checked_add(Duration::from_millis(
                acceptance.claims.expires_at_milliseconds,
            ))
            .ok_or(TrustError::InvalidProviderCostAcceptance)?;
        Ok(VerifiedProviderCostAcceptance {
            nonce: acceptance.claims.nonce,
            expires_at,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteExposureApproval {
    LoopbackOnly,
    Approved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeApiExposure {
    bind_address: IpAddr,
    tls_enabled: bool,
    allowed_origins: BTreeSet<String>,
    reverse_proxy_trusted: bool,
    approval: RemoteExposureApproval,
}

impl NativeApiExposure {
    pub fn new(
        bind_address: IpAddr,
        tls_enabled: bool,
        allowed_origins: impl IntoIterator<Item = String>,
        reverse_proxy_trusted: bool,
        approval: RemoteExposureApproval,
    ) -> Result<Self, TrustError> {
        let allowed_origins = allowed_origins
            .into_iter()
            .map(|origin| canonical_origin(&origin))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let exposure = Self {
            bind_address,
            tls_enabled,
            allowed_origins,
            reverse_proxy_trusted,
            approval,
        };
        exposure.validate()?;
        Ok(exposure)
    }

    pub fn bind_address(&self) -> IpAddr {
        self.bind_address
    }

    pub fn tls_enabled(&self) -> bool {
        self.tls_enabled
    }

    pub fn allowed_origins(&self) -> &BTreeSet<String> {
        &self.allowed_origins
    }

    pub fn reverse_proxy_trusted(&self) -> bool {
        self.reverse_proxy_trusted
    }

    fn validate(&self) -> Result<(), TrustError> {
        if self.bind_address.is_loopback() {
            return Ok(());
        }
        if self.approval != RemoteExposureApproval::Approved
            || !self.tls_enabled
            || self.allowed_origins.is_empty()
        {
            return Err(TrustError::UnsafeApiExposure);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalNavigationPolicy {
    allowed_schemes: BTreeSet<String>,
    require_user_gesture: bool,
}

impl ExternalNavigationPolicy {
    pub fn https_user_gesture() -> Self {
        Self {
            allowed_schemes: BTreeSet::from(["https".to_owned()]),
            require_user_gesture: true,
        }
    }

    pub fn new(
        allowed_schemes: impl IntoIterator<Item = String>,
        require_user_gesture: bool,
    ) -> Result<Self, TrustError> {
        let allowed_schemes = allowed_schemes
            .into_iter()
            .map(|scheme| scheme.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        if allowed_schemes.is_empty()
            || allowed_schemes
                .iter()
                .any(|scheme| !valid_navigation_scheme(scheme))
        {
            return Err(TrustError::InvalidNavigationPolicy);
        }
        Ok(Self {
            allowed_schemes,
            require_user_gesture,
        })
    }

    pub fn authorize(&self, url: &str, user_gesture: bool) -> Result<(), TrustError> {
        if url.len() > 16 * 1024 || url.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(TrustError::NavigationDenied);
        }
        let parsed = Url::parse(url).map_err(|_| TrustError::NavigationDenied)?;
        if !self.allowed_schemes.contains(parsed.scheme())
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || matches!(parsed.scheme(), "http" | "https") && !parsed.has_host()
            || (self.require_user_gesture && !user_gesture)
        {
            return Err(TrustError::NavigationDenied);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeFfiContract {
    library_id: String,
    digest_sha256: String,
    abi_version: String,
    required_symbols: BTreeSet<String>,
    required_by: Option<String>,
    unsafe_owner: String,
}

impl NativeFfiContract {
    pub fn new(
        library_id: impl Into<String>,
        digest_sha256: impl Into<String>,
        abi_version: impl Into<String>,
        required_symbols: impl IntoIterator<Item = String>,
        unsafe_owner: impl Into<String>,
    ) -> Result<Self, TrustError> {
        let contract = Self {
            library_id: library_id.into(),
            digest_sha256: digest_sha256.into(),
            abi_version: abi_version.into(),
            required_symbols: required_symbols.into_iter().collect(),
            required_by: None,
            unsafe_owner: unsafe_owner.into(),
        };
        contract.validate()?;
        Ok(contract)
    }

    pub fn new_dependency(
        library_id: impl Into<String>,
        digest_sha256: impl Into<String>,
        abi_version: impl Into<String>,
        required_by: impl Into<String>,
        unsafe_owner: impl Into<String>,
    ) -> Result<Self, TrustError> {
        let contract = Self {
            library_id: library_id.into(),
            digest_sha256: digest_sha256.into(),
            abi_version: abi_version.into(),
            required_symbols: BTreeSet::new(),
            required_by: Some(required_by.into()),
            unsafe_owner: unsafe_owner.into(),
        };
        contract.validate()?;
        Ok(contract)
    }

    fn validate(&self) -> Result<(), TrustError> {
        if self.library_id.trim().is_empty()
            || validate_sha256(&self.digest_sha256).is_err()
            || self.abi_version.trim().is_empty()
            || (self.required_symbols.is_empty() == self.required_by.is_none())
            || self.required_by.as_ref().is_some_and(|required_by| {
                required_by.trim().is_empty()
                    || required_by == &self.library_id
                    || !required_by
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
            })
            || self.required_symbols.iter().any(|symbol| {
                symbol.is_empty()
                    || !symbol
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
            })
            || self.unsafe_owner.trim().is_empty()
        {
            return Err(TrustError::InvalidFfiContract);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeFfiRegistry {
    contracts: BTreeMap<String, NativeFfiContract>,
}

impl NativeFfiRegistry {
    pub fn new(contracts: impl IntoIterator<Item = NativeFfiContract>) -> Result<Self, TrustError> {
        let mut checked_contracts = BTreeMap::new();
        for contract in contracts {
            contract.validate()?;
            let library_id = contract.library_id.clone();
            if checked_contracts
                .insert(library_id.clone(), contract)
                .is_some()
            {
                return Err(TrustError::DuplicateFfiContract(library_id));
            }
        }
        for contract in checked_contracts.values() {
            let mut current = contract;
            let mut visited = BTreeSet::new();
            while let Some(required_by) = current.required_by.as_deref() {
                if !visited.insert(current.library_id.as_str()) {
                    return Err(TrustError::InvalidFfiContract);
                }
                current = checked_contracts
                    .get(required_by)
                    .ok_or(TrustError::InvalidFfiContract)?;
            }
        }
        Ok(Self {
            contracts: checked_contracts,
        })
    }

    pub fn authorize(
        &self,
        library_id: &str,
        digest_sha256: &str,
        abi_version: &str,
        available_symbols: &BTreeSet<String>,
    ) -> Result<CertifiedNativeFfi, TrustError> {
        let contract = self
            .contracts
            .get(library_id)
            .ok_or(TrustError::UncertifiedFfi)?;
        if contract.required_by.is_some()
            || !constant_time_equal(contract.digest_sha256.as_bytes(), digest_sha256.as_bytes())
            || contract.abi_version != abi_version
            || !contract.required_symbols.is_subset(available_symbols)
        {
            return Err(TrustError::UncertifiedFfi);
        }
        Ok(CertifiedNativeFfi {
            library_id: contract.library_id.clone(),
            digest_sha256: contract.digest_sha256.clone(),
            abi_version: contract.abi_version.clone(),
            required_symbols: contract.required_symbols.clone(),
            unsafe_owner: contract.unsafe_owner.clone(),
        })
    }

    pub fn authorize_dependency(
        &self,
        library_id: &str,
        digest_sha256: &str,
        abi_version: &str,
        required_by: &str,
    ) -> Result<CertifiedNativeFfi, TrustError> {
        let contract = self
            .contracts
            .get(library_id)
            .ok_or(TrustError::UncertifiedFfi)?;
        if !contract.required_symbols.is_empty()
            || contract.required_by.as_deref() != Some(required_by)
            || !constant_time_equal(contract.digest_sha256.as_bytes(), digest_sha256.as_bytes())
            || contract.abi_version != abi_version
        {
            return Err(TrustError::UncertifiedFfi);
        }
        Ok(CertifiedNativeFfi {
            library_id: contract.library_id.clone(),
            digest_sha256: contract.digest_sha256.clone(),
            abi_version: contract.abi_version.clone(),
            required_symbols: BTreeSet::new(),
            unsafe_owner: contract.unsafe_owner.clone(),
        })
    }

    pub fn required_symbols_for(
        &self,
        library_id: &str,
        abi_version: &str,
        unsafe_owner: &str,
    ) -> Result<BTreeSet<String>, TrustError> {
        let contract = self
            .contracts
            .get(library_id)
            .ok_or(TrustError::UncertifiedFfi)?;
        if contract.abi_version != abi_version
            || contract.unsafe_owner != unsafe_owner
            || contract.required_symbols.is_empty()
        {
            return Err(TrustError::UncertifiedFfi);
        }
        Ok(contract.required_symbols.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifiedNativeFfi {
    library_id: String,
    digest_sha256: String,
    abi_version: String,
    required_symbols: BTreeSet<String>,
    unsafe_owner: String,
}

impl CertifiedNativeFfi {
    pub fn library_id(&self) -> &str {
        &self.library_id
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub fn abi_version(&self) -> &str {
        &self.abi_version
    }

    pub fn required_symbols(&self) -> &BTreeSet<String> {
        &self.required_symbols
    }

    pub fn unsafe_owner(&self) -> &str {
        &self.unsafe_owner
    }
}

pub fn cudart_exact_native<'a>(
    certification: &'a CertifiedNativeFfi,
    cancellation: &CancellationToken,
) -> Result<&'a CertifiedNativeFfi, TrustError> {
    cancellation.check().map_err(|_| TrustError::Cancelled)?;
    if certification.library_id() != CUDART_LIBRARY_ID
        || !certification
            .required_symbols()
            .contains("cudaHostRegister")
        || !certification
            .required_symbols()
            .contains("cudaHostUnregister")
    {
        return Err(TrustError::UncertifiedFfi);
    }
    cancellation.check().map_err(|_| TrustError::Cancelled)?;
    Ok(certification)
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum TrustError {
    #[error("trust-policy operation was cancelled")]
    Cancelled,
    #[error("plugin signature is missing or invalid")]
    InvalidPluginSignature,
    #[error("plugin verification key is invalid")]
    InvalidVerificationKey,
    #[error("plugin verification key is duplicated: {0}")]
    DuplicateVerificationKey(String),
    #[error("plugin verification key is not trusted: {0}")]
    UnknownVerificationKey(String),
    #[error("ROCm package verification key is invalid")]
    InvalidRocmPackageVerificationKey,
    #[error("ROCm package signer is not the explicitly configured authority")]
    UnknownRocmPackageSigner,
    #[error("ROCm package signature is missing, malformed, or invalid")]
    InvalidRocmPackageSignature,
    #[error("Metal package verification key is invalid")]
    InvalidMetalPackageVerificationKey,
    #[error("Metal package signer is not the explicitly configured authority")]
    UnknownMetalPackageSigner,
    #[error("Metal package signature is missing, malformed, or invalid")]
    InvalidMetalPackageSignature,
    #[error("MLU package verification key is invalid")]
    InvalidMluPackageVerificationKey,
    #[error("MLU package signer is not the explicitly configured authority")]
    UnknownMluPackageSigner,
    #[error("MLU package signature is missing, malformed, or invalid")]
    InvalidMluPackageSignature,
    #[error("NPU package verification key is invalid")]
    InvalidNpuPackageVerificationKey,
    #[error("NPU package signer is not the explicitly configured authority")]
    UnknownNpuPackageSigner,
    #[error("NPU package signature is missing, malformed, or invalid")]
    InvalidNpuPackageSignature,
    #[error("XPU package verification key is invalid")]
    InvalidXpuPackageVerificationKey,
    #[error("XPU package signer is not the explicitly configured authority")]
    UnknownXpuPackageSigner,
    #[error("XPU package signature is missing, malformed, or invalid")]
    InvalidXpuPackageSignature,
    #[error("CUDA package verification key is invalid")]
    InvalidCudaPackageVerificationKey,
    #[error("CUDA package signer is not the explicitly configured authority")]
    UnknownCudaPackageSigner,
    #[error("CUDA package signature is missing, malformed, or invalid")]
    InvalidCudaPackageSignature,
    #[error("DirectML package verification key is invalid")]
    InvalidDirectMlPackageVerificationKey,
    #[error("DirectML package signer is not the explicitly configured authority")]
    UnknownDirectMlPackageSigner,
    #[error("DirectML package signature is missing, malformed, or invalid")]
    InvalidDirectMlPackageSignature,
    #[error("plugin authorization does not belong to this manifest")]
    AuthorizationManifestMismatch,
    #[error("sealed plugin authorization is malformed or internally inconsistent")]
    InvalidSealedPluginAuthorization,
    #[error("sealed plugin authorization exceeds its bounded wire size")]
    SealedPluginAuthorizationTooLarge,
    #[error("plugin authorization sealing key generation failed")]
    AuthorizationSealingUnavailable,
    #[error("plugin authorization verification key is invalid")]
    InvalidAuthorizationVerificationKey,
    #[error(transparent)]
    PluginContract(#[from] PluginContractError),
    #[error(transparent)]
    Permission(#[from] PermissionError),
    #[error("secret identifier is invalid")]
    InvalidSecretId,
    #[error("provider identifier is invalid")]
    InvalidProviderId,
    #[error("provider credential scope is invalid")]
    InvalidCredentialScope,
    #[error("provider profile is invalid")]
    InvalidProviderProfile,
    #[error("provider endpoint is invalid")]
    InvalidProviderEndpoint,
    #[error("provider is disabled")]
    ProviderDisabled,
    #[error("provider request is forbidden while offline")]
    Offline,
    #[error("provider request belongs to a different profile")]
    ProviderProfileMismatch,
    #[error("provider endpoint is not explicitly allowed")]
    ProviderEndpointDenied,
    #[error("provider secret grant is missing")]
    MissingSecretGrant,
    #[error("provider price bound is invalid")]
    InvalidProviderPriceBound,
    #[error("provider cost acceptance is invalid or does not match the request")]
    InvalidProviderCostAcceptance,
    #[error("provider cost acceptance has expired")]
    ExpiredProviderCostAcceptance,
    #[error("provider cost acceptance sealing key generation failed")]
    ProviderCostAcceptanceSealingUnavailable,
    #[error("native API remote exposure is unsafe")]
    UnsafeApiExposure,
    #[error("external navigation policy is invalid")]
    InvalidNavigationPolicy,
    #[error("external navigation is not permitted")]
    NavigationDenied,
    #[error("native FFI contract is invalid")]
    InvalidFfiContract,
    #[error("native FFI contract is duplicated: {0}")]
    DuplicateFfiContract(String),
    #[error("native FFI library is not certified")]
    UncertifiedFfi,
}

fn validate_sha256(value: &str) -> Result<(), ()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(())
    }
}

fn valid_ascii_identifier(value: &str, maximum_bytes: usize) -> bool {
    if value.is_empty() || value.len() > maximum_bytes || !value.is_ascii() {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    let mut previous_separator = false;
    for byte in bytes {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_separator = false;
        } else if matches!(byte, b'.' | b'-' | b'_') && !previous_separator {
            previous_separator = true;
        } else {
            return false;
        }
    }
    true
}

fn canonical_origin(origin: &str) -> Result<String, TrustError> {
    if origin.bytes().any(|byte| byte.is_ascii_control()) || origin.contains('*') {
        return Err(TrustError::UnsafeApiExposure);
    }
    let parsed = Url::parse(origin).map_err(|_| TrustError::UnsafeApiExposure)?;
    let host = parsed.host().ok_or(TrustError::UnsafeApiExposure)?;
    let insecure_loopback = parsed.scheme() == "http"
        && match host {
            Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
            Host::Ipv4(address) => address.is_loopback(),
            Host::Ipv6(address) => address.is_loopback(),
        };
    if (parsed.scheme() != "https" && !insecure_loopback)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(TrustError::UnsafeApiExposure);
    }
    Ok(parsed.origin().ascii_serialization())
}

fn valid_navigation_scheme(scheme: &str) -> bool {
    let mut bytes = scheme.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        && !matches!(scheme, "data" | "file" | "javascript" | "vbscript")
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn decode_hex_exact<const LENGTH: usize>(value: &str) -> Option<[u8; LENGTH]> {
    if value.len() != LENGTH * 2 {
        return None;
    }
    let mut decoded = [0_u8; LENGTH];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        decoded[index] = (high << 4) | low;
    }
    Some(decoded)
}

fn decode_hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

pub(crate) fn parse_strict_json_value(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let strict =
        StrictJsonValue::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    Ok(strict.0)
}

struct StrictJsonValue(serde_json::Value);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictJsonValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::Bool(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::Number(value.into())))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::Number(value.into())))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                serde_json::Number::from_f64(value)
                    .map(serde_json::Value::Number)
                    .map(StrictJsonValue)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::String(value.to_owned())))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::String(value)))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::Null))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(StrictJsonValue(serde_json::Value::Null))
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<StrictJsonValue>()? {
                    values.push(value.0);
                }
                Ok(StrictJsonValue(serde_json::Value::Array(values)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate object key: {key}"
                        )));
                    }
                    let value = map.next_value::<StrictJsonValue>()?;
                    values.insert(key, value.0);
                }
                Ok(StrictJsonValue(serde_json::Value::Object(values)))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

fn authorization_signing_payload(payload: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(PLUGIN_AUTHORIZATION_SIGNATURE_DOMAIN);
    digest.update(
        u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    digest.update(payload);
    digest.finalize().into()
}

fn provider_cost_milliseconds(origin: Instant, value: Instant) -> Result<u64, TrustError> {
    let duration = value
        .checked_duration_since(origin)
        .ok_or(TrustError::InvalidProviderCostAcceptance)?;
    u64::try_from(duration.as_millis()).map_err(|_| TrustError::InvalidProviderCostAcceptance)
}

fn provider_cost_acceptance_signing_payload(claims: &ProviderCostAcceptanceClaims) -> [u8; 32] {
    fn update_field(digest: &mut Sha256, value: &[u8]) {
        digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
        digest.update(value);
    }

    let mut digest = Sha256::new();
    digest.update(PROVIDER_COST_ACCEPTANCE_SIGNATURE_DOMAIN);
    for field in [
        claims.scope.principal_id().as_bytes(),
        claims.scope.profile_id().as_bytes(),
        claims.scope.prompt_id().as_bytes(),
        claims.scope.prompt_sha256().as_bytes(),
        claims.scope.attempt_id().as_bytes(),
        claims.scope.node_id().as_bytes(),
        claims.scope.request_sha256().as_bytes(),
        claims.scope.plugin_id().as_bytes(),
        claims.scope.plugin_digest_sha256().as_bytes(),
        claims.scope.provider_binding_sha256().as_bytes(),
        claims.scope.provider().as_bytes(),
        claims.scope.endpoint().as_bytes(),
        claims.scope.price_bound().currency_code().as_bytes(),
    ] {
        update_field(&mut digest, field);
    }
    digest.update(claims.scope.request_ordinal().to_le_bytes());
    digest.update(
        claims
            .scope
            .price_bound()
            .maximum_microunits()
            .to_le_bytes(),
    );
    digest.update(claims.issued_at_milliseconds.to_le_bytes());
    digest.update(claims.expires_at_milliseconds.to_le_bytes());
    digest.update(claims.nonce.as_bytes());
    digest.finalize().into()
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs, io,
        io::{Seek as _, SeekFrom, Write as _},
        net::Ipv4Addr,
        path::{Path, PathBuf},
        str::FromStr,
    };

    use comfy_model::{ParserLimits, RestrictedPickleError, parse_restricted_pickle};
    use comfy_plugin_sdk::{
        ApiRequirement, ApiVersion, CachePolicy, CapabilityKind, CapabilityQuota,
        CapabilityRequest, DeterminismPolicy, EffectPolicy, ManifestProvenance, ManifestSignature,
        PluginManifest, PluginNode, PluginSigningKey,
    };
    use serde_json::json;

    use crate::{Capability, PermissionGrant};

    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const SIGNING_KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";

    const PACKAGE_LIMITS: [NativePackagePayloadLimit; 4] = [
        NativePackagePayloadLimit::new("a.txt", 16),
        NativePackagePayloadLimit::new("nested/b.bin", 16),
        NativePackagePayloadLimit::new("package-coverage.sha256", 1024),
        NativePackagePayloadLimit::new("signature.sig", 1024),
    ];

    #[test]
    fn native_library_image_capture_owns_bounds_identity_digest_and_cancellation()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            NativeLibraryImageError::UnsupportedPlatform.to_string(),
            "native-library image capture or sealing is unsupported on this platform"
        );
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("libfixture.so");
        fs::write(&source, b"data")?;
        let cancellation = CancellationToken::default();
        let captured =
            capture_native_library_image_with_limit(&source, 4, &mut || cancellation.check())?;
        assert_eq!(captured.bytes(), b"data");
        assert_eq!(
            captured.digest_sha256(),
            format!("{:x}", Sha256::digest(b"data"))
        );

        fs::write(&source, b"")?;
        assert!(matches!(
            capture_native_library_image_with_limit(&source, 4, &mut || Ok(())),
            Err(NativeLibraryImageError::Invalid(reason)) if reason.contains("nonempty regular file")
        ));
        fs::write(&source, b"overs")?;
        assert!(matches!(
            capture_native_library_image_with_limit(&source, 4, &mut || Ok(())),
            Err(NativeLibraryImageError::Invalid(reason)) if reason.contains("no larger than 4")
        ));
        assert!(matches!(
            capture_native_library_image_with_limit(directory.path(), 4, &mut || Ok(())),
            Err(NativeLibraryImageError::Invalid(reason)) if reason.contains("regular file")
        ));

        fs::write(&source, vec![7_u8; NATIVE_LIBRARY_IMAGE_CHUNK_BYTES * 2])?;
        let cancellation = CancellationToken::default();
        let checks = std::cell::Cell::new(0_usize);
        assert!(matches!(
            capture_native_library_image_with_limit(
                &source,
                u64::try_from(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES * 2)?,
                &mut || {
                    let current = checks.get();
                    checks.set(current + 1);
                    if current == 1 {
                        cancellation.cancel();
                    }
                    cancellation.check()
                },
            ),
            Err(NativeLibraryImageError::Cancelled)
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn native_library_image_capture_rejects_symlinks_and_concurrent_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir()?;
        let source = directory.path().join("libfixture.so");
        let link = directory.path().join("libfixture-link.so");
        fs::write(&source, vec![1_u8; NATIVE_LIBRARY_IMAGE_CHUNK_BYTES * 2])?;
        symlink(&source, &link)?;
        assert!(matches!(
            capture_native_library_image(&link, &CancellationToken::default()),
            Err(NativeLibraryImageError::Invalid(_))
        ));

        let checks = std::cell::Cell::new(0_usize);
        assert!(matches!(
            capture_native_library_image_with_limit(
                &source,
                u64::try_from(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES * 3)?,
                &mut || {
                    let current = checks.get();
                    checks.set(current + 1);
                    if current == 2 {
                        fs::write(&source, b"changed length").map_err(|_| CancellationError)?;
                    }
                    Ok(())
                },
            ),
            Err(NativeLibraryImageError::Invalid(reason))
                if reason.contains("changed while it was captured")
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn native_library_image_capture_rejects_growth_shrink_and_in_place_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        fn capture_while_mutating(
            path: &Path,
            mutate: impl FnOnce(&Path) -> io::Result<()>,
        ) -> Result<CapturedNativeLibraryImage, NativeLibraryImageError> {
            let checks = std::cell::Cell::new(0_usize);
            let mut mutate = Some(mutate);
            capture_native_library_image_with_limit(
                path,
                u64::try_from(NATIVE_LIBRARY_IMAGE_CHUNK_BYTES * 3).unwrap_or(u64::MAX),
                &mut || {
                    let current = checks.get();
                    checks.set(current + 1);
                    if current == 2
                        && let Some(mutate) = mutate.take()
                    {
                        mutate(path).map_err(|_| CancellationError)?;
                    }
                    Ok(())
                },
            )
        }

        let directory = tempfile::tempdir()?;
        let source = directory.path().join("libfixture.so");
        let original = vec![11_u8; NATIVE_LIBRARY_IMAGE_CHUNK_BYTES * 2];

        fs::write(&source, &original)?;
        assert!(matches!(
            capture_while_mutating(&source, |path| {
                OpenOptions::new().append(true).open(path)?.write_all(b"growth")
            }),
            Err(NativeLibraryImageError::Invalid(reason))
                if reason.contains("changed while it was captured")
        ));

        fs::write(&source, &original)?;
        assert!(matches!(
            capture_while_mutating(&source, |path| {
                OpenOptions::new().write(true).open(path)?.set_len(1)
            }),
            Err(NativeLibraryImageError::Invalid(reason))
                if reason.contains("changed while it was captured")
        ));

        fs::write(&source, &original)?;
        assert!(matches!(
            capture_while_mutating(&source, |path| {
                let mut file = OpenOptions::new().write(true).open(path)?;
                file.seek(SeekFrom::Start(0))?;
                file.write_all(b"mutated")
            }),
            Err(NativeLibraryImageError::Invalid(reason))
                if reason.contains("changed while it was captured")
        ));
        Ok(())
    }

    #[test]
    fn retained_native_library_image_owns_loader_path_lifetime()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("libfixture.so");
        fs::write(&source, b"immutable")?;
        let retained = capture_native_library_image(&source, &CancellationToken::default())?
            .seal("fixture", &CancellationToken::default())?;
        let loader_path = retained.loader_path().to_path_buf();
        assert_eq!(fs::read(&loader_path)?, b"immutable");
        assert_eq!(retained.file().metadata()?.len(), 9);

        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            use std::os::unix::fs::MetadataExt;

            let required =
                libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
            let actual = unsafe { libc::fcntl(retained.file().as_raw_fd(), libc::F_GET_SEALS) };
            assert_eq!(actual & required, required);
            assert_eq!(
                retained.file().metadata()?.ino(),
                fs::metadata(&loader_path)?.ino()
            );
            assert!(
                fs::OpenOptions::new()
                    .write(true)
                    .open(&loader_path)
                    .is_err()
            );
            assert!(retained.file().set_len(0).is_err());
        }

        drop(retained);
        assert!(!loader_path.exists());
        Ok(())
    }

    fn package_fixture_root()
    -> Result<(tempfile::TempDir, ArtifactRoot), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let root = ArtifactRoot::canonical(
            "native-package-admission-fixture",
            "native-ffi-package",
            directory.path(),
            std::iter::empty::<String>(),
        )?;
        root.write_private_file("a.txt", b"alpha")?;
        root.write_private_file("nested/b.bin", b"bravo")?;
        root.write_private_file("package-coverage.sha256", b"pending\n")?;
        root.write_private_file("signature.sig", b"pending\n")?;
        Ok((directory, root))
    }

    #[test]
    fn native_package_capture_owns_exact_bounded_stable_membership()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_directory, root) = package_fixture_root()?;
        let cancellation = CancellationToken::default();
        let payloads = capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancellation)?;
        assert_eq!(payloads.len(), PACKAGE_LIMITS.len());
        assert_eq!(
            payloads.get("a.txt").map(Vec::as_slice),
            Some(b"alpha".as_slice())
        );

        root.write_private_file("extra.bin", b"extra")?;
        assert!(matches!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancellation),
            Err(NativePackageAdmissionError::UnsafePackage(message))
                if message.contains("membership")
        ));
        root.quarantine_private_file("extra.bin")?;
        root.quarantine_private_file("a.txt")?;
        assert!(matches!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancellation),
            Err(NativePackageAdmissionError::UnsafePackage(message))
                if message.contains("membership")
        ));

        let (_directory, root) = package_fixture_root()?;
        let quarantined = root
            .quarantine_private_file("a.txt")?
            .ok_or("fixture payload was not quarantined")?;
        assert!(root.remove_contained_file(&quarantined)?);
        fs::create_dir(root.canonical_path().join("a.txt"))?;
        assert!(matches!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancellation),
            Err(NativePackageAdmissionError::UnsafePackage(message))
                if message.contains("membership")
        ));
        Ok(())
    }

    #[test]
    fn native_package_capture_rejects_empty_oversized_replacement_and_cancellation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_directory, root) = package_fixture_root()?;
        let cancellation = CancellationToken::default();
        root.write_private_file("a.txt", b"")?;
        assert!(matches!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancellation),
            Err(NativePackageAdmissionError::UnsafePackage(message))
                if message.contains("empty")
        ));

        root.write_private_file("a.txt", b"more-than-sixteen-bytes")?;
        assert!(matches!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancellation),
            Err(NativePackageAdmissionError::UnsafePackage(_))
        ));

        root.write_private_file("a.txt", b"alpha")?;
        assert!(matches!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 20, &cancellation),
            Err(NativePackageAdmissionError::UnsafePackage(message))
                if message.contains("aggregate bound")
        ));
        assert!(matches!(
            capture_native_package_with_hook(
                &root,
                &PACKAGE_LIMITS,
                8,
                4096,
                &cancellation,
                || {
                    root.write_private_file("a.txt", b"omega")
                        .map_err(|error| {
                            NativePackageAdmissionError::UnsafePackage(error.to_string())
                        })
                },
            ),
            Err(NativePackageAdmissionError::UnsafePackage(message))
                if message.contains("changed after capture")
        ));

        let cancelled = CancellationToken::default();
        assert!(cancelled.cancel());
        assert_eq!(
            capture_native_package(&root, &PACKAGE_LIMITS, 8, 4096, &cancelled),
            Err(NativePackageAdmissionError::Cancelled)
        );
        Ok(())
    }

    #[test]
    fn native_package_coverage_is_exact_sorted_unique_and_content_bound() {
        let payloads = BTreeMap::from([
            ("a.txt".to_owned(), b"alpha".to_vec()),
            ("nested/b.bin".to_owned(), b"bravo".to_vec()),
            ("package-coverage.sha256".to_owned(), b"pending\n".to_vec()),
            ("signature.sig".to_owned(), b"pending\n".to_vec()),
        ]);
        let row = |path: &str, bytes: &[u8]| {
            format!("{:x} {}  {path}\n", Sha256::digest(bytes), bytes.len())
        };
        let first = row("a.txt", b"alpha");
        let second = row("nested/b.bin", b"bravo");
        let valid = format!("{first}{second}");
        let excludes = ["package-coverage.sha256", "signature.sig"];
        assert!(
            validate_native_package_coverage(valid.as_bytes(), &payloads, &excludes, 1024).is_ok()
        );

        let cases = [
            format!("{second}{first}"),
            format!("{first}{first}{second}"),
            first,
            valid.replace("nested/b.bin", "unknown.bin"),
            valid.replacen(
                &format!("{:x}", Sha256::digest(b"alpha")),
                &"0".repeat(64),
                1,
            ),
            valid.replacen("5  a.txt", "6  a.txt", 1),
        ];
        for invalid in cases {
            assert!(matches!(
                validate_native_package_coverage(invalid.as_bytes(), &payloads, &excludes, 1024),
                Err(NativePackageAdmissionError::InvalidCoverage(_))
            ));
        }
    }

    #[test]
    fn rocm_package_signature_has_a_distinct_signer_bound_domain()
    -> Result<(), Box<dyn std::error::Error>> {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(SIGNING_KEY)
            .map_err(|error| std::io::Error::other(format!("fixture key rejected: {error:?}")))?;
        let verifier =
            RocmPackageVerificationKey::new("rocm.release", key_pair.public_key().as_ref())?;
        let coverage = b"fixture coverage\n";
        let payload = rocm_package_signing_payload("rocm.release", coverage)?;
        let signature = encode_hex(key_pair.sign(&payload).as_ref());
        let signed_receipt = |signature: String| -> Result<Vec<u8>, serde_json::Error> {
            let mut bytes = serde_json::to_vec(&NativePackageSignatureReceipt {
                schema_version: 1,
                algorithm: "ed25519".to_owned(),
                signature,
            })?;
            bytes.push(b'\n');
            Ok(bytes)
        };
        let receipt = signed_receipt(signature.clone())?;
        verifier.verify_package("rocm.release", coverage, &receipt)?;
        let metal_verifier =
            MetalPackageVerificationKey::new("rocm.release", key_pair.public_key().as_ref())?;
        assert_eq!(
            metal_verifier.verify_package("rocm.release", coverage, &receipt),
            Err(TrustError::InvalidMetalPackageSignature)
        );
        let metal_payload = metal_package_signing_payload("rocm.release", coverage)?;
        let metal_receipt = signed_receipt(encode_hex(key_pair.sign(&metal_payload).as_ref()))?;
        metal_verifier.verify_package("rocm.release", coverage, &metal_receipt)?;
        assert_eq!(
            verifier.verify_package("rocm.release", coverage, &metal_receipt),
            Err(TrustError::InvalidRocmPackageSignature)
        );
        let unknown_key_pair =
            Ed25519KeyPair::from_seed_unchecked(b"abcdef0123456789abcdef0123456789")
                .map_err(|error| io::Error::other(format!("fixture key rejected: {error:?}")))?;
        let unknown_metal_key = MetalPackageVerificationKey::new(
            "rocm.release",
            unknown_key_pair.public_key().as_ref(),
        )?;
        assert_eq!(
            unknown_metal_key.verify_package("rocm.release", coverage, &metal_receipt),
            Err(TrustError::InvalidMetalPackageSignature)
        );
        let duplicate_receipt = String::from_utf8(receipt.clone())?
            .replacen(
                "{\"schema_version\":1,",
                "{\"schema_version\":1,\"schema_version\":1,",
                1,
            )
            .into_bytes();
        assert_eq!(
            verifier.verify_package("rocm.release", coverage, &duplicate_receipt),
            Err(TrustError::InvalidRocmPackageSignature)
        );
        assert_eq!(
            metal_verifier.verify_package("rocm.release", coverage, &duplicate_receipt),
            Err(TrustError::InvalidMetalPackageSignature)
        );
        let mut noncanonical_receipt = receipt.clone();
        if noncanonical_receipt.last() == Some(&b'\n') {
            noncanonical_receipt.pop();
        }
        assert_eq!(
            verifier.verify_package("rocm.release", coverage, &noncanonical_receipt),
            Err(TrustError::InvalidRocmPackageSignature)
        );
        assert_eq!(
            verifier.verify_package("plugin.release", coverage, &receipt),
            Err(TrustError::UnknownRocmPackageSigner)
        );
        assert_eq!(
            verifier.verify_package("rocm.release", b"changed coverage\n", &receipt),
            Err(TrustError::InvalidRocmPackageSignature)
        );
        let plugin_domain_signature = encode_hex(
            key_pair
                .sign(&authorization_signing_payload(coverage))
                .as_ref(),
        );
        let plugin_domain_receipt = signed_receipt(plugin_domain_signature)?;
        assert_eq!(
            verifier.verify_package("rocm.release", coverage, &plugin_domain_receipt),
            Err(TrustError::InvalidRocmPackageSignature)
        );
        let receipt_with_unknown_field = serde_json::to_vec(&json!({
            "schema_version": 1,
            "algorithm": "ed25519",
            "signature": signature,
            "plugin_key_id": "must-not-authorize-backends",
        }))?;
        assert_eq!(
            verifier.verify_package("rocm.release", coverage, &receipt_with_unknown_field),
            Err(TrustError::InvalidRocmPackageSignature)
        );
        assert_eq!(
            RocmPackageVerificationKey::new("plugin release", key_pair.public_key().as_ref()),
            Err(TrustError::InvalidRocmPackageVerificationKey)
        );
        Ok(())
    }

    #[test]
    fn directml_package_signature_has_a_distinct_signer_bound_domain()
    -> Result<(), Box<dyn std::error::Error>> {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(SIGNING_KEY)
            .map_err(|error| io::Error::other(format!("fixture key rejected: {error:?}")))?;
        let verifier = DirectMlPackageVerificationKey::new(
            "directml.release",
            key_pair.public_key().as_ref(),
        )?;
        let coverage = b"fixture coverage\n";
        let payload = directml_package_signing_payload("directml.release", coverage)?;
        let signature = encode_hex(key_pair.sign(&payload).as_ref());
        let mut receipt = serde_json::to_vec(&NativePackageSignatureReceipt {
            schema_version: 1,
            algorithm: "ed25519".to_owned(),
            signature,
        })?;
        receipt.push(b'\n');

        verifier.verify_package("directml.release", coverage, &receipt)?;
        assert_eq!(
            verifier.verify_package("plugin.release", coverage, &receipt),
            Err(TrustError::UnknownDirectMlPackageSigner)
        );
        assert_eq!(
            verifier.verify_package("directml.release", b"changed coverage\n", &receipt),
            Err(TrustError::InvalidDirectMlPackageSignature)
        );

        let mlu_verifier =
            MluPackageVerificationKey::new("directml.release", key_pair.public_key().as_ref())?;
        assert_eq!(
            mlu_verifier.verify_package("directml.release", coverage, &receipt),
            Err(TrustError::InvalidMluPackageSignature)
        );

        let plugin_signature = encode_hex(
            key_pair
                .sign(&authorization_signing_payload(coverage))
                .as_ref(),
        );
        let mut plugin_receipt = serde_json::to_vec(&NativePackageSignatureReceipt {
            schema_version: 1,
            algorithm: "ed25519".to_owned(),
            signature: plugin_signature,
        })?;
        plugin_receipt.push(b'\n');
        assert_eq!(
            verifier.verify_package("directml.release", coverage, &plugin_receipt),
            Err(TrustError::InvalidDirectMlPackageSignature)
        );

        let duplicate_receipt = String::from_utf8(receipt.clone())?
            .replacen(
                "{\"schema_version\":1,",
                "{\"schema_version\":1,\"schema_version\":1,",
                1,
            )
            .into_bytes();
        assert_eq!(
            verifier.verify_package("directml.release", coverage, &duplicate_receipt),
            Err(TrustError::InvalidDirectMlPackageSignature)
        );
        assert_eq!(
            DirectMlPackageVerificationKey::new("plugin release", key_pair.public_key().as_ref()),
            Err(TrustError::InvalidDirectMlPackageVerificationKey)
        );
        Ok(())
    }

    #[test]
    fn task_59_cudart_requires_registry_issued_exact_symbol_certification()
    -> Result<(), Box<dyn std::error::Error>> {
        let contract = NativeFfiContract::new(
            CUDART_LIBRARY_ID,
            DIGEST,
            "cuda-runtime-v1",
            [
                "cudaHostRegister".to_owned(),
                "cudaHostUnregister".to_owned(),
            ],
            "comfy_runtime::NativeFfiRegistry",
        )?;
        let registry = NativeFfiRegistry::new([contract])?;
        let certification = registry.authorize(
            CUDART_LIBRARY_ID,
            DIGEST,
            "cuda-runtime-v1",
            &BTreeSet::from([
                "cudaHostRegister".to_owned(),
                "cudaHostUnregister".to_owned(),
            ]),
        )?;
        let cancellation = CancellationToken::default();
        assert_eq!(
            cudart_exact_native(&certification, &cancellation)?,
            &certification
        );

        let incomplete = NativeFfiContract::new(
            CUDART_LIBRARY_ID,
            DIGEST,
            "cuda-runtime-v1",
            ["cudaHostRegister".to_owned()],
            "comfy_runtime::NativeFfiRegistry",
        )?;
        let incomplete = NativeFfiRegistry::new([incomplete])?.authorize(
            CUDART_LIBRARY_ID,
            DIGEST,
            "cuda-runtime-v1",
            &BTreeSet::from(["cudaHostRegister".to_owned()]),
        )?;
        assert_eq!(
            cudart_exact_native(&incomplete, &cancellation),
            Err(TrustError::UncertifiedFfi)
        );
        let cancelled = CancellationToken::default();
        assert!(cancelled.cancel());
        assert_eq!(
            cudart_exact_native(&certification, &cancelled),
            Err(TrustError::Cancelled)
        );
        Ok(())
    }

    #[test]
    fn native_ffi_requirements_are_available_only_for_the_exact_registered_owner_and_abi()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = NativeFfiRegistry::new([NativeFfiContract::new(
            "rocm-dependency:libfixture.so",
            DIGEST,
            "6.1.0",
            ["fixture_symbol".to_owned()],
            "comfy_backend_rocm::loader",
        )?])?;

        assert_eq!(
            registry.required_symbols_for(
                "rocm-dependency:libfixture.so",
                "6.1.0",
                "comfy_backend_rocm::loader",
            )?,
            BTreeSet::from(["fixture_symbol".to_owned()])
        );
        for (library_id, abi_version, unsafe_owner) in [
            (
                "rocm-dependency:missing.so",
                "6.1.0",
                "comfy_backend_rocm::loader",
            ),
            (
                "rocm-dependency:libfixture.so",
                "6.0.0",
                "comfy_backend_rocm::loader",
            ),
            ("rocm-dependency:libfixture.so", "6.1.0", "comfy_runtime"),
        ] {
            assert_eq!(
                registry.required_symbols_for(library_id, abi_version, unsafe_owner),
                Err(TrustError::UncertifiedFfi)
            );
        }
        Ok(())
    }

    #[test]
    fn native_ffi_dependency_contracts_are_exact_acyclic_and_never_callable()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = NativeFfiRegistry::new([
            NativeFfiContract::new(
                "ascendcl",
                DIGEST,
                "CANN-8.0.RC3-AscendCL",
                ["aclInit".to_owned()],
                "comfy_backend_npu::loader",
            )?,
            NativeFfiContract::new_dependency(
                "runtime",
                DIGEST,
                "CANN-8.0.RC3-AscendCL",
                "ascendcl",
                "comfy_backend_npu::loader",
            )?,
        ])?;
        assert_eq!(
            registry.required_symbols_for(
                "runtime",
                "CANN-8.0.RC3-AscendCL",
                "comfy_backend_npu::loader",
            ),
            Err(TrustError::UncertifiedFfi)
        );
        assert_eq!(
            registry.authorize("runtime", DIGEST, "CANN-8.0.RC3-AscendCL", &BTreeSet::new(),),
            Err(TrustError::UncertifiedFfi)
        );
        registry.authorize_dependency("runtime", DIGEST, "CANN-8.0.RC3-AscendCL", "ascendcl")?;
        assert_eq!(
            registry.authorize_dependency("runtime", DIGEST, "CANN-8.0.RC3-AscendCL", "other",),
            Err(TrustError::UncertifiedFfi)
        );

        assert_eq!(
            NativeFfiRegistry::new([NativeFfiContract::new_dependency(
                "runtime",
                DIGEST,
                "CANN-8.0.RC3-AscendCL",
                "missing",
                "comfy_backend_npu::loader",
            )?]),
            Err(TrustError::InvalidFfiContract)
        );
        assert_eq!(
            NativeFfiRegistry::new([
                NativeFfiContract::new_dependency(
                    "first",
                    DIGEST,
                    "CANN-8.0.RC3-AscendCL",
                    "second",
                    "comfy_backend_npu::loader",
                )?,
                NativeFfiContract::new_dependency(
                    "second",
                    DIGEST,
                    "CANN-8.0.RC3-AscendCL",
                    "first",
                    "comfy_backend_npu::loader",
                )?,
            ]),
            Err(TrustError::InvalidFfiContract)
        );
        Ok(())
    }

    fn permission_policy(capabilities: CapabilitySet) -> Result<PermissionPolicy, PermissionError> {
        PermissionPolicy::new(
            "profile-a",
            [PermissionGrant::new(
                "profile-a",
                "plugin.fixture",
                capabilities,
                "approved-settings",
            )?],
        )
    }

    fn capability(kind: CapabilityKind, scope: &str) -> CapabilityRequest {
        CapabilityRequest {
            kind,
            scope: scope.to_owned(),
            quota: CapabilityQuota {
                maximum_operations: 4,
                maximum_request_bytes: 4_096,
                maximum_response_bytes: 4_096,
                maximum_total_bytes: 8_192,
                maximum_handles: 4,
                timeout_milliseconds: 1_000,
            },
        }
    }

    fn signed_plugin(
        requested_capabilities: Vec<CapabilityRequest>,
    ) -> Result<PluginManifest, TrustError> {
        let mut manifest = PluginManifest {
            schema_version: 1,
            identifier: "plugin.fixture".to_owned(),
            plugin_version: ApiVersion::new(1, 0, 0),
            api: ApiRequirement {
                major: 1,
                minimum_minor: 0,
                maximum_minor: 0,
                required_features: Vec::new(),
            },
            digest_sha256: DIGEST.to_owned(),
            signature: ManifestSignature {
                algorithm: PLUGIN_SIGNATURE_ALGORITHM.to_owned(),
                key_id: "fixture.key".to_owned(),
                value: "0".repeat(ED25519_SIGNATURE_BYTES * 2),
            },
            provenance: ManifestProvenance {
                source: "fixture://plugin.fixture".to_owned(),
                publisher: "fixture publisher".to_owned(),
                registry: Some("fixture://registry".to_owned()),
            },
            provider_binding: None,
            nodes: vec![PluginNode {
                id: "node.fixture".to_owned(),
                version: ApiVersion::new(1, 0, 0),
                display_name: "Fixture".to_owned(),
                category: "tests".to_owned(),
                ports: Vec::new(),
                determinism: DeterminismPolicy::Deterministic,
                cache: CachePolicy::InputIdentity,
                effects: EffectPolicy::Pure,
            }],
            capabilities: requested_capabilities,
            ui: Vec::new(),
            routes: Vec::new(),
            legacy_mappings: Vec::new(),
        };
        manifest.signature.value = signing_key()?.sign_manifest(&manifest)?;
        Ok(manifest)
    }

    fn trust_policy() -> Result<PluginTrustPolicy, TrustError> {
        let signing_key = signing_key()?;
        PluginTrustPolicy::new([PluginVerificationKey::new(
            signing_key.key_id(),
            signing_key.verification_key_bytes()?,
        )?])
    }

    fn signing_key() -> Result<PluginSigningKey, PluginContractError> {
        PluginSigningKey::new("fixture.key", SIGNING_KEY)
    }

    fn reseal_authorization_value(
        value: serde_json::Value,
        sealer: &PluginAuthorizationSealer,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut sealed: SealedPluginAuthorization = serde_json::from_value(value)?;
        let payload = serde_json::to_vec(&sealed.payload)?;
        sealed.signature = encode_hex(&sealer.sign(&authorization_signing_payload(&payload))?);
        Ok(serde_json::to_vec(&sealed)?)
    }

    #[test]
    fn invalid_signatures_and_overprivileged_plugins_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let requested = CapabilitySet::new([Capability::ModelHandle {
            model_id: "model.fixture".to_owned(),
        }]);
        let permissions = permission_policy(requested)?;
        let mut invalid = signed_plugin(vec![capability(CapabilityKind::Model, "model.fixture")])?;
        let replacement = if invalid.signature.value.starts_with('0') {
            "1"
        } else {
            "0"
        };
        invalid.signature.value.replace_range(..1, replacement);
        assert_eq!(
            trust_policy()?.authorize_manifest(&invalid, &permissions),
            Err(TrustError::InvalidPluginSignature)
        );
        let overprivileged = signed_plugin(vec![
            capability(CapabilityKind::Model, "model.fixture"),
            capability(CapabilityKind::Filesystem, "input-root"),
        ])?;
        assert_eq!(
            trust_policy()?.authorize_manifest(&overprivileged, &permissions),
            Err(TrustError::Permission(PermissionError::Denied(vec![
                Capability::Asset {
                    namespace: "input".to_owned(),
                    action: crate::AssetOperation::Read,
                }
            ])))
        );
        Ok(())
    }

    #[test]
    fn plugin_authorization_contains_only_requested_capabilities()
    -> Result<(), Box<dyn std::error::Error>> {
        let read_input = Capability::Asset {
            namespace: "input-root".to_owned(),
            action: crate::AssetOperation::Read,
        };
        let model = Capability::ModelHandle {
            model_id: "model.fixture".to_owned(),
        };
        let granted = CapabilitySet::new([read_input.clone(), model]);
        let manifest = signed_plugin(vec![capability(CapabilityKind::Model, "model.fixture")])?;
        let authorization =
            trust_policy()?.authorize_manifest(&manifest, &permission_policy(granted)?)?;
        assert_eq!(authorization.plugin_id(), "plugin.fixture");
        assert_eq!(authorization.digest_sha256(), DIGEST);
        assert_eq!(authorization.capabilities().capabilities().len(), 1);
        assert!(authorization.capabilities().require(&read_input).is_err());
        authorization.require_manifest(&manifest)?;
        let mut changed_manifest = manifest;
        changed_manifest.nodes[0].display_name = "Changed after authorization".to_owned();
        assert_eq!(
            authorization.require_manifest(&changed_manifest),
            Err(TrustError::AuthorizationManifestMismatch)
        );
        assert_eq!(
            trust_policy()?.authorize_manifest(
                &changed_manifest,
                &permission_policy(CapabilitySet::new([Capability::ModelHandle {
                    model_id: "model.fixture".to_owned(),
                }]))?
            ),
            Err(TrustError::InvalidPluginSignature)
        );
        Ok(())
    }

    #[test]
    fn sealed_plugin_authorization_rejects_tampering_and_untrusted_signers()
    -> Result<(), Box<dyn std::error::Error>> {
        let manifest = signed_plugin(vec![capability(CapabilityKind::Model, "model.fixture")])?;
        let authorization = trust_policy()?.authorize_manifest(
            &manifest,
            &permission_policy(CapabilitySet::new([Capability::ModelHandle {
                model_id: "model.fixture".to_owned(),
            }]))?,
        )?;
        let generation = PermissionPolicyGeneration::new(1)?;
        let sealer = PluginAuthorizationSealer::from_seed([7; 32], generation)?;
        let verifier = sealer.verifier()?;
        let bytes = authorization.sealed_bytes(&sealer)?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &bytes,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            )?,
            authorization
        );

        let mut tampered: serde_json::Value = serde_json::from_slice(&bytes)?;
        tampered["payload"]["digest_sha256"] = serde_json::Value::String("f".repeat(64));
        let tampered = serde_json::to_vec(&tampered)?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &tampered,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );

        let mut expanded_grants: serde_json::Value = serde_json::from_slice(&bytes)?;
        expanded_grants["payload"]["capabilities"]["capabilities"]
            .as_array_mut()
            .ok_or("sealed capability set is not an array")?
            .push(serde_json::to_value(Capability::Asset {
                namespace: "input".to_owned(),
                action: crate::AssetOperation::Read,
            })?);
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &serde_json::to_vec(&expanded_grants)?,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );

        let mut unknown_envelope_field: serde_json::Value = serde_json::from_slice(&bytes)?;
        unknown_envelope_field["future_authority"] = serde_json::json!(true);
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &serde_json::to_vec(&unknown_envelope_field)?,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );
        let mut unknown_payload_field: serde_json::Value = serde_json::from_slice(&bytes)?;
        unknown_payload_field["payload"]["future_authority"] = serde_json::json!(true);
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &serde_json::to_vec(&unknown_payload_field)?,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                bytes
                    .get(..bytes.len().saturating_sub(1))
                    .ok_or("sealed bytes")?,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );

        let mut unsupported_version: serde_json::Value = serde_json::from_slice(&bytes)?;
        unsupported_version["payload"]["version"] =
            serde_json::json!(SEALED_PLUGIN_AUTHORIZATION_VERSION + 1);
        let unsupported_version = reseal_authorization_value(unsupported_version, &sealer)?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &unsupported_version,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );

        let mut cross_subject_payload: serde_json::Value = serde_json::from_slice(&bytes)?;
        cross_subject_payload["payload"]["plugin_id"] = serde_json::json!("plugin.other");
        let cross_subject_payload = reseal_authorization_value(cross_subject_payload, &sealer)?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &cross_subject_payload,
                &manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );

        let mut other_manifest = manifest.clone();
        other_manifest.identifier = "plugin.other".to_owned();
        other_manifest.signature.value = signing_key()?.sign_manifest(&other_manifest)?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &bytes,
                &other_manifest,
                &verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::AuthorizationManifestMismatch)
        );

        let other_verifier =
            PluginAuthorizationSealer::from_seed([8; 32], generation)?.verifier()?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &bytes,
                &manifest,
                &other_verifier,
                generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );
        let next_generation = PermissionPolicyGeneration::new(2)?;
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &bytes,
                &manifest,
                &verifier,
                next_generation,
                "profile-a",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );
        assert_eq!(
            PluginAuthorization::from_sealed_bytes(
                &bytes,
                &manifest,
                &verifier,
                generation,
                "profile-b",
            ),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );
        let next_sealer = PluginAuthorizationSealer::from_seed([7; 32], next_generation)?;
        assert_eq!(
            authorization.sealed_bytes(&next_sealer),
            Err(TrustError::InvalidSealedPluginAuthorization)
        );
        assert_eq!(
            PluginAuthorizationVerifier::from_token(&verifier.to_token())?,
            verifier
        );
        for malformed in [
            "",
            "1",
            "0:0000000000000000000000000000000000000000000000000000000000000000",
            "01:0000000000000000000000000000000000000000000000000000000000000000",
            "1:00",
            "1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            assert_eq!(
                PluginAuthorizationVerifier::from_token(malformed),
                Err(TrustError::InvalidAuthorizationVerificationKey)
            );
        }
        Ok(())
    }

    #[test]
    fn secrets_are_checked_opaque_and_debug_redacted() {
        for invalid in ["", " secret", "secret ", "secret\nidentifier"] {
            assert_eq!(SecretId::new(invalid), Err(TrustError::InvalidSecretId));
        }
        let secret = SecretValue::new(b"do-not-log".to_vec());
        assert_eq!(format!("{secret:?}"), "SecretValue([REDACTED])");
        assert!(!format!("{secret:?}").contains("do-not-log"));
        assert_eq!(secret.expose_to(<[u8]>::len), 10);
        for invalid_json in [
            serde_json::json!(" secret"),
            serde_json::json!("secret\nidentifier"),
            serde_json::json!("x".repeat(1_025)),
        ] {
            assert!(serde_json::from_value::<SecretId>(invalid_json).is_err());
        }
    }

    #[test]
    fn providers_default_closed_and_are_profile_and_grant_scoped()
    -> Result<(), Box<dyn std::error::Error>> {
        let secret_id = SecretId::new("provider-key")?;
        assert_eq!(
            ProviderPolicy::default().authorize(
                "profile-a",
                "plugin.fixture",
                "fixture",
                "https://fixture.invalid/v1/generate",
                Some(&secret_id)
            ),
            Err(TrustError::ProviderDisabled)
        );
        let offline = ProviderPolicy::new(
            "profile-a",
            ProviderMode::Offline,
            [ProviderEndpoint::new(
                "fixture",
                "https://fixture.invalid/v1/generate",
            )?],
            [CredentialScope::new(
                "profile-a",
                "plugin.fixture",
                "fixture",
                secret_id.clone(),
            )?],
        )?;
        assert_eq!(
            offline.authorize(
                "profile-a",
                "plugin.fixture",
                "fixture",
                "https://fixture.invalid/v1/generate",
                Some(&secret_id),
            ),
            Err(TrustError::Offline)
        );
        let enabled = ProviderPolicy::new(
            "profile-a",
            ProviderMode::Enabled,
            [ProviderEndpoint::new(
                "fixture",
                "https://fixture.invalid/v1/generate",
            )?],
            [CredentialScope::new(
                "profile-a",
                "plugin.fixture",
                "fixture",
                secret_id.clone(),
            )?],
        )?;
        assert_eq!(
            enabled.authorize(
                "profile-b",
                "plugin.fixture",
                "fixture",
                "https://fixture.invalid/v1/generate",
                Some(&secret_id),
            ),
            Err(TrustError::ProviderProfileMismatch)
        );
        assert_eq!(
            enabled.authorize(
                "profile-a",
                "plugin.fixture",
                "fixture",
                "https://fixture.invalid/v1/other",
                None,
            ),
            Err(TrustError::ProviderEndpointDenied)
        );
        assert_eq!(
            enabled
                .authorize(
                    "profile-a",
                    "plugin.fixture",
                    "fixture",
                    "https://fixture.invalid/v1/generate",
                    Some(&secret_id),
                )?
                .secret_id(),
            Some(&secret_id)
        );
        assert!(ProviderEndpoint::new("fixture", "http://127.0.0.1:8188/v1/generate").is_ok());
        for invalid_endpoint in [
            "/v1/generate",
            "http://provider.invalid/v1/generate",
            "https://user@provider.invalid/v1/generate",
            "https://provider.invalid/v1/generate?token=value",
            "https://provider.invalid/v1/generate#fragment",
            "https://PROVIDER.invalid/v1/generate",
            "https://provider.invalid:443/v1/generate",
        ] {
            assert_eq!(
                ProviderEndpoint::new("fixture", invalid_endpoint),
                Err(TrustError::InvalidProviderEndpoint)
            );
        }
        assert_eq!(
            enabled.authorize(
                "profile-a",
                "plugin.other",
                "fixture",
                "https://fixture.invalid/v1/generate",
                Some(&secret_id),
            ),
            Err(TrustError::MissingSecretGrant)
        );
        for invalid_identifier in ["Fixture", "fixture..provider", "-fixture", "fixture-"] {
            assert_eq!(
                ProviderId::new(invalid_identifier),
                Err(TrustError::InvalidProviderId)
            );
            assert_eq!(
                SecretId::new(invalid_identifier),
                Err(TrustError::InvalidSecretId)
            );
        }
        for invalid in [" fixture", "fixture ", "fixture\n", &"x".repeat(1_025)] {
            assert_eq!(
                ProviderEndpoint::new(invalid, "https://fixture.invalid/v1/generate"),
                Err(TrustError::InvalidProviderId)
            );
            assert_eq!(
                ProviderEndpoint::new("fixture", invalid),
                Err(TrustError::InvalidProviderEndpoint)
            );
            assert_eq!(
                ProviderPolicy::new(
                    invalid,
                    ProviderMode::Enabled,
                    std::iter::empty(),
                    std::iter::empty(),
                ),
                Err(TrustError::InvalidProviderProfile)
            );
        }
        Ok(())
    }

    #[test]
    fn provider_cost_acceptance_is_sealed_exact_bounded_and_expiring()
    -> Result<(), Box<dyn std::error::Error>> {
        let origin = Instant::now();
        let issuer = ProviderCostAcceptanceIssuer::from_seed([7; 32], origin)?;
        let verifier = issuer.verifier()?;
        let price = ProviderPriceBound::new("USD", 25_000)?;
        let scope_for = |prompt_sha256: &str,
                         attempt_id: &str,
                         node_id: &str,
                         request_ordinal: u32,
                         request_sha256: &str,
                         provider_binding_sha256: &str| {
            ProviderCostAcceptanceScope::new(
                "principal-a",
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000002",
                prompt_sha256,
                attempt_id,
                node_id,
                request_ordinal,
                request_sha256,
                "plugin.fixture",
                DIGEST,
                provider_binding_sha256,
                "fixture",
                "https://fixture.invalid/v1/generate",
                price.clone(),
            )
        };
        let prompt_sha256 = "c".repeat(64);
        let request_sha256 = "d".repeat(64);
        let binding_sha256 = "a".repeat(64);
        let scope = scope_for(
            &prompt_sha256,
            "00000000-0000-0000-0000-000000000003",
            "node.fixture",
            7,
            &request_sha256,
            &binding_sha256,
        )?;
        let nonce = ProviderCostNonce::new([9; 32])?;
        let acceptance = issuer.issue(
            scope.clone(),
            origin + Duration::from_secs(1),
            origin + Duration::from_secs(31),
            nonce,
        )?;

        verifier.verify(&acceptance, &scope, origin + Duration::from_secs(1))?;
        assert_eq!(acceptance.nonce(), nonce);
        assert_eq!(
            format!("{acceptance:?}"),
            "ProviderCostAcceptance([SEALED])"
        );
        assert_eq!(
            verifier.verify(&acceptance, &scope, origin + Duration::from_secs(31)),
            Err(TrustError::ExpiredProviderCostAcceptance)
        );
        for wrong_scope in [
            scope_for(
                &"e".repeat(64),
                scope.attempt_id(),
                scope.node_id(),
                scope.request_ordinal(),
                scope.request_sha256(),
                scope.provider_binding_sha256(),
            )?,
            scope_for(
                scope.prompt_sha256(),
                "00000000-0000-0000-0000-000000000004",
                scope.node_id(),
                scope.request_ordinal(),
                scope.request_sha256(),
                scope.provider_binding_sha256(),
            )?,
            scope_for(
                scope.prompt_sha256(),
                scope.attempt_id(),
                "node.other",
                scope.request_ordinal(),
                scope.request_sha256(),
                scope.provider_binding_sha256(),
            )?,
            scope_for(
                scope.prompt_sha256(),
                scope.attempt_id(),
                scope.node_id(),
                8,
                scope.request_sha256(),
                scope.provider_binding_sha256(),
            )?,
            scope_for(
                scope.prompt_sha256(),
                scope.attempt_id(),
                scope.node_id(),
                scope.request_ordinal(),
                &"f".repeat(64),
                scope.provider_binding_sha256(),
            )?,
            scope_for(
                scope.prompt_sha256(),
                scope.attempt_id(),
                scope.node_id(),
                scope.request_ordinal(),
                scope.request_sha256(),
                &"b".repeat(64),
            )?,
        ] {
            assert_eq!(
                verifier.verify(&acceptance, &wrong_scope, origin + Duration::from_secs(2)),
                Err(TrustError::InvalidProviderCostAcceptance)
            );
        }

        let mut forged = acceptance;
        forged.signature[0] ^= 1;
        assert_eq!(
            verifier.verify(&forged, &scope, origin + Duration::from_secs(2)),
            Err(TrustError::InvalidProviderCostAcceptance)
        );
        assert_eq!(
            ProviderCostNonce::new([0; 32]),
            Err(TrustError::InvalidProviderCostAcceptance)
        );
        assert_eq!(
            ProviderPriceBound::new("usd", 25_000),
            Err(TrustError::InvalidProviderPriceBound)
        );
        assert!(matches!(
            issuer.issue(
                scope,
                origin,
                origin + MAX_PROVIDER_COST_ACCEPTANCE_LIFETIME + Duration::from_millis(1),
                nonce,
            ),
            Err(TrustError::InvalidProviderCostAcceptance)
        ));
        Ok(())
    }

    #[test]
    fn unsafe_api_navigation_ffi_and_pickle_fail_before_effects()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            NativeApiExposure::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                false,
                std::iter::empty(),
                false,
                RemoteExposureApproval::LoopbackOnly,
            ),
            Err(TrustError::UnsafeApiExposure)
        );
        for unsafe_origin in [
            "https://",
            "http://localhost.evil.com",
            "https://example.com/path",
        ] {
            assert_eq!(
                NativeApiExposure::new(
                    IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    true,
                    [unsafe_origin.to_owned()],
                    false,
                    RemoteExposureApproval::Approved,
                ),
                Err(TrustError::UnsafeApiExposure)
            );
        }
        let exposure = NativeApiExposure::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            true,
            [
                "https://example.com".to_owned(),
                "http://localhost:3000".to_owned(),
            ],
            false,
            RemoteExposureApproval::Approved,
        )?;
        assert_eq!(
            exposure.allowed_origins(),
            &BTreeSet::from([
                "http://localhost:3000".to_owned(),
                "https://example.com".to_owned(),
            ])
        );
        let navigation = ExternalNavigationPolicy::new(["https".to_owned()], true)?;
        assert_eq!(
            navigation.authorize("javascript:alert(1)", true),
            Err(TrustError::NavigationDenied)
        );
        assert_eq!(
            navigation.authorize("https://example.com", false),
            Err(TrustError::NavigationDenied)
        );
        for malformed in ["https:", "https://", "https://user@example.com"] {
            assert_eq!(
                navigation.authorize(malformed, true),
                Err(TrustError::NavigationDenied)
            );
        }
        assert_eq!(
            ExternalNavigationPolicy::new(["javascript".to_owned()], true),
            Err(TrustError::InvalidNavigationPolicy)
        );

        let contract =
            NativeFfiContract::new("codec", DIGEST, "1", ["decode".to_owned()], "comfy_media")?;
        let registry = NativeFfiRegistry::new([contract])?;
        assert_eq!(
            registry.authorize("codec", DIGEST, "2", &BTreeSet::from(["decode".to_owned()])),
            Err(TrustError::UncertifiedFfi)
        );
        assert!(
            NativeFfiRegistry::default()
                .authorize("codec", DIGEST, "1", &BTreeSet::from(["decode".to_owned()]))
                .is_err()
        );

        let executable = b"cos\nsystem\n.";
        assert!(matches!(
            parse_restricted_pickle(executable, &ParserLimits::default()),
            Err(RestrictedPickleError::ForbiddenTarget { .. })
        ));
        assert!(IpAddr::from_str("127.0.0.1").map(|address| address.is_loopback())?);
        Ok(())
    }

    #[test]
    fn every_cataloged_provider_endpoint_requires_an_explicit_grant()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = include_str!(
            "../../../.agents/specs/comfy-parity/catalogs/backend-external-services.csv"
        );
        let policy = ProviderPolicy::new(
            "profile-a",
            ProviderMode::Enabled,
            std::iter::empty(),
            std::iter::empty(),
        )?;
        let mut count = 0;
        for line in catalog.lines().skip(1) {
            let mut columns = line.splitn(4, ',');
            let _method = columns.next().ok_or("method column")?;
            let endpoint = columns.next().ok_or("path column")?;
            let provider = columns.next().ok_or("provider column")?;
            let endpoint = format!("https://catalog.invalid{endpoint}");
            assert!(matches!(
                policy.authorize("profile-a", "plugin.fixture", provider, &endpoint, None,),
                Err(TrustError::ProviderEndpointDenied | TrustError::InvalidProviderEndpoint)
            ));
            count += 1;
        }
        assert_eq!(count, 217);
        Ok(())
    }

    fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("workspace root is unavailable")?
            .to_path_buf())
    }

    fn source_digest(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
    }

    fn write_trust_validation_artifact(
        root: &Path,
        cases: &BTreeMap<&str, bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let artifact_directory = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target"))
            .join("comfy-parity");
        let artifact_path = artifact_directory.join("val-runtime-trust-001.json");
        match fs::remove_file(&artifact_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let fixture_paths = [
            "crates/comfy_runtime/src/permissions.rs",
            "crates/comfy_runtime/src/trust.rs",
            "crates/comfy_runtime/src/native_ffi_rocm.rs",
            "crates/comfy_runtime/Cargo.toml",
            "crates/comfy_runtime/src/settings.rs",
            "crates/comfy_plugin_sdk/Cargo.toml",
            "crates/comfy_plugin_sdk/src/comfy_plugin_sdk.rs",
            "crates/comfy_plugin_host/Cargo.toml",
            "crates/comfy_worker/src/plugin_runtime.rs",
            "crates/settings_content/src/settings_content.rs",
            "crates/comfy_model/src/artifact_index.rs",
            "crates/comfy_model/src/formats.rs",
            "crates/comfy_model/src/restricted_pickle.rs",
            "crates/comfy_backend_rocm/src/loader.rs",
            "crates/comfy_backend_rocm/abi/symbols-v1.json",
            "crates/comfy_backend_rocm/abi/reviewed-bindings-v1.txt",
            "crates/comfy_backend_rocm/abi/verify-completion-evidence.sh",
            "crates/comfy_backend_rocm/build.rs",
            "script/package-comfy-backend-rocm",
            "nix/comfy-backends/rocm/package-policy.json",
            "crates/credentials_provider/src/credentials_provider.rs",
            ".agents/specs/comfy-parity/catalogs/backend-external-services.csv",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            ".agents/specs/comfy-parity/generate_ownership_catalog.py",
            ".agents/specs/comfy-parity/ownership-policy.json",
        ];
        let mut fixture_digests = BTreeMap::new();
        for relative_path in fixture_paths {
            fixture_digests.insert(relative_path, source_digest(&root.join(relative_path))?);
        }
        let failures = cases.values().filter(|passed| !**passed).count();
        let artifact = json!({
            "validation": "VAL-RUNTIME-TRUST-001",
            "task": "comfy-parity-trust-foundation",
            "environment": {
                "backend": "native-rust",
                "platform": std::env::consts::OS,
                "external_processes": 0,
                "network_requests": 0,
            },
            "authoritative_owners": {
                "authorization": "comfy_runtime::PermissionPolicy",
                "artifact_paths": "comfy_model::ArtifactRoot",
                "archive_model_parsing": "comfy_model::formats",
                "credentials": "credentials_provider::CredentialsProvider",
                "model_content_trust": "comfy_model::restricted_pickle",
                "plugin_verification": "comfy_runtime::PluginTrustPolicy",
            },
            "fixture_digests": fixture_digests,
            "cases": cases,
            "passes": cases.len().saturating_sub(failures),
            "failures": failures,
            "skips": 0,
        });
        fs::create_dir_all(&artifact_directory)?;
        let mut bytes = serde_json::to_vec_pretty(&artifact)?;
        bytes.push(b'\n');
        fs::write(artifact_path, bytes)?;
        Ok(())
    }

    #[test]
    fn val_runtime_trust_001() -> Result<(), Box<dyn std::error::Error>> {
        let root = workspace_root()?;
        let read_input = Capability::Asset {
            namespace: "input-root".to_owned(),
            action: crate::AssetOperation::Read,
        };
        let model = Capability::ModelHandle {
            model_id: "model.fixture".to_owned(),
        };
        let permissions = permission_policy(CapabilitySet::new([read_input.clone(), model]))?;
        let manifest = signed_plugin(vec![capability(CapabilityKind::Model, "model.fixture")])?;
        let authorization = trust_policy()?.authorize_manifest(&manifest, &permissions)?;

        let mut invalid_signature = manifest.clone();
        let replacement = if invalid_signature.signature.value.starts_with('0') {
            "1"
        } else {
            "0"
        };
        invalid_signature
            .signature
            .value
            .replace_range(..1, replacement);

        let policy_generation = permissions.generation();
        let authorization_sealer =
            PluginAuthorizationSealer::from_seed([9; 32], policy_generation)?;
        let authorization_verifier = authorization_sealer.verifier()?;
        let sealed_authorization = authorization.sealed_bytes(&authorization_sealer)?;

        let mut unknown_envelope: serde_json::Value =
            serde_json::from_slice(&sealed_authorization)?;
        unknown_envelope["future_authority"] = serde_json::json!(true);
        let unknown_envelope = serde_json::to_vec(&unknown_envelope)?;
        let mut unsupported_version: serde_json::Value =
            serde_json::from_slice(&sealed_authorization)?;
        unsupported_version["payload"]["version"] =
            serde_json::json!(SEALED_PLUGIN_AUTHORIZATION_VERSION + 1);
        let unsupported_version =
            reseal_authorization_value(unsupported_version, &authorization_sealer)?;
        let mut expanded_grants: serde_json::Value = serde_json::from_slice(&sealed_authorization)?;
        expanded_grants["payload"]["capabilities"]["capabilities"]
            .as_array_mut()
            .ok_or("sealed capability set is not an array")?
            .push(serde_json::to_value(Capability::Asset {
                namespace: "input".to_owned(),
                action: crate::AssetOperation::Read,
            })?);
        let expanded_grants = serde_json::to_vec(&expanded_grants)?;
        let mut cross_subject_manifest = manifest.clone();
        cross_subject_manifest.identifier = "plugin.other".to_owned();
        cross_subject_manifest.signature.value =
            signing_key()?.sign_manifest(&cross_subject_manifest)?;
        let next_generation = PermissionPolicyGeneration::new(policy_generation.get() + 1)?;

        let provider_secret = SecretId::new("provider-key")?;
        let enabled_provider = ProviderPolicy::new(
            "profile-a",
            ProviderMode::Enabled,
            [ProviderEndpoint::new(
                "fixture",
                "https://fixture.invalid/v1/generate",
            )?],
            [CredentialScope::new(
                "profile-a",
                "plugin.fixture",
                "fixture",
                provider_secret.clone(),
            )?],
        )?;

        let navigation = ExternalNavigationPolicy::new(["https".to_owned()], true)?;
        let ffi_contract =
            NativeFfiContract::new("codec", DIGEST, "1", ["decode".to_owned()], "comfy_media")?;
        let ffi_registry = NativeFfiRegistry::new([ffi_contract])?;

        let artifact_directory = tempfile::tempdir()?;
        let artifact_root = comfy_model::ArtifactRoot::canonical(
            "models",
            "checkpoints",
            artifact_directory.path(),
            ["safetensors"],
        )?;

        let runtime_trust_source =
            fs::read_to_string(root.join("crates/comfy_runtime/src/trust.rs"))?;
        let runtime_trust_production = runtime_trust_source
            .split_once("#[cfg(test)]")
            .map(|(production, _)| production)
            .ok_or("trust test boundary is unavailable")?;
        let runtime_rocm_source =
            fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_rocm.rs"))?;
        let backend_rocm_loader =
            fs::read_to_string(root.join("crates/comfy_backend_rocm/src/loader.rs"))?;
        let backend_rocm_packager =
            fs::read_to_string(root.join("script/package-comfy-backend-rocm"))?;
        let backend_rocm_package_policy =
            fs::read_to_string(root.join("nix/comfy-backends/rocm/package-policy.json"))?;
        let signature_assertion_marker = ["signature", "verified"].join("_");
        let runtime_permissions_source =
            fs::read_to_string(root.join("crates/comfy_runtime/src/permissions.rs"))?;
        let runtime_settings_source =
            fs::read_to_string(root.join("crates/comfy_runtime/src/settings.rs"))?;
        let settings_content_source =
            fs::read_to_string(root.join("crates/settings_content/src/settings_content.rs"))?;
        let sdk_manifest_source =
            fs::read_to_string(root.join("crates/comfy_plugin_sdk/Cargo.toml"))?;
        let sdk_source =
            fs::read_to_string(root.join("crates/comfy_plugin_sdk/src/comfy_plugin_sdk.rs"))?;
        let runtime_manifest_source =
            fs::read_to_string(root.join("crates/comfy_runtime/Cargo.toml"))?;
        let host_manifest_source =
            fs::read_to_string(root.join("crates/comfy_plugin_host/Cargo.toml"))?;
        let runtime_root_source =
            fs::read_to_string(root.join("crates/comfy_runtime/src/comfy_runtime.rs"))?;
        let artifact_source =
            fs::read_to_string(root.join("crates/comfy_model/src/artifact_index.rs"))?;
        let model_formats_source =
            fs::read_to_string(root.join("crates/comfy_model/src/formats.rs"))?;
        let credentials_source = fs::read_to_string(
            root.join("crates/credentials_provider/src/credentials_provider.rs"),
        )?;
        let ownership_catalog = fs::read_to_string(
            root.join(".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv"),
        )?;

        let cases = BTreeMap::from([
            (
                "authorization_envelope_rejects_unknown_version_and_truncation",
                PluginAuthorization::from_sealed_bytes(
                    &unknown_envelope,
                    &manifest,
                    &authorization_verifier,
                    policy_generation,
                    "profile-a",
                ) == Err(TrustError::InvalidSealedPluginAuthorization)
                    && PluginAuthorization::from_sealed_bytes(
                        &unsupported_version,
                        &manifest,
                        &authorization_verifier,
                        policy_generation,
                        "profile-a",
                    ) == Err(TrustError::InvalidSealedPluginAuthorization)
                    && PluginAuthorization::from_sealed_bytes(
                        sealed_authorization
                            .get(..sealed_authorization.len().saturating_sub(1))
                            .ok_or("sealed authorization bytes")?,
                        &manifest,
                        &authorization_verifier,
                        policy_generation,
                        "profile-a",
                    ) == Err(TrustError::InvalidSealedPluginAuthorization),
            ),
            (
                "authorization_is_bound_to_subject_profile_generation_and_exact_grants",
                PluginAuthorization::from_sealed_bytes(
                    &sealed_authorization,
                    &cross_subject_manifest,
                    &authorization_verifier,
                    policy_generation,
                    "profile-a",
                ) == Err(TrustError::AuthorizationManifestMismatch)
                    && PluginAuthorization::from_sealed_bytes(
                        &sealed_authorization,
                        &manifest,
                        &authorization_verifier,
                        policy_generation,
                        "profile-b",
                    ) == Err(TrustError::InvalidSealedPluginAuthorization)
                    && PluginAuthorization::from_sealed_bytes(
                        &sealed_authorization,
                        &manifest,
                        &authorization_verifier,
                        next_generation,
                        "profile-a",
                    ) == Err(TrustError::InvalidSealedPluginAuthorization)
                    && PluginAuthorization::from_sealed_bytes(
                        &expanded_grants,
                        &manifest,
                        &authorization_verifier,
                        policy_generation,
                        "profile-a",
                    ) == Err(TrustError::InvalidSealedPluginAuthorization),
            ),
            (
                "authorization_verifier_token_is_canonical_and_generation_bound",
                PluginAuthorizationVerifier::from_token(&authorization_verifier.to_token())
                    .is_ok_and(|parsed| parsed == authorization_verifier)
                    && PluginAuthorizationVerifier::from_token(
                        "01:0000000000000000000000000000000000000000000000000000000000000000",
                    ) == Err(TrustError::InvalidAuthorizationVerificationKey)
                    && PluginAuthorizationVerifier::from_token(
                        "0:0000000000000000000000000000000000000000000000000000000000000000",
                    ) == Err(TrustError::InvalidAuthorizationVerificationKey),
            ),
            (
                "artifact_root_is_sole_generic_path_validator",
                artifact_root
                    .resolve_for_create("nested/model.safetensors")
                    .is_ok()
                    && artifact_root.resolve_for_create("../escape").is_err()
                    && artifact_source.contains("pub fn resolve_existing")
                    && artifact_source.contains("pub fn resolve_for_create")
                    && !runtime_root_source.contains("secure_paths")
                    && ownership_catalog.contains(
                        "secure_root_path_validation,comfy_model::ArtifactRoot",
                    ),
            ),
            (
                "archive_validation_has_a_focused_model_owner",
                model_formats_source.contains("fn canonical_archive_path")
                    && model_formats_source.contains("ArchiveEntry")
                    && ownership_catalog
                        .contains("model_archive_content_validation,comfy_model::formats"),
            ),
            (
                "canonical_model_pickle_policy_rejects_execution",
                matches!(
                    parse_restricted_pickle(b"cos\nsystem\n.", &ParserLimits::default()),
                    Err(RestrictedPickleError::ForbiddenTarget { .. })
                ) && !runtime_trust_production.contains("ALLOWED_PICKLE_GLOBALS"),
            ),
            (
                "credentials_remain_identifier_only_at_policy_boundary",
                credentials_source.contains("pub trait CredentialsProvider")
                    && runtime_trust_source.contains("pub struct SecretId")
                    && !runtime_trust_source.contains("Serialize)]\npub struct SecretValue")
                    && ownership_catalog.contains(
                        "credential_secret_storage,credentials_provider::CredentialsProvider",
                    ),
            ),
            (
                "ffi_certification_is_registry_issued",
                ffi_registry
                    .authorize("codec", DIGEST, "1", &BTreeSet::from(["decode".to_owned()]))
                    .is_ok()
                    && NativeFfiRegistry::default()
                        .authorize("codec", DIGEST, "1", &BTreeSet::from(["decode".to_owned()]))
                        .is_err()
                    && !runtime_trust_production.contains("certified: bool")
                    && ownership_catalog
                        .contains("native_ffi_certification,comfy_runtime::NativeFfiRegistry"),
            ),
            (
                "rocm_ffi_adapter_consumes_registry_authority_without_self_certification",
                runtime_rocm_source.contains("cancellation: &CancellationToken")
                    && runtime_rocm_source
                        .contains("elf64_dynamic_contract(image.bytes(), cancellation)")
                    && runtime_rocm_source.contains("exactly one PT_DYNAMIC segment")
                    && runtime_rocm_source.contains("TRUSTED_SYSTEM_ELF_DEPENDENCIES")
                    && runtime_rocm_source.contains("CancellableChunks")
                    && runtime_rocm_source
                        .contains("exact_soname_and_system_only_dependencies_prevent_ambient_binding")
                    && runtime_rocm_source
                        .contains("cancellation_is_injected_during_read_hash_parse_and_snapshot")
                    && runtime_rocm_source.contains("let certificate = registry")
                    && runtime_rocm_source.contains(".authorize(")
                    && runtime_rocm_source.contains(".seal(")
                    && runtime_rocm_source
                        .split_once("#[cfg(test)]\nmod tests")
                        .map_or(runtime_rocm_source.as_str(), |(production, _)| production)
                        .matches("NativeFfiContract::new")
                        .count()
                        == 1
                    && backend_rocm_loader.contains("validate_sealed_memfd")
                    && backend_rocm_loader.contains("F_GET_SEALS")
                    && backend_rocm_loader.contains("validate_signed_package")
                    && backend_rocm_loader.contains("validate_tree_membership")
                    && backend_rocm_loader.contains("validate_coverage")
                    && backend_rocm_loader.contains("compiled reviewed policy")
                    && !backend_rocm_loader.contains("NativeFfiRegistry::")
                    && backend_rocm_package_policy
                        .contains("comfy_runtime-native-rust-ed25519")
                    && !backend_rocm_packager.contains("COMFY_ROCM_SIGNATURE_VERIFIER")
                    && backend_rocm_packager.contains("package-coverage.sha256"),
            ),
            (
                "rocm_package_trust_is_native_domain_bound_and_plugin_separate",
                runtime_trust_source.contains("pub struct RocmPackageVerificationKey")
                    && runtime_trust_source.contains("ROCM_PACKAGE_SIGNATURE_DOMAIN")
                    && runtime_trust_source.contains("canonical_receipt.push(b'\\n')")
                    && runtime_rocm_source.contains("NativeRocmPackageVerifier")
                    && runtime_rocm_source
                        .find("verify_signed_package_root")
                        .zip(runtime_rocm_source.find("parse_rocm_ffi_contract_catalog"))
                        .is_some_and(|(verification, catalog)| verification < catalog)
                    && runtime_settings_source.contains("project_rocm_package_settings")
                    && runtime_settings_source.contains("RocmPackageVerificationKey::new")
                    && runtime_settings_source.contains("is_private_signing_setting")
                    && ownership_catalog.contains(
                        "rocm_package_trust_and_contract_mapping,comfy_runtime::RocmPackageVerificationKey",
                    ),
            ),
            (
                "navigation_requires_scheme_and_user_gesture",
                navigation.authorize("https://example.com", true).is_ok()
                    && navigation.authorize("https://example.com", false).is_err()
                    && navigation.authorize("javascript:alert(1)", true).is_err()
                    && navigation.authorize("https:", true).is_err()
                    && navigation.authorize("https://", true).is_err()
                    && ownership_catalog.contains(
                        "external_navigation_authorization,comfy_runtime::ExternalNavigationPolicy",
                    ),
            ),
            (
                "permission_authorization_is_exact_and_sealed",
                authorization.capabilities().capabilities().len() == 1
                    && authorization
                        .capabilities()
                        .require(&read_input)
                        .is_err()
                    && runtime_permissions_source.contains("pub struct PermissionPolicy")
                    && runtime_permissions_source.contains("pub struct AuthorizedCapabilities")
                    && ownership_catalog.contains(
                        "permission_capability_domain,comfy_runtime::PermissionPolicy and Capability",
                    ),
            ),
            (
                "plugin_signing_tooling_is_feature_gated_and_private_seeds_are_zeroized",
                sdk_manifest_source.contains("default = []")
                    && sdk_manifest_source
                        .contains("signing-tooling = [\"dep:ring\", \"dep:zeroize\"]")
                    && sdk_manifest_source
                        .contains("ring = { workspace = true, optional = true }")
                    && sdk_source.contains(
                        "#[cfg(any(feature = \"signing-tooling\", test))]\npub struct PluginSigningKey",
                    )
                    && sdk_source.contains("seed: Zeroizing<[")
                    && runtime_trust_source.contains("seed: Zeroizing<[")
                    && runtime_trust_source.contains("pub struct SecretValue(Zeroizing<Vec<u8>>)")
                    && runtime_manifest_source
                        .split_once("[dev-dependencies]")
                        .is_some_and(|(_, dependencies)| {
                            dependencies.contains(
                                "comfy_plugin_sdk = { workspace = true, features = [\"signing-tooling\"] }",
                            )
                        })
                    && host_manifest_source
                        .split_once("[dev-dependencies]")
                        .is_some_and(|(_, dependencies)| {
                            dependencies.contains(
                                "comfy_plugin_sdk = { workspace = true, features = [\"signing-tooling\"] }",
                            )
                        }),
            ),
            (
                "plugin_signature_is_verified_not_asserted",
                trust_policy()?.authorize_manifest(&invalid_signature, &permissions)
                    == Err(TrustError::InvalidPluginSignature)
                    && !runtime_trust_production.contains(&signature_assertion_marker)
                    && ownership_catalog.contains(
                        "plugin_signature_verification,comfy_runtime::PluginTrustPolicy",
                    ),
            ),
            (
                "provider_policy_defaults_closed_and_is_profile_scoped",
                ProviderPolicy::default().authorize(
                    "profile-a",
                    "plugin.fixture",
                    "fixture",
                    "https://fixture.invalid/v1/generate",
                    None,
                )
                    == Err(TrustError::ProviderDisabled)
                    && enabled_provider.authorize(
                        "profile-b",
                        "plugin.fixture",
                        "fixture",
                        "https://fixture.invalid/v1/generate",
                        Some(&provider_secret),
                    ) == Err(TrustError::ProviderProfileMismatch)
                    && enabled_provider.authorize(
                        "profile-a",
                        "plugin.other",
                        "fixture",
                        "https://fixture.invalid/v1/generate",
                        Some(&provider_secret),
                    ) == Err(TrustError::MissingSecretGrant)
                    && ProviderEndpoint::new(
                        "fixture",
                        "http://provider.invalid/v1/generate",
                    ) == Err(TrustError::InvalidProviderEndpoint)
                    && ProviderEndpoint::new(
                        "fixture",
                        "https://PROVIDER.invalid/v1/generate",
                    ) == Err(TrustError::InvalidProviderEndpoint)
                    && ownership_catalog.contains(
                        "provider_request_authorization,comfy_runtime::ProviderPolicy",
                    ),
            ),
            (
                "remote_api_exposure_fails_closed",
                NativeApiExposure::new(
                    IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    false,
                    std::iter::empty(),
                    false,
                    RemoteExposureApproval::LoopbackOnly,
                ) == Err(TrustError::UnsafeApiExposure)
                    && NativeApiExposure::new(
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        true,
                        ["http://localhost.evil.com".to_owned()],
                        false,
                        RemoteExposureApproval::Approved,
                    ) == Err(TrustError::UnsafeApiExposure)
                    && NativeApiExposure::new(
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        true,
                        ["https://example.com".to_owned()],
                        false,
                        RemoteExposureApproval::Approved,
                    )
                    .is_ok(),
            ),
            (
                "settings_store_public_verifiers_and_reject_private_or_global_secret_authority",
                settings_content_source.contains("pub public_key_hex: Option<String>")
                    && !settings_content_source.contains("pub private_key")
                    && !settings_content_source.contains("pub signing_key")
                    && !settings_content_source.contains("pub seed: Option")
                    && runtime_settings_source.contains(
                        "plugin security settings must not contain private signing material",
                    )
                    && runtime_settings_source.contains(
                        "legacy provider_secret_ids cannot grant credential authority",
                    ),
            ),
            (
                "secret_values_are_nonserializable_and_redacted",
                format!("{:?}", SecretValue::new(b"do-not-log".to_vec()))
                    == "SecretValue([REDACTED])"
                    && SecretId::new("secret\nidentifier").is_err()
                    && !runtime_trust_source
                        .contains("#[derive(Serialize, Deserialize)]\npub struct SecretValue"),
            ),
        ]);
        assert!(cases.values().all(|passed| *passed), "{cases:#?}");
        write_trust_validation_artifact(&root, &cases)?;
        Ok(())
    }
}
