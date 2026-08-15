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
use comfy_model::FrameInterpolationError;
use comfy_tensor::{
    DType, DeviceId, ImageTensor, Layout, MemoryFormatReference, NativeTensorPayload,
    NativeTensorPayloadError, NativeTensorRole, Scalar, TensorError,
    generated_comfy_operator_indirection_01::{
        OperatorIndirectionError, cast_to_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_03::ElementwiseOperand,
    generated_elementwise_or_runtime_operation_05::div_with_context_exact_native,
    generated_storage_dtype_device_01::contiguous_with_context_exact_native,
};
use comfy_types::CancellationToken;
use futures::future::BoxFuture;
use std::{collections::BTreeMap, sync::Arc};

pub const NODE_DESCRIPTOR_IDS: &[&str] = &["CreateVideo", "GetVideoComponents", "FrameInterpolate"];
pub const NATIVE_NODE_BINDINGS: NativeNodeBindingsFactory = native_node_bindings;

const CLASS_TYPE: &str = "CreateVideo";
const FEATURE_ID: &str = "COMFY-NODE-0124";
const IMPLEMENTATION_VERSION: &str = "source-7b8f73c9-v1";
const CACHE_CHANGE_TOKEN: &str = "source-7b8f73c9-video-components-v1";
const COMPONENTS_CLASS_TYPE: &str = "GetVideoComponents";
const COMPONENTS_FEATURE_ID: &str = "COMFY-NODE-0207";
const COMPONENTS_IMPLEMENTATION_VERSION: &str = "source-b2232b2c-v1";
const COMPONENTS_CACHE_CHANGE_TOKEN: &str = "source-b2232b2c-video-components-v1";
const INTERPOLATE_CLASS_TYPE: &str = "FrameInterpolate";
const INTERPOLATE_FEATURE_ID: &str = "COMFY-NODE-0190";
const INTERPOLATE_IMPLEMENTATION_VERSION: &str = "source-e0b9dd6e-v1";
const INTERPOLATE_CACHE_CHANGE_TOKEN: &str = "source-e0b9dd6e-frame-interpolate-v1";

fn native_node_bindings() -> Result<Vec<NativeNodeBinding>, NativeNodeContractError> {
    Ok(vec![
        native_node_binding()?,
        components_node_binding()?,
        interpolate_node_binding()?,
    ])
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

fn components_node_binding() -> Result<NativeNodeBinding, NativeNodeContractError> {
    let source_schema = built_in_source_schema(COMPONENTS_CLASS_TYPE)
        .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?
        .bind_execution_ports(
            &["video".to_owned()],
            &[],
            &[
                "images".to_owned(),
                "audio".to_owned(),
                "fps".to_owned(),
                "bit_depth".to_owned(),
            ],
        )
        .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?;
    Ok(NativeNodeBinding::Executable {
        feature_id: COMPONENTS_FEATURE_ID.to_owned(),
        descriptor: NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: COMPONENTS_CLASS_TYPE.to_owned(),
            implementation_version: COMPONENTS_IMPLEMENTATION_VERSION.to_owned(),
            source_schema: Some(source_schema),
            inputs: vec![handle_input("video", video_type()?, true)?],
            dynamic_inputs: Vec::new(),
            outputs: vec![
                NativeOutputDescriptor {
                    name: "images".to_owned(),
                    produced_type: NativeValueType::Handle(image_type()?),
                    is_list: false,
                },
                NativeOutputDescriptor {
                    name: "audio".to_owned(),
                    produced_type: NativeValueType::Handle(audio_type()?),
                    is_list: false,
                },
                NativeOutputDescriptor {
                    name: "fps".to_owned(),
                    produced_type: NativeValueType::Primitive(NativePrimitiveType::Number),
                    is_list: false,
                },
                NativeOutputDescriptor {
                    name: "bit_depth".to_owned(),
                    produced_type: NativeValueType::Primitive(NativePrimitiveType::Integer),
                    is_list: false,
                },
            ],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        },
        presentation: NativeNodePresentation {
            display_name: "Get Video Components".to_owned(),
            category: "video".to_owned(),
            description:
                "Extracts all components from a video: frames, audio, framerate, and bit depth."
                    .to_owned(),
            output_names: vec![
                "images".to_owned(),
                "audio".to_owned(),
                "fps".to_owned(),
                "bit_depth".to_owned(),
            ],
            search_aliases: vec![
                "extract frames".to_owned(),
                "split video".to_owned(),
                "video to images".to_owned(),
                "demux".to_owned(),
            ],
            is_deprecated: false,
            is_experimental: false,
        },
        node: Arc::new(GetVideoComponentsNode),
    })
}

fn interpolate_node_binding() -> Result<NativeNodeBinding, NativeNodeContractError> {
    let source_schema = built_in_source_schema(INTERPOLATE_CLASS_TYPE)
        .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?
        .bind_execution_ports(
            &[
                "interp_model".to_owned(),
                "images".to_owned(),
                "multiplier".to_owned(),
            ],
            &[],
            &["images".to_owned()],
        )
        .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?;
    Ok(NativeNodeBinding::Executable {
        feature_id: INTERPOLATE_FEATURE_ID.to_owned(),
        descriptor: NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: INTERPOLATE_CLASS_TYPE.to_owned(),
            implementation_version: INTERPOLATE_IMPLEMENTATION_VERSION.to_owned(),
            source_schema: Some(source_schema),
            inputs: vec![
                handle_input("interp_model", interpolation_model_type()?, true)?,
                handle_input("images", image_type()?, true)?,
                primitive_input("multiplier", NativePrimitiveType::Integer, true)?,
            ],
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "images".to_owned(),
                produced_type: NativeValueType::Handle(image_type()?),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        },
        presentation: NativeNodePresentation {
            display_name: "Run Frame Interpolation Model".to_owned(),
            category: "video".to_owned(),
            description: String::new(),
            output_names: vec!["images".to_owned()],
            search_aliases: vec![
                "rife".to_owned(),
                "film".to_owned(),
                "frame interpolation".to_owned(),
                "slow motion".to_owned(),
                "interpolate frames".to_owned(),
                "vfi".to_owned(),
            ],
            is_deprecated: false,
            is_experimental: false,
        },
        node: Arc::new(FrameInterpolateNode),
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

fn interpolation_model_type() -> Result<NativeHandleType, NativeNodeContractError> {
    NativeHandleType::new(NativeHandleKind::Model, "INTERP_MODEL")
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
                return Err(invalid_payload(
                    "IMAGE handle does not contain a tensor payload",
                ));
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
                    _ => Err(invalid_payload(
                        "AUDIO handle does not contain an audio payload",
                    )),
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

#[derive(Debug)]
struct GetVideoComponentsNode;

impl NativeNode for GetVideoComponentsNode {
    fn class_type(&self) -> &str {
        COMPONENTS_CLASS_TYPE
    }

    fn implementation_version(&self) -> &str {
        COMPONENTS_IMPLEMENTATION_VERSION
    }

    fn demanded_lazy_inputs(
        &self,
        context: &NativeNodeContext,
        available_inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<std::collections::BTreeSet<String>, NativeNodeFailure> {
        check_components_cancellation(context)?;
        components_input(available_inputs)?;
        Ok(std::collections::BTreeSet::new())
    }

    fn cache_change_token(
        &self,
        inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<String, NativeNodeFailure> {
        components_input(inputs)?;
        Ok(COMPONENTS_CACHE_CHANGE_TOKEN.to_owned())
    }

    fn execute<'a>(
        &'a self,
        context: NativeNodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
        Box::pin(async move {
            check_components_cancellation(&context)?;
            let video_handle = components_input(&inputs)?;
            let resolved = context
                .handle_store()
                .resolve(
                    video_handle,
                    &video_type().map_err(|error| components_invalid(error.to_string()))?,
                    &context.cancellation,
                )
                .map_err(components_handle_failure)?;
            let NativeStoredPayload::Video(video) = resolved.as_ref() else {
                return Err(components_invalid(
                    "VIDEO handle does not contain a canonical video payload",
                ));
            };
            video
                .validate()
                .map_err(|error| components_invalid(error.to_string()))?;
            let image = project_video_frames(&context, video.frames())?;
            let image_payload = NativeTensorPayload::from_image(NativeTensorRole::Image, image)
                .map_err(|error| components_invalid(error.to_string()))?;
            let audio_payload = video
                .audio()
                .cloned()
                .map(|audio| NativeStoredPayload::Audio(Arc::new(audio)));
            let (frame_rate_numerator, frame_rate_denominator) = video.frame_rate();
            let frame_rate = frame_rate_numerator as f64 / frame_rate_denominator as f64;
            let bit_depth = i64::from(video.bit_depth().bits());
            drop(resolved);

            let mut published = Vec::with_capacity(2);
            let image_handle = publish_component(
                &context,
                NativeStoredPayload::Tensor(Arc::new(image_payload)),
                &mut published,
            )?;
            let audio_value = if let Some(audio_payload) = audio_payload {
                match publish_component(&context, audio_payload, &mut published) {
                    Ok(handle) => NativeValue::Handle { value: handle },
                    Err(error) => {
                        rollback_components(&context, &published)?;
                        return Err(error);
                    }
                }
            } else {
                NativeValue::Primitive {
                    value: NativePrimitive::Null,
                }
            };
            if let Err(error) = check_components_cancellation(&context) {
                rollback_components(&context, &published)?;
                return Err(error);
            }
            let outcome = NativeNodeOutcome::Values {
                outputs: vec![
                    NativeValue::Handle {
                        value: image_handle,
                    },
                    audio_value,
                    NativeValue::Primitive {
                        value: NativePrimitive::Number(frame_rate),
                    },
                    NativeValue::Primitive {
                        value: NativePrimitive::Integer(bit_depth),
                    },
                ],
                ui: None,
                effects: Vec::new(),
            };
            if let Err(error) = outcome.validate() {
                rollback_components(&context, &published)?;
                return Err(components_invalid(error.to_string()));
            }
            Ok(outcome)
        })
    }
}

#[derive(Debug)]
struct FrameInterpolateNode;

impl NativeNode for FrameInterpolateNode {
    fn class_type(&self) -> &str {
        INTERPOLATE_CLASS_TYPE
    }

    fn implementation_version(&self) -> &str {
        INTERPOLATE_IMPLEMENTATION_VERSION
    }

    fn demanded_lazy_inputs(
        &self,
        context: &NativeNodeContext,
        available_inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<std::collections::BTreeSet<String>, NativeNodeFailure> {
        check_interpolation_cancellation(context)?;
        interpolation_inputs(available_inputs)?;
        Ok(std::collections::BTreeSet::new())
    }

    fn cache_change_token(
        &self,
        inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<String, NativeNodeFailure> {
        interpolation_inputs(inputs)?;
        Ok(INTERPOLATE_CACHE_CHANGE_TOKEN.to_owned())
    }

    fn execute<'a>(
        &'a self,
        context: NativeNodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
        Box::pin(async move { execute_frame_interpolate(&context, &inputs) })
    }
}

fn execute_frame_interpolate(
    context: &NativeNodeContext,
    inputs: &BTreeMap<String, NativeValue>,
) -> Result<NativeNodeOutcome, NativeNodeFailure> {
    check_interpolation_cancellation(context)?;
    let parsed = interpolation_inputs(inputs)?;
    let resolved_model = context
        .handle_store()
        .resolve(
            parsed.interp_model,
            &interpolation_model_type()
                .map_err(|error| interpolation_invalid(error.to_string()))?,
            &context.cancellation,
        )
        .map_err(interpolation_handle_failure)?;
    let resolved_image = context
        .handle_store()
        .resolve(
            parsed.images,
            &image_type().map_err(|error| interpolation_invalid(error.to_string()))?,
            &context.cancellation,
        )
        .map_err(interpolation_handle_failure)?;
    let NativeStoredPayload::Tensor(image_payload) = resolved_image.as_ref() else {
        return Err(interpolation_invalid(
            "IMAGE handle does not contain a tensor payload",
        ));
    };
    if image_payload.role() != NativeTensorRole::Image {
        return Err(interpolation_invalid(
            "IMAGE handle has the wrong tensor role",
        ));
    }
    let image = image_payload
        .image()
        .ok_or_else(|| interpolation_invalid("IMAGE handle has no canonical ImageTensor"))?;
    let (frame_count, _, _, channels) = image.dimensions().map_err(interpolation_tensor_failure)?;

    if frame_count < 2 {
        check_interpolation_cancellation(context)?;
        let outcome = interpolation_outcome(parsed.images.clone());
        outcome
            .validate()
            .map_err(|error| interpolation_invalid(error.to_string()))?;
        drop(resolved_image);
        drop(resolved_model);
        return Ok(outcome);
    }

    if channels != 3 {
        return Err(interpolation_invalid(
            "non-bypass IMAGE input must have three channels",
        ));
    }
    let NativeStoredPayload::Model(stored_model) = resolved_model.as_ref() else {
        return Err(interpolation_invalid(
            "INTERP_MODEL handle does not contain a model payload",
        ));
    };
    let model = stored_model
        .model_payload()
        .frame_interpolation_resource()
        .ok_or_else(|| {
            interpolation_invalid(
                "INTERP_MODEL handle has no concrete frame-interpolation resource",
            )
        })?;
    let compute = context
        .compute_session()
        .map_err(interpolation_compute_failure)?;
    let execution = compute
        .execution_context(context)
        .map_err(interpolation_compute_failure)?;
    if model.stream() != compute.stream()
        || image.tensor().descriptor().stream() != compute.stream()
    {
        return Err(interpolation_invalid(
            "IMAGE, INTERP_MODEL, and compute streams must match",
        ));
    }
    let prepared = cast_to_with_context_exact_native(
        compute.backend(),
        image.tensor(),
        model.dtype(),
        DeviceId::CPU,
        false,
        false,
        &execution,
    )
    .map_err(|error| interpolation_operator_failure(context, error))?;
    let output = model
        .interpolate_sequence(compute.backend(), &prepared, parsed.multiplier, &execution)
        .map_err(|error| interpolation_model_failure(context, error))?;
    let output = cast_to_with_context_exact_native(
        compute.backend(),
        &output,
        DType::F32,
        DeviceId::CPU,
        false,
        false,
        &execution,
    )
    .map_err(|error| interpolation_operator_failure(context, error))?;
    let output = ImageTensor::from_tensor(output).map_err(interpolation_tensor_failure)?;
    let payload = NativeTensorPayload::from_image(NativeTensorRole::Image, output)
        .map_err(interpolation_payload_failure)?;
    drop(resolved_image);
    drop(resolved_model);
    check_interpolation_cancellation(context)?;
    let output_handle = context
        .handle_store()
        .publish(
            NativeStoredPayload::Tensor(Arc::new(payload)),
            &context.cancellation,
        )
        .map_err(interpolation_handle_failure)?;
    let completion = (|| {
        check_interpolation_cancellation(context)?;
        let outcome = interpolation_outcome(output_handle.clone());
        outcome
            .validate()
            .map_err(|error| interpolation_invalid(error.to_string()))?;
        Ok(outcome)
    })();
    match completion {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            rollback_interpolation_output(context, &output_handle)?;
            Err(error)
        }
    }
}

struct InterpolationInputs<'a> {
    interp_model: &'a NativeOpaqueHandle,
    images: &'a NativeOpaqueHandle,
    multiplier: u64,
}

fn interpolation_inputs(
    inputs: &BTreeMap<String, NativeValue>,
) -> Result<InterpolationInputs<'_>, NativeNodeFailure> {
    if inputs.len() != 3
        || inputs
            .keys()
            .any(|name| !matches!(name.as_str(), "interp_model" | "images" | "multiplier"))
    {
        return Err(interpolation_invalid(
            "requires exactly interp_model, images, and multiplier",
        ));
    }
    let interp_model = interpolation_exact_handle(
        inputs.get("interp_model"),
        NativeHandleKind::Model,
        "INTERP_MODEL",
        "interp_model",
    )?;
    let images = interpolation_exact_handle(
        inputs.get("images"),
        NativeHandleKind::Image,
        "IMAGE",
        "images",
    )?;
    let multiplier = match inputs.get("multiplier") {
        Some(NativeValue::Primitive {
            value: NativePrimitive::Integer(value),
        }) if (2..=16).contains(value) => u64::try_from(*value)
            .map_err(|_| interpolation_invalid("multiplier must be between 2 and 16"))?,
        Some(NativeValue::Primitive {
            value: NativePrimitive::UnsignedInteger(value),
        }) if (2..=16).contains(value) => *value,
        _ => {
            return Err(interpolation_invalid(
                "multiplier must be an integer between 2 and 16",
            ));
        }
    };
    Ok(InterpolationInputs {
        interp_model,
        images,
        multiplier,
    })
}

fn interpolation_exact_handle<'a>(
    value: Option<&'a NativeValue>,
    kind: NativeHandleKind,
    type_id: &str,
    name: &str,
) -> Result<&'a NativeOpaqueHandle, NativeNodeFailure> {
    let Some(NativeValue::Handle { value }) = value else {
        return Err(interpolation_invalid(format!("{name} must be a handle")));
    };
    if value.handle_type().kind != kind || value.handle_type().type_id != type_id {
        return Err(interpolation_invalid(format!(
            "{name} must be an exact {type_id} handle"
        )));
    }
    Ok(value)
}

fn interpolation_outcome(handle: NativeOpaqueHandle) -> NativeNodeOutcome {
    NativeNodeOutcome::Values {
        outputs: vec![NativeValue::Handle { value: handle }],
        ui: None,
        effects: Vec::new(),
    }
}

fn rollback_interpolation_output(
    context: &NativeNodeContext,
    handle: &NativeOpaqueHandle,
) -> Result<(), NativeNodeFailure> {
    context
        .handle_store()
        .revoke(handle, &CancellationToken::default())
        .map_err(|error| NativeNodeFailure {
            code: "frame_interpolation_rollback_failed".to_owned(),
            message: format!("FrameInterpolate could not revoke partial output: {error}"),
            kind: NativeNodeFailureKind::Failure,
            retryable: false,
        })
}

fn check_interpolation_cancellation(context: &NativeNodeContext) -> Result<(), NativeNodeFailure> {
    context
        .cancellation
        .check()
        .map_err(|_| interpolation_interrupted())
}

fn interpolation_handle_failure(error: NativeHandleStoreError) -> NativeNodeFailure {
    if matches!(error, NativeHandleStoreError::Cancelled) {
        interpolation_interrupted()
    } else {
        NativeNodeFailure {
            code: "invalid_frame_interpolation_handle".to_owned(),
            message: format!("FrameInterpolate input handle is unavailable: {error}"),
            kind: NativeNodeFailureKind::Failure,
            retryable: false,
        }
    }
}

fn interpolation_compute_failure(error: NativeNodeContractError) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "native_compute_unavailable".to_owned(),
        message: format!("FrameInterpolate compute session is unavailable: {error}"),
        kind: NativeNodeFailureKind::Failure,
        retryable: true,
    }
}

