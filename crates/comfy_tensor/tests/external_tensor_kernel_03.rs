#[path = "ops/external_tensor_kernel_03.rs"]
mod external_tensor_kernel_03;

use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Component, Path},
};

use comfy_tensor::{
    GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES,
    generated_external_tensor_kernel_03::{
        BASS_BIQUAD_OPERATION_ID, BOTTOM_HAT_OPERATION_ID, BOX_CONVERT_OPERATION_ID,
        COMPOSE_OPERATION_ID, LAB_TO_RGB_OPERATION_ID, MEL_SCALE_OPERATION_ID,
        TO_TENSOR_OPERATION_ID,
    },
};
use sha2::{Digest, Sha256};

const EXPECTED: [(&str, &str, &str); 7] = [
    (
        "COMFY-TENSOR-OP-F37B4E403ACF",
        "zed.native.kornia.lab-to-rgb-0.8.2-f32.v1",
        "10e55e2fae5c07c34094194468cca6fa93a9e651abaee4d8758201b8324258df",
    ),
    (
        "COMFY-TENSOR-OP-C5A306EB73FD",
        "zed.native.kornia.morphology-bottom-hat-0.8.2-flat-geodesic.v1",
        "7834635173a02cfece9a4522525bf65e53d929b5b0e3952286e330a88d5a2328",
    ),
    (
        "COMFY-TENSOR-OP-F73C7107B450",
        "zed.native.torchaudio.bass-biquad-2.10.0-f32.v1",
        "f5467af3c463f32cc79e56e80f659bb26231c0b2229334ac3e211a9e1105bc31",
    ),
    (
        "COMFY-TENSOR-OP-EBA0D3470A35",
        "zed.native.torchaudio.mel-scale-2.10.0-slaney-f32.v1",
        "ec64ad096b6fc5243917e1a24ea682ee1d89447bf7de122df8fafe8e88ca8ebc",
    ),
    (
        "COMFY-TENSOR-OP-E937CE70AC37",
        "zed.native.torchvision.box-convert-0.27-cxcywh-xyxy-f32.v1",
        "d098104be1596881ec5059b8547a72912cb7897ab3b6a0ea6cedddca7c8d26c2",
    ),
    (
        "COMFY-TENSOR-OP-FBC26239461B",
        "zed.native.torchvision.compose-normalize-0.27-f32.v1",
        "335930a1f84a18b4f2f3fc6e97384e8c16e063c8f08410e43edbc71ff23cbced",
    ),
    (
        "COMFY-TENSOR-OP-D2AF4145E6CE",
        "zed.native.torchvision.to-tensor-0.27-rgb8-boundary.v1",
        "e5b04f615210697627188e780e7390eb3f41fdf471a12d5c6413469abe978242",
    ),
];

