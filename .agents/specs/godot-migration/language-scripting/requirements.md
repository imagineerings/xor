# Requirements: Language and Scripting

## Introduction

Sim should support SimScript as the native executable game language, with natural language as the primary authoring interface. Creators describe gameplay, assets, scene behavior, and generation intent in natural language; Sim turns those intents into inspectable, editable SimScript. Legacy `.gd` scripts remains a source-format migration path, but the native language surface is SimScript registered through `LanguageRegistry::add` just like Rust, Python, and TypeScript.

### Requirement 1: SimScript Recognition

#### Acceptance Criteria

1.1 WHEN `.simscript` files are opened THEN THE system SHALL classify them as SimScript.
1.2 WHEN imported `.gd` files are opened THEN THE system SHALL classify them as SimScript source-compatible files and preserve migration metadata.
1.3 WHEN grammar support is available THEN THE system SHALL provide syntax and symbol extraction.

### Requirement 2: Natural Language Authoring

#### Acceptance Criteria

2.1 WHEN a creator describes gameplay behavior in natural language THEN THE system SHALL produce or update executable SimScript rather than treating the natural language text as executable code.
2.2 WHEN an agent edits SimScript from a natural-language instruction THEN THE system SHALL show the generated SimScript diff before applying changes.
2.3 IF a natural-language instruction is ambiguous THEN THE system SHALL request clarification or produce a non-destructive draft.

### Requirement 3: LSP and Docs

#### Acceptance Criteria

3.1 WHEN a SimScript LSP is configured THEN THE system SHALL connect through existing LSP infrastructure.
3.2 WHEN Sim game API docs are indexed THEN THE system SHALL make class and capability docs available to language features.
3.3 WHEN Godot API docs are indexed for migrated projects THEN THE system SHALL expose them as migration/reference docs, not as the primary SimScript API surface.
