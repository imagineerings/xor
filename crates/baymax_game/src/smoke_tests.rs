use std::path::Path;

use crate::{
    BaymaxGameFeatureArea, BaymaxGameMetadata, BaymaxGameMigrationInventory,
    BaymaxGameProjectDescriptor, BaymaxGameSourcePath, BaymaxGameSourceReference,
    DefaultBoundaryPolicy, DiagnosticCollection, DiagnosticSeverity, FixtureAttribution,
    FixtureLicense, FixtureSource, LineIndexer, MigrationDecision, MigrationInventory,
    MigrationSourceArea, MigrationSpecCoverage, ParseResult, ParserContext,
    RuntimeBoundaryDecision, RuntimeBoundaryPolicy, SourceDiagnostic, SourcePosition, SourceRange,
};

// ---------------------------------------------------------------------------
// Smoke test: godot project detection + metadata wiring
// ---------------------------------------------------------------------------

#[test]
fn detects_godot_project_and_creates_metadata() {
    // Simulate finding a project.godot manifest and creating metadata.
    let manifest_path = Path::new("/workspace/my-game/project.godot");
    let descriptor =
        BaymaxGameProjectDescriptor::from_godot_compatible_manifest_path(manifest_path);
    assert!(
        descriptor.is_some(),
        "Expected descriptor for project.godot"
    );

    let desc = descriptor.unwrap();
    assert_eq!(desc.format, crate::BaymaxGameProjectFormat::GodotCompatible);
    assert!(desc.root_path.ends_with("my-game"));

    // Create metadata routing for a scene file.
    let source_ref = BaymaxGameSourceReference::new("scenes/main.tscn").with_position(10, 5);
    let metadata = BaymaxGameMetadata::new(
        BaymaxGameFeatureArea::SceneResourceMetadata,
        source_ref,
        RuntimeBoundaryDecision::NativeBaymaxFeature,
    );
    assert_eq!(
        metadata.feature_area,
        BaymaxGameFeatureArea::SceneResourceMetadata
    );
    assert!(metadata.boundary.is_executable_inside_baymax());
}

#[test]
fn rejects_non_godot_manifest() {
    let manifest = Path::new("/workspace/my-game/package.json");
    let descriptor = BaymaxGameProjectDescriptor::from_godot_compatible_manifest_path(manifest);
    assert!(descriptor.is_none(), "package.json should not be detected");
}

// ---------------------------------------------------------------------------
// Smoke test: boundary policy decisions
// ---------------------------------------------------------------------------

#[test]
fn boundary_policy_routes_all_major_categories() {
    let policy = DefaultBoundaryPolicy;

    // Baymax-owned → Native
    let decision = policy.classify("ui", "Editor UI and panels");
    assert_eq!(decision, RuntimeBoundaryDecision::NativeBaymaxFeature);

    // Godot runtime → Excluded
    let decision = policy.classify("physics", "Godot physics engine runtime");
    assert!(matches!(decision, RuntimeBoundaryDecision::Excluded { .. }));

    // External tool → ExternalCommand
    let decision = policy.classify("export", "Build and export pipeline");
    assert!(matches!(
        decision,
        RuntimeBoundaryDecision::ExternalCommand { .. }
    ));

    // World generation → BaymaxAdapter
    let decision = policy.classify("world-gen", "World generation inference");
    assert!(matches!(
        decision,
        RuntimeBoundaryDecision::BaymaxAdapter { .. }
    ));
}

// ---------------------------------------------------------------------------
// Smoke test: diagnostics end-to-end
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_collection_flow() {
    let mut coll = DiagnosticCollection::new();

    coll.push(
        SourceDiagnostic::new("missing import", DiagnosticSeverity::Error)
            .with_code("E001")
            .with_range(SourceRange::new(
                SourcePosition::new(0, 0),
                SourcePosition::new(0, 15),
            )),
    );
    coll.push(
        SourceDiagnostic::new("unused variable", DiagnosticSeverity::Warning).with_code("W001"),
    );

    assert!(coll.has_errors());
    assert_eq!(coll.errors().count(), 1);
    assert_eq!(coll.warnings().count(), 1);
}

// ---------------------------------------------------------------------------
// Smoke test: parser context with diagnostics
// ---------------------------------------------------------------------------

