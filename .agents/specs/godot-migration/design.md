# Design: Sim Game Development Surface

## Architecture

The game development surface is organized as Sim-native integration layers:

- `crates/sim_game`: Sim-owned game authoring metadata, boundary policy, parsing diagnostics, project descriptors, fixture attribution, and task/debug/export descriptors for Godot-format source projects.
- `crates/world_model`: world-model request/control/worker/graph/mesh/artifact/provenance primitives.
- Comfy harness modules inside `crates/world_model`: core runtime control-plane adapters, Comfy graph/node schemas, sampler/scheduler execution semantics, conditioning, latent/VAE behavior, model patching, diffusion/world-model runner profiles, model folder and memory policy, asset APIs, workflow/blueprint catalogs, provider connectors, extension loading policy, and compatibility fixtures.
- Existing Sim crates: project, worktree, language, LSP, tasks, debugger, media, UI, agent, and app registry own their existing domains.

### Design Principle: Native Integration

Every Godot-originated feature is modeled as a native Sim equivalent. There is no compatibility shim layer, no "registrar trait" bridging game features to Sim registries, and no parallel language config type. SimScript is registered as the first-class executable game language via `LanguageRegistry::add` with the same `Language` type used for Rust, Python, and TypeScript; natural language is the authoring interface that produces inspectable SimScript. Legacy `.gd` files and Godot-format assets are import sources rather than the primary product path. `sim_game` exports pure-data helpers for external game task templates and game asset preview routes so follow-on task and preview sub-specs can wire them into the native task source and preview action systems without introducing an intermediate compatibility layer.

Every Comfy-derived world-model harness capability follows the same native-integration rule. A supported Comfy workflow, graph, node, sampler, scheduler, conditioning, latent, VAE, model patch, diffusion/world-model runner, model, asset, provider, extension, API route, or packaging/quality feature must be implemented through Sim-owned services, typed records, worker boundaries, artifact/provenance models, and diagnostics. Compatibility data may define expected behavior and fixtures, but support is not satisfied by a label, a hidden ComfyUI proxy, or a pass-through to ComfyUI.

### Design Principle: Value-First Sequencing

The migration gatekeeper ranks available work by target-product value after gates and write conflicts are checked. W2-W4 Comfy/world-model harness tasks should be selected before W7 Godot-origin compatibility tasks. W5 native authoring and agentic tools should consume the harness instead of building around missing execution paths. W6 provider, extension, and packaging hardening remains available when it blocks worker safety, provenance, policy, or dependency review; otherwise it follows the local harness core.

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
    ValueFirstWorldModelServingSubstrate,
    ComfyExecutionCore,
    GenerationOutputsAndAssetPipelines,
    ProductAuthoringAndAgenticTools,
    ComfyProviderExtensionAndPackagingHardening,
    DeferredGodotOriginCompatibility,
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

### Property 7A: Native Comfy Recreation

_For any_ Comfy-derived endpoint, node, workflow, model, asset, provider, extension, media operation, or execution behavior, the implementation SHALL provide a native Sim feature backed by Sim services and data models rather than a thin compatibility label, hidden ComfyUI pass-through, or unsupported placeholder.

**Validates: Requirement 13.7**

### Property 8: Direct Registry Integration

_For any_ game feature that maps to an existing Sim capability (language support, task providers, preview routing), the integration SHALL use Sim's native registries directly rather than through an intermediate abstraction layer.

**Validates: Requirement 2.1**

### Property 9: Value-First Task Selection

_For any_ available post-W1 task set that includes both Comfy/world-model harness work and W7 deferred Godot-origin compatibility work, task selection SHALL rank the Comfy/world-model work first unless the W7 task records an explicit product-enabling dependency.

**Validates: Requirement 12.5, 14.1, 14.2, 14.4**

### Property 10: Native Authoring Priority

_For any_ SimScript or natural-language authoring task, the task SHALL model natural language as the primary authoring interface and SimScript as the executable language, while treating legacy `.gd` and Godot-format support as import/migration inputs.

**Validates: Requirement 14.3**

## Error Handling

- Missing spec files produce blocking G0 errors.
- Runtime-adjacent work without boundary policy produces blocking G1 errors.
- Worker execution without diagnostics produces blocking G4 errors.
- Graph execution without validation produces blocking G5 errors.
- Generated artifact import without provenance produces blocking G6 errors.
- Heavy/native dependencies without review produce blocking G7 errors.
- Comfy features without an owning spec or explicit delegation produce blocking G0 errors.
- World-model harness changes that bypass applicable Comfy specs produce blocking G8 errors.
- W7 deferred Godot-origin tasks selected ahead of available W2-W6 product work without an explicit product-enabling dependency produce priority-policy errors.
