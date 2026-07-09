use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use world_model::{SimSourceInventory, SimSourceInventorySummary};
use world_model::{SimSourceItem, SimSourceKind};

use crate::{
    SIM_COVERAGE_DUPLICATE_OWNER_CODE, SIM_COVERAGE_IMPLEMENTED_WITHOUT_EVIDENCE_CODE,
    SIM_COVERAGE_INVALID_OWNER_PATH_CODE, SIM_COVERAGE_MISSING_OWNER_CODE,
    SIM_COVERAGE_UNSUPPORTED_WITHOUT_REASON_CODE, SimCoverageBoundaryDecision,
    SimCoverageDependencyGate, SimCoverageEvidence, SimCoverageEvidenceKind, SimCoverageLedger,
    SimCoverageOwner, SimCoverageOwnerResolver, SimCoverageRecord, SimCoverageStatus,
};

#[test]
fn validates_complete_sim_coverage_record() {
    let ledger = SimCoverageLedger::new(
        1,
        [implemented_record("route:server_py:POST__prompt")
            .with_owner(SimCoverageOwner::RuntimeControlPlane)],
    );

    assert!(ledger.validate().is_empty());
    assert_eq!(
        ledger.owner_for("route:server_py:POST__prompt"),
        Some(&SimCoverageOwner::RuntimeControlPlane)
    );
    assert_eq!(
        ledger
            .records_by_owner(SimCoverageOwner::RuntimeControlPlane)
            .len(),
        1
    );
}

#[test]
fn reports_missing_owner() {
    let ledger = SimCoverageLedger::new(
        1,
        [SimCoverageRecord::new(
            "route:server_py:POST__prompt",
            "projects/comfy/server.py",
            SimSourceKind::Route,
            SimCoverageStatus::Planned,
        )],
    );

    assert_diagnostic_code(&ledger, SIM_COVERAGE_MISSING_OWNER_CODE);
}

#[test]
fn reports_duplicate_source_owners() {
    let ledger = SimCoverageLedger::new(
        1,
        [
            implemented_record("route:server_py:POST__prompt")
                .with_owner(SimCoverageOwner::RuntimeControlPlane),
            implemented_record("route:server_py:POST__prompt")
                .with_owner(SimCoverageOwner::GraphNodeRuntime),
        ],
    );

    let diagnostics = ledger.validate();
    let duplicate = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == SIM_COVERAGE_DUPLICATE_OWNER_CODE)
        .expect("expected duplicate-owner diagnostic");
    assert!(
        duplicate
            .message
            .contains(".agents/specs/godot-migration/comfy-runtime-control-plane")
    );
    assert!(
        duplicate
            .message
            .contains(".agents/specs/godot-migration/comfy-graph-node-runtime")
    );
}

#[test]
fn reports_implemented_without_evidence() {
    let ledger = SimCoverageLedger::new(
        1,
        [SimCoverageRecord::new(
            "node:nodes_py:KSampler",
            "projects/comfy/nodes.py",
            SimSourceKind::CoreNode,
            SimCoverageStatus::Implemented,
        )
        .with_owner(SimCoverageOwner::GraphNodeRuntime)],
    );

    assert_diagnostic_code(&ledger, SIM_COVERAGE_IMPLEMENTED_WITHOUT_EVIDENCE_CODE);
}

#[test]
fn reports_unsupported_divergent_and_delegated_without_reason() {
    for status in [
        SimCoverageStatus::Unsupported,
        SimCoverageStatus::Divergent,
        SimCoverageStatus::Delegated,
    ] {
        let ledger = SimCoverageLedger::new(
            1,
            [SimCoverageRecord::new(
                "node:external_provider:OpenAIImage",
                "projects/comfy/comfy_api_nodes/apis/openai.py",
                SimSourceKind::ApiProviderNode,
                status,
            )
            .with_owner(SimCoverageOwner::ApiProviderNodes)],
        );

        assert_diagnostic_code(&ledger, SIM_COVERAGE_UNSUPPORTED_WITHOUT_REASON_CODE);
    }
}

