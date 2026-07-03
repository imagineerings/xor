# Implementation Plan: Language and Scripting

## Dependency Gates

- **Primary wave**: W2 Godot compatibility substrate
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy, G2 Shared Godot metadata
- **Dependency gate**: G7 Dependency review before adding a new tree-sitter grammar, language server binary, or large fixture corpus

## Tasks

- [ ] 1. Add Godot language support
  - Register GDScript, optional Godot LSP configuration, Godot C# affordances, and API docs indexing.
  - _Requirements: 1.1, 1.2, 2.1, 2.2_
  - _writes: crates/languages/src/gdscript.rs, crates/godot/src/language.rs, crates/godot/src/docs.rs_
