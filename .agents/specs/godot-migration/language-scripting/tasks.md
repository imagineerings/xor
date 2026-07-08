# Implementation Plan: Language and Scripting

## Overview

Wire SimScript as the native executable game language and natural language as the authoring interface. No parallel registration types — SimScript uses `LanguageRegistry::add` directly. Legacy `.gd` scripts remains a source-format migration path, but native authoring produces `.simscript` files.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: language detection, optional grammar/symbol extraction, LSP configuration, natural-language-to-SimScript draft/diff behavior, and docs lookup tests pass.
- Handoff gate: unavailable grammar, LSP, docs, or natural-language generation services produce stable degraded-mode diagnostics.
- Completion gate: new tree-sitter grammars, language server binaries, generation services, or large fixture corpora require G7 dependency review.

## Dependency Waves

- W2 Sim game compatibility substrate: language registration and docs indexing depend on G2 shared metadata.

## Tasks

- [ ] 1. Add SimScript language support
  - Register SimScript as a native Sim language via `LanguageRegistry::add`, configure SimScript LSP adapter, legacy `.gd` classification, Sim game API docs indexing, and natural-language-to-SimScript draft/diff behavior.
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3_
  - _writes: crates/languages/src/simscript.rs, crates/sim_game/src/language.rs, crates/sim_game/src/docs.rs_
