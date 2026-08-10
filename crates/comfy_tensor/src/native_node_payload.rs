#[cfg(feature = "cpu")]
use crate::ImageTensor;
use crate::{DType, DeviceId, Tensor, TensorDescriptor, TensorError, ViewAccess};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const NATIVE_TENSOR_PROJECTION_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTensorRole {
    Image,
    Mask,
    Conditioning,
    Latent,
    Sigmas,
    WanCameraEmbedding,
}

impl NativeTensorRole {
    pub const fn handle_type_id(self) -> &'static str {
        match self {
            Self::Image => "IMAGE",
            Self::Mask => "MASK",
            Self::Conditioning => "CONDITIONING",
            Self::Latent => "LATENT",
            Self::Sigmas => "SIGMAS",
            Self::WanCameraEmbedding => "WAN_CAMERA_EMBEDDING",
        }
    }

    const fn digest_tag(self) -> u8 {
        match self {
            Self::Image => 1,
            Self::Mask => 2,
            Self::Conditioning => 3,
            Self::Latent => 4,
            Self::Sigmas => 5,
            Self::WanCameraEmbedding => 6,
        }
    }

    const fn accepts_image_tensor(self) -> bool {
        matches!(self, Self::Image)
    }

    const fn accepts_raw_tensor(self) -> bool {
        matches!(
            self,
            Self::Mask
                | Self::Conditioning
                | Self::Latent
                | Self::Sigmas
                | Self::WanCameraEmbedding
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NativeTensorProjection {
    schema_version: u16,
    role: NativeTensorRole,
    descriptor: TensorDescriptor,
    content_digest: String,
    resident_bytes: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeTensorProjectionWire {
    schema_version: u16,
    role: NativeTensorRole,
    descriptor: TensorDescriptor,
    content_digest: String,
    resident_bytes: u64,
}

impl NativeTensorProjection {
    fn new(
        role: NativeTensorRole,
        descriptor: TensorDescriptor,
        content_digest: String,
        resident_bytes: u64,
    ) -> Result<Self, NativeTensorPayloadError> {
        let projection = Self {
            schema_version: NATIVE_TENSOR_PROJECTION_SCHEMA_VERSION,
            role,
            descriptor,
            content_digest,
            resident_bytes,
        };
        projection.validate()?;
        Ok(projection)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn role(&self) -> NativeTensorRole {
        self.role
    }

    pub fn descriptor(&self) -> &TensorDescriptor {
        &self.descriptor
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeTensorPayloadError> {
        if self.schema_version != NATIVE_TENSOR_PROJECTION_SCHEMA_VERSION {
            return Err(NativeTensorPayloadError::UnsupportedSchemaVersion {
                actual: self.schema_version,
            });
        }
        if self.content_digest.len() != 64
            || !self
                .content_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(NativeTensorPayloadError::InvalidContentDigest);
        }
        if !self.descriptor.is_contiguous()? {
            return Err(NativeTensorPayloadError::NonContiguousDescriptor);
        }
        validate_role_descriptor(self.role, &self.descriptor)?;
        let minimum = self.descriptor.minimum_backing_byte_length()?;
        if self.resident_bytes < minimum {
            return Err(NativeTensorPayloadError::ResidentBytesTooSmall {
                minimum,
                actual: self.resident_bytes,
            });
        }
        Ok(())
    }
}

impl TryFrom<NativeTensorProjectionWire> for NativeTensorProjection {
    type Error = NativeTensorPayloadError;

    fn try_from(wire: NativeTensorProjectionWire) -> Result<Self, Self::Error> {
        let projection = Self {
            schema_version: wire.schema_version,
            role: wire.role,
            descriptor: wire.descriptor,
            content_digest: wire.content_digest,
            resident_bytes: wire.resident_bytes,
        };
        projection.validate()?;
        Ok(projection)
    }
}

impl<'de> Deserialize<'de> for NativeTensorProjection {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let wire = NativeTensorProjectionWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug)]
enum NativeTensorStorage {
    #[cfg(feature = "cpu")]
    Image(ImageTensor),
    Tensor(Tensor),
}

#[derive(Clone, Debug)]
pub struct NativeTensorPayload {
    projection: NativeTensorProjection,
    storage: NativeTensorStorage,
}

impl NativeTensorPayload {
    #[cfg(feature = "cpu")]
    pub fn from_image(
        role: NativeTensorRole,
        image: ImageTensor,
    ) -> Result<Self, NativeTensorPayloadError> {
        if role == NativeTensorRole::Mask {
            let (batch, height, width, channels) = image.dimensions()?;
            if channels != 1 {
                return Err(NativeTensorPayloadError::RoleDescriptorMismatch { role });
            }
            let descriptor = image
                .tensor()
                .descriptor()
                .reshaped_view(vec![batch, height, width])?;
            let tensor = image.tensor().view(descriptor, ViewAccess::ReadOnly)?;
            let projection = project_tensor(role, &tensor)?;
            return Ok(Self {
                projection,
                storage: NativeTensorStorage::Tensor(tensor),
            });
        }
        if !role.accepts_image_tensor() {
            return Err(NativeTensorPayloadError::RoleStorageMismatch {
                role,
                storage: "ImageTensor",
            });
        }
        let projection = project_tensor(role, image.tensor())?;
        Ok(Self {
            projection,
            storage: NativeTensorStorage::Image(image),
        })
    }

    pub fn from_tensor(
        role: NativeTensorRole,
        tensor: Tensor,
    ) -> Result<Self, NativeTensorPayloadError> {
        if !role.accepts_raw_tensor() {
            return Err(NativeTensorPayloadError::RoleStorageMismatch {
                role,
                storage: "Tensor",
            });
        }
        let projection = project_tensor(role, &tensor)?;
        Ok(Self {
            projection,
            storage: NativeTensorStorage::Tensor(tensor),
        })
    }

    pub fn projection(&self) -> &NativeTensorProjection {
        &self.projection
    }

    pub const fn role(&self) -> NativeTensorRole {
        self.projection.role
    }

    pub fn handle_type_id(&self) -> &'static str {
        self.role().handle_type_id()
    }

    pub fn tensor(&self) -> &Tensor {
        match &self.storage {
            #[cfg(feature = "cpu")]
            NativeTensorStorage::Image(image) => image.tensor(),
            NativeTensorStorage::Tensor(tensor) => tensor,
        }
    }

    #[cfg(feature = "cpu")]
    pub fn image(&self) -> Option<&ImageTensor> {
        match &self.storage {
            NativeTensorStorage::Image(image) => Some(image),
            NativeTensorStorage::Tensor(_) => None,
        }
    }

    pub fn validate(&self) -> Result<(), NativeTensorPayloadError> {
        self.projection.validate()?;
        match &self.storage {
            #[cfg(feature = "cpu")]
            NativeTensorStorage::Image(_) if !self.role().accepts_image_tensor() => {
                return Err(NativeTensorPayloadError::RoleStorageMismatch {
                    role: self.role(),
                    storage: "ImageTensor",
                });
            }
            NativeTensorStorage::Tensor(_) if !self.role().accepts_raw_tensor() => {
                return Err(NativeTensorPayloadError::RoleStorageMismatch {
                    role: self.role(),
                    storage: "Tensor",
                });
            }
            _ => {}
        }
        let actual = project_tensor(self.role(), self.tensor())?;
        if actual != self.projection {
            return Err(NativeTensorPayloadError::ProjectionChanged);
        }
        Ok(())
    }
}

pub fn native_tensor_digest(
    role: NativeTensorRole,
    descriptor: &TensorDescriptor,
    contiguous_bytes: &[u8],
) -> Result<String, NativeTensorPayloadError> {
    if !descriptor.is_contiguous()? {
        return Err(NativeTensorPayloadError::NonContiguousDescriptor);
    }
    let expected = descriptor.byte_len()?;
    let actual = u64::try_from(contiguous_bytes.len()).map_err(|_| TensorError::ShapeOverflow)?;
    if actual != expected {
        return Err(NativeTensorPayloadError::ContentByteLength { expected, actual });
    }
    let mut hasher = Sha256::new();
    hasher.update(b"sim.comfy.native-tensor.semantic.v1");
    hasher.update([role.digest_tag()]);
    hasher.update(serde_json::to_vec(&descriptor.dtype())?);
    hasher.update(
        u64::try_from(descriptor.rank())
            .map_err(|_| TensorError::ShapeOverflow)?
            .to_le_bytes(),
    );
    for dimension in descriptor.shape() {
        hasher.update(dimension.to_le_bytes());
    }
    hasher.update(contiguous_bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn project_tensor(
    role: NativeTensorRole,
    tensor: &Tensor,
) -> Result<NativeTensorProjection, NativeTensorPayloadError> {
    let descriptor = tensor.descriptor().clone();
    let content_digest = native_tensor_digest(role, &descriptor, tensor.contiguous_bytes()?)?;
    NativeTensorProjection::new(role, descriptor, content_digest, tensor.storage_byte_len())
}

fn validate_role_descriptor(
    role: NativeTensorRole,
    descriptor: &TensorDescriptor,
) -> Result<(), NativeTensorPayloadError> {
    if descriptor.device() != DeviceId::CPU || descriptor.dtype() != DType::F32 {
        return Err(NativeTensorPayloadError::RoleDescriptorMismatch { role });
    }
    let shape = descriptor.shape();
    let valid = match role {
        NativeTensorRole::Image => {
            shape.len() == 4
                && shape
                    .get(3)
                    .is_some_and(|channels| matches!(*channels, 1 | 3 | 4))
        }
        NativeTensorRole::Mask => shape.len() == 3,
        NativeTensorRole::Conditioning => shape.len() == 3,
        NativeTensorRole::Latent => shape.len() == 4,
        NativeTensorRole::Sigmas => shape.len() == 1,
        NativeTensorRole::WanCameraEmbedding => !shape.is_empty(),
    };
    if valid {
        Ok(())
    } else {
        Err(NativeTensorPayloadError::RoleDescriptorMismatch { role })
    }
}

#[derive(Debug, Error)]
pub enum NativeTensorPayloadError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error("native tensor descriptor encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("native tensor projection schema version {actual} is unsupported")]
    UnsupportedSchemaVersion { actual: u16 },
    #[error("native tensor projection content digest must be 64 lowercase hexadecimal characters")]
    InvalidContentDigest,
    #[error("native tensor projection descriptor must be contiguous")]
    NonContiguousDescriptor,
    #[error("native tensor projection requires at least {minimum} resident bytes, got {actual}")]
    ResidentBytesTooSmall { minimum: u64, actual: u64 },
    #[error("native tensor content requires {expected} bytes, got {actual}")]
    ContentByteLength { expected: u64, actual: u64 },
    #[error("native tensor role {role:?} cannot contain {storage} storage")]
    RoleStorageMismatch {
        role: NativeTensorRole,
        storage: &'static str,
    },
    #[error("native tensor descriptor does not match role {role:?}")]
    RoleDescriptorMismatch { role: NativeTensorRole },
    #[error("native tensor projection no longer matches its compute payload")]
    ProjectionChanged,
}

#[cfg(all(test, feature = "cpu"))]
mod tests {
    use super::*;
    use crate::{
        CancellationToken, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext, ImageTensor,
        StreamId, TensorDescriptor,
    };
    use serde_json::json;
    use std::error::Error;

    const TEST_MEMORY_LIMIT_BYTES: u64 = 1024 * 1024;

    fn with_context<ResultType>(
        run: impl FnOnce(
            &crate::CpuBackend,
            &ExecutionContext<'_>,
        ) -> Result<ResultType, Box<dyn Error>>,
    ) -> Result<ResultType, Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(TEST_MEMORY_LIMIT_BYTES)?;
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(0)?,
            &cancellation,
        );
        run(&backend, &context)
    }

    fn tensor_payload(
        role: NativeTensorRole,
        values: &[f32],
    ) -> Result<NativeTensorPayload, Box<dyn Error>> {
        with_context(|backend, context| {
            let value_count = u64::try_from(values.len())?;
            let shape = match role {
                NativeTensorRole::Conditioning => vec![1, 1, value_count],
                NativeTensorRole::Latent => vec![1, 1, 1, value_count],
                NativeTensorRole::Sigmas => vec![value_count],
                NativeTensorRole::WanCameraEmbedding => vec![1, value_count],
                NativeTensorRole::Image | NativeTensorRole::Mask => vec![1, value_count],
            };
            let descriptor =
                TensorDescriptor::contiguous(shape, DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
            let (tensor, _) = backend.upload_f32(descriptor, values, context)?;
            Ok(NativeTensorPayload::from_tensor(role, tensor)?)
        })
    }

    #[test]
    fn roles_preserve_source_socket_type_and_digest_identity() -> Result<(), Box<dyn Error>> {
        let expected = [
            (NativeTensorRole::Image, "IMAGE"),
            (NativeTensorRole::Mask, "MASK"),
            (NativeTensorRole::Conditioning, "CONDITIONING"),
            (NativeTensorRole::Latent, "LATENT"),
        ];
        for (role, type_id) in expected {
            assert_eq!(role.handle_type_id(), type_id);
        }

        let conditioning = tensor_payload(NativeTensorRole::Conditioning, &[1.0, -2.0])?;
        let latent = tensor_payload(NativeTensorRole::Latent, &[1.0, -2.0])?;
        assert_ne!(
            conditioning.projection().content_digest(),
            latent.projection().content_digest()
        );
        assert_eq!(
            serde_json::to_string(&NativeTensorRole::Conditioning)?,
            "\"conditioning\""
        );
        Ok(())
    }

    #[test]
    fn semantic_digest_ignores_stream_placement() -> Result<(), Box<dyn Error>> {
        let default_stream =
            TensorDescriptor::contiguous(vec![1, 2], DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
        let other_stream =
            TensorDescriptor::contiguous(vec![1, 2], DType::F32, DeviceId::CPU, StreamId::new(7))?;
        let bytes = [1.0_f32, -2.0_f32]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();

        assert_eq!(
            native_tensor_digest(NativeTensorRole::Latent, &default_stream, &bytes)?,
            native_tensor_digest(NativeTensorRole::Latent, &other_stream, &bytes)?
        );
        assert_ne!(
            native_tensor_digest(NativeTensorRole::Latent, &default_stream, &bytes)?,
            native_tensor_digest(NativeTensorRole::Sigmas, &default_stream, &bytes)?
        );
        Ok(())
    }

    #[test]
    fn tensor_payload_projects_real_storage_and_revalidates() -> Result<(), Box<dyn Error>> {
        let payload = tensor_payload(NativeTensorRole::Latent, &[0.25, 0.5, 0.75])?;
        assert_eq!(payload.projection().schema_version(), 1);
        assert_eq!(payload.projection().role(), NativeTensorRole::Latent);
        assert_eq!(payload.handle_type_id(), "LATENT");
        assert_eq!(
            payload.projection().descriptor(),
            payload.tensor().descriptor()
        );
        assert_eq!(
            payload.projection().resident_bytes(),
            payload.tensor().storage_byte_len()
        );
        assert_eq!(
            payload.projection().content_digest(),
            native_tensor_digest(
                NativeTensorRole::Latent,
                payload.tensor().descriptor(),
                payload.tensor().contiguous_bytes()?,
            )?
        );
        payload.validate()?;
        assert!(payload.image().is_none());
        Ok(())
    }

    #[test]
    fn image_and_mask_payloads_preserve_source_exact_bhwc_and_bhw_shapes()
    -> Result<(), Box<dyn Error>> {
        with_context(|backend, context| {
            let image = ImageTensor::from_f32(
                backend,
                context,
                1,
                1,
                2,
                3,
                &[0.0, 0.25, 0.5, 0.75, 1.0, 0.125],
            )?;
            let payload = NativeTensorPayload::from_image(NativeTensorRole::Image, image)?;
            assert_eq!(
                payload
                    .image()
                    .ok_or("image storage is missing")?
                    .dimensions()?,
                (1, 1, 2, 3)
            );
            payload.validate()?;

            let mask = ImageTensor::from_f32(backend, context, 1, 1, 2, 1, &[0.0, 1.0])?;
            let payload = NativeTensorPayload::from_image(NativeTensorRole::Mask, mask)?;
            assert_eq!(payload.handle_type_id(), "MASK");
            assert_eq!(payload.tensor().descriptor().shape(), &[1, 1, 2]);
            assert!(payload.image().is_none());
            payload.validate()?;
            Ok(())
        })
    }

    #[test]
    fn role_and_storage_mismatches_fail_closed() -> Result<(), Box<dyn Error>> {
        let latent_as_image = with_context(|backend, context| {
            let image = ImageTensor::from_f32(backend, context, 1, 1, 1, 1, &[0.0])?;
            Ok(NativeTensorPayload::from_image(
                NativeTensorRole::Latent,
                image,
            ))
        })?;
        assert!(matches!(
            latent_as_image,
            Err(NativeTensorPayloadError::RoleStorageMismatch {
                role: NativeTensorRole::Latent,
                storage: "ImageTensor",
            })
        ));

        let image_as_tensor = tensor_payload(NativeTensorRole::Image, &[0.0]);
        assert!(matches!(
            image_as_tensor,
            Err(error)
                if error.downcast_ref::<NativeTensorPayloadError>().is_some_and(|error| matches!(
                    error,
                    NativeTensorPayloadError::RoleStorageMismatch {
                        role: NativeTensorRole::Image,
                        storage: "Tensor",
                    }
                ))
        ));
        Ok(())
    }

    #[test]
    fn digest_rejects_wrong_byte_length() -> Result<(), Box<dyn Error>> {
        let descriptor =
            TensorDescriptor::contiguous(vec![2], DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
        assert_eq!(
            native_tensor_digest(NativeTensorRole::Latent, &descriptor, &[0; 8])?,
            "91c18e0daf9452e16386af89697cc7043c1e8cdf086adb6b5d272e31cb4fdf4d"
        );
        assert!(matches!(
            native_tensor_digest(NativeTensorRole::Latent, &descriptor, &[0; 4]),
            Err(NativeTensorPayloadError::ContentByteLength {
                expected: 8,
                actual: 4,
            })
        ));
        Ok(())
    }

    #[test]
    fn projection_deserialization_is_bounded_and_validated() -> Result<(), Box<dyn Error>> {
        let payload = tensor_payload(NativeTensorRole::Conditioning, &[1.0])?;
        let encoded = serde_json::to_value(payload.projection())?;
        let decoded: NativeTensorProjection = serde_json::from_value(encoded.clone())?;
        assert_eq!(&decoded, payload.projection());

        let mut invalid_version = encoded.clone();
        invalid_version["schema_version"] = json!(2);
        assert!(serde_json::from_value::<NativeTensorProjection>(invalid_version).is_err());

        let mut invalid_digest = encoded.clone();
        invalid_digest["content_digest"] = json!("ABCDEF");
        assert!(serde_json::from_value::<NativeTensorProjection>(invalid_digest).is_err());

        let mut invalid_resident_bytes = encoded;
        invalid_resident_bytes["resident_bytes"] = json!(0);
        assert!(serde_json::from_value::<NativeTensorProjection>(invalid_resident_bytes).is_err());
        Ok(())
    }

    #[test]
    fn zero_byte_tensor_has_a_valid_zero_resident_projection() -> Result<(), Box<dyn Error>> {
        let payload = tensor_payload(NativeTensorRole::Latent, &[])?;
        assert_eq!(payload.projection().resident_bytes(), 0);
        assert!(payload.tensor().contiguous_bytes()?.is_empty());
        payload.validate()?;
        Ok(())
    }

    #[test]
    fn validation_detects_projection_drift() -> Result<(), Box<dyn Error>> {
        let mut payload = tensor_payload(NativeTensorRole::Latent, &[1.0])?;
        let replacement = if payload.projection.content_digest.starts_with('0') {
            "1"
        } else {
            "0"
        };
        payload
            .projection
            .content_digest
            .replace_range(0..1, replacement);
        assert!(matches!(
            payload.validate(),
            Err(NativeTensorPayloadError::ProjectionChanged)
        ));
        Ok(())
    }
}
