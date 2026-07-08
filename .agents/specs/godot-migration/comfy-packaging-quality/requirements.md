# Requirements: Comfy Packaging, Configuration, and Quality

## Introduction

Sim needs Comfy migration support for launch configuration, feature flags, frontend/template/doc packages, OpenAPI/schema compatibility, examples, tests, CI, dependency review, and platform packaging. These controls protect core Comfy-derived world-model harness behavior from drift across implementation tasks. This spec owns migration quality controls and compatibility fixtures. It delegates runtime behavior to the other Comfy specs. Comfy compatibility defines the expected packaging, schema, and fixture semantics, but every supported quality-control feature must validate native Sim functionality rather than certify a thin compatibility label, hidden ComfyUI pass-through, or unsupported placeholder.

## Glossary

- **Launch Profile**: A named set of runtime options for networking, directories, devices, memory, precision, caching, assets, API nodes, custom nodes, and logging.
- **Feature Flag**: A server/client capability flag negotiated through CLI, config, or WebSocket.
- **Compatibility Fixture**: A captured workflow, API request, blueprint, node schema, or expected response used to prevent migration regressions.
- **Dependency Review**: A record of license, maintenance, security, platform, binary-size, and runtime impact for a new dependency.
- **Packaging Target**: A supported distribution or launch environment such as local desktop, portable Windows, CPU-only, GPU-specific, or remote worker.

## Requirements

### Requirement 1: Launch Configuration

**User Story:** As a developer, I want Comfy-compatible launch options mapped to Sim configuration so runtime behavior is predictable.

#### Acceptance Criteria

1.1 WHEN a launch profile is parsed THEN THE system SHALL capture listen address, port, TLS, CORS, upload size, base/input/output/temp/user directories, auto-launch, logging, assets, database URL, API nodes, custom nodes, manager mode, feature flags, and compression settings.
1.2 WHEN device, precision, memory, attention, cache, or performance options are present THEN THE system SHALL pass validated settings to the model runtime policy resolver.
1.3 IF a launch option is unsupported THEN THE system SHALL report the option, reason, and nearest Sim equivalent.
1.4 WHEN Comfy-compatible launch options are represented in Sim THEN THE system SHALL use native `SimLaunch*` implementation types and SHALL NOT expose Sim-owned launch profiles as `ComfyLaunch*` pass-through records.

### Requirement 2: Feature Flags and Frontend Packages

**User Story:** As a frontend integrator, I want server capabilities and frontend/template/docs packages versioned clearly.

#### Acceptance Criteria

2.1 WHEN server features are requested THEN THE system SHALL return core flags for preview metadata, upload size, manager support, node replacements, assets, and CLI-provided flags that do not overwrite core flags.
2.2 WHEN client feature flags are negotiated THEN THE system SHALL store connection-specific flags for event behavior.
2.3 WHEN frontend, workflow template, or embedded docs packages are missing or outdated THEN THE system SHALL show actionable diagnostics.
2.4 WHEN Comfy-compatible feature flags are represented in Sim THEN THE system SHALL use native `SimFeatureFlag*` implementation types and SHALL NOT expose Sim-owned flag registries as `ComfyFeatureFlag*` pass-through records.

### Requirement 3: API Schemas and Examples

**User Story:** As an API client author, I want compatibility documented and tested through schemas and examples.

#### Acceptance Criteria

3.1 WHEN Comfy-compatible APIs are exposed THEN THE system SHALL include OpenAPI or equivalent schema coverage for supported routes.
3.2 WHEN example scripts are migrated THEN THE system SHALL provide automated fixtures for basic HTTP prompt submission and WebSocket completion/output retrieval.
3.3 IF a documented OpenAPI route is not implemented locally THEN THE system SHALL mark it as cloud-only, external, unsupported, or planned.
3.4 WHEN Comfy-compatible API schema status is represented in Sim THEN THE system SHALL use native `SimApiSchema*` implementation types and SHALL NOT expose Sim-owned schema catalogs as `ComfyApiSchema*` pass-through records.

### Requirement 4: Automated Test Coverage

**User Story:** As a maintainer, I want migration coverage that catches behavior drift.

#### Acceptance Criteria

4.1 WHEN runtime control-plane features are implemented THEN THE system SHALL include route, WebSocket, queue, jobs, cancellation, and safety tests.
4.2 WHEN node, model, asset, workflow, media, provider, or extension features are implemented THEN THE system SHALL include fixture or snapshot tests for each owned capability group.
4.3 WHEN generated media quality comparisons are practical THEN THE system SHALL support deterministic or threshold-based comparison tests without requiring production model downloads by default.

### Requirement 5: Packaging and Dependency Governance

**User Story:** As a maintainer, I want packaging and dependency changes controlled before they affect Sim distributions.

#### Acceptance Criteria

5.1 IF a task adds a native library, codec, Python package, provider SDK, model dependency, frontend package, or vendored code THEN THE system SHALL require dependency review before implementation.
5.2 WHEN platform packaging is configured THEN THE system SHALL describe CPU-only and GPU-specific launch profiles without duplicating Sim platform packaging.
5.3 WHEN a task requires network access or large downloads THEN THE system SHALL require explicit user approval and preserve an audit record.

### Requirement 6: Logs and Internal Diagnostics

**User Story:** As a developer, I want logs and internal diagnostics for Comfy compatibility work.

#### Acceptance Criteria

6.1 WHEN logs are requested THEN THE system SHALL expose raw and formatted logs through Sim diagnostics with terminal size metadata where available.
6.2 WHEN folder paths or recent files are requested for diagnostics THEN THE system SHALL expose only approved input, output, temp, model, and user roots.
6.3 IF a diagnostic endpoint is internal-only THEN THE system SHALL mark it unstable and avoid depending on it for public API compatibility.