fn interpolation_tensor_failure(error: TensorError) -> NativeNodeFailure {
    match error {
        TensorError::Cancelled => interpolation_interrupted(),
        error @ (TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. }) => NativeNodeFailure {
            code: "frame_interpolation_resource_exhausted".to_owned(),
            message: format!("FrameInterpolate exhausted a bounded resource: {error}"),
            kind: NativeNodeFailureKind::Failure,
            retryable: false,
        },
        error => interpolation_invalid(error.to_string()),
    }
}

fn interpolation_operator_failure(
    context: &NativeNodeContext,
    error: OperatorIndirectionError,
) -> NativeNodeFailure {
    if context.cancellation.is_cancelled() {
        return interpolation_interrupted();
    }
    match error {
        OperatorIndirectionError::Cancelled => interpolation_interrupted(),
        OperatorIndirectionError::Tensor(error) => interpolation_tensor_failure(error),
        error => interpolation_invalid(error.to_string()),
    }
}

fn interpolation_payload_failure(error: NativeTensorPayloadError) -> NativeNodeFailure {
    match error {
        NativeTensorPayloadError::Tensor(error) => interpolation_tensor_failure(error),
        error => interpolation_invalid(error.to_string()),
    }
}

fn interpolation_model_failure(
    context: &NativeNodeContext,
    error: FrameInterpolationError,
) -> NativeNodeFailure {
    if context.cancellation.is_cancelled() {
        return interpolation_interrupted();
    }
    match error {
        FrameInterpolationError::Cancelled => interpolation_interrupted(),
        FrameInterpolationError::ResourceExhausted(message) => NativeNodeFailure {
            code: "frame_interpolation_resource_exhausted".to_owned(),
            message: format!("FrameInterpolate exhausted a bounded resource: {message}"),
            kind: NativeNodeFailureKind::Failure,
            retryable: false,
        },
        FrameInterpolationError::Tensor(error) => interpolation_tensor_failure(error),
        error => NativeNodeFailure {
            code: "execution_error".to_owned(),
            message: format!("FrameInterpolate execution failed: {error}"),
            kind: NativeNodeFailureKind::Failure,
            retryable: false,
        },
    }
}

