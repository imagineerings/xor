use crate::{
    MODEL_STATE_TRANSFORM_PLAN_SCHEMA_VERSION, MemoryEstimatorDescriptor,
    ModelClipConfigurationFactDefinition, ModelClipModelInvocationDefinition,
    ModelClipTargetCandidateDefinition, ModelClipTargetDefinition, ModelClipTargetSelector,
    ModelDetectionRule, ModelFamilyComponent, ModelFamilyComponentStateSchema,
    ModelFamilyDefinition, ModelFamilyRegistration, ModelFamilyStatePlanCase,
    ModelFamilyStatePlanSelector, ModelForwardOperation, ModelForwardStep, ModelLayoutSignature,
    ModelSourceConfigurationRule, ModelStateLayout, ModelStateTransformPlanDefinition,
    ModelWeightRule,
};
use comfy_tensor::DType;
use comfy_types::DeviceKind;

pub const MODEL_FAMILY_IDENTIFIER: &str = "Boogu";
pub const MODEL_FAMILY_FEATURE_ID: &str = "COMFY-MODEL-0065";
pub const MODEL_FAMILY_FIXTURE: &str = "boogu-comfy-model-0065";
pub const MODEL_FAMILY_SOURCE_ORDINAL: u16 = 76;
pub const MODEL_FAMILY_SAMPLING_MULTIPLIER: f64 = 1.0;
pub const MODEL_FAMILY_SAMPLING_SHIFT: f64 = 3.16;
pub const MODEL_FAMILY_MEMORY_USAGE_FACTOR: f64 = 2.15;

const CLIP_CONFIGURATION: &[ModelClipConfigurationFactDefinition] =
    &[ModelClipConfigurationFactDefinition::Expand {
        source: "comfy.text_encoders.hunyuan_video.llama_detect",
    }];

const CLIP_CANDIDATES: &[ModelClipTargetCandidateDefinition] =
    &[ModelClipTargetCandidateDefinition {
        tokenizer: "comfy.text_encoders.boogu.BooguTokenizer",
        clip_model: "comfy.text_encoders.boogu.te",
        invocation: ModelClipModelInvocationDefinition::Factory {
            configuration: CLIP_CONFIGURATION,
        },
    }];

const CLIP_TARGET: ModelClipTargetDefinition = ModelClipTargetDefinition {
    candidates: CLIP_CANDIDATES,
    dynamic_selection: false,
};

const COMPONENTS: &[ModelFamilyComponent] = &[
    ModelFamilyComponent {
        identifier: "model",
        role: "image diffusion transformer",
        required: true,
    },
    ModelFamilyComponent {
        identifier: "runtime_conditioning",
        role: "generated reference conditioning",
        required: true,
    },
    ModelFamilyComponent {
        identifier: "vae",
        role: "latent decoder",
        required: false,
    },
    ModelFamilyComponent {
        identifier: "text_encoder",
        role: "multimodal conditioning",
        required: false,
    },
];

const DETECTION_RULES: &[ModelDetectionRule] = &[ModelDetectionRule::Metadata {
    key: "image_model",
    value: "boogu",
    score: 1_000,
}];

const WEIGHT_RULES: &[ModelWeightRule] = &[ModelWeightRule {
    source_prefix: "model.diffusion_model.",
    target_prefix: "native.",
    required: true,
}];

const REQUIRED_KEYS: &[&str] = &[
    "native.x_embedder.weight",
    "native.x_embedder.bias",
    "native.double_stream_layers.0.img_instruct_attn.processor.img_to_q.weight",
    "native.norm_out.linear_2.weight",
    "native.norm_out.linear_2.bias",
];

const OPTIONAL_KEYS: &[&str] = &[
    "native.time_caption_embed.caption_embedder.0.weight",
    "native.single_stream_layers.0.attn.to_q.weight",
    "native.noise_refiner.0.attn.to_q.weight",
];
const SUPPORTED_DTYPES: &[DType] = &[DType::Bf16, DType::F32];
const SUPPORTED_DEVICES: &[DeviceKind] = &[DeviceKind::Cpu];

