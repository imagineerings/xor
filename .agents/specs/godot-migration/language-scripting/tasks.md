# Implementation Plan: Language and Scripting

## Overview

Wire SimScript as the native executable game language and natural language as the authoring interface. No parallel registration types — SimScript uses `LanguageRegistry::add` directly. Legacy `.gd` scripts remain a W7 source-format migration path, but native authoring produces `.simscript` files.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Zed game metadata are satisfied.
- Validation gate: language detection, optional grammar/symbol extraction, LSP configuration, natural-language-to-SimScript draft/diff behavior, and docs lookup tests pass.
- Handoff gate: unavailable grammar, LSP, docs, or natural-language generation services produce stable degraded-mode diagnostics.
- Completion gate: new tree-sitter grammars, language server binaries, generation services, or large fixture corpora require G7 dependency review.

## Dependency Waves

- W5 Product authoring and agentic tools: native SimScript and natural-language authoring depend on G2 shared metadata and consume the W2-W4 harness.
- W7 Deferred Godot-origin compatibility: legacy `.gd`, Godot C#, and Godot API docs indexing start only when they directly support import/migration or SimScript authoring.

## Tasks

- [ ] 1. Add SimScript language support
  - Register SimScript as a native Zed language via `LanguageRegistry::add`, configure SimScript LSP adapter metadata, legacy `.gd` classification, Zed game API docs indexing, and natural-language-to-SimScript draft/diff behavior.
  - Recreate Comfy-era generation intent as native SimScript authoring intent, not as a thin compatibility label or pass-through.
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/languages/src/lib.rs, crates/language/src/language_registry.rs, crates/lsp/src/lsp.rs, crates/dap/src/dap.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/language-scripting/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/language-scripting/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/language-scripting; run recognition, translation, execution, LSP, DAP, and unsupported-language scenarios without Godot and inspect all spawned servers/processes and dependencies_

- [ ] 2. Prove native language and script ownership without Godot
  - Add hermetic recognition, translation, execution, reload, trust, failure, cancellation, LSP/DAP server, process, package, loader, and dependency checks; keep unimplemented GDScript/C# behavior unresolved.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/languages/src/lib.rs, crates/language/src/language_registry.rs, crates/lsp/src/lsp.rs, crates/dap/src/dap.rs, crates/extension_host/src/extension_host.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/languages/src/lib.rs, crates/language/src/language_registry.rs, crates/lsp/src/lsp.rs, crates/dap/src/dap.rs_
  - _Validation: execute every supported script/language scenario on a clean machine without Godot and assert no Godot process, language server, debugger, runtime, library, CLI, hidden instance, or dependency_