fn interpolation_interrupted() -> NativeNodeFailure {
    NativeNodeFailure {
        code: "execution_interrupted".to_owned(),
        message: "FrameInterpolate execution was interrupted".to_owned(),
        kind: NativeNodeFailureKind::Interrupted,
        retryable: false,
    }
}

fn interpolation_invalid(message: impl Into<String>) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "invalid_node_inputs".to_owned(),
        message: format!("FrameInterpolate: {}", message.into()),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn components_input(
    inputs: &BTreeMap<String, NativeValue>,
) -> Result<&NativeOpaqueHandle, NativeNodeFailure> {
    if inputs.len() != 1 || !inputs.contains_key("video") {
        return Err(components_invalid(
            "GetVideoComponents requires exactly one video input",
        ));
    }
    exact_components_handle(inputs.get("video"))
}

fn exact_components_handle(
    value: Option<&NativeValue>,
) -> Result<&NativeOpaqueHandle, NativeNodeFailure> {
    let Some(NativeValue::Handle { value }) = value else {
        return Err(components_invalid(
            "GetVideoComponents video must be a handle",
        ));
    };
    if value.handle_type().kind != NativeHandleKind::Video || value.handle_type().type_id != "VIDEO"
    {
        return Err(components_invalid(
            "GetVideoComponents video must be an exact VIDEO handle",
        ));
    }
    Ok(value)
}