#[test]
fn parser_context_with_parse_result() {
    let mut ctx = ParserContext::new("test.gd", "extends Node\n\nfunc _ready():\n    pass\n");

    ctx.emit(SourceDiagnostic::new("unused import", DiagnosticSeverity::Warning).with_code("W002"));

    assert!(!ctx.has_errors());
    assert_eq!(ctx.line_count(), 4);

    let diagnostics = ctx.finalize();
    assert_eq!(diagnostics.diagnostics.len(), 1);
    assert_eq!(
        diagnostics.diagnostics[0].source_path.as_deref(),
        Some(Path::new("test.gd"))
    );
}

#[test]
fn parse_result_combined_with_diagnostics() {
    let result: ParseResult<i32> = ParseResult::ok(42);
    assert!(result.is_valid());
    assert_eq!(result.into_value(), Some(42));
}

// ---------------------------------------------------------------------------
// Smoke test: fixture attribution recording
// ---------------------------------------------------------------------------

#[test]
fn fixture_attribution_records_source() {
    let attr = FixtureAttribution::new(
        "fixtures/textures/floor.png",
        FixtureSource::Url {
            url: "https://example.com/floor.png".into(),
        },
        FixtureLicense::Spdx("CC0-1.0".into()),
    )
    .with_author("TextureAuthor")
    .with_notes("Converted from PNG to WebP");

    assert_eq!(attr.author.as_deref(), Some("TextureAuthor"));
    assert!(attr.source.description().contains("URL"));
    assert_eq!(attr.license.label(), "CC0-1.0");
}

// ---------------------------------------------------------------------------
// Smoke test: LineIndexer for source file navigation
// ---------------------------------------------------------------------------

#[test]
fn line_indexer_navigates_source() {
    let indexer = LineIndexer::new("line1\nline2\nline3\n");
    assert_eq!(indexer.line_count(), 3);
    assert_eq!(indexer.get(0), Some("line1"));
    assert_eq!(indexer.get(2), Some("line3"));
    assert_eq!(indexer.position(1, 2), Some(SourcePosition::new(1, 2)));
}

// ---------------------------------------------------------------------------
// Smoke test: inventory boundary decisions
// ---------------------------------------------------------------------------

#[test]
fn inventory_classifies_source_areas() {
    let mut inventory = MigrationInventory::new("/specs");
    inventory.source_areas = vec![
        MigrationSourceArea::new(
            "godot-engine",
            "/godot",
            "Godot engine runtime",
            MigrationDecision::Excluded {
                reason: "Duplicates Baymax runtime (Req 2.2)".into(),
            },
            None::<String>,
        ),
        MigrationSourceArea::new(
            "world-gen",
            "/world-model",
            "World model generation",
            MigrationDecision::BaymaxAdapter {
                owner: "world_model".into(),
            },
            Some("/specs/world-model-runtime"),
        ),
    ];

    let decision =
        inventory.classify_source_area(&BaymaxGameSourcePath::new("/world-model/some/file.rs"));
    assert!(matches!(decision, MigrationDecision::BaymaxAdapter { .. }));

    let decision =
        inventory.classify_source_area(&BaymaxGameSourcePath::new("/godot/engine/core.cpp"));
    assert!(matches!(decision, MigrationDecision::Excluded { .. }));
}

// ---------------------------------------------------------------------------
// Smoke test: spec coverage validation
// ---------------------------------------------------------------------------

#[test]
fn spec_coverage_creates_with_name_scope_location() {
    let spec =
        MigrationSpecCoverage::new("engine-core-runtime", "core", "/specs/engine-core-runtime");
    assert_eq!(spec.name, "engine-core-runtime");
    assert_eq!(spec.scope, "core");
    assert!(spec.location.to_string_lossy().contains("specs"));
}

// ---------------------------------------------------------------------------
// Smoke test: error diagnostic display
// ---------------------------------------------------------------------------

#[test]
fn source_diagnostic_display_formats() {
    let diag = SourceDiagnostic::new("file not found", DiagnosticSeverity::Error)
        .with_code("FS001")
        .with_source_path("project.godot");
    let text = diag.to_string();
    assert!(text.contains("error"));
    assert!(text.contains("FS001"));
    assert!(text.contains("project.godot"));
    assert!(text.contains("file not found"));
}
