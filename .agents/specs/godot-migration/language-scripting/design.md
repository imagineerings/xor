# Design: Language and Scripting

## Architecture

SimScript is registered as a native Sim language through the same `LanguageRegistry::add` API used for Rust, Python, and TypeScript. Natural language is the primary authoring interface: creators and agents express intent in natural language, and Sim materializes that intent as executable SimScript. Natural language is not the runtime language and is never treated as executable code.

Legacy `.gd` scripts are a migration source format. `.gd` files can be classified under SimScript-compatible language support so users can inspect and migrate existing scripts, but native authoring writes `.simscript` files and exposes SimScript docs, diagnostics, and language services.

There is no separate legacy script registration or compatibility registrar type. The language config is constructed from standard `LanguageConfig` metadata (name, extensions, line comments, LSP adapter name) and fed directly into `Language::new` + `LanguageRegistry::add`.

Comfy-era generation intent is recreated as native SimScript authoring intent. Natural language remains an authoring input, never executable source, and no `Comfy*` language or generation record is exposed from the SimScript layer.

## Components

- `sim_game::simscript_language_config()` — returns metadata for SimScript (name, native/imported extensions, line comment tokens, LSP adapter name). Called from `sim::register_game_integration` to build the native `Language` instance.
- `SimScriptLanguageSupport` — classifies `.simscript` files as native and `.gd` files as imported migration source.
- `NaturalLanguageGameAuthoring` — translates creator intent into inspectable SimScript drafts or diffs before execution.
- `SimGameDocsIndex` — indexes Sim game API documentation, plus Godot API migration/reference docs when needed.
- `SimScriptLspAdapter` — LSP adapter for SimScript (delegates to existing LSP infrastructure).

## Correctness Properties

### Property 1: Existing LSP Reuse

_For any_ configured SimScript language server, Sim SHALL use existing LSP client infrastructure.

**Validates: Requirement 3.1**

### Property 2: Native Language Registration

_For any_ SimScript file, Sim SHALL classify and highlight it using the same `Language` type used for all other languages, registered through `LanguageRegistry::add`.

**Validates: Requirement 1.1, 1.3**

### Property 3: Natural Language Is Intent, SimScript Is Executable

_For any_ natural-language gameplay instruction, Sim SHALL convert the instruction into inspectable SimScript before execution or application.

**Validates: Requirement 2.1, 2.2, 2.3, 2.4**

### Property 4: Docs Scope Separation

_For any_ indexed Godot API reference, Sim SHALL expose it as migration reference docs while keeping Sim game API docs as the primary SimScript surface.

**Validates: Requirement 3.2, 3.3**
