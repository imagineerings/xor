# Implementation Plan: Unified Authoring App

## Overview

Build a unified authoring workspace over graph workflows, world-model requests, generated artifacts, natural-language authoring, and SimScript execution. Godot-compatible metadata and native Zed run/export affordances are included only when they unlock the target product; external Godot execution is prohibited.

## Gates

- Start gate: G0 spec consistency, G2 shared Zed game metadata, G3 shared world-model foundations, and applicable G8 Comfy workflow alignment decisions are satisfied.
- Validation gate: app registration, authoring item routing, preview routing, and generated artifact tests pass.
- Handoff gate: unsupported previews and unavailable workers produce actionable diagnostics in the workspace model.
- Completion gate: interactive previews require G4 worker safety, generated artifact views require G6 provenance, and UI work delegates graph editing and workflow metadata to their owning specs.

## Dependency Waves

- W5 Product authoring and agentic tools: authoring workspace routing depends on G2, G3, and the W2-W4 Comfy/world-model harness.

## Tasks

- [ ] 1. Add unified game authoring app model
  - Register the app, define authoring items, route previews, and surface generated artifacts.
  - Represent workspace items, routes, previews, diagnostics, and generated artifact surfaces by composing records at existing Zed workspace/project/editor/media/task owners.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/editor/src/editor.rs, crates/inspector_ui/src/inspector_ui.rs, crates/media/src/media.rs, crates/task/src/task.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/unified-authoring-app/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/unified-authoring-app/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/unified-authoring-app; run workspace, preview, run/export, persistence, cancellation, and recovery scenarios without Godot and inspect processes/packages/loaders/dependencies_

- [ ] 2. Prove native unified authoring without Godot
  - Add hermetic routing, preview, unsupported-runtime, task, artifact, persistence, cancellation, recovery, process, loader, package, and dependency checks.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/editor/src/editor.rs, crates/inspector_ui/src/inspector_ui.rs, crates/media/src/media.rs, crates/task/src/task.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/editor/src/editor.rs, crates/inspector_ui/src/inspector_ui.rs_
  - _Validation: execute every supported authoring capability on a clean machine without Godot and assert no Godot process, library, server, CLI, hidden instance, or runtime dependency_