#[test]
fn task_69_resolution_slice_is_exact_build_sealed_and_source_aligned() -> Result<(), Box<dyn Error>>
{
    let slice = GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES
        .iter()
        .find(|slice| slice.module_name == "external_tensor_kernel_03")
        .ok_or("Task 69 resolution slice is missing")?;
    assert_eq!(slice.contracts.len(), EXPECTED.len());
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| (
                contract.operation_id,
                contract.overload_id,
                contract.evidence_fixture_sha256,
            ))
            .collect::<BTreeSet<_>>(),
        EXPECTED.into_iter().collect(),
    );
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| contract.operation_id)
            .collect::<BTreeSet<_>>()
            .len(),
        7,
    );
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| contract.overload_id)
            .collect::<BTreeSet<_>>()
            .len(),
        7,
    );
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| contract.evidence_fixture_sha256)
            .collect::<BTreeSet<_>>()
            .len(),
        7,
    );

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root")?;
    let module_root = workspace
        .join("crates/comfy_test_support/fixtures/tensor_operations/external_tensor_kernel_03");
    assert_eq!(module_root.canonicalize()?, module_root);
    let mut provenances = BTreeSet::new();
    for contract in slice.contracts {
        assert_ne!(
            contract.baseline_fixture_sha256,
            contract.evidence_fixture_sha256
        );
        let relative = Path::new(contract.evidence_fixture);
        assert!(
            relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        );
        assert_eq!(
            relative.parent(),
            Some(Path::new(
                "crates/comfy_test_support/fixtures/tensor_operations/external_tensor_kernel_03"
            ))
        );
        let path = workspace.join(relative);
        assert!(!fs::symlink_metadata(&path)?.file_type().is_symlink());
        assert_eq!(path.canonicalize()?.parent(), Some(module_root.as_path()));
        let bytes = fs::read(path)?;
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            contract.evidence_fixture_sha256,
        );
        let evidence: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(evidence["schema_version"], 1);
        assert_eq!(evidence["resolution_module"], contract.resolution_module);
        assert_eq!(evidence["operation_id"], contract.operation_id);
        assert_eq!(
            evidence["baseline_overload_id"],
            contract.baseline_overload_id
        );
        assert_eq!(
            evidence["baseline_fixture_sha256"],
            contract.baseline_fixture_sha256
        );
        assert_eq!(evidence["overload_id"], contract.overload_id);
        assert_eq!(evidence["owner_task_id"], contract.owner_task_id);
        let source_profile = evidence["source_profile"]
            .as_object()
            .ok_or("Task 69 evidence has no source profile")?;
        assert!(
            source_profile["dependency"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            source_profile["version"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            source_profile["profile"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        let fingerprint = source_profile["fingerprint_sha256"]
            .as_str()
            .ok_or("Task 69 source profile has no fingerprint")?;
        assert_eq!(fingerprint.len(), 64);
        assert!(fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let observations = evidence["source_observations"]
            .as_array()
            .filter(|observations| !observations.is_empty())
            .ok_or("Task 69 evidence has no observations")?;
        for observation in observations {
            provenances.insert(
                observation["provenance"]
                    .as_str()
                    .ok_or("observation provenance")?
                    .to_owned(),
            );
            assert!(
                observation["inputs"]
                    .as_object()
                    .is_some_and(|value| !value.is_empty())
            );
            assert!(
                observation["expected"]
                    .as_object()
                    .is_some_and(|value| !value.is_empty())
            );
            assert!(
                observation["tolerance"]
                    .as_object()
                    .is_some_and(|value| !value.is_empty())
            );
        }
        assert_eq!(
            evidence["ordered_parameters"],
            serde_json::from_str::<serde_json::Value>(contract.ordered_parameters_json)?,
        );
        assert_eq!(
            evidence["output_types"],
            serde_json::from_str::<serde_json::Value>(contract.output_types_json)?,
        );
        for (name, expected) in [
            ("rust_signature", contract.rust_signature),
            ("mutation_rule", contract.mutation_rule),
            ("alias_rule", contract.alias_rule),
            ("shape_rule", contract.shape_rule),
            ("dtype_rule", contract.dtype_rule),
            ("accumulation_dtype", contract.accumulation_dtype),
            ("layout_rule", contract.layout_rule),
            ("device_rule", contract.device_rule),
            ("numeric_rule", contract.numeric_rule),
            ("tolerance", contract.tolerance),
            ("determinism", contract.determinism),
            ("cancellation_points", contract.cancellation_points),
            ("vjp_rule", contract.vjp_rule),
            ("jvp_rule", contract.jvp_rule),
        ] {
            assert_eq!(evidence["semantics"][name], expected);
        }
    }
    assert!(provenances.contains("source_derived"));
    assert!(provenances.contains("independently_analytical"));
    assert!(provenances.contains("native_validated"));

    assert_eq!(LAB_TO_RGB_OPERATION_ID, EXPECTED[0].0);
    assert_eq!(BOTTOM_HAT_OPERATION_ID, EXPECTED[1].0);
    assert_eq!(BASS_BIQUAD_OPERATION_ID, EXPECTED[2].0);
    assert_eq!(MEL_SCALE_OPERATION_ID, EXPECTED[3].0);
    assert_eq!(BOX_CONVERT_OPERATION_ID, EXPECTED[4].0);
    assert_eq!(COMPOSE_OPERATION_ID, EXPECTED[5].0);
    assert_eq!(TO_TENSOR_OPERATION_ID, EXPECTED[6].0);
    Ok(())
}

#[test]
fn task_69_adapters_preserve_single_authoritative_owners() -> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root")?;
    let part_one = fs::read_to_string(
        workspace.join("crates/comfy_tensor/src/ops/external_tensor_kernel_01.rs"),
    )?;
    let part_two = fs::read_to_string(
        workspace.join("crates/comfy_tensor/src/ops/external_tensor_kernel_02.rs"),
    )?;
    let part_three = fs::read_to_string(
        workspace.join("crates/comfy_tensor/src/ops/external_tensor_kernel_03.rs"),
    )?;
    assert_eq!(part_one.matches("fn mel_filter_bank(").count(), 1);
    assert_eq!(part_one.matches("fn frequency_to_mel(").count(), 1);
    assert_eq!(part_one.matches("fn mel_to_frequency(").count(), 1);
    assert_eq!(part_two.matches("fn map_color_inputs<").count(), 1);
    assert_eq!(
        part_one
            .matches("pub fn native_morphology_with_context_exact(")
            .count(),
        1
    );
    assert_eq!(
        part_one
            .matches("pub fn biquad_with_context_exact_native(")
            .count(),
        1
    );
    assert!(part_three.contains("map_color("));
    assert!(part_three.contains("NativeMorphologyOperation::BottomHat"));
    assert!(part_three.contains("biquad_with_context_exact_native("));
    assert!(part_three.contains("mel_scale_project_with_context_exact_native("));
    assert!(part_three.contains("normalize_with_context_exact_native("));
    assert!(part_three.contains("image_bytes_to_tensor_with_context_exact_native("));
    assert!(!part_three.contains("fn mel_filter_bank("));
    assert!(!part_three.contains("fn map_color_inputs<"));
    Ok(())
}
