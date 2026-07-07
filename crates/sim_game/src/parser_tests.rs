use crate::{
    DiagnosticSeverity, LineIndexer, ParseResult, ParseStatus, ParserContext, RecoverableError,
    SourceDiagnostic, SourcePosition, SourceRange, line_at, position_to_byte_offset,
};

// ---------------------------------------------------------------------------
// SourcePosition
// ---------------------------------------------------------------------------

#[test]
fn source_position_display_uses_one_indexed() {
    let pos = SourcePosition::new(0, 0);
    assert_eq!(pos.to_string(), "1:1");
}

#[test]
fn source_position_display_five_three() {
    let pos = SourcePosition::new(4, 2);
    assert_eq!(pos.to_string(), "5:3");
}

// ---------------------------------------------------------------------------
// SourceRange
// ---------------------------------------------------------------------------

#[test]
fn source_range_point_creates_zero_width_range() {
    let pos = SourcePosition::new(1, 5);
    let range = SourceRange::point(pos);
    assert_eq!(range.start, pos);
    assert_eq!(range.end, pos);
}

#[test]
fn source_range_new_creates_half_open_range() {
    let start = SourcePosition::new(0, 0);
    let end = SourcePosition::new(0, 10);
    let range = SourceRange::new(start, end);
    assert_eq!(range.start, start);
    assert_eq!(range.end, end);
}

// ---------------------------------------------------------------------------
// DiagnosticSeverity
// ---------------------------------------------------------------------------

#[test]
fn error_severity_is_error() {
    assert!(DiagnosticSeverity::Error.is_error());
}

#[test]
fn warning_severity_is_not_error() {
    assert!(!DiagnosticSeverity::Warning.is_error());
}

#[test]
fn severity_labels_match() {
    assert_eq!(DiagnosticSeverity::Error.label(), "error");
    assert_eq!(DiagnosticSeverity::Warning.label(), "warning");
    assert_eq!(DiagnosticSeverity::Info.label(), "info");
    assert_eq!(DiagnosticSeverity::Hint.label(), "hint");
}

// ---------------------------------------------------------------------------
// SourceDiagnostic
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_builder_creates_error_with_message() {
    let diag = SourceDiagnostic::new("something went wrong", DiagnosticSeverity::Error);
    assert_eq!(diag.message, "something went wrong");
    assert!(diag.is_error());
    assert!(diag.range.is_none());
}

#[test]
fn diagnostic_with_range_sets_range() {
    let range = SourceRange::point(SourcePosition::new(0, 5));
    let diag = SourceDiagnostic::new("test", DiagnosticSeverity::Warning).with_range(range);
    assert_eq!(diag.range, Some(range));
}

#[test]
fn diagnostic_with_code_sets_code() {
    let diag = SourceDiagnostic::new("test", DiagnosticSeverity::Error).with_code("E001");
    assert_eq!(diag.code.as_deref(), Some("E001"));
}

#[test]
fn diagnostic_display_includes_severity_and_message() {
    let diag = SourceDiagnostic::new("file not found", DiagnosticSeverity::Error);
    let text = diag.to_string();
    assert!(text.contains("error"));
    assert!(text.contains("file not found"));
}

// ---------------------------------------------------------------------------
// DiagnosticCollection
// ---------------------------------------------------------------------------

#[test]
fn collection_starts_empty() {
    let coll = crate::DiagnosticCollection::new();
    assert!(coll.is_empty());
    assert!(!coll.has_errors());
}

#[test]
fn collection_detects_errors() {
    let mut coll = crate::DiagnosticCollection::new();
    coll.push(SourceDiagnostic::new("err", DiagnosticSeverity::Error));
    assert!(coll.has_errors());
}

#[test]
fn collection_errors_iterator_returns_only_errors() {
    let mut coll = crate::DiagnosticCollection::new();
    coll.push(SourceDiagnostic::new("err", DiagnosticSeverity::Error));
    coll.push(SourceDiagnostic::new("warn", DiagnosticSeverity::Warning));
    assert_eq!(coll.errors().count(), 1);
    assert_eq!(coll.warnings().count(), 1);
}

// ---------------------------------------------------------------------------
// ParseStatus
// ---------------------------------------------------------------------------

#[test]
fn parse_status_complete_is_valid() {
    assert!(ParseStatus::Complete.is_valid());
}

#[test]
fn parse_status_partial_is_valid() {
    assert!(ParseStatus::Partial.is_valid());
}

#[test]
fn parse_status_error_is_not_valid() {
    assert!(!ParseStatus::Error.is_valid());
}

// ---------------------------------------------------------------------------
// RecoverableError
// ---------------------------------------------------------------------------

#[test]
fn recoverable_error_creates_with_message_and_position() {
    let err = RecoverableError::new("unexpected token", SourcePosition::new(2, 10));
    assert_eq!(err.message, "unexpected token");
    assert_eq!(err.position, SourcePosition::new(2, 10));
    assert!(err.recovery_hint.is_none());
}

#[test]
fn recoverable_error_with_hint_sets_hint() {
    let err = RecoverableError::new("bad indent", SourcePosition::new(1, 0))
        .with_hint("check indentation");
    assert_eq!(err.recovery_hint.as_deref(), Some("check indentation"));
}

