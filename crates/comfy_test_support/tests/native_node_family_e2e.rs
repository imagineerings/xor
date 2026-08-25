use comfy_media::{PngLimits, encode_png_frame};
use comfy_model::{
    BackgroundRemovalFixtureMutation, DepthAnything3FixtureMutation, DepthAnything3FixtureProfile,
    NATIVE_UPSCALE_ADMITTED_ARCHITECTURE_COUNT, NATIVE_UPSCALE_ARCHITECTURE_COUNT,
    NATIVE_UPSCALE_CONTRACT_SHA256, NativeBackgroundRemovalError, NativeBackgroundRemovalResource,
    NativeDepthAnything3Invocation, NativeDepthAnything3ReferenceStrategy,
    NativeDepthAnything3ResizeMethod, NativeDepthAnything3Resource, NativeFrameInterpolationModel,
    NativeLatentUpscaleCheckpoint, NativeLatentUpscaleModelResource, NativeModelPayload,
    NativeModelPayloadError, NativeSdPoseHeatmapHead, NativeSdPoseModel, NativeSdPoseSd2Denoiser,
    NativeUpscaleContractError, NativeUpscaleModelError, NativeUpscaleModelResource,
    NativeUpscaleStateDictionaryLayout, NativeUpscaleUnavailableReason,
    SdPoseHeatmapHeadConfiguration, SdPoseSd2Configuration, compiled_native_upscale_contract,
    deterministic_reduced_depth_anything_3_checkpoint, mutate_reduced_depth_anything_3_checkpoint,
    reduced_depth_anything_3_checkpoint_parity_for_fixture, sdpose_heatmap_head_weight_manifest,
    sdpose_sd2_weight_manifest, select_reduced_depth_anything_3_reference_for_fixture,
};
use comfy_nodes::{
    NativePreparedEffectKind, NativeStoredModelPayload, NativeStructuredValue,
    built_in_source_schema,
};
use comfy_runtime::{
    AttemptState, NATIVE_IMAGE_REGISTRY_VERSION, NativeHandleKind, NativeHandleStoreError,
    NativeHandleStoreGeneration, NativeHandleType, NativeImageWorkerEvent, NativeImageWorkerPlan,
    NativeInputDescriptor, NativeNodeDescriptor, NativeNodeFailure, NativeNodeFailureKind,
    NativeNodeOutcome, NativeOpaqueHandle, NativePortCardinality, NativePreparedEffectRequest,
    NativePrimitive, NativePrimitiveType, NativeStoredPayload, NativeTypeUnion, NativeValue,
    NativeValueType, RuntimeSupervisor, SupervisorPolicy, WorkerHealth, WorkerLaunchConfig,
    compile_generated_native_prompt, generated_native_frontend_descriptors,
    generated_native_node_registry_projection, graph_to_prompt, native_image_registry_projection,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, ExecutionContext, ImageTensor,
    NativeTensorPayload, NativeTensorRole, RetryRngPolicy, RngStreamAddress, StreamId, Tensor,
    TensorBackend,
    generated_comfy_operator_indirection_01::tensor_from_f32_with_context_exact_native,
    generated_neural_network_functional_01::pixel_shuffle_tensor_with_context_exact_native,
    generated_spatial_functional_kernel_01::{
        InterpolateConfiguration, InterpolateMode, bislerp_tensor_with_context_exact_native,
        interpolate_tensor_with_context_exact_native,
        pixel_shuffle_nd_tensor_with_context_exact_native,
    },
};
use comfy_types::{AttemptId, NodeId, ProfileId, PromptId, WorkerId, WorkerMessage};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

const PROFILE_ID: ProfileId = ProfileId(Uuid::from_u128(0x3670));
const WORKFLOW_FIXTURE: &[u8] = include_bytes!("../fixtures/native_image/workflow.json");
const SDPOSE_FIXTURE_ARTIFACT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const UPSCALE_MODEL_RESOURCE_FIXTURE: &str =
    include_str!("../fixtures/models/upscale-model-resource-foundation/contract.json");
const BACKGROUND_REMOVAL_RESOURCE_FIXTURE: &str =
    include_str!("../fixtures/models/background-removal-resource-foundation/manifest.json");
const BACKGROUND_REMOVAL_RESOURCE_ORACLE: &str =
    include_str!("../fixtures/models/background-removal-resource-foundation/oracle.json");
const BACKGROUND_REMOVAL_RESOURCE_PROVENANCE: &str =
    include_str!("../fixtures/models/background-removal-resource-foundation/provenance.json");
const BACKGROUND_REMOVAL_RESOURCE_GENERATOR: &str =
    include_str!("../fixtures/models/background-removal-resource-foundation/generate_oracle.py");
const DEPTH_ANYTHING_3_RESOURCE_FIXTURE: &str =
    include_str!("../fixtures/models/depth-anything-3-resource-foundation/manifest.json");
const DEPTH_ANYTHING_3_RESOURCE_ORACLE: &str =
    include_str!("../fixtures/models/depth-anything-3-resource-foundation/oracle.json");
const DEPTH_ANYTHING_3_RESOURCE_PROVENANCE: &str =
    include_str!("../fixtures/models/depth-anything-3-resource-foundation/provenance.json");
const DEPTH_ANYTHING_3_RESOURCE_GENERATOR: &str =
    include_str!("../fixtures/models/depth-anything-3-resource-foundation/generate_oracle.py");
const DEPTH_ANYTHING_3_RESOURCE_SOURCE_GRAPH: &str =
    include_str!("../fixtures/models/depth-anything-3-resource-foundation/source_graph.py");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpscaleModelResourceFixture {
    schema_version: u32,
    contract_sha256: String,
    architecture_count: usize,
    admitted_architecture_count: usize,
    cases: Vec<UpscaleModelResourceCase>,
    forbidden_substitutes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpscaleModelResourceCase {
    layout: String,
    state_keys: Vec<String>,
    architecture_id: String,
    diagnostic: String,
}

fn fixture_raw_bits(value: &serde_json::Value) -> Result<Vec<u32>, Box<dyn Error>> {
    fn collect(value: &serde_json::Value, output: &mut Vec<u32>) -> Result<(), Box<dyn Error>> {
        if let Some(array) = value.as_array() {
            for value in array {
                collect(value, output)?;
            }
            return Ok(());
        }
        output.push(u32::try_from(
            value.as_u64().ok_or("fixture bit is not an integer")?,
        )?);
        Ok(())
    }
    let mut output = Vec::new();
    collect(value, &mut output)?;
    Ok(output)
}

fn require_exact_fixture_bits(
    phase: &str,
    actual: &[u32],
    expected: &[u32],
) -> Result<(), Box<dyn Error>> {
    if actual.len() != expected.len() {
        return Err(format!(
            "{phase}: actual length {}, expected {}",
            actual.len(),
            expected.len()
        )
        .into());
    }
    if let Some((index, (actual, expected))) = actual
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (actual, expected))| actual != expected)
    {
        return Err(
            format!("{phase}[{index}]: actual {actual:#010x}, expected {expected:#010x}").into(),
        );
    }
    Ok(())
}

fn sdpose_tensor(
    backend: &CpuBackend,
    shape: &[u64],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn Error>> {
    let elements = shape.iter().try_fold(1usize, |count, dimension| {
        count.checked_mul(usize::try_from(*dimension).ok()?)
    });
    let elements = elements.ok_or("SDPose fixture tensor shape overflowed")?;
    let mut values = Vec::new();
    values.try_reserve_exact(elements)?;
    values.resize(elements, 0.0);
    Ok(tensor_from_f32_with_context_exact_native(
        backend,
        shape,
        &values,
        DType::F32,
        backend.device(),
        context,
    )?)
}

fn reduced_sdpose_stored_payload() -> Result<NativeStoredPayload, Box<dyn Error>> {
    let workspace_bytes = 64 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let denoiser_configuration = SdPoseSd2Configuration::reduced_fixture(4, 3, 1, 1, 8, 8)?;
    let mut denoiser_weights = BTreeMap::new();
    for specification in sdpose_sd2_weight_manifest(&denoiser_configuration)? {
        denoiser_weights.insert(
            specification.key().to_owned(),
            sdpose_tensor(&backend, specification.shape(), &context)?,
        );
    }
    let denoiser = NativeSdPoseSd2Denoiser::from_reduced_fixture(
        denoiser_configuration,
        denoiser_weights,
        &cancellation,
    )?;
    let head_configuration = SdPoseHeatmapHeadConfiguration::reduced_fixture(8, 8, 3)?;
    let mut head_weights = BTreeMap::new();
    for specification in sdpose_heatmap_head_weight_manifest(&head_configuration)? {
        head_weights.insert(
            specification.key().to_owned(),
            sdpose_tensor(&backend, specification.shape(), &context)?,
        );
    }
    let head = NativeSdPoseHeatmapHead::from_reduced_fixture(
        head_configuration,
        head_weights,
        &cancellation,
    )?;
    let resource = Arc::new(NativeSdPoseModel::from_reduced_fixture(
        SDPOSE_FIXTURE_ARTIFACT.to_owned(),
        denoiser,
        head,
        &cancellation,
    )?);
    let model = Arc::new(NativeModelPayload::sdpose_model_test_fixture(resource)?);
    Ok(NativeStoredPayload::Model(Arc::new(
        NativeStoredModelPayload::model_resource(model)?,
    )))
}

fn reduced_frame_interpolation_stored_payload() -> Result<NativeStoredPayload, Box<dyn Error>> {
    let workspace_bytes = 16 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let resource = Arc::new(NativeFrameInterpolationModel::reduced_rife_test_fixture(
        &backend, &context,
    )?);
    let model = Arc::new(NativeModelPayload::frame_interpolation(resource)?);
    Ok(NativeStoredPayload::Model(Arc::new(
        NativeStoredModelPayload::model_resource(model)?,
    )))
}

#[test]
fn native_upscale_model_resource_is_closed_and_source_specific() -> Result<(), Box<dyn Error>> {
    let fixture: UpscaleModelResourceFixture =
        serde_json::from_str(UPSCALE_MODEL_RESOURCE_FIXTURE)?;
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.contract_sha256, NATIVE_UPSCALE_CONTRACT_SHA256);
    assert_eq!(
        fixture.architecture_count,
        NATIVE_UPSCALE_ARCHITECTURE_COUNT
    );
    assert_eq!(
        fixture.admitted_architecture_count,
        NATIVE_UPSCALE_ADMITTED_ARCHITECTURE_COUNT
    );
    assert_eq!(fixture.forbidden_substitutes.len(), 5);

    let cancellation = CancellationToken::default();
    let contract = compiled_native_upscale_contract()?;
    assert_eq!(contract.architectures().len(), fixture.architecture_count);
    assert_eq!(contract.admitted_architecture_count(), 0);
    for architecture in contract.architectures() {
        let error = match NativeUpscaleModelResource::checked(
            NativeUpscaleStateDictionaryLayout::Flat,
            architecture.detection_state_keys.iter(),
            &cancellation,
        ) {
            Err(error) => error,
            Ok(resource) => match resource {},
        };
        let unavailable = error
            .unavailable()
            .ok_or("architecture rejection lost its typed diagnostic")?;
        assert_eq!(unavailable.ordinal(), architecture.ordinal);
        assert_eq!(unavailable.architecture_id(), architecture.architecture_id);
        let expected_reason = if architecture.origin == "main" {
            NativeUpscaleUnavailableReason::MissingIndividualLicense
        } else {
            NativeUpscaleUnavailableReason::ReferenceOnlyExtraArchitecture
        };
        assert_eq!(unavailable.reason(), expected_reason);
        assert_eq!(unavailable.diagnostic(), architecture.license_disposition);
    }

    for fixture_case in fixture.cases {
        let layout = match fixture_case.layout.as_str() {
            "flat" => NativeUpscaleStateDictionaryLayout::Flat,
            "state_dict" => NativeUpscaleStateDictionaryLayout::StateDict,
            _ => return Err("unknown upscale fixture layout".into()),
        };
        let error = match NativeUpscaleModelResource::checked(
            layout,
            fixture_case.state_keys.iter(),
            &cancellation,
        ) {
            Err(error) => error,
            Ok(resource) => match resource {},
        };
        let NativeUpscaleModelError::Unavailable {
            architecture_id,
            reason,
            ..
        } = error
        else {
            return Err("upscale rejection lost its source-specific error".into());
        };
        assert_eq!(architecture_id, fixture_case.architecture_id);
        assert_eq!(reason.diagnostic(), fixture_case.diagnostic);
    }

    assert!(matches!(
        NativeUpscaleModelResource::checked(
            NativeUpscaleStateDictionaryLayout::Flat,
            ["not.an.upscale.architecture"],
            &cancellation,
        ),
        Err(NativeUpscaleModelError::Contract(
            NativeUpscaleContractError::NoArchitectureMatch
        ))
    ));
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        NativeUpscaleModelResource::checked(
            NativeUpscaleStateDictionaryLayout::Flat,
            ["body.0.weight", "body.1.weight"],
            &cancelled,
        ),
        Err(NativeUpscaleModelError::Contract(
            NativeUpscaleContractError::Cancelled
        ))
    ));

    let resource_source = include_str!("../../comfy_model/src/upscale_model.rs");
    let payload_source = include_str!("../../comfy_model/src/native_node_payload.rs");
    for forbidden in [
        "std::fs",
        "pyo3",
        "Python::",
        "image_resize",
        "generic_fallback",
        "model.safetensors",
    ] {
        assert!(!resource_source.contains(forbidden));
    }
    assert!(!payload_source.contains("NativeModelResource::UpscaleModel"));
    Ok(())
}

