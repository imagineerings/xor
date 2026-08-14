use crate::{
    NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeCachePolicy, NativeEffectClass, NativeHandleKind,
    NativeHandleStoreError, NativeHandleType, NativeInputDescriptor, NativeNode, NativeNodeBinding,
    NativeNodeBindingsFactory, NativeNodeContext, NativeNodeContractError, NativeNodeDescriptor,
    NativeNodeFailure, NativeNodeFailureKind, NativeNodeOutcome, NativeNodePresentation,
    NativeOpaqueHandle, NativeOutputDescriptor, NativePortCardinality, NativePrimitive,
    NativePrimitiveType, NativeStoredPayload, NativeTypeUnion, NativeValue, NativeValueType,
    built_in_source_schema,
};
use comfy_media::{NativeVideoBitDepth, NativeVideoPayload};
use comfy_tensor::NativeTensorRole;
use futures::future::BoxFuture;
use std::{collections::BTreeMap, sync::Arc};

pub const NODE_DESCRIPTOR_IDS: &[&str] = &["CreateVideo"];
pub const NATIVE_NODE_BINDINGS: NativeNodeBindingsFactory = native_node_bindings;

const CLASS_TYPE: &str = "CreateVideo";
const FEATURE_ID: &str = "COMFY-NODE-0124";
const IMPLEMENTATION_VERSION: &str = "source-7b8f73c9-v1";
const CACHE_CHANGE_TOKEN: &str = "source-7b8f73c9-video-components-v1";

fn native_node_bindings() -> Result<Vec<NativeNodeBinding>, NativeNodeContractError> {
    Ok(vec![native_node_binding()?])
}

fn native_node_binding() -> Result<NativeNodeBinding, NativeNodeContractError> {
    let source_schema = built_in_source_schema(CLASS_TYPE)
        .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?
        .bind_execution_ports(
            &[
                "images".to_owned(),
                "fps".to_owned(),
                "audio".to_owned(),
                "bit_depth".to_owned(),
            ],
            &[],
            &["video".to_owned()],
        )
        .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?;
    Ok(NativeNodeBinding::Executable {
        feature_id: FEATURE_ID.to_owned(),
        descriptor: NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: CLASS_TYPE.to_owned(),
            implementation_version: IMPLEMENTATION_VERSION.to_owned(),
            source_schema: Some(source_schema),
            inputs: vec![
                handle_input("images", image_type()?, true)?,
                primitive_input("fps", NativePrimitiveType::Number, true)?,
                handle_input("audio", audio_type()?, false)?,
                primitive_input("bit_depth", NativePrimitiveType::Integer, false)?,
            ],
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "video".to_owned(),
                produced_type: NativeValueType::Handle(video_type()?),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        },
        presentation: NativeNodePresentation {
            display_name: "Create Video".to_owned(),
            category: "video".to_owned(),
            description: "Create a video from images.".to_owned(),
            output_names: vec!["video".to_owned()],
            search_aliases: vec!["images to video".to_owned()],
            is_deprecated: false,
            is_experimental: false,
        },
        node: Arc::new(CreateVideoNode),
    })
}

fn handle_input(
    name: &str,
    handle_type: NativeHandleType,
    required: bool,
) -> Result<NativeInputDescriptor, NativeNodeContractError> {
    Ok(NativeInputDescriptor {
        name: name.to_owned(),
        accepted_types: NativeTypeUnion::new([NativeValueType::Handle(handle_type)])?,
        required,
        hidden: false,
        lazy: false,
        cardinality: NativePortCardinality::Scalar,
        allows_literal: false,
    })
}

fn primitive_input(
    name: &str,
    primitive_type: NativePrimitiveType,
    required: bool,
) -> Result<NativeInputDescriptor, NativeNodeContractError> {
    Ok(NativeInputDescriptor {
        name: name.to_owned(),
        accepted_types: NativeTypeUnion::new([NativeValueType::Primitive(primitive_type)])?,
        required,
        hidden: false,
        lazy: false,
        cardinality: NativePortCardinality::Scalar,
        allows_literal: true,
    })
}

fn image_type() -> Result<NativeHandleType, NativeNodeContractError> {
    NativeHandleType::new(NativeHandleKind::Image, "IMAGE")
}

fn audio_type() -> Result<NativeHandleType, NativeNodeContractError> {
    NativeHandleType::new(NativeHandleKind::Audio, "AUDIO")
}

