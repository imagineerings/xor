# Requirements: Comfy Full Port Coverage

## Introduction

Sim already has focused Comfy migration specs for runtime APIs, graph execution, model and memory policy, diffusion/world-model semantics, assets, workflows, media nodes, provider nodes, extensions, and packaging quality. Those specs intentionally avoid duplicating Sim infrastructure, but the actual `projects/comfy` tree is broad enough that Sim also needs a coverage layer: a source-to-Sim ownership map that proves every Comfy feature area is either implemented by an existing Sim capability, owned by exactly one migration spec, explicitly delegated to another Sim subsystem, or intentionally rejected with a boundary reason.

This spec does not create a parallel Comfy runtime. It creates the completeness and anti-duplication gates required to port all Comfy features and functionality into native Sim systems without hidden ComfyUI pass-through, duplicate runtimes, or unowned feature gaps.

## Glossary

- **Sim Source Inventory**: A native Sim structured snapshot of feature surfaces in `/Users/ahmad.vegah/repos/projects/sim/projects/comfy`, including server routes, nodes, model families, blueprints, assets, extensions, API providers, CLI/config, tests, and packaging hooks.
- **Coverage Owner**: The single Sim spec, crate, module, or existing subsystem that owns a Comfy source feature area.
- **Coverage Ledger**: A machine-checkable record mapping each Comfy source feature area to its coverage owner and implementation status.
- **Duplicate Port**: Any implementation that recreates behavior already owned by an existing Sim crate/spec without documenting delegation or extension.
- **Native Sim Recreation**: Comfy-derived behavior implemented with Sim records, services, workers, artifacts, registries, routes, diagnostics, and tests, not by running or proxying ComfyUI.
- **Parity Fixture**: A test fixture derived from Comfy source behavior that validates Sim compatibility while preserving source attribution and avoiding unsupported downloads.

## Requirements

### Requirement 1: Complete Sim Source Inventory

**User Story:** As a migration owner, I want a complete inventory of Comfy source functionality so the port does not miss major feature areas.

#### Acceptance Criteria

1.1 WHEN the Sim source inventory is generated from the Comfy checkout THEN THE system SHALL include server/API routes, WebSocket protocol behavior, prompt/job lifecycle, execution/caching/progress modules, core node mappings, extra node modules, API provider node modules, model families, model folder categories, asset APIs, workflow/blueprint files, extension/custom-node hooks, CLI/config flags, OpenAPI operations, tests, and packaging/CI surfaces.
1.2 WHEN the Sim source inventory records node functionality from the Comfy checkout THEN THE system SHALL distinguish core nodes, built-in extra nodes, V3 `ComfyExtension` nodes, API-provider nodes, and custom-node examples.
1.3 WHEN the Sim source inventory records workflows from the Comfy checkout THEN THE system SHALL include every shipped blueprint JSON and any global-subgraph/template source.
1.4 WHEN a Comfy source file cannot be parsed or classified THEN THE system SHALL preserve the file path with an `unclassified` status and a diagnostic.

### Requirement 2: Single Ownership and No Duplication

**User Story:** As a maintainer, I want every Comfy-derived feature to have one owner so Sim does not grow duplicate subsystems.

#### Acceptance Criteria

2.1 WHEN a Comfy source feature is mapped THEN THE system SHALL assign exactly one coverage owner.
2.2 IF a Comfy feature overlaps an existing Sim subsystem THEN THE coverage ledger SHALL mark the existing subsystem as the owner and list the migration spec as an adapter or test owner only.
2.3 IF two specs claim the same Comfy feature as implementation owner THEN THE system SHALL fail the coverage gate with both spec paths.
2.4 WHEN a new Comfy-derived implementation is proposed THEN THE system SHALL require the task to reference the coverage owner before code work starts.

### Requirement 3: Native Sim Recreation Boundary

**User Story:** As a product owner, I want Comfy functionality recreated inside Sim without depending on ComfyUI as a hidden runtime.

#### Acceptance Criteria

3.1 WHEN a supported Comfy behavior is implemented THEN THE system SHALL implement it through native Sim types, routes, services, workers, artifacts, and diagnostics.
3.2 IF a behavior remains unsupported THEN THE coverage ledger SHALL include a user-visible unsupported reason and the owning spec responsible for any future support.
3.3 IF a safety, security, dependency, licensing, or platform gate prevents exact parity THEN THE system SHALL record the divergence and the fallback behavior.
3.4 WHEN Sim implementation names represent Sim-owned runtime state THEN THE system SHALL use `Sim*` naming and reserve `Comfy*` names for source references and compatibility formats.

### Requirement 4: Existing Spec Reuse

**User Story:** As an implementer, I want the coverage layer to reuse the existing Comfy specs so this work coordinates rather than duplicates planning.

#### Acceptance Criteria