#[test]
fn native_latent_upscale_model_fixture_oracles_are_complete_and_consumed()
-> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("comfy_test_support has no repository root")?;
    let fixture_path = root.join(
        "crates/comfy_test_support/fixtures/models/latent-upscale-model-resource-foundation/manifest.json",
    );
    let fixture: serde_json::Value = serde_json::from_str(&fs::read_to_string(&fixture_path)?)?;
    assert_eq!(
        fixture
            .get("oracle_domain")
            .and_then(serde_json::Value::as_str),
        Some("zed.comfy.latent-upscale-independent-source-equations.v1")
    );
    let generator_sha = fixture
        .get("oracle_generator_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("latent-upscale generator SHA is missing")?;
    let actual_generator_sha = format!(
        "{:x}",
        Sha256::digest(fs::read(root.join(
            "crates/comfy_test_support/src/bin/generate_latent_upscale_model_fixture.rs"
        ),)?)
    );
    assert_eq!(generator_sha, actual_generator_sha);

    let cases = fixture
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .ok_or("latent-upscale fixture cases are missing")?;
    let identifiers = cases
        .iter()
        .filter_map(|case| case.get("id").and_then(serde_json::Value::as_str))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        identifiers,
        BTreeSet::from([
            "720-integrated-residual-order",
            "1080-repeat-rms-shortcut-order",
            "pixel-shuffle-dimension-one",
            "pixel-shuffle-dimension-two",
            "pixel-shuffle-dimension-three",
            "rational-blur-center-delta",
            "nearest-exact-half-coordinate",
            "bislerp-edge-cases",
            "ltx-vae-statistics-order",
        ])
    );
    for case in cases {
        let identifier = case
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or("latent-upscale fixture case lacks an id")?;
        if identifier != "rational-blur-center-delta" {
            assert!(
                case.as_object()
                    .is_some_and(|object| { object.keys().any(|key| key.ends_with("_bits")) }),
                "latent-upscale case {identifier} lacks a raw-bit oracle"
            );
        }
    }
    let expected_720_bits = cases
        .iter()
        .find(|case| {
            case.get("id").and_then(serde_json::Value::as_str)
                == Some("720-integrated-residual-order")
        })
        .and_then(|case| case.get("expected_bits"))
        .and_then(serde_json::Value::as_array)
        .ok_or("720 raw oracle bits are missing")?
        .iter()
        .map(|value| value.as_u64().ok_or("720 raw oracle bit is invalid"))
        .collect::<Result<Vec<_>, _>>()?;
    let workspace_bytes = 1024 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let tensor = |shape: &[u64], values: &[f32]| {
        tensor_from_f32_with_context_exact_native(
            &backend,
            shape,
            values,
            DType::F32,
            backend.device(),
            &context,
        )
    };
    let case = |identifier: &str| -> Result<&serde_json::Value, Box<dyn Error>> {
        cases
            .iter()
            .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some(identifier))
            .ok_or_else(|| format!("missing latent-upscale case {identifier}").into())
    };
    let tensor_bits = |tensor: &Tensor| -> Result<Vec<u32>, Box<dyn Error>> {
        Ok(comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
            &backend,
            tensor,
            &context,
        )?
        .iter()
        .map(|value| value.to_bits())
        .collect())
    };
    let mut identity = vec![0.0_f32; 27];
    identity[22] = 1.0;
    let mut ordered_state = Vec::new();
    for prefix in [
        "in_conv.conv",
        "blocks.0.block.0.conv",
        "blocks.0.block.2.conv",
        "blocks.0.block.4.conv",
        "out_conv.conv",
    ] {
        ordered_state.push((
            format!("{prefix}.weight"),
            tensor(&[1, 1, 3, 3, 3], &identity)?,
        ));
        ordered_state.push((format!("{prefix}.bias"), tensor(&[1], &[0.0])?));
    }
    let ordered_state_720 = ordered_state.clone();
    let resource = NativeLatentUpscaleModelResource::from_checkpoint(
        NativeLatentUpscaleCheckpoint {
            artifact_sha256: "1".repeat(64),
            metadata: BTreeMap::new(),
            ordered_state,
            memory_budget_bytes: workspace_bytes,
        },
        &context,
    )?;
    let input = tensor(&[1, 1, 3, 1, 1], &[-1.0, 0.0, 1.0])?;
    let output = resource.invoke_hunyuan_720p(&backend, &input, &context)?;
    let actual = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &output,
        &context,
    )?;
    assert_eq!(
        actual
            .iter()
            .map(|value| u64::from(value.to_bits()))
            .collect::<Vec<_>>(),
        expected_720_bits
    );
    let mutated_720_input = resource.invoke_hunyuan_720p(
        &backend,
        &tensor(&[1, 1, 3, 1, 1], &[-1.0, 0.0, 2.0])?,
        &context,
    )?;
    assert_ne!(
        actual
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        tensor_bits(&mutated_720_input)?
    );

    let mut early_branch_state = ordered_state_720.clone();
    let mut late_branch_state = ordered_state_720;
    let mut half_identity = identity.clone();
    half_identity[22] = 0.5;
    for (state, key) in [
        (&mut early_branch_state, "blocks.0.block.0.conv.weight"),
        (&mut late_branch_state, "blocks.0.block.2.conv.weight"),
    ] {
        let (_, weight) = state
            .iter_mut()
            .find(|(candidate, _)| candidate == key)
            .ok_or("720 branch weight is missing")?;
        *weight = tensor(&[1, 1, 3, 3, 3], &half_identity)?;
    }
    let build_720 = |artifact: char, ordered_state| {
        NativeLatentUpscaleModelResource::from_checkpoint(
            NativeLatentUpscaleCheckpoint {
                artifact_sha256: artifact.to_string().repeat(64),
                metadata: BTreeMap::new(),
                ordered_state,
                memory_budget_bytes: workspace_bytes,
            },
            &context,
        )
    };
    let early_branch = build_720('4', early_branch_state)?;
    let late_branch = build_720('5', late_branch_state)?;
    assert_ne!(
        tensor_bits(&early_branch.invoke_hunyuan_720p(&backend, &input, &context)?)?,
        tensor_bits(&late_branch.invoke_hunyuan_720p(&backend, &input, &context)?)?
    );

    let mut state_1080 = Vec::new();
    let conv = |output_channels: usize, input_channels: usize, centers: &[(usize, f32)]| {
        let mut values = vec![0.0_f32; output_channels * input_channels * 27];
        for (index, value) in centers {
            values[*index] = *value;
        }
        values
    };
    state_1080.push((
        "conv_in.conv.weight".to_owned(),
        tensor(&[2, 1, 3, 3, 3], &conv(2, 1, &[(22, 1.0), (49, 2.0)]))?,
    ));
    state_1080.push(("conv_in.conv.bias".to_owned(), tensor(&[2], &[0.0; 2])?));
    let residual_branch_weights = [[0.5_f32, -0.25], [-0.75, 0.5], [0.25, 1.0]];
    for (block, branch) in residual_branch_weights.iter().enumerate() {
        for norm in ["norm1", "norm2"] {
            state_1080.push((
                format!("up.0.block.{block}.{norm}.gamma"),
                tensor(&[2, 1, 1, 1], &[1.0, 1.0])?,
            ));
        }
        state_1080.push((
            format!("up.0.block.{block}.conv1.conv.weight"),
            tensor(&[2, 2, 3, 3, 3], &conv(2, 2, &[(22, 1.0), (103, 1.0)]))?,
        ));
        state_1080.push((
            format!("up.0.block.{block}.conv1.conv.bias"),
            tensor(&[2], &[0.0; 2])?,
        ));
        state_1080.push((
            format!("up.0.block.{block}.conv2.conv.weight"),
            tensor(
                &[2, 2, 3, 3, 3],
                &conv(2, 2, &[(22, branch[0]), (103, branch[1])]),
            )?,
        ));
        state_1080.push((
            format!("up.0.block.{block}.conv2.conv.bias"),
            tensor(&[2], &[0.0; 2])?,
        ));
    }
    state_1080.push((
        "norm_out.gamma".to_owned(),
        tensor(&[2, 1, 1, 1], &[1.0, 1.0])?,
    ));
    state_1080.push((
        "conv_out.conv.weight".to_owned(),
        tensor(&[1, 2, 3, 3, 3], &conv(1, 2, &[(22, 1.0), (49, -1.0)]))?,
    ));
    state_1080.push(("conv_out.conv.bias".to_owned(), tensor(&[1], &[0.0])?));
    let mut mutated_state_1080 = state_1080.clone();
    let resource_1080 = NativeLatentUpscaleModelResource::from_checkpoint(
        NativeLatentUpscaleCheckpoint {
            artifact_sha256: "2".repeat(64),
            metadata: BTreeMap::new(),
            ordered_state: state_1080,
            memory_budget_bytes: workspace_bytes,
        },
        &context,
    )?;
    let output_1080 = resource_1080.invoke_hunyuan_1080p(
        &backend,
        &tensor(&[1, 1, 1, 1, 1], &[1.0])?,
        &context,
    )?;
    assert_eq!(
        tensor_bits(&output_1080)?,
        fixture_raw_bits(
            case("1080-repeat-rms-shortcut-order")?
                .get("expected_bits")
                .ok_or("1080 bits")?
        )?
    );
    let (_, branch_weight) = mutated_state_1080
        .iter_mut()
        .find(|(key, _)| key == "up.0.block.1.conv2.conv.weight")
        .ok_or("ordered 1080 branch weight is missing")?;
    *branch_weight = tensor(&[2, 2, 3, 3, 3], &conv(2, 2, &[(22, -0.5), (103, 0.75)]))?;
    let mutated_resource_1080 = NativeLatentUpscaleModelResource::from_checkpoint(
        NativeLatentUpscaleCheckpoint {
            artifact_sha256: "6".repeat(64),
            metadata: BTreeMap::new(),
            ordered_state: mutated_state_1080,
            memory_budget_bytes: workspace_bytes,
        },
        &context,
    )?;
    assert_ne!(
        tensor_bits(&output_1080)?,
        tensor_bits(&mutated_resource_1080.invoke_hunyuan_1080p(
            &backend,
            &tensor(&[1, 1, 1, 1, 1], &[1.0])?,
            &context,
        )?)?
    );

    let shuffle_one = pixel_shuffle_nd_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 2, 2, 1, 1], &[10.0, 20.0, 11.0, 21.0])?,
        1,
        2,
        &context,
    )?;
    assert_eq!(
        tensor_bits(&shuffle_one)?,
        fixture_raw_bits(
            case("pixel-shuffle-dimension-one")?
                .get("expected_frames_bits")
                .ok_or("shuffle-one bits")?
        )?
    );
    let mutated_shuffle_one = pixel_shuffle_nd_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 2, 2, 1, 1], &[10.0, 20.0, 12.0, 21.0])?,
        1,
        2,
        &context,
    )?;
    assert_ne!(
        tensor_bits(&mutated_shuffle_one)?,
        tensor_bits(&shuffle_one)?
    );
    let shuffle_two = pixel_shuffle_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 4, 1, 1], &[1.0, 2.0, 3.0, 4.0])?,
        2,
        &context,
    )?;
    assert_eq!(
        tensor_bits(&shuffle_two)?,
        fixture_raw_bits(
            case("pixel-shuffle-dimension-two")?
                .get("expected_bits")
                .ok_or("shuffle-two bits")?
        )?
    );
    let mutated_shuffle_two = pixel_shuffle_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 4, 1, 1], &[1.0, 2.0, 3.0, 5.0])?,
        2,
        &context,
    )?;
    assert_ne!(
        tensor_bits(&mutated_shuffle_two)?,
        tensor_bits(&shuffle_two)?
    );
    let shuffle_three = pixel_shuffle_nd_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 8, 1, 1, 1], &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0])?,
        3,
        2,
        &context,
    )?;
    assert_eq!(
        tensor_bits(&shuffle_three)?,
        fixture_raw_bits(
            case("pixel-shuffle-dimension-three")?
                .get("expected_bits")
                .ok_or("shuffle-three bits")?
        )?
    );
    let mutated_shuffle = pixel_shuffle_nd_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 8, 1, 1, 1], &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0])?,
        3,
        2,
        &context,
    )?;
    assert_ne!(tensor_bits(&mutated_shuffle)?, tensor_bits(&shuffle_three)?);

    let nearest = interpolate_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 1, 1, 2], &[0.0, 10.0])?,
        &InterpolateConfiguration {
            output_size: Some(vec![1, 3]),
            scale_factor: None,
            mode: InterpolateMode::NearestExact,
            align_corners: None,
            recompute_scale_factor: None,
            antialias: false,
        },
        &context,
    )?;
    assert_eq!(
        tensor_bits(&nearest)?,
        fixture_raw_bits(
            case("nearest-exact-half-coordinate")?
                .get("expected_bits")
                .ok_or("nearest bits")?
        )?
    );
    let floor_nearest = interpolate_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 1, 1, 2], &[0.0, 10.0])?,
        &InterpolateConfiguration {
            output_size: Some(vec![1, 3]),
            scale_factor: None,
            mode: InterpolateMode::Nearest,
            align_corners: None,
            recompute_scale_factor: None,
            antialias: false,
        },
        &context,
    )?;
    assert_ne!(tensor_bits(&floor_nearest)?, tensor_bits(&nearest)?);
    let mutated_nearest = interpolate_tensor_with_context_exact_native(
        &backend,
        &tensor(&[1, 1, 1, 2], &[0.0, 11.0])?,
        &InterpolateConfiguration {
            output_size: Some(vec![1, 3]),
            scale_factor: None,
            mode: InterpolateMode::NearestExact,
            align_corners: None,
            recompute_scale_factor: None,
            antialias: false,
        },
        &context,
    )?;
    assert_ne!(tensor_bits(&mutated_nearest)?, tensor_bits(&nearest)?);

    let bislerp_case = case("bislerp-edge-cases")?;
    let pairs = bislerp_case
        .get("pairs")
        .and_then(serde_json::Value::as_array)
        .ok_or("bislerp pairs")?;
    let expected = fixture_raw_bits(bislerp_case.get("expected_bits").ok_or("bislerp bits")?)?;
    let targets = [(4_u64, 1_usize), (3, 1), (3, 1), (8, 3)];
    let changed_ratio_targets = [(3_u64, 1_usize), (4, 1), (4, 1), (4, 1)];
    let mut actual_bislerp = Vec::new();
    for (pair_index, (pair, (target_width, sample))) in pairs.iter().zip(targets).enumerate() {
        let left = pair
            .get("left")
            .and_then(serde_json::Value::as_array)
            .ok_or("bislerp left")?;
        let right = pair
            .get("right")
            .and_then(serde_json::Value::as_array)
            .ok_or("bislerp right")?;
        let values = [
            left[0].as_f64().ok_or("left 0")? as f32,
            right[0].as_f64().ok_or("right 0")? as f32,
            left[1].as_f64().ok_or("left 1")? as f32,
            right[1].as_f64().ok_or("right 1")? as f32,
        ];
        let output = bislerp_tensor_with_context_exact_native(
            &backend,
            &tensor(&[1, 2, 1, 2], &values)?,
            target_width,
            1,
            &context,
        )?;
        let bits = tensor_bits(&output)?;
        actual_bislerp.extend([bits[sample], bits[target_width as usize + sample]]);

        let mut mutated_values = values;
        mutated_values[3] += 0.125;
        let mutated = bislerp_tensor_with_context_exact_native(
            &backend,
            &tensor(&[1, 2, 1, 2], &mutated_values)?,
            target_width,
            1,
            &context,
        )?;
        let mutated_bits = tensor_bits(&mutated)?;
        assert_ne!(
            [bits[sample], bits[target_width as usize + sample]],
            [
                mutated_bits[sample],
                mutated_bits[target_width as usize + sample]
            ],
            "bislerp pair {pair_index} did not discriminate a vector mutation"
        );

        let (changed_width, changed_sample) = changed_ratio_targets[pair_index];
        let changed_ratio = bislerp_tensor_with_context_exact_native(
            &backend,
            &tensor(&[1, 2, 1, 2], &values)?,
            changed_width,
            1,
            &context,
        )?;
        let changed_ratio_bits = tensor_bits(&changed_ratio)?;
        let baseline_sample = [bits[sample], bits[target_width as usize + sample]];
        let changed_sample = [
            changed_ratio_bits[changed_sample],
            changed_ratio_bits[changed_width as usize + changed_sample],
        ];
        if pair_index == 1 {
            assert_eq!(baseline_sample, changed_sample);
        } else {
            assert_ne!(
                baseline_sample, changed_sample,
                "bislerp pair {pair_index} did not discriminate a ratio mutation"
            );
        }
    }
    assert_eq!(actual_bislerp, expected);

    let blur_case = case("rational-blur-center-delta")?;
    let blur_numerator = blur_case
        .get("expected_numerator")
        .and_then(serde_json::Value::as_array)
        .ok_or("blur numerator")?;
    let blur_denominator = blur_case
        .get("normalization_denominator")
        .and_then(serde_json::Value::as_u64)
        .ok_or("blur denominator")? as f32;
    let mut blur_kernel = Vec::new();
    for row in blur_numerator {
        for value in row.as_array().ok_or("blur numerator row")? {
            blur_kernel
                .push(value.as_u64().ok_or("blur numerator value")? as f32 / blur_denominator);
        }
    }
    let blur_axis = [1.0_f32, 4.0, 6.0, 4.0, 1.0];
    let source_kernel = blur_axis
        .iter()
        .flat_map(|vertical| {
            blur_axis
                .iter()
                .map(move |horizontal| vertical * horizontal / 256.0)
        })
        .collect::<Vec<_>>();
    let config = serde_json::json!({
        "_class_name": "LatentUpsampler",
        "in_channels": 1,
        "mid_channels": 32,
        "num_blocks_per_stage": 1,
        "dims": 3,
        "spatial_upsample": true,
        "temporal_upsample": false,
        "spatial_scale": 1.5,
        "rational_resampler": true,
    });
    let zeros = |length: usize| vec![0.0_f32; length];
    let mut rational_state = vec![
        (
            "initial_conv.weight".to_owned(),
            tensor(&[32, 1, 3, 3, 3], &zeros(32 * 27))?,
        ),
        ("initial_conv.bias".to_owned(), tensor(&[32], &zeros(32))?),
        ("initial_norm.weight".to_owned(), tensor(&[32], &[1.0; 32])?),
        ("initial_norm.bias".to_owned(), tensor(&[32], &zeros(32))?),
        (
            "upsampler.conv.weight".to_owned(),
            tensor(&[288, 32, 3, 3], &zeros(288 * 32 * 9))?,
        ),
        (
            "upsampler.conv.bias".to_owned(),
            tensor(&[288], &zeros(288))?,
        ),
        (
            "upsampler.blur_down.kernel".to_owned(),
            tensor(&[1, 1, 5, 5], &source_kernel)?,
        ),
        (
            "final_conv.weight".to_owned(),
            tensor(&[1, 32, 3, 3, 3], &zeros(32 * 27))?,
        ),
        ("final_conv.bias".to_owned(), tensor(&[1], &[0.0])?),
    ];
    for family in ["res_blocks", "post_upsample_res_blocks"] {
        for convolution in ["conv1", "conv2"] {
            rational_state.push((
                format!("{family}.0.{convolution}.weight"),
                tensor(&[32, 32, 3, 3, 3], &zeros(32 * 32 * 27))?,
            ));
            rational_state.push((
                format!("{family}.0.{convolution}.bias"),
                tensor(&[32], &zeros(32))?,
            ));
        }
        for normalization in ["norm1", "norm2"] {
            rational_state.push((
                format!("{family}.0.{normalization}.weight"),
                tensor(&[32], &[1.0; 32])?,
            ));
            rational_state.push((
                format!("{family}.0.{normalization}.bias"),
                tensor(&[32], &zeros(32))?,
            ));
        }
    }
    let mut mutated_rational_state = rational_state.clone();
    let rational_resource = NativeLatentUpscaleModelResource::from_checkpoint(
        NativeLatentUpscaleCheckpoint {
            artifact_sha256: "3".repeat(64),
            metadata: BTreeMap::from([("config".to_owned(), serde_json::to_string(&config)?)]),
            ordered_state: rational_state,
            memory_budget_bytes: workspace_bytes,
        },
        &context,
    )?;
    let mut center_delta = vec![0.0_f32; 25];
    center_delta[12] = 1.0;
    let blurred = rational_resource.rational_blur_test_support(
        &backend,
        &tensor(&[1, 1, 5, 5], &center_delta)?,
        2,
        &context,
    )?;
    let blurred_values = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &blurred,
        &context,
    )?;
    assert_eq!(blurred_values, blur_kernel);
    let mut mutated_kernel = source_kernel;
    mutated_kernel[12] += 1.0 / 256.0;
    let (_, kernel) = mutated_rational_state
        .iter_mut()
        .find(|(key, _)| key == "upsampler.blur_down.kernel")
        .ok_or("rational blur kernel is missing")?;
    *kernel = tensor(&[1, 1, 5, 5], &mutated_kernel)?;
    let mutated_rational_resource = NativeLatentUpscaleModelResource::from_checkpoint(
        NativeLatentUpscaleCheckpoint {
            artifact_sha256: "7".repeat(64),
            metadata: BTreeMap::from([("config".to_owned(), serde_json::to_string(&config)?)]),
            ordered_state: mutated_rational_state,
            memory_budget_bytes: workspace_bytes,
        },
        &context,
    )?;
    let mutated_blur = mutated_rational_resource.rational_blur_test_support(
        &backend,
        &tensor(&[1, 1, 5, 5], &center_delta)?,
        2,
        &context,
    )?;
    assert_ne!(tensor_bits(&mutated_blur)?, tensor_bits(&blurred)?);

    let ltx_case = case("ltx-vae-statistics-order")?;
    let fixture_floats = |field: &str| -> Result<[f32; 4], Box<dyn Error>> {
        let values = ltx_case
            .get(field)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("missing {field}"))?;
        Ok([
            values[0].as_f64().ok_or("fixture float 0")? as f32,
            values[1].as_f64().ok_or("fixture float 1")? as f32,
            values[2].as_f64().ok_or("fixture float 2")? as f32,
            values[3].as_f64().ok_or("fixture float 3")? as f32,
        ])
    };
    let mut means = [0.0_f32; 128];
    means[..4].copy_from_slice(&fixture_floats("mean")?);
    let mut standard_deviations = [1.0_f32; 128];
    standard_deviations[..4].copy_from_slice(&fixture_floats("standard_deviation")?);
    let mut ltx_input = [0.0_f32; 128];
    ltx_input[..4].copy_from_slice(&fixture_floats("input")?);
    let unnormalized = comfy_model::vae_video::ltx_latent_statistics_test_support(
        &backend,
        &tensor(&[1, 128, 1, 1, 1], &ltx_input)?,
        &means,
        &standard_deviations,
        false,
        &context,
    )?;
    assert_eq!(
        &tensor_bits(&unnormalized)?[..4],
        fixture_raw_bits(
            ltx_case
                .get("expected_unnormalized_bits")
                .ok_or("unnormalized bits")?
        )?
        .as_slice()
    );
    let mut model_raw = [0.0_f32; 128];
    model_raw[..4].copy_from_slice(&fixture_floats("model_raw")?);
    let normalized = comfy_model::vae_video::ltx_latent_statistics_test_support(
        &backend,
        &tensor(&[1, 128, 1, 1, 1], &model_raw)?,
        &means,
        &standard_deviations,
        true,
        &context,
    )?;
    assert_eq!(
        &tensor_bits(&normalized)?[..4],
        fixture_raw_bits(
            ltx_case
                .get("expected_normalized_bits")
                .ok_or("normalized bits")?
        )?
        .as_slice()
    );
    let mut mutated_means = means;
    mutated_means[0] += 1.0;
    let mean_mutation = comfy_model::vae_video::ltx_latent_statistics_test_support(
        &backend,
        &tensor(&[1, 128, 1, 1, 1], &ltx_input)?,
        &mutated_means,
        &standard_deviations,
        false,
        &context,
    )?;
    assert_ne!(
        &tensor_bits(&mean_mutation)?[..4],
        &tensor_bits(&unnormalized)?[..4]
    );

    let mut mutated_standard_deviations = standard_deviations;
    mutated_standard_deviations[0] += 1.0;
    let standard_deviation_mutation = comfy_model::vae_video::ltx_latent_statistics_test_support(
        &backend,
        &tensor(&[1, 128, 1, 1, 1], &model_raw)?,
        &means,
        &mutated_standard_deviations,
        true,
        &context,
    )?;
    assert_ne!(
        &tensor_bits(&standard_deviation_mutation)?[..4],
        &tensor_bits(&normalized)?[..4]
    );

    let reversed_order = comfy_model::vae_video::ltx_latent_statistics_test_support(
        &backend,
        &tensor(&[1, 128, 1, 1, 1], &ltx_input)?,
        &means,
        &standard_deviations,
        true,
        &context,
    )?;
    assert_ne!(
        &tensor_bits(&reversed_order)?[..4],
        &tensor_bits(&unnormalized)?[..4]
    );
    Ok(())
}

