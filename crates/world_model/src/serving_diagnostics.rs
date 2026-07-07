use serde::{Deserialize, Serialize};

use crate::serving::ModelServingTarget;

// ---------------------------------------------------------------------------
// Diagnostic severity
// ---------------------------------------------------------------------------

/// How severe a serving diagnostic is.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Blocking — serving cannot proceed.
    Error,
    /// Non-blocking but notable — serving may degrade.
    Warning,
    /// Informational — no action required.
    Info,
}

// ---------------------------------------------------------------------------
// Diagnostic category
// ---------------------------------------------------------------------------

/// The category of a serving diagnostic, corresponding to requirement areas.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    /// Python environment issues (Req 9.1).
    Environment,
    /// Missing or incompatible packages.
    Package,
    /// Missing or wrong checkpoint.
    Checkpoint,
    /// GPU/VRAM issues.
    Gpu,
    /// Disk space issues.
    Disk,
    /// Remote endpoint unreachable or misconfigured (Req 9.2).
    Endpoint,
    /// Missing or expired authentication.
    Authentication,
    /// Backend capability does not match requirement.
    Capability,
    /// Quota exceeded or unknown.
    Quota,
    /// Download-related warnings (Req 9.3).
    Download,
    /// Catch-all for other diagnostics.
    Other,
}

impl DiagnosticCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::Package => "package",
            Self::Checkpoint => "checkpoint",
            Self::Gpu => "gpu",
            Self::Disk => "disk",
            Self::Endpoint => "endpoint",
            Self::Authentication => "authentication",
            Self::Capability => "capability",
            Self::Quota => "quota",
            Self::Download => "download",
            Self::Other => "other",
        }
    }
}

// ---------------------------------------------------------------------------
// Serving diagnostic
// ---------------------------------------------------------------------------

/// A single diagnostic item about a serving target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServingDiagnostic {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub detail: Option<String>,
}

impl ServingDiagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        category: DiagnosticCategory,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            message: message.into(),
            detail: None,
        }
    }

    pub fn error(category: DiagnosticCategory, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Error, category, message)
    }

    pub fn warning(category: DiagnosticCategory, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Warning, category, message)
    }

    pub fn info(category: DiagnosticCategory, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Info, category, message)
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Diagnostic report
// ---------------------------------------------------------------------------

/// The result of validating a model serving target.
///
/// `is_ready` is `true` only when there are no `Error`-severity diagnostics.
/// Warnings and info items can exist while the target is ready.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ServingDiagnosticReport {
    pub diagnostics: Vec<ServingDiagnostic>,
    pub is_ready: bool,
}

impl ServingDiagnosticReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a report that is ready (no blocking diagnostics).
    pub fn ready() -> Self {
        Self {
            diagnostics: Vec::new(),
            is_ready: true,
        }
    }

    /// Create a report with the given diagnostics, computing `is_ready`.
    pub fn with_diagnostics(diagnostics: Vec<ServingDiagnostic>) -> Self {
        let has_errors = diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error);
        Self {
            is_ready: !has_errors,
            diagnostics,
        }
    }

    pub fn push(&mut self, diagnostic: ServingDiagnostic) {
        if diagnostic.severity == DiagnosticSeverity::Error {
            self.is_ready = false;
        }
        self.diagnostics.push(diagnostic);
    }

    pub fn merge(&mut self, other: ServingDiagnosticReport) {
        for diag in other.diagnostics {
            self.push(diag);
        }
    }

    pub fn errors(&self) -> impl Iterator<Item = &ServingDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ServingDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
    }
}

// ---------------------------------------------------------------------------
// Validator trait
// ---------------------------------------------------------------------------

/// Validates a `ModelServingTarget` and produces a `ServingDiagnosticReport`.
///
/// Implementations should check environment, package, checkpoint, GPU, disk,
/// endpoint, authentication, capability, quota, and download prerequisites
/// depending on whether the target uses local or remote serving.
pub trait ServingValidator {
    fn validate(&self, target: &ModelServingTarget) -> ServingDiagnosticReport;
}
