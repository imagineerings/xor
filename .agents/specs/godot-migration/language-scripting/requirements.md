# Requirements: Language and Scripting

## Introduction

Sim should support GDScript, Godot C# affordances, and Godot API documentation using existing language infrastructure.

### Requirement 1: GDScript Recognition

#### Acceptance Criteria

1.1 WHEN `.gd` files are opened THEN THE system SHALL classify them as GDScript.
1.2 WHEN grammar support is available THEN THE system SHALL provide syntax and symbol extraction.

### Requirement 2: LSP and Docs

#### Acceptance Criteria

2.1 WHEN a Godot LSP is configured THEN THE system SHALL connect through existing LSP infrastructure.
2.2 WHEN Godot API docs are indexed THEN THE system SHALL make class docs available to language features.