fn video_type() -> Result<NativeHandleType, NativeNodeContractError> {
    NativeHandleType::new(NativeHandleKind::Video, "VIDEO")
}

#[derive(Debug)]
struct CreateVideoNode;

impl NativeNode for CreateVideoNode {
    fn class_type(&self) -> &str {
        CLASS_TYPE
    }

    fn implementation_version(&self) -> &str {
        IMPLEMENTATION_VERSION
    }

    fn demanded_lazy_inputs(
        &self,
        context: &NativeNodeContext,
        available_inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<std::collections::BTreeSet<String>, NativeNodeFailure> {
        check_cancellation(context)?;
        parse_inputs(available_inputs)?;
        Ok(std::collections::BTreeSet::new())
    }

    fn cache_change_token(
        &self,
        inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<String, NativeNodeFailure> {
        parse_inputs(inputs)?;
        Ok(CACHE_CHANGE_TOKEN.to_owned())
    }

    fn execute<'a>(
        &'a self,
        context: NativeNodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
        Box::pin(async move {
            check_cancellation(&context)?;
            let parsed = parse_inputs(&inputs)?;
            let image_type = image_type().map_err(|error| invalid_inputs(error.to_string()))?;
            let resolved_image = context
                .handle_store()
                .resolve(parsed.images, &image_type, &context.cancellation)
                .map_err(handle_failure)?;
            let NativeStoredPayload::Tensor(image_payload) = resolved_image.as_ref() else {
                return Err(invalid_payload("IMAGE handle does not contain a tensor payload"));
            };
            if image_payload.role() != NativeTensorRole::Image {
                return Err(invalid_payload("IMAGE handle has the wrong tensor role"));
            }
            let image = image_payload
                .image()
                .ok_or_else(|| invalid_payload("IMAGE handle has no canonical ImageTensor"))?;

            let resolved_audio = match parsed.audio {
                Some(audio) => Some(
                    context
                        .handle_store()
                        .resolve(
                            audio,
                            &audio_type().map_err(|error| invalid_inputs(error.to_string()))?,
                            &context.cancellation,
                        )
                        .map_err(handle_failure)?,
                ),
                None => None,
            };
            let audio = resolved_audio
                .as_ref()
                .map(|resolved| match resolved.as_ref() {
                    NativeStoredPayload::Audio(audio) => Ok(audio.as_ref().clone()),
                    _ => Err(invalid_payload("AUDIO handle does not contain an audio payload")),
                })
                .transpose()?;

            check_cancellation(&context)?;
            let video = NativeVideoPayload::checked(
                image.tensor().clone(),
                parsed.frame_rate.0,
                parsed.frame_rate.1,
                parsed.bit_depth,
                audio,
                None,
                BTreeMap::new(),
            )
            .map_err(|error| invalid_payload(error.to_string()))?;
            check_cancellation(&context)?;
            let output_handle = context
                .handle_store()
                .publish(
                    NativeStoredPayload::Video(Arc::new(video)),
                    &context.cancellation,
                )
                .map_err(handle_failure)?;
            let outcome = NativeNodeOutcome::Values {
                outputs: vec![NativeValue::Handle {
                    value: output_handle,
                }],
                ui: None,
                effects: Vec::new(),
            };
            outcome
                .validate()
                .map_err(|error| invalid_inputs(error.to_string()))?;
            drop(resolved_audio);
            drop(resolved_image);
            Ok(outcome)
        })
    }
}

struct ParsedInputs<'a> {
    images: &'a NativeOpaqueHandle,
    audio: Option<&'a NativeOpaqueHandle>,
    frame_rate: (u64, u64),
    bit_depth: NativeVideoBitDepth,
}

