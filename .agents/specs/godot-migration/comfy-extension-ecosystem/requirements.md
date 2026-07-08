# Requirements: Comfy Extension Ecosystem

## Introduction

Sim needs a controlled harness extension path for Comfy custom nodes, extension web assets, translations, example workflows, prestartup scripts, and ComfyUI-Manager-like package metadata. Custom node discovery is part of the world-model harness feature surface, but execution remains policy-gated. This spec owns extension discovery and policy. It delegates executable node runtime to `comfy-graph-node-runtime/`, provider calls to `comfy-api-provider-nodes/`, and packaging/dependency installation to `comfy-packaging-quality/`. Comfy compatibility defines the expected extension semantics and fixtures, but every supported extension feature must be recreated as native Sim functionality backed by Sim extension policy, asset, web-service, node-schema, and diagnostic services rather than passed through to ComfyUI or represented by a compatibility label alone.

## Glossary

- **Custom Node Pack**: A third-party Comfy extension directory or Python file that registers node classes or a modern extension entrypoint.
- **Extension Web Directory**: Static JavaScript or frontend assets shipped by a custom node pack.
- **Prestartup Script**: A custom node script executed before normal node loading.
- **Locale Bundle**: Custom node translation files under `locales/<lang>/`.
- **Manager Policy**: Rules that enable, disable, install, update, or isolate custom node packs.

## Requirements

### Requirement 1: Extension Discovery

**User Story:** As a user, I want Sim to discover installed Comfy custom node packs while keeping them isolated and diagnosable.

#### Acceptance Criteria

1.1 WHEN custom nodes are enabled THEN THE system SHALL discover Python files and directories under configured custom node roots.
1.2 WHEN custom nodes are disabled THEN THE system SHALL skip all custom node loading except explicitly whitelisted packs.
1.3 WHEN a node pack is blocked by policy THEN THE system SHALL record a blocked diagnostic and omit its nodes and web assets.
1.4 IF an extension fails to import THEN THE system SHALL continue startup and expose an import diagnostic.

### Requirement 2: Node and Extension Registration

**User Story:** As a node author, I want supported custom nodes to register schemas, display names, and web assets.

#### Acceptance Criteria

2.1 WHEN a node pack exposes `NODE_CLASS_MAPPINGS` THEN THE system SHALL register supported node classes and display names.
2.2 WHEN a node pack exposes a modern extension entrypoint THEN THE system SHALL call it through the approved runtime boundary and register returned node schemas.
2.3 WHEN a node pack declares a web directory THEN THE system SHALL serve static assets through Sim's extension asset service with safe path confinement.
2.4 IF a node pack lacks a supported registration mechanism THEN THE system SHALL skip it with a diagnostic.

### Requirement 3: Startup Scripts and Dependency Policy

**User Story:** As a maintainer, I want extension startup behavior controlled because arbitrary scripts can modify process state.

#### Acceptance Criteria

3.1 WHEN a prestartup script exists THEN THE system SHALL execute it only if extension policy allows scripts for that pack.
3.2 WHEN a prestartup or import script changes global hooks or runtime state THEN THE system SHALL restore protected Sim hooks after loading.
3.3 IF an extension requires missing dependencies THEN THE system SHALL report installation instructions without silently installing packages.

### Requirement 4: Translations, Templates, and Subgraphs

**User Story:** As a frontend user, I want custom node translations, templates, and subgraphs available in Sim.

#### Acceptance Criteria

4.1 WHEN locale bundles exist THEN THE system SHALL merge `main.json`, `nodeDefs.json`, `commands.json`, and `settings.json` by language.
4.2 WHEN example workflow folders exist THEN THE system SHALL expose workflow template names and static template assets.
4.3 WHEN subgraph folders exist THEN THE system SHALL expose reusable subgraphs through the shared subgraph index.

### Requirement 5: Manager Compatibility Boundary

**User Story:** As a user, I want manager-like custom node workflows without giving extensions uncontrolled package-manager access.

#### Acceptance Criteria

5.1 WHEN manager integration is enabled THEN THE system SHALL expose manager status and policy metadata through Sim-approved routes or tools.
5.2 IF manager UI or endpoints are disabled THEN THE system SHALL still honor scheduled background operations only when policy permits them.
5.3 WHEN install, update, or disable actions require network or filesystem writes THEN THE system SHALL require explicit user approval and dependency review.
