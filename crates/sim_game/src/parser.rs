use std::path::PathBuf;

use crate::{
    DiagnosticCollection, DiagnosticSeverity, SourceDiagnostic, SourcePosition, SourceRange,
};

// ---------------------------------------------------------------------------
// Parse status
// ---------------------------------------------------------------------------

/// The overall status of a parse operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ParseStatus {
    /// The input was parsed completely and correctly.
    Complete,
    /// The input was parsed partially; some constructs were skipped.
    Partial,
    /// The input could not be parsed due to errors.
    Error,
}

impl ParseStatus {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Complete | Self::Partial)
    }
}

// ---------------------------------------------------------------------------
// Recoverable error
// ---------------------------------------------------------------------------

/// An error that can be recovered from during parsing, carrying a hint for
/// how to proceed.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RecoverableError {
    pub message: String,
    pub position: SourcePosition,
    pub recovery_hint: Option<String>,
}

impl RecoverableError {
    pub fn new(message: impl Into<String>, position: SourcePosition) -> Self {
        Self {
            message: message.into(),
            position,
            recovery_hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.recovery_hint = Some(hint.into());
        self
    }

    pub fn to_diagnostic(&self) -> SourceDiagnostic {
        let mut diag = SourceDiagnostic::new(&self.message, DiagnosticSeverity::Error)
            .with_range(SourceRange::point(self.position));
        if let Some(ref hint) = self.recovery_hint {
            diag = SourceDiagnostic::new(
                format!("{} (hint: {})", self.message, hint),
                DiagnosticSeverity::Error,
            )
            .with_range(SourceRange::point(self.position));
        }
        diag
    }
}

// ---------------------------------------------------------------------------
// Parse result
// ---------------------------------------------------------------------------

/// The result of a parse operation, combining the output value with
/// diagnostics collected during parsing.
#[derive(Clone, Debug)]
pub struct ParseResult<T> {
    pub value: Option<T>,
    pub status: ParseStatus,
    pub diagnostics: DiagnosticCollection,
}

impl<T> ParseResult<T> {
    /// Create a successful parse result.
    pub fn ok(value: T) -> Self {
        Self {
            value: Some(value),
            status: ParseStatus::Complete,
            diagnostics: DiagnosticCollection::new(),
        }
    }

    /// Create a partial parse result with a value and any diagnostics.
    pub fn partial(value: T, diagnostics: DiagnosticCollection) -> Self {
        Self {
            value: Some(value),
            status: ParseStatus::Partial,
            diagnostics,
        }
    }

    /// Create an error result with diagnostics and no value.
    pub fn err(diagnostics: DiagnosticCollection) -> Self {
        Self {
            value: None,
            status: ParseStatus::Error,
            diagnostics,
        }
    }

    /// Returns `true` if the parse completed successfully (complete or
    /// partial).
    pub fn is_valid(&self) -> bool {
        self.status.is_valid()
    }

    /// Convert a recoverable error into a parse result with an error
    /// diagnostic.
    pub fn from_recoverable(err: RecoverableError) -> Self {
        let mut diags = DiagnosticCollection::new();
        diags.push(err.to_diagnostic());
        Self {
            value: None,
            status: ParseStatus::Error,
            diagnostics: diags,
        }
    }

    /// Merge diagnostics from another parse result into this one.
    pub fn absorb(&mut self, other: &mut Self) {
        self.diagnostics
            .extend(std::mem::take(&mut other.diagnostics));
        if other.status == ParseStatus::Error && self.status != ParseStatus::Error {
            self.status = ParseStatus::Partial;
        }
    }

    /// Take the value out of the result, if present.
    pub fn into_value(self) -> Option<T> {
        self.value
    }
}

// ---------------------------------------------------------------------------
// Parser context
// ---------------------------------------------------------------------------

/// Context for parsing a source file, providing the file path, content, and
/// diagnostic collection.
#[derive(Clone, Debug)]
pub struct ParserContext {
    pub source_path: PathBuf,
    pub content: String,
    pub diagnostics: DiagnosticCollection,
}

impl ParserContext {
    pub fn new(source_path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            content: content.into(),
            diagnostics: DiagnosticCollection::new(),
        }
    }

    /// Create a context from a file path, reading the content.
    pub fn from_file(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path: PathBuf = path.into();
        let content = std::fs::read_to_string(&path)?;
        Ok(Self::new(path, content))
    }

    /// The number of lines in the source content.
    pub fn line_count(&self) -> usize {
        self.content.lines().count()
    }

    /// Emit a diagnostic into the context's collection.
    pub fn emit(&mut self, diagnostic: SourceDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Returns `true` if any error-level diagnostics have been emitted.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Associate diagnostics with this context's source path.
    pub fn finalize(mut self) -> DiagnosticCollection {
        for diag in &mut self.diagnostics.diagnostics {
            if diag.source_path.is_none() {
                diag.source_path = Some(self.source_path.clone());
            }
        }
        self.diagnostics
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Find the byte offset of a source position (line, column) within `content`.
///
/// Returns `None` if the position is out of bounds.
pub fn position_to_byte_offset(content: &str, position: SourcePosition) -> Option<usize> {
    let mut offset = 0usize;

    for (current_line, line) in content.lines().enumerate() {
        if current_line == position.line {
            if position.column <= line.len() {
                // Column is measured in UTF-8 bytes for simplicity.
                return Some(offset + position.column);
            }
            return None;
        }
        // +1 for the newline character.
        offset += line.len() + 1;
    }

    None
}

/// Extract the line content at the given 0-based line index.
pub fn line_at(content: &str, line: usize) -> Option<&str> {
    content.lines().nth(line)
}

/// A line iterator that remembers the current line number for diagnostics.
pub struct LineIndexer<'a> {
    lines: Vec<&'a str>,
}

impl<'a> LineIndexer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            lines: content.lines().collect(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn get(&self, line: usize) -> Option<&'a str> {
        self.lines.get(line).copied()
    }

    pub fn position(&self, line: usize, column: usize) -> Option<SourcePosition> {
        if line < self.lines.len() && column <= self.lines[line].len() {
            Some(SourcePosition::new(line, column))
        } else {
            None
        }
    }
}
