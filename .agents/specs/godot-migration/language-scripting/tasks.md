# Implementation Plan: Language and Scripting

## Overview

Wire GDScript and Godot documentation support through existing Baymax language and docs infrastructure without adding a separate scripting runtime.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Baymax game metadata are satisfied.
- Validation gate: language detection, optional grammar/symbol extraction, LSP configuration, and docs lookup tests pass.
- Handoff gate: unavailable grammar, LSP, or docs sources produce stable degraded-mode diagnostics.
- Completion gate: new tree-sitter grammars, language server binaries, or large fixture corpora require G7 dependency review.

## Dependency Waves

- W2 Baymax game compatibility substrate: language registration and docs indexing depend on G2 shared metadata.

## Tasks

- [ ] 1. Add Godot language support
  - Register GDScript, optional Godot LSP configuration, Godot C# affordances, and API docs indexing.
  - _Requirements: 1.1, 1.2, 2.1, 2.2_
  - _writes: crates/languages/src/gdscript.rs, crates/baymax_game/src/language.rs, crates/baymax_game/src/docs.rs_
