use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use world_model::{SimSourceItem, SimSourceKind};

pub const SIM_COVERAGE_MISSING_OWNER_CODE: &str = "sim_coverage.missing_owner";
pub const SIM_COVERAGE_DUPLICATE_OWNER_CODE: &str = "sim_coverage.duplicate_owner";
pub const SIM_COVERAGE_IMPLEMENTED_WITHOUT_EVIDENCE_CODE: &str =
    "sim_coverage.implemented_without_evidence";
pub const SIM_COVERAGE_UNSUPPORTED_WITHOUT_REASON_CODE: &str =
    "sim_coverage.unsupported_without_reason";
pub const SIM_COVERAGE_INVALID_OWNER_PATH_CODE: &str = "sim_coverage.invalid_owner_path";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageLedger {
    pub schema_version: u32,
    pub records: Vec<SimCoverageRecord>,
}

impl SimCoverageLedger {
    pub fn new(schema_version: u32, records: impl IntoIterator<Item = SimCoverageRecord>) -> Self {
        Self {
            schema_version,
            records: records.into_iter().collect(),
        }
    }

    pub fn owner_for(&self, source_id: &str) -> Option<&SimCoverageOwner> {
        self.records
            .iter()
            .find(|record| record.source_id == source_id)
            .and_then(|record| record.owner.as_ref())
    }

    pub fn records_by_owner(&self, owner: SimCoverageOwner) -> Vec<&SimCoverageRecord> {
        self.records
            .iter()
            .filter(|record| record.owner == Some(owner))
            .collect()
    }