#[test]
fn accepts_unsupported_record_with_user_visible_reason() {
    let ledger = SimCoverageLedger::new(
        1,
        [SimCoverageRecord::new(
            "node:external_provider:OpenAIImage",
            "projects/comfy/comfy_api_nodes/apis/openai.py",
            SimSourceKind::ApiProviderNode,
            SimCoverageStatus::Unsupported,
        )
        .with_owner(SimCoverageOwner::ApiProviderNodes)
        .with_boundary_decision(
            SimCoverageBoundaryDecision::new("Provider calls require explicit connector policy")
                .with_technical_reason("No real provider request is issued during fixture tests"),
        )],
    );

    assert!(ledger.validate().is_empty());
}

#[test]
fn reports_invalid_owner_path() {
    let mut record = implemented_record("route:server_py:POST__prompt")
        .with_owner(SimCoverageOwner::RuntimeControlPlane);
    record.owner_path = ".agents/specs/godot-migration/comfy-graph-node-runtime".to_string();
    let ledger = SimCoverageLedger::new(1, [record]);

    assert_diagnostic_code(&ledger, SIM_COVERAGE_INVALID_OWNER_PATH_CODE);
}

#[test]
fn validates_existing_sim_owner_paths_without_comfy_spec_claims() {
    let valid = SimCoverageLedger::new(
        1,
        [implemented_record("asset:storage:upload")
            .with_existing_sim_owner_path("crates/world_model/src/sim_asset_upload.rs")],
    );
    assert!(valid.validate().is_empty());

    let invalid = SimCoverageLedger::new(
        1,
        [implemented_record("asset:storage:upload")
            .with_existing_sim_owner_path(".agents/specs/godot-migration/comfy-asset-library")],
    );
    assert_diagnostic_code(&invalid, SIM_COVERAGE_INVALID_OWNER_PATH_CODE);
}

#[test]
fn serializes_sim_coverage_ledger() {
    let ledger = SimCoverageLedger::new(
        1,
        [implemented_record("route:server_py:POST__prompt")
            .with_owner(SimCoverageOwner::RuntimeControlPlane)],
    );

    let serialized = serde_json::to_string(&ledger).expect("failed to serialize ledger");
    let deserialized: SimCoverageLedger =
        serde_json::from_str(&serialized).expect("failed to deserialize ledger");
    assert_eq!(deserialized, ledger);
}

#[test]
fn suggests_runtime_control_plane_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::Route,
            "projects/comfy/server.py",
            "POST /prompt",
        ),
        SimCoverageOwner::RuntimeControlPlane,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::WebSocketProtocol,
            "projects/comfy/server.py",
            "GET /ws",
        ),
        SimCoverageOwner::RuntimeControlPlane,
    );
}

#[test]
fn suggests_graph_node_runtime_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::CoreNode,
            "projects/comfy/nodes.py",
            "Reroute",
        )
        .with_category("graph"),
        SimCoverageOwner::GraphNodeRuntime,
    );
}

#[test]
fn suggests_model_memory_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ModelFolder,
            "projects/comfy/folder_paths.py",
            "checkpoints",
        ),
        SimCoverageOwner::ModelMemoryRuntime,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ModelFamily,
            "projects/comfy/comfy/supported_models.py",
            "SDXL",
        ),
        SimCoverageOwner::ModelMemoryRuntime,
    );
}

#[test]
fn suggests_diffusion_world_model_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::CoreNode,
            "projects/comfy/nodes.py",
            "KSampler",
        ),
        SimCoverageOwner::DiffusionWorldModelRuntime,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ExtraNode,
            "projects/comfy/comfy_extras/nodes_model_merging.py",
            "ModelMergeSimple",
        )
        .with_category("advanced/model_merging"),
        SimCoverageOwner::DiffusionWorldModelRuntime,
    );
}

