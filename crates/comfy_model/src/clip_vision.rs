use crate::native_ops::{
    GeluApproximation, NativeExecutionRequirements, NativeModule, NativeOpsError,
};
use comfy_tensor::{
    BinaryOperation, CpuBackend, DType, DeviceId, ExecutionContext, Layout, MemoryFormatReference,
    OperationSupport, ResizeCrop, ResizeMode, ResizeSpec, Scalar, ScalarSide, StreamId, Tensor,
    TensorBackend, TensorDescriptor, TensorError, UnaryOperation, ViewAccess,
    generated_comfy_operator_indirection_01::{
        OperatorIndirectionError, cast_to_with_context_exact_native,
    },
    generated_external_tensor_kernel_01::{
        ExternalTensorKernelPartOneError, normalize_with_context_exact_native,
        resize_with_context_exact_native,
    },
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_native_diffusion::{NativeDiffusionTensorError, quick_gelu as canonical_quick_gelu},
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, tensor_repeat_with_context_exact_native,
        torch_cat_with_context_exact_native, torch_reshape_with_context_exact_native,
        torch_stack_with_context_exact_native,
    },
    generated_shape_layout_transform_03::{
        ShapeLayoutTransformPartThreeError, tensor_permute_exact_native,
    },
    generated_storage_dtype_device_01::{
        StorageDTypeDeviceError, contiguous_with_context_exact_native,
    },
};
use thiserror::Error;

pub const CLIP_VISION_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/clip_model.py";
pub const CLIP_VISION_SOURCE_SHA256: &str =
    "08be993d86c3b494b58305fb868638b4b525bbe40abead89e9c94da021716845";
pub const CLIP_VISION_CATALOG_SYMBOLS: [&str; 9] = [
    "clip_preprocess",
    "siglip2_flex_calc_resolution",
    "siglip2_preprocess",
    "siglip2_pos_embed",
    "Siglip2Embeddings",
    "CLIPVisionEmbeddings",
    "CLIPVision",
    "LlavaProjector",
    "CLIPVisionModelProjection",
];

