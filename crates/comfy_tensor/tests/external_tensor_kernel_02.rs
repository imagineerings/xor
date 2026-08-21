#[path = "ops/external_tensor_kernel_02.rs"]
mod external_tensor_kernel_02;

use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Component, Path},
};

use comfy_tensor::{
    GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES,
    generated_external_tensor_kernel_02::{
        CANNY_OPERATION_ID, DEFORM_CONV2D_OPERATION_ID, DILATION_OPERATION_ID,
        EFFICIENTNET_V2_S_OPERATION_ID, EINOPS_REARRANGE_OPERATION_ID, EROSION_OPERATION_ID,
        RAFT_LARGE_OPERATION_ID, RGB_TO_LAB_OPERATION_ID, RGB_TO_YCBCR_OPERATION_ID,
        TO_PIL_IMAGE_OPERATION_ID, TOP_HAT_OPERATION_ID, YCBCR_TO_RGB_OPERATION_ID,
    },
};
use sha2::{Digest, Sha256};

const EXPECTED: [(&str, &str, &str); 12] = [
    (
        "COMFY-TENSOR-OP-A56F89536902",
        "zed.native.einops.rearrange-0.8.1.v1",
        "9fbbde89b35613cd94421fb8a6464542d946ec40a517663e7183473ea801959d",
    ),
    (
        "COMFY-TENSOR-OP-4F9C05E204D4",
        "zed.native.kornia.rgb-to-lab-0.8.2-f32.v1",
        "62601ee4f38d75c9494e10c16e32e0a666aa76a6f750ef47a6c1aa166e7a9da0",
    ),
    (
        "COMFY-TENSOR-OP-A555F803F554",
        "zed.native.kornia.rgb-to-ycbcr-0.8.2-f32.v1",
        "7261afd10d510b782e77235480a2e603be8b97bd72ec14f17cd705ac75f52dd7",
    ),
    (
        "COMFY-TENSOR-OP-9EF1D9EB674A",
        "zed.native.kornia.ycbcr-to-rgb-0.8.2-f32.v1",
        "6ff4fa8ff87c096af3bd75afd8681f8d5a3b8d51e0ed240b8b10e9ab152dbd0f",
    ),
    (
        "COMFY-TENSOR-OP-A551C36699B7",
        "zed.native.kornia.canny-0.8.2-f32.v1",
        "e71b92be3beddf73aa314b238da7656ae1e89517a86e01a55033ed6e91b49156",
    ),
    (
        "COMFY-TENSOR-OP-AF5C2820E4C3",
        "zed.native.kornia.morphology-dilation-0.8.2-flat-geodesic.v1",
        "7019329d959a699fb3dcd368d2f959bbd88b1c2d84482f0eefa045102dde6cdb",
    ),
    (
        "COMFY-TENSOR-OP-9236C1C08976",
        "zed.native.kornia.morphology-erosion-0.8.2-flat-geodesic.v1",
        "3bf748576b53bfb6d5a56c225bfa7725a3049dd414c8e939f6d259ba7b2b418f",
    ),
    (
        "COMFY-TENSOR-OP-AC69F309A190",
        "zed.native.kornia.morphology-top-hat-0.8.2-flat-geodesic.v1",
        "7bde9a0bfcd6521c0788ae0d02b33bb0d1be2a3cd8b31657240cd448fbe702c9",
    ),
    (
        "COMFY-TENSOR-OP-638DE6179D46",
        "zed.native.torchvision.efficientnet-v2-s-0.27.v1",
        "7c3ba3a770a9d1eaa4a1184eb78e955dd8117476c90830c157d73e8f2d189999",
    ),
    (
        "COMFY-TENSOR-OP-852D8E9DBC9C",
        "zed.native.torchvision.raft-large-0.27.v1",
        "53147794c771878de08ab379beebeb54cc52caa7c11f9e9ea9fe2809f0563931",
    ),
    (
        "COMFY-TENSOR-OP-9E730487CA71",
        "zed.native.torchvision.deform-conv2d-0.27-f32.v1",
        "5d099e5597608f27ce8e8e5e5f1580b1445418c3d2c71b263dfd529b1a860114",
    ),
    (
        "COMFY-TENSOR-OP-B7926028DA57",
        "zed.native.torchvision.to-pil-image-0.27-rgb8-boundary.v1",
        "5f287104cedb74eaacf7da30c492c2127d6a4fe56c8216bb607f2b42d0aced80",
    ),
];