fn parse_inputs(inputs: &BTreeMap<String, NativeValue>) -> Result<ParsedInputs<'_>, NativeNodeFailure> {
    if !(2..=4).contains(&inputs.len())
        || inputs
            .keys()
            .any(|name| !matches!(name.as_str(), "images" | "fps" | "audio" | "bit_depth"))
    {
        return Err(invalid_inputs("CreateVideo received an unexpected input set"));
    }
    let images = exact_handle(inputs.get("images"), NativeHandleKind::Image, "IMAGE", "images")?;
    let audio = inputs
        .get("audio")
        .map(|value| exact_handle(Some(value), NativeHandleKind::Audio, "AUDIO", "audio"))
        .transpose()?;
    let fps = match inputs.get("fps") {
        Some(NativeValue::Primitive {
            value: NativePrimitive::Number(value),
        }) => *value,
        _ => return Err(invalid_inputs("CreateVideo fps must be a floating-point number")),
    };
    let frame_rate = exact_positive_f64_fraction(fps)
        .filter(|(numerator, denominator)| {
            let value = *numerator as f64 / *denominator as f64;
            (1.0..=120.0).contains(&value)
        })
        .ok_or_else(|| invalid_inputs("CreateVideo fps must be finite and in 1 through 120"))?;
    let bit_depth = match inputs.get("bit_depth") {
        None => NativeVideoBitDepth::Eight,
        Some(NativeValue::Primitive {
            value: NativePrimitive::Integer(8) | NativePrimitive::UnsignedInteger(8),
        }) => NativeVideoBitDepth::Eight,
        Some(NativeValue::Primitive {
            value: NativePrimitive::Integer(10) | NativePrimitive::UnsignedInteger(10),
        }) => NativeVideoBitDepth::Ten,
        _ => return Err(invalid_inputs("CreateVideo bit_depth must be 8 or 10")),
    };
    Ok(ParsedInputs {
        images,
        audio,
        frame_rate,
        bit_depth,
    })
}

fn exact_handle<'a>(
    value: Option<&'a NativeValue>,
    kind: NativeHandleKind,
    type_id: &str,
    name: &str,
) -> Result<&'a NativeOpaqueHandle, NativeNodeFailure> {
    let Some(NativeValue::Handle { value }) = value else {
        return Err(invalid_inputs(format!("CreateVideo {name} must be a handle")));
    };
    if value.handle_type().kind != kind || value.handle_type().type_id != type_id {
        return Err(invalid_inputs(format!(
            "CreateVideo {name} must be an exact {type_id} handle"
        )));
    }
    Ok(value)
}

fn exact_positive_f64_fraction(value: f64) -> Option<(u64, u64)> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let bits = value.to_bits();
    if bits >> 63 != 0 {
        return None;
    }
    let exponent_bits = i32::try_from((bits >> 52) & 0x7ff).ok()?;
    if exponent_bits == 0 || exponent_bits == 0x7ff {
        return None;
    }
    let significand = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let binary_exponent = exponent_bits - 1023 - 52;
    let (numerator, denominator) = if binary_exponent >= 0 {
        (
            significand.checked_shl(u32::try_from(binary_exponent).ok()?)?,
            1,
        )
    } else {
        (
            significand,
            1_u64.checked_shl(u32::try_from(-binary_exponent).ok()?)?,
        )
    };
    let divisor = greatest_common_divisor(numerator, denominator);
    Some((numerator / divisor, denominator / divisor))
}

fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn check_cancellation(context: &NativeNodeContext) -> Result<(), NativeNodeFailure> {
    context
        .cancellation
        .check()
        .map_err(|_| interrupted_failure())
}

fn handle_failure(error: NativeHandleStoreError) -> NativeNodeFailure {
    if matches!(error, NativeHandleStoreError::Cancelled) {
        interrupted_failure()
    } else {
        NativeNodeFailure {
            code: "invalid_media_handle".to_owned(),
            message: format!("CreateVideo input handle is unavailable: {error}"),
            kind: NativeNodeFailureKind::Failure,
            retryable: false,
        }
    }
}

