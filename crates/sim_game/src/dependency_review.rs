use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimGameDependencyKind {
    NativeLibrary,
    Codec,
    Model,
    Mesh,
    MediaRuntime,
    VendoredCode,
    FrontendPackage,
}

impl SimGameDependencyKind {
    pub fn requires_review(self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDependencyProposal {
    pub id: String,
    pub name: String,
    pub kind: SimGameDependencyKind,
}

impl SimGameDependencyProposal {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: SimGameDependencyKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameDependencyReviewStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDependencyReviewRecord {
    pub dependency_id: String,
    pub status: SimGameDependencyReviewStatus,
    pub license: String,
    pub maintenance: String,
    pub security: String,
    pub binary_size: String,
    pub platform_impact: String,
    pub reviewed_by: String,
    pub reviewed_at: String,
}

impl SimGameDependencyReviewRecord {
    pub fn approved(
        dependency_id: impl Into<String>,
        reviewed_by: impl Into<String>,
        reviewed_at: impl Into<String>,
    ) -> Self {
        Self {
            dependency_id: dependency_id.into(),
            status: SimGameDependencyReviewStatus::Approved,
            license: String::new(),
            maintenance: String::new(),
            security: String::new(),
            binary_size: String::new(),
            platform_impact: String::new(),
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

    pub fn with_status(mut self, status: SimGameDependencyReviewStatus) -> Self {
        self.status = status;
        self
    }

    fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        for (field, value) in [
            ("license", self.license.as_str()),
            ("maintenance", self.maintenance.as_str()),
            ("security", self.security.as_str()),
            ("binary_size", self.binary_size.as_str()),
            ("platform_impact", self.platform_impact.as_str()),
            ("reviewed_by", self.reviewed_by.as_str()),
            ("reviewed_at", self.reviewed_at.as_str()),
        ] {
            if value.trim().is_empty() {
                missing.push(field);
            }
        }
        missing
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyReviewGate {
    reviews: BTreeMap<String, SimGameDependencyReviewRecord>,
}

impl DependencyReviewGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_review(mut self, review: SimGameDependencyReviewRecord) -> Self {
        self.reviews.insert(review.dependency_id.clone(), review);
        self
    }

    pub fn evaluate(
        &self,
        proposals: impl IntoIterator<Item = SimGameDependencyProposal>,
    ) -> SimGameDependencyReviewReport {
        let decisions = proposals
            .into_iter()
            .map(|proposal| self.evaluate_proposal(proposal))
            .collect();
        SimGameDependencyReviewReport { decisions }
    }

    fn evaluate_proposal(
        &self,
        proposal: SimGameDependencyProposal,
    ) -> SimGameDependencyReviewDecision {
        let mut diagnostics = Vec::new();

        if proposal.kind.requires_review() {
            let Some(review) = self.reviews.get(&proposal.id) else {
                diagnostics.push(SimGameDependencyReviewDiagnostic {
                    dependency_id: proposal.id.clone(),
                    field: "review",
                    message: "Sim dependency review is required before implementation".to_string(),
                });
                return SimGameDependencyReviewDecision {
                    proposal,
                    allowed: false,
                    diagnostics,
                };
            };

            let missing = review.missing_fields();
            if !missing.is_empty() {
                diagnostics.push(SimGameDependencyReviewDiagnostic {
                    dependency_id: proposal.id.clone(),
                    field: "review.metadata",
                    message: format!(
                        "Sim dependency review metadata is missing {}",
                        missing.join(", ")
                    ),
                });
            }

            if review.status != SimGameDependencyReviewStatus::Approved {
                diagnostics.push(SimGameDependencyReviewDiagnostic {
                    dependency_id: proposal.id.clone(),
                    field: "review.status",
                    message: "Sim dependency review must be approved before implementation"
                        .to_string(),
                });
            }
        }

        SimGameDependencyReviewDecision {
            proposal,
            allowed: diagnostics.is_empty(),
            diagnostics,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimGameDependencyReviewReport {
    pub decisions: Vec<SimGameDependencyReviewDecision>,
}

impl SimGameDependencyReviewReport {
    pub fn is_allowed(&self) -> bool {
        self.decisions.iter().all(|decision| decision.allowed)
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &SimGameDependencyReviewDiagnostic> {
        self.decisions
            .iter()
            .flat_map(|decision| decision.diagnostics.iter())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimGameDependencyReviewDecision {
    pub proposal: SimGameDependencyProposal,
    pub allowed: bool,
    pub diagnostics: Vec<SimGameDependencyReviewDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimGameDependencyReviewDiagnostic {
    pub dependency_id: String,
    pub field: &'static str,
    pub message: String,
}