const DEFAULT_CLIP_MEAN: [f32; 3] = [0.481_454_66, 0.457_827_5, 0.408_210_73];
const DEFAULT_CLIP_STD: [f32; 3] = [0.268_629_54, 0.261_302_6, 0.275_777_1];
const DEFAULT_SIGLIP_MEAN: [f32; 3] = [0.5; 3];
const DEFAULT_SIGLIP_STD: [f32; 3] = [0.5; 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipVisionModelType {
    Clip,
    Siglip,
    Siglip2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipVisionActivation {
    QuickGelu,
    Gelu,
    GeluTanh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipVisionIntermediate {
    None,
    Layer(isize),
    All,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipVisionConfiguration {
    pub model_type: ClipVisionModelType,
    pub dtype: DType,
    pub device: DeviceId,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub attention_heads: usize,
    pub layer_count: usize,
    pub image_size: usize,
    pub patch_size: usize,
    pub num_channels: usize,
    pub max_num_patches: usize,
    pub activation: ClipVisionActivation,
    pub projection_dimension: Option<usize>,
    pub llava_projection_dimension: Option<usize>,
}

impl ClipVisionConfiguration {
    pub fn validate(&self) -> Result<(), ClipVisionError> {
        if self.device != DeviceId::CPU || self.dtype != DType::F32 {
            return Err(ClipVisionError::UnsupportedTarget {
                dtype: self.dtype,
                device: self.device,
            });
        }
        if self.hidden_size == 0
            || self.intermediate_size == 0
            || self.attention_heads == 0
            || self.layer_count == 0
            || self.patch_size == 0
            || self.num_channels != 3
            || !self.hidden_size.is_multiple_of(self.attention_heads)
            || self.projection_dimension == Some(0)
            || self.llava_projection_dimension == Some(0)
        {
            return Err(ClipVisionError::InvalidConfiguration(
                "hidden/intermediate/layer dimensions, heads, channels, or projections are invalid",
            ));
        }
        if self.model_type != ClipVisionModelType::Siglip2 && self.image_size == 0 {
            return Err(ClipVisionError::InvalidConfiguration(
                "fixed-resolution CLIP vision requires a nonzero image size",
            ));
        }
        if self.image_size > 0 && !self.image_size.is_multiple_of(self.patch_size) {
            return Err(ClipVisionError::InvalidConfiguration(
                "fixed-resolution CLIP vision image size must be divisible by patch size",
            ));
        }
        if self.model_type == ClipVisionModelType::Siglip2 && self.max_num_patches == 0 {
            return Err(ClipVisionError::InvalidConfiguration(
                "SigLIP2 requires a nonzero maximum patch count",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ClipVisionLayerWeights {
    pub layer_norm_1_weight: Tensor,
    pub layer_norm_1_bias: Tensor,
    pub query_weight: Tensor,
    pub query_bias: Tensor,
    pub key_weight: Tensor,
    pub key_bias: Tensor,
    pub value_weight: Tensor,
    pub value_bias: Tensor,
    pub output_weight: Tensor,
    pub output_bias: Tensor,
    pub layer_norm_2_weight: Tensor,
    pub layer_norm_2_bias: Tensor,
    pub feed_forward_1_weight: Tensor,
    pub feed_forward_1_bias: Tensor,
    pub feed_forward_2_weight: Tensor,
    pub feed_forward_2_bias: Tensor,
}

#[derive(Clone, Debug)]
pub struct ClipVisionWeights {
    pub patch_embedding_weight: Tensor,
    pub patch_embedding_bias: Option<Tensor>,
    pub class_embedding: Option<Tensor>,
    pub position_embedding: Tensor,
    pub pre_layer_norm_weight: Option<Tensor>,
    pub pre_layer_norm_bias: Option<Tensor>,
    pub layers: Vec<ClipVisionLayerWeights>,
    pub post_layer_norm_weight: Tensor,
    pub post_layer_norm_bias: Tensor,
    pub visual_projection_weight: Option<Tensor>,
    pub llava_linear_1_weight: Option<Tensor>,
    pub llava_linear_1_bias: Option<Tensor>,
    pub llava_linear_2_weight: Option<Tensor>,
    pub llava_linear_2_bias: Option<Tensor>,
}

#[derive(Clone, Debug)]
struct ClipVisionLayer {
    layer_norm_1: NativeModule,
    attention: NativeModule,
    layer_norm_2: NativeModule,
    feed_forward_1: NativeModule,
    activation: NativeModule,
    feed_forward_2: NativeModule,
    quick_gelu: bool,
}

#[derive(Clone, Debug)]
pub struct NativeClipVision {
    configuration: ClipVisionConfiguration,
    stream: StreamId,
    patch_embedding: NativeModule,
    class_embedding: Option<Tensor>,
    position_embedding: Tensor,
    pre_layer_norm: Option<NativeModule>,
    layers: Vec<ClipVisionLayer>,
    post_layer_norm: NativeModule,
    visual_projection: Option<NativeModule>,
    llava_linear_1: Option<NativeModule>,
    llava_activation: NativeModule,
    llava_linear_2: Option<NativeModule>,
}

#[derive(Clone, Debug)]
pub struct ClipVisionOutput {
    pub last_hidden_state: Tensor,
    pub intermediate: Option<Tensor>,
    pub image_embeds: Tensor,
    pub projected_intermediate: Option<Tensor>,
}

#[derive(Debug, Error)]
pub enum ClipVisionError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    Operator(#[from] OperatorIndirectionError),
    #[error(transparent)]
    ImageOperation(#[from] ExternalTensorKernelPartOneError),
    #[error(transparent)]
    Module(#[from] NativeOpsError),
    #[error(transparent)]
    NativeDiffusion(#[from] NativeDiffusionTensorError),
    #[error(transparent)]
    Indexing(#[from] IndexingMaskingPartOneError),
    #[error(transparent)]
    ShapeLayoutTwo(#[from] ShapeLayoutTransformPartTwoError),
    #[error(transparent)]
    ShapeLayoutThree(#[from] ShapeLayoutTransformPartThreeError),
    #[error(transparent)]
    StorageDTypeDevice(#[from] StorageDTypeDeviceError),
    #[error("CLIP vision configuration is invalid: {0}")]
    InvalidConfiguration(&'static str),
    #[error("CLIP vision input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("CLIP vision tensor {name} has shape {actual:?}; expected {expected:?}")]
    Shape {
        name: &'static str,
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error(
        "CLIP vision tensor {name} target mismatch: expected {expected_dtype:?} on {expected_device:?} stream {expected_stream:?}, got {actual_dtype:?} on {actual_device:?} stream {actual_stream:?}"
    )]
    TensorTarget {
        name: &'static str,
        expected_dtype: DType,
        actual_dtype: DType,
        expected_device: DeviceId,
        actual_device: DeviceId,
        expected_stream: StreamId,
        actual_stream: StreamId,
    },
    #[error("CLIP vision execution target {dtype:?} on {device:?} is unsupported")]
    UnsupportedTarget { dtype: DType, device: DeviceId },
    #[error("CLIP vision intermediate layer {requested} is outside {available} layers")]
    IntermediateOutOfRange { requested: isize, available: usize },
    #[error("Llava projection requires an intermediate CLIP vision tensor")]
    MissingLlavaIntermediate,
    #[error("CLIP vision shape arithmetic overflowed while computing {0}")]
    Overflow(&'static str),
    #[error("CLIP vision execution was cancelled")]
    Cancelled,
}

impl From<comfy_types::CancellationError> for ClipVisionError {
    fn from(_: comfy_types::CancellationError) -> Self {
        Self::Cancelled
    }
}

impl NativeClipVision {
    pub fn new(
        configuration: ClipVisionConfiguration,
        weights: ClipVisionWeights,
    ) -> Result<Self, ClipVisionError> {
        configuration.validate()?;
        if weights.layers.len() != configuration.layer_count {
            return Err(ClipVisionError::InvalidConfiguration(
                "configured layer count does not match the weight graph",
            ));
        }
        let stream = weights.position_embedding.descriptor().stream();
        validate_parameter_target(
            "position embedding",
            &weights.position_embedding,
            configuration.dtype,
            configuration.device,
            stream,
        )?;
        for (name, tensor) in [
            (
                "patch embedding weight",
                Some(&weights.patch_embedding_weight),
            ),
            (
                "patch embedding bias",
                weights.patch_embedding_bias.as_ref(),
            ),
            ("class embedding", weights.class_embedding.as_ref()),
            (
                "pre-layer norm weight",
                weights.pre_layer_norm_weight.as_ref(),
            ),
            ("pre-layer norm bias", weights.pre_layer_norm_bias.as_ref()),
            (
                "post-layer norm weight",
                Some(&weights.post_layer_norm_weight),
            ),
            ("post-layer norm bias", Some(&weights.post_layer_norm_bias)),
            (
                "visual projection",
                weights.visual_projection_weight.as_ref(),
            ),
            (
                "Llava linear 1 weight",
                weights.llava_linear_1_weight.as_ref(),
            ),
            ("Llava linear 1 bias", weights.llava_linear_1_bias.as_ref()),
            (
                "Llava linear 2 weight",
                weights.llava_linear_2_weight.as_ref(),
            ),
            ("Llava linear 2 bias", weights.llava_linear_2_bias.as_ref()),
        ] {
            if let Some(tensor) = tensor {
                validate_parameter_target(
                    name,
                    tensor,
                    configuration.dtype,
                    configuration.device,
                    stream,
                )?;
            }
        }

        let patch_width = configuration
            .num_channels
            .checked_mul(configuration.patch_size)
            .and_then(|value| value.checked_mul(configuration.patch_size))
            .ok_or(ClipVisionError::Overflow("patch width"))?;
        let has_patch_bias = weights.patch_embedding_bias.is_some();
        let mut patch_embedding = match configuration.model_type {
            ClipVisionModelType::Siglip2 => NativeModule::linear(
                "vision_model.embeddings.patch_embedding",
                patch_width,
                configuration.hidden_size,
                true,
                false,
            )?,
            ClipVisionModelType::Clip | ClipVisionModelType::Siglip => NativeModule::conv_2d(
                "vision_model.embeddings.patch_embedding",
                configuration.num_channels,
                configuration.hidden_size,
                [configuration.patch_size; 2],
                [configuration.patch_size; 2],
                [0; 2],
                [1; 2],
                1,
                configuration.model_type == ClipVisionModelType::Siglip,
                false,
            )?,
        };
        patch_embedding
            .load_dense_parameters(weights.patch_embedding_weight, weights.patch_embedding_bias)?;

        let expected_positions = match configuration.model_type {
            ClipVisionModelType::Clip => fixed_patch_count(&configuration)?
                .checked_add(1)
                .ok_or(ClipVisionError::Overflow("CLIP positions"))?,
            ClipVisionModelType::Siglip => fixed_patch_count(&configuration)?,
            ClipVisionModelType::Siglip2 => configuration.max_num_patches,
        };
        require_shape(
            "position embedding",
            &weights.position_embedding,
            &[
                usize_to_u64(expected_positions, "position count")?,
                usize_to_u64(configuration.hidden_size, "hidden size")?,
            ],
        )?;
        match configuration.model_type {
            ClipVisionModelType::Clip => {
                let class_embedding = weights.class_embedding.as_ref().ok_or(
                    ClipVisionError::InvalidConfiguration("CLIP requires a class embedding"),
                )?;
                require_shape(
                    "class embedding",
                    class_embedding,
                    &[usize_to_u64(configuration.hidden_size, "hidden size")?],
                )?;
                if has_patch_bias {
                    return Err(ClipVisionError::InvalidConfiguration(
                        "CLIP patch convolution must not have a bias",
                    ));
                }
            }
            ClipVisionModelType::Siglip => {
                if weights.class_embedding.is_some() || !has_patch_bias {
                    return Err(ClipVisionError::InvalidConfiguration(
                        "SigLIP requires a biased patch convolution and no class embedding",
                    ));
                }
            }
            ClipVisionModelType::Siglip2 => {
                if weights.class_embedding.is_some() || !has_patch_bias {
                    return Err(ClipVisionError::InvalidConfiguration(
                        "SigLIP2 requires a biased patch projection and no class embedding",
                    ));
                }
                let side = integer_square_root(expected_positions);
                if side.checked_mul(side) != Some(expected_positions) {
                    return Err(ClipVisionError::InvalidConfiguration(
                        "SigLIP2 position count must be a square",
                    ));
                }
            }
        }

        let pre_layer_norm = match configuration.model_type {
            ClipVisionModelType::Clip => Some(layer_norm_from_weights(
                "vision_model.pre_layrnorm",
                configuration.hidden_size,
                weights
                    .pre_layer_norm_weight
                    .ok_or(ClipVisionError::InvalidConfiguration(
                        "CLIP requires pre-layer norm weight",
                    ))?,
                weights
                    .pre_layer_norm_bias
                    .ok_or(ClipVisionError::InvalidConfiguration(
                        "CLIP requires pre-layer norm bias",
                    ))?,
            )?),
            ClipVisionModelType::Siglip | ClipVisionModelType::Siglip2 => {
                if weights.pre_layer_norm_weight.is_some() || weights.pre_layer_norm_bias.is_some()
                {
                    return Err(ClipVisionError::InvalidConfiguration(
                        "SigLIP vision must not configure a pre-layer norm",
                    ));
                }
                None
            }
        };

        let mut layers = Vec::new();
        layers
            .try_reserve_exact(configuration.layer_count)
            .map_err(|_| ClipVisionError::Overflow("vision layers"))?;
        for (index, weights) in weights.layers.into_iter().enumerate() {
            layers.push(build_layer(index, &configuration, weights, stream)?);
        }
        let post_layer_norm = layer_norm_from_weights(
            "vision_model.post_layernorm",
            configuration.hidden_size,
            weights.post_layer_norm_weight,
            weights.post_layer_norm_bias,
        )?;
        let visual_projection = optional_linear(
            "visual_projection",
            configuration.hidden_size,
            configuration.projection_dimension,
            weights.visual_projection_weight,
            None,
        )?;
        let llava_linear_1 = optional_linear(
            "multi_modal_projector.linear_1",
            configuration.hidden_size,
            configuration.llava_projection_dimension,
            weights.llava_linear_1_weight,
            weights.llava_linear_1_bias,
        )?;
        let llava_linear_2 = optional_linear(
            "multi_modal_projector.linear_2",
            configuration.llava_projection_dimension.unwrap_or(1),
            configuration.llava_projection_dimension,
            weights.llava_linear_2_weight,
            weights.llava_linear_2_bias,
        )?;
        let llava_activation =
            NativeModule::gelu("multi_modal_projector.gelu", GeluApproximation::None)?;

        Ok(Self {
            configuration,
            stream,
            patch_embedding,
            class_embedding: weights.class_embedding,
            position_embedding: weights.position_embedding,
            pre_layer_norm,
            layers,
            post_layer_norm,
            visual_projection,
            llava_linear_1,
            llava_activation,
            llava_linear_2,
        })
    }

    pub fn configuration(&self) -> &ClipVisionConfiguration {
        &self.configuration
    }

    pub fn dtype(&self) -> DType {
        self.configuration.dtype
    }

    pub fn device(&self) -> DeviceId {
        self.configuration.device
    }

    fn execution_requirements(&self) -> NativeExecutionRequirements {
        let mut requirements = clip_vision_tensor_requirements(false);
        for module in [
            Some(&self.patch_embedding),
            self.pre_layer_norm.as_ref(),
            Some(&self.post_layer_norm),
            self.visual_projection.as_ref(),
            self.llava_linear_1.as_ref(),
            Some(&self.llava_activation),
            self.llava_linear_2.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            requirements.extend(module.execution_requirements(DType::F32).iter());
        }
        for layer in &self.layers {
            for module in [
                &layer.layer_norm_1,
                &layer.attention,
                &layer.layer_norm_2,
                &layer.feed_forward_1,
                &layer.activation,
                &layer.feed_forward_2,
            ] {
                requirements.extend(module.execution_requirements(DType::F32).iter());
            }
            if layer.quick_gelu {
                requirements.extend([
                    OperationSupport::unary_input(
                        UnaryOperation::Exponential,
                        DType::F32,
                        Layout::Contiguous,
                    ),
                    OperationSupport::unary_output(
                        UnaryOperation::Exponential,
                        DType::F32,
                        Layout::Contiguous,
                    ),
                ]);
            }
        }
        requirements
    }

    pub fn preprocess(
        &self,
        backend: &CpuBackend,
        image: &Tensor,
        crop: bool,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, ClipVisionError> {
        clip_vision_tensor_requirements(true).admit_backend_target(
            backend,
            self.configuration.device,
            self.configuration.dtype,
            Layout::Contiguous,
            self.stream,
            context,
        )?;
        let image = match self.configuration.model_type {
            ClipVisionModelType::Clip => clip_preprocess_with_context(
                backend,
                image,
                self.configuration.image_size,
                DEFAULT_CLIP_MEAN,
                DEFAULT_CLIP_STD,
                crop,
                context,
            ),
            ClipVisionModelType::Siglip => clip_preprocess_with_context(
                backend,
                image,
                self.configuration.image_size,
                DEFAULT_SIGLIP_MEAN,
                DEFAULT_SIGLIP_STD,
                crop,
                context,
            ),
            ClipVisionModelType::Siglip2 => siglip2_preprocess_with_context(
                backend,
                image,
                self.configuration.image_size,
                self.configuration.patch_size,
                self.configuration.max_num_patches,
                DEFAULT_SIGLIP_MEAN,
                DEFAULT_SIGLIP_STD,
                crop,
                context,
            ),
        }?;
        if image.descriptor().dtype() == self.configuration.dtype
            && image.descriptor().device() == self.configuration.device
        {
            Ok(image)
        } else {
            Ok(cast_to_with_context_exact_native(
                backend,
                &image,
                self.configuration.dtype,
                self.configuration.device,
                false,
                false,
                context,
            )?)
        }
    }

    pub fn forward(
        &mut self,
        backend: &CpuBackend,
        pixel_values: &Tensor,
        intermediate: ClipVisionIntermediate,
        context: &ExecutionContext<'_>,
    ) -> Result<ClipVisionOutput, ClipVisionError> {
        self.execution_requirements().admit_backend_target(
            backend,
            self.configuration.device,
            self.configuration.dtype,
            Layout::Contiguous,
            self.stream,
            context,
        )?;
        require_execution_tensor(
            "pixel values",
            pixel_values,
            self.configuration.dtype,
            self.configuration.device,
            self.stream,
            context,
        )?;
        let shape = pixel_values.descriptor().shape();
        if shape.len() != 4 || shape[0] == 0 || shape[1] != 3 || shape[2] == 0 || shape[3] == 0 {
            return Err(ClipVisionError::InvalidInput(
                "pixel values must have nonempty [batch, 3, height, width] shape",
            ));
        }
        let patch_size = usize_to_u64(self.configuration.patch_size, "patch size")?;
        if !shape[2].is_multiple_of(patch_size) || !shape[3].is_multiple_of(patch_size) {
            return Err(ClipVisionError::InvalidInput(
                "pixel dimensions must be divisible by patch size",
            ));
        }
        if self.configuration.model_type != ClipVisionModelType::Siglip2
            && (shape[2] != usize_to_u64(self.configuration.image_size, "image size")?
                || shape[3] != usize_to_u64(self.configuration.image_size, "image size")?)
        {
            return Err(ClipVisionError::InvalidInput(
                "fixed-resolution CLIP vision input does not match configured image size",
            ));
        }
        let patch_count = shape[2]
            .checked_div(patch_size)
            .and_then(|height| {
                shape[3]
                    .checked_div(patch_size)
                    .and_then(|width| height.checked_mul(width))
            })
            .ok_or(ClipVisionError::Overflow("input patch count"))?;
        if self.configuration.model_type == ClipVisionModelType::Siglip2
            && patch_count
                > usize_to_u64(self.configuration.max_num_patches, "maximum patch count")?
        {
            return Err(ClipVisionError::InvalidInput(
                "SigLIP2 input exceeds the configured maximum patch count",
            ));
        }
        let selected_layer = resolve_intermediate(intermediate, self.layers.len())?;
        let mut hidden = self.patch_embeddings(backend, pixel_values, context)?;
        if let Some(pre_layer_norm) = self.pre_layer_norm.as_mut() {
            hidden = pre_layer_norm.forward_with_context(backend, &hidden, context)?;
        }
        let mut captured = None;
        let mut all = match intermediate {
            ClipVisionIntermediate::All => {
                let mut layers = Vec::new();
                layers
                    .try_reserve_exact(self.layers.len())
                    .map_err(|_| ClipVisionError::Overflow("all-layer capture"))?;
                Some(layers)
            }
            _ => None,
        };
        for (index, layer) in self.layers.iter_mut().enumerate() {
            context.check()?;
            hidden = layer.forward(backend, &hidden, context)?;
            if selected_layer == Some(index) {
                captured = Some(hidden.clone());
            }
            if let Some(all) = all.as_mut() {
                all.push(hidden.clone());
            }
        }
        let intermediate = match all {
            Some(all) => Some(stack_layers(backend, &all, context)?),
            None => captured,
        };
        let (last_hidden_state, pooled) = match self.configuration.model_type {
            ClipVisionModelType::Clip => {
                let pooled = select_token(backend, &hidden, 0, context)?;
                let pooled = self
                    .post_layer_norm
                    .forward_with_context(backend, &pooled, context)?;
                (hidden, pooled)
            }
            ClipVisionModelType::Siglip | ClipVisionModelType::Siglip2 => {
                let normalized = self
                    .post_layer_norm
                    .forward_with_context(backend, &hidden, context)?;
                (normalized.clone(), normalized)
            }
        };
        let image_embeds = match self.visual_projection.as_mut() {
            Some(projection) => projection.forward_with_context(backend, &pooled, context)?,
            None => pooled,
        };
        let projected_intermediate =
            match (self.llava_linear_1.as_mut(), self.llava_linear_2.as_mut()) {
                (Some(linear_1), Some(linear_2)) => {
                    let intermediate = intermediate
                        .as_ref()
                        .ok_or(ClipVisionError::MissingLlavaIntermediate)?;
                    if intermediate.descriptor().rank() != 3 {
                        return Err(ClipVisionError::InvalidInput(
                            "Llava projection requires one selected intermediate layer",
                        ));
                    }
                    let intermediate = drop_first_token(backend, intermediate, context)?;
                    let projected =
                        linear_1.forward_with_context(backend, &intermediate, context)?;
                    let projected = self
                        .llava_activation
                        .forward_with_context(backend, &projected, context)?;
                    Some(linear_2.forward_with_context(backend, &projected, context)?)
                }
                (None, None) => None,
                _ => {
                    return Err(ClipVisionError::InvalidConfiguration(
                        "Llava projection module graph is incomplete",
                    ));
                }
            };
        context.check()?;
        Ok(ClipVisionOutput {
            last_hidden_state,
            intermediate,
            image_embeds,
            projected_intermediate,
        })
    }

    fn patch_embeddings(
        &mut self,
        backend: &CpuBackend,
        pixel_values: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, ClipVisionError> {
        let shape = pixel_values.descriptor().shape();
        let batch = u64_to_usize(shape[0], "batch")?;
        let height = u64_to_usize(shape[2], "height")?;
        let width = u64_to_usize(shape[3], "width")?;
        let patch = self.configuration.patch_size;
        let grid_height = height / patch;
        let grid_width = width / patch;
        let embeds = match self.configuration.model_type {
            ClipVisionModelType::Siglip2 => {
                let patches =
                    patchify(backend, pixel_values, batch, height, width, patch, context)?;
                self.patch_embedding
                    .forward_with_context(backend, &patches, context)?
            }
            ClipVisionModelType::Clip | ClipVisionModelType::Siglip => {
                let convolution =
                    self.patch_embedding
                        .forward_with_context(backend, pixel_values, context)?;
                flatten_convolution_patches(
                    backend,
                    &convolution,
                    self.configuration.hidden_size,
                    context,
                )?
            }
        };
        match self.configuration.model_type {
            ClipVisionModelType::Siglip2 => siglip2_position_embedding(
                backend,
                &embeds,
                &self.position_embedding,
                grid_height,
                grid_width,
                context,
            ),
            ClipVisionModelType::Clip | ClipVisionModelType::Siglip => {
                let embeds = match &self.class_embedding {
                    Some(class_embedding) => {
                        prepend_class_embedding(backend, &embeds, class_embedding, context)?
                    }
                    None => embeds,
                };
                add_position_embedding(backend, &embeds, &self.position_embedding, context)
            }
        }
    }
}

impl ClipVisionLayer {
    fn forward(
        &mut self,
        backend: &CpuBackend,
        input: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, ClipVisionError> {
        let normalized = self
            .layer_norm_1
            .forward_with_context(backend, input, context)?;
        let attention = self.attention.forward_attention_with_context(
            backend,
            &normalized,
            &normalized,
            &normalized,
            context,
        )?;
        let hidden = add_exact(backend, input, &attention, context)?;
        let normalized = self
            .layer_norm_2
            .forward_with_context(backend, &hidden, context)?;
        let feed_forward =
            self.feed_forward_1
                .forward_with_context(backend, &normalized, context)?;
        let feed_forward = if self.quick_gelu {
            quick_gelu(backend, &feed_forward, context)?
        } else {
            self.activation
                .forward_with_context(backend, &feed_forward, context)?
        };
        let feed_forward =
            self.feed_forward_2
                .forward_with_context(backend, &feed_forward, context)?;
        add_exact(backend, &hidden, &feed_forward, context)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn clip_preprocess_with_context(
    backend: &CpuBackend,
    image: &Tensor,
    size: usize,
    mean: [f32; 3],
    standard_deviation: [f32; 3],
    crop: bool,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    clip_vision_tensor_requirements(true).admit_backend_target(
        backend,
        image.descriptor().device(),
        image.descriptor().dtype(),
        image.descriptor().layout(),
        image.descriptor().stream(),
        context,
    )?;
    if size == 0 {
        return Err(ClipVisionError::InvalidConfiguration(
            "CLIP preprocessing size must be nonzero",
        ));
    }
    validate_normalization(mean, standard_deviation)?;
    let (_, height, width, _) = bhwc_shape(image, context)?;
    let mut image = bhwc_rgb_to_nchw(backend, image, context)?;
    if height != size || width != size {
        let (resize_height, resize_width) = if crop {
            let scale = size as f64 / height.min(width) as f64;
            (
                (scale * height as f64).round_ties_even().max(1.0) as usize,
                (scale * width as f64).round_ties_even().max(1.0) as usize,
            )
        } else {
            (size, size)
        };
        let mut output_shape = image.descriptor().shape().to_vec();
        output_shape[2] = usize_to_u64(resize_height, "resize height")?;
        output_shape[3] = usize_to_u64(resize_width, "resize width")?;
        let output_descriptor =
            TensorDescriptor::contiguous(output_shape, DType::F32, DeviceId::CPU, context.stream)?;
        let (resized, event) = backend.resize(
            ResizeSpec {
                width: usize_to_u64(resize_width, "resize width")?,
                height: usize_to_u64(resize_height, "resize height")?,
                mode: ResizeMode::Bicubic,
                crop: ResizeCrop::Disabled,
                antialias: true,
                align_corners: false,
            },
            &image,
            output_descriptor,
            context,
        )?;
        backend.wait_event(event, context)?;
        image = if crop {
            let offset_y = (resize_height - size) / 2;
            let offset_x = (resize_width - size) / 2;
            let cropped = narrow_method_exact_native(
                &resized,
                2,
                usize_to_i64(offset_y, "crop y offset")?,
                usize_to_u64(size, "crop height")?,
                context.cancellation,
            )?;
            let cropped = narrow_method_exact_native(
                &cropped,
                3,
                usize_to_i64(offset_x, "crop x offset")?,
                usize_to_u64(size, "crop width")?,
                context.cancellation,
            )?;
            contiguous_with_context_exact_native(
                backend,
                &cropped,
                MemoryFormatReference::Layout(Layout::Contiguous),
                context,
            )?
        } else {
            resized
        };
    }
    quantize_and_normalize(backend, &image, mean, standard_deviation, context)
}

#[allow(clippy::too_many_arguments)]
pub fn siglip2_preprocess_with_context(
    backend: &CpuBackend,
    image: &Tensor,
    size: usize,
    patch_size: usize,
    max_num_patches: usize,
    mean: [f32; 3],
    standard_deviation: [f32; 3],
    crop: bool,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    clip_vision_tensor_requirements(true).admit_backend_target(
        backend,
        image.descriptor().device(),
        image.descriptor().dtype(),
        image.descriptor().layout(),
        image.descriptor().stream(),
        context,
    )?;
    if size > 0 {
        return clip_preprocess_with_context(
            backend,
            image,
            size,
            mean,
            standard_deviation,
            crop,
            context,
        );
    }
    if patch_size == 0 || max_num_patches == 0 {
        return Err(ClipVisionError::InvalidConfiguration(
            "flexible SigLIP2 preprocessing requires patch size and patch count",
        ));
    }
    validate_normalization(mean, standard_deviation)?;
    let (_, height, width, _) = bhwc_shape(image, context)?;
    let (height, width) = siglip2_flex_resolution(height, width, patch_size, max_num_patches)?;
    let image = bhwc_rgb_to_nchw(backend, image, context)?;
    let image = resize_with_context_exact_native(
        backend,
        &image,
        usize_to_u64(height, "SigLIP2 height")?,
        usize_to_u64(width, "SigLIP2 width")?,
        ResizeMode::Bilinear,
        true,
        context,
    )?;
    quantize_and_normalize(backend, &image, mean, standard_deviation, context)
}

pub fn siglip2_flex_resolution(
    original_height: usize,
    original_width: usize,
    patch_size: usize,
    max_num_patches: usize,
) -> Result<(usize, usize), ClipVisionError> {
    if original_height == 0 || original_width == 0 || patch_size == 0 || max_num_patches == 0 {
        return Err(ClipVisionError::InvalidInput(
            "flexible resolution dimensions and limits must be nonzero",
        ));
    }
    fn scale_dimension(size: usize, scale: f64, patch: usize) -> Result<usize, ClipVisionError> {
        let scaled = (size as f64 * scale / patch as f64).ceil() * patch as f64;
        if !scaled.is_finite() || scaled > usize::MAX as f64 {
            return Err(ClipVisionError::Overflow("flexible resolution"));
        }
        Ok((scaled as usize).max(patch))
    }
    let mut low = 1.0e-6_f64;
    let mut high = 100.0_f64;
    while high - low >= 1.0e-5 {
        let middle = (low + high) / 2.0;
        let height = scale_dimension(original_height, middle, patch_size)?;
        let width = scale_dimension(original_width, middle, patch_size)?;
        let patches = height
            .checked_div(patch_size)
            .and_then(|height| {
                width
                    .checked_div(patch_size)
                    .and_then(|width| height.checked_mul(width))
            })
            .ok_or(ClipVisionError::Overflow("flexible patch count"))?;
        if patches <= max_num_patches {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok((
        scale_dimension(original_height, low, patch_size)?,
        scale_dimension(original_width, low, patch_size)?,
    ))
}

fn build_layer(
    index: usize,
    configuration: &ClipVisionConfiguration,
    weights: ClipVisionLayerWeights,
    stream: StreamId,
) -> Result<ClipVisionLayer, ClipVisionError> {
    for (name, tensor) in layer_tensors(&weights) {
        validate_parameter_target(
            name,
            tensor,
            configuration.dtype,
            configuration.device,
            stream,
        )?;
    }
    let prefix = format!("vision_model.encoder.layers.{index}");
    let layer_norm_1 = layer_norm_from_weights(
        format!("{prefix}.layer_norm1"),
        configuration.hidden_size,
        weights.layer_norm_1_weight,
        weights.layer_norm_1_bias,
    )?;
    let mut attention = NativeModule::multihead_attention(
        format!("{prefix}.self_attn"),
        configuration.hidden_size,
        configuration.attention_heads,
        true,
        false,
    )?;
    for (name, weight, bias) in [
        ("q_proj", weights.query_weight, weights.query_bias),
        ("k_proj", weights.key_weight, weights.key_bias),
        ("v_proj", weights.value_weight, weights.value_bias),
        ("out_proj", weights.output_weight, weights.output_bias),
    ] {
        attention
            .child_mut(name)
            .ok_or(ClipVisionError::InvalidConfiguration(
                "canonical attention projection is missing",
            ))?
            .load_dense_parameters(weight, Some(bias))?;
    }
    let layer_norm_2 = layer_norm_from_weights(
        format!("{prefix}.layer_norm2"),
        configuration.hidden_size,
        weights.layer_norm_2_weight,
        weights.layer_norm_2_bias,
    )?;
    let mut feed_forward_1 = NativeModule::linear(
        format!("{prefix}.mlp.fc1"),
        configuration.hidden_size,
        configuration.intermediate_size,
        true,
        false,
    )?;
    feed_forward_1.load_dense_parameters(
        weights.feed_forward_1_weight,
        Some(weights.feed_forward_1_bias),
    )?;
    let mut feed_forward_2 = NativeModule::linear(
        format!("{prefix}.mlp.fc2"),
        configuration.intermediate_size,
        configuration.hidden_size,
        true,
        false,
    )?;
    feed_forward_2.load_dense_parameters(
        weights.feed_forward_2_weight,
        Some(weights.feed_forward_2_bias),
    )?;
    let approximation = match configuration.activation {
        ClipVisionActivation::Gelu => GeluApproximation::None,
        ClipVisionActivation::GeluTanh => GeluApproximation::Tanh,
        ClipVisionActivation::QuickGelu => GeluApproximation::None,
    };
    Ok(ClipVisionLayer {
        layer_norm_1,
        attention,
        layer_norm_2,
        feed_forward_1,
        activation: NativeModule::gelu(format!("{prefix}.mlp.activation"), approximation)?,
        feed_forward_2,
        quick_gelu: configuration.activation == ClipVisionActivation::QuickGelu,
    })
}

fn layer_tensors(weights: &ClipVisionLayerWeights) -> [(&'static str, &Tensor); 16] {
    [
        ("layer norm 1 weight", &weights.layer_norm_1_weight),
        ("layer norm 1 bias", &weights.layer_norm_1_bias),
        ("query weight", &weights.query_weight),
        ("query bias", &weights.query_bias),
        ("key weight", &weights.key_weight),
        ("key bias", &weights.key_bias),
        ("value weight", &weights.value_weight),
        ("value bias", &weights.value_bias),
        ("output weight", &weights.output_weight),
        ("output bias", &weights.output_bias),
        ("layer norm 2 weight", &weights.layer_norm_2_weight),
        ("layer norm 2 bias", &weights.layer_norm_2_bias),
        ("feed forward 1 weight", &weights.feed_forward_1_weight),
        ("feed forward 1 bias", &weights.feed_forward_1_bias),
        ("feed forward 2 weight", &weights.feed_forward_2_weight),
        ("feed forward 2 bias", &weights.feed_forward_2_bias),
    ]
}

fn layer_norm_from_weights(
    name: impl Into<String>,
    width: usize,
    weight: Tensor,
    bias: Tensor,
) -> Result<NativeModule, ClipVisionError> {
    let mut module = NativeModule::layer_norm(name, vec![width], 1.0e-5, true, true, false)?;
    module.load_dense_parameters(weight, Some(bias))?;
    Ok(module)
}

fn optional_linear(
    name: &'static str,
    input: usize,
    output: Option<usize>,
    weight: Option<Tensor>,
    bias: Option<Tensor>,
) -> Result<Option<NativeModule>, ClipVisionError> {
    match (output, weight, bias) {
        (None, None, None) => Ok(None),
        (Some(output), Some(weight), bias) => {
            let mut module = NativeModule::linear(name, input, output, bias.is_some(), false)?;
            module.load_dense_parameters(weight, bias)?;
            Ok(Some(module))
        }
        _ => Err(ClipVisionError::InvalidConfiguration(
            "projection configuration and weights do not match",
        )),
    }
}

fn fixed_patch_count(configuration: &ClipVisionConfiguration) -> Result<usize, ClipVisionError> {
    let side = configuration.image_size / configuration.patch_size;
    side.checked_mul(side)
        .ok_or(ClipVisionError::Overflow("fixed patch count"))
}

fn resolve_intermediate(
    requested: ClipVisionIntermediate,
    layers: usize,
) -> Result<Option<usize>, ClipVisionError> {
    let ClipVisionIntermediate::Layer(requested) = requested else {
        return Ok(None);
    };
    let resolved = if requested < 0 {
        isize::try_from(layers)
            .map_err(|_| ClipVisionError::Overflow("layer count"))?
            .checked_add(requested)
    } else {
        Some(requested)
    }
    .ok_or(ClipVisionError::IntermediateOutOfRange {
        requested,
        available: layers,
    })?;
    let resolved =
        usize::try_from(resolved).map_err(|_| ClipVisionError::IntermediateOutOfRange {
            requested,
            available: layers,
        })?;
    if resolved >= layers {
        return Err(ClipVisionError::IntermediateOutOfRange {
            requested,
            available: layers,
        });
    }
    Ok(Some(resolved))
}

fn bhwc_shape(
    image: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<(usize, usize, usize, usize), ClipVisionError> {
    require_execution_tensor(
        "input image",
        image,
        DType::F32,
        DeviceId::CPU,
        context.stream,
        context,
    )?;
    let [batch, height, width, channels] = image.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "preprocessing expects a [batch, height, width, channels] tensor",
        ));
    };
    if *batch == 0 || *height == 0 || *width == 0 || *channels < 3 {
        return Err(ClipVisionError::InvalidInput(
            "preprocessing requires nonempty images with at least three channels",
        ));
    }
    Ok((
        u64_to_usize(*batch, "batch")?,
        u64_to_usize(*height, "height")?,
        u64_to_usize(*width, "width")?,
        u64_to_usize(*channels, "channels")?,
    ))
}

fn bhwc_rgb_to_nchw(
    backend: &CpuBackend,
    image: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    bhwc_shape(image, context)?;
    let rgb = narrow_method_exact_native(image, 3, 0, 3, context.cancellation)?;
    let nchw = tensor_permute_exact_native(&rgb, &[0, 3, 1, 2], context.cancellation)?;
    Ok(contiguous_with_context_exact_native(
        backend,
        &nchw,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?)
}

fn quantize_and_normalize(
    backend: &CpuBackend,
    image: &Tensor,
    mean: [f32; 3],
    standard_deviation: [f32; 3],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let shape = image.descriptor().shape();
    if shape.len() != 4 || shape[1] != 3 {
        return Err(ClipVisionError::InvalidInput(
            "normalization requires NCHW RGB input",
        ));
    }
    let scaled = binary_scalar_exact(backend, image, BinaryOperation::Multiply, 255.0, context)?;
    let clamped = binary_scalar_exact(backend, &scaled, BinaryOperation::Maximum, 0.0, context)?;
    let clamped = binary_scalar_exact(backend, &clamped, BinaryOperation::Minimum, 255.0, context)?;
    let rounded = unary_exact(backend, &clamped, UnaryOperation::Round, context)?;
    let unit = binary_scalar_exact(backend, &rounded, BinaryOperation::Divide, 255.0, context)?;
    Ok(normalize_with_context_exact_native(
        backend,
        &unit,
        &mean,
        &standard_deviation,
        context,
    )?)
}

fn validate_normalization(
    mean: [f32; 3],
    standard_deviation: [f32; 3],
) -> Result<(), ClipVisionError> {
    if mean.iter().any(|value| !value.is_finite())
        || standard_deviation
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(ClipVisionError::InvalidConfiguration(
            "normalization statistics must be finite and standard deviations positive",
        ));
    }
    Ok(())
}

fn patchify(
    backend: &CpuBackend,
    image: &Tensor,
    batch: usize,
    height: usize,
    width: usize,
    patch: usize,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let grid_height = height / patch;
    let grid_width = width / patch;
    let patch_width = 3_usize
        .checked_mul(patch)
        .and_then(|value| value.checked_mul(patch))
        .ok_or(ClipVisionError::Overflow("patch width"))?;
    let patch_count = grid_height
        .checked_mul(grid_width)
        .ok_or(ClipVisionError::Overflow("patch count"))?;
    let image = contiguous_with_context_exact_native(
        backend,
        image,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    let channel_stride = height
        .checked_mul(width)
        .ok_or(ClipVisionError::Overflow("patch channel stride"))?;
    let batch_stride = channel_stride
        .checked_mul(3)
        .ok_or(ClipVisionError::Overflow("patch batch stride"))?;
    let view = image.view(
        TensorDescriptor::new_strided(
            vec![
                usize_to_u64(batch, "batch")?,
                usize_to_u64(grid_height, "grid height")?,
                usize_to_u64(grid_width, "grid width")?,
                usize_to_u64(patch, "patch height")?,
                usize_to_u64(patch, "patch width")?,
                3,
            ],
            vec![
                usize_to_i64(batch_stride, "patch batch stride")?,
                usize_to_i64(
                    patch
                        .checked_mul(width)
                        .ok_or(ClipVisionError::Overflow("patch row stride"))?,
                    "patch row stride",
                )?,
                usize_to_i64(patch, "patch column stride")?,
                usize_to_i64(width, "patch offset row stride")?,
                1,
                usize_to_i64(channel_stride, "patch channel stride")?,
            ],
            0,
            DType::F32,
            Layout::Strided,
            DeviceId::CPU,
            context.stream,
        )?,
        ViewAccess::ReadOnly,
    )?;
    let patches = contiguous_with_context_exact_native(
        backend,
        &view,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    Ok(torch_reshape_with_context_exact_native(
        backend,
        &patches,
        &[
            usize_to_i64(batch, "batch")?,
            usize_to_i64(patch_count, "patch count")?,
            usize_to_i64(patch_width, "patch width")?,
        ],
        context,
    )?)
}

fn flatten_convolution_patches(
    backend: &CpuBackend,
    convolution: &Tensor,
    hidden: usize,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let [batch, channels, height, width] = convolution.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "patch convolution did not produce NCHW output",
        ));
    };
    if *channels != usize_to_u64(hidden, "hidden size")? {
        return Err(ClipVisionError::InvalidInput(
            "patch convolution output width is invalid",
        ));
    }
    let patches = tensor_permute_exact_native(convolution, &[0, 2, 3, 1], context.cancellation)?;
    let patches = contiguous_with_context_exact_native(
        backend,
        &patches,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    Ok(torch_reshape_with_context_exact_native(
        backend,
        &patches,
        &[
            u64_to_i64(*batch, "batch")?,
            u64_to_i64(
                height
                    .checked_mul(*width)
                    .ok_or(ClipVisionError::Overflow("patch count"))?,
                "patch count",
            )?,
            usize_to_i64(hidden, "hidden size")?,
        ],
        context,
    )?)
}

fn prepend_class_embedding(
    backend: &CpuBackend,
    embeds: &Tensor,
    class_embedding: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let [batch, _tokens, hidden] = embeds.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "patch embeddings must have rank three",
        ));
    };
    require_shape("class embedding", class_embedding, &[*hidden])?;
    let class = torch_reshape_with_context_exact_native(
        backend,
        class_embedding,
        &[1, 1, u64_to_i64(*hidden, "hidden")?],
        context,
    )?;
    let class = tensor_repeat_with_context_exact_native(
        backend,
        &class,
        &[u64_to_i64(*batch, "batch")?, 1, 1],
        context,
    )?;
    Ok(torch_cat_with_context_exact_native(
        backend,
        &[class, embeds.clone()],
        1,
        context,
    )?)
}

fn add_position_embedding(
    backend: &CpuBackend,
    embeds: &Tensor,
    position_embedding: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let [batch, tokens, hidden] = embeds.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "vision embeddings must have rank three",
        ));
    };
    require_shape(
        "position embedding",
        position_embedding,
        &[*tokens, *hidden],
    )?;
    let positions = torch_reshape_with_context_exact_native(
        backend,
        position_embedding,
        &[
            1,
            u64_to_i64(*tokens, "tokens")?,
            u64_to_i64(*hidden, "hidden")?,
        ],
        context,
    )?;
    let positions = tensor_repeat_with_context_exact_native(
        backend,
        &positions,
        &[u64_to_i64(*batch, "batch")?, 1, 1],
        context,
    )?;
    add_exact(backend, embeds, &positions, context)
}

fn siglip2_position_embedding(
    backend: &CpuBackend,
    embeds: &Tensor,
    position_embedding: &Tensor,
    grid_height: usize,
    grid_width: usize,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let [batch, tokens, hidden] = embeds.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "SigLIP2 embeddings must have rank three",
        ));
    };
    let grid_tokens = grid_height
        .checked_mul(grid_width)
        .ok_or(ClipVisionError::Overflow("grid tokens"))?;
    if *tokens != usize_to_u64(grid_tokens, "grid tokens")? {
        return Err(ClipVisionError::InvalidInput(
            "SigLIP2 embeddings do not match the patch grid",
        ));
    }
    let [position_count, position_hidden] = position_embedding.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "SigLIP2 position embedding must have rank two",
        ));
    };
    if position_hidden != hidden {
        return Err(ClipVisionError::Shape {
            name: "SigLIP2 position embedding",
            expected: vec![*position_count, *hidden],
            actual: position_embedding.descriptor().shape().to_vec(),
        });
    }
    let side = integer_square_root(u64_to_usize(*position_count, "position count")?);
    if side.checked_mul(side) != Some(u64_to_usize(*position_count, "position count")?) {
        return Err(ClipVisionError::InvalidConfiguration(
            "SigLIP2 position embedding count must be square",
        ));
    }
    let positions = torch_reshape_with_context_exact_native(
        backend,
        position_embedding,
        &[
            usize_to_i64(side, "position side")?,
            usize_to_i64(side, "position side")?,
            u64_to_i64(*hidden, "hidden")?,
        ],
        context,
    )?;
    let positions = tensor_permute_exact_native(&positions, &[2, 0, 1], context.cancellation)?;
    let positions = contiguous_with_context_exact_native(
        backend,
        &positions,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    let nchw = torch_reshape_with_context_exact_native(
        backend,
        &positions,
        &[
            1,
            u64_to_i64(*hidden, "hidden")?,
            usize_to_i64(side, "position side")?,
            usize_to_i64(side, "position side")?,
        ],
        context,
    )?;
    let resized = resize_with_context_exact_native(
        backend,
        &nchw,
        usize_to_u64(grid_height, "grid height")?,
        usize_to_u64(grid_width, "grid width")?,
        ResizeMode::Bilinear,
        true,
        context,
    )?;
    let positions = tensor_permute_exact_native(&resized, &[0, 2, 3, 1], context.cancellation)?;
    let positions = contiguous_with_context_exact_native(
        backend,
        &positions,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    let positions = torch_reshape_with_context_exact_native(
        backend,
        &positions,
        &[
            1,
            usize_to_i64(
                grid_height
                    .checked_mul(grid_width)
                    .ok_or(ClipVisionError::Overflow("grid positions"))?,
                "grid positions",
            )?,
            u64_to_i64(*hidden, "hidden")?,
        ],
        context,
    )?;
    let positions = tensor_repeat_with_context_exact_native(
        backend,
        &positions,
        &[u64_to_i64(*batch, "batch")?, 1, 1],
        context,
    )?;
    add_exact(backend, embeds, &positions, context)
}

fn select_token(
    backend: &CpuBackend,
    input: &Tensor,
    token: usize,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let [batch, tokens, hidden] = input.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "token selection requires rank-three input",
        ));
    };
    if usize_to_u64(token, "selected token")? >= *tokens {
        return Err(ClipVisionError::InvalidInput(
            "selected token is out of range",
        ));
    }
    let selected = narrow_method_exact_native(
        input,
        1,
        usize_to_i64(token, "selected token")?,
        1,
        context.cancellation,
    )?;
    Ok(torch_reshape_with_context_exact_native(
        backend,
        &selected,
        &[u64_to_i64(*batch, "batch")?, u64_to_i64(*hidden, "hidden")?],
        context,
    )?)
}

fn drop_first_token(
    _backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let [_batch, tokens, _hidden] = input.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "token slicing requires rank-three input",
        ));
    };
    if *tokens <= 1 {
        return Err(ClipVisionError::InvalidInput(
            "Llava projection requires a class token and at least one patch token",
        ));
    }
    Ok(narrow_method_exact_native(
        input,
        1,
        1,
        tokens - 1,
        context.cancellation,
    )?)
}

fn stack_layers(
    backend: &CpuBackend,
    layers: &[Tensor],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let first = layers.first().ok_or(ClipVisionError::InvalidConfiguration(
        "all-layer capture requires at least one layer",
    ))?;
    let [_batch, _tokens, _hidden] = first.descriptor().shape() else {
        return Err(ClipVisionError::InvalidInput(
            "captured layer must have rank three",
        ));
    };
    for layer in layers {
        if layer.descriptor().shape() != first.descriptor().shape() {
            return Err(ClipVisionError::InvalidInput(
                "captured layer shapes differ",
            ));
        }
    }
    Ok(torch_stack_with_context_exact_native(
        backend, layers, 1, context,
    )?)
}

fn quick_gelu(
    backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    Ok(canonical_quick_gelu(backend, input, context)?)
}

fn clip_vision_tensor_requirements(preprocess: bool) -> NativeExecutionRequirements {
    let mut requirements = NativeExecutionRequirements::new();
    requirements.extend([
        OperationSupport::allocation(DType::F32, Layout::Contiguous),
        OperationSupport::copy_input(DType::F32, Layout::Contiguous),
        OperationSupport::copy_output(DType::F32, Layout::Contiguous),
        OperationSupport::select_input(DType::F32, Layout::Contiguous),
        OperationSupport::select_output(DType::F32, Layout::Contiguous),
        OperationSupport::narrow_input(DType::F32, Layout::Contiguous),
        OperationSupport::narrow_output(DType::F32, Layout::Contiguous),
        OperationSupport::record_event(),
        OperationSupport::wait_event(),
    ]);
    for operation in [
        BinaryOperation::Add,
        BinaryOperation::Subtract,
        BinaryOperation::Multiply,
        BinaryOperation::Divide,
        BinaryOperation::Minimum,
        BinaryOperation::Maximum,
    ] {
        requirements.extend([
            OperationSupport::binary_input(operation, DType::F32, Layout::Contiguous),
            OperationSupport::binary_output(operation, DType::F32, Layout::Contiguous),
            OperationSupport::binary_scalar_input(operation, DType::F32, Layout::Contiguous),
            OperationSupport::binary_scalar_output(operation, DType::F32, Layout::Contiguous),
        ]);
    }
    requirements.extend([
        OperationSupport::unary_input(UnaryOperation::Round, DType::F32, Layout::Contiguous),
        OperationSupport::unary_output(UnaryOperation::Round, DType::F32, Layout::Contiguous),
        OperationSupport::resize_input(ResizeMode::Bilinear, DType::F32, Layout::Contiguous),
        OperationSupport::resize_output(ResizeMode::Bilinear, DType::F32, Layout::Contiguous),
    ]);
    if preprocess {
        requirements.extend([
            OperationSupport::resize_input(ResizeMode::Bicubic, DType::F32, Layout::Contiguous),
            OperationSupport::resize_output(ResizeMode::Bicubic, DType::F32, Layout::Contiguous),
        ]);
    }
    requirements
}

fn add_exact(
    backend: &CpuBackend,
    left: &Tensor,
    right: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    if left.descriptor().shape() != right.descriptor().shape()
        || left.descriptor().dtype() != right.descriptor().dtype()
        || left.descriptor().device() != right.descriptor().device()
        || left.descriptor().stream() != right.descriptor().stream()
    {
        return Err(ClipVisionError::InvalidInput(
            "added tensors must have identical shape, dtype, device, and stream",
        ));
    }
    let left = contiguous_with_context_exact_native(
        backend,
        left,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    let right = contiguous_with_context_exact_native(
        backend,
        right,
        MemoryFormatReference::Layout(Layout::Contiguous),
        context,
    )?;
    let (output, event) = backend.binary(
        BinaryOperation::Add,
        &left,
        &right,
        left.descriptor().clone(),
        context,
    )?;
    backend.wait_event(event, context)?;
    Ok(output)
}

fn binary_scalar_exact(
    backend: &CpuBackend,
    input: &Tensor,
    operation: BinaryOperation,
    scalar: f64,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let (output, event) = backend.binary_scalar(
        operation,
        input,
        Scalar::Float(scalar),
        ScalarSide::Right,
        input.descriptor().clone(),
        context,
    )?;
    backend.wait_event(event, context)?;
    Ok(output)
}

fn unary_exact(
    backend: &CpuBackend,
    input: &Tensor,
    operation: UnaryOperation,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ClipVisionError> {
    let (output, event) = backend.unary(operation, input, input.descriptor().clone(), context)?;
    backend.wait_event(event, context)?;
    Ok(output)
}

fn validate_parameter_target(
    name: &'static str,
    tensor: &Tensor,
    dtype: DType,
    device: DeviceId,
    stream: StreamId,
) -> Result<(), ClipVisionError> {
    let descriptor = tensor.descriptor();
    if descriptor.dtype() != dtype || descriptor.device() != device || descriptor.stream() != stream
    {
        return Err(ClipVisionError::TensorTarget {
            name,
            expected_dtype: dtype,
            actual_dtype: descriptor.dtype(),
            expected_device: device,
            actual_device: descriptor.device(),
            expected_stream: stream,
            actual_stream: descriptor.stream(),
        });
    }
    Ok(())
}

fn require_execution_tensor(
    name: &'static str,
    tensor: &Tensor,
    dtype: DType,
    device: DeviceId,
    stream: StreamId,
    context: &ExecutionContext<'_>,
) -> Result<(), ClipVisionError> {
    context.check()?;
    let descriptor = tensor.descriptor();
    if descriptor.dtype() != dtype
        || descriptor.device() != device
        || descriptor.stream() != stream
        || stream != context.stream
    {
        return Err(ClipVisionError::TensorTarget {
            name,
            expected_dtype: dtype,
            actual_dtype: descriptor.dtype(),
            expected_device: device,
            actual_device: descriptor.device(),
            expected_stream: stream,
            actual_stream: descriptor.stream(),
        });
    }
    Ok(())
}

fn require_shape(
    name: &'static str,
    tensor: &Tensor,
    expected: &[u64],
) -> Result<(), ClipVisionError> {
    if tensor.descriptor().shape() != expected {
        return Err(ClipVisionError::Shape {
            name,
            expected: expected.to_vec(),
            actual: tensor.descriptor().shape().to_vec(),
        });
    }
    Ok(())
}

fn integer_square_root(value: usize) -> usize {
    if value < 2 {
        return value;
    }
    let mut low = 1_usize;
    let mut high = value / 2 + 1;
    let mut root = 1_usize;
    while low <= high {
        let middle = low + (high - low) / 2;
        if middle <= value / middle {
            root = middle;
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    root
}

fn usize_to_u64(value: usize, operation: &'static str) -> Result<u64, ClipVisionError> {
    u64::try_from(value).map_err(|_| ClipVisionError::Overflow(operation))
}

fn usize_to_i64(value: usize, operation: &'static str) -> Result<i64, ClipVisionError> {
    i64::try_from(value).map_err(|_| ClipVisionError::Overflow(operation))
}

fn u64_to_i64(value: u64, operation: &'static str) -> Result<i64, ClipVisionError> {
    i64::try_from(value).map_err(|_| ClipVisionError::Overflow(operation))
}

fn u64_to_usize(value: u64, operation: &'static str) -> Result<usize, ClipVisionError> {
    usize::try_from(value).map_err(|_| ClipVisionError::Overflow(operation))
}
