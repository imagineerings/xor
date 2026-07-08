use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const DEPENDENCY_REVIEW_MISSING_CODE: &str = "world_model.dependency_review.missing";
pub const DEPENDENCY_REVIEW_INCOMPLETE_CODE: &str = "world_model.dependency_review.incomplete";
pub const DEPENDENCY_REVIEW_NOT_APPROVED_CODE: &str = "world_model.dependency_review.not_approved";
pub const DEPENDENCY_REVIEW_AUDIT_MISSING_CODE: &str =
    "world_model.dependency_review.audit_missing";

pub const DEFAULT_LARGE_DOWNLOAD_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimDependencyKind {
    NativeLibrary,
    Codec,
    PythonPackage,
    ProviderSdk,
    ModelDependency,
    FrontendPackage,
    VendoredCode,
    NetworkAccess,
    LargeDownload,
}

impl SimDependencyKind {
    pub fn requires_review(self) -> bool {
        matches!(
            self,
            Self::NativeLibrary
                | Self::Codec
                | Self::PythonPackage
                | Self::ProviderSdk
                | Self::ModelDependency
                | Self::FrontendPackage
                | Self::VendoredCode
                | Self::NetworkAccess
                | Self::LargeDownload
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyProposal {
    pub id: String,
    pub name: String,
    pub kind: SimDependencyKind,
    pub requires_network_access: bool,
    pub estimated_download_bytes: Option<u64>,
}

impl SimDependencyProposal {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: SimDependencyKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            requires_network_access: false,
            estimated_download_bytes: None,
        }
    }

    pub fn with_network_access(mut self, requires_network_access: bool) -> Self {
        self.requires_network_access = requires_network_access;
        self
    }

    pub fn with_estimated_download_bytes(mut self, estimated_download_bytes: u64) -> Self {
        self.estimated_download_bytes = Some(estimated_download_bytes);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimDependencyReviewStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyReviewRecord {
    pub dependency_id: String,
    pub status: SimDependencyReviewStatus,
    pub license: String,
    pub maintenance: String,
    pub security: String,
    pub binary_size: String,
    pub platform_impact: String,
    pub runtime_impact: String,
    pub fallback_strategy: String,
    pub reviewed_by: String,
    pub reviewed_at: String,
}

impl SimDependencyReviewRecord {
    pub fn approved(
        dependency_id: impl Into<String>,
        reviewed_by: impl Into<String>,
        reviewed_at: impl Into<String>,
    ) -> Self {
        Self {
            dependency_id: dependency_id.into(),
            status: SimDependencyReviewStatus::Approved,
            license: String::new(),
            maintenance: String::new(),
            security: String::new(),
            binary_size: String::new(),
            platform_impact: String::new(),
            runtime_impact: String::new(),
            fallback_strategy: String::new(),
            reviewed_by: reviewed_by.into(),
            reviewed_at: reviewed_at.into(),
        }
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = license.into();
        self
    }

    pub fn with_maintenance(mut self, maintenance: impl Into<String>) -> Self {
        self.maintenance = maintenance.into();
        self
    }

    pub fn with_security(mut self, security: impl Into<String>) -> Self {
        self.security = security.into();
        self
    }

    pub fn with_binary_size(mut self, binary_size: impl Into<String>) -> Self {
        self.binary_size = binary_size.into();
        self
    }

    pub fn with_platform_impact(mut self, platform_impact: impl Into<String>) -> Self {
        self.platform_impact = platform_impact.into();
        self
    }

    pub fn with_runtime_impact(mut self, runtime_impact: impl Into<String>) -> Self {
        self.runtime_impact = runtime_impact.into();
        self
    }

    pub fn with_fallback_strategy(mut self, fallback_strategy: impl Into<String>) -> Self {
        self.fallback_strategy = fallback_strategy.into();
        self
    }

    pub fn with_status(mut self, status: SimDependencyReviewStatus) -> Self {
        self.status = status;
        self
    }

    fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        for (name, value) in [
            ("license", self.license.as_str()),
            ("maintenance", self.maintenance.as_str()),
            ("security", self.security.as_str()),
            ("binary_size", self.binary_size.as_str()),
            ("platform_impact", self.platform_impact.as_str()),
            ("runtime_impact", self.runtime_impact.as_str()),
            ("fallback_strategy", self.fallback_strategy.as_str()),
            ("reviewed_by", self.reviewed_by.as_str()),
            ("reviewed_at", self.reviewed_at.as_str()),
        ] {
            if value.trim().is_empty() {
                missing.push(name);
            }
        }
        missing
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimDependencyAuditKind {
    NetworkAccess,
    LargeDownload,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyAuditRecord {
    pub dependency_id: String,
    pub kind: SimDependencyAuditKind,
    pub approved_by: String,
    pub approved_at: String,
    pub reason: String,
}

impl SimDependencyAuditRecord {
    pub fn new(
        dependency_id: impl Into<String>,
        kind: SimDependencyAuditKind,
        approved_by: impl Into<String>,
        approved_at: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            dependency_id: dependency_id.into(),
            kind,
            approved_by: approved_by.into(),
            approved_at: approved_at.into(),
            reason: reason.into(),
        }
    }

    fn is_complete(&self) -> bool {
        !self.approved_by.trim().is_empty()
            && !self.approved_at.trim().is_empty()
            && !self.reason.trim().is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyReviewGate {
    reviews: BTreeMap<String, SimDependencyReviewRecord>,
    audit_records: Vec<SimDependencyAuditRecord>,
    large_download_threshold_bytes: u64,
}

impl SimDependencyReviewGate {
    pub fn new() -> Self {
        Self {
            reviews: BTreeMap::new(),
            audit_records: Vec::new(),
            large_download_threshold_bytes: DEFAULT_LARGE_DOWNLOAD_THRESHOLD_BYTES,
        }
    }

    pub fn with_large_download_threshold_bytes(mut self, threshold_bytes: u64) -> Self {
        self.large_download_threshold_bytes = threshold_bytes;
        self
    }

    pub fn with_review(mut self, record: SimDependencyReviewRecord) -> Self {
        self.reviews.insert(record.dependency_id.clone(), record);
        self
    }

    pub fn with_audit_record(mut self, record: SimDependencyAuditRecord) -> Self {
        self.audit_records.push(record);
        self
    }

    pub fn evaluate(
        &self,
        proposals: impl IntoIterator<Item = SimDependencyProposal>,
    ) -> SimDependencyReviewReport {
        let decisions = proposals
            .into_iter()
            .map(|proposal| self.evaluate_proposal(proposal))
            .collect::<Vec<_>>();
        SimDependencyReviewReport { decisions }
    }

    fn evaluate_proposal(&self, proposal: SimDependencyProposal) -> SimDependencyReviewDecision {
        let mut diagnostics = Vec::new();

        if proposal.kind.requires_review() {
            self.validate_review(&proposal, &mut diagnostics);
        }

        for audit_kind in required_audits(&proposal, self.large_download_threshold_bytes) {
            self.validate_audit(&proposal, audit_kind, &mut diagnostics);
        }

        let allowed = diagnostics.is_empty();
        SimDependencyReviewDecision {
            proposal,
            allowed,
            diagnostics,
        }
    }

    fn validate_review(
        &self,
        proposal: &SimDependencyProposal,
        diagnostics: &mut Vec<SimDependencyReviewDiagnostic>,
    ) {
        let Some(review) = self.reviews.get(&proposal.id) else {
            diagnostics.push(SimDependencyReviewDiagnostic::error(
                DEPENDENCY_REVIEW_MISSING_CODE,
                proposal.id.clone(),
                "dependency review metadata is required before implementation",
            ));
            return;
        };

        let missing_fields = review.missing_fields();
        if !missing_fields.is_empty() {
            diagnostics.push(SimDependencyReviewDiagnostic::error(
                DEPENDENCY_REVIEW_INCOMPLETE_CODE,
                proposal.id.clone(),
                format!(
                    "dependency review metadata is missing {}",
                    missing_fields.join(", ")
                ),
            ));
        }

        if review.status != SimDependencyReviewStatus::Approved {
            diagnostics.push(SimDependencyReviewDiagnostic::error(
                DEPENDENCY_REVIEW_NOT_APPROVED_CODE,
                proposal.id.clone(),
                "dependency review must be approved before implementation",
            ));
        }
    }

    fn validate_audit(
        &self,
        proposal: &SimDependencyProposal,
        audit_kind: SimDependencyAuditKind,
        diagnostics: &mut Vec<SimDependencyReviewDiagnostic>,
    ) {
        let has_audit = self.audit_records.iter().any(|record| {
            record.dependency_id == proposal.id && record.kind == audit_kind && record.is_complete()
        });

        if !has_audit {
            diagnostics.push(SimDependencyReviewDiagnostic::error(
                DEPENDENCY_REVIEW_AUDIT_MISSING_CODE,
                proposal.id.clone(),
                format!("{audit_kind:?} requires explicit user approval and an audit record"),
            ));
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyReviewReport {
    pub decisions: Vec<SimDependencyReviewDecision>,
}

impl SimDependencyReviewReport {
    pub fn is_allowed(&self) -> bool {
        self.decisions.iter().all(|decision| decision.allowed)
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &SimDependencyReviewDiagnostic> {
        self.decisions
            .iter()
            .flat_map(|decision| decision.diagnostics.iter())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyReviewDecision {
    pub proposal: SimDependencyProposal,
    pub allowed: bool,
    pub diagnostics: Vec<SimDependencyReviewDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimDependencyReviewDiagnosticSeverity {
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDependencyReviewDiagnostic {
    pub code: String,
    pub severity: SimDependencyReviewDiagnosticSeverity,
    pub dependency_id: String,
    pub message: String,
}

impl SimDependencyReviewDiagnostic {
    fn error(
        code: impl Into<String>,
        dependency_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: SimDependencyReviewDiagnosticSeverity::Error,
            dependency_id: dependency_id.into(),
            message: message.into(),
        }
    }
}

fn required_audits(
    proposal: &SimDependencyProposal,
    large_download_threshold_bytes: u64,
) -> BTreeSet<SimDependencyAuditKind> {
    let mut audits = BTreeSet::new();
    if proposal.requires_network_access || proposal.kind == SimDependencyKind::NetworkAccess {
        audits.insert(SimDependencyAuditKind::NetworkAccess);
    }

    if proposal.kind == SimDependencyKind::LargeDownload
        || proposal
            .estimated_download_bytes
            .is_some_and(|bytes| bytes >= large_download_threshold_bytes)
    {
        audits.insert(SimDependencyAuditKind::LargeDownload);
    }

    audits
}
