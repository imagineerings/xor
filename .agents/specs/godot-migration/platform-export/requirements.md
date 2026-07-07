# Requirements: Platform and Export

## Introduction

Sim should support Godot project run/export workflows through external task integration, not platform runtime migration.

### Requirement 1: Platform Stack Boundary

#### Acceptance Criteria

1.1 IF a platform feature duplicates Sim platform crates THEN THE system SHALL not port it.

### Requirement 2: Export Task Integration

#### Acceptance Criteria

2.1 WHEN `export_presets.cfg` exists THEN THE system SHALL parse export presets for task templates.
2.2 IF Godot executable configuration is missing THEN THE system SHALL report setup guidance.