fn project_video_frames(
    context: &NativeNodeContext,
    frames: &comfy_tensor::Tensor,
) -> Result<ImageTensor, NativeNodeFailure> {
    let descriptor = frames.descriptor();
    if descriptor.device() != DeviceId::CPU {
        return Err(components_invalid("VIDEO frames must use CPU storage"));
    }
    let [_, _, _, channels] = descriptor.shape() else {
        return Err(components_invalid("VIDEO frames must use BHWC rank four"));
    };
    if !matches!(channels, 1 | 3 | 4) {
        return Err(components_invalid(
            "VIDEO frames must have one, three, or four channels",
        ));
    }
    let contiguous = descriptor
        .is_contiguous()
        .map_err(components_tensor_failure)?;
    match (descriptor.dtype(), contiguous) {
        (DType::F32, true) => {
            ImageTensor::from_tensor(frames.clone()).map_err(components_tensor_failure)
        }
        (DType::F32, false) => {
            let compute = context.compute_session().map_err(|error| {
                components_invalid(format!(
                    "VIDEO frame conversion needs compute access: {error}"
                ))
            })?;
            let execution = compute
                .execution_context(context)
                .map_err(|error| components_invalid(error.to_string()))?;
            let frames = contiguous_with_context_exact_native(
                compute.backend(),
                frames,
                MemoryFormatReference::Layout(Layout::Contiguous),
                &execution,
            )
            .map_err(|error| components_operation_failure(context, error.to_string()))?;
            ImageTensor::from_tensor(frames).map_err(components_tensor_failure)
        }
        (DType::U8, _) => {
            let compute = context.compute_session().map_err(|error| {
                components_invalid(format!(
                    "VIDEO frame conversion needs compute access: {error}"
                ))
            })?;
            let execution = compute
                .execution_context(context)
                .map_err(|error| components_invalid(error.to_string()))?;
            let frames = cast_to_with_context_exact_native(
                compute.backend(),
                frames,
                DType::F32,
                DeviceId::CPU,
                false,
                false,
                &execution,
            )
            .map_err(|error| components_operation_failure(context, error.to_string()))?;
            let frames = div_with_context_exact_native(
                compute.backend(),
                &frames,
                ElementwiseOperand::Scalar(Scalar::Float(255.0)),
                &execution,
            )
            .map_err(|error| components_operation_failure(context, error.to_string()))?;
            check_components_cancellation(context)?;
            ImageTensor::from_tensor(frames).map_err(components_tensor_failure)
        }
        _ => Err(components_invalid(
            "VIDEO frames must use F32 or U8 storage",
        )),
    }
}

fn components_operation_failure(context: &NativeNodeContext, message: String) -> NativeNodeFailure {
    if context.cancellation.is_cancelled() {
        return NativeNodeFailure {
            code: "execution_interrupted".to_owned(),
            message: "GetVideoComponents execution was interrupted".to_owned(),
            kind: NativeNodeFailureKind::Interrupted,
            retryable: false,
        };
    }
    components_invalid(message)
}

fn publish_component(
    context: &NativeNodeContext,
    payload: NativeStoredPayload,
    published: &mut Vec<NativeOpaqueHandle>,
) -> Result<NativeOpaqueHandle, NativeNodeFailure> {
    check_components_cancellation(context)?;
    let handle = context
        .handle_store()
        .publish(payload, &context.cancellation)
        .map_err(components_handle_failure)?;
    published.push(handle.clone());
    Ok(handle)
}

fn rollback_components(
    context: &NativeNodeContext,
    published: &[NativeOpaqueHandle],
) -> Result<(), NativeNodeFailure> {
    let cleanup = CancellationToken::default();
    for handle in published.iter().rev() {
        context
            .handle_store()
            .revoke(handle, &cleanup)
            .map_err(|error| NativeNodeFailure {
                code: "video_component_rollback_failed".to_owned(),
                message: format!("GetVideoComponents could not revoke partial output: {error}"),
                kind: NativeNodeFailureKind::Failure,
                retryable: false,
            })?;
    }
    Ok(())
}

fn check_components_cancellation(context: &NativeNodeContext) -> Result<(), NativeNodeFailure> {
    context.cancellation.check().map_err(|_| NativeNodeFailure {
        code: "execution_interrupted".to_owned(),
        message: "GetVideoComponents execution was interrupted".to_owned(),
        kind: NativeNodeFailureKind::Interrupted,
        retryable: false,
    })
}

fn components_handle_failure(error: NativeHandleStoreError) -> NativeNodeFailure {
    if matches!(error, NativeHandleStoreError::Cancelled) {
        return NativeNodeFailure {
            code: "execution_interrupted".to_owned(),
            message: "GetVideoComponents execution was interrupted".to_owned(),
            kind: NativeNodeFailureKind::Interrupted,
            retryable: false,
        };
    }
    components_invalid(format!("video component handle is unavailable: {error}"))
}

fn components_tensor_failure(error: TensorError) -> NativeNodeFailure {
    if matches!(error, TensorError::Cancelled) {
        return NativeNodeFailure {
            code: "execution_interrupted".to_owned(),
            message: "GetVideoComponents execution was interrupted".to_owned(),
            kind: NativeNodeFailureKind::Interrupted,
            retryable: false,
        };
    }
    components_invalid(error.to_string())
}

fn components_invalid(message: impl Into<String>) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "invalid_video_components".to_owned(),
        message: format!("GetVideoComponents: {}", message.into()),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

struct ParsedInputs<'a> {
    images: &'a NativeOpaqueHandle,
    audio: Option<&'a NativeOpaqueHandle>,
    frame_rate: (u64, u64),
    bit_depth: NativeVideoBitDepth,
}

