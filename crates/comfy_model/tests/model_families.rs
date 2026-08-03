include!(concat!(env!("OUT_DIR"), "/generated_model_family_tests.rs"));

use comfy_model::{
    GENERATED_MODEL_FAMILIES, GENERATED_MODEL_FAMILY_MANIFEST, ModelFamilyRegistry, ModelProbe,
    NativeFamilyBuildOptions, PatchGraph, PatchOperation, build_model_family,
    describe_model_family, map_model_weights,
};
use comfy_tensor::{
    CpuBackend, CpuWorkspaceAuthority, DType, StreamId, Tensor, TensorBackend, TensorDescriptor,
    generated_comfy_operator_indirection_01::{
        cast_to_with_context_exact_native, tensor_to_f32_with_context_exact_native,
    },
};
use comfy_types::{CancellationToken, DeviceKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilyFixture {
    fixture_id: String,
    feature_id: String,
    detector: ModelProbeFixture,
    base_artifact_digest: String,
    source_weights: Vec<TensorFixture>,
    input: TensorFixture,
    dtype: DType,
    device: DeviceKind,
    activation_elements: u64,
    memory_budget_bytes: u64,
    expected_memory_bytes: u64,
    checkpoints: Vec<CheckpointFixture>,
    patches: Vec<PatchOperation>,
    patched_checkpoints: Vec<CheckpointFixture>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelProbeFixture {
    tensor_shapes: BTreeMap<String, Vec<u64>>,
    metadata: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TensorFixture {
    key: String,
    shape: Vec<u64>,
    values: Vec<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointFixture {
    name: String,
    values: Vec<f32>,
}

#[derive(Serialize)]
struct ModelFamilyRowValidationArtifact<'a> {
    schema_version: u8,
    validation_id: &'static str,
    fixture_id: &'a str,
    feature_id: &'a str,
    identifier: &'a str,
    source_ordinal: u16,
    backend: &'static str,
    aggregate_model_family_breadth_claimed: bool,
    environment: ModelFamilyRowValidationEnvironment,
    digests: BTreeMap<&'static str, String>,
    cases: Vec<ModelFamilyRowValidationCase<'a>>,
    passed: usize,
    failed: u8,
    skipped: u8,
}

#[derive(Serialize)]
struct ModelFamilyRowValidationEnvironment {
    os: &'static str,
    arch: &'static str,
}

#[derive(Serialize)]
struct ModelFamilyRowValidationCase<'a> {
    id: &'a str,
    status: &'static str,
}

fn write_model_family_row_artifact(
    fixture_id: &str,
    feature_id: &str,
    identifier: &str,
    source_ordinal: u16,
    module: &str,
    cases: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    if fixture_id.is_empty()
        || !fixture_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || module.is_empty()
        || !module
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || cases.is_empty()
        || cases.iter().any(|case| case.is_empty())
    {
        return Err("model-family row validation identity or cases are invalid".into());
    }
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        (
            "row_source",
            repository_root.join(format!("crates/comfy_model/src/families/{module}.rs")),
        ),
        (
            "row_test",
            repository_root.join(format!("crates/comfy_model/tests/families/{module}.rs")),
        ),
        (
            "fixture",
            repository_root.join(format!(
                "crates/comfy_test_support/fixtures/models/{fixture_id}/family.json"
            )),
        ),
        (
            "provenance",
            repository_root.join(format!(
                "crates/comfy_test_support/fixtures/models/{fixture_id}/provenance.json"
            )),
        ),
    ];
    let mut digests = BTreeMap::new();
    for (name, path) in paths {
        digests.insert(name, format!("{:x}", Sha256::digest(std::fs::read(path)?)));
    }
    let artifact = ModelFamilyRowValidationArtifact {
        schema_version: 1,
        validation_id: "VAL-MODEL-FAMILY-ROW-001",
        fixture_id,
        feature_id,
        identifier,
        source_ordinal,
        backend: "comfy_tensor::CpuBackend",
        aggregate_model_family_breadth_claimed: false,
        environment: ModelFamilyRowValidationEnvironment {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        digests,
        cases: cases
            .iter()
            .map(|id| ModelFamilyRowValidationCase {
                id,
                status: "passed",
            })
            .collect(),
        passed: cases.len(),
        failed: 0,
        skipped: 0,
    };
    let output_directory = repository_root.join("target/comfy-parity/val-model-family-row-001");
    std::fs::create_dir_all(&output_directory)?;
    let mut bytes = serde_json::to_vec_pretty(&artifact)?;
    bytes.push(b'\n');
    std::fs::write(output_directory.join(format!("{fixture_id}.json")), bytes)?;
    Ok(())
}

#[test]
fn every_generated_model_family_fixture_executes_the_canonical_harness()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        GENERATED_MODEL_FAMILY_TEST_FIXTURES.len(),
        GENERATED_MODEL_FAMILY_MANIFEST.len()
    );
    let registry = ModelFamilyRegistry::checked(GENERATED_MODEL_FAMILIES)?;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(16 * 1024 * 1024)?;

    for (fixture_name, definition) in GENERATED_MODEL_FAMILY_MANIFEST {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("comfy_test_support")
            .join("fixtures")
            .join("models")
            .join(fixture_name)
            .join("family.json");
        let fixture: FamilyFixture = serde_json::from_slice(&std::fs::read(path)?)?;
        assert_eq!(fixture.fixture_id, *fixture_name);
        assert_eq!(fixture.feature_id, definition.feature_id);
        describe_model_family(definition)?;
        let detection = registry.detect(&ModelProbe {
            tensor_shapes: fixture.detector.tensor_shapes,
            metadata: fixture.detector.metadata,
        })?;
        assert_eq!(detection.identity.feature_id(), definition.feature_id);

        let cancellation = CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(8 * 1024 * 1024)?,
            &cancellation,
        );
        let source_weights = fixture
            .source_weights
            .iter()
            .map(|weight| {
                Ok((
                    weight.key.clone(),
                    tensor(&backend, weight, fixture.dtype, &context)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, Box<dyn std::error::Error>>>()?;
        let mapped = map_model_weights(definition, &fixture.base_artifact_digest, source_weights)?;
        let options = NativeFamilyBuildOptions {
            dtype: fixture.dtype,
            device: fixture.device,
            activation_elements: fixture.activation_elements,
            memory_budget_bytes: fixture.memory_budget_bytes,
            allow_unexpected_weights: false,
        };
        let model = build_model_family(definition, mapped, options)?;
        assert_eq!(
            model.memory_estimate().total_bytes,
            fixture.expected_memory_bytes
        );
        let input = tensor(&backend, &fixture.input, fixture.dtype, &context)?;
        assert_checkpoints(
            &backend,
            model.forward_checkpoints(&backend, &input, &context)?,
            &fixture.checkpoints,
            &context,
        )?;

        if !fixture.patches.is_empty() {
            let patched = PatchGraph::checked(&fixture.base_artifact_digest, fixture.patches)?
                .apply(&backend, model.weights(), &context)?;
            let patched_model = model.with_weights(patched)?;
            assert_checkpoints(
                &backend,
                patched_model.forward_checkpoints(&backend, &input, &context)?,
                &fixture.patched_checkpoints,
                &context,
            )?;
        } else {
            assert!(fixture.patched_checkpoints.is_empty());
        }
    }
    Ok(())
}

fn tensor(
    backend: &CpuBackend,
    fixture: &TensorFixture,
    dtype: DType,
    context: &comfy_tensor::ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn std::error::Error>> {
    let descriptor = TensorDescriptor::contiguous(
        fixture.shape.clone(),
        DType::F32,
        backend.device(),
        context.stream,
    )?;
    let (tensor, _) = backend.upload_f32(descriptor, &fixture.values, context)?;
    if dtype == DType::F32 {
        Ok(tensor)
    } else {
        Ok(cast_to_with_context_exact_native(
            backend,
            &tensor,
            dtype,
            backend.device(),
            false,
            false,
            context,
        )?)
    }
}

fn assert_checkpoints(
    backend: &CpuBackend,
    actual: Vec<comfy_model::ModelForwardCheckpoint>,
    expected: &[CheckpointFixture],
    context: &comfy_tensor::ExecutionContext<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(actual.len(), expected.len());
    for (checkpoint, expected) in actual.iter().zip(expected) {
        assert_eq!(checkpoint.name, expected.name);
        let actual_values =
            tensor_to_f32_with_context_exact_native(backend, &checkpoint.tensor, context)?;
        assert_eq!(actual_values.len(), expected.values.len());
        for (actual_value, expected_value) in actual_values.iter().zip(&expected.values) {
            assert!(
                (actual_value - expected_value).abs() <= 1.0e-5,
                "checkpoint {} expected {expected_value}, got {actual_value}",
                checkpoint.name,
            );
        }
    }
    Ok(())
}
