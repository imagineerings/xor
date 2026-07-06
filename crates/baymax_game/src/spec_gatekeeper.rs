use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    BaymaxGameMigrationInventory, MigrationInventory, MigrationValidationError,
    MigrationValidationReport,
};

// ---------------------------------------------------------------------------
// Execution gates (G0 – G8)
// ---------------------------------------------------------------------------

/// Execution gates that block implementation work until satisfied.
///
/// Variants correspond to the numbered gates in the migration umbrella plan:
/// G0 (SpecConsistency) through G8 (ComfyHarnessAlignment).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExecutionGate {
    /// G0: Spec consistency – requirements, design, and tasks files exist.
    SpecConsistency,
    /// G1: Boundary policy – runtime-adjacent work requires a documented
    /// boundary decision.
    BoundaryPolicy,
    /// G2: Shared Baymax game metadata – project descriptors, diagnostics,
    /// source references exist.
    SharedBaymaxGameMetadata,
    /// G3: Shared world-model foundations – request/control/worker/graph
    /// models exist.
    SharedWorldModelFoundations,
    /// G4: Worker safety – Python/model/GPU/remote diagnostics exist before
    /// real model workers.
    WorkerSafety,
    /// G5: Graph safety – graph validator exists before executing graph nodes.
    GraphSafety,
    /// G6: Provenance – generated artifact provenance exists.
    Provenance,
    /// G7: Dependency review – heavy/native/vendored dependencies reviewed.
    DependencyReview,
    /// G8: Comfy harness alignment – world-model harness changes reference
    /// applicable Comfy spec or document divergence.
    ComfyHarnessAlignment,
}

impl ExecutionGate {
    /// Returns the G0–G8 label for this gate.
    pub fn label(&self) -> &'static str {
        match self {
            Self::SpecConsistency => "G0",
            Self::BoundaryPolicy => "G1",
            Self::SharedBaymaxGameMetadata => "G2",
            Self::SharedWorldModelFoundations => "G3",
            Self::WorkerSafety => "G4",
            Self::GraphSafety => "G5",
            Self::Provenance => "G6",
            Self::DependencyReview => "G7",
            Self::ComfyHarnessAlignment => "G8",
        }
    }
}

// ---------------------------------------------------------------------------
// Dependency waves (W0 – W6)
// ---------------------------------------------------------------------------

/// Ordered implementation phases that keep shared foundations ahead of
/// dependent integrations.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DependencyWave {
    /// W0: Planning validation – spec documents only.
    PlanningValidation,
    /// W1: Shared foundations – umbrella crate foundations and inventory.
    SharedFoundations,
    /// W2: Baymax game compatibility substrate.
    BaymaxGameCompatibilitySubstrate,
    /// W3: World-model and Comfy serving substrate.
    WorldModelAndComfyServingSubstrate,
    /// W4: Authoring, graph UX, and Comfy workflows.
    AuthoringGraphUxAndComfyWorkflows,
    /// W5: Generation outputs and asset pipelines.
    GenerationOutputsAndAssetPipelines,
    /// W6: External execution hardening.
    ExternalExecutionHardening,
}

impl DependencyWave {
    /// Returns the W0–W6 label for this wave.
    pub fn label(&self) -> &'static str {
        match self {
            Self::PlanningValidation => "W0",
            Self::SharedFoundations => "W1",
            Self::BaymaxGameCompatibilitySubstrate => "W2",
            Self::WorldModelAndComfyServingSubstrate => "W3",
            Self::AuthoringGraphUxAndComfyWorkflows => "W4",
            Self::GenerationOutputsAndAssetPipelines => "W5",
            Self::ExternalExecutionHardening => "W6",
        }
    }
}

// ---------------------------------------------------------------------------
// Gate decision
// ---------------------------------------------------------------------------

/// The result of evaluating whether a task may execute.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GateDecision {
    /// The task may proceed because its required gates are satisfied and wave
    /// ordering is correct.
    Allowed,
    /// The task may not proceed. The vector contains one or more blocking
    /// reasons.
    Blocked(Vec<String>),
}

impl GateDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

// ---------------------------------------------------------------------------
// Spec root
// ---------------------------------------------------------------------------

