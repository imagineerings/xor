# Design: Zed Game Development Surface

## Architecture

The game development surface is organized as Zed-native integration layers:

- `crates/world_model`: world-model request/control/worker/graph/mesh/artifact/provenance primitives.
- Comfy harness modules inside `crates/world_model`: core runtime control-plane adapters, Comfy graph/node schemas, sampler/scheduler execution semantics, conditioning, latent/VAE behavior, model patching, diffusion/world-model runner profiles, model folder and memory policy, asset APIs, workflow/blueprint catalogs, provider connectors, extension loading policy, and compatibility fixtures.
- Existing Zed crates: project, worktree, language, LSP, DAP, tasks, debugger, media, UI, agent, app registry, settings, persistence, networking, extensions, rendering, diagnostics, and platform crates own Godot-origin behavior in their existing domains.

### Design Principle: Native Integration

Every supported Godot-originated feature is modeled, stored, presented, executed, canceled, recovered, and retired through a named Zed owner. Godot is a behavioral and compatibility reference only: no compatibility shim, hidden instance, process invocation, runtime linkage, external Godot task, registrar layer, or parallel Godot-specific subsystem may establish support. SimScript uses the existing `LanguageRegistry`; legacy `.gd` files and Godot-format assets are compatibility inputs whose successful outputs are Zed-native. Capabilities without an approved native owner remain unresolved or intentionally excluded.

Every Comfy-derived world-model harness capability follows the same native-integration rule. A supported Comfy workflow, graph, node, sampler, scheduler, conditioning, latent, VAE, model patch, diffusion/world-model runner, model, asset, provider, extension, API route, or packaging/quality feature must be implemented through Zed-owned services, typed records, worker boundaries, artifact/provenance models, and diagnostics. Compatibility data may define expected behavior and fixtures, but support is not satisfied by a label, a hidden ComfyUI proxy, or a pass-through to ComfyUI.

### Design Principle: Value-First Sequencing

The migration gatekeeper ranks available work by target-product value after gates and write conflicts are checked. W2-W4 Comfy/world-model harness tasks should be selected before W7 Godot-origin compatibility tasks. W5 native authoring and agentic tools should consume the harness instead of building around missing execution paths. W6 provider, extension, and packaging hardening remains available when it blocks worker safety, provenance, policy, or dependency review; otherwise it follows the local harness core.

## Components

### Coverage catalog and validator

The specification-owned catalog and validators—not a product crate—record grouped coverage, native owners, compatibility boundaries, unresolved/excluded behavior, and no-Godot evidence. Product code uses existing Zed owners directly.

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

### SimHarnessLayer

Owns Comfy-derived world-model harness semantics, protocol adapters, and compatibility catalogs while delegating storage, media, tasks, secrets, UI, model serving, and dependency review to Zed systems.

```rust
pub trait SimHarnessLayer {
    fn validate_prompt(&self, prompt: &SimPromptGraph) -> SimValidationResult;
    fn route_status(&self, route: &SimRouteId) -> SimRouteSupport;
    fn node_capability(&self, node_id: &NodeTypeId) -> Option<SimNodeCapability>;
    fn execution_capability(&self, profile: &ModelFamilyId) -> Option<SimExecutionCapability>;
}
```

### Specification gatekeeper

Feature-spec validation, the master coverage catalog, and audit scripts block implementation work when traceability, native ownership, dependency waves, or execution gates are unsatisfied. This governance does not introduce a runtime registry or product abstraction.

### Workspace integration

Project detection extends `project`/`worktree`; language support extends `languages`/`language`; tasks extend `task`; previews extend existing media/image/component preview owners; commands and UI extend workspace/editor registries. No intermediate game/Godot registrar owns duplicate state.

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
    SimHarnessAlignment,
}

