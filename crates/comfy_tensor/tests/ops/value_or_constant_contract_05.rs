use comfy_tensor::{
    ContractInventoryKind, GENERATED_OPERATION_RESOLUTION_MODULES,
    GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES, OPERATION_CONTRACTS, TypedReferenceContract,
    generated_value_or_constant_contract_05 as leaf,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io, path::Path};

const OWNER: &str =
    "comfy-parity-tensor-ops-value-or-constant-contract-comfy-tensor-op-b37b2e52befe";
type ContractFacade = fn() -> Option<TypedReferenceContract>;

fn facades() -> [ContractFacade; 12] {
    [
        leaf::cudnn_benchmark_contract,
        leaf::torch_complex64_contract,
        leaf::torch_float_contract,
        leaf::torch_float8_e4m3fn_contract,
        leaf::torch_float8_e5m2fnuz_contract,
        leaf::torch_int16_contract,
        leaf::torch_nn_hardswish_contract,
        leaf::torch_nn_selu_contract,
        leaf::xpu_total_memory_contract,
        leaf::interpolation_nearest_contract,
        leaf::functional_interpolation_bicubic_contract,
        leaf::xformers_module_version_contract,
    ]
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, io::Error> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other(format!("reference evidence is missing {field}")))
}

#[test]
fn value_and_constant_facades_use_the_canonical_reference_owner()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(leaf::ASSIGNED_VALUE_OR_CONSTANT_REFERENCES.len(), 12);
    for (facade, (operation_id, semantic)) in facades()
        .into_iter()
        .zip(leaf::ASSIGNED_VALUE_OR_CONSTANT_REFERENCES)
    {
        let contract =
            facade().ok_or_else(|| format!("typed contract is missing for {operation_id}"))?;
        assert_eq!(contract.operation_id(), *operation_id);
        assert_eq!(contract.semantic(), *semantic);
        assert_eq!(
            contract.inventory_kind(),
            ContractInventoryKind::NamespaceValueReference
        );
        assert_eq!(
            leaf::assigned_value_or_constant_contract(operation_id),
            Some(contract)
        );
    }
    assert!(leaf::assigned_value_or_constant_contract("COMFY-TENSOR-OP-UNASSIGNED").is_none());
    Ok(())
}