const FORWARD_PROGRAM: &[ModelForwardStep] = &[
    ModelForwardStep {
        checkpoint: "patch_embedding",
        operation: ModelForwardOperation::Linear {
            weight: "native.x_embedder.weight",
            bias: Some("native.x_embedder.bias"),
            input_features: 2,
            output_features: 2,
        },
    },
    ModelForwardStep {
        checkpoint: "double_stream_query",
        operation: ModelForwardOperation::Linear {
            weight: "native.double_stream_layers.0.img_instruct_attn.processor.img_to_q.weight",
            bias: None,
            input_features: 2,
            output_features: 2,
        },
    },
    ModelForwardStep {
        checkpoint: "double_stream_activation",
        operation: ModelForwardOperation::Silu,
    },
    ModelForwardStep {
        checkpoint: "output_projection",
        operation: ModelForwardOperation::Linear {
            weight: "native.norm_out.linear_2.weight",
            bias: Some("native.norm_out.linear_2.bias"),
            input_features: 2,
            output_features: 2,
        },
    },
    ModelForwardStep {
        checkpoint: "negated_output",
        operation: ModelForwardOperation::MultiplyScalar(-1.0),
    },
];

pub const MODEL_FAMILY: ModelFamilyDefinition = ModelFamilyDefinition {
    feature_id: MODEL_FAMILY_FEATURE_ID,
    identifier: MODEL_FAMILY_IDENTIFIER,
    architecture_version: "boogu-transformer-2d-v1",
    latent_feature_id: "COMFY-MODEL-0029",
    latent_identifier: "Flux",
    clip_target: &CLIP_TARGET,
    components: COMPONENTS,
    detection_rules: DETECTION_RULES,
    weight_rules: WEIGHT_RULES,
    required_keys: REQUIRED_KEYS,
    optional_keys: OPTIONAL_KEYS,
    supported_dtypes: SUPPORTED_DTYPES,
    supported_devices: SUPPORTED_DEVICES,
    memory_estimator: MemoryEstimatorDescriptor {
        fixed_bytes: 0,
        bytes_per_parameter: 4,
        activation_bytes_per_element: 9,
    },
    forward_program: FORWARD_PROGRAM,
};

const SOURCE_CONFIGURATION: &[ModelSourceConfigurationRule] =
    &[ModelSourceConfigurationRule::Metadata {
        key: "image_model",
        value: "boogu",
    }];

const NATIVE_STATE_PLAN: ModelStateTransformPlanDefinition = ModelStateTransformPlanDefinition {
    schema_version: MODEL_STATE_TRANSFORM_PLAN_SCHEMA_VERSION,
    encoded_plan: r#"{
        "operations": [
            {"Move":{"selector":{"predicate":{"Prefix":"model.diffusion_model."},"minimum_matches":1,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"model.diffusion_model.","to":"native."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"vae."},"minimum_matches":0,"maximum_matches":16384},"rewrite":"Identity","component":"vae"}},
            {"Move":{"selector":{"predicate":{"Prefix":"text_encoders."},"minimum_matches":0,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"text_encoders.","to":"text_encoder."}},"component":"text_encoder"}},
            {"Generate":{"shape":[{"Literal":1}],"fill":{"unsigned":0},"dtype":"i64","output":{"component":"runtime_conditioning","key":"reference_latent_count"}}}
        ],
        "unmatched":"Reject"
    }"#,
};

