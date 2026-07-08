# Requirements: Editor Experience

## Introduction

Sim should expose native game-development editor affordances through existing Sim UI, project, task, command, and debug systems.

### Requirement 1: Command Integration

#### Acceptance Criteria

1.1 WHEN a Godot project is open THEN THE system SHALL register relevant commands.
1.2 IF no Godot project is detected THEN THE system SHALL not show Godot-specific commands.

### Requirement 2: Project Panel Affordances

#### Acceptance Criteria

2.1 WHEN Godot assets appear in the project panel THEN THE system SHALL classify them by type.
2.2 WHEN `.import` metadata exists THEN THE system SHALL link imported artifacts to source assets.

### Requirement 3: Run and Debug Workflows

#### Acceptance Criteria

3.1 WHEN run/debug is requested THEN THE system SHALL use external Godot task/debug configuration.
3.2 IF the Godot executable is missing THEN THE system SHALL show setup guidance.
