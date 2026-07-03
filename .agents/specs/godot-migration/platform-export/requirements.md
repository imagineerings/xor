# Requirements: Platform and Export

## Introduction

Baymax should support Godot project run/export workflows through external task integration, not platform runtime migration.

### Requirement 1: Platform Stack Boundary

#### Acceptance Criteria

1. IF a platform feature duplicates Baymax platform crates THEN THE system SHALL not port it.

### Requirement 2: Export Task Integration

#### Acceptance Criteria

1. WHEN `export_presets.cfg` exists THEN THE system SHALL parse export presets for task templates.
2. IF Godot executable configuration is missing THEN THE system SHALL report setup guidance.
