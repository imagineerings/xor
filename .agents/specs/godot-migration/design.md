# Design: Godot Migration Umbrella

## Architecture

The migration is organized as Baymax-native integration layers:

- `crates/godot`: Godot-compatible metadata, boundary policy, parsing diagnostics, project descriptors, fixture attribution, and task/debug/export descriptors.
- `crates/world_model`: world-model request/control/worker/graph/mesh/artifact/provenance primitives.
- Existing Baymax crates: project, worktree, language, LSP, tasks, debugger, media, UI, agent, and app registry own their existing domains.

## Components

### GodotMigrationInventory

Validates that grouped spec coverage exists for every accepted feature area and that excluded runtime areas have explicit boundary reasons.

```rust
pub trait GodotMigrationInventory {
    fn validate_spec_pack(&self) -> MigrationValidationReport;
    fn classify_source_area(&self, path: &GodotSourcePath) -> MigrationDecision;
}
```

### RuntimeBoundaryPolicy

Encodes whether a feature is metadata-only, Baymax-adapter, external-command, or excluded.

```rust
pub enum MigrationDecision {
    MetadataOnly,
    BaymaxAdapter,
    ExternalCommand,
    Excluded,
}
```

### WorldModelHarness

Owns typed generation requests, worker diagnostics, persistent sessions, generated artifacts, and provenance.

```rust
pub struct WorldGenerationRequest {
    pub prompt: String,
    pub source_image: Option<String>,
    pub controls: Vec<WorldControl>,
    pub model_profile: String,
    pub output_target: String,
}
```

### DiffusionGraphRuntime

Owns graph nodes, edges, validation, execution plans, and artifact outputs.

```rust
pub trait DiffusionGraphValidator {
    fn validate(&self, graph: &DiffusionGraph) -> GraphValidationResult;
}
```

### MigrationGatekeeper

Blocks implementation work when spec consistency, dependency waves, or execution gates are not satisfied.

```rust
pub trait MigrationGatekeeper {
    fn validate_spec_pack(&self, root: &SpecRoot) -> MigrationValidationReport;
    fn can_execute_task(&self, task: &MigrationTaskRef, satisfied_gates: &BTreeSet<ExecutionGate>) -> GateDecision;
}
```

## Data Models

```rust
pub enum ExecutionGate {
    SpecConsistency,
    BoundaryPolicy,
    SharedGodotMetadata,
    SharedWorldModelFoundations,
    WorkerSafety,
    GraphSafety,
    Provenance,
    DependencyReview,
}

pub enum DependencyWave {
    PlanningValidation,
    SharedFoundations,
    GodotCompatibilitySubstrate,
    WorldModelServingSubstrate,
    AuthoringAndGraphUx,
    GenerationOutputsAndAssetPipelines,
    ExternalExecutionHardening,
}
```

## Correctness Properties

### Property 1: No Runtime Duplication

_For any_ Godot subsystem that duplicates Baymax platform, rendering, UI, input, physics, networking, audio, XR, or text infrastructure, the boundary policy SHALL classify it as excluded or external-command only.

**Validates: Requirement 2.1, 2.2**

### Property 2: Complete Spec Pack

_For any_ grouped migration spec listed in the master plan, validation SHALL require `requirements.md`, `design.md`, and `tasks.md`.

**Validates: Requirement 1.2, 12.4**

### Property 3: Task Traceability

_For any_ implementation task, validation SHALL require both requirement references and expected write targets.

**Validates: Requirement 10.1, 10.2**

### Property 4: Gate Enforcement

_For any_ implementation task, the gatekeeper SHALL block execution when required gates for that task are unsatisfied.

**Validates: Requirement 10.3, 12.1, 12.2, 12.3**

### Property 5: World-Model Provenance

_For any_ generated video, mesh, texture, or exported artifact, the artifact record SHALL retain prompt, controls, model settings, source assets, graph node, and output path metadata.

**Validates: Requirement 4.2, 5.4, 7.2, 8.3**

## Error Handling

- Missing spec files produce blocking G0 errors.
- Runtime-adjacent work without boundary policy produces blocking G1 errors.
- Worker execution without diagnostics produces blocking G4 errors.
- Graph execution without validation produces blocking G5 errors.
- Generated artifact import without provenance produces blocking G6 errors.
- Heavy/native dependencies without review produce blocking G7 errors.
