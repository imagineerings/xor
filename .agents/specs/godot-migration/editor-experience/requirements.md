# Requirements: Editor Experience

## Introduction

Zed should expose native game-development editor affordances through existing Zed UI, project, task, command, and debug systems. Comfy-era authoring affordances must be recreated as native Zed commands and metadata, not exposed as compatibility labels or pass-through workflows.

### Requirement 1: Command Integration

#### Acceptance Criteria

1. **1.1** WHEN a Godot project is open THEN THE system SHALL register relevant commands.
2. **1.2** IF no Godot project is detected THEN THE system SHALL not show Godot-specific commands.
3. **1.3** WHEN commands originate from migrated Comfy/world-model authoring flows THEN THE system SHALL expose them as native Zed game commands.

### Requirement 2: Project Panel Affordances

#### Acceptance Criteria

1. **2.1** WHEN Godot assets appear in the project panel THEN THE system SHALL classify them by type.
2. **2.2** WHEN `.import` metadata exists THEN THE system SHALL link imported artifacts to source assets.

### Requirement 3: Run and Debug Workflows

#### Acceptance Criteria

1. **3.1** WHEN run/debug is requested for a supported capability THEN THE system SHALL execute through Zed-owned task, runtime, debugger, and diagnostic services.
2. **3.2** IF Zed does not natively implement the requested run/debug behavior THEN THE system SHALL report an unresolved or intentionally unsupported capability and SHALL NOT offer Godot setup or delegation as support.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** WHEN an editor capability is supported THEN its command, UI, task, debugger, persistence, cancellation, and lifecycle paths SHALL be owned by `workspace`, `project_panel`, `editor`, `command_palette`, `task`, and `dap` as applicable.
2. **9.2** THE editor SHALL NOT launch, embed, wrap, proxy, or communicate with a Godot editor or runtime.
3. **9.3** Godot-compatible command names, settings, files, and debugger payloads MAY cross explicit compatibility boundaries, but Zed-owned state and execution SHALL remain authoritative.
4. **9.4** The design SHALL extend existing command, project-panel, task, and debugger integration points instead of creating Godot-specific providers that duplicate them.
5. **9.5** Run/debug validation SHALL pass with Godot absent and SHALL inspect process and runtime dependencies for Godot delegation.
