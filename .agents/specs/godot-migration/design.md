# Design: Sim Game Development Surface

## Architecture

The game development surface is organized as Sim-native integration layers:

- `crates/sim_game`: Sim-owned game authoring metadata, boundary policy, parsing diagnostics, project descriptors, fixture attribution, and task/debug/export descriptors for Godot-format source projects.
- `crates/world_model`: world-model request/control/worker/graph/mesh/artifact/provenance primitives.
- Comfy harness modules inside `crates/world_model`: core runtime control-plane adapters, Comfy graph/node schemas, sampler/scheduler execution semantics, conditioning, latent/VAE behavior, model patching, diffusion/world-model runner profiles, model folder and memory policy, asset APIs, workflow/blueprint catalogs, provider connectors, extension loading policy, and compatibility fixtures.
- Existing Sim crates: project, worktree, language, LSP, tasks, debugger, media, UI, agent, and app registry own their existing domains.

### Design Principle: Native Integration

Every Godot-originated feature becomes a native Sim feature. There is no compatibility shim layer, no "registrar trait" bridging game features to Sim registries, and no parallel language config type. SimScript is registered as the first-class executable game language via `LanguageRegistry::add` with the same `Language` type used for Rust, Python, and TypeScript; natural language is the authoring interface that produces inspectable SimScript. `sim_game` exports pure-data helpers for external game task templates and game asset preview routes so follow-on task and preview sub-specs can wire them into the native task source and preview action systems without introducing an intermediate compatibility layer.

## Components

### SimGameMigrationInventory

Validates that grouped spec coverage exists for every accepted feature area and that excluded runtime areas have explicit boundary reasons.

```rust
pub trait SimGameMigrationInventory {
    fn validate_spec_pack(&self) -> MigrationValidationReport;
    fn classify_source_area(&self, path: &SimGameSourcePath) -> MigrationDecision;
}
```

### RuntimeBoundaryPolicy

Encodes whether a feature is metadata-only, Sim-adapter, external-command, or excluded.

```rust
pub enum MigrationDecision {
    MetadataOnly,
    SimAdapter,
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

### ComfyHarnessLayer

Owns Comfy-derived world-model harness semantics, protocol adapters, and compatibility catalogs while delegating storage, media, tasks, secrets, UI, model serving, and dependency review to Sim systems.

```rust
pub trait ComfyHarnessLayer {
    fn validate_prompt(&self, prompt: &ComfyPromptGraph) -> ComfyValidationResult;
    fn route_status(&self, route: &ComfyRouteId) -> ComfyRouteSupport;
    fn node_capability(&self, node_id: &NodeTypeId) -> Option<ComfyNodeCapability>;
    fn execution_capability(&self, profile: &ModelFamilyId) -> Option<ComfyExecutionCapability>;
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

### Workspace Integration (sim.rs consumption)

The workspace integration is not a separate component with its own registrar. It is a function in `sim.rs` that calls Sim's native language registry directly and consumes pure-data descriptors for later task/preview hooks:

<!-- impl: crates/sim/src/sim.rs#register_game_integration -->
<!-- impl: crates/sim_game/src/integration.rs#simscript_language_config -->
<!-- impl: crates/sim_game/src/integration.rs#default_game_task_providers -->
<!-- impl: crates/sim_game/src/integration.rs#default_game_preview_routes -->

```rust
fn register_game_integration(app_state: &AppState, cx: &mut App) {
    let config = sim_game::simscript_language_config();
    let language = Language::new(LanguageConfig {
        name: LanguageName::new_static("SimScript"),
        matcher: LanguageMatcher { path_suffixes: config.extensions, ..Default::default() },
        line_comments: config.line_comment.map(|comment| vec![comment.into()]).unwrap_or_default(),
        ..Default::default()
    }, None);
    app_state.languages.add(Arc::new(language));

    for provider in sim_game::default_game_task_providers() {
        log::info!("game task provider registered: {}", provider.id);
    }

    for route in sim_game::default_game_preview_routes() {
        log::info!("game preview route registered: .{}", route.extension);
    }
}
```

## Data Models

```rust
pub enum ExecutionGate {
    SpecConsistency,
    BoundaryPolicy,
    SharedSimGameMetadata,
    SharedWorldModelFoundations,
    WorkerSafety,
    GraphSafety,
    Provenance,
    DependencyReview,
    ComfyHarnessAlignment,
}

pub enum DependencyWave {
    PlanningValidation,
    SharedFoundations,
    SimGameCompatibilitySubstrate,
    WorldModelAndComfyServingSubstrate,
    AuthoringGraphUxAndComfyWorkflows,
    GenerationOutputsAndAssetPipelines,
    ExternalExecutionHardening,
}
```

## Correctness Properties

### Property 1: No Runtime Duplication

_For any_ Godot subsystem that duplicates Sim platform, rendering, UI, input, physics, networking, audio, XR, or text infrastructure, the boundary policy SHALL classify it as excluded or external-command only.

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

### Property 6: Comfy Ownership Boundaries

_For any_ Comfy feature, if an existing Sim or game/world-model migration spec owns the underlying UI, task, media, asset, secret, model-serving, mesh, or dependency-review behavior, the Comfy harness layer SHALL delegate to that owner and SHALL NOT add a parallel subsystem.

**Validates: Requirement 2.1, 2.2, 13.2, 13.3**

### Property 7: Comfy Harness Semantics

_For any_ world-model harness implementation decision involving prompt jobs, graph orchestration, sampler/scheduler behavior, conditioning, diffusion/world-model execution, model resolution, assets, media nodes, provider calls, or extensions, the migration gatekeeper SHALL require a matching Comfy spec reference or an explicit safety, security, dependency, or platform divergence decision.

**Validates: Requirement 13.4, 13.5, 13.6**

### Property 8: Direct Registry Integration

_For any_ game feature that maps to an existing Sim capability (language support, task providers, preview routing), the integration SHALL use Sim's native registries directly rather than through an intermediate abstraction layer.

**Validates: Requirement 2.1**

## Error Handling

- Missing spec files produce blocking G0 errors.
- Runtime-adjacent work without boundary policy produces blocking G1 errors.
- Worker execution without diagnostics produces blocking G4 errors.
- Graph execution without validation produces blocking G5 errors.
- Generated artifact import without provenance produces blocking G6 errors.
- Heavy/native dependencies without review produce blocking G7 errors.
- Comfy features without an owning spec or explicit delegation produce blocking G0 errors.
- World-model harness changes that bypass applicable Comfy specs produce blocking G8 errors.
