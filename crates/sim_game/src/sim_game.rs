pub mod boundary;
pub mod diagnostics;
pub mod fixtures;
pub mod inventory;
pub mod migration;
pub mod parser;
pub mod spec_gatekeeper;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod fixtures_tests;
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
pub use inventory::{
    SimGameMigrationInventory, SimGameSourcePath, MigrationDecision, MigrationInventory,
    MigrationSourceArea, MigrationSpecCoverage, MigrationValidationError,
    MigrationValidationReport,
};
pub use migration::{
    SimGameFeatureArea, SimGameMetadata, SimGameProjectDescriptor,
    SimGameProjectFormat, SimGameSourceReference, RuntimeBoundaryDecision,
};
pub use parser::{
    LineIndexer, ParseResult, ParseStatus, ParserContext, RecoverableError, line_at,
    position_to_byte_offset,
};
pub use spec_gatekeeper::{
    DependencyWave, ExecutionGate, GateDecision, MigrationGatekeeper, MigrationTaskRef,
    SpecGatekeeper, SpecRoot,
};