#[test]
fn native_background_removal_fixture_executes_and_discriminates_source_phases()
-> Result<(), Box<dyn Error>> {
    let fixture: serde_json::Value = serde_json::from_str(BACKGROUND_REMOVAL_RESOURCE_FIXTURE)?;
    let oracle: serde_json::Value = serde_json::from_str(BACKGROUND_REMOVAL_RESOURCE_ORACLE)?;
    let provenance: serde_json::Value =
        serde_json::from_str(BACKGROUND_REMOVAL_RESOURCE_PROVENANCE)?;
    assert_eq!(
        fixture
            .get("oracle_domain")
            .and_then(serde_json::Value::as_str),
        Some("zed.comfy.background-removal-source-profile.v1")
    );
    assert_eq!(
        fixture
            .get("source_sha256")
            .and_then(serde_json::Value::as_str),
        Some(comfy_model::BIREFNET_SOURCE_SHA256)
    );
    assert_eq!(
        oracle.get("format").and_then(serde_json::Value::as_str),
        Some("zed.comfy.background-removal-reduced-oracle.v1")
    );
    let generator_sha256 = format!(
        "{:x}",
        Sha256::digest(BACKGROUND_REMOVAL_RESOURCE_GENERATOR.as_bytes())
    );
    let oracle_sha256 = format!(
        "{:x}",
        Sha256::digest(BACKGROUND_REMOVAL_RESOURCE_ORACLE.as_bytes())
    );
    for document in [&fixture, &oracle, &provenance] {
        assert_eq!(
            document
                .get("generator_sha256")
                .and_then(serde_json::Value::as_str),
            Some(generator_sha256.as_str())
        );
    }
    for document in [&fixture, &provenance] {
        assert_eq!(
            document
                .get("oracle_sha256")
                .and_then(serde_json::Value::as_str),
            Some(oracle_sha256.as_str())
        );
    }
    for field in ["generator_command", "platform", "python"] {
        let oracle_value = oracle
            .get(field)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or("background-removal oracle provenance field is missing")?;
        assert_eq!(
            provenance.get(field).and_then(serde_json::Value::as_str),
            Some(oracle_value)
        );
    }
    assert!(
        oracle
            .get("f32_rule")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty())
    );
    let expected_sources = BTreeMap::from([
        (
            "projects/comfy/ComfyUI/comfy_extras/nodes_bg_removal.py",
            comfy_model::NODES_BACKGROUND_REMOVAL_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/bg_removal_model.py",
            comfy_model::BACKGROUND_REMOVAL_MODEL_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/background_removal/birefnet.py",
            comfy_model::BIREFNET_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/background_removal/birefnet.json",
            comfy_model::BIREFNET_CONFIG_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/clip_model.py",
            comfy_model::CLIP_MODEL_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ops.py",
            comfy_model::COMFY_OPS_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/model_management.py",
            comfy_model::MODEL_MANAGEMENT_SOURCE_SHA256,
        ),
    ]);
    for document in [&oracle, &provenance] {
        let pinned_sources = document
            .get("pinned_sources")
            .and_then(serde_json::Value::as_object)
            .ok_or("background-removal pinned sources are missing")?;
        assert_eq!(pinned_sources.len(), expected_sources.len());
        for (path, expected_sha256) in &expected_sources {
            assert_eq!(
                pinned_sources
                    .get(*path)
                    .and_then(serde_json::Value::as_str),
                Some(*expected_sha256)
            );
        }
    }
    let input_shape = fixture
        .get("input_shape")
        .and_then(serde_json::Value::as_array)
        .ok_or("background-removal input shape is missing")?
        .iter()
        .map(|value| value.as_u64().ok_or("invalid input-shape dimension"))
        .collect::<Result<Vec<_>, _>>()?;
    let output_shape = fixture
        .get("output_shape")
        .and_then(serde_json::Value::as_array)
        .ok_or("background-removal output shape is missing")?
        .iter()
        .map(|value| value.as_u64().ok_or("invalid output-shape dimension"))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(input_shape, vec![1, 3, 5, 4]);
    assert_eq!(output_shape, vec![1, 3, 5]);
    assert_eq!(oracle.get("input_shape"), fixture.get("input_shape"));
    assert_eq!(oracle.get("output_shape"), fixture.get("output_shape"));
    let input = fixture
        .get("input_rgba")
        .and_then(serde_json::Value::as_array)
        .ok_or("background-removal input fixture is missing")?
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|value| value as f32)
                .ok_or("background-removal input value is invalid")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_bits = fixture_raw_bits(
        oracle
            .get("output_bits")
            .ok_or("background-removal oracle output is missing")?,
    )?;
    assert_eq!(
        fixture_raw_bits(
            fixture
                .get("baseline_output_bits")
                .ok_or("background-removal output fixture is missing")?,
        )?,
        expected_bits
    );
    let expected_raw_f32_sha256 = oracle
        .get("raw_f32_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("background-removal raw-output hash is missing")?;
    for document in [&fixture, &provenance] {
        assert_eq!(
            document
                .get("raw_f32_sha256")
                .and_then(serde_json::Value::as_str),
            Some(expected_raw_f32_sha256)
        );
    }
    let required_mutations = fixture
        .get("required_mutations")
        .and_then(serde_json::Value::as_array)
        .ok_or("background-removal mutation fixture is missing")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or("background-removal mutation is invalid")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    assert_eq!(
        required_mutations,
        BTreeSet::from([
            "aspp-dilated-branch",
            "aspp-global-pool",
            "deform-mask",
            "deform-offset",
            "relative-position-index",
            "shifted-window-block",
            "unused-decoder-head",
        ])
    );
    let mutation_oracles = oracle
        .get("mutations")
        .and_then(serde_json::Value::as_object)
        .ok_or("background-removal mutation oracles are missing")?;
    assert_eq!(
        mutation_oracles
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        required_mutations
    );
    for (name, mutation_oracle) in mutation_oracles {
        let mutation_bits = fixture_raw_bits(
            mutation_oracle
                .get("output_bits")
                .ok_or("background-removal mutation output is missing")?,
        )?;
        assert_eq!(
            mutation_bits != expected_bits,
            name != "unused-decoder-head",
            "{name} is not a discriminating source-equation oracle"
        );
        let mut mutation_hasher = Sha256::new();
        for bits in &mutation_bits {
            mutation_hasher.update(f32::from_bits(*bits).to_le_bytes());
        }
        assert_eq!(
            format!("{:x}", mutation_hasher.finalize()),
            mutation_oracle
                .get("raw_f32_sha256")
                .and_then(serde_json::Value::as_str)
                .ok_or("background-removal mutation hash is missing")?
        );
    }

    let workspace_bytes = 512 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let image = ImageTensor::from_f32(&backend, &context, 1, 3, 5, 4, &input)?;
    let resource = Arc::new(
        NativeBackgroundRemovalResource::deterministic_reduced_test_fixture(
            &backend,
            &context,
            BackgroundRemovalFixtureMutation::None,
        )?,
    );
    let output = resource.encode_image(&backend, &image, &context)?;
    let output_values =
        comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
            &backend,
            &output,
            &context,
        )?;
    let output_bits = output_values
        .iter()
        .map(|value| value.to_bits())
        .collect::<Vec<_>>();
    let mut raw_output_hasher = Sha256::new();
    for value in &output_values {
        raw_output_hasher.update(value.to_le_bytes());
    }
    let actual_raw_f32_sha256 = format!("{:x}", raw_output_hasher.finalize());
    assert_eq!(actual_raw_f32_sha256, expected_raw_f32_sha256);
    assert_eq!(output.descriptor().shape(), output_shape.as_slice());
    assert!(
        output_values
            .iter()
            .all(|value| (0.0..=1.0).contains(value))
    );
    assert_eq!(
        output_bits, expected_bits,
        "background-removal baseline bits were {output_bits:?}"
    );

    let mut alpha_mutation = input.clone();
    for alpha in alpha_mutation.iter_mut().skip(3).step_by(4) {
        *alpha = 1.0 - *alpha;
    }
    let alpha_output = resource.encode_image(
        &backend,
        &ImageTensor::from_f32(&backend, &context, 1, 3, 5, 4, &alpha_mutation)?,
        &context,
    )?;
    assert_eq!(
        comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
            &backend,
            &alpha_output,
            &context,
        )?
        .iter()
        .map(|value| value.to_bits())
        .collect::<Vec<_>>(),
        output_bits
    );

    let mut rgb_mutation = input.clone();
    rgb_mutation[0] = 0.75;
    let rgb_output = resource.encode_image(
        &backend,
        &ImageTensor::from_f32(&backend, &context, 1, 3, 5, 4, &rgb_mutation)?,
        &context,
    )?;
    let rgb_bits = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &rgb_output,
        &context,
    )?
    .iter()
    .map(|value| value.to_bits())
    .collect::<Vec<_>>();
    assert_ne!(rgb_bits, output_bits);

    let mut ordered_batch = input.clone();
    ordered_batch.extend_from_slice(&rgb_mutation);
    let mut reversed_batch = rgb_mutation.clone();
    reversed_batch.extend_from_slice(&input);
    let ordered = resource.encode_image(
        &backend,
        &ImageTensor::from_f32(&backend, &context, 2, 3, 5, 4, &ordered_batch)?,
        &context,
    )?;
    let reversed = resource.encode_image(
        &backend,
        &ImageTensor::from_f32(&backend, &context, 2, 3, 5, 4, &reversed_batch)?,
        &context,
    )?;
    let ordered_bits = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &ordered,
        &context,
    )?
    .iter()
    .map(|value| value.to_bits())
    .collect::<Vec<_>>();
    let reversed_bits = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &reversed,
        &context,
    )?
    .iter()
    .map(|value| value.to_bits())
    .collect::<Vec<_>>();
    assert_eq!(&ordered_bits[..15], output_bits.as_slice());
    assert_eq!(&ordered_bits[15..], rgb_bits.as_slice());
    assert_eq!(&reversed_bits[..15], rgb_bits.as_slice());
    assert_eq!(&reversed_bits[15..], output_bits.as_slice());

    for (mutation, mutation_name, changes_output) in [
        (
            BackgroundRemovalFixtureMutation::ShiftedWindowBlock,
            "shifted-window-block",
            true,
        ),
        (
            BackgroundRemovalFixtureMutation::RelativePositionIndex,
            "relative-position-index",
            true,
        ),
        (
            BackgroundRemovalFixtureMutation::DeformOffset,
            "deform-offset",
            true,
        ),
        (
            BackgroundRemovalFixtureMutation::DeformMask,
            "deform-mask",
            true,
        ),
        (
            BackgroundRemovalFixtureMutation::AsppDilatedBranch,
            "aspp-dilated-branch",
            true,
        ),
        (
            BackgroundRemovalFixtureMutation::AsppGlobalPool,
            "aspp-global-pool",
            true,
        ),
        (
            BackgroundRemovalFixtureMutation::UnusedDecoderHead,
            "unused-decoder-head",
            false,
        ),
    ] {
        let mutated = NativeBackgroundRemovalResource::deterministic_reduced_test_fixture(
            &backend, &context, mutation,
        )?;
        assert_ne!(
            mutated.semantic_digest_sha256(),
            resource.semantic_digest_sha256(),
            "{mutation:?} did not change resource identity"
        );
        let mutated_output = mutated.encode_image(&backend, &image, &context)?;
        let mutated_bits = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
            &backend,
            &mutated_output,
            &context,
        )?
        .iter()
        .map(|value| value.to_bits())
        .collect::<Vec<_>>();
        let expected_mutated_bits = fixture_raw_bits(
            mutation_oracles
                .get(mutation_name)
                .and_then(|value| value.get("output_bits"))
                .ok_or("background-removal mutation oracle is missing")?,
        )?;
        assert_eq!(
            mutated_bits, expected_mutated_bits,
            "{mutation:?} diverged from the independent source-equation oracle"
        );
        assert_eq!(
            mutated_bits != output_bits,
            changes_output,
            "{mutation:?} output disposition changed"
        );
    }

    let payload =
        NativeModelPayload::background_removal_test_fixture(resource.clone(), &cancellation)?;
    assert_eq!(
        payload
            .background_removal_resource()
            .ok_or("background-removal resource projection is missing")?
            .semantic_digest_sha256(),
        resource.semantic_digest_sha256()
    );
    assert_eq!(
        payload.resident_parts()?.resident_bytes()?,
        payload.resident_bytes()
    );

    let memory_before_cancellation = backend.memory_snapshot();
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        resource.encode_image(
            &backend,
            &image,
            &backend.execution_context(
                StreamId::DEFAULT,
                authority.authorize_workspace(workspace_bytes)?,
                &cancelled,
            ),
        ),
        Err(NativeBackgroundRemovalError::Cancelled)
    ));
    assert!(matches!(
        NativeModelPayload::background_removal_test_fixture(resource, &cancelled),
        Err(NativeModelPayloadError::Tensor(
            comfy_tensor::TensorError::Cancelled
        ))
    ));
    assert_eq!(backend.memory_snapshot(), memory_before_cancellation);
    Ok(())
}

