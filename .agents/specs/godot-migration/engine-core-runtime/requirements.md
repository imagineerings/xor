# Requirements: Engine Core and Runtime

## Introduction

Baymax should understand Godot project structure, resources, scenes, and class metadata without porting the Godot engine runtime.

### Requirement 1: Avoid Runtime Duplication

#### Acceptance Criteria

1.1 IF a feature requires Godot scene-tree execution THEN THE system SHALL classify it as external-command or excluded.
1.2 WHEN core metadata is needed THEN THE system SHALL model only the data required for Baymax indexing, preview, and tooling.

### Requirement 2: Godot Project Detection

#### Acceptance Criteria

2.1 WHEN a workspace contains `project.godot` THEN THE system SHALL detect the Godot project root.
2.2 WHEN project metadata is invalid THEN THE system SHALL report diagnostics instead of panicking.

### Requirement 3: Resource and Scene Indexing

#### Acceptance Criteria

3.1 WHEN `.tscn` or `.tres` files are indexed THEN THE system SHALL extract resource references and diagnostics.
3.2 IF a resource cannot be parsed THEN THE system SHALL preserve partial metadata where possible.