#[test]
fn task_68_resolution_slice_is_exact_build_sealed_and_source_aligned() -> Result<(), Box<dyn Error>>
{
    let slice = GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES
        .iter()
        .find(|slice| slice.module_name == "external_tensor_kernel_02")
        .ok_or("Task 68 resolution slice is missing")?;
    assert_eq!(slice.contracts.len(), EXPECTED.len());
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| (
                contract.operation_id,
                contract.overload_id,
                contract.evidence_fixture_sha256
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
        12
    );
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| contract.overload_id)
            .collect::<BTreeSet<_>>()
            .len(),
        12
    );
    assert_eq!(
        slice
            .contracts
            .iter()
            .map(|contract| contract.evidence_fixture_sha256)
            .collect::<BTreeSet<_>>()
            .len(),
        12
    );

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root")?;
    let expected_module_root = workspace
        .join("crates/comfy_test_support/fixtures/tensor_operations/external_tensor_kernel_02");
    let canonical_module_root = expected_module_root.canonicalize()?;
    assert_eq!(canonical_module_root, expected_module_root);
    let mut source_derived_count = 0;
    let mut independently_analytical_count = 0;
    let mut native_validated_count = 0;
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
                "crates/comfy_test_support/fixtures/tensor_operations/external_tensor_kernel_02"
            ))
        );
        let path = workspace.join(relative);
        assert!(!fs::symlink_metadata(&path)?.file_type().is_symlink());
        let canonical_path = path.canonicalize()?;
        assert_eq!(
            canonical_path.parent(),
            Some(canonical_module_root.as_path())
        );
        let bytes = fs::read(&canonical_path)?;
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            contract.evidence_fixture_sha256
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
            .ok_or("Task 68 evidence has no structured source profile")?;
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
            .ok_or("Task 68 source profile has no fingerprint")?;
        assert_eq!(fingerprint.len(), 64);
        assert!(fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(fingerprint, contract.baseline_fixture_sha256);
        assert_ne!(fingerprint, contract.evidence_fixture_sha256);
        let observations = evidence["source_observations"]
            .as_array()
            .filter(|observations| !observations.is_empty())
            .ok_or("Task 68 evidence has no structured numerical observations")?;
        let mut observation_ids = BTreeSet::new();
        let mut has_numeric_or_architecture_checkpoint = false;
        for observation in observations {
            let observation = observation
                .as_object()
                .ok_or("Task 68 observation is not structured")?;
            assert!(
                observation_ids.insert(
                    observation["id"]
                        .as_str()
                        .ok_or("Task 68 observation has no ID")?
                )
            );
            assert!(observation["case"].as_str().is_some());
            match observation["provenance"].as_str() {
                Some("source_derived") => source_derived_count += 1,
                Some("independently_analytical") => independently_analytical_count += 1,
                Some("native_validated") => native_validated_count += 1,
                _ => return Err("Task 68 observation has an invalid provenance".into()),
            }
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
            has_numeric_or_architecture_checkpoint |=
                matches!(
                    observation["case"].as_str(),
                    Some("architecture" | "nonzero_execution")
                ) || observation["expected"].as_object().is_some_and(|expected| {
                    ["values", "magnitude", "edges", "rgb8"]
                        .iter()
                        .any(|key| expected.contains_key(*key))
                });
        }
        assert!(has_numeric_or_architecture_checkpoint);
        match contract.operation_id {
            CANNY_OPERATION_ID => {
                let diagonal_cases = observations
                    .iter()
                    .filter(|observation| {
                        matches!(
                            observation["id"].as_str(),
                            Some("diagonal_down_forward" | "diagonal_up_forward")
                        )
                    })
                    .collect::<Vec<_>>();
                assert_eq!(diagonal_cases.len(), 2);
                for observation in diagonal_cases {
                    assert_eq!(
                        observation["inputs"]["values"].as_array().map(Vec::len),
                        Some(49)
                    );
                    assert_eq!(
                        observation["expected"]["magnitude"]
                            .as_array()
                            .map(Vec::len),
                        Some(49)
                    );
                    assert_eq!(
                        observation["expected"]["edges"].as_array().map(Vec::len),
                        Some(49)
                    );
                }
            }
            DILATION_OPERATION_ID | EROSION_OPERATION_ID | TOP_HAT_OPERATION_ID => {
                assert!(observations.iter().any(|observation| {
                    observation["id"] == "asymmetric_sparse_forward"
                        || observation["id"] == "asymmetric_sparse_composition"
                }));
                assert!(observations.iter().any(|observation| {
                    observation["expected"].get("inactive_sentinel").is_some()
                        || observation["expected"].get("erosion_sentinel").is_some()
                }));
            }
            EFFICIENTNET_V2_S_OPERATION_ID | RAFT_LARGE_OPERATION_ID => {
                assert!(
                    observations
                        .iter()
                        .any(|observation| observation["case"] == "architecture")
                );
                assert!(
                    observations
                        .iter()
                        .any(|observation| observation["case"] == "nonzero_execution")
                );
                assert!(
                    observations
                        .iter()
                        .any(|observation| observation["expected"]["eval_only"] == true)
                );
            }
            _ => {}
        }
        assert_eq!(
            evidence["ordered_parameters"],
            serde_json::from_str::<serde_json::Value>(contract.ordered_parameters_json)?
        );
        assert_eq!(
            evidence["output_types"],
            serde_json::from_str::<serde_json::Value>(contract.output_types_json)?
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
    assert!(source_derived_count > 0);
    assert!(independently_analytical_count > 0);
    assert!(native_validated_count > 0);

    assert_eq!(EINOPS_REARRANGE_OPERATION_ID, EXPECTED[0].0);
    assert_eq!(RGB_TO_LAB_OPERATION_ID, EXPECTED[1].0);
    assert_eq!(RGB_TO_YCBCR_OPERATION_ID, EXPECTED[2].0);
    assert_eq!(YCBCR_TO_RGB_OPERATION_ID, EXPECTED[3].0);
    assert_eq!(CANNY_OPERATION_ID, EXPECTED[4].0);
    assert_eq!(DILATION_OPERATION_ID, EXPECTED[5].0);
    assert_eq!(EROSION_OPERATION_ID, EXPECTED[6].0);
    assert_eq!(TOP_HAT_OPERATION_ID, EXPECTED[7].0);
    assert_eq!(EFFICIENTNET_V2_S_OPERATION_ID, EXPECTED[8].0);
    assert_eq!(RAFT_LARGE_OPERATION_ID, EXPECTED[9].0);
    assert_eq!(DEFORM_CONV2D_OPERATION_ID, EXPECTED[10].0);
    assert_eq!(TO_PIL_IMAGE_OPERATION_ID, EXPECTED[11].0);

    let model_source =
        fs::read_to_string(workspace.join("crates/comfy_model/src/vision_models.rs"))?;
    assert!(model_source.contains(&format!(
        "pub const EFFICIENTNET_V2_S_OPERATION_ID: &str = \"{}\";",
        EXPECTED[8].0
    )));
    assert!(model_source.contains(&format!(
        "pub const RAFT_LARGE_OPERATION_ID: &str = \"{}\";",
        EXPECTED[9].0
    )));
    Ok(())
}