fn assert_depth_anything_3_checkpoint_projection(
    oracle: &serde_json::Value,
    profile: &str,
    dtype: &str,
    resource: &NativeDepthAnything3Resource,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn Error>> {
    let expected = oracle
        .pointer(&format!("/checkpoint_projection/{profile}/{dtype}"))
        .ok_or("DA3 checkpoint projection oracle is missing")?;
    assert_eq!(
        expected.get("ordering").and_then(serde_json::Value::as_str),
        Some("utf8-key-ascending")
    );
    let actual = reduced_depth_anything_3_checkpoint_parity_for_fixture(resource, cancellation)?;
    assert_eq!(
        u64::try_from(actual.states.len())?,
        expected
            .get("key_count")
            .and_then(serde_json::Value::as_u64)
            .ok_or("DA3 checkpoint projection key count is missing")?
    );
    let expected_states = expected
        .get("states")
        .and_then(serde_json::Value::as_array)
        .ok_or("DA3 checkpoint state projection list is missing")?;
    assert_eq!(actual.states.len(), expected_states.len());
    for (actual, expected) in actual.states.iter().zip(expected_states) {
        assert_eq!(
            actual.key,
            expected
                .get("key")
                .and_then(serde_json::Value::as_str)
                .ok_or("DA3 projected checkpoint key is missing")?
        );
        let expected_shape = expected
            .get("shape")
            .and_then(serde_json::Value::as_array)
            .ok_or("DA3 projected checkpoint shape is missing")?
            .iter()
            .map(|dimension| {
                dimension
                    .as_u64()
                    .ok_or("DA3 projected checkpoint dimension is invalid")
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(actual.shape, expected_shape);
        assert_eq!(
            actual.source_sha256,
            expected
                .get("source_sha256")
                .and_then(serde_json::Value::as_str)
                .ok_or("DA3 checkpoint state source digest is missing")?
        );
        assert_eq!(
            actual.projected_f32_sha256,
            expected
                .get("projected_f32_sha256")
                .and_then(serde_json::Value::as_str)
                .ok_or("DA3 checkpoint state projected digest is missing")?
        );
    }
    assert_eq!(
        actual.source_sha256,
        expected
            .get("source_sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or("DA3 checkpoint source digest is missing")?
    );
    assert_eq!(
        actual.projected_f32_sha256,
        expected
            .get("projected_f32_sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or("DA3 projected checkpoint digest is missing")?
    );
    Ok(())
}

#[test]
fn native_depth_anything_3_reduced_resources_execute_and_publish_typed_geometry()
-> Result<(), Box<dyn Error>> {
    let manifest: serde_json::Value = serde_json::from_str(DEPTH_ANYTHING_3_RESOURCE_FIXTURE)?;
    let oracle: serde_json::Value = serde_json::from_str(DEPTH_ANYTHING_3_RESOURCE_ORACLE)?;
    let provenance: serde_json::Value = serde_json::from_str(DEPTH_ANYTHING_3_RESOURCE_PROVENANCE)?;
    assert_eq!(
        manifest
            .get("oracle_domain")
            .and_then(serde_json::Value::as_str),
        oracle.get("format").and_then(serde_json::Value::as_str)
    );
    let generator_sha256 = format!("{:x}", Sha256::digest(DEPTH_ANYTHING_3_RESOURCE_GENERATOR));
    let source_graph_sha256 = format!(
        "{:x}",
        Sha256::digest(DEPTH_ANYTHING_3_RESOURCE_SOURCE_GRAPH)
    );
    let oracle_sha256 = format!("{:x}", Sha256::digest(DEPTH_ANYTHING_3_RESOURCE_ORACLE));
    for document in [&manifest, &provenance] {
        assert_eq!(
            document
                .get("generator_sha256")
                .and_then(serde_json::Value::as_str),
            Some(generator_sha256.as_str())
        );
        assert_eq!(
            document
                .get("oracle_sha256")
                .and_then(serde_json::Value::as_str),
            Some(oracle_sha256.as_str())
        );
        assert_eq!(
            document
                .get("source_graph_sha256")
                .and_then(serde_json::Value::as_str),
            Some(source_graph_sha256.as_str())
        );
    }
    assert_eq!(
        provenance
            .get("transcendental_boundary")
            .and_then(serde_json::Value::as_str),
        Some(
            "F32 camera atan, DPT positional sin/cos/pow, and canonical exponential oracle bits call the pinned macOS host libc atanf/sinf/cosf/powf/expf through Python ctypes; they are not claimed portable to another libc or platform."
        )
    );
    let expected_sources = BTreeMap::from([
        (
            "projects/comfy/ComfyUI/comfy_extras/nodes_depth_anything_3.py",
            comfy_model::NODES_DEPTH_ANYTHING_3_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/model.py",
            comfy_model::DEPTH_ANYTHING_3_MODEL_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/preprocess.py",
            comfy_model::DEPTH_ANYTHING_3_PREPROCESS_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/dpt.py",
            comfy_model::DEPTH_ANYTHING_3_DPT_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/camera.py",
            comfy_model::DEPTH_ANYTHING_3_CAMERA_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/ray_pose.py",
            comfy_model::DEPTH_ANYTHING_3_RAY_POSE_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/reference_view_selector.py",
            comfy_model::DEPTH_ANYTHING_3_REFERENCE_VIEW_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/transform.py",
            comfy_model::DEPTH_ANYTHING_3_TRANSFORM_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/image_encoders/dino2.py",
            comfy_model::DEPTH_ANYTHING_3_DINO2_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/model_detection.py",
            comfy_model::DEPTH_ANYTHING_3_MODEL_DETECTION_SOURCE_SHA256,
        ),
        (
            "projects/comfy/ComfyUI/comfy/text_encoders/bert.py",
            "3f1f32353da95790285a10f452959a871aa949aab15a89b646a95abc6165955c",
        ),
        (
            "projects/comfy/ComfyUI/comfy/ldm/modules/attention.py",
            "436e1d91f8d5d84c5667e051cdf3ab2f91d8db25b66d88a084c89a202de0579e",
        ),
        (
            "projects/comfy/ComfyUI/comfy/utils.py",
            "8b8805ca837e20c922a846854156d10e214654f69df96be90969522f9def2bdb",
        ),
        (
            "projects/comfy/ComfyUI/comfy/ops.py",
            "9d8a4ec8357a9bfcd98dddbf06fcc2a0244643a392aacbe0970d945462c86a42",
        ),
        (
            "projects/comfy/ComfyUI/comfy/model_management.py",
            "c2ca243c80a5262ecafe19feb15cec22d4003c16e523b5376f543f0f75acabaa",
        ),
        (
            "projects/comfy/ComfyUI/comfy/supported_models.py",
            "3801a60d15fe0abf8573cfa60f90e796d773450370f80784f2e0603cda3ffd69",
        ),
        (
            "projects/comfy/ComfyUI/comfy/model_base.py",
            "99dc53baee665eca1a6aea70cfb9ab071d55784dff339b5e919dc14ae4fde8bd",
        ),
    ]);
    for document in [&oracle, &provenance] {
        let pinned = document
            .get("pinned_sources")
            .and_then(serde_json::Value::as_object)
            .ok_or("DA3 pinned sources are missing")?;
        assert_eq!(pinned.len(), expected_sources.len());
        for (path, digest) in &expected_sources {
            assert_eq!(
                pinned.get(*path).and_then(serde_json::Value::as_str),
                Some(*digest)
            );
        }
    }
    assert_ne!(
        oracle
            .pointer("/reference_fixture/saddle_balanced")
            .and_then(serde_json::Value::as_u64),
        oracle
            .pointer("/reference_fixture/saddle_sim_range")
            .and_then(serde_json::Value::as_u64),
        "the independent reference fixture must discriminate both saddle strategies"
    );
    let reference_tokens = fixture_raw_bits(
        oracle
            .pointer("/reference_fixture/token_bits")
            .ok_or("DA3 reference tokens are missing")?,
    )?
    .into_iter()
    .map(f32::from_bits)
    .collect::<Vec<_>>();
    for (name, strategy) in [
        ("first", NativeDepthAnything3ReferenceStrategy::First),
        ("middle", NativeDepthAnything3ReferenceStrategy::Middle),
        (
            "saddle_balanced",
            NativeDepthAnything3ReferenceStrategy::SaddleBalanced,
        ),
        (
            "saddle_sim_range",
            NativeDepthAnything3ReferenceStrategy::SaddleSimRange,
        ),
    ] {
        assert_eq!(
            u64::try_from(select_reduced_depth_anything_3_reference_for_fixture(
                &reference_tokens,
                4,
                strategy,
                &CancellationToken::default(),
            )?)?,
            oracle
                .pointer(&format!("/reference_fixture/{name}"))
                .and_then(serde_json::Value::as_u64)
                .ok_or("DA3 reference index is missing")?,
            "production reference selection diverged for {name}"
        );
    }

    let workspace_bytes = 256 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let tensor_bits = |tensor: &Tensor,
                       context: &ExecutionContext<'_>|
     -> Result<Vec<u32>, Box<dyn Error>> {
        Ok(comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
            &backend,
            tensor,
            context,
        )?
        .into_iter()
        .map(f32::to_bits)
        .collect())
    };
    let image_bits = fixture_raw_bits(
        oracle
            .get("input_bits")
            .ok_or("DA3 input oracle is missing")?,
    )?;
    let image_values = image_bits
        .into_iter()
        .map(f32::from_bits)
        .collect::<Vec<_>>();
    let image = ImageTensor::from_f32(&backend, &context, 1, 4, 4, 3, &image_values)?;
    let mut dtype_identities = BTreeSet::new();
    let mut f32_resource = None;
    let mut resources = Vec::new();
    for dtype in [DType::F16, DType::Bf16, DType::F32] {
        let resource = NativeDepthAnything3Resource::from_reduced_fixture(
            &backend,
            deterministic_reduced_depth_anything_3_checkpoint(
                &backend,
                DepthAnything3FixtureProfile::Dpt,
                dtype,
                workspace_bytes,
                &context,
            )?,
            &context,
        )?;
        let dtype_name = match dtype {
            DType::F16 => "f16",
            DType::Bf16 => "bf16",
            DType::F32 => "f32",
            _ => return Err("unexpected DA3 fixture dtype".into()),
        };
        assert_depth_anything_3_checkpoint_projection(
            &oracle,
            "dpt",
            dtype_name,
            &resource,
            &cancellation,
        )?;
        let resource = Arc::new(resource);
        dtype_identities.insert(resource.semantic_digest_sha256().to_owned());
        if dtype == DType::F32 {
            f32_resource = Some(resource.clone());
        }
        resources.push((dtype, resource));
    }
    assert_eq!(dtype_identities.len(), 3);
    let resource = f32_resource.ok_or("F32 DA3 fixture was not retained")?;
    for (dtype, resource) in &resources {
        let dtype_name = match dtype {
            DType::F16 => "f16",
            DType::Bf16 => "bf16",
            DType::F32 => "f32",
            _ => return Err("unexpected DA3 execution dtype".into()),
        };
        let geometry = resource.execute(
            &backend,
            NativeDepthAnything3Invocation {
                image: &image,
                views_per_sample: 1,
                process_resolution: 4,
                resize_method: NativeDepthAnything3ResizeMethod::UpperBound,
                reference_strategy: NativeDepthAnything3ReferenceStrategy::First,
                use_ray_pose: false,
                ransac_seed: 17,
                extrinsics: None,
                intrinsics: None,
            },
            &context,
        )?;
        assert_eq!(geometry.depth.descriptor().shape(), &[1, 4, 4]);
        assert!(geometry.confidence.is_none());
        assert_eq!(
            geometry
                .sky
                .as_ref()
                .map(|value| value.descriptor().shape()),
            Some([1, 4, 4].as_slice())
        );
        for (actual, field) in [
            (&geometry.depth, "depth"),
            (
                geometry.sky.as_ref().ok_or("DA3 sky output is missing")?,
                "sky",
            ),
        ] {
            let pointer = if *dtype == DType::F32 {
                format!("/reduced_dpt/{field}/bits")
            } else {
                format!("/reduced_dpt/low_precision/{dtype_name}/{field}/bits")
            };
            assert_eq!(
                tensor_bits(actual, &context)?,
                fixture_raw_bits(
                    oracle
                        .pointer(&pointer)
                        .ok_or_else(|| format!("missing DA3 execution oracle {pointer}"))?,
                )?,
                "{dtype_name} projected execution diverged"
            );
        }
    }
    let nonsquare_values = fixture_raw_bits(
        oracle
            .pointer("/nonsquare_resize_projection/input_bits")
            .ok_or("DA3 non-square input oracle is missing")?,
    )?
    .into_iter()
    .map(f32::from_bits)
    .collect::<Vec<_>>();
    let nonsquare = ImageTensor::from_f32(&backend, &context, 1, 2, 4, 3, &nonsquare_values)?;
    for (name, resize_method) in [
        ("upper_bound", NativeDepthAnything3ResizeMethod::UpperBound),
        ("lower_bound", NativeDepthAnything3ResizeMethod::LowerBound),
    ] {
        let resized_geometry = resource.execute(
            &backend,
            NativeDepthAnything3Invocation {
                image: &nonsquare,
                views_per_sample: 1,
                process_resolution: 6,
                resize_method,
                reference_strategy: NativeDepthAnything3ReferenceStrategy::First,
                use_ray_pose: false,
                ransac_seed: 17,
                extrinsics: None,
                intrinsics: None,
            },
            &context,
        )?;
        assert_eq!(resized_geometry.depth.descriptor().shape(), &[1, 2, 4]);
        for (actual, field) in [
            (&resized_geometry.depth, "depth"),
            (
                resized_geometry
                    .sky
                    .as_ref()
                    .ok_or("DA3 non-square sky output is missing")?,
                "sky",
            ),
        ] {
            let pointer = format!("/nonsquare_resize_projection/cases/{name}/{field}/bits");
            assert_eq!(
                tensor_bits(actual, &context)?,
                fixture_raw_bits(
                    oracle
                        .pointer(&pointer)
                        .ok_or_else(|| format!("missing DA3 resize oracle {pointer}"))?,
                )?,
                "{name} preprocessing/final projection diverged"
            );
        }
    }
    let payload =
        NativeModelPayload::depth_anything_3_test_fixture(resource.clone(), &cancellation)?;
    assert_eq!(
        payload
            .depth_anything_3_resource()
            .map(|value| value.semantic_digest_sha256()),
        Some(resource.semantic_digest_sha256())
    );
    assert_eq!(
        payload.resident_parts()?.resident_bytes()?,
        payload.resident_bytes()
    );

    let multiview_values = fixture_raw_bits(
        oracle
            .get("multiview_input_bits")
            .ok_or("DA3 multiview input oracle is missing")?,
    )?
    .into_iter()
    .map(f32::from_bits)
    .collect::<Vec<_>>();
    let multiview = ImageTensor::from_f32(&backend, &context, 3, 4, 4, 3, &multiview_values)?;
    let dual = Arc::new(NativeDepthAnything3Resource::from_reduced_fixture(
        &backend,
        deterministic_reduced_depth_anything_3_checkpoint(
            &backend,
            DepthAnything3FixtureProfile::DualDpt,
            DType::F32,
            workspace_bytes,
            &context,
        )?,
        &context,
    )?);
    assert_depth_anything_3_checkpoint_projection(&oracle, "dualdpt", "f32", &dual, &cancellation)?;
    let dual_geometry = dual.execute(
        &backend,
        NativeDepthAnything3Invocation {
            image: &multiview,
            views_per_sample: 3,
            process_resolution: 4,
            resize_method: NativeDepthAnything3ResizeMethod::LowerBound,
            reference_strategy: NativeDepthAnything3ReferenceStrategy::SaddleSimRange,
            use_ray_pose: false,
            ransac_seed: 17,
            extrinsics: None,
            intrinsics: None,
        },
        &context,
    )?;
    assert_eq!(dual_geometry.depth.descriptor().shape(), &[3, 4, 4]);
    assert_eq!(
        dual_geometry
            .confidence
            .as_ref()
            .map(|value| value.descriptor().shape()),
        Some([3, 4, 4].as_slice())
    );
    assert!(dual_geometry.sky.is_none());
    assert_eq!(
        dual_geometry
            .extrinsics
            .as_ref()
            .map(|value| value.descriptor().shape()),
        Some([1, 3, 3, 4].as_slice())
    );
    assert_eq!(
        dual_geometry
            .intrinsics
            .as_ref()
            .map(|value| value.descriptor().shape()),
        Some([1, 3, 3, 3].as_slice())
    );
    assert_eq!(
        tensor_bits(&dual_geometry.depth, &context)?,
        fixture_raw_bits(
            oracle
                .pointer("/reduced_dualdpt/depth/bits")
                .ok_or("DA3 DualDPT depth oracle is missing")?,
        )?
    );
    assert_eq!(
        tensor_bits(
            dual_geometry
                .confidence
                .as_ref()
                .ok_or("DA3 DualDPT confidence output is missing")?,
            &context,
        )?,
        fixture_raw_bits(
            oracle
                .pointer("/reduced_dualdpt/confidence/bits")
                .ok_or("DA3 DualDPT confidence oracle is missing")?,
        )?
    );
    for (name, strategy) in [
        ("first", NativeDepthAnything3ReferenceStrategy::First),
        ("middle", NativeDepthAnything3ReferenceStrategy::Middle),
        (
            "saddle_balanced",
            NativeDepthAnything3ReferenceStrategy::SaddleBalanced,
        ),
        (
            "saddle_sim_range",
            NativeDepthAnything3ReferenceStrategy::SaddleSimRange,
        ),
    ] {
        let strategy_geometry = dual.execute(
            &backend,
            NativeDepthAnything3Invocation {
                image: &multiview,
                views_per_sample: 3,
                process_resolution: 4,
                resize_method: NativeDepthAnything3ResizeMethod::LowerBound,
                reference_strategy: strategy,
                use_ray_pose: false,
                ransac_seed: 17,
                extrinsics: None,
                intrinsics: None,
            },
            &context,
        )?;
        for (actual, field) in [
            (&strategy_geometry.depth, "depth"),
            (
                strategy_geometry
                    .confidence
                    .as_ref()
                    .ok_or("DA3 strategy confidence is missing")?,
                "confidence",
            ),
            (
                strategy_geometry
                    .extrinsics
                    .as_ref()
                    .ok_or("DA3 strategy extrinsics are missing")?,
                "camera/extrinsics",
            ),
            (
                strategy_geometry
                    .intrinsics
                    .as_ref()
                    .ok_or("DA3 strategy intrinsics are missing")?,
                "camera/intrinsics",
            ),
        ] {
            let pointer = format!("/reduced_dualdpt/reference_strategies/{name}/{field}/bits");
            assert_eq!(
                tensor_bits(actual, &context)?,
                fixture_raw_bits(
                    oracle
                        .pointer(&pointer)
                        .ok_or_else(|| format!("missing DA3 strategy oracle {pointer}"))?,
                )?,
                "reference strategy {name} did not preserve source reorder/restore"
            );
        }
    }
    assert_eq!(
        tensor_bits(
            dual_geometry
                .extrinsics
                .as_ref()
                .ok_or("DA3 decoded extrinsics are missing")?,
            &context,
        )?,
        fixture_raw_bits(
            oracle
                .pointer("/reduced_dualdpt/camera/extrinsics/bits")
                .ok_or("DA3 decoded extrinsics oracle is missing")?,
        )?
    );
    assert_eq!(
        tensor_bits(
            dual_geometry
                .intrinsics
                .as_ref()
                .ok_or("DA3 decoded intrinsics are missing")?,
            &context,
        )?,
        fixture_raw_bits(
            oracle
                .pointer("/reduced_dualdpt/camera/intrinsics/bits")
                .ok_or("DA3 decoded intrinsics oracle is missing")?,
        )?
    );

    let oracle_tensor = |pointer: &str, shape: &[u64]| -> Result<Tensor, Box<dyn Error>> {
        let values = fixture_raw_bits(
            oracle
                .pointer(pointer)
                .ok_or_else(|| format!("missing DA3 tensor oracle {pointer}"))?,
        )?
        .into_iter()
        .map(f32::from_bits)
        .collect::<Vec<_>>();
        Ok(tensor_from_f32_with_context_exact_native(
            &backend,
            shape,
            &values,
            DType::F32,
            backend.device(),
            &context,
        )?)
    };
    let camera_extrinsics = oracle_tensor("/camera_inputs/extrinsics/bits", &[1, 3, 3, 4])?;
    let camera_intrinsics = oracle_tensor("/camera_inputs/intrinsics/bits", &[1, 3, 3, 3])?;
    let supplied_camera_geometry = dual
        .execute(
            &backend,
            NativeDepthAnything3Invocation {
                image: &multiview,
                views_per_sample: 3,
                process_resolution: 4,
                resize_method: NativeDepthAnything3ResizeMethod::LowerBound,
                reference_strategy: NativeDepthAnything3ReferenceStrategy::SaddleSimRange,
                use_ray_pose: false,
                ransac_seed: 17,
                extrinsics: Some(&camera_extrinsics),
                intrinsics: Some(&camera_intrinsics),
            },
            &context,
        )
        .map_err(|error| -> Box<dyn Error> {
            format!("DA3 supplied-camera execution failed: {error}").into()
        })?;
    for (actual, pointer) in [
        (
            &supplied_camera_geometry.depth,
            "/reduced_dualdpt/supplied_camera_depth/bits",
        ),
        (
            supplied_camera_geometry
                .confidence
                .as_ref()
                .ok_or("DA3 supplied-camera confidence is missing")?,
            "/reduced_dualdpt/supplied_camera_confidence/bits",
        ),
        (
            supplied_camera_geometry
                .extrinsics
                .as_ref()
                .ok_or("DA3 supplied-camera extrinsics are missing")?,
            "/reduced_dualdpt/supplied_camera/extrinsics/bits",
        ),
        (
            supplied_camera_geometry
                .intrinsics
                .as_ref()
                .ok_or("DA3 supplied-camera intrinsics are missing")?,
            "/reduced_dualdpt/supplied_camera/intrinsics/bits",
        ),
    ] {
        assert_eq!(
            tensor_bits(actual, &context)?,
            fixture_raw_bits(
                oracle
                    .pointer(pointer)
                    .ok_or_else(|| format!("missing DA3 supplied-camera oracle {pointer}"))?,
            )?,
            "{pointer}: supplied-camera output diverged"
        );
    }

    let ransac_address = RngStreamAddress::new(
        "task390",
        "fixture",
        "depth-anything-3-ray-pose",
        0,
        "reduced-ray-pose",
        0,
        0,
        RetryRngPolicy::Replay,
    )?;
    let ray_context = ExecutionContext {
        stream: context.stream,
        scratch: context.scratch.clone(),
        rng_phase: Some(&ransac_address),
        cancellation: context.cancellation,
    };
    let (ray_geometry, ray_trace) = dual.execute_with_test_trace(
        &backend,
        NativeDepthAnything3Invocation {
            image: &multiview,
            views_per_sample: 3,
            process_resolution: 4,
            resize_method: NativeDepthAnything3ResizeMethod::LowerBound,
            reference_strategy: NativeDepthAnything3ReferenceStrategy::SaddleSimRange,
            use_ray_pose: true,
            ransac_seed: 17,
            extrinsics: None,
            intrinsics: None,
        },
        &ray_context,
    )?;
    for (actual, pointer) in [
        (
            &ray_trace.raw_ray,
            "/reduced_dualdpt/ray_pose_trace/pre_geometry_ray/bits",
        ),
        (
            &ray_trace.raw_ray_confidence,
            "/reduced_dualdpt/ray_pose_trace/pre_geometry_confidence/bits",
        ),
    ] {
        let document_pointer = pointer
            .strip_suffix("/bits")
            .ok_or("DA3 ray trace pointer has no bits suffix")?;
        let mut expected_shape = oracle
            .pointer(&format!("{document_pointer}/shape"))
            .and_then(Value::as_array)
            .ok_or_else(|| format!("missing DA3 ray trace shape {document_pointer}"))?
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| format!("invalid DA3 ray trace shape {document_pointer}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        expected_shape.insert(0, 1);
        assert_eq!(actual.descriptor().shape(), expected_shape);
        require_exact_fixture_bits(
            pointer,
            &tensor_bits(actual, &ray_context)?,
            &fixture_raw_bits(
                oracle
                    .pointer(pointer)
                    .ok_or_else(|| format!("missing DA3 ray trace oracle {pointer}"))?,
            )?,
        )?;
    }
    let raw_ray_values = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &ray_trace.raw_ray,
        &ray_context,
    )?;
    let raw_confidence_values = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
        &backend,
        &ray_trace.raw_ray_confidence,
        &ray_context,
    )?;
    let admission = oracle
        .pointer("/reduced_dualdpt/ray_pose_trace/admission")
        .ok_or("DA3 ray-pose admission trace is missing")?;
    assert_eq!(
        admission.get("candidate_count").and_then(Value::as_u64),
        Some(76)
    );
    assert_eq!(
        admission
            .pointer("/confidence_ordering/source")
            .and_then(Value::as_str),
        Some("torch.argsort(descending=True, stable=False-default)")
    );
    assert_eq!(
        admission
            .pointer("/confidence_ordering/native_owner")
            .and_then(Value::as_str),
        Some("argsort_with_context_exact_native(descending=true, stable=false)")
    );
    assert_eq!(
        admission
            .pointer("/confidence_ordering/tied_order_pinned")
            .and_then(Value::as_bool),
        Some(false)
    );
    let admission_views = admission
        .get("views")
        .and_then(Value::as_array)
        .ok_or("DA3 ray-pose admission views are missing")?;
    assert_eq!(admission_views.len(), 3);
    for (view, expected) in admission_views.iter().enumerate() {
        let confidence_start = view * 256;
        let confidence_end = confidence_start + 256;
        let confidence = raw_confidence_values
            .get(confidence_start..confidence_end)
            .ok_or("DA3 ray confidence view is unavailable")?;
        assert_eq!(
            expected.get("confidence_count").and_then(Value::as_u64),
            Some(256)
        );
        assert_eq!(
            expected
                .get("confidence_all_finite_positive")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            expected
                .get("confidence_all_bit_distinct")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            confidence
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        );
        let mut sorted_confidence = confidence.to_vec();
        sorted_confidence.sort_by(f32::total_cmp);
        assert!(
            sorted_confidence
                .windows(2)
                .all(|pair| pair[0].to_bits() != pair[1].to_bits())
        );
        let minimum_ulp_gap = sorted_confidence
            .windows(2)
            .map(|pair| pair[1].to_bits() - pair[0].to_bits())
            .min()
            .ok_or("DA3 ray confidence ULP gap is unavailable")?;
        let minimum_value_gap = sorted_confidence
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .min_by(f32::total_cmp)
            .ok_or("DA3 ray confidence value gap is unavailable")?;
        assert_eq!(
            u64::from(minimum_ulp_gap),
            expected
                .get("minimum_adjacent_ulp_gap")
                .and_then(Value::as_u64)
                .ok_or("DA3 ray confidence ULP gap oracle is missing")?
        );
        assert_eq!(
            u64::from(minimum_value_gap.to_bits()),
            expected
                .get("minimum_adjacent_value_gap_bits")
                .and_then(Value::as_u64)
                .ok_or("DA3 ray confidence value gap oracle is missing")?
        );
        let ray_start = view * 256 * 6;
        let ray_end = ray_start + 256 * 6;
        let ray_z = raw_ray_values
            .get(ray_start..ray_end)
            .ok_or("DA3 ray view is unavailable")?
            .chunks_exact(6)
            .map(|ray| {
                ray.get(2)
                    .copied()
                    .ok_or("DA3 ray z lane is unavailable")
                    .map(f32::abs)
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(ray_z.len(), 256);
        assert_eq!(
            expected.get("ray_z_all_valid").and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            ray_z
                .iter()
                .all(|value| value.is_finite() && *value > 1.0e-4)
        );
        let minimum_ray_z = ray_z
            .into_iter()
            .min_by(f32::total_cmp)
            .ok_or("DA3 ray z minimum is unavailable")?;
        assert_eq!(
            u64::from(minimum_ray_z.to_bits()),
            expected
                .get("minimum_ray_z_abs_bits")
                .and_then(Value::as_u64)
                .ok_or("DA3 ray z minimum oracle is missing")?
        );
    }
    let expected_samples = oracle
        .pointer("/reduced_dualdpt/ray_pose_trace/samples")
        .and_then(Value::as_array)
        .ok_or("DA3 RANSAC samples are missing")?
        .iter()
        .map(|row| {
            row.as_array()
                .ok_or("DA3 RANSAC sample row is invalid")?
                .iter()
                .map(|value| value.as_u64().ok_or("DA3 RANSAC sample is invalid"))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(ray_trace.ransac_samples, expected_samples);
    let index_rows_sha256 = |domain: &str, rows: &[Vec<u64>]| {
        let mut digest = Sha256::new();
        digest.update((domain.len() as u64).to_le_bytes());
        digest.update(domain.as_bytes());
        digest.update((rows.len() as u64).to_le_bytes());
        for row in rows {
            digest.update((row.len() as u64).to_le_bytes());
            for value in row {
                digest.update(value.to_le_bytes());
            }
        }
        format!("{:x}", digest.finalize())
    };
    let samples_domain = oracle
        .pointer("/reduced_dualdpt/ray_pose_trace/samples_sha256_domain")
        .and_then(Value::as_str)
        .ok_or("DA3 RANSAC sample SHA domain is missing")?;
    assert_eq!(
        index_rows_sha256(samples_domain, &ray_trace.ransac_samples),
        oracle
            .pointer("/reduced_dualdpt/ray_pose_trace/samples_sha256")
            .and_then(Value::as_str)
            .ok_or("DA3 RANSAC sample SHA is missing")?
    );
    assert_eq!(
        oracle
            .pointer("/reduced_dualdpt/ray_pose_trace/profile_version")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        oracle
            .pointer("/reduced_dualdpt/ray_pose_trace/algorithm")
            .and_then(Value::as_str),
        Some("mt19937")
    );
    assert_eq!(
        oracle
            .pointer("/reduced_dualdpt/ray_pose_trace/seed")
            .and_then(Value::as_u64),
        Some(17)
    );
    for (pointer, expected) in [
        ("workflow", "task390"),
        ("attempt", "fixture"),
        ("node", "depth-anything-3-ray-pose"),
        ("phase", "reduced-ray-pose"),
        ("retry_policy", "replay"),
        ("device_kind", "cpu"),
    ] {
        assert_eq!(
            oracle
                .pointer(&format!(
                    "/reduced_dualdpt/ray_pose_trace/address/{pointer}"
                ))
                .and_then(Value::as_str),
            Some(expected)
        );
    }
    let expected_views = oracle
        .pointer("/reduced_dualdpt/ray_pose_trace/views")
        .and_then(Value::as_array)
        .ok_or("DA3 RANSAC view traces are missing")?;
    assert_eq!(ray_trace.ransac_views.len(), expected_views.len());
    for (actual, expected) in ray_trace.ransac_views.iter().zip(expected_views) {
        let candidates = expected
            .get("candidate_indices")
            .and_then(Value::as_array)
            .ok_or("DA3 RANSAC candidates are missing")?
            .iter()
            .map(|value| value.as_u64().ok_or("DA3 RANSAC candidate is invalid"))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(actual.candidate_indices, candidates);
        assert_eq!(
            actual.best_iteration,
            expected
                .get("best_iteration")
                .and_then(Value::as_u64)
                .ok_or("DA3 RANSAC best iteration is missing")?
        );
        assert_eq!(
            actual.best_inliers.len() as u64,
            expected
                .get("best_inlier_count")
                .and_then(Value::as_u64)
                .ok_or("DA3 RANSAC inlier count is missing")?
        );
        assert_eq!(
            u64::from(actual.best_score_bits),
            expected
                .get("best_score_bits")
                .and_then(Value::as_u64)
                .ok_or("DA3 RANSAC best score is missing")?
        );
        assert_eq!(
            actual.fallback,
            expected
                .get("fallback")
                .and_then(Value::as_bool)
                .ok_or("DA3 RANSAC fallback disposition is missing")?
        );
        for (phase, actual_values) in [
            (
                "normalized_homography_bits",
                actual.normalized_homography.as_slice(),
            ),
            (
                "homography_post_sign_bits",
                actual.signed_homography.as_slice(),
            ),
            ("rotation_bits", actual.rotation.as_slice()),
            ("lower_bits", actual.lower.as_slice()),
            ("c2w_pre_inverse_bits", actual.c2w_pre_inverse.as_slice()),
        ] {
            let expected_values = fixture_raw_bits(
                expected
                    .get(phase)
                    .ok_or_else(|| format!("DA3 RANSAC {phase} is missing"))?,
            )?
            .into_iter()
            .map(f32::from_bits)
            .collect::<Vec<_>>();
            assert_eq!(actual_values.len(), expected_values.len());
            for (lane, (actual_value, expected_value)) in actual_values
                .iter()
                .copied()
                .zip(expected_values)
                .enumerate()
            {
                let tolerance = 2.5e-3_f32.max(expected_value.abs() * 2.5e-3);
                assert!(
                    (actual_value - expected_value).abs() <= tolerance,
                    "DA3 RANSAC view phase {phase}[{lane}]: {actual_value} != {expected_value}"
                );
            }
        }
        let inlier_domain = expected
            .get("best_inliers_sha256_domain")
            .and_then(Value::as_str)
            .ok_or("DA3 RANSAC inlier SHA domain is missing")?;
        assert_eq!(
            index_rows_sha256(inlier_domain, std::slice::from_ref(&actual.best_inliers)),
            expected
                .get("best_inliers_sha256")
                .and_then(Value::as_str)
                .ok_or("DA3 RANSAC inlier SHA is missing")?
        );
    }
    assert_eq!(
        tensor_bits(&ray_geometry.depth, &ray_context)?,
        fixture_raw_bits(
            oracle
                .pointer("/reduced_dualdpt/ray_depth/bits")
                .ok_or("DA3 ray depth oracle is missing")?,
        )?
    );
    assert_eq!(
        tensor_bits(
            ray_geometry
                .confidence
                .as_ref()
                .ok_or("DA3 ray confidence is missing")?,
            &ray_context,
        )?,
        fixture_raw_bits(
            oracle
                .pointer("/reduced_dualdpt/ray_confidence/bits")
                .ok_or("DA3 ray confidence oracle is missing")?,
        )?
    );
    for (actual, pointer) in [
        (
            ray_geometry
                .extrinsics
                .as_ref()
                .ok_or("DA3 ray extrinsics are missing")?,
            "/reduced_dualdpt/ray_extrinsics/bits",
        ),
        (
            ray_geometry
                .intrinsics
                .as_ref()
                .ok_or("DA3 ray intrinsics are missing")?,
            "/reduced_dualdpt/ray_intrinsics/bits",
        ),
    ] {
        let expected = fixture_raw_bits(
            oracle
                .pointer(pointer)
                .ok_or_else(|| format!("missing DA3 ray geometry oracle {pointer}"))?,
        )?
        .into_iter()
        .map(f32::from_bits)
        .collect::<Vec<_>>();
        let actual = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
            &backend,
            actual,
            &ray_context,
        )?;
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.into_iter().zip(expected) {
            let tolerance = 2.5e-3_f32.max(expected.abs() * 2.5e-3);
            assert!(
                (actual - expected).abs() <= tolerance,
                "{pointer}: {actual} != {expected}"
            );
        }
    }

    let public_geometry_identity = |geometry: &comfy_model::NativeDepthAnything3Geometry,
                                    context: &ExecutionContext<'_>|
     -> Result<String, Box<dyn Error>> {
        let mut hasher = Sha256::new();
        for tensor in [
            Some(&geometry.depth),
            geometry.confidence.as_ref(),
            geometry.sky.as_ref(),
            geometry.extrinsics.as_ref(),
            geometry.intrinsics.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for value in comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_context_exact_native(
                &backend,
                tensor,
                context,
            )? {
                hasher.update(value.to_le_bytes());
            }
        }
        Ok(format!("{:x}", hasher.finalize()))
    };
    let mutations = oracle
        .get("full_path_mutations")
        .and_then(serde_json::Value::as_object)
        .ok_or("DA3 full-path mutations are missing")?;
    for (name, mutation) in mutations {
        let execution = mutation
            .get("execution")
            .and_then(serde_json::Value::as_str)
            .ok_or("DA3 mutation execution is missing")?;
        let profile = if execution == "dpt" {
            DepthAnything3FixtureProfile::Dpt
        } else {
            DepthAnything3FixtureProfile::DualDpt
        };
        let mut checkpoint = deterministic_reduced_depth_anything_3_checkpoint(
            &backend,
            profile,
            DType::F32,
            workspace_bytes,
            &context,
        )?;
        mutate_reduced_depth_anything_3_checkpoint(
            &backend,
            &mut checkpoint,
            DepthAnything3FixtureMutation {
                state_key: mutation
                    .get("state_key")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("DA3 mutation state key is missing")?,
                lane: usize::try_from(
                    mutation
                        .get("lane")
                        .and_then(serde_json::Value::as_u64)
                        .ok_or("DA3 mutation lane is missing")?,
                )?,
                delta: f32::from_bits(u32::try_from(
                    mutation
                        .get("delta_bits")
                        .and_then(serde_json::Value::as_u64)
                        .ok_or("DA3 mutation delta is missing")?,
                )?),
            },
            &context,
        )?;
        let mutated =
            NativeDepthAnything3Resource::from_reduced_fixture(&backend, checkpoint, &context)?;
        let (input, views, camera, use_ray, execution_context) = match execution {
            "dpt" => (&image, 1, None, false, &context),
            "camera" => (
                &multiview,
                3,
                Some((&camera_extrinsics, &camera_intrinsics)),
                false,
                &context,
            ),
            "ray" => (&multiview, 3, None, true, &ray_context),
            "dual" => (&multiview, 3, None, false, &context),
            _ => return Err(format!("unknown DA3 mutation execution {execution}").into()),
        };
        let geometry = mutated.execute(
            &backend,
            NativeDepthAnything3Invocation {
                image: input,
                views_per_sample: views,
                process_resolution: 4,
                resize_method: NativeDepthAnything3ResizeMethod::LowerBound,
                reference_strategy: NativeDepthAnything3ReferenceStrategy::SaddleSimRange,
                use_ray_pose: use_ray,
                ransac_seed: 17,
                extrinsics: camera.map(|camera| camera.0),
                intrinsics: camera.map(|camera| camera.1),
            },
            execution_context,
        )?;
        let actual_identity = public_geometry_identity(&geometry, execution_context)?;
        assert_eq!(
            Some(actual_identity.as_str()),
            mutation
                .get("output_identity_sha256")
                .and_then(serde_json::Value::as_str),
            "DA3 mutation {name} diverged from the source-equation oracle"
        );
        let baseline_identity = match execution {
            "dpt" => oracle.pointer("/reduced_dpt/output_identity_sha256"),
            "camera" => oracle.pointer("/reduced_dualdpt/supplied_camera_output_identity_sha256"),
            "ray" => oracle.pointer("/reduced_dualdpt/ray_output_identity_sha256"),
            _ => oracle.pointer("/reduced_dualdpt/output_identity_sha256"),
        }
        .and_then(serde_json::Value::as_str)
        .ok_or("DA3 mutation baseline identity is missing")?;
        let changes_output = mutation
            .get("changes_output")
            .and_then(serde_json::Value::as_bool)
            .ok_or("DA3 mutation disposition is missing")?;
        assert_eq!(
            actual_identity != baseline_identity,
            changes_output,
            "{name}"
        );
        if !changes_output {
            let baseline_digest = match profile {
                DepthAnything3FixtureProfile::Dpt => resource.semantic_digest_sha256(),
                DepthAnything3FixtureProfile::DualDpt => dual.semantic_digest_sha256(),
            };
            assert_ne!(
                mutated.semantic_digest_sha256(),
                baseline_digest,
                "forward-unused retained state must still change resource identity"
            );
        }
    }
    Ok(())
}

#[test]
fn generated_registry_frontend_compiler_and_worker_dispatch_share_one_path()
-> Result<(), Box<dyn Error>> {
    let registry = generated_native_node_registry_projection(None)?;
    registry.validate_comprehensive_bindings()?;
    let early = native_image_registry_projection()?;
    assert!(
        early
            .descriptors()
            .all(|(class_type, _)| registry.descriptor(class_type).is_some())
    );

    let frontend = generated_native_frontend_descriptors(None)?;
    assert_eq!(frontend.len(), registry.descriptor_len());
    assert!(
        registry
            .descriptors()
            .all(|(class_type, _)| frontend.contains_key(class_type))
    );

    let workflow = comfy_runtime::WorkflowFormatDocument::parse(WORKFLOW_FIXTURE)?;
    let submission = graph_to_prompt(&workflow, &frontend, "task367-generated-native")?;
    let mut plan = compile_generated_native_prompt(submission, None)?;
    plan.prompt_id = PromptId(Uuid::from_u128(0x3671));
    assert_eq!(plan.nodes.len(), 5);
    assert_eq!(
        plan.output_nodes,
        vec![NodeId("4".to_owned()), NodeId("5".to_owned())]
    );

    let input_bytes = encode_png_frame(
        &[0.25, 0.5, 0.75],
        1,
        1,
        1,
        3,
        0,
        &BTreeMap::new(),
        PngLimits::default(),
    )?;
    let worker_plan = NativeImageWorkerPlan::new(
        plan.clone(),
        BTreeMap::from([("fixture.png".to_owned(), input_bytes)]),
        true,
        0,
    )?;
    let directory = tempfile::tempdir()?;
    let worker_directory = directory.path().join("worker");
    fs::create_dir(&worker_directory)?;
    let mut launch = WorkerLaunchConfig::new(
        env!("CARGO_BIN_EXE_comfy_native_image_worker_fixture"),
        PROFILE_ID,
        WorkerId(Uuid::from_u128(0x3672)),
        NATIVE_IMAGE_REGISTRY_VERSION,
        1024 * 1024 * 1024,
    );
    launch.working_directory = Some(worker_directory);
    launch.environment = vec![("PATH".to_owned(), String::new())];
    launch.policy = SupervisorPolicy {
        heartbeat_interval: Duration::from_secs(30),
        missed_heartbeat_limit: 3,
        shutdown_timeout: Duration::from_secs(3),
        ready_timeout: Duration::from_secs(10),
        maximum_automatic_restarts: 0,
        restart_backoff: Duration::from_millis(1),
    };
    let mut supervisor = smol::block_on(RuntimeSupervisor::start(launch))?;
    assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);
    smol::block_on(supervisor.execute(
        plan.prompt_id,
        AttemptId(Uuid::from_u128(0x3673)),
        serde_json::to_vec(&worker_plan)?,
    ))?;
    let terminal = smol::block_on(await_terminal_worker_event(&supervisor))?;
    let NativeImageWorkerEvent::Completed { result } = terminal else {
        return Err(format!("generated plan did not complete in the worker: {terminal:?}").into());
    };
    assert_eq!(result.report.state, AttemptState::Succeeded);
    assert_eq!(result.executed_node_count, 5);
    smol::block_on(supervisor.shutdown())?;
    Ok(())
}

#[test]
fn portable_values_dynamic_ports_and_attempt_handles_fail_closed() -> Result<(), Box<dyn Error>> {
    let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
    let union = NativeTypeUnion::new([
        NativeValueType::Primitive(NativePrimitiveType::Integer),
        NativeValueType::Primitive(NativePrimitiveType::String),
        NativeValueType::Handle(image_type.clone()),
    ])?;
    let descriptor = NativeNodeDescriptor {
        schema_version: comfy_nodes::NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
        class_type: "Task367PortableProbe".to_owned(),
        implementation_version: "1".to_owned(),
        source_schema: Some(comfy_nodes::NativeDescriptorSchemaMetadata::synthetic(
            ["value".to_owned()],
            [comfy_nodes::NativeDynamicSchemaMetadata::compatibility(
                "value_{index}",
                1,
                1,
                8,
                comfy_nodes::NativeInputSchemaMetadata::compatibility("value", "ANY"),
            )],
            std::iter::empty(),
        )),
        inputs: vec![NativeInputDescriptor {
            name: "value".to_owned(),
            accepted_types: union.clone(),
            required: true,
            hidden: false,
            lazy: true,
            cardinality: NativePortCardinality::Mapped,
            allows_literal: true,
        }],
        dynamic_inputs: vec![comfy_runtime::NativeDynamicInputDescriptor {
            name_template: "value_{index}".to_owned(),
            start_index: 1,
            minimum_count: 1,
            maximum_count: 8,
            input: NativeInputDescriptor {
                name: "value".to_owned(),
                accepted_types: union.clone(),
                required: false,
                hidden: false,
                lazy: false,
                cardinality: NativePortCardinality::List,
                allows_literal: true,
            },
        }],
        outputs: Vec::new(),
        output_node: true,
        effect: comfy_runtime::NativeEffectClass::WritesArtifact,
        cache: comfy_runtime::NativeCachePolicy::Never,
    };
    descriptor.validate()?;

    let scalar = NativeValue::Primitive {
        value: NativePrimitive::Integer(7),
    };
    let list = NativeValue::List {
        values: vec![
            scalar.clone(),
            NativeValue::Primitive {
                value: NativePrimitive::String("seven".to_owned()),
            },
        ],
    };
    assert!(union.accepts(&scalar));
    list.validate()?;
    let restored: NativeValue = serde_json::from_slice(&serde_json::to_vec(&list)?)?;
    assert_eq!(restored, list);

    let first_generation = NativeHandleStoreGeneration::with_capacities(4, 1024)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x3674));
    let first_store = first_generation.handle_store_for_attempt(attempt_id);
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
    let image_cancellation = CancellationToken::default();
    let image_context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority.authorize_workspace(1024 * 1024)?,
        &image_cancellation,
    );
    let image = ImageTensor::from_f32(&backend, &image_context, 1, 1, 1, 3, &[0.25, 0.5, 0.75])?;
    let payload = NativeStoredPayload::Tensor(Arc::new(NativeTensorPayload::from_image(
        NativeTensorRole::Image,
        image,
    )?));
    let handle = first_store.publish(payload.clone(), &CancellationToken::default())?;
    let handle_value = NativeValue::Handle {
        value: handle.clone(),
    };
    assert!(union.accepts(&handle_value));
    assert!(
        first_store
            .resolve(&handle, &image_type, &CancellationToken::default())
            .is_ok()
    );

    let recovered_generation = NativeHandleStoreGeneration::with_capacities(4, 1024)?;
    let recovered_store = recovered_generation.handle_store_for_attempt(attempt_id);
    assert!(matches!(
        recovered_store.resolve(&handle, &image_type, &CancellationToken::default()),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    assert!(matches!(
        first_store.publish(payload, &cancellation),
        Err(NativeHandleStoreError::Cancelled)
    ));

    let effect = NativePreparedEffectRequest::checked(
        Uuid::from_u128(0x3674),
        Uuid::from_u128(0x3675),
        NativePreparedEffectKind::Output,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )?;
    effect.validate()?;
    NativeNodeOutcome::Values {
        outputs: vec![list],
        ui: Some(json!({"task": 367})),
        effects: vec![effect],
    }
    .validate()?;
    NativeNodeOutcome::Blocked {
        reason: "provider activation required".to_owned(),
    }
    .validate()?;
    assert!(
        NativeNodeOutcome::Expansion {
            prompt: comfy_types::ApiPrompt::default(),
            output_node: NodeId("missing".to_owned()),
        }
        .validate()
        .is_err()
    );
    let failure = NativeNodeFailure {
        code: "task367_failure".to_owned(),
        message: "deterministic failure".to_owned(),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    };
    failure.validate()?;
    assert_eq!(
        serde_json::from_slice::<NativeNodeFailure>(&serde_json::to_vec(&failure)?)?,
        failure
    );
    Ok(())
}

#[test]
fn source_structured_values_keep_resolved_handles_typed_across_recovery()
-> Result<(), Box<dyn Error>> {
    let schema = built_in_source_schema("ResizeImageMaskNode")?;
    let resize_type = schema
        .inputs
        .iter()
        .find(|input| input.schema.name == "resize_type")
        .ok_or("ResizeImageMaskNode has no resize_type input")?;
    let match_size = resize_type
        .schema
        .structured_options()?
        .into_iter()
        .find(|option| option.selector == "match size")
        .ok_or("ResizeImageMaskNode has no match size option")?;
    assert!(match_size.fields.iter().any(|field| {
        field.path.as_slice() == ["match"]
            && field.schema.source_type_names.as_slice() == ["IMAGE", "MASK"]
    }));

    let generation = NativeHandleStoreGeneration::with_capacities(4, 1024)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x3760));
    let store = generation.handle_store_for_attempt(attempt_id);
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority.authorize_workspace(1024 * 1024)?,
        &cancellation,
    );
    let image = ImageTensor::from_f32(&backend, &context, 1, 1, 1, 3, &[0.1, 0.2, 0.3])?;
    let payload = NativeStoredPayload::Tensor(Arc::new(NativeTensorPayload::from_image(
        NativeTensorRole::Image,
        image,
    )?));
    let handle = store.publish(payload, &cancellation)?;
    let structured = NativeStructuredValue::checked(
        "COMFY_DYNAMICCOMBO_V3",
        BTreeMap::from([
            (
                "resize_type".to_owned(),
                NativeValue::Primitive {
                    value: NativePrimitive::String("match size".to_owned()),
                },
            ),
            (
                "crop".to_owned(),
                NativeValue::Primitive {
                    value: NativePrimitive::String("center".to_owned()),
                },
            ),
            (
                "match".to_owned(),
                NativeValue::Handle {
                    value: handle.clone(),
                },
            ),
        ]),
    )?;
    let value = structured.into_native_value();
    let expected = NativeTypeUnion::new([NativeValueType::NamedPreservedUnknown(
        "COMFY_DYNAMICCOMBO_V3".to_owned(),
    )])?;
    assert!(expected.accepts(&value));
    let restored: NativeValue = serde_json::from_slice(&serde_json::to_vec(&value)?)?;
    let restored = NativeStructuredValue::from_native_value(&restored)?
        .ok_or("structured value lost its typed representation")?;
    assert_eq!(
        restored.get("match"),
        Some(&NativeValue::Handle {
            value: handle.clone(),
        })
    );
    let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
    store.resolve(&handle, &image_type, &cancellation)?;

    let recovered =
        NativeHandleStoreGeneration::with_capacities(4, 1024)?.handle_store_for_attempt(attempt_id);
    assert!(matches!(
        recovered.resolve(&handle, &image_type, &cancellation),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        store.resolve(&handle, &image_type, &cancelled),
        Err(NativeHandleStoreError::Cancelled)
    ));
    Ok(())
}

#[test]
fn sdpose_model_resource_handle_is_sealed_alias_aware_and_restart_safe()
-> Result<(), Box<dyn Error>> {
    let payload = reduced_sdpose_stored_payload()?;
    payload.validate()?;
    let handle_type = payload.handle_type()?;
    assert_eq!(handle_type.kind, NativeHandleKind::Model);
    assert_eq!(handle_type.type_id, "MODEL");
    let byte_capacity = payload.resident_bytes()?;
    let generation = NativeHandleStoreGeneration::with_capacities(3, byte_capacity)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x4030));
    let store = generation.handle_store_for_attempt(attempt_id);
    let cancellation = CancellationToken::default();
    let first = store.publish(payload.clone(), &cancellation)?;
    let first_bytes = generation.resident_bytes();
    assert_eq!(first_bytes, byte_capacity);
    let second = store.publish(payload, &cancellation)?;
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let resolved = store.resolve(&first, &handle_type, &cancellation)?;
    let NativeStoredPayload::Model(model) = resolved.as_ref() else {
        return Err("SDPose MODEL handle resolved to another stored payload kind".into());
    };
    assert!(model.model_payload().sdpose_model_resource().is_some());
    assert_eq!(Some(model.digest_sha256()), first.digest_sha256());

    let distinct = reduced_sdpose_stored_payload()?;
    assert_eq!(distinct.digest_sha256(), model.digest_sha256());
    assert!(matches!(
        store.publish(distinct, &cancellation),
        Err(NativeHandleStoreError::Rejected(message)) if message.contains("capacity is exhausted")
    ));
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let forged = NativeOpaqueHandle::new(
        handle_type.clone(),
        first.store_identity(),
        first.identifier(),
        first.generation(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned()),
    )?;
    assert!(matches!(
        store.resolve(&forged, &handle_type, &cancellation),
        Err(NativeHandleStoreError::DigestMismatch)
    ));

    let restarted = NativeHandleStoreGeneration::with_capacities(2, byte_capacity)?
        .handle_store_for_attempt(attempt_id);
    assert!(matches!(
        restarted.resolve(&first, &handle_type, &cancellation),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));

    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let before_len = generation.len();
    let before_bytes = generation.resident_bytes();
    assert!(matches!(
        store.publish(reduced_sdpose_stored_payload()?, &cancelled),
        Err(NativeHandleStoreError::Cancelled)
    ));
    assert_eq!(generation.len(), before_len);
    assert_eq!(generation.resident_bytes(), before_bytes);
    assert_ne!(first.identifier(), second.identifier());
    Ok(())
}