#[test]
fn suggests_asset_library_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::AssetApi,
            "projects/comfy/api_server/routes/internal/files.py",
            "asset upload",
        ),
        SimCoverageOwner::AssetLibrary,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ExtraNode,
            "projects/comfy/comfy_extras/nodes_asset_metadata.py",
            "AssetMetadata",
        ),
        SimCoverageOwner::AssetLibrary,
    );
}

#[test]
fn suggests_workflows_blueprints_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::Blueprint,
            "projects/comfy/blueprints/Text to Image.json",
            "Text to Image",
        ),
        SimCoverageOwner::WorkflowsBlueprints,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ExtraNode,
            "projects/comfy/comfy_extras/nodes_template.py",
            "WorkflowTemplate",
        ),
        SimCoverageOwner::WorkflowsBlueprints,
    );
}

#[test]
fn suggests_media_node_pipeline_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ExtraNode,
            "projects/comfy/comfy_extras/nodes_video.py",
            "VideoCombine",
        )
        .with_category("video"),
        SimCoverageOwner::MediaNodePipelines,
    );
}

#[test]
fn suggests_api_provider_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ApiProviderNode,
            "projects/comfy/comfy_api_nodes/apis/openai.py",
            "OpenAIImage",
        ),
        SimCoverageOwner::ApiProviderNodes,
    );
}

#[test]
fn suggests_extension_ecosystem_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ExtensionHook,
            "projects/comfy/app/extension_manager.py",
            "ComfyExtension",
        ),
        SimCoverageOwner::ExtensionEcosystem,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::ExtraNode,
            "projects/comfy/custom_nodes/example_node.py",
            "CustomNode",
        ),
        SimCoverageOwner::ExtensionEcosystem,
    );
}

#[test]
fn suggests_packaging_quality_owner() {
    assert_suggested_owner(
        SimSourceItem::classified(SimSourceKind::CliFlag, "projects/comfy/main.py", "--listen"),
        SimCoverageOwner::PackagingQuality,
    );
    assert_suggested_owner(
        SimSourceItem::classified(
            SimSourceKind::OpenApiOperation,
            "projects/comfy/openapi.yaml",
            "getQueueInfo",
        ),
        SimCoverageOwner::PackagingQuality,
    );
}

#[test]
fn coverage_ledger_fixture_maps_every_inventory_item_once() {
    let inventory = read_source_inventory_fixture();
    let ledger = read_coverage_ledger_fixture();

    assert_eq!(ledger.schema_version, 1);
    assert_eq!(ledger.records.len(), inventory.items.len());
    assert!(ledger.validate().is_empty());

    let inventory_by_id = inventory
        .items
        .iter()
        .map(|source_item| (source_item.source_id.as_str(), source_item))
        .collect::<BTreeMap<_, _>>();
    let mut ledger_source_ids = BTreeSet::new();

    for record in &ledger.records {
        assert!(
            ledger_source_ids.insert(record.source_id.as_str()),
            "duplicate coverage record for {}",
            record.source_id
        );
        let source_item = inventory_by_id
            .get(record.source_id.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "coverage record {} is missing from inventory",
                    record.source_id
                )
            });

        assert_eq!(record.source_path, source_item.source_path);
        assert_eq!(record.source_kind, source_item.source_kind);
        if record.status == SimCoverageStatus::Implemented {
            assert!(
                !record.evidence.is_empty(),
                "{} implemented records must keep evidence",
                record.source_id
            );
        }

        let owner = record.owner.expect("coverage record should have owner");
        let suggestion = SimCoverageOwnerResolver::suggest_owner(source_item);
        assert_eq!(owner, suggestion.owner);
        assert_eq!(record.owner_path, suggestion.owner_path);
    }
}