fn invalid_payload(message: impl Into<String>) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "invalid_video_components".to_owned(),
        message: format!("CreateVideo: {}", message.into()),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn invalid_inputs(message: impl Into<String>) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "invalid_node_inputs".to_owned(),
        message: message.into(),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn interrupted_failure() -> NativeNodeFailure {
    NativeNodeFailure {
        code: "execution_interrupted".to_owned(),
        message: "CreateVideo execution was interrupted".to_owned(),
        kind: NativeNodeFailureKind::Interrupted,
        retryable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        NativeHandleStore, NativeHandleStoreIdentity, NativeResolvedPayload,
        NativeResolvedPayloadRetention,
    };
    use comfy_media::NativeAudioPayload;
    use comfy_tensor::{
        CpuWorkspaceAuthority, DType, DeviceId, ImageTensor, NativeTensorPayload, StreamId,
        TensorDescriptor,
    };
    use comfy_types::{AttemptId, CancellationToken, NodeId, PromptId};
    use serde_json::Value;
    use std::{
        error::Error,
        sync::{
            Arc, Mutex,
            atomic::{AtomicU64, Ordering},
        },
    };
    use uuid::Uuid;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/nodes/video-comfy-node-0124/fixture.json"
    ));

    #[derive(Debug)]
    struct TestRetention;

    impl NativeResolvedPayloadRetention for TestRetention {}

    #[derive(Debug)]
    struct TestStore {
        identity: NativeHandleStoreIdentity,
        attempt_id: AttemptId,
        next_identifier: AtomicU64,
        values: Mutex<BTreeMap<String, Arc<NativeStoredPayload>>>,
    }

    impl TestStore {
        fn new(attempt_id: AttemptId) -> Result<Arc<Self>, Box<dyn Error>> {
            Ok(Arc::new(Self {
                identity: NativeHandleStoreIdentity::new(
                    Uuid::from_u128(0x12401),
                    Uuid::from_u128(0x12402),
                )?,
                attempt_id,
                next_identifier: AtomicU64::new(1),
                values: Mutex::new(BTreeMap::new()),
            }))
        }

        fn count(&self) -> Result<usize, NativeHandleStoreError> {
            self.values
                .lock()
                .map(|values| values.len())
                .map_err(|_| NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned()))
        }
    }

    impl NativeHandleStore for TestStore {
        fn identity(&self) -> NativeHandleStoreIdentity {
            self.identity
        }

        fn attempt_id(&self) -> AttemptId {
            self.attempt_id
        }

        fn resolve(
            &self,
            handle: &NativeOpaqueHandle,
            expected_type: &NativeHandleType,
            cancellation: &CancellationToken,
        ) -> Result<NativeResolvedPayload, NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            if handle.store_identity() != self.identity {
                return Err(NativeHandleStoreError::WrongStore);
            }
            if handle.handle_type() != expected_type {
                return Err(NativeHandleStoreError::WrongType {
                    expected: expected_type.type_id.clone(),
                    actual: handle.handle_type().type_id.clone(),
                });
            }
            let payload = self
                .values
                .lock()
                .map_err(|_| {
                    NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned())
                })?
                .get(handle.identifier())
                .cloned()
                .ok_or_else(|| NativeHandleStoreError::Missing(handle.identifier().to_owned()))?;
            if handle.digest_sha256() != Some(payload.digest_sha256().as_str()) {
                return Err(NativeHandleStoreError::DigestMismatch);
            }
            NativeResolvedPayload::checked(payload, Arc::new(TestRetention)).map_err(Into::into)
        }

        fn publish(
            &self,
            payload: NativeStoredPayload,
            cancellation: &CancellationToken,
        ) -> Result<NativeOpaqueHandle, NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            payload.validate()?;
            let handle_type = payload.handle_type()?;
            let digest = payload.digest_sha256();
            let identifier = format!(
                "video-component-{}",
                self.next_identifier.fetch_add(1, Ordering::AcqRel)
            );
            self.values
                .lock()
                .map_err(|_| {
                    NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned())
                })?
                .insert(identifier.clone(), Arc::new(payload));
            NativeOpaqueHandle::new(handle_type, self.identity, identifier, 1, Some(digest))
                .map_err(Into::into)
        }

        fn revoke(
            &self,
            handle: &NativeOpaqueHandle,
            cancellation: &CancellationToken,
        ) -> Result<(), NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            self.values
                .lock()
                .map_err(|_| {
                    NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned())
                })?
                .remove(handle.identifier())
                .ok_or_else(|| NativeHandleStoreError::Missing(handle.identifier().to_owned()))?;
            Ok(())
        }
    }

    struct Harness {
        store: Arc<TestStore>,
        backend: comfy_tensor::CpuBackend,
        workspace: CpuWorkspaceAuthority,
        attempt_id: AttemptId,
        node_id: NodeId,
    }

    impl Harness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let attempt_id = AttemptId(Uuid::from_u128(0x12403));
            let (backend, workspace) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
            Ok(Self {
                store: TestStore::new(attempt_id)?,
                backend,
                workspace,
                attempt_id,
                node_id: NodeId("create-video-test".to_owned()),
            })
        }

        fn image_handle(&self) -> Result<(NativeOpaqueHandle, comfy_tensor::StorageId), Box<dyn Error>> {
            let cancellation = CancellationToken::default();
            let context = self.backend.execution_context(
                StreamId::DEFAULT,
                self.workspace.authorize_workspace(0)?,
                &cancellation,
            );
            let image = ImageTensor::from_f32(
                &self.backend,
                &context,
                2,
                2,
                2,
                3,
                &[0.25; 24],
            )?;
            let storage_id = image.tensor().storage_id();
            let handle = self.store.publish(
                NativeStoredPayload::Tensor(Arc::new(NativeTensorPayload::from_image(
                    NativeTensorRole::Image,
                    image,
                )?)),
                &cancellation,
            )?;
            Ok((handle, storage_id))
        }

        fn audio_handle(&self) -> Result<(NativeOpaqueHandle, comfy_tensor::StorageId), Box<dyn Error>> {
            let descriptor = TensorDescriptor::contiguous(
                vec![1, 1, 4],
                DType::F32,
                DeviceId::CPU,
                StreamId::DEFAULT,
            )?;
            let cancellation = CancellationToken::default();
            let context = self.backend.execution_context(
                StreamId::DEFAULT,
                self.workspace.authorize_workspace(0)?,
                &cancellation,
            );
            let (waveform, _) = self.backend.upload_bytes(descriptor, &[0; 16], &context)?;
            let storage_id = waveform.storage_id();
            let handle = self.store.publish(
                NativeStoredPayload::Audio(Arc::new(NativeAudioPayload::checked(
                    waveform, 48_000,
                )?)),
                &cancellation,
            )?;
            Ok((handle, storage_id))
        }

        fn context(
            &self,
            cancellation: CancellationToken,
        ) -> Result<NativeNodeContext, Box<dyn Error>> {
            Ok(NativeNodeContext::new(
                PromptId(Uuid::from_u128(0x12404)),
                self.attempt_id,
                self.node_id.clone(),
                cancellation,
                self.workspace.authorize_workspace(0)?,
                self.store.clone(),
            )?)
        }

        fn inputs(
            &self,
            image: NativeOpaqueHandle,
            fps: f64,
            audio: Option<NativeOpaqueHandle>,
            bit_depth: Option<NativePrimitive>,
        ) -> BTreeMap<String, NativeValue> {
            let mut inputs = BTreeMap::from([
                ("images".to_owned(), NativeValue::Handle { value: image }),
                (
                    "fps".to_owned(),
                    NativeValue::Primitive {
                        value: NativePrimitive::Number(fps),
                    },
                ),
            ]);
            if let Some(audio) = audio {
                inputs.insert("audio".to_owned(), NativeValue::Handle { value: audio });
            }
            if let Some(bit_depth) = bit_depth {
                inputs.insert(
                    "bit_depth".to_owned(),
                    NativeValue::Primitive { value: bit_depth },
                );
            }
            inputs
        }

        fn video(&self, outcome: NativeNodeOutcome) -> Result<NativeVideoPayload, Box<dyn Error>> {
            let NativeNodeOutcome::Values { outputs, ui, effects } = outcome else {
                return Err("CreateVideo did not return values".into());
            };
            assert!(ui.is_none());
            assert!(effects.is_empty());
            let Some(NativeValue::Handle { value }) = outputs.first() else {
                return Err("CreateVideo output handle is absent".into());
            };
            let resolved = self.store.resolve(
                value,
                &video_type()?,
                &CancellationToken::default(),
            )?;
            let NativeStoredPayload::Video(video) = resolved.as_ref() else {
                return Err("CreateVideo output is not a VIDEO payload".into());
            };
            Ok(video.as_ref().clone())
        }
    }

    fn executable() -> Result<Arc<dyn NativeNode>, Box<dyn Error>> {
        native_node_bindings()?
            .into_iter()
            .find_map(|binding| match binding {
                NativeNodeBinding::Executable { node, .. } => Some(node),
                _ => None,
            })
            .ok_or_else(|| "CreateVideo executable binding is absent".into())
    }

    #[test]
    fn create_video_descriptor_and_fraction_match_source() -> Result<(), Box<dyn Error>> {
        let fixture: Value = serde_json::from_str(FIXTURE)?;
        assert_eq!(fixture["feature_id"], FEATURE_ID);
        assert_eq!(
            fixture["source"]["sha256"],
            "db1d0d40e065a50a4b10c2780511e4a9a75482916a76c175000e58ab875da7c9"
        );
        assert_eq!(exact_positive_f64_fraction(29.97), Some((1_054_475_631_502_295, 35_184_372_088_832)));
        assert_eq!(exact_positive_f64_fraction(30.0), Some((30, 1)));
        let binding = native_node_binding()?;
        binding.validate()?;
        let descriptor = binding.descriptor();
        assert_eq!(descriptor.class_type, CLASS_TYPE);
        assert_eq!(descriptor.inputs.iter().map(|input| input.name.as_str()).collect::<Vec<_>>(), ["images", "fps", "audio", "bit_depth"]);
        assert_eq!(descriptor.outputs[0].name, "video");
        assert_eq!(descriptor.effect, NativeEffectClass::Pure);
        assert_eq!(descriptor.cache, NativeCachePolicy::InputIdentity);
        let schema = descriptor.source_schema.as_ref().ok_or("missing source schema")?;
        assert_eq!(
            schema.inputs[1].default,
            Some(crate::NativeSchemaValue::FiniteDecimal {
                value: "30.0".to_owned()
            })
        );
        assert_eq!(schema.inputs[3].default, Some(crate::NativeSchemaValue::UnsignedInteger { value: 8 }));
        Ok(())
    }

    #[test]
    fn create_video_publishes_exact_component_aliases() -> Result<(), Box<dyn Error>> {
        let harness = Harness::new()?;
        let (image, image_storage) = harness.image_handle()?;
        let (audio, audio_storage) = harness.audio_handle()?;
        let outcome = futures::executor::block_on(executable()?.execute(
            harness.context(CancellationToken::default())?,
            harness.inputs(
                image,
                29.97,
                Some(audio),
                Some(NativePrimitive::UnsignedInteger(10)),
            ),
        ))?;
        let video = harness.video(outcome)?;
        assert_eq!(video.frames().storage_id(), image_storage);
        assert_eq!(video.frame_rate(), (1_054_475_631_502_295, 35_184_372_088_832));
        assert_eq!(video.bit_depth(), NativeVideoBitDepth::Ten);
        assert_eq!(
            video.audio().ok_or("audio was not retained")?.waveform().storage_id(),
            audio_storage
        );
        assert!(video.alpha().is_none());
        assert!(video.metadata().is_empty());
        assert_eq!(harness.store.count()?, 3);
        Ok(())
    }

    #[test]
    fn create_video_rejects_invalid_inputs_and_cancellation_without_publication()
    -> Result<(), Box<dyn Error>> {
        let harness = Harness::new()?;
        let (image, _) = harness.image_handle()?;
        let node = executable()?;
        for (fps, depth) in [
            (0.0, Some(NativePrimitive::Integer(8))),
            (121.0, Some(NativePrimitive::Integer(8))),
            (f64::NAN, Some(NativePrimitive::Integer(8))),
            (30.0, Some(NativePrimitive::Integer(9))),
        ] {
            let error = futures::executor::block_on(node.execute(
                harness.context(CancellationToken::default())?,
                harness.inputs(image.clone(), fps, None, depth),
            ))
            .expect_err("invalid CreateVideo input must fail");
            assert_eq!(error.code, "invalid_node_inputs");
            assert_eq!(harness.store.count()?, 1);
        }

        let cancellation = CancellationToken::default();
        cancellation.cancel();
        let error = futures::executor::block_on(node.execute(
            harness.context(cancellation)?,
            harness.inputs(image.clone(), 30.0, None, None),
        ))
        .expect_err("cancelled CreateVideo must fail");
        assert_eq!(error.kind, NativeNodeFailureKind::Interrupted);
        assert_eq!(harness.store.count()?, 1);

        let outcome = futures::executor::block_on(node.execute(
            harness.context(CancellationToken::default())?,
            harness.inputs(image, 1.0, None, None),
        ))?;
        let video = harness.video(outcome)?;
        assert_eq!(video.frame_rate(), (1, 1));
        assert_eq!(video.bit_depth(), NativeVideoBitDepth::Eight);
        assert!(video.audio().is_none());
        assert_eq!(harness.store.count()?, 2);
        Ok(())
    }
}