#[test]
fn frame_interpolation_resource_handle_is_concrete_alias_aware_and_restart_safe()
-> Result<(), Box<dyn Error>> {
    let payload = reduced_frame_interpolation_stored_payload()?;
    payload.validate()?;
    let handle_type = payload.handle_type()?;
    assert_eq!(handle_type.kind, NativeHandleKind::Model);
    assert_eq!(handle_type.type_id, "INTERP_MODEL");
    let byte_capacity = payload.resident_bytes()?;
    let generation = NativeHandleStoreGeneration::with_capacities(3, byte_capacity)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x4080));
    let store = generation.handle_store_for_attempt(attempt_id);
    let cancellation = CancellationToken::default();
    let first = store.publish(payload.clone(), &cancellation)?;
    let first_bytes = generation.resident_bytes();
    let second = store.publish(payload, &cancellation)?;
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let resolved = store.resolve(&first, &handle_type, &cancellation)?;
    let NativeStoredPayload::Model(model) = resolved.as_ref() else {
        return Err("INTERP_MODEL handle resolved to another stored payload kind".into());
    };
    assert!(
        model
            .model_payload()
            .frame_interpolation_resource()
            .is_some()
    );
    assert_eq!(Some(model.digest_sha256()), first.digest_sha256());

    let distinct = reduced_frame_interpolation_stored_payload()?;
    assert_eq!(distinct.digest_sha256(), model.digest_sha256());
    assert!(matches!(
        store.publish(distinct, &cancellation),
        Err(NativeHandleStoreError::Rejected(message)) if message.contains("capacity is exhausted")
    ));
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let forged = NativeOpaqueHandle::new(
        handle_type.clone(),
        first.store_identity(),
        first.identifier(),
        first.generation(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned()),
    )?;
    assert!(matches!(
        store.resolve(&forged, &handle_type, &cancellation),
        Err(NativeHandleStoreError::DigestMismatch)
    ));
    let restarted = NativeHandleStoreGeneration::with_capacities(2, byte_capacity)?
        .handle_store_for_attempt(attempt_id);
    assert!(matches!(
        restarted.resolve(&first, &handle_type, &cancellation),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));
    assert_ne!(first.identifier(), second.identifier());
    Ok(())
}

async fn await_terminal_worker_event(
    supervisor: &RuntimeSupervisor,
) -> Result<NativeImageWorkerEvent, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("generated native worker dispatch timed out".into());
        }
        let envelope = supervisor.next_event(remaining).await?;
        if let WorkerMessage::Event { event } = envelope.message
            && let Ok(event) = postcard::from_bytes::<NativeImageWorkerEvent>(&event)
            && matches!(
                event,
                NativeImageWorkerEvent::Completed { .. }
                    | NativeImageWorkerEvent::BackendUnavailable { .. }
                    | NativeImageWorkerEvent::Failed { .. }
            )
        {
            return Ok(event);
        }
    }
}
