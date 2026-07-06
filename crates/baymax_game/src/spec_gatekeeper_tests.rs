use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use pretty_assertions::assert_eq;

use crate::{
    ExecutionGate, GateDecision, MigrationGatekeeper, MigrationTaskRef, MigrationValidationError,
    MigrationValidationReport, SpecGatekeeper, SpecRoot,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_spec_root(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is earlier than unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("baymax-gatekeeper-{name}-{timestamp}"));
    fs::create_dir_all(&path).expect("failed to create spec root");
    path
}

fn create_grouped_spec(spec_root: &Path, name: &str) {
    let spec_path = spec_root.join(name);
    fs::create_dir_all(&spec_path).expect("failed to create grouped spec directory");
    for file in SpecGatekeeper::REQUIRED_SPEC_FILES {
        fs::write(spec_path.join(file), format!("# {file}\n"))
            .expect("failed to write grouped spec document");
    }
}

/// Write a tasks.md with explicit `_Requirements:` and `_writes:` lines.
fn write_tasks_with_manifests(spec_path: &Path, content: &str) {
    fs::write(spec_path.join("tasks.md"), content).expect("failed to write tasks.md");
}

// ---------------------------------------------------------------------------
// Spec-pack validation tests
// ---------------------------------------------------------------------------

#[test]
fn validates_complete_spec_pack_with_task_manifests() {
    let root_path = create_spec_root("complete");
    create_grouped_spec(&root_path, "engine-core-runtime");
    create_grouped_spec(&root_path, "language-scripting");

    // Rewrite tasks.md with proper manifests
    let tasks_content = r#"# Tasks

- [ ] 1. Detect Godot project descriptor
  - Parse project.godot manifest.
  - _Requirements: 3.1, 3.2_
  - _writes: crates/baymax_game/src/project.rs

- [ ] 2. Register GDScript language support
  - Add GDScript detection and syntax configuration.
  - _Requirements: 2.1, 2.2_
  - _writes: crates/languages/src/gdscript.rs, crates/baymax_game/src/language.rs
"#;

    write_tasks_with_manifests(&root_path.join("engine-core-runtime"), tasks_content);

    let gatekeeper = SpecGatekeeper::new(vec![
        "engine-core-runtime".into(),
        "language-scripting".into(),
    ]);
    let root = SpecRoot::new(&root_path);
    let report = gatekeeper.validate_spec_pack(&root);

    assert!(report.is_valid(), "Expected no errors, got: {report:?}");
}

#[test]
fn reports_missing_task_requirement_refs() {
    let root_path = create_spec_root("missing-reqs");
    let spec_path = root_path.join("platform-export");
    fs::create_dir_all(&spec_path).expect("failed to create spec directory");
    for file in SpecGatekeeper::REQUIRED_SPEC_FILES {
        fs::write(spec_path.join(file), format!("# {file}\n"))
            .expect("failed to write spec document");
    }

    // Write tasks.md without _Requirements:
    let tasks_content = r#"# Tasks

- [ ] 1. Add export preset parsing
  - Parse export_presets.cfg.
  - _writes: crates/baymax_game/src/export.rs
"#;
    write_tasks_with_manifests(&root_path.join("platform-export"), tasks_content);

    let gatekeeper = SpecGatekeeper::new(vec!["platform-export".into()]);
    let root = SpecRoot::new(&root_path);
    let report = gatekeeper.validate_spec_pack(&root);

    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingTaskRequirementRefs {
                task: "1. Add export preset parsing".to_string(),
                spec: "platform-export".to_string(),
            })
    );
    assert!(
        !report
            .errors
            .contains(&MigrationValidationError::MissingTaskWriteTargets {
                task: "1. Add export preset parsing".to_string(),
                spec: "platform-export".to_string(),
            })
    );
}

#[test]
fn reports_missing_task_write_targets() {
    let root_path = create_spec_root("missing-writes");
    let spec_path = root_path.join("mesh-generation");
    fs::create_dir_all(&spec_path).expect("failed to create spec directory");
    for file in SpecGatekeeper::REQUIRED_SPEC_FILES {
        fs::write(spec_path.join(file), format!("# {file}\n"))
            .expect("failed to write spec document");
    }

    // Write tasks.md without _writes:
    let tasks_content = r#"# Tasks

- [ ] 1. Generate mesh from prompt
  - Call mesh backend and store output.
  - _Requirements: 7.1, 7.2
"#;
    write_tasks_with_manifests(&root_path.join("mesh-generation"), tasks_content);

    let gatekeeper = SpecGatekeeper::new(vec!["mesh-generation".into()]);
    let root = SpecRoot::new(&root_path);
    let report = gatekeeper.validate_spec_pack(&root);

    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingTaskWriteTargets {
                task: "1. Generate mesh from prompt".to_string(),
                spec: "mesh-generation".to_string(),
            })
    );
    assert!(
        !report
            .errors
            .contains(&MigrationValidationError::MissingTaskRequirementRefs {
                task: "1. Generate mesh from prompt".to_string(),
                spec: "mesh-generation".to_string(),
            })
    );
}

