# Requirements: Sim Game Development Surface

## Introduction

Sim needs a native game development surface for building 2D and 3D games, with Godot-format project and asset compatibility, world-model foundation harness support, and Comfy-aware workflow orchestration — all without copying duplicate game-engine runtime infrastructure. The migration adds project detection, authoring affordances, language support, generation, and serving primitives while preserving Sim ownership of UI, platform, rendering, task execution, agents, media, storage, and project systems. Comfy provides core functionality for the world-model harness, so implementation decisions must evaluate Comfy workflow, graph, sampler, scheduler, conditioning, diffusion/world-model execution, node, model, asset, provider, and extension semantics before adding Sim-only behavior.

## Glossary

- **Boundary policy**: the explicit rule set for what Sim adopts as native features, adapts, invokes externally, or refuses to duplicate.
- **Execution gate**: a prerequisite validation checkpoint that blocks task execution until satisfied.
- **Dependency wave**: the ordered implementation phase used to keep shared foundations ahead of dependent integrations.
- **World-model engine harness**: Sim-managed typed requests, controls, workers, artifacts, and provenance around `projects/world-model`.
- **Comfy world-model harness substrate**: Sim-managed core protocol, graph, sampler, scheduler, conditioning, diffusion/world-model execution, node, model, asset, provider, blueprint, extension, and packaging behavior derived from `projects/comfy`.
- **Deferred Godot-origin compatibility**: Godot-format, runtime, editor, export, XR, physics, networking, and legacy language work that is modeled as native Sim functionality but selected later unless it directly supports the target Comfy/world-model game-development product.

### Requirement 1: Complete Inventory

**User Story:** As a developer, I want all game, world-model, and Comfy feature areas inventoried so migration work does not miss major functionality.

#### Acceptance Criteria

1.1 WHEN the migration inventory is reviewed THEN THE system SHALL list every grouped migration spec and its scope.
1.2 WHEN a grouped spec is added THEN THE system SHALL include requirements, design, and tasks documents.
1.3 WHEN a source area is intentionally excluded THEN THE system SHALL document the boundary reason.

### Requirement 2: Duplication Avoidance

**User Story:** As a maintainer, I want Sim to reuse existing infrastructure so the migration does not fork duplicate runtimes.

#### Acceptance Criteria

2.1 IF Sim already owns a platform, rendering, UI, media, task, project, agent, or language capability THEN THE migration SHALL reuse that capability.
2.2 IF a Godot runtime subsystem duplicates Sim runtime architecture THEN THE migration SHALL mark it as excluded or external-command only.
2.3 WHEN a new crate is proposed THEN THE migration SHALL justify why existing crates cannot hold the behavior.

### Requirement 3: Game Project Support

**User Story:** As a game developer, I want Sim to understand Godot-format projects so I can inspect and edit existing assets.

#### Acceptance Criteria

3.1 WHEN a workspace contains `project.godot` THEN THE system SHALL detect it as a Godot-format game project.
3.2 WHEN SimScript, legacy `.gd`, scene, resource, shader, or asset files are opened THEN THE system SHALL provide metadata, diagnostics, and preview routing where supported.
3.3 IF runtime execution is required THEN THE system SHALL use explicit external task/debug/export integration instead of embedding Godot runtime systems.

### Requirement 4: Unified Game Authoring Product

**User Story:** As a game creator, I want one interface for 2D/3D game authoring, asset generation, graph pipelines, and world-model preview.

#### Acceptance Criteria

4.1 WHEN the authoring app opens THEN THE system SHALL present project assets, generated artifacts, graph pipelines, and runtime preview entries through one workspace model.
4.2 WHEN generated videos, meshes, textures, or controls are produced THEN THE system SHALL attach them to project-visible artifacts.
4.3 IF an artifact cannot be previewed natively THEN THE system SHALL show an actionable unsupported-preview reason.

### Requirement 5: World Model Runtime Harness

**User Story:** As a game creator, I want world foundation models to drive interactive game-world generation from Sim.

#### Acceptance Criteria

5.1 WHEN a world generation request is created THEN THE system SHALL capture prompt, source image, camera/action controls, model profile, seed, and output target.
5.2 WHEN action controls are provided THEN THE system SHALL validate WASD/IJKL semantics and frame-count padding rules.
5.3 WHEN a model worker is unavailable THEN THE system SHALL report environment, checkpoint, GPU, or remote-worker diagnostics.
5.4 WHEN output is imported THEN THE system SHALL preserve generation provenance.

### Requirement 6: Diffusion Graph Authoring

**User Story:** As a technical artist, I want to design diffusion pipelines with nodes and edges so generation flows are inspectable and reusable.

#### Acceptance Criteria

6.1 WHEN a graph is edited THEN THE system SHALL validate node types, ports, dependencies, and cycles.
6.2 WHEN an agent edits a graph THEN THE system SHALL apply the same validation used by the UI.
6.3 IF graph execution would use an unavailable backend THEN THE system SHALL block execution with diagnostics.

### Requirement 7: Textured 3D Mesh Generation

**User Story:** As a game creator, I want to generate textured 3D meshes with topology, detail, and textures suitable for game assets.

#### Acceptance Criteria

7.1 WHEN a mesh request is submitted THEN THE system SHALL capture prompt/reference inputs, target format, texture options, and backend.
7.2 WHEN a mesh artifact is produced THEN THE system SHALL register preview, export, provenance, and source-asset metadata.
7.3 IF a mesh backend requires a new native or heavy dependency THEN THE system SHALL require dependency review before implementation.

