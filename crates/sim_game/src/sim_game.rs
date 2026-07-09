pub mod boundary;
pub mod debug_metadata;
pub mod dependency_review;
pub mod diagnostics;
pub mod docs_ingestion;
pub mod editor;
pub mod executable;
pub mod export;
pub mod fixtures;
pub mod formats;
pub mod generated_assets;
pub mod imports;
pub mod integration;
pub mod inventory;
pub mod language;
pub mod media;
pub mod migration;
pub mod navigation;
pub mod networking;
pub mod parser;
pub mod physics;
pub mod project;
pub mod resource_index;
pub mod spec_gatekeeper;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod dependency_review_tests;
#[cfg(test)]
mod docs_ingestion_tests;
#[cfg(test)]
mod editor_tests;
#[cfg(test)]
mod export_tests;
#[cfg(test)]
mod fixtures_tests;
#[cfg(test)]
mod formats_tests;
#[cfg(test)]
mod generated_assets_tests;
#[cfg(test)]
mod imports_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod language_tests;
#[cfg(test)]
mod media_tests;
#[cfg(test)]
mod networking_tests;
#[cfg(test)]
mod parser_tests;
#[cfg(test)]
mod physics_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod resource_index_tests;
#[cfg(test)]
mod smoke_tests;
#[cfg(test)]
mod spec_gatekeeper_tests;
#[cfg(test)]
mod tests;

pub use boundary::{DefaultBoundaryPolicy, RuntimeBoundaryPolicy};
pub use debug_metadata::{SimGameDebugEndpoint, SimGameDebugMetadata, SimGameDebugProtocol};
pub use dependency_review::{
    DependencyReviewGate, SimGameDependencyKind, SimGameDependencyProposal,
    SimGameDependencyReviewDecision, SimGameDependencyReviewDiagnostic,
    SimGameDependencyReviewRecord, SimGameDependencyReviewReport, SimGameDependencyReviewStatus,
};
pub use diagnostics::{
    DiagnosticCollection, DiagnosticSeverity, SourceDiagnostic, SourcePosition, SourceRange,
};
pub use docs_ingestion::{
    SimGameDocsIngestion, SimGameDocsIngestionDiagnostic, SimGameDocsIngestionReport,
    SimGameDocsRecord, SimGameDocsSource,
};
pub use editor::{
    SimGameCommand, SimGameCommandProvider, SimGameProjectPanelMetadata, SimGameRunDebugTemplate,
    SimGameRunDebugTemplateKind, SimGameSetupDiagnostic,
};
pub use executable::{SimGameExecutableDiagnostic, SimGameExecutableSettings};
pub use export::{
    SimGameExportPreset, SimGameExportPresetParser, SimGameExportTaskDiagnostic,
    SimGameExportTaskTemplate,
};
pub use fixtures::{
    FixtureAttribution, FixtureAttributionDiagnostic, FixtureAttributionReport,
    FixtureAttributionValidator, FixtureLicense, FixtureManifest, FixtureSource,
};
pub use formats::{
    SimGameFormatClassification, SimGameFormatClassifier, SimGameFormatDiagnostic,
    SimGameFormatKind, SimGameResourceReference, SimGameTextResourceParse,
    SimGameTextResourceParser,
};
pub use generated_assets::{
    SimGeneratedAssetDiagnostic, SimGeneratedAssetRecord, SimGeneratedAssetRegistry,
};
pub use imports::{SimGameImportDiagnostic, SimGameImportLink, SimGameImportMetadataLinker};
pub use integration::{
    ExternalGameTaskProvider, GameAssetPreviewRoute, PreviewKind, SimScriptLanguageConfig,
    default_game_preview_routes, default_game_task_providers, detect_game_project_roots,
    is_game_project_manifest, simscript_language_config, target_project_format,
};
pub use inventory::{
    MigrationDecision, MigrationInventory, MigrationSourceArea, MigrationSpecCoverage,
    MigrationValidationError, MigrationValidationReport, SimGameMigrationInventory,
    SimGameSourcePath,
};
pub use language::{
    NaturalLanguageGameAuthoring, SimGameDocsEntry, SimGameDocsIndex, SimGameDocsScope,
    SimScriptAuthoringDiagnostic, SimScriptAuthoringDraft, SimScriptAuthoringStatus, SimScriptDiff,
    SimScriptDiffKind, SimScriptFileClassification, SimScriptFileKind, SimScriptLanguageSupport,
};
pub use media::{SimGameMediaClassification, SimGameMediaClassifier, SimGameMediaKind};
pub use migration::{
    RuntimeBoundaryDecision, SimGameFeatureArea, SimGameMetadata, SimGameProjectDescriptor,
    SimGameProjectFormat, SimGameSourceReference,
};
pub use navigation::{SimGameNavigationMetadata, SimGameNavigationMetadataExtractor};
pub use networking::{
    SimGameNetworkBoundary, SimGameNetworkBoundaryDecision, SimGameNetworkFeature,
    SimGameNetworkFeatureKind,
};
pub use parser::{
    LineIndexer, ParseResult, ParseStatus, ParserContext, RecoverableError, line_at,
    position_to_byte_offset,
};
pub use physics::{
    SimGamePhysicsMetadata, SimGamePhysicsMetadataExtractor, SimGameSimulationBoundary,
    SimGameSimulationBoundaryDecision, SimGameSimulationFallbackTask, SimGameSimulationFeature,
    SimGameSimulationFeatureKind,
};
pub use project::{SimGameProjectDiagnostic, SimGameProjectMetadata, SimGameProjectMetadataParser};
pub use resource_index::{SimGameIndexedResource, SimGameResourceIndex, SimGameResourceParseState};
pub use spec_gatekeeper::{
    DependencyWave, ExecutionGate, GateDecision, MigrationGatekeeper, MigrationTaskRef,
    SpecGatekeeper, SpecRoot,
};