#[test]
fn reports_missing_spec_file_with_gatekeeper() {
    let root_path = create_spec_root("missing-file");
    let spec_path = root_path.join("physics-navigation");
    fs::create_dir_all(&spec_path).expect("failed to create spec directory");
    // Only write requirements.md and tasks.md, skip design.md
    fs::write(spec_path.join("requirements.md"), "# Requirements\n")
        .expect("failed to write requirements");
    fs::write(spec_path.join("tasks.md"), "# Tasks\n").expect("failed to write tasks");

    let gatekeeper = SpecGatekeeper::new(vec!["physics-navigation".into()]);
    let root = SpecRoot::new(&root_path);
    let report = gatekeeper.validate_spec_pack(&root);

    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingSpecFile {
                spec: "physics-navigation".to_string(),
                file: "design.md".to_string(),
            })
    );
}

#[test]
fn auto_discovers_spec_directories() {
    let root_path = create_spec_root("auto-discover");
    create_grouped_spec(&root_path, "rendering-media");
    create_grouped_spec(&root_path, "networking-collaboration");

    // Give both specs proper task manifests
    let tasks = r#"# Tasks

- [ ] 1. Classify media files
  - _Requirements: 3.1
  - _writes: crates/baymax_game/src/media.rs
"#;
    write_tasks_with_manifests(&root_path.join("rendering-media"), tasks);
    write_tasks_with_manifests(&root_path.join("networking-collaboration"), tasks);

    // Empty spec_names triggers auto-discovery
    let gatekeeper = SpecGatekeeper::default();
    let root = SpecRoot::new(&root_path);
    let report = gatekeeper.validate_spec_pack(&root);

    assert!(report.is_valid(), "Expected no errors, got: {report:?}");
}

#[test]
fn parses_checkbox_task_formats() {
    let root_path = create_spec_root("checkbox-formats");
    let spec_path = root_path.join("world-model");
    fs::create_dir_all(&spec_path).expect("failed to create spec directory");
    for file in SpecGatekeeper::REQUIRED_SPEC_FILES {
        fs::write(spec_path.join(file), format!("# {file}\n"))
            .expect("failed to write spec document");
    }

    // Test that both [ ] and [x] task markers are parsed
    let tasks_content = r#"# Implementation Plan

- [ ] 1. Add request types
  - Define generation request and control structs.
  - _Requirements: 5.1, 5.4_
  - _writes: crates/world_model/src/request.rs

- [x] 2. Add action control parsing
  - Port WASD/IJKL semantics.
  - _Requirements: 5.1, 5.2_
  - _writes: crates/world_model/src/controls.rs
"#;
    write_tasks_with_manifests(&root_path.join("world-model"), tasks_content);

    let gatekeeper = SpecGatekeeper::new(vec!["world-model".into()]);
    let root = SpecRoot::new(&root_path);
    let report = gatekeeper.validate_spec_pack(&root);

    assert!(report.is_valid(), "Expected no errors, got: {report:?}");
}

// ---------------------------------------------------------------------------
// Gate decision tests
// ---------------------------------------------------------------------------

#[test]
fn allows_task_when_all_required_gates_are_satisfied() {
    let task = MigrationTaskRef::new("task-1", "engine-core-runtime").with_gates(BTreeSet::from([
        ExecutionGate::SpecConsistency,
        ExecutionGate::BoundaryPolicy,
    ]));

    let satisfied = BTreeSet::from([
        ExecutionGate::SpecConsistency,
        ExecutionGate::BoundaryPolicy,
        ExecutionGate::SharedBaymaxGameMetadata,
    ]);

    let gatekeeper = SpecGatekeeper::default();
    let decision = gatekeeper.can_execute_task(&task, &satisfied);

    assert_eq!(decision, GateDecision::Allowed);
}

#[test]
fn blocks_task_when_required_gates_are_unsatisfied() {
    let task = MigrationTaskRef::new("task-3", "world-model-runtime").with_gates(BTreeSet::from([
        ExecutionGate::SpecConsistency,
        ExecutionGate::SharedWorldModelFoundations,
        ExecutionGate::WorkerSafety,
    ]));

    // WorkerSafety is missing
    let satisfied = BTreeSet::from([
        ExecutionGate::SpecConsistency,
        ExecutionGate::SharedWorldModelFoundations,
    ]);

    let gatekeeper = SpecGatekeeper::default();
    let decision = gatekeeper.can_execute_task(&task, &satisfied);

    let blocked = match decision {
        GateDecision::Blocked(ref reasons) => reasons.clone(),
        _ => vec![],
    };
    assert!(
        blocked
            .iter()
            .any(|r| r.contains("G4") && r.contains("WorkerSafety")),
        "Expected blocking reason mentioning G4/WorkerSafety, got: {blocked:?}"
    );
}

