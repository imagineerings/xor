# Design: Language and Scripting

## Architecture

Register GDScript and Godot C# affordances through Baymax language and LSP registries. Godot API docs are indexed as documentation metadata.

## Components

- `GdScriptLanguageRegistration`
- `GodotLspAdapter`
- `GodotApiDocsIndex`

## Correctness Properties

### Property 1: Existing LSP Reuse

_For any_ configured Godot language server, Baymax SHALL use existing LSP client infrastructure.

**Validates: Requirement 2.1**
