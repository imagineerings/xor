# Requirements: Comfy Graph and Node Runtime

## Introduction

Sim needs Comfy graph and node runtime compatibility so Comfy workflows can be validated, introspected, partially executed, cached, and connected to Sim artifacts. These graph and node semantics are core world-model harness functionality, so Sim graph implementation decisions must account for this spec before adding alternate execution behavior. This spec owns runtime graph scheduling, validation, and node schema adaptation. It delegates canvas UI to `diffusion-graph-editor/`, HTTP submission to `comfy-runtime-control-plane/`, model loading to `comfy-model-memory-runtime/`, and sampler/model-family execution semantics to `comfy-diffusion-world-model-runtime/`.

## Glossary

- **Node Class**: A registered executable node type with inputs, outputs, category, display name, and execution function.
- **Node Schema**: The introspection data returned to clients through object info.
- **Prompt Graph**: A Comfy execution graph keyed by node id where linked inputs reference upstream node outputs.
- **Execution Plan**: A validated topological order and output target set for running a prompt graph.
- **Node Cache**: Reusable node output state keyed by node identity, input signature, or Sim cache policy.

## Requirements

### Requirement 1: Node Registry and Schema Introspection

**User Story:** As a workflow author, I want Sim to expose Comfy node definitions so workflows can be edited and validated.

#### Acceptance Criteria

1.1 WHEN built-in Comfy node definitions are loaded THEN THE system SHALL register stable node ids, display names, categories, inputs, outputs, tooltips, and search aliases.
1.2 WHEN a client requests object info THEN THE system SHALL return schema data for core, extra, API-provider, and custom nodes that are enabled.
1.3 IF a node is disabled by launch policy THEN THE system SHALL omit it from object info and reject prompts that reference it.

### Requirement 2: Prompt Graph Validation

**User Story:** As a user, I want invalid workflows rejected before expensive generation starts.

#### Acceptance Criteria

2.1 WHEN a prompt graph is submitted THEN THE system SHALL validate node existence, required inputs, linked output indexes, input type compatibility, dependency cycles, and partial execution targets.
2.2 IF validation fails THEN THE system SHALL return per-node errors without enqueueing the prompt.
2.3 WHEN node replacements exist for missing node ids THEN THE system SHALL apply validated replacement mappings before final validation.

### Requirement 3: Execution Planning and Caching

**User Story:** As a workflow author, I want Sim to execute only the graph parts that need to run.

#### Acceptance Criteria

3.1 WHEN a graph has unchanged cached inputs THEN THE system SHALL reuse valid cached outputs according to the selected cache policy.
3.2 WHEN partial execution targets are supplied THEN THE system SHALL execute only the dependency closure needed for those targets.
3.3 IF cache policy is disabled THEN THE system SHALL execute all required nodes for each job.
3.4 WHEN a node emits UI output THEN THE system SHALL preserve output data for history, jobs, previews, and asset enrichment.

### Requirement 4: Async and Batched Node Execution

**User Story:** As a node author, I want Sim to support Comfy nodes that operate over lists or asynchronous work.

#### Acceptance Criteria

4.1 WHEN a node declares list-mapped inputs THEN THE system SHALL map execution over the input list and merge result data consistently.
4.2 WHEN a node returns an execution blocker THEN THE system SHALL stop dependent execution and report a structured block reason.
4.3 WHEN a node performs async work THEN THE system SHALL keep cancellation, progress, and error propagation connected to the parent job.

### Requirement 5: Sim Integration Boundary

**User Story:** As a maintainer, I want runtime compatibility without duplicating the visual editor.

#### Acceptance Criteria

5.1 IF a feature is about drawing, arranging, or interacting with the graph canvas THEN THE implementation SHALL delegate to `diffusion-graph-editor/`.
5.2 WHEN graph runtime produces artifacts THEN THE system SHALL emit provenance records through shared artifact models.
5.3 IF a node requires a provider, model folder, or asset capability that is unavailable THEN THE system SHALL fail validation or execution with actionable diagnostics.
5.4 IF a node requires sampler, scheduler, conditioning, VAE, latent, model patch, diffusion, or world-model execution semantics THEN THE implementation SHALL delegate to `comfy-diffusion-world-model-runtime/`.
