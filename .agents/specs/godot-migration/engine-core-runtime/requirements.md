# Requirements: Engine Core and Runtime

## Introduction

Zed should understand Godot project structure, resources, scenes, and class metadata as native Zed generative game-engine metadata without porting the Godot engine runtime or creating a compatibility shim layer. Godot-origin files are import/source formats; the resulting project, resource, diagnostic, preview, and tooling records are Zed-owned functionality.

### Requirement 1: Avoid Runtime Duplication

#### Acceptance Criteria

1. **1.1** IF a feature requires scene-tree execution that Zed does not natively own THEN THE system SHALL classify it as unresolved, intentionally excluded, or requiring an architecture decision; external Godot execution SHALL NOT count as support.
2. **1.2** WHEN core metadata is needed THEN THE system SHALL model only the data required for Zed indexing, preview, and tooling.
3. **1.3** WHEN Godot-origin project or resource metadata is represented in Zed THEN THE system SHALL store it through native records owned by `project`, `worktree`, or `language`, not a parallel game registry or Godot runtime pass-through record.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** WHEN project, resource, or scene support is claimed THEN storage, parsing, indexing, diagnostics, persistence, and lifecycle SHALL execute in existing Zed owners without a Godot installation.
2. **9.2** THE implementation SHALL NOT launch, link, wrap, bundle, or communicate with Godot and SHALL NOT add a parallel `zed_game` project/resource registry.
3. **9.3** WHEN Godot files are imported THEN outputs SHALL be Zed-native project, worktree, language, resource, and diagnostic state.
4. **9.4** IF executable scene-tree behavior has no approved Zed owner THEN it SHALL remain unresolved or intentionally excluded rather than delegated.
5. **9.5** WHEN validation runs THEN Godot SHALL be absent from PATH and loader paths and process/dependency inspection SHALL prove Zed-owned execution.

### Requirement 2: Godot Project Detection

#### Acceptance Criteria

1. **2.1** WHEN a workspace contains `project.godot` THEN THE system SHALL detect the Godot project root.
2. **2.2** WHEN project metadata is invalid THEN THE system SHALL report diagnostics instead of panicking.

### Requirement 3: Resource and Scene Indexing

#### Acceptance Criteria

1. **3.1** WHEN `.tscn` or `.tres` files are indexed THEN THE system SHALL extract resource references and diagnostics.
2. **3.2** IF a resource cannot be parsed THEN THE system SHALL preserve partial metadata where possible.
