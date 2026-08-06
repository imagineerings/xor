use crate::{
    ModelClipTargetSelector, ModelFamilyDefinition, ModelFamilyRegistration,
    ModelFamilyStatePlanCase, ModelFamilyStatePlanSelector, ModelLayoutSignature,
    ModelSourceConfigurationRule, ModelStateLayout, ModelWeightRule,
    omnigen2_boogu_family::{
        BOOGU_CLIP_TARGET, BOOGU_COMPONENT_STATE_SCHEMAS, BOOGU_DETECTION_RULES,
        BOOGU_FORWARD_PROGRAM, BOOGU_MEMORY_ESTIMATOR, BOOGU_MODEL_OPTIONAL_KEYS,
        BOOGU_MODEL_REQUIRED_KEYS, BOOGU_SUPPORTED_DTYPES, OMNIGEN2_BOOGU_COMPONENTS,
        OMNIGEN2_BOOGU_PREFIXED_STATE_PLAN, OMNIGEN2_BOOGU_STANDALONE_STATE_PLAN,
        OMNIGEN2_BOOGU_SUPPORTED_DEVICES,
    },
};

pub const MODEL_FAMILY_IDENTIFIER: &str = "Boogu";
pub const MODEL_FAMILY_FEATURE_ID: &str = "COMFY-MODEL-0065";
pub const MODEL_FAMILY_FIXTURE: &str = "boogu-comfy-model-0065";
pub const MODEL_FAMILY_SOURCE_ORDINAL: u16 = 76;
pub const MODEL_FAMILY_SAMPLING_MULTIPLIER: f64 = 1.0;
pub const MODEL_FAMILY_SAMPLING_SHIFT: f64 = 3.16;
pub const MODEL_FAMILY_MEMORY_USAGE_FACTOR: f64 = 2.15;

const WEIGHT_RULES: &[ModelWeightRule] = &[ModelWeightRule {
    source_prefix: "model.diffusion_model.",
    target_prefix: "native.",
    required: true,
}];

pub const MODEL_FAMILY: ModelFamilyDefinition = ModelFamilyDefinition {
    feature_id: MODEL_FAMILY_FEATURE_ID,
    identifier: MODEL_FAMILY_IDENTIFIER,
    architecture_version: "boogu-transformer-2d-v1",
    latent_feature_id: "COMFY-MODEL-0029",
    latent_identifier: "Flux",
    clip_target: &BOOGU_CLIP_TARGET,
    components: OMNIGEN2_BOOGU_COMPONENTS,
    detection_rules: BOOGU_DETECTION_RULES,
    weight_rules: WEIGHT_RULES,
    required_keys: BOOGU_MODEL_REQUIRED_KEYS,
    optional_keys: BOOGU_MODEL_OPTIONAL_KEYS,
    supported_dtypes: BOOGU_SUPPORTED_DTYPES,
    supported_devices: OMNIGEN2_BOOGU_SUPPORTED_DEVICES,
    memory_estimator: BOOGU_MEMORY_ESTIMATOR,
    forward_program: BOOGU_FORWARD_PROGRAM,
};

const SOURCE_CONFIGURATION: &[ModelSourceConfigurationRule] = &[];

const STATE_PLAN_CASES: &[ModelFamilyStatePlanCase] = &[
    ModelFamilyStatePlanCase {
        layout: ModelStateLayout::PrefixedNative,
        plan: &OMNIGEN2_BOOGU_PREFIXED_STATE_PLAN,
    },
    ModelFamilyStatePlanCase {
        layout: ModelStateLayout::StandaloneNative,
        plan: &OMNIGEN2_BOOGU_STANDALONE_STATE_PLAN,
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
        layout: ModelStateLayout::StandaloneNative,
        required_keys: &[
            "x_embedder.weight",
            "double_stream_layers.0.img_instruct_attn.processor.img_to_q.weight",
            "norm_out.linear_2.weight",
        ],
        required_prefixes: &[],
    },
];

pub const MODEL_FAMILY_REGISTRATION: ModelFamilyRegistration = ModelFamilyRegistration {
    definition: &MODEL_FAMILY,
    source_ordinal: 76,
    source_architecture: "model_base.Boogu",
    source_configuration: SOURCE_CONFIGURATION,
    required_state_keys: &[],
    profile_selector: None,
    clip_target_selector: ModelClipTargetSelector::Static(&BOOGU_CLIP_TARGET),
    state_plan_selector: ModelFamilyStatePlanSelector::Layout {
        signatures: LAYOUT_SIGNATURES,
        cases: STATE_PLAN_CASES,
    },
    component_state_schemas: BOOGU_COMPONENT_STATE_SCHEMAS,
};
