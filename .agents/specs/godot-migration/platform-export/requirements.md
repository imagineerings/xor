# Requirements: Platform and Export

## Introduction

Zed should support approved Godot-origin export expectations through Zed-owned packaging and deployment paths. Godot export presets are source metadata; Zed owns preset state, packaging execution, artifacts, diagnostics, cancellation, and dependency-review boundaries. A task template that launches Godot is not export support.

### Requirement 1: Platform Stack Boundary

#### Acceptance Criteria

1. **1.1** IF a platform feature duplicates Zed platform crates THEN THE system SHALL not port it.
2. **1.2** WHEN Godot-origin platform/export metadata is represented in Zed THEN THE system SHALL expose records owned by existing Zed task, project, settings, and platform components rather than Godot platform runtime records or a parallel exporter registry.

### Requirement 2: Export Task Integration

#### Acceptance Criteria

1. **2.1** WHEN `export_presets.cfg` exists THEN THE system SHALL parse supported presets into Zed-owned packaging/deployment requests and preserve unsupported options diagnostically.
2. **2.2** IF a native Zed packager, signer, deployer, template, or platform owner is unavailable THEN THE system SHALL report the target as unresolved or unsupported and SHALL NOT request a Godot executable.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** WHEN export support is claimed THEN preset storage, packaging, signing, deployment, artifacts, logs, cancellation, cleanup, and lifecycle SHALL be owned by existing Zed task, project, settings, and platform components.
2. **9.2** Export SHALL NOT invoke, bundle, embed, link, wrap, or delegate to any Godot executable, editor, engine, server, shared library, or command-line tool.
3. **9.3** Importing `export_presets.cfg` MAY preserve compatible names and options, but successful outputs SHALL be Zed-native packages and runtime state.
4. **9.4** Exported projects SHALL execute on target machines without Godot installed; separately approved compatibility tooling, if any, SHALL be isolated from shipped runtime artifacts and cannot establish coverage.
5. **9.5** Validation SHALL inspect package contents, dynamic dependencies, process trees, spawned commands, and runtime behavior in a no-Godot environment.
