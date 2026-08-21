# Design: Language and Scripting

## Architecture

SimScript is registered as a native Zed language through the same `LanguageRegistry::add` API used for Rust, Python, and TypeScript. Natural language is the primary authoring interface: creators and agents express intent in natural language, and Zed materializes that intent as executable SimScript. Natural language is not the runtime language and is never treated as executable code.

Legacy `.gd` scripts are a migration source format. `.gd` files are classified as Godot migration sources so users can inspect and migrate existing scripts; they are not described as SimScript-compatible until a reviewed translator proves syntax, semantics, lifecycle, failure, and execution behavior. Native authoring writes `.simscript` files and exposes SimScript docs, diagnostics, and language services.

There is no separate legacy script registration or compatibility registrar type. The language config is constructed from standard `LanguageConfig` metadata (name, extensions, line comments, LSP adapter name) and fed directly into `Language::new` + `LanguageRegistry::add`.

Comfy-era generation intent is recreated as native SimScript authoring intent. Natural language remains an authoring input, never executable source, and no `Comfy*` language or generation record is exposed from the SimScript layer.

## Components

- Existing `languages` and `LanguageRegistry` integration for SimScript and Godot migration-source classification.
- Existing editor/agent diff flow for natural-language-to-SimScript proposals.
- Existing docs index for Zed API documentation and Godot migration/reference material.
- Existing LSP and DAP infrastructure for approved Zed-owned language/debug services.

## Correctness Properties

### Property 1: Existing LSP Reuse

_For any_ configured SimScript language server, Zed SHALL use existing LSP client infrastructure.

**Validates: Requirement 3.1**

### Property 2: Native Language Registration

_For any_ SimScript file, Zed SHALL classify and highlight it using the same `Language` type used for all other languages, registered through `LanguageRegistry::add`.

**Validates: Requirement 1.1, 1.3**

### Property 3: Natural Language Is Intent, SimScript Is Executable

_For any_ natural-language gameplay instruction, Zed SHALL convert the instruction into inspectable SimScript before execution or application.

**Validates: Requirement 2.1, 2.2, 2.3, 2.4**

### Property 4: Docs Scope Separation

_For any_ indexed Godot API reference, Zed SHALL expose it as migration reference docs while keeping Zed game API docs as the primary SimScript surface.

**Validates: Requirement 3.2, 3.3**

### D-NATIVE: Native scripting path

Source compatibility terminates at explicit parser/translator/protocol boundaries. Existing Zed language/LSP/DAP/task/extension owners control source state, execution, diagnostics, trust, cancellation, reload, and cleanup. No Godot language server, runtime, debugger, Mono host, or editor process is launched.

**Validates: Requirement 1.2, 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.4 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