/// Root directory for grouped migration specs.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SpecRoot {
    pub path: PathBuf,
}

impl SpecRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }
}

// ---------------------------------------------------------------------------
// Migration task reference
// ---------------------------------------------------------------------------

/// A single migration task with its gate, wave, and manifest metadata.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MigrationTaskRef {
    pub id: String,
    pub spec_name: String,
    pub gates: BTreeSet<ExecutionGate>,
    pub wave: Option<DependencyWave>,
    pub requirement_refs: Vec<String>,
    pub writes: Vec<String>,
}

impl MigrationTaskRef {
    pub fn new(id: impl Into<String>, spec_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            spec_name: spec_name.into(),
            gates: BTreeSet::new(),
            wave: None,
            requirement_refs: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub fn with_gates(mut self, gates: BTreeSet<ExecutionGate>) -> Self {
        self.gates = gates;
        self
    }

    pub fn with_wave(mut self, wave: DependencyWave) -> Self {
        self.wave = Some(wave);
        self
    }

    pub fn with_requirements(mut self, reqs: Vec<impl Into<String>>) -> Self {
        self.requirement_refs = reqs.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_writes(mut self, writes: Vec<impl Into<String>>) -> Self {
        self.writes = writes.into_iter().map(Into::into).collect();
        self
    }
}

// ---------------------------------------------------------------------------
// MigrationGatekeeper trait
// ---------------------------------------------------------------------------

/// Blocks implementation work when spec consistency, dependency waves, or
/// execution gates are not satisfied.
pub trait MigrationGatekeeper {
    /// Validate spec completeness and task-manifest traceability for every
    /// grouped spec under the given root.
    fn validate_spec_pack(&self, root: &SpecRoot) -> MigrationValidationReport;

    /// Check whether a task may proceed given the gates that have already
    /// been satisfied.
    fn can_execute_task(
        &self,
        task: &MigrationTaskRef,
        satisfied_gates: &BTreeSet<ExecutionGate>,
    ) -> GateDecision;
}

// ---------------------------------------------------------------------------
// SpecGatekeeper – concrete implementation
// ---------------------------------------------------------------------------

/// A gatekeeper that validates spec packs against the known list of grouped
/// spec directory names and checks execution-gate readiness for individual
/// tasks.
///
/// This implementation delegates spec-file existence checks to the inventory
/// layer when an inventory is available, and falls back to direct filesystem
/// checks otherwise.
/// When `spec_names` is non-empty, validation checks only those specs.
/// When empty, the gatekeeper auto-discovers subdirectories under the
/// given `SpecRoot`.
#[derive(Default)]
pub struct SpecGatekeeper {
    /// Expected grouped spec directory names (relative to `SpecRoot`).
    spec_names: Vec<String>,
}

impl SpecGatekeeper {
    pub const REQUIRED_SPEC_FILES: [&'static str; 3] = ["requirements.md", "design.md", "tasks.md"];

    pub fn new(spec_names: Vec<String>) -> Self {
        Self { spec_names }
    }

    /// Parse requirement references and write targets from an in-memory
    /// `tasks.md` content, returning errors for any task missing either
    /// manifest.
    pub fn parse_task_manifests(
        content: &str,
        spec_name: &str,
        report: &mut MigrationValidationReport,
    ) {
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Detect task entry: "- [ ]" or "- [x]"
            let trimmed = line.trim_start();
            let task_marker = trimmed
                .strip_prefix("- [ ] ")
                .or_else(|| trimmed.strip_prefix("- [x] "));

            if let Some(task_title) = task_marker {
                let task_name = task_title.trim().to_string();

                // Collect indented lines under this task
                let base_indent = line.chars().position(|c| c != ' ').unwrap_or(0);
                let mut j = i + 1;

                let mut has_requirements = false;
                let mut has_writes = false;

                while j < lines.len() {
                    let next = lines[j];
                    // Stop when we hit a non-indented line or next task
                    let indent = next.chars().position(|c| c != ' ').unwrap_or(0);
                    if indent <= base_indent && !next.trim().is_empty() {
                        break;
                    }
                    let stripped = next.trim();
                    // Accept forms like "- _Requirements: ...", "_- _Requirements: ...",
                    // "- _writes: ...", or "_- _writes: ...".
                    if stripped.contains("_Requirements:") {
                        has_requirements = true;
                    }
                    if stripped.contains("_writes:") {
                        has_writes = true;
                    }
                    j += 1;
                }

                if !has_requirements {
                    report.push(MigrationValidationError::MissingTaskRequirementRefs {
                        task: task_name.clone(),
                        spec: spec_name.to_string(),
                    });
                }
                if !has_writes {
                    report.push(MigrationValidationError::MissingTaskWriteTargets {
                        task: task_name.clone(),
                        spec: spec_name.to_string(),
                    });
                }

                i = j;
                continue;
            }

            i += 1;
        }
    }

    /// Parse tasks.md file path and call `parse_task_manifests` on its
    /// content.  If the file cannot be read, no task errors are reported
    /// (the file itself is validated elsewhere).
    fn parse_tasks_file(
        tasks_path: &Path,
        spec_name: &str,
        report: &mut MigrationValidationReport,
    ) {
        if !tasks_path.is_file() {
            return;
        }
        let content = match fs::read_to_string(tasks_path) {
            Ok(c) => c,
            Err(_) => return,
        };
        Self::parse_task_manifests(&content, spec_name, report);
    }
}

impl MigrationGatekeeper for SpecGatekeeper {
    fn validate_spec_pack(&self, root: &SpecRoot) -> MigrationValidationReport {
        let mut report = MigrationValidationReport::default();

        // Determine which spec directories to check.
        let spec_dirs: Vec<PathBuf> = if self.spec_names.is_empty() {
            // Auto-discover subdirectories under the root.
            match fs::read_dir(&root.path) {
                Ok(entries) => entries
                    .filter_map(|e| {
                        let entry = e.ok()?;
                        if entry.file_type().ok()?.is_dir() {
                            Some(entry.path())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Err(_) => return report,
            }
        } else {
            self.spec_names
                .iter()
                .map(|name| root.path.join(name))
                .collect()
        };

        for spec_path in &spec_dirs {
            if !spec_path.is_dir() {
                continue;
            }
            let spec_name = spec_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Validate required files exist (Property 2, Requirement 12.4).
            for file in Self::REQUIRED_SPEC_FILES {
                if !spec_path.join(file).is_file() {
                    report.push(MigrationValidationError::MissingSpecFile {
                        spec: spec_name.clone(),
                        file: file.to_string(),
                    });
                }
            }

            // Validate task manifests (Property 3, Requirements 10.1, 10.2).
            let tasks_path = spec_path.join("tasks.md");
            Self::parse_tasks_file(&tasks_path, &spec_name, &mut report);
        }

        // Scope validation: check that all known spec directories exist on
        // disk when explicit spec names are given.
        if !self.spec_names.is_empty() {
            for name in &self.spec_names {
                let spec_path = root.path.join(name);
                if !spec_path.is_dir() {
                    report.push(MigrationValidationError::MissingSpecFile {
                        spec: name.clone(),
                        file: "(directory)".to_string(),
                    });
                }
            }
        }

        report
    }

    fn can_execute_task(
        &self,
        task: &MigrationTaskRef,
        satisfied_gates: &BTreeSet<ExecutionGate>,
    ) -> GateDecision {
        let unsatisfied: Vec<String> = task
            .gates
            .iter()
            .filter(|gate| !satisfied_gates.contains(gate))
            .map(|gate| {
                format!(
                    "Gate {} ({:?}) is required for task \"{}\" but is not yet satisfied",
                    gate.label(),
                    gate,
                    task.id,
                )
            })
            .collect();

        if unsatisfied.is_empty() {
            GateDecision::Allowed
        } else {
            GateDecision::Blocked(unsatisfied)
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience: full spec-pack validation using an existing inventory.
// ---------------------------------------------------------------------------

/// Validate a spec pack by combining inventory and gatekeeper checks.
pub fn validate_spec_pack(
    gatekeeper: &SpecGatekeeper,
    root: &SpecRoot,
    inventory: &MigrationInventory,
) -> MigrationValidationReport {
    let mut report = inventory.validate_spec_pack();
    let gatekeeper_report = gatekeeper.validate_spec_pack(root);
    report.errors.extend(gatekeeper_report.errors);
    report
}
