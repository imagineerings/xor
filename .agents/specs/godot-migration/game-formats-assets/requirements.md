# Requirements: Game Formats and Assets

## Introduction

Sim should parse and index Godot scene/resource/import files and generated game assets without importing duplicate geometry or codec stacks.

### Requirement 1: Scene and Resource Parsing

#### Acceptance Criteria

1.1 WHEN `.tscn`, `.tres`, or `.godot` files are parsed THEN THE system SHALL extract references and diagnostics.

### Requirement 2: Import Metadata Linking

#### Acceptance Criteria

2.1 WHEN `.import` metadata exists THEN THE system SHALL link source and generated imported assets.

### Requirement 3: Generated Asset Integration

#### Acceptance Criteria

3.1 WHEN generated mesh assets are imported THEN THE system SHALL register preview, export, and provenance metadata.