pub enum DependencyWave {
    PlanningValidation,
    SharedFoundations,
    ValueFirstWorldModelServingSubstrate,
    SimExecutionCore,
    GenerationOutputsAndAssetPipelines,
    ProductAuthoringAndAgenticTools,
    SimProviderExtensionAndPackagingHardening,
    DeferredGodotOriginCompatibility,
}
```

## Correctness Properties

### Property 1: No Runtime Duplication

_For any_ Godot subsystem that duplicates Zed platform, rendering, UI, input, physics, networking, audio, XR, or text infrastructure, the design SHALL extend the existing owner or classify the capability unresolved/intentionally excluded; it SHALL NOT invoke or wrap Godot.

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

_For any_ Comfy feature, if an existing Zed or game/world-model migration spec owns the underlying UI, task, media, asset, secret, model-serving, mesh, or dependency-review behavior, the Comfy harness layer SHALL delegate to that owner and SHALL NOT add a parallel subsystem.

**Validates: Requirement 2.1, 2.2, 13.2, 13.3**

### Property 7: Comfy Harness Semantics

_For any_ world-model harness implementation decision involving prompt jobs, graph orchestration, sampler/scheduler behavior, conditioning, diffusion/world-model execution, model resolution, assets, media nodes, provider calls, or extensions, the migration gatekeeper SHALL require a matching Comfy spec reference or an explicit safety, security, dependency, or platform divergence decision.

**Validates: Requirement 13.4, 13.5, 13.6**

### Property 7A: Native Comfy Recreation

_For any_ Comfy-derived endpoint, node, workflow, model, asset, provider, extension, media operation, or execution behavior, the implementation SHALL provide a native Zed feature backed by Zed services and data models rather than a thin compatibility label, hidden ComfyUI pass-through, or unsupported placeholder.

**Validates: Requirement 13.7**

### Property 8: Direct Registry Integration

_For any_ game feature that maps to an existing Zed capability (language support, task providers, preview routing), the integration SHALL use Zed's native registries directly rather than through an intermediate abstraction layer.

**Validates: Requirement 2.1**

### Property 9: Value-First Task Selection

_For any_ available post-W1 task set that includes both Comfy/world-model harness work and W7 deferred Godot-origin compatibility work, task selection SHALL rank the Comfy/world-model work first unless the W7 task records an explicit product-enabling dependency.

**Validates: Requirement 12.5, 14.1, 14.2, 14.4**

### Property 10: Native Authoring Priority

_For any_ SimScript or natural-language authoring task, the task SHALL model natural language as the primary authoring interface and SimScript as the executable language, while treating legacy `.gd` and Godot-format support as import/migration inputs.

**Validates: Requirement 14.3**

### Property 11: Native Zed ownership without Godot

_For any_ supported or fully specified Godot-origin capability, the catalog and owner spec SHALL name the Zed-native storage, execution, UI, lifecycle, and compatibility paths and SHALL prove operation with Godot absent from the machine, package, process tree, loader, and dependency graph.

**Validates: Requirement 15.1, 15.2, 15.3, 15.4, 15.5, 15.6, 15.7, 15.8, 15.9, 15.10**

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
- Any Godot process, library, server, command, hidden instance, wrapper, runtime linkage, external delegation, unreviewed source copy, or duplicate Godot-specific owner produces a blocking G11 native-ownership error.


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 4.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 4.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 4.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 5.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 5.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 5.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 5.4 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 6.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 6.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 6.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 6.4 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 7.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 7.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 7.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 8.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 8.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 8.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 9.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 9.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 9.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 10.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 10.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 10.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 11.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 11.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 11.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 12.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 12.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 12.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 12.4 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 12.5 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.4 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.5 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.6 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.7 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 13.8 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 14.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 14.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 14.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 14.4 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 15.1 | Property 11 and owner-spec D-NATIVE elements | Native owner and Zed storage/execution/UI/lifecycle review |
| 15.2 | Property 11 and owner-spec D-NATIVE elements | Hermetic process, package, loader, and dependency inspection |
| 15.3 | Property 11 and owner-spec D-NATIVE elements | Compatibility-boundary and Zed-native output validation |
| 15.4 | Property 11 and owner-spec D-NATIVE elements | Existing-owner reuse and duplicate-abstraction scan |
| 15.5 | Property 11 and owner-spec D-NATIVE elements | Import success/failure/cancellation/recovery without Godot |
| 15.6 | Property 11 and owner-spec D-NATIVE elements | Exported-artifact execution on a no-Godot machine image |
| 15.7 | Property 11 and owner-spec D-NATIVE elements | Unsupported/unresolved/decision classification gate |
| 15.8 | Property 11 and owner-spec D-NATIVE elements | Exact source-copy licensing and architecture review gate |
| 15.9 | Property 11 and owner-spec D-NATIVE elements | Class 1/3 acceptance and connected implementation evidence gate |
| 15.10 | Property 11 and owner-spec D-NATIVE elements | Violation, duplicate, placeholder, dependency, and decision reports |