    pub fn uncovered_records_by_product_sequence(&self) -> Vec<&SimCoverageRecord> {
        let mut records = self
            .records
            .iter()
            .filter(|record| record.status.is_uncovered())
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            let left_rank = left
                .owner
                .map(SimCoverageOwner::product_sequence_rank)
                .unwrap_or(u8::MAX);
            let right_rank = right
                .owner
                .map(SimCoverageOwner::product_sequence_rank)
                .unwrap_or(u8::MAX);
            left_rank
                .cmp(&right_rank)
                .then_with(|| left.source_id.cmp(&right.source_id))
        });
        records
    }

    pub fn validate(&self) -> Vec<SimCoverageDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen_source_ids: BTreeMap<&str, &SimCoverageRecord> = BTreeMap::new();

        for record in &self.records {
            if record.owner.is_none() {
                diagnostics.push(SimCoverageDiagnostic::for_record(
                    SIM_COVERAGE_MISSING_OWNER_CODE,
                    record,
                    "Coverage records must name one Sim owner",
                    SimCoverageDiagnosticSeverity::Error,
                ));
            }

            if let Some(previous) = seen_source_ids.insert(record.source_id.as_str(), record) {
                let mut owner_paths = BTreeSet::new();
                owner_paths.insert(previous.owner_path.as_str());
                owner_paths.insert(record.owner_path.as_str());
                diagnostics.push(SimCoverageDiagnostic::for_record(
                    SIM_COVERAGE_DUPLICATE_OWNER_CODE,
                    record,
                    format!(
                        "Source item is claimed by multiple coverage records: {}",
                        owner_paths.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                    SimCoverageDiagnosticSeverity::Error,
                ));
            }

            if record.status == SimCoverageStatus::Implemented && record.evidence.is_empty() {
                diagnostics.push(SimCoverageDiagnostic::for_record(
                    SIM_COVERAGE_IMPLEMENTED_WITHOUT_EVIDENCE_CODE,
                    record,
                    "Implemented coverage must reference native Sim evidence",
                    SimCoverageDiagnosticSeverity::Error,
                ));
            }

            if record.status.requires_boundary_reason()
                && record
                    .boundary_decision
                    .as_ref()
                    .is_none_or(SimCoverageBoundaryDecision::is_empty)
            {
                diagnostics.push(SimCoverageDiagnostic::for_record(
                    SIM_COVERAGE_UNSUPPORTED_WITHOUT_REASON_CODE,
                    record,
                    "Unsupported, divergent, and delegated coverage must include a user-visible reason",
                    SimCoverageDiagnosticSeverity::Error,
                ));
            }

            if let Some(owner) = record.owner
                && !owner.accepts_owner_path(&record.owner_path)
            {
                diagnostics.push(SimCoverageDiagnostic::for_record(
                    SIM_COVERAGE_INVALID_OWNER_PATH_CODE,
                    record,
                    format!(
                        "Owner path `{}` does not match expected owner `{}`",
                        record.owner_path,
                        owner.expected_owner_path()
                    ),
                    SimCoverageDiagnosticSeverity::Error,
                ));
            }
        }

        diagnostics
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageOwnerResolver;

impl SimCoverageOwnerResolver {
    pub fn suggest_owner(source_item: &SimSourceItem) -> SimCoverageOwnerSuggestion {
        let owner = match source_item.source_kind {
            SimSourceKind::Route | SimSourceKind::WebSocketProtocol => {
                SimCoverageOwner::RuntimeControlPlane
            }
            SimSourceKind::CoreNode => {
                if source_item_matches(source_item, DIFFUSION_OWNER_KEYWORDS) {
                    SimCoverageOwner::DiffusionWorldModelRuntime
                } else {
                    SimCoverageOwner::GraphNodeRuntime
                }
            }
            SimSourceKind::ModelFamily | SimSourceKind::ModelFolder => {
                SimCoverageOwner::ModelMemoryRuntime
            }
            SimSourceKind::Blueprint => SimCoverageOwner::WorkflowsBlueprints,
            SimSourceKind::AssetApi => SimCoverageOwner::AssetLibrary,
            SimSourceKind::ExtraNode => suggest_extra_node_owner(source_item),
            SimSourceKind::ApiProviderNode => SimCoverageOwner::ApiProviderNodes,
            SimSourceKind::ExtensionHook => SimCoverageOwner::ExtensionEcosystem,
            SimSourceKind::CliFlag
            | SimSourceKind::OpenApiOperation
            | SimSourceKind::TestSurface
            | SimSourceKind::PackagingSurface
            | SimSourceKind::FrontendSurface
            | SimSourceKind::Unknown => SimCoverageOwner::PackagingQuality,
        };

        SimCoverageOwnerSuggestion {
            owner,
            owner_path: owner.expected_owner_path().to_string(),
            reason: Self::explain(source_item, owner),
        }
    }

    pub fn explain(source_item: &SimSourceItem, owner: SimCoverageOwner) -> String {
        format!(
            "{:?} `{}` from `{}` maps to {:?}",
            source_item.source_kind, source_item.symbol, source_item.source_path, owner
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageOwnerSuggestion {
    pub owner: SimCoverageOwner,
    pub owner_path: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageRecord {
    pub source_id: String,
    pub source_path: String,
    pub source_kind: SimSourceKind,
    pub owner: Option<SimCoverageOwner>,
    pub owner_path: String,
    pub status: SimCoverageStatus,
    pub boundary_decision: Option<SimCoverageBoundaryDecision>,
    pub evidence: Vec<SimCoverageEvidence>,
    pub dependency_gate: Option<SimCoverageDependencyGate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backlog_task: Option<SimCoverageBacklogRef>,
}

impl SimCoverageRecord {
    pub fn new(
        source_id: impl Into<String>,
        source_path: impl Into<String>,
        source_kind: SimSourceKind,
        status: SimCoverageStatus,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            source_path: source_path.into(),
            source_kind,
            owner: None,
            owner_path: String::new(),
            status,
            boundary_decision: None,
            evidence: Vec::new(),
            dependency_gate: None,
            backlog_task: None,
        }
    }

    pub fn with_owner(mut self, owner: SimCoverageOwner) -> Self {
        self.owner_path = owner.expected_owner_path().to_string();
        self.owner = Some(owner);
        self
    }

    pub fn with_existing_sim_owner_path(mut self, owner_path: impl Into<String>) -> Self {
        self.owner = Some(SimCoverageOwner::ExistingSimSubsystem);
        self.owner_path = owner_path.into();
        self
    }

    pub fn with_boundary_decision(
        mut self,
        boundary_decision: SimCoverageBoundaryDecision,
    ) -> Self {
        self.boundary_decision = Some(boundary_decision);
        self
    }

    pub fn with_evidence(mut self, evidence: SimCoverageEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn with_dependency_gate(mut self, dependency_gate: SimCoverageDependencyGate) -> Self {
        self.dependency_gate = Some(dependency_gate);
        self
    }

    pub fn with_backlog_task(mut self, backlog_task: SimCoverageBacklogRef) -> Self {
        self.backlog_task = Some(backlog_task);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimCoverageOwner {
    ExistingSimSubsystem,
    RuntimeControlPlane,
    GraphNodeRuntime,
    ModelMemoryRuntime,
    DiffusionWorldModelRuntime,
    AssetLibrary,
    WorkflowsBlueprints,
    MediaNodePipelines,
    ApiProviderNodes,
    ExtensionEcosystem,
    PackagingQuality,
}

impl SimCoverageOwner {
    pub fn expected_owner_path(self) -> &'static str {
        match self {
            Self::ExistingSimSubsystem => "<existing-sim-subsystem>",
            Self::RuntimeControlPlane => {
                ".agents/specs/godot-migration/comfy-runtime-control-plane"
            }
            Self::GraphNodeRuntime => ".agents/specs/godot-migration/comfy-graph-node-runtime",
            Self::ModelMemoryRuntime => ".agents/specs/godot-migration/comfy-model-memory-runtime",
            Self::DiffusionWorldModelRuntime => {
                ".agents/specs/godot-migration/comfy-diffusion-world-model-runtime"
            }
            Self::AssetLibrary => ".agents/specs/godot-migration/comfy-asset-library",
            Self::WorkflowsBlueprints => ".agents/specs/godot-migration/comfy-workflows-blueprints",
            Self::MediaNodePipelines => ".agents/specs/godot-migration/comfy-media-node-pipelines",
            Self::ApiProviderNodes => ".agents/specs/godot-migration/comfy-api-provider-nodes",
            Self::ExtensionEcosystem => ".agents/specs/godot-migration/comfy-extension-ecosystem",
            Self::PackagingQuality => ".agents/specs/godot-migration/comfy-packaging-quality",
        }
    }

    pub fn product_sequence_rank(self) -> u8 {
        match self {
            Self::ExistingSimSubsystem => 0,
            Self::RuntimeControlPlane => 10,
            Self::GraphNodeRuntime => 20,
            Self::ModelMemoryRuntime => 30,
            Self::DiffusionWorldModelRuntime => 40,
            Self::AssetLibrary => 50,
            Self::WorkflowsBlueprints => 60,
            Self::MediaNodePipelines => 70,
            Self::ApiProviderNodes => 80,
            Self::ExtensionEcosystem => 90,
            Self::PackagingQuality => 100,
        }
    }

    pub fn is_policy_gated_tail_work(self) -> bool {
        matches!(
            self,
            Self::ApiProviderNodes | Self::ExtensionEcosystem | Self::PackagingQuality
        )
    }

    fn accepts_owner_path(self, owner_path: &str) -> bool {
        let owner_path = owner_path.trim();
        if owner_path.is_empty() {
            return false;
        }

        match self {
            Self::ExistingSimSubsystem => {
                !owner_path.starts_with(".agents/specs/godot-migration/comfy-")
            }
            _ => owner_path == self.expected_owner_path(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimCoverageStatus {
    Implemented,
    Planned,
    Delegated,
    Unsupported,
    Divergent,
}

impl SimCoverageStatus {
    fn requires_boundary_reason(self) -> bool {
        matches!(self, Self::Delegated | Self::Unsupported | Self::Divergent)
    }

    fn is_uncovered(self) -> bool {
        matches!(self, Self::Planned | Self::Unsupported | Self::Divergent)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageBoundaryDecision {
    pub user_visible_reason: String,
    pub technical_reason: Option<String>,
}

impl SimCoverageBoundaryDecision {
    pub fn new(user_visible_reason: impl Into<String>) -> Self {
        Self {
            user_visible_reason: user_visible_reason.into(),
            technical_reason: None,
        }
    }

    pub fn with_technical_reason(mut self, technical_reason: impl Into<String>) -> Self {
        self.technical_reason = Some(technical_reason.into());
        self
    }

    fn is_empty(&self) -> bool {
        self.user_visible_reason.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageEvidence {
    pub kind: SimCoverageEvidenceKind,
    pub reference: String,
}

impl SimCoverageEvidence {
    pub fn new(kind: SimCoverageEvidenceKind, reference: impl Into<String>) -> Self {
        Self {
            kind,
            reference: reference.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimCoverageEvidenceKind {
    Test,
    Fixture,
    Module,
    ExistingSimEquivalence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageDependencyGate {
    pub gate_id: String,
    pub review_path: String,
}

impl SimCoverageDependencyGate {
    pub fn new(gate_id: impl Into<String>, review_path: impl Into<String>) -> Self {
        Self {
            gate_id: gate_id.into(),
            review_path: review_path.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageBacklogRef {
    pub task_id: String,
    pub owner_spec: String,
    pub capability_family: String,
    pub expected_writes: Vec<String>,
    pub validation: String,
    pub evidence_policy: String,
}

impl SimCoverageBacklogRef {
    pub fn new(
        task_id: impl Into<String>,
        owner_spec: impl Into<String>,
        capability_family: impl Into<String>,
        expected_writes: Vec<impl Into<String>>,
        validation: impl Into<String>,
        evidence_policy: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            owner_spec: owner_spec.into(),
            capability_family: capability_family.into(),
            expected_writes: expected_writes.into_iter().map(Into::into).collect(),
            validation: validation.into(),
            evidence_policy: evidence_policy.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCoverageDiagnostic {
    pub code: String,
    pub source_id: Option<String>,
    pub owner: Option<SimCoverageOwner>,
    pub message: String,
    pub severity: SimCoverageDiagnosticSeverity,
}

impl SimCoverageDiagnostic {
    fn for_record(
        code: impl Into<String>,
        record: &SimCoverageRecord,
        message: impl Into<String>,
        severity: SimCoverageDiagnosticSeverity,
    ) -> Self {
        Self {
            code: code.into(),
            source_id: Some(record.source_id.clone()),
            owner: record.owner,
            message: message.into(),
            severity,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimCoverageDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

const DIFFUSION_OWNER_KEYWORDS: &[&str] = &[
    "sampler",
    "scheduler",
    "guider",
    "conditioning",
    "latent",
    "vae",
    "lora",
    "hypernetwork",
    "diffusion",
    "checkpoint",
    "modelmerge",
    "model_merging",
    "modelpatch",
    "model_patches",
    "model_sampling",
];

const ASSET_OWNER_KEYWORDS: &[&str] = &[
    "asset",
    "upload",
    "download",
    "tag",
    "metadata",
    "userdata",
    "user_data",
    "scan",
    "prune",
    "output",
    "enrich",
];

const WORKFLOW_OWNER_KEYWORDS: &[&str] = &[
    "blueprint",
    "workflow",
    "template",
    "subgraph",
    "replacement",
    "app_mode",
    "embedded",
];

const EXTENSION_OWNER_KEYWORDS: &[&str] = &[
    "custom_node",
    "custom_nodes",
    "extension",
    "translation",
    "i18n",
    "manager",
    "startup",
];

const PACKAGING_OWNER_KEYWORDS: &[&str] = &[
    "cli",
    "openapi",
    "example",
    "test",
    "dependency",
    "logging",
    "ci",
    "package",
    "frontend",
];

fn suggest_extra_node_owner(source_item: &SimSourceItem) -> SimCoverageOwner {
    for (keywords, owner) in [
        (
            DIFFUSION_OWNER_KEYWORDS,
            SimCoverageOwner::DiffusionWorldModelRuntime,
        ),
        (ASSET_OWNER_KEYWORDS, SimCoverageOwner::AssetLibrary),
        (
            WORKFLOW_OWNER_KEYWORDS,
            SimCoverageOwner::WorkflowsBlueprints,
        ),
        (
            EXTENSION_OWNER_KEYWORDS,
            SimCoverageOwner::ExtensionEcosystem,
        ),
        (PACKAGING_OWNER_KEYWORDS, SimCoverageOwner::PackagingQuality),
    ] {
        if source_item_matches(source_item, keywords) {
            return owner;
        }
    }

    SimCoverageOwner::MediaNodePipelines
}

fn source_item_matches(source_item: &SimSourceItem, keywords: &[&str]) -> bool {
    let haystack = source_item_haystack(source_item);
    keywords
        .iter()
        .any(|keyword| haystack.contains(&keyword.to_ascii_lowercase()))
}

fn source_item_haystack(source_item: &SimSourceItem) -> String {
    let category = source_item.category.as_deref().unwrap_or_default();
    format!(
        "{}\n{}\n{}\n{}",
        source_item.source_path, source_item.symbol, category, source_item.metadata
    )
    .to_ascii_lowercase()
}