const DIFFUSERS_STATE_PLAN: ModelStateTransformPlanDefinition = ModelStateTransformPlanDefinition {
    schema_version: MODEL_STATE_TRANSFORM_PLAN_SCHEMA_VERSION,
    encoded_plan: r#"{
        "operations": [
            {"Move":{"selector":{"predicate":{"Prefix":"x_embedder."},"minimum_matches":1,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"x_embedder.","to":"native.x_embedder."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"double_stream_layers."},"minimum_matches":1,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"double_stream_layers.","to":"native.double_stream_layers."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"norm_out."},"minimum_matches":1,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"norm_out.","to":"native.norm_out."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"time_caption_embed."},"minimum_matches":0,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"time_caption_embed.","to":"native.time_caption_embed."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"single_stream_layers."},"minimum_matches":0,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"single_stream_layers.","to":"native.single_stream_layers."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"noise_refiner."},"minimum_matches":0,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"noise_refiner.","to":"native.noise_refiner."}},"component":"model"}},
            {"Move":{"selector":{"predicate":{"Prefix":"vae."},"minimum_matches":0,"maximum_matches":16384},"rewrite":"Identity","component":"vae"}},
            {"Move":{"selector":{"predicate":{"Prefix":"text_encoders."},"minimum_matches":0,"maximum_matches":16384},"rewrite":{"Prefix":{"from":"text_encoders.","to":"text_encoder."}},"component":"text_encoder"}},
            {"Generate":{"shape":[{"Literal":1}],"fill":{"unsigned":0},"dtype":"i64","output":{"component":"runtime_conditioning","key":"reference_latent_count"}}}
        ],
        "unmatched":"Reject"
    }"#,
};

const STATE_PLAN_CASES: &[ModelFamilyStatePlanCase] = &[
    ModelFamilyStatePlanCase {
        layout: ModelStateLayout::PrefixedNative,
        plan: &NATIVE_STATE_PLAN,
    },
    ModelFamilyStatePlanCase {
        layout: ModelStateLayout::Diffusers,
        plan: &DIFFUSERS_STATE_PLAN,
    },
];

const LAYOUT_SIGNATURES: &[ModelLayoutSignature] = &[
    ModelLayoutSignature {
        layout: ModelStateLayout::PrefixedNative,
        required_keys: &[
            "model.diffusion_model.x_embedder.weight",
            "model.diffusion_model.double_stream_layers.0.img_instruct_attn.processor.img_to_q.weight",
            "model.diffusion_model.norm_out.linear_2.weight",
        ],
        required_prefixes: &[],
    },
    ModelLayoutSignature {
        layout: ModelStateLayout::Diffusers,
        required_keys: &[
            "x_embedder.weight",
            "double_stream_layers.0.img_instruct_attn.processor.img_to_q.weight",
            "norm_out.linear_2.weight",
        ],
        required_prefixes: &[],
    },
];

const COMPONENT_STATE_SCHEMAS: &[ModelFamilyComponentStateSchema] = &[
    ModelFamilyComponentStateSchema {
        component: "model",
        required_keys: REQUIRED_KEYS,
        optional_keys: OPTIONAL_KEYS,
        allow_unexpected: true,
    },
    ModelFamilyComponentStateSchema {
        component: "runtime_conditioning",
        required_keys: &["reference_latent_count"],
        optional_keys: &[],
        allow_unexpected: false,
    },
    ModelFamilyComponentStateSchema {
        component: "vae",
        required_keys: &[],
        optional_keys: &[],
        allow_unexpected: true,
    },
    ModelFamilyComponentStateSchema {
        component: "text_encoder",
        required_keys: &[],
        optional_keys: &[],
        allow_unexpected: true,
    },
];

pub const MODEL_FAMILY_REGISTRATION: ModelFamilyRegistration = ModelFamilyRegistration {
    definition: &MODEL_FAMILY,
    source_ordinal: 76,
    source_architecture: "model_base.Boogu",
    source_configuration: SOURCE_CONFIGURATION,
    required_state_keys: &[],
    profile_selector: None,
    clip_target_selector: ModelClipTargetSelector::Static(&CLIP_TARGET),
    state_plan_selector: ModelFamilyStatePlanSelector::Layout {
        signatures: LAYOUT_SIGNATURES,
        cases: STATE_PLAN_CASES,
    },
    component_state_schemas: COMPONENT_STATE_SCHEMAS,
};
