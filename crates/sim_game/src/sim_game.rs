pub mod boundary;
pub mod diagnostics;
pub mod fixtures;
pub mod integration;
pub mod inventory;
pub mod migration;
pub mod parser;
pub mod spec_gatekeeper;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod fixtures_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod parser_tests;
#[cfg(test)]
mod smoke_tests;
#[cfg(test)]
mod spec_gatekeeper_tests;
#[cfg(test)]
mod tests;

pub use boundary::{DefaultBoundaryPolicy, RuntimeBoundaryPolicy};
pub use diagnostics::{
    DiagnosticCollection, DiagnosticSeverity, SourceDiagnostic, SourcePosition, SourceRange,
};
pub use fixtures::{FixtureAttribution, FixtureLicense, FixtureManifest, FixtureSource};
pub use integration::{
    ExternalGameTaskProvider, GameAssetPreviewRoute, SimScriptLanguageConfig, PreviewKind,
    default_game_preview_routes, default_game_task_providers, detect_game_project_roots,
    simscript_language_config, is_game_project_manifest, target_project_format,
};
pub use inventory::{
    MigrationDecision, MigrationInventory, MigrationSourceArea, MigrationSpecCoverage,
    MigrationValidationError, MigrationValidationReport, SimGameMigrationInventory,
    SimGameSourcePath,
};
pub use migration::{
    RuntimeBoundaryDecision, SimGameFeatureArea, SimGameMetadata, SimGameProjectDescriptor,
    SimGameProjectFormat, SimGameSourceReference,
};
pub use parser::{
    LineIndexer, ParseResult, ParseStatus, ParserContext, RecoverableError, line_at,
    position_to_byte_offset,
};
pub use spec_gatekeeper::{
    DependencyWave, ExecutionGate, GateDecision, MigrationGatekeeper, MigrationTaskRef,
    SpecGatekeeper, SpecRoot,
};
