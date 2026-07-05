pub mod inventory;
pub mod migration;

#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod tests;

pub use inventory::{
    BaymaxGameMigrationInventory, BaymaxGameSourcePath, MigrationDecision, MigrationInventory,
    MigrationSourceArea, MigrationSpecCoverage, MigrationValidationError,
    MigrationValidationReport,
};
pub use migration::{
    BaymaxGameFeatureArea, BaymaxGameMetadata, BaymaxGameProjectDescriptor,
    BaymaxGameProjectFormat, BaymaxGameSourceReference, RuntimeBoundaryDecision,
};