#[test]
fn allows_task_with_no_required_gates() {
    let task = MigrationTaskRef::new("task-0", "planning");
    let satisfied = BTreeSet::new();

    let gatekeeper = SpecGatekeeper::default();
    let decision = gatekeeper.can_execute_task(&task, &satisfied);

    assert_eq!(decision, GateDecision::Allowed);
}

#[test]
fn blocks_task_when_multiple_gates_unsatisfied() {
    let task = MigrationTaskRef::new("task-5", "comfy-integration").with_gates(BTreeSet::from([
        ExecutionGate::SpecConsistency,
        ExecutionGate::BoundaryPolicy,
        ExecutionGate::ComfyHarnessAlignment,
    ]));

    let satisfied = BTreeSet::from([ExecutionGate::SpecConsistency]);

    let gatekeeper = SpecGatekeeper::default();
    let decision = gatekeeper.can_execute_task(&task, &satisfied);

    match decision {
        GateDecision::Blocked(reasons) => {
            assert_eq!(reasons.len(), 2, "Expected two blocking reasons");
            assert!(reasons.iter().any(|r| r.contains("G1")));
            assert!(reasons.iter().any(|r| r.contains("G8")));
        }
        _ => panic!("Expected Blocked decision"),
    }
}

// ---------------------------------------------------------------------------
// ExecutionGate label tests
// ---------------------------------------------------------------------------

#[test]
fn execution_gate_labels_match_expected_pattern() {
    assert_eq!(ExecutionGate::SpecConsistency.label(), "G0");
    assert_eq!(ExecutionGate::BoundaryPolicy.label(), "G1");
    assert_eq!(ExecutionGate::SharedBaymaxGameMetadata.label(), "G2");
    assert_eq!(ExecutionGate::SharedWorldModelFoundations.label(), "G3");
    assert_eq!(ExecutionGate::WorkerSafety.label(), "G4");
    assert_eq!(ExecutionGate::GraphSafety.label(), "G5");
    assert_eq!(ExecutionGate::Provenance.label(), "G6");
    assert_eq!(ExecutionGate::DependencyReview.label(), "G7");
    assert_eq!(ExecutionGate::ComfyHarnessAlignment.label(), "G8");
}

// ---------------------------------------------------------------------------
// Integration: parse_task_manifests
// ---------------------------------------------------------------------------

#[test]
fn reports_missing_manifests_on_multiple_tasks() {
    let content = r#"# Tasks

- [ ] 1. No manifests at all
  - Some description without any markers.

- [ ] 2. Only requirements
  - _Requirements: 1.1_
  - Missing writes.

- [ ] 3. Only writes
  - _writes: some/path.rs
  - Missing requirements.

- [ ] 4. Both present
  - _Requirements: 2.1, 2.2_
  - _writes: some/path.rs
"#;

    let mut report = MigrationValidationReport::default();
    SpecGatekeeper::parse_task_manifests(content, "test-spec", &mut report);

    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingTaskRequirementRefs {
                task: "1. No manifests at all".to_string(),
                spec: "test-spec".to_string(),
            })
    );
    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingTaskWriteTargets {
                task: "1. No manifests at all".to_string(),
                spec: "test-spec".to_string(),
            })
    );

    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingTaskWriteTargets {
                task: "2. Only requirements".to_string(),
                spec: "test-spec".to_string(),
            })
    );

    assert!(
        report
            .errors
            .contains(&MigrationValidationError::MissingTaskRequirementRefs {
                task: "3. Only writes".to_string(),
                spec: "test-spec".to_string(),
            })
    );

    // Task 4 should have no errors
    assert!(
        !report
            .errors
            .contains(&MigrationValidationError::MissingTaskRequirementRefs {
                task: "4. Both present".to_string(),
                spec: "test-spec".to_string(),
            })
    );
    assert!(
        !report
            .errors
            .contains(&MigrationValidationError::MissingTaskWriteTargets {
                task: "4. Both present".to_string(),
                spec: "test-spec".to_string(),
            })
    );
}

#[test]
fn handles_empty_tasks_file() {
    let content = "";
    let mut report = MigrationValidationReport::default();
    SpecGatekeeper::parse_task_manifests(content, "empty-spec", &mut report);
    assert!(report.is_valid());
}

#[test]
fn handles_tasks_file_with_no_checkbox_lines() {
    let content = "# Just a heading\n\nSome text without tasks.\n";
    let mut report = MigrationValidationReport::default();
    SpecGatekeeper::parse_task_manifests(content, "no-tasks", &mut report);
    assert!(report.is_valid());
}
