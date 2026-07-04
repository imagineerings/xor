# Requirements: Comfy Workflows and Blueprints

## Introduction

Baymax needs to migrate Comfy workflow files, blueprint templates, global subgraphs, node replacement metadata, and workflow metadata embedded in generated files. These workflows are first-class world-model harness resources, not secondary import artifacts. This spec owns workflow/template lifecycle and compatibility. It delegates graph validation to `comfy-graph-node-runtime/`, authoring UI to `unified-authoring-app/` and `diffusion-graph-editor/`, and assets to `comfy-asset-library/`.

## Glossary

- **Workflow**: A saved Comfy graph with UI metadata and optional API prompt export form.
- **Blueprint**: A shipped workflow template under `projects/comfy/blueprints`, often named for a user-facing capability.
- **Subgraph**: A reusable graph fragment exposed from blueprints or custom node packs.
- **Node Replacement**: Metadata for replacing stale node ids with new node ids and input/output mappings.
- **Embedded Workflow Metadata**: Prompt or workflow JSON stored inside generated PNG, WebP, FLAC, or other supported output metadata.

## Requirements

### Requirement 1: Blueprint Inventory and Import

**User Story:** As a creator, I want shipped Comfy blueprints available in Baymax so common generation workflows are discoverable.

#### Acceptance Criteria

1.1 WHEN Comfy migration blueprints are imported THEN THE system SHALL preserve all shipped blueprint names, source paths, categories, graph JSON, and source attribution.
1.2 WHEN a blueprint references GLSL shader files or helper assets THEN THE system SHALL register those dependencies with the blueprint.
1.3 IF a blueprint uses unsupported nodes or models THEN THE system SHALL show missing capability diagnostics without dropping the blueprint.

### Requirement 2: Workflow Save, Load, and API Export

**User Story:** As a workflow author, I want Baymax to load, save, and export Comfy workflows for automation.

#### Acceptance Criteria

2.1 WHEN a workflow JSON is opened THEN THE system SHALL parse graph nodes, links, UI metadata, workflow id, version, and default view where present.
2.2 WHEN a workflow is saved THEN THE system SHALL preserve graph structure, UI metadata, source references, and Baymax provenance fields.
2.3 WHEN API export is requested THEN THE system SHALL emit the Comfy API prompt form accepted by the runtime control plane.

### Requirement 3: Global Subgraphs and Templates

**User Story:** As a user, I want reusable workflow fragments from blueprints and extensions.

#### Acceptance Criteria

3.1 WHEN global subgraphs are listed THEN THE system SHALL include blueprint subgraphs and custom-node subgraphs with stable ids, names, source type, and node pack metadata.
3.2 WHEN a subgraph is opened THEN THE system SHALL return its graph data and sanitized metadata.
3.3 WHEN custom node templates exist THEN THE system SHALL expose template names and static template assets through Baymax's template service.

### Requirement 4: Node Replacement Compatibility

**User Story:** As a workflow author, I want older workflows to remain usable when node ids change.

#### Acceptance Criteria

4.1 WHEN a workflow references a missing node with a registered replacement THEN THE system SHALL apply the replacement before validation.
4.2 WHEN replacement mappings change inputs or outputs THEN THE system SHALL preserve compatible values and linked output indexes.
4.3 IF no valid replacement exists THEN THE system SHALL keep the original node reference and show a missing-node diagnostic.

### Requirement 5: Embedded Metadata Import

**User Story:** As a user, I want generated outputs to carry recoverable workflow metadata.

#### Acceptance Criteria

5.1 WHEN a generated file with embedded prompt or workflow metadata is imported THEN THE system SHALL extract workflow metadata when the format is supported.
5.2 WHEN metadata extraction succeeds THEN THE system SHALL link the recovered workflow to the source asset and provenance record.
5.3 IF metadata extraction fails THEN THE system SHALL preserve the file asset and report a non-fatal metadata diagnostic.

### Requirement 6: App Mode and Baymax Authoring Boundary

**User Story:** As a creator, I want sophisticated workflows exposed through simpler app-like controls when possible.

#### Acceptance Criteria

6.1 WHEN a workflow declares app-mode controls THEN THE system SHALL expose them through the unified authoring app using Baymax UI components.
6.2 IF a workflow has no app-mode metadata THEN THE system SHALL remain available as a graph workflow.
6.3 IF a workflow feature overlaps with the graph editor or authoring app THEN THE implementation SHALL store workflow metadata here and delegate interaction UI to the owning spec.
