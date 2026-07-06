pub mod boundary;
pub mod inventory;
pub mod migration;
pub mod spec_gatekeeper;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod spec_gatekeeper_tests;
#[cfg(test)]
mod tests;

pub use boundary::{DefaultBoundaryPolicy, RuntimeBoundaryPolicy};
pub use inventory::{
    BaymaxGameMigrationInventory, BaymaxGameSourcePath, MigrationDecision, MigrationInventory,
    MigrationSourceArea, MigrationSpecCoverage, MigrationValidationError,
    MigrationValidationReport,
};
pub use migration::{
    BaymaxGameFeatureArea, BaymaxGameMetadata, BaymaxGameProjectDescriptor,
    BaymaxGameProjectFormat, BaymaxGameSourceReference, RuntimeBoundaryDecision,
};
pub use spec_gatekeeper::{
    DependencyWave, ExecutionGate, GateDecision, MigrationGatekeeper, MigrationTaskRef,
    SpecGatekeeper, SpecRoot,
};
