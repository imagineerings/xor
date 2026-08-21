# Requirements: Language and Scripting

## Introduction

Zed should support SimScript as the native executable game language, with natural language as the primary authoring interface. Creators describe gameplay, assets, scene behavior, and generation intent in natural language; Zed turns those intents into inspectable, editable SimScript. Legacy `.gd` scripts remains a source-format migration path, but the native language surface is SimScript registered through `LanguageRegistry::add` just like Rust, Python, and TypeScript.

### Requirement 1: SimScript Recognition

#### Acceptance Criteria

1. **1.1** WHEN `.simscript` files are opened THEN THE system SHALL classify them as SimScript.
2. **1.2** WHEN imported `.gd` files are opened THEN THE system SHALL classify them as Godot migration sources, preserve migration metadata, and SHALL NOT claim SimScript source compatibility without grammar, semantic, translation, and execution evidence.
3. **1.3** WHEN grammar support is available THEN THE system SHALL provide syntax and symbol extraction.

### Requirement 2: Natural Language Authoring

#### Acceptance Criteria

1. **2.1** WHEN a creator describes gameplay behavior in natural language THEN THE system SHALL produce or update executable SimScript rather than treating the natural language text as executable code.
2. **2.2** WHEN an agent edits SimScript from a natural-language instruction THEN THE system SHALL show the generated SimScript diff before applying changes.
3. **2.3** IF a natural-language instruction is ambiguous THEN THE system SHALL request clarification or produce a non-destructive draft.
4. **2.4** WHEN generation intent originates from Comfy-era workflows THEN THE system SHALL recreate it as native SimScript authoring intent, not as a Comfy pass-through label or executable natural-language prompt.

### Requirement 3: LSP and Docs

#### Acceptance Criteria

1. **3.1** WHEN a SimScript LSP is configured THEN THE system SHALL connect through existing LSP infrastructure.
2. **3.2** WHEN Zed game API docs are indexed THEN THE system SHALL make class and capability docs available to language features.
3. **3.3** WHEN Godot API docs are indexed for migrated projects THEN THE system SHALL expose them as migration/reference docs, not as the primary SimScript API surface.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** Supported parsing, translation, language services, script execution, debugging, persistence, trust, and lifecycle SHALL be owned by existing Zed `language`, `languages`, `lsp`, `dap`, `extension_host`, and task components as applicable.
2. **9.2** THE system SHALL NOT launch, wrap, proxy, embed, link, or communicate with the Godot editor, engine, GDScript runtime, language server, debugger, Mono integration, or command-line tools.
3. **9.3** `.gd`, C# project, Godot API, and protocol compatibility MAY be preserved at explicit source/file/API boundaries, but successful translations and execution SHALL produce Zed-native source, diagnostics, state, and runtime behavior.
4. **9.4** Syntax declarations, file recognition, interfaces, placeholder instances, or configured external Godot tools SHALL NOT count as executable script support.
5. **9.5** Every implemented or fully specified language capability SHALL validate with Godot absent and inspect spawned processes, language/debug servers, packages, loaders, and runtime dependencies.