#[test]
fn coverage_ledger_fixture_owner_counts_are_recomputable() {
    let ledger = read_coverage_ledger_fixture();
    let counts = ledger.records.iter().fold(
        BTreeMap::<SimCoverageOwner, usize>::new(),
        |mut counts, record| {
            let owner = record.owner.expect("coverage record should have owner");
            *counts.entry(owner).or_default() += 1;
            counts
        },
    );

    assert_eq!(
        counts.values().sum::<usize>(),
        ledger.records.len(),
        "owner counts should account for every ledger record"
    );
    for owner in [
        SimCoverageOwner::RuntimeControlPlane,
        SimCoverageOwner::GraphNodeRuntime,
        SimCoverageOwner::ModelMemoryRuntime,
        SimCoverageOwner::DiffusionWorldModelRuntime,
        SimCoverageOwner::WorkflowsBlueprints,
        SimCoverageOwner::MediaNodePipelines,
        SimCoverageOwner::ApiProviderNodes,
        SimCoverageOwner::ExtensionEcosystem,
        SimCoverageOwner::PackagingQuality,
    ] {
        assert!(
            counts.get(&owner).copied().unwrap_or_default() > 0,
            "expected fixture coverage for {owner:?}"
        );
    }
}

#[test]
fn product_sequence_ranks_local_execution_before_policy_gated_tail_work() {
    for local_owner in [
        SimCoverageOwner::RuntimeControlPlane,
        SimCoverageOwner::GraphNodeRuntime,
        SimCoverageOwner::ModelMemoryRuntime,
        SimCoverageOwner::DiffusionWorldModelRuntime,
        SimCoverageOwner::AssetLibrary,
        SimCoverageOwner::WorkflowsBlueprints,
        SimCoverageOwner::MediaNodePipelines,
    ] {
        for tail_owner in [
            SimCoverageOwner::ApiProviderNodes,
            SimCoverageOwner::ExtensionEcosystem,
            SimCoverageOwner::PackagingQuality,
        ] {
            assert!(
                local_owner.product_sequence_rank() < tail_owner.product_sequence_rank(),
                "{local_owner:?} should rank ahead of {tail_owner:?}"
            );
            assert!(tail_owner.is_policy_gated_tail_work());
        }
    }
}

#[test]
fn product_sequence_ranks_existing_sim_owner_before_comfy_owners() {
    assert_eq!(
        SimCoverageOwner::ExistingSimSubsystem.product_sequence_rank(),
        0
    );
    for owner in [
        SimCoverageOwner::RuntimeControlPlane,
        SimCoverageOwner::GraphNodeRuntime,
        SimCoverageOwner::ModelMemoryRuntime,
        SimCoverageOwner::DiffusionWorldModelRuntime,
        SimCoverageOwner::AssetLibrary,
        SimCoverageOwner::WorkflowsBlueprints,
        SimCoverageOwner::MediaNodePipelines,
        SimCoverageOwner::ApiProviderNodes,
        SimCoverageOwner::ExtensionEcosystem,
        SimCoverageOwner::PackagingQuality,
    ] {
        assert!(
            SimCoverageOwner::ExistingSimSubsystem.product_sequence_rank()
                < owner.product_sequence_rank(),
            "existing Sim owner should avoid duplicate Comfy-owned infrastructure for {owner:?}"
        );
    }
}