4.1 WHEN mapping runtime routes, prompt submission, queue/history, jobs, WebSocket events, HTTP safety, upload/view, or preview streaming THEN THE coverage ledger SHALL delegate to `comfy-runtime-control-plane`.
4.2 WHEN mapping node schemas, object-info, prompt graph validation, node replacement, execution planning, caching, node dispatch, async/list execution, or core node fixtures THEN THE coverage ledger SHALL delegate to `comfy-graph-node-runtime`.
4.3 WHEN mapping model folders, model cataloging, safetensors metadata, preview resolution, model family detection, precision, quantization, device, memory policy, or model resource release THEN THE coverage ledger SHALL delegate to `comfy-model-memory-runtime`.
4.4 WHEN mapping samplers, schedulers, guiders, conditioning, latent formats, VAE behavior, model components, patches, LoRA/hypernetworks, diffusion execution, world-model runners, or worker execution THEN THE coverage ledger SHALL delegate to `comfy-diffusion-world-model-runtime`.
4.5 WHEN mapping asset CRUD, uploads, downloads, tags, metadata filters, user data, settings, scans, pruning, output registration, or enrichment THEN THE coverage ledger SHALL delegate to `comfy-asset-library`.
4.6 WHEN mapping blueprints, workflow save/load/export, embedded metadata, app-mode metadata, global subgraphs, templates, or node replacements THEN THE coverage ledger SHALL delegate to `comfy-workflows-blueprints`.
4.7 WHEN mapping image, mask, video, audio, 3D, Gaussian splat, geometry, detection, segmentation, control, utility, dataset, shader, or post-processing nodes THEN THE coverage ledger SHALL delegate to `comfy-media-node-pipelines` unless a model-execution spec is the sole owner.
4.8 WHEN mapping external provider nodes, provider secrets, remote task lifecycle, provider uploads/downloads, cost policy, quota policy, or output import THEN THE coverage ledger SHALL delegate to `comfy-api-provider-nodes`.
4.9 WHEN mapping custom node discovery, V1/V3 node registration, extension web directories, translations, extension workflow templates, extension subgraphs, startup scripts, or manager compatibility THEN THE coverage ledger SHALL delegate to `comfy-extension-ecosystem`.
4.10 WHEN mapping CLI launch flags, feature flags, OpenAPI operations, API examples, frontend package/version diagnostics, tests, dependency review, logging, CI, or packaging profiles THEN THE coverage ledger SHALL delegate to `comfy-packaging-quality`.

### Requirement 5: Capability Gap Detection

**User Story:** As a reviewer, I want automated gap detection so newly discovered Comfy functionality cannot silently fall through the port.

#### Acceptance Criteria

5.1 WHEN the coverage gate runs THEN THE system SHALL fail if any inventory item lacks an owner, status, source path, or boundary decision.
5.2 WHEN the Sim source inventory changes THEN THE system SHALL identify added, removed, and reclassified feature areas.
5.3 IF a new source item is added under `comfy_extras`, `comfy_api_nodes`, `comfy`, `comfy_execution`, `app`, `api_server`, `blueprints`, `custom_nodes`, or `server.py` THEN THE system SHALL require a coverage-ledger update.
5.4 WHEN a coverage item is marked implemented THEN THE system SHALL require at least one test, fixture, or explicit existing-Sim equivalence reference.

### Requirement 6: Source Attribution and Fixture Safety

**User Story:** As a maintainer, I want parity fixtures to preserve upstream provenance and avoid unsafe setup.

#### Acceptance Criteria

6.1 WHEN a parity fixture is generated from Comfy source behavior THEN THE fixture SHALL include source path, source category, capture date, and implementation owner.
6.2 IF a parity fixture would require model downloads, API keys, paid provider calls, native codecs, or heavyweight dependencies THEN THE fixture SHALL use metadata-only or mock-runner records unless dependency review approves otherwise.
6.3 WHEN a fixture represents implemented Sim behavior THEN THE fixture SHALL mark `native_sim_records` true and `comfyui_passthrough` false.
6.4 WHEN a fixture represents an unsupported or divergent behavior THEN THE fixture SHALL include the user-visible diagnostic code or boundary reason.

### Requirement 7: Implementation Task Gating

**User Story:** As an engineer, I want implementation tasks to consult coverage before changing code so duplicate work is caught early.

#### Acceptance Criteria

7.1 WHEN a task touches Comfy-derived behavior THEN THE start gate SHALL verify the coverage owner, dependency wave, and expected writes before implementation begins.
7.2 IF the expected writes overlap an existing owner spec or crate THEN THE task SHALL document whether it extends that owner or conflicts with it.
7.3 WHEN a task completes THEN THE completion gate SHALL update the coverage ledger status and validate that requirements, design properties, tasks, fixtures, and implementation still agree.
7.4 IF implementation discovers a new Comfy behavior not present in the inventory THEN THE task SHALL add the inventory item before marking completion.

### Requirement 8: Product Sequencing

**User Story:** As a product owner, I want full Comfy parity work ordered so local world-model authoring value lands before lower-value compatibility.

#### Acceptance Criteria

8.1 WHEN uncovered Comfy functionality is prioritized THEN THE system SHALL rank local prompt/job, graph/node, model/runtime, diffusion/world-model, asset, workflow, and media-node capabilities ahead of provider, extension, packaging, or legacy compatibility hardening unless a policy gate blocks local work.
8.2 IF a provider, extension, or packaging gap is selected before local execution parity gaps THEN THE task SHALL document the worker-safety, dependency-review, provenance, or product-blocking reason.
8.3 WHEN a Comfy-derived feature overlaps Godot-origin or world-model-origin work THEN THE system SHALL choose the owner that advances native Sim world-model authoring with the least duplicate infrastructure.

### Requirement 9: Missing Functionality Port Backlog

**User Story:** As an implementation lead, I want every missing Comfy feature to become an owner-specific port task so the generated specs lead to actual native Sim functionality.

#### Acceptance Criteria

9.1 WHEN the coverage ledger identifies a Comfy source feature as planned, unsupported-but-product-approved, divergent-with-fallback, or missing evidence THEN THE system SHALL create or update an implementation task in the owning spec for that source feature.
9.2 WHEN an owner-specific implementation task is generated THEN THE task SHALL include the coverage IDs, native Sim module or subsystem owner, expected writes, validation command, and parity evidence required to mark the feature implemented.
9.3 IF the missing functionality needs a new foundation before feature parity can be implemented THEN THE owning spec SHALL add the foundation task before the feature task and keep both tasks under the same coverage owner.
9.4 WHEN an owner-specific implementation task completes THEN THE system SHALL update the coverage ledger status and evidence rather than leaving the missing functionality tracked only in prose.