### Requirement 8: Agentic Game Tools

**User Story:** As a creator, I want agents to create and update game assets, pipeline graphs, and world-model requests safely.

#### Acceptance Criteria

8.1 WHEN an agent proposes graph edits THEN THE system SHALL validate and diff the graph before applying changes.
8.2 WHEN an agent starts generation THEN THE system SHALL use typed generation requests and worker diagnostics.
8.3 WHEN an agent imports generated outputs THEN THE system SHALL attach provenance.

### Requirement 9: Model Serving and Packaging

**User Story:** As a developer, I want local and remote model execution to be diagnosable and packageable.

#### Acceptance Criteria

9.1 WHEN local serving is configured THEN THE system SHALL validate Python, package, checkpoint, GPU, and disk prerequisites.
9.2 WHEN remote serving is configured THEN THE system SHALL validate endpoint, authentication, capability, and quota metadata.
9.3 IF setup requires downloads THEN THE system SHALL not silently download large assets.

### Requirement 10: Validation

**User Story:** As a maintainer, I want every migration task tied to requirements and writes so implementation can be reviewed safely.

#### Acceptance Criteria

10.1 WHEN tasks are reviewed THEN THE system SHALL expose requirement references for every task.
10.2 WHEN tasks are reviewed THEN THE system SHALL expose expected write targets for every task.
10.3 WHEN implementation starts THEN THE system SHALL check dependency gates and wave ordering.

### Requirement 11: Third-Party and License Control

**User Story:** As a maintainer, I want third-party code and assets controlled before dependencies enter Sim.

#### Acceptance Criteria

11.1 IF a task adds vendored Godot code, model code, codecs, mesh backends, or native dependencies THEN THE system SHALL require dependency review.
11.2 WHEN fixtures are copied or converted THEN THE system SHALL preserve source attribution.
11.3 WHEN dependencies are rejected THEN THE system SHALL document the fallback strategy.

### Requirement 12: Execution Gates and Dependency Waves

**User Story:** As an implementer, I want explicit gates and dependency waves so dependent work does not start before shared foundations exist.

#### Acceptance Criteria

12.1 WHEN a task is selected THEN THE system SHALL identify its applicable execution gates.
12.2 WHEN a task is selected THEN THE system SHALL identify its dependency wave.
12.3 IF required gates are unsatisfied THEN THE system SHALL block the task.
12.4 WHEN the spec pack is validated THEN THE system SHALL confirm every grouped spec has requirements, design, and tasks files.
12.5 WHEN tasks are selected after shared foundations THEN THE system SHALL apply the value-first sequencing policy before dependency-wave ordering alone.

### Requirement 13: Comfy Workflow Orchestration Migration

**User Story:** As a workflow creator, I want Comfy features represented in Sim specs so visual AI workflows, assets, and provider nodes can migrate without duplicate infrastructure.

#### Acceptance Criteria

13.1 WHEN Comfy migration specs are reviewed THEN THE system SHALL include non-overlapping specs for runtime APIs, graph/node runtime, model/memory runtime, diffusion/world-model runtime, assets, workflows/blueprints, media node pipelines, provider API nodes, extension ecosystem, and packaging/quality.
13.2 IF a Comfy feature overlaps an existing game/world-model migration spec THEN THE Comfy spec SHALL name the owning spec and delegate that behavior.
13.3 WHEN Comfy endpoints, nodes, assets, providers, or extensions are implemented THEN THE system SHALL use Sim task, media, artifact, secret, storage, diagnostic, and dependency-review infrastructure.
13.4 WHEN a world-model harness implementation decision involves graph orchestration, prompt/job lifecycle, model resolution, sampler/scheduler behavior, conditioning, diffusion/world-model execution, asset handling, media nodes, provider calls, or extension loading THEN THE system SHALL consult the applicable Comfy spec before introducing Sim-only behavior.
13.5 IF Comfy semantics conflict with existing Sim infrastructure THEN THE system SHALL document the decision and preserve Comfy workflow compatibility unless safety, security, dependency, or platform gates require divergence.
13.6 WHEN local diffusion or world-model execution is implemented THEN THE system SHALL preserve Comfy sampler, scheduler, conditioning, latent, VAE, model patch, guidance, and model-family execution semantics unless a documented gate requires divergence.

### Requirement 14: Value-First Product Sequencing

**User Story:** As a product owner, I want Comfy and world-model harness work completed before lower-value Godot-origin compatibility work so each migration task advances the target native Sim product.

#### Acceptance Criteria

14.1 WHEN migration tasks are ranked after W1 shared foundations THEN THE system SHALL prioritize W2-W4 Comfy and world-model harness tasks ahead of W7 Godot-format, runtime, editor, export, XR, physics, networking, and legacy language tasks.
14.2 IF a W7 deferred Godot-origin task is selected before available W2-W6 product work is complete THEN THE system SHALL document the product-enabling dependency that justifies the exception.
14.3 WHEN SimScript or natural-language authoring work is selected THEN THE system SHALL treat natural language as the primary authoring interface and SimScript as the executable game language, while legacy `.gd` and Godot-format support remain migration/import sources.
14.4 WHEN Comfy provider, extension, or packaging hardening is ranked THEN THE system SHALL keep it behind local W2-W4 harness functionality unless the hardening task blocks worker safety, provenance, policy, or dependency-review gates.
