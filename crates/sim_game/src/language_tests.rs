use std::path::Path;

use crate::{
    NaturalLanguageGameAuthoring, SimGameDocsEntry, SimGameDocsIndex, SimGameDocsScope,
    SimScriptAuthoringStatus, SimScriptDiffKind, SimScriptFileKind, SimScriptLanguageSupport,
};

#[test]
fn language_support_classifies_native_simscript_files() {
    let support = SimScriptLanguageSupport::native();
    let classification = support
        .classify_path("scripts/player.simscript")
        .expect("classification");

    assert_eq!(classification.kind, SimScriptFileKind::Native);
    assert_eq!(classification.language_name, "SimScript");
    assert!(classification.migration_source_format.is_none());
    assert_eq!(support.lsp_adapter_name(), Some("simscript-lsp"));
}

#[test]
fn language_support_classifies_gd_as_imported_source() {
    let support = SimScriptLanguageSupport::native();
    let classification = support
        .classify_path("legacy/player.gd")
        .expect("classification");

    assert_eq!(classification.kind, SimScriptFileKind::ImportedGdSource);
    assert_eq!(
        classification.migration_source_format.as_deref(),
        Some("gdscript")
    );
}

#[test]
fn docs_index_keeps_sim_api_primary_and_godot_as_reference() {
    let index = SimGameDocsIndex::new()
        .with_entry(SimGameDocsEntry::sim_api(
            "SimCharacter",
            "Native Sim gameplay character API",
        ))
        .with_entry(SimGameDocsEntry::migration_reference(
            "Node3D",
            "Godot node reference for imported scenes",
            Path::new("godot/classes/Node3D.xml"),
        ));

    assert_eq!(index.sim_api_entries().len(), 1);
    assert_eq!(index.migration_reference_entries().len(), 1);
    assert_eq!(
        index.lookup("character")[0].scope,
        SimGameDocsScope::PrimarySimApi
    );
    assert_eq!(
        index.lookup("Node3D")[0].scope,
        SimGameDocsScope::MigrationReference
    );
}

#[test]
fn natural_language_authoring_produces_inspectable_simscript() {
    let authoring = NaturalLanguageGameAuthoring::new();
    let draft = authoring.draft("make the player jump when pressing space");

    assert_eq!(draft.status, SimScriptAuthoringStatus::Draft);
    assert!(draft.simscript.contains("behavior GeneratedBehavior"));
    assert!(draft.simscript.contains("intent"));
    assert!(
        !draft
            .simscript
            .contains("make the player jump when pressing space\n")
    );
}

#[test]
fn natural_language_authoring_reports_ambiguous_empty_instruction() {
    let authoring = NaturalLanguageGameAuthoring::new();
    let diff = authoring.diff("behavior Player:\n", "   ");

    assert_eq!(diff.kind, SimScriptDiffKind::NonDestructiveDraft);
    assert_eq!(diff.original, "behavior Player:\n");
    assert!(diff.generated.is_empty());
    assert_eq!(
        diff.draft.status,
        SimScriptAuthoringStatus::NeedsClarification
    );
}

#[test]
fn natural_language_authoring_shows_update_diff_before_application() {
    let authoring = NaturalLanguageGameAuthoring::new();
    let diff = authoring.diff(
        "behavior Player:\n    on ready:\n        pass\n",
        "add dash",
    );

    assert_eq!(diff.kind, SimScriptDiffKind::Update);
    assert!(diff.generated.contains("add dash"));
    assert_ne!(diff.original, diff.generated);
}
