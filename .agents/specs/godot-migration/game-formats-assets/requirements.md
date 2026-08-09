# Requirements: Game Formats and Assets

## Introduction

Sim should parse and index Godot scene/resource/import files and generated game assets without importing duplicate geometry or codec stacks.

### Requirement 1: Scene and Resource Parsing

#### Acceptance Criteria

1. **1.1** WHEN `.tscn`, `.tres`, or `.godot` files are parsed THEN THE system SHALL extract references and diagnostics.

### Requirement 2: Import Metadata Linking

#### Acceptance Criteria

1. **2.1** WHEN `.import` metadata exists THEN THE system SHALL link source and generated imported assets.

### Requirement 3: Generated Asset Integration

#### Acceptance Criteria

1. **3.1** WHEN generated mesh assets are imported THEN THE system SHALL register preview, export, and provenance metadata.

### Requirement 9: Native Sim Ownership

#### Acceptance Criteria

1. **9.1** Supported format parsing, importing, caching, dependency tracking, storage, UI, cancellation, recovery, and lifecycle SHALL be owned by existing Sim `project`, `worktree`, `fs`, preview, media, and artifact components.
2. **9.2** Importers and migration tools SHALL NOT invoke, wrap, link, embed, proxy, or communicate with Godot.
3. **9.3** WHEN Godot formats are read THEN successful outputs SHALL be Sim-native records, resources, scenes, artifacts, cache entries, dependencies, previews, and runtime state.
4. **9.4** A format declaration, parser interface, metadata link, placeholder, or external delegation SHALL NOT count as successful import or executable compatibility.
5. **9.5** Validation SHALL run with Godot absent and inspect importer processes, outputs, caches, packages, loaders, dependencies, cancellation, and recovery.