#[test]
fn coverage_ledger_fixture_orders_local_gaps_before_provider_extension_and_packaging() {
    let ledger = read_coverage_ledger_fixture();
    let ordered = ledger.uncovered_records_by_product_sequence();
    if ordered.is_empty() {
        assert!(
            ledger.records.iter().all(|record| {
                record.status == SimCoverageStatus::Implemented && !record.evidence.is_empty()
            }),
            "closed coverage ledger must keep evidence on every implemented record"
        );
        return;
    }

    let first_tail_index = ordered
        .iter()
        .position(|record| {
            record
                .owner
                .expect("coverage record should have owner")
                .is_policy_gated_tail_work()
        })
        .expect("fixture should include provider, extension, or packaging gaps");
    if let Some(last_local_index) = ordered.iter().rposition(|record| {
        !record
            .owner
            .expect("coverage record should have owner")
            .is_policy_gated_tail_work()
    }) {
        assert!(
            last_local_index < first_tail_index,
            "local execution and authoring gaps must sort before provider/extension/packaging hardening"
        );
    } else {
        assert!(
            ordered.iter().all(|record| record
                .owner
                .expect("coverage record should have owner")
                .is_policy_gated_tail_work()),
            "when local execution gaps are complete, remaining gaps must be policy-gated tail work"
        );
    }
}

#[test]
fn coverage_ledger_fixture_assigns_every_missing_record_to_owner_backlog() {
    let ledger = read_coverage_ledger_fixture();

    for record in ledger.records.iter().filter(|record| {
        record.status != SimCoverageStatus::Implemented || record.evidence.is_empty()
    }) {
        let backlog_task = record
            .backlog_task
            .as_ref()
            .unwrap_or_else(|| panic!("{} must have a backlog task", record.source_id));
        assert!(
            backlog_task.task_id.starts_with("coverage-backlog-"),
            "{} must use a stable coverage backlog task id",
            record.source_id
        );
        assert_eq!(
            Some(backlog_task.owner_spec.as_str()),
            record
                .owner_path
                .strip_suffix('/')
                .or(Some(record.owner_path.as_str())),
            "{} backlog owner spec must match coverage owner path",
            record.source_id
        );
        assert!(
            !backlog_task.expected_writes.is_empty(),
            "{} backlog task must list expected writes",
            record.source_id
        );
        assert!(
            !backlog_task.validation.trim().is_empty(),
            "{} backlog task must list validation",
            record.source_id
        );
        assert!(
            !backlog_task.evidence_policy.trim().is_empty(),
            "{} backlog task must list parity evidence policy",
            record.source_id
        );
    }
}

#[test]
fn owner_spec_backlog_tasks_reference_ledger_task_ids() {
    let ledger = read_coverage_ledger_fixture();
    let mut backlog_tasks = BTreeMap::<String, &crate::SimCoverageBacklogRef>::new();
    for record in ledger.records.iter().filter(|record| {
        record.status != SimCoverageStatus::Implemented || record.evidence.is_empty()
    }) {
        if let Some(backlog_task) = record.backlog_task.as_ref() {
            backlog_tasks
                .entry(backlog_task.task_id.clone())
                .or_insert(backlog_task);
        }
    }

    for backlog_task in backlog_tasks.values() {
        let manifest = read_repo_file(format!("{}/tasks.md", backlog_task.owner_spec));
        assert!(
            manifest.contains(&format!("_CoverageTask: {}_", backlog_task.task_id)),
            "{} must be referenced in {}",
            backlog_task.task_id,
            backlog_task.owner_spec
        );
        assert!(
            manifest.contains("Coverage IDs:"),
            "{} must document coverage IDs",
            backlog_task.task_id
        );
        assert!(
            manifest.contains("Expected native Sim writes:"),
            "{} must document expected writes",
            backlog_task.task_id
        );
        assert!(
            manifest.contains("Validation:"),
            "{} must document validation command",
            backlog_task.task_id
        );
        assert!(
            manifest.contains("Parity evidence:"),
            "{} must document parity evidence",
            backlog_task.task_id
        );
    }
}

#[test]
fn policy_gated_tail_work_records_gate_reason_when_selected_early() {
    let provider_record = SimCoverageRecord::new(
        "provider:openai:image",
        "projects/comfy/comfy_api_nodes/apis/openai.py",
        SimSourceKind::ApiProviderNode,
        SimCoverageStatus::Planned,
    )
    .with_owner(SimCoverageOwner::ApiProviderNodes)
    .with_dependency_gate(SimCoverageDependencyGate::new(
        "dependency-review.provider-openai",
        ".agents/specs/godot-migration/comfy-api-provider-nodes/tasks.md",
    ));

    assert!(
        provider_record
            .owner
            .expect("provider record should have owner")
            .is_policy_gated_tail_work()
    );
    assert!(
        provider_record.dependency_gate.is_some(),
        "tail work selected before local parity must carry a gate reason"
    );
}

