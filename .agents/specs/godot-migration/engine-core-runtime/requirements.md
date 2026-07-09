# Requirements: Engine Core and Runtime

## Introduction

Sim should understand Godot project structure, resources, scenes, and class metadata as native Sim generative game-engine metadata without porting the Godot engine runtime or creating a compatibility shim layer. Godot-origin files are import/source formats; the resulting project, resource, diagnostic, preview, and tooling records are Sim-owned functionality.

### Requirement 1: Avoid Runtime Duplication

#### Acceptance Criteria

1.1 IF a feature requires Godot scene-tree execution THEN THE system SHALL classify it as external-command or excluded.
1.2 WHEN core metadata is needed THEN THE system SHALL model only the data required for Sim indexing, preview, and tooling.
1.3 WHEN Godot-origin project or resource metadata is represented in Sim THEN THE system SHALL use native `SimGame*` records that serve the generative game engine, not Godot runtime pass-through records.

### Requirement 2: Godot Project Detection

#### Acceptance Criteria

2.1 WHEN a workspace contains `project.godot` THEN THE system SHALL detect the Godot project root.
2.2 WHEN project metadata is invalid THEN THE system SHALL report diagnostics instead of panicking.

### Requirement 3: Resource and Scene Indexing

#### Acceptance Criteria

3.1 WHEN `.tscn` or `.tres` files are indexed THEN THE system SHALL extract resource references and diagnostics.
3.2 IF a resource cannot be parsed THEN THE system SHALL preserve partial metadata where possible.
