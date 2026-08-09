# Implementation Plan: XR and Spatial Tooling

## Overview

Keep XR runtime support out of Sim and expose W7 native Sim spatial metadata, docs, and preview routing only when they directly support the target generative game engine.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: XR runtime exclusion, spatial metadata, docs hook, and preview routing tests pass.
- Handoff gate: unsupported XR runtime paths produce explicit excluded or decision-required diagnostics.
- Completion gate: XR fallback work waits for W7 task diagnostics and does not embed OpenXR, WebXR, or VR runtime stacks.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: spatial metadata and native preview/docs hooks wait for G1, G2, and an explicit product-enabling dependency.

## Tasks

- [ ] 1. Add XR boundary and spatial metadata support
  - Encode XR runtime exclusions and expose native Sim spatial asset metadata/docs hooks.
  - _Requirements: 1.1, 1.2, 2.1, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/project/src/project.rs, crates/media/src/media.rs, crates/component_preview/src/component_preview.rs, crates/settings/src/settings.rs, crates/gpui_platform/src/gpui_platform.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/xr-spatial/requirements.md, /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/xr-spatial/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/xr-spatial; run supported metadata/preview and excluded-runtime scenarios without Godot and inspect processes, loaders, packages, and dependencies_

- [ ] 2. Prove native spatial ownership without Godot
  - Add hermetic metadata, preview, unsupported-runtime, cancellation, cleanup, package, process, loader, and dependency checks.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/project/src/project.rs, crates/media/src/media.rs, crates/component_preview/src/component_preview.rs, crates/settings/src/settings.rs, crates/gpui_platform/src/gpui_platform.rs_
  - _Writes: crates/project/src/project.rs, crates/component_preview/src/component_preview.rs, crates/settings/src/settings.rs_
  - _Validation: execute supported spatial scenarios on a machine without Godot and assert no Godot process, library, server, CLI, hidden instance, or runtime dependency_
