# Requirements: Platform and Export

## Introduction

Sim should support Godot-origin run/export expectations as native Sim game task templates for the generative game engine. Godot export presets are source metadata; Sim owns the executable settings, task records, diagnostics, and dependency-review boundaries. There is no Godot platform compatibility shim.

### Requirement 1: Platform Stack Boundary

#### Acceptance Criteria

1.1 IF a platform feature duplicates Sim platform crates THEN THE system SHALL not port it.
1.2 WHEN Godot-origin platform/export metadata is represented in Sim THEN THE system SHALL expose native `SimGame*` task records and diagnostics rather than Godot platform runtime records.

### Requirement 2: Export Task Integration

#### Acceptance Criteria

2.1 WHEN `export_presets.cfg` exists THEN THE system SHALL parse export presets for task templates.
2.2 IF Godot executable configuration is missing THEN THE system SHALL report setup guidance.