#[test]
fn recoverable_error_converts_to_diagnostic() {
    let err = RecoverableError::new("bad value", SourcePosition::new(0, 3));
    let diag = err.to_diagnostic();
    assert!(diag.is_error());
    assert!(diag.message.contains("bad value"));
}

// ---------------------------------------------------------------------------
// ParseResult
// ---------------------------------------------------------------------------

#[test]
fn parse_result_ok_creates_complete_result() {
    let result = ParseResult::ok(42);
    assert_eq!(result.value, Some(42));
    assert_eq!(result.status, ParseStatus::Complete);
    assert!(result.is_valid());
}

#[test]
fn parse_result_err_creates_error_result() {
    let mut diags = crate::DiagnosticCollection::new();
    diags.push(SourceDiagnostic::new("fail", DiagnosticSeverity::Error));
    let result: ParseResult<i32> = ParseResult::err(diags);
    assert!(result.value.is_none());
    assert_eq!(result.status, ParseStatus::Error);
    assert!(!result.is_valid());
}

#[test]
fn parse_result_absorb_merges_diagnostics() {
    let mut a = ParseResult::ok("hello");
    let mut diags = crate::DiagnosticCollection::new();
    diags.push(SourceDiagnostic::new("warn", DiagnosticSeverity::Warning));
    let mut b: ParseResult<&str> = ParseResult::partial("partial", diags);
    a.absorb(&mut b);
    assert_eq!(a.diagnostics.diagnostics.len(), 1);
    // Absorbing a non-error does not downgrade the status.
    assert_eq!(a.status, ParseStatus::Complete);
}

#[test]
fn parse_result_absorb_from_error_downgrades_to_partial() {
    let mut a = ParseResult::ok("hello");
    let mut diags = crate::DiagnosticCollection::new();
    diags.push(SourceDiagnostic::new("err", DiagnosticSeverity::Error));
    let mut b: ParseResult<&str> = ParseResult::err(diags);
    a.absorb(&mut b);
    assert_eq!(a.status, ParseStatus::Partial);
    assert!(a.is_valid()); // Partial is valid
}

#[test]
fn parse_result_into_value_returns_option() {
    let result = ParseResult::ok(99);
    assert_eq!(result.into_value(), Some(99));
}

// ---------------------------------------------------------------------------
// ParserContext
// ---------------------------------------------------------------------------

#[test]
fn parser_context_creates_from_path_and_content() {
    let ctx = ParserContext::new("test.txt", "line1\nline2\n");
    assert_eq!(ctx.source_path.to_str(), Some("test.txt"));
    assert_eq!(ctx.line_count(), 2);
}

#[test]
fn parser_context_finalize_attaches_source_path() {
    let mut ctx = ParserContext::new("source.gd", "hello");
    ctx.emit(SourceDiagnostic::new(
        "test error",
        DiagnosticSeverity::Error,
    ));
    let coll = ctx.finalize();
    assert_eq!(coll.diagnostics.len(), 1);
    assert_eq!(
        coll.diagnostics[0].source_path.as_deref(),
        Some(std::path::Path::new("source.gd"))
    );
}

// ---------------------------------------------------------------------------
// Position helpers
// ---------------------------------------------------------------------------

#[test]
fn position_to_byte_offset_first_line() {
    // "abc\ndef\n" — 'a'=0, 'b'=1, 'c'=2, '\n'=3, 'd'=4, ...
    let content = "abc\ndef\n";
    let offset = position_to_byte_offset(content, SourcePosition::new(0, 1)).unwrap();
    assert_eq!(offset, 1); // 'b'
}

#[test]
fn position_to_byte_offset_second_line() {
    let content = "abc\ndef\n";
    let offset = position_to_byte_offset(content, SourcePosition::new(1, 0)).unwrap();
    assert_eq!(offset, 4); // 'd'
}

#[test]
fn position_to_byte_offset_out_of_bounds() {
    let content = "hi";
    let offset = position_to_byte_offset(content, SourcePosition::new(5, 0));
    assert!(offset.is_none());
}

#[test]
fn line_at_returns_correct_line() {
    let content = "first\nsecond\nthird";
    assert_eq!(line_at(content, 0), Some("first"));
    assert_eq!(line_at(content, 1), Some("second"));
    assert_eq!(line_at(content, 2), Some("third"));
    assert_eq!(line_at(content, 3), None);
}

// ---------------------------------------------------------------------------
// LineIndexer
// ---------------------------------------------------------------------------

#[test]
fn line_indexer_counts_lines() {
    let indexer = LineIndexer::new("a\nb\nc\n");
    assert_eq!(indexer.line_count(), 3);
}

#[test]
fn line_indexer_gets_line() {
    let indexer = LineIndexer::new("hello\nworld");
    assert_eq!(indexer.get(0), Some("hello"));
    assert_eq!(indexer.get(1), Some("world"));
    assert_eq!(indexer.get(2), None);
}

#[test]
fn line_indexer_position_creates_valid_position() {
    let indexer = LineIndexer::new("abcdef");
    let pos = indexer.position(0, 3);
    assert_eq!(pos, Some(SourcePosition::new(0, 3)));
}

#[test]
fn line_indexer_position_out_of_bounds_column() {
    let indexer = LineIndexer::new("abc");
    assert!(indexer.position(0, 10).is_none());
}
