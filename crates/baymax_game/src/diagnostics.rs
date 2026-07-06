use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Source positions and ranges
// ---------------------------------------------------------------------------

/// A zero-indexed position in a source file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

impl SourcePosition {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.column + 1)
    }
}

/// A half-open range `[start, end)` in a source file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceRange {
    pub const fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    /// Create a range spanning a single position.
    pub fn point(position: SourcePosition) -> Self {
        Self {
            start: position,
            end: position,
        }
    }
}

impl fmt::Display for SourceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.start, self.end)
    }
}

// ---------------------------------------------------------------------------
// Diagnostic severity
// ---------------------------------------------------------------------------

/// The severity level of a source diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// An error that blocks further processing.
    Error,
    /// A warning that indicates a potential problem.
    Warning,
    /// An informational message.
    Info,
    /// A hint that suggests improvements.
    Hint,
}

impl DiagnosticSeverity {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

// ---------------------------------------------------------------------------
// Source diagnostics
// ---------------------------------------------------------------------------

/// A diagnostic message associated with a source location.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourceDiagnostic {
    pub message: String,
    pub range: Option<SourceRange>,
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub source_path: Option<PathBuf>,
}

impl SourceDiagnostic {
    pub fn new(message: impl Into<String>, severity: DiagnosticSeverity) -> Self {
        Self {
            message: message.into(),
            range: None,
            severity,
            code: None,
            source_path: None,
        }
    }

    pub fn with_range(mut self, range: SourceRange) -> Self {
        self.range = Some(range);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Returns `true` if this diagnostic is an error.
    pub fn is_error(&self) -> bool {
        self.severity.is_error()
    }
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sev = self.severity.label();
        if let Some(ref path) = self.source_path {
            write!(f, "{sev}: {}", path.display())?;
        } else {
            write!(f, "{sev}")?;
        }
        if let Some(range) = self.range {
            write!(f, " [{range}]")?;
        }
        if let Some(ref code) = self.code {
            write!(f, " ({code})")?;
        }
        write!(f, ": {}", self.message)
    }
}

// ---------------------------------------------------------------------------
// Diagnostic collection
// ---------------------------------------------------------------------------

/// A collection of source diagnostics produced during parsing or validation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCollection {
    pub diagnostics: Vec<SourceDiagnostic>,
}

impl DiagnosticCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: SourceDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, other: Self) {
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn errors(&self) -> impl Iterator<Item = &SourceDiagnostic> {
        self.diagnostics.iter().filter(|d| d.is_error())
    }

    pub fn warnings(&self) -> impl Iterator<Item = &SourceDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
    }
}

impl FromIterator<SourceDiagnostic> for DiagnosticCollection {
    fn from_iter<I: IntoIterator<Item = SourceDiagnostic>>(iter: I) -> Self {
        Self {
            diagnostics: iter.into_iter().collect(),
        }
    }
}