#[test]
fn implemented_coverage_fixture_records_require_native_sim_evidence() {
    let ledger = read_coverage_ledger_fixture();

    for record in ledger
        .records
        .iter()
        .filter(|record| record.status == SimCoverageStatus::Implemented)
    {
        assert!(
            !record.evidence.is_empty(),
            "implemented record {} must have evidence",
            record.source_id
        );
        for evidence in &record.evidence {
            assert!(
                !evidence.reference.contains("comfyui_passthrough"),
                "implemented evidence must not point at pass-through markers"
            );
        }
    }
}

#[test]
fn existing_comfy_fixtures_are_native_sim_records() {
    for fixture_path in [
        "basic_api_prompt.json",
        "blueprints_manifest.json",
        "core_nodes.json",
        "model_execution_manifest.json",
        "provider_nodes.json",
    ] {
        let fixture: serde_json::Value = serde_json::from_str(&read_comfy_fixture(fixture_path))
            .unwrap_or_else(|error| {
                panic!("failed to parse fixture {fixture_path}: {error}");
            });
        assert_eq!(
            fixture["native_sim_records"], true,
            "{fixture_path} should be marked as native Sim data"
        );
        assert_eq!(
            fixture["comfyui_passthrough"], false,
            "{fixture_path} should not rely on ComfyUI pass-through"
        );
    }
}

fn implemented_record(source_id: &str) -> SimCoverageRecord {
    SimCoverageRecord::new(
        source_id,
        "projects/comfy/server.py",
        SimSourceKind::Route,
        SimCoverageStatus::Implemented,
    )
    .with_evidence(SimCoverageEvidence::new(
        SimCoverageEvidenceKind::Test,
        "crates/world_model/tests/comfy_api_compat.rs",
    ))
}

fn assert_suggested_owner(source_item: SimSourceItem, owner: SimCoverageOwner) {
    let suggestion = SimCoverageOwnerResolver::suggest_owner(&source_item);
    assert_eq!(suggestion.owner, owner);
    assert_eq!(suggestion.owner_path, owner.expected_owner_path());
    assert!(
        suggestion.reason.contains(&source_item.symbol),
        "expected suggestion reason to mention source item, got {}",
        suggestion.reason
    );
}

fn read_source_inventory_fixture() -> SimSourceInventory {
    let inventory: SimSourceInventory =
        serde_json::from_str(&read_comfy_fixture("source_inventory.json"))
            .expect("failed to parse source inventory fixture");
    assert_eq!(
        inventory.summary,
        SimSourceInventorySummary::from_items(&inventory.items)
    );
    inventory
}

fn read_coverage_ledger_fixture() -> SimCoverageLedger {
    serde_json::from_str(&read_comfy_fixture("coverage_ledger.json"))
        .expect("failed to parse coverage ledger fixture")
}

fn read_comfy_fixture(file_name: &str) -> String {
    fs::read_to_string(comfy_fixture_path(file_name)).unwrap_or_else(|error| {
        panic!("failed to read Comfy fixture {file_name}: {error}");
    })
}

fn comfy_fixture_path(file_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../world_model/fixtures/comfy")
        .join(file_name)
}

fn read_repo_file(path: impl AsRef<Path>) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path.as_ref()),
    )
    .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.as_ref().display()))
}

fn assert_diagnostic_code(ledger: &SimCoverageLedger, code: &str) {
    let diagnostics = ledger.validate();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code == code),
        "expected diagnostic code {code}, got {diagnostics:?}"
    );
}