fn parse_inputs(
    inputs: &BTreeMap<String, NativeValue>,
) -> Result<ParsedInputs<'_>, NativeNodeFailure> {
    if !(2..=4).contains(&inputs.len())
        || inputs
            .keys()
            .any(|name| !matches!(name.as_str(), "images" | "fps" | "audio" | "bit_depth"))
    {
        return Err(invalid_inputs(
            "CreateVideo received an unexpected input set",
        ));
    }
    let images = exact_handle(
        inputs.get("images"),
        NativeHandleKind::Image,
        "IMAGE",
        "images",
    )?;
    let audio = inputs
        .get("audio")
        .map(|value| exact_handle(Some(value), NativeHandleKind::Audio, "AUDIO", "audio"))
        .transpose()?;
    let fps = match inputs.get("fps") {
        Some(NativeValue::Primitive {
            value: NativePrimitive::Number(value),
        }) => *value,
        _ => {
            return Err(invalid_inputs(
                "CreateVideo fps must be a floating-point number",
            ));
        }
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
        return Err(invalid_inputs(format!(
            "CreateVideo {name} must be a handle"
        )));
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
        NativeHandleStore, NativeHandleStoreIdentity, NativeNodeComputeSession,
        NativeNodeServiceIdentity, NativeNodeServices, NativeResolvedPayload,
        NativeResolvedPayloadRetention, NativeStoredModelPayload,
    };
    use comfy_media::NativeAudioPayload;
    use comfy_model::{NativeFrameInterpolationModel, NativeModelPayload};
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
    const COMPONENTS_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/nodes/video-comfy-node-0207/fixture.json"
    ));
    const INTERPOLATION_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/nodes/video-comfy-node-0190/fixture.json"
    ));

    #[derive(Debug)]
    struct TestRetention;

    impl NativeResolvedPayloadRetention for TestRetention {}

    #[derive(Debug)]
    struct TestStore {
        identity: NativeHandleStoreIdentity,
        attempt_id: AttemptId,
        next_identifier: AtomicU64,
        cancel_after_publish: std::sync::atomic::AtomicBool,
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
                cancel_after_publish: std::sync::atomic::AtomicBool::new(false),
                values: Mutex::new(BTreeMap::new()),
            }))
        }

        fn cancel_after_next_publish(&self) {
            self.cancel_after_publish.store(true, Ordering::Release);
        }

        fn count(&self) -> Result<usize, NativeHandleStoreError> {
            self.values.lock().map(|values| values.len()).map_err(|_| {
                NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned())
            })
        }

        fn payload(
            &self,
            handle: &NativeOpaqueHandle,
        ) -> Result<Arc<NativeStoredPayload>, NativeHandleStoreError> {
            self.values
                .lock()
                .map_err(|_| {
                    NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned())
                })?
                .get(handle.identifier())
                .cloned()
                .ok_or_else(|| NativeHandleStoreError::Missing(handle.identifier().to_owned()))
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
            let handle = NativeOpaqueHandle::new(
                handle_type,
                self.identity,
                identifier.clone(),
                1,
                Some(digest),
            )?;
            self.values
                .lock()
                .map_err(|_| {
                    NativeHandleStoreError::Rejected("test store lock was poisoned".to_owned())
                })?
                .insert(identifier, Arc::new(payload));
            if self.cancel_after_publish.swap(false, Ordering::AcqRel) {
                cancellation.cancel();
            }
            Ok(handle)
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
        backend: Arc<comfy_tensor::CpuBackend>,
        workspace: CpuWorkspaceAuthority,
        attempt_id: AttemptId,
        node_id: NodeId,
        scratch_bytes: u64,
    }

    impl Harness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let attempt_id = AttemptId(Uuid::from_u128(0x12403));
            let (backend, workspace) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
            Ok(Self {
                store: TestStore::new(attempt_id)?,
                backend: Arc::new(backend),
                workspace,
                attempt_id,
                node_id: NodeId("create-video-test".to_owned()),
                scratch_bytes: 1024 * 1024,
            })
        }

        fn interpolation() -> Result<Self, Box<dyn Error>> {
            let attempt_id = AttemptId(Uuid::from_u128(0x19003));
            let (backend, workspace) = CpuWorkspaceAuthority::create_backend(512 * 1024 * 1024)?;
            Ok(Self {
                store: TestStore::new(attempt_id)?,
                backend: Arc::new(backend),
                workspace,
                attempt_id,
                node_id: NodeId("frame-interpolate-test".to_owned()),
                scratch_bytes: 256 * 1024 * 1024,
            })
        }

        fn image_handle(
            &self,
        ) -> Result<(NativeOpaqueHandle, comfy_tensor::StorageId), Box<dyn Error>> {
            self.image_handle_with_shape(2, 2, 2, 3, &[0.25; 24])
        }

        fn image_handle_with_shape(
            &self,
            batch: u64,
            height: u64,
            width: u64,
            channels: u64,
            values: &[f32],
        ) -> Result<(NativeOpaqueHandle, comfy_tensor::StorageId), Box<dyn Error>> {
            let cancellation = CancellationToken::default();
            let context = self.backend.execution_context(
                StreamId::DEFAULT,
                self.workspace.authorize_workspace(0)?,
                &cancellation,
            );
            let image = ImageTensor::from_f32(
                &self.backend,
                &context,
                batch,
                height,
                width,
                channels,
                values,
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

        fn interpolation_model_handle(
            &self,
        ) -> Result<(NativeOpaqueHandle, Arc<NativeFrameInterpolationModel>), Box<dyn Error>>
        {
            let cancellation = CancellationToken::default();
            let context = self.backend.execution_context(
                StreamId::DEFAULT,
                self.workspace.authorize_workspace(self.scratch_bytes)?,
                &cancellation,
            );
            let model = Arc::new(NativeFrameInterpolationModel::reduced_rife_test_fixture(
                &self.backend,
                &context,
            )?);
            let payload = Arc::new(NativeModelPayload::frame_interpolation(model.clone())?);
            let stored = NativeStoredModelPayload::model_resource(payload)?;
            let handle = self
                .store
                .publish(NativeStoredPayload::Model(Arc::new(stored)), &cancellation)?;
            Ok((handle, model))
        }

        fn audio_handle(
            &self,
        ) -> Result<(NativeOpaqueHandle, comfy_tensor::StorageId), Box<dyn Error>> {
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

        fn video_handle(
            &self,
            frames: comfy_tensor::Tensor,
            audio: Option<NativeAudioPayload>,
            frame_rate: (u64, u64),
            bit_depth: NativeVideoBitDepth,
        ) -> Result<NativeOpaqueHandle, Box<dyn Error>> {
            Ok(self.store.publish(
                NativeStoredPayload::Video(Arc::new(NativeVideoPayload::checked(
                    frames,
                    frame_rate.0,
                    frame_rate.1,
                    bit_depth,
                    audio,
                    None,
                    BTreeMap::new(),
                )?)),
                &CancellationToken::default(),
            )?)
        }

        fn context(
            &self,
            cancellation: CancellationToken,
        ) -> Result<NativeNodeContext, Box<dyn Error>> {
            self.context_with_scratch(cancellation, self.scratch_bytes)
        }

        fn context_with_scratch(
            &self,
            cancellation: CancellationToken,
            scratch_bytes: u64,
        ) -> Result<NativeNodeContext, Box<dyn Error>> {
            let scratch = self.workspace.authorize_workspace(scratch_bytes)?;
            let compute = NativeNodeComputeSession::checked(
                NativeNodeServiceIdentity::checked(
                    Uuid::from_u128(0x12405),
                    self.attempt_id,
                    self.node_id.clone(),
                )?,
                self.backend.clone(),
                StreamId::DEFAULT,
                &scratch,
            )?;
            Ok(NativeNodeContext::new_with_services(
                PromptId(Uuid::from_u128(0x12404)),
                self.attempt_id,
                self.node_id.clone(),
                cancellation,
                scratch,
                self.store.clone(),
                NativeNodeServices::checked(None, None, Some(compute))?,
            )?)
        }

        fn context_without_compute(
            &self,
            cancellation: CancellationToken,
        ) -> Result<NativeNodeContext, Box<dyn Error>> {
            Ok(NativeNodeContext::new(
                PromptId(Uuid::from_u128(0x19004)),
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
            let NativeNodeOutcome::Values {
                outputs,
                ui,
                effects,
            } = outcome
            else {
                return Err("CreateVideo did not return values".into());
            };
            assert!(ui.is_none());
            assert!(effects.is_empty());
            let Some(NativeValue::Handle { value }) = outputs.first() else {
                return Err("CreateVideo output handle is absent".into());
            };
            let resolved =
                self.store
                    .resolve(value, &video_type()?, &CancellationToken::default())?;
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
                NativeNodeBinding::Executable {
                    descriptor, node, ..
                } if descriptor.class_type == CLASS_TYPE => Some(node),
                _ => None,
            })
            .ok_or_else(|| "CreateVideo executable binding is absent".into())
    }

    fn components_executable() -> Result<Arc<dyn NativeNode>, Box<dyn Error>> {
        native_node_bindings()?
            .into_iter()
            .find_map(|binding| match binding {
                NativeNodeBinding::Executable {
                    descriptor, node, ..
                } if descriptor.class_type == COMPONENTS_CLASS_TYPE => Some(node),
                _ => None,
            })
            .ok_or_else(|| "GetVideoComponents executable binding is absent".into())
    }

    fn interpolation_executable() -> Result<Arc<dyn NativeNode>, Box<dyn Error>> {
        native_node_bindings()?
            .into_iter()
            .find_map(|binding| match binding {
                NativeNodeBinding::Executable {
                    descriptor, node, ..
                } if descriptor.class_type == INTERPOLATE_CLASS_TYPE => Some(node),
                _ => None,
            })
            .ok_or_else(|| "FrameInterpolate executable binding is absent".into())
    }

    fn interpolation_inputs_for_test(
        model: NativeOpaqueHandle,
        images: NativeOpaqueHandle,
        multiplier: NativePrimitive,
    ) -> BTreeMap<String, NativeValue> {
        BTreeMap::from([
            (
                "interp_model".to_owned(),
                NativeValue::Handle { value: model },
            ),
            ("images".to_owned(), NativeValue::Handle { value: images }),
            (
                "multiplier".to_owned(),
                NativeValue::Primitive { value: multiplier },
            ),
        ])
    }

    #[test]
    fn create_video_descriptor_and_fraction_match_source() -> Result<(), Box<dyn Error>> {
        let fixture: Value = serde_json::from_str(FIXTURE)?;
        assert_eq!(fixture["feature_id"], FEATURE_ID);
        assert_eq!(
            fixture["source"]["sha256"],
            "db1d0d40e065a50a4b10c2780511e4a9a75482916a76c175000e58ab875da7c9"
        );
        assert_eq!(
            exact_positive_f64_fraction(29.97),
            Some((1_054_475_631_502_295, 35_184_372_088_832))
        );
        assert_eq!(exact_positive_f64_fraction(30.0), Some((30, 1)));
        let binding = native_node_binding()?;
        binding.validate()?;
        let descriptor = binding.descriptor();
        assert_eq!(descriptor.class_type, CLASS_TYPE);
        assert_eq!(
            descriptor
                .inputs
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            ["images", "fps", "audio", "bit_depth"]
        );
        assert_eq!(descriptor.outputs[0].name, "video");
        assert_eq!(descriptor.effect, NativeEffectClass::Pure);
        assert_eq!(descriptor.cache, NativeCachePolicy::InputIdentity);
        let schema = descriptor
            .source_schema
            .as_ref()
            .ok_or("missing source schema")?;
        assert_eq!(
            schema.inputs[1].default,
            Some(crate::NativeSchemaValue::FiniteDecimal {
                value: "30.0".to_owned()
            })
        );
        assert_eq!(
            schema.inputs[3].default,
            Some(crate::NativeSchemaValue::UnsignedInteger { value: 8 })
        );
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
        assert_eq!(
            video.frame_rate(),
            (1_054_475_631_502_295, 35_184_372_088_832)
        );
        assert_eq!(video.bit_depth(), NativeVideoBitDepth::Ten);
        assert_eq!(
            video
                .audio()
                .ok_or("audio was not retained")?
                .waveform()
                .storage_id(),
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

    #[test]
    fn get_video_components_descriptor_and_aliases_match_source() -> Result<(), Box<dyn Error>> {
        let fixture: Value = serde_json::from_str(COMPONENTS_FIXTURE)?;
        assert_eq!(fixture["feature_id"], COMPONENTS_FEATURE_ID);
        assert_eq!(
            fixture["source"]["definition_sha256"],
            "b2232b2c558b472226418d56910b9b763d8398612b31c4019d4881d0c4717b8d"
        );
        let binding = components_node_binding()?;
        binding.validate()?;
        assert_eq!(binding.feature_id(), COMPONENTS_FEATURE_ID);
        let descriptor = binding.descriptor();
        assert_eq!(descriptor.class_type, COMPONENTS_CLASS_TYPE);
        assert_eq!(descriptor.inputs.len(), 1);
        assert_eq!(descriptor.inputs[0].name, "video");
        assert_eq!(
            descriptor
                .outputs
                .iter()
                .map(|output| output.name.as_str())
                .collect::<Vec<_>>(),
            ["images", "audio", "fps", "bit_depth"]
        );
        assert_eq!(descriptor.effect, NativeEffectClass::Pure);
        assert_eq!(descriptor.cache, NativeCachePolicy::InputIdentity);
        let NativeNodeBinding::Executable { presentation, .. } = binding else {
            return Err("GetVideoComponents binding is not executable".into());
        };
        assert_eq!(
            presentation.description,
            "Extracts all components from a video: frames, audio, framerate, and bit depth."
        );
        assert_eq!(
            presentation.search_aliases,
            ["extract frames", "split video", "video to images", "demux"]
        );
        Ok(())
    }

    #[test]
    fn get_video_components_preserves_aliases_and_nullable_audio() -> Result<(), Box<dyn Error>> {
        let harness = Harness::new()?;
        let (image_handle, image_storage) = harness.image_handle()?;
        let image_payload = harness.store.payload(&image_handle)?;
        let NativeStoredPayload::Tensor(image_payload) = image_payload.as_ref() else {
            return Err("test IMAGE payload is unavailable".into());
        };
        let (audio_handle, audio_storage) = harness.audio_handle()?;
        let audio_payload = harness.store.payload(&audio_handle)?;
        let NativeStoredPayload::Audio(audio_payload) = audio_payload.as_ref() else {
            return Err("test AUDIO payload is unavailable".into());
        };
        let video = harness.video_handle(
            image_payload.tensor().clone(),
            Some(audio_payload.as_ref().clone()),
            (24_000, 1_001),
            NativeVideoBitDepth::Ten,
        )?;
        let outcome = futures::executor::block_on(components_executable()?.execute(
            harness.context(CancellationToken::default())?,
            BTreeMap::from([(
                "video".to_owned(),
                NativeValue::Handle {
                    value: video,
                },
            )]),
        ))?;
        let NativeNodeOutcome::Values {
            outputs,
            ui,
            effects,
        } = outcome
        else {
            return Err("GetVideoComponents did not return values".into());
        };
        assert!(ui.is_none());
        assert!(effects.is_empty());
        let [
            NativeValue::Handle { value: image },
            NativeValue::Handle { value: audio },
            NativeValue::Primitive {
                value: NativePrimitive::Number(fps),
            },
            NativeValue::Primitive {
                value: NativePrimitive::Integer(depth),
            },
        ] = outputs.as_slice()
        else {
            return Err("GetVideoComponents returned the wrong output shape".into());
        };
        let image = harness.store.payload(image)?;
        let NativeStoredPayload::Tensor(image) = image.as_ref() else {
            return Err("GetVideoComponents IMAGE output is invalid".into());
        };
        let audio = harness.store.payload(audio)?;
        let NativeStoredPayload::Audio(audio) = audio.as_ref() else {
            return Err("GetVideoComponents AUDIO output is invalid".into());
        };
        assert_eq!(image.tensor().storage_id(), image_storage);
        assert_eq!(audio.waveform().storage_id(), audio_storage);
        assert_eq!(*fps, 24_000.0 / 1_001.0);
        assert_eq!(*depth, 10);

        let video = harness.video_handle(
            image_payload.tensor().clone(),
            None,
            (30, 1),
            NativeVideoBitDepth::Eight,
        )?;
        let outcome = futures::executor::block_on(components_executable()?.execute(
            harness.context(CancellationToken::default())?,
            BTreeMap::from([(
                "video".to_owned(),
                NativeValue::Handle {
                    value: video,
                },
            )]),
        ))?;
        let NativeNodeOutcome::Values { outputs, .. } = outcome else {
            return Err("GetVideoComponents did not return values".into());
        };
        assert!(matches!(
            outputs.get(1),
            Some(NativeValue::Primitive {
                value: NativePrimitive::Null
            })
        ));
        assert!(matches!(
            outputs.get(2),
            Some(NativeValue::Primitive {
                value: NativePrimitive::Number(value)
            }) if *value == 30.0
        ));
        assert!(matches!(
            outputs.get(3),
            Some(NativeValue::Primitive {
                value: NativePrimitive::Integer(8)
            })
        ));
        Ok(())
    }

    #[test]
    fn get_video_components_normalizes_u8_and_cancels_atomically() -> Result<(), Box<dyn Error>> {
        let harness = Harness::new()?;
        let descriptor = TensorDescriptor::contiguous(
            vec![1, 1, 2, 3],
            DType::U8,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let cancellation = CancellationToken::default();
        let context = harness.backend.execution_context(
            StreamId::DEFAULT,
            harness.workspace.authorize_workspace(0)?,
            &cancellation,
        );
        let (frames, _) =
            harness
                .backend
                .upload_bytes(descriptor, &[0, 1, 127, 128, 254, 255], &context)?;
        let video = harness.video_handle(frames, None, (30, 1), NativeVideoBitDepth::Eight)?;
        let outcome = futures::executor::block_on(components_executable()?.execute(
            harness.context(CancellationToken::default())?,
            BTreeMap::from([(
                "video".to_owned(),
                NativeValue::Handle {
                    value: video.clone(),
                },
            )]),
        ))?;
        let NativeNodeOutcome::Values { outputs, .. } = outcome else {
            return Err("GetVideoComponents did not return values".into());
        };
        let Some(NativeValue::Handle { value: image }) = outputs.first() else {
            return Err("GetVideoComponents IMAGE output is absent".into());
        };
        let image = harness.store.payload(image)?;
        let NativeStoredPayload::Tensor(image) = image.as_ref() else {
            return Err("GetVideoComponents IMAGE output is invalid".into());
        };
        assert_eq!(
            image
                .image()
                .ok_or("IMAGE wrapper is absent")?
                .as_f32_slice()?,
            &[
                0.0,
                1.0 / 255.0,
                127.0 / 255.0,
                128.0 / 255.0,
                254.0 / 255.0,
                1.0
            ]
        );

        let baseline = harness.store.count()?;
        let cancellation = CancellationToken::default();
        cancellation.cancel();
        let error = futures::executor::block_on(components_executable()?.execute(
            harness.context(cancellation)?,
            BTreeMap::from([("video".to_owned(), NativeValue::Handle { value: video })]),
        ))
        .expect_err("cancelled GetVideoComponents must fail");
        assert_eq!(error.kind, NativeNodeFailureKind::Interrupted);
        assert_eq!(harness.store.count()?, baseline);
        Ok(())
    }

    #[test]
    fn frame_interpolate_descriptor_fixture_and_cache_match_source() -> Result<(), Box<dyn Error>> {
        let fixture: Value = serde_json::from_str(INTERPOLATION_FIXTURE)?;
        assert_eq!(fixture["feature_id"], INTERPOLATE_FEATURE_ID);
        assert_eq!(
            fixture["source"]["definition_sha256"],
            "e0b9dd6ec3b09e665bcc0f95d2b7a0209d9045ba9b96828e46f126e6914f049c"
        );
        let binding = interpolate_node_binding()?;
        binding.validate()?;
        assert_eq!(binding.feature_id(), INTERPOLATE_FEATURE_ID);
        let descriptor = binding.descriptor();
        assert_eq!(descriptor.class_type, INTERPOLATE_CLASS_TYPE);
        assert_eq!(
            descriptor
                .inputs
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            ["interp_model", "images", "multiplier"]
        );
        assert_eq!(descriptor.outputs.len(), 1);
        assert_eq!(descriptor.outputs[0].name, "images");
        assert_eq!(descriptor.effect, NativeEffectClass::Pure);
        assert_eq!(descriptor.cache, NativeCachePolicy::InputIdentity);
        let schema = descriptor
            .source_schema
            .as_ref()
            .ok_or("FrameInterpolate source schema is absent")?;
        assert_eq!(
            schema.inputs[2].default,
            Some(crate::NativeSchemaValue::UnsignedInteger { value: 2 })
        );
        assert_eq!(
            schema.inputs[2].minimum,
            Some(crate::NativeSchemaValue::UnsignedInteger { value: 2 })
        );
        assert_eq!(
            schema.inputs[2].maximum,
            Some(crate::NativeSchemaValue::UnsignedInteger { value: 16 })
        );
        let NativeNodeBinding::Executable { presentation, .. } = binding else {
            return Err("FrameInterpolate binding is not executable".into());
        };
        assert_eq!(presentation.display_name, "Run Frame Interpolation Model");
        assert_eq!(presentation.category, "video");
        assert!(presentation.description.is_empty());
        assert_eq!(presentation.output_names, ["images"]);
        assert_eq!(
            presentation.search_aliases,
            [
                "rife",
                "film",
                "frame interpolation",
                "slow motion",
                "interpolate frames",
                "vfi"
            ]
        );
        Ok(())
    }

    #[test]
    fn frame_interpolate_bypass_returns_exact_input_handle_without_compute()
    -> Result<(), Box<dyn Error>> {
        let harness = Harness::interpolation()?;
        let (model, retained_model) = harness.interpolation_model_handle()?;
        let model_digest = retained_model.semantic_state_digest_sha256().to_owned();
        let (images, storage_id) = harness.image_handle_with_shape(1, 2, 2, 4, &[0.25; 16])?;
        let baseline = harness.store.count()?;
        let outcome = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context_without_compute(CancellationToken::default())?,
            interpolation_inputs_for_test(
                model,
                images.clone(),
                NativePrimitive::UnsignedInteger(2),
            ),
        ))?;
        let NativeNodeOutcome::Values {
            outputs,
            ui,
            effects,
        } = outcome
        else {
            return Err("FrameInterpolate bypass did not return values".into());
        };
        assert!(ui.is_none());
        assert!(effects.is_empty());
        assert_eq!(
            outputs,
            [NativeValue::Handle {
                value: images.clone()
            }]
        );
        assert_eq!(harness.store.count()?, baseline);
        let payload = harness.store.payload(&images)?;
        let NativeStoredPayload::Tensor(payload) = payload.as_ref() else {
            return Err("FrameInterpolate bypass IMAGE is invalid".into());
        };
        assert_eq!(payload.tensor().storage_id(), storage_id);
        assert_eq!(retained_model.semantic_state_digest_sha256(), model_digest);
        Ok(())
    }

    #[test]
    fn frame_interpolate_executes_reduced_rife_and_publishes_canonical_image()
    -> Result<(), Box<dyn Error>> {
        let harness = Harness::interpolation()?;
        let (model, retained_model) = harness.interpolation_model_handle()?;
        let model_digest = retained_model.semantic_state_digest_sha256().to_owned();
        let frame_elements = 64 * 64 * 3;
        let mut values = vec![0.0; frame_elements];
        values.extend(std::iter::repeat_n(1.0, frame_elements));
        let (images, input_storage) = harness.image_handle_with_shape(2, 64, 64, 3, &values)?;
        let baseline = harness.store.count()?;
        let outcome = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context(CancellationToken::default())?,
            interpolation_inputs_for_test(model, images, NativePrimitive::UnsignedInteger(2)),
        ))?;
        let NativeNodeOutcome::Values {
            outputs,
            ui,
            effects,
        } = outcome
        else {
            return Err("FrameInterpolate did not return values".into());
        };
        assert!(ui.is_none());
        assert!(effects.is_empty());
        let [NativeValue::Handle { value: output }] = outputs.as_slice() else {
            return Err("FrameInterpolate returned the wrong output shape".into());
        };
        let payload = harness.store.payload(output)?;
        let NativeStoredPayload::Tensor(payload) = payload.as_ref() else {
            return Err("FrameInterpolate output is not an IMAGE payload".into());
        };
        let image = payload
            .image()
            .ok_or("FrameInterpolate output has no canonical ImageTensor")?;
        assert_eq!(image.dimensions()?, (3, 64, 64, 3));
        assert_eq!(image.tensor().descriptor().dtype(), DType::F32);
        assert_eq!(image.tensor().descriptor().device(), DeviceId::CPU);
        assert_eq!(image.tensor().descriptor().stream(), StreamId::DEFAULT);
        assert!(image.tensor().descriptor().is_contiguous()?);
        assert_ne!(image.tensor().storage_id(), input_storage);
        let output_values = image.as_f32_slice()?;
        assert_eq!(output_values.first(), Some(&0.0));
        assert_eq!(output_values.get(frame_elements), Some(&0.5));
        assert_eq!(output_values.last(), Some(&1.0));
        assert_eq!(harness.store.count()?, baseline + 1);
        assert_eq!(retained_model.semantic_state_digest_sha256(), model_digest);
        Ok(())
    }

    #[test]
    fn frame_interpolate_rejects_invalid_requests_and_cancellation_atomically()
    -> Result<(), Box<dyn Error>> {
        let harness = Harness::interpolation()?;
        let (model, _) = harness.interpolation_model_handle()?;
        let (images, _) = harness.image_handle_with_shape(2, 2, 2, 4, &[0.5; 32])?;
        let baseline = harness.store.count()?;
        let error = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context(CancellationToken::default())?,
            interpolation_inputs_for_test(
                model.clone(),
                images.clone(),
                NativePrimitive::UnsignedInteger(17),
            ),
        ))
        .expect_err("out-of-range multiplier must fail");
        assert_eq!(error.code, "invalid_node_inputs");
        let error = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context(CancellationToken::default())?,
            interpolation_inputs_for_test(
                model.clone(),
                images.clone(),
                NativePrimitive::UnsignedInteger(2),
            ),
        ))
        .expect_err("non-bypass RGBA input must fail");
        assert_eq!(error.code, "invalid_node_inputs");
        let cancellation = CancellationToken::default();
        cancellation.cancel();
        let error = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context(cancellation)?,
            interpolation_inputs_for_test(model, images, NativePrimitive::UnsignedInteger(2)),
        ))
        .expect_err("cancelled FrameInterpolate must fail");
        assert_eq!(error.kind, NativeNodeFailureKind::Interrupted);
        assert_eq!(harness.store.count()?, baseline);
        Ok(())
    }

    #[test]
    fn frame_interpolate_maps_exhaustion_rolls_back_late_cancellation_and_retries()
    -> Result<(), Box<dyn Error>> {
        let harness = Harness::interpolation()?;
        let (model, _) = harness.interpolation_model_handle()?;
        let frame_elements = 64 * 64 * 3;
        let mut values = vec![0.0; frame_elements];
        values.extend(std::iter::repeat_n(1.0, frame_elements));
        let (images, _) = harness.image_handle_with_shape(2, 64, 64, 3, &values)?;
        let baseline_handles = harness.store.count()?;
        let baseline_memory = harness.workspace.memory_snapshot().current_bytes;

        let error = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context_with_scratch(CancellationToken::default(), 1_024)?,
            interpolation_inputs_for_test(
                model.clone(),
                images.clone(),
                NativePrimitive::UnsignedInteger(2),
            ),
        ))
        .expect_err("constrained FrameInterpolate must fail");
        assert_eq!(error.code, "frame_interpolation_resource_exhausted");
        assert!(!error.retryable);
        assert_eq!(harness.store.count()?, baseline_handles);
        assert_eq!(
            harness.workspace.memory_snapshot().current_bytes,
            baseline_memory
        );

        let cancellation = CancellationToken::default();
        harness.store.cancel_after_next_publish();
        let error = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context(cancellation)?,
            interpolation_inputs_for_test(
                model.clone(),
                images.clone(),
                NativePrimitive::UnsignedInteger(2),
            ),
        ))
        .expect_err("late-cancelled FrameInterpolate must fail");
        assert_eq!(error.kind, NativeNodeFailureKind::Interrupted);
        assert_eq!(harness.store.count()?, baseline_handles);
        assert_eq!(
            harness.workspace.memory_snapshot().current_bytes,
            baseline_memory
        );

        let outcome = futures::executor::block_on(interpolation_executable()?.execute(
            harness.context(CancellationToken::default())?,
            interpolation_inputs_for_test(model, images, NativePrimitive::UnsignedInteger(2)),
        ))?;
        assert!(matches!(outcome, NativeNodeOutcome::Values { .. }));
        assert_eq!(harness.store.count()?, baseline_handles + 1);
        assert!(harness.workspace.memory_snapshot().current_bytes > baseline_memory);
        Ok(())
    }
}