#[test]
fn assigned_catalog_rows_are_exact_and_never_enter_kernel_resolution() {
    let assigned_ids = leaf::ASSIGNED_VALUE_OR_CONSTANT_REFERENCES
        .iter()
        .map(|(operation_id, _)| *operation_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(assigned_ids.len(), 12);
    assert!(!GENERATED_OPERATION_RESOLUTION_MODULES.contains(&"value_or_constant_contract_05"));
    assert!(
        GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES
            .iter()
            .all(|slice| slice.module_name != "value_or_constant_contract_05")
    );
    assert!(
        GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES
            .iter()
            .flat_map(|slice| slice.iter())
            .all(|contract| !assigned_ids.contains(contract.operation_id))
    );
}

#[test]
fn no_unassigned_catalog_row_is_claimed() {
    let assigned_operation_ids = leaf::ASSIGNED_VALUE_OR_CONSTANT_REFERENCES
        .iter()
        .map(|(operation_id, _)| *operation_id)
        .collect::<BTreeSet<_>>();
    let mut claimed_operation_ids = BTreeSet::new();
    for contract in OPERATION_CONTRACTS {
        let Some(reference) = contract.typed_reference() else {
            continue;
        };
        if assigned_operation_ids.contains(reference.operation_id()) {
            assert!(leaf::assigned_value_or_constant_contract(reference.operation_id()).is_some());
            assert!(claimed_operation_ids.insert(reference.operation_id()));
        } else {
            assert!(leaf::assigned_value_or_constant_contract(reference.operation_id()).is_none());
        }
    }
    assert_eq!(claimed_operation_ids, assigned_operation_ids);
}

#[test]
fn typed_reference_evidence_is_exact_and_hash_sealed() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture_directory = workspace_root
        .join("crates/comfy_test_support/fixtures/tensor_operations/value_or_constant_contract_05");
    let fixture_entries = fs::read_dir(&fixture_directory)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(fixture_entries.len(), 12);
    assert!(fixture_entries.iter().all(|entry| {
        entry
            .file_type()
            .is_ok_and(|file_type| file_type.is_file() && !file_type.is_symlink())
            && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
    }));
    let expected_fixture_fields = [
        "schema_version",
        "resolution_module",
        "operation_id",
        "baseline_overload_id",
        "baseline_fixture_sha256",
        "owner_task_id",
        "inventory_kind",
        "canonical_target",
        "reference_semantic",
        "canonical_rust_semantic",
        "executable",
        "source_observations",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    for (file_name, expected_semantic, expected_digest) in [
        (
            "cudnn_benchmark.json",
            "BooleanCapabilityReference::CudnnBenchmark",
            "87b64815c48ac9497080ca46a8b569f7033b27e09df866a3c3877cc9d2c2536c",
        ),
        (
            "torch_complex64.json",
            "DType::Complex64",
            "3b3f8a174b4d2de5247ea6416ae20dfbc60e4098339541c64dfdf9a2af2feec1",
        ),
        (
            "torch_float.json",
            "DType::F32",
            "8e6e1965ca17f8b62321bb67d9888c97f2dbcf4ef3bcdbb1c12436df739c64ed",
        ),
        (
            "torch_float8_e4m3fn.json",
            "DType::Float8E4m3Fn",
            "2cf3d51f9a47e6bf1d2b1ba295804d16a9d8a5d195d7900c12635490f93a83ce",
        ),
        (
            "torch_float8_e5m2fnuz.json",
            "DType::Float8E5m2Fnuz",
            "39225299abc0b1182bc0a5a89b03749b1e36330239e365d6cd3a7ba0c2199f69",
        ),
        (
            "torch_int16.json",
            "DType::I16",
            "c92e79f2486d30ee1f1f3985a050bbba2640f93eddd50ef1ad9dca3bd8bec653",
        ),
        (
            "torch_nn_hardswish.json",
            "FunctionReference::Hardswish",
            "f5b5d457c30a416ac6e9b372f47b5d750e02fd685305be66d8869ecb8a019f80",
        ),
        (
            "torch_nn_selu.json",
            "FunctionReference::Selu",
            "cd436dafafd1008af3c0baefb030f9113cf9274cb26af0c1e81d6d90dc3edc21",
        ),
        (
            "xpu_total_memory.json",
            "DevicePropertyReference::XpuTotalMemory",
            "b1da1ff94d1817fc71b53aca020f9b4837851de55bbdd108682a10c1cc30a716",
        ),
        (
            "interpolation_nearest.json",
            "EnumVariantReference::InterpolationNearest",
            "f2fcad6e61e9137c56c13cb927e27c20a49619dc884e910878624b66bfdcbcf1",
        ),
        (
            "functional_interpolation_bicubic.json",
            "EnumVariantReference::FunctionalInterpolationBicubic",
            "08ebad1dbd740d370dcd0df6e6d1d1bbbc70055752103bfd5deae8c5ffda4827",
        ),
        (
            "xformers_module_version.json",
            "VersionValueReference::XformersModule",
            "9edd779a2933d9bff85ee21c020d558ef2e60d437035f321b1de35333df68135",
        ),
    ] {
        let fixture_path = fixture_directory.join(file_name);
        let bytes = fs::read(fixture_path)?;
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected_digest);
        let fixture: Value = serde_json::from_slice(&bytes)?;
        let fixture_fields = fixture
            .as_object()
            .ok_or("reference evidence must be an object")?
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(fixture_fields, expected_fixture_fields);
        assert_eq!(
            fixture.get("schema_version").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            required_string(&fixture, "resolution_module")?,
            "value_or_constant_contract_05"
        );
        assert_eq!(required_string(&fixture, "owner_task_id")?, OWNER);
        assert_eq!(
            required_string(&fixture, "inventory_kind")?,
            "namespace_value_reference"
        );
        assert_eq!(
            required_string(&fixture, "canonical_rust_semantic")?,
            expected_semantic
        );
        assert_eq!(
            fixture.get("executable").and_then(Value::as_bool),
            Some(false)
        );
        let observations = fixture
            .get("source_observations")
            .and_then(Value::as_array)
            .ok_or("reference evidence is missing source observations")?;
        assert!(!observations.is_empty());
        for observation in observations {
            let fields = observation
                .as_object()
                .ok_or("source observation must be an object")?;
            assert_eq!(
                fields.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                BTreeSet::from(["path", "line", "use"])
            );
            let path = required_string(observation, "path")?;
            assert!(!path.is_empty());
            assert!(!Path::new(path).is_absolute());
            assert!(!Path::new(path).components().any(|component| matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )));
            assert!(
                observation
                    .get("line")
                    .and_then(Value::as_u64)
                    .is_some_and(|line| line > 0)
            );
            assert!(!required_string(observation, "use")?.is_empty());
        }

        let operation_id = required_string(&fixture, "operation_id")?;
        let contract = leaf::assigned_value_or_constant_contract(operation_id)
            .ok_or_else(|| format!("fixture claims an unassigned operation {operation_id}"))?;
        assert_eq!(
            required_string(&fixture, "canonical_target")?,
            contract.canonical_target()
        );
        assert_eq!(
            required_string(&fixture, "baseline_overload_id")?,
            format!("{operation_id}:reference")
        );

        let operation_suffix = operation_id
            .strip_prefix("COMFY-TENSOR-OP-")
            .ok_or("operation ID has an invalid prefix")?
            .to_ascii_lowercase();
        let baseline_path = workspace_root.join(format!(
            "crates/comfy_test_support/fixtures/tensor_signatures/contracts/comfy-tensor-op-{operation_suffix}.json"
        ));
        let baseline_bytes = fs::read(baseline_path)?;
        let baseline_digest = format!("{:x}", Sha256::digest(&baseline_bytes));
        assert_eq!(
            required_string(&fixture, "baseline_fixture_sha256")?,
            baseline_digest
        );
        assert_ne!(baseline_digest, expected_digest);
        let baseline: Value = serde_json::from_slice(&baseline_bytes)?;
        assert_eq!(required_string(&baseline, "operation_id")?, operation_id);
        assert_eq!(
            required_string(&baseline, "canonical_target")?,
            contract.canonical_target()
        );
        assert_eq!(
            baseline.get("reference_semantic"),
            fixture.get("reference_semantic")
        );
        let reference_semantic = fixture
            .get("reference_semantic")
            .and_then(Value::as_object)
            .ok_or("reference semantic must be an object")?;
        assert_eq!(
            reference_semantic
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["category", "value"])
        );
    }
    Ok(())
}
