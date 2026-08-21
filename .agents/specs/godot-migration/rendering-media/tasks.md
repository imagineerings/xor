# Implementation Plan: Rendering and Media

## Overview

Route generated outputs through existing Zed preview/media systems first; Godot media classification is deferred W7 compatibility unless it unlocks the target product. Generated media diagnostics start in W2, while generated asset previews finish in W4.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and applicable G3 shared world-model foundations are satisfied.
- Validation gate: media classification, unsupported-preview, generated-output routing, and provenance tests pass.
- Handoff gate: unsupported render/audio/text server features and missing previews produce explicit diagnostics.
- Completion gate: generated media import waits for G4 worker safety and G6 provenance, new codecs/native media/shader dependencies require G7 dependency review, and Comfy media node behavior references G8 Comfy harness alignment.

## Dependency Waves

- W2 Value-first world-model serving substrate: generated media diagnostics and routing depend on G3 and G4.
- W4 Generation outputs and asset pipelines: generated-asset previews and artifact import depend on G6 provenance.
- W7 Deferred Godot-origin compatibility: Godot media classification starts only when it directly supports import or preview of target-product assets.

## Tasks

- [ ] 1. Add Godot media and generated-output preview routing
  - Classify media files, preserve unsupported reasons, and route generated media with provenance.
  - Represent media classification, decoded resources, provenance, and preview routing through existing Zed media/image/component preview records and render surfaces.
  - _Requirements: 1.1, 2.1, 2.2, 3.1, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/media/src/media.rs, crates/image_viewer/src/image_viewer.rs, crates/component_preview/src/component_preview.rs, crates/gpui_wgpu/src/wgpu_renderer.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/rendering-media/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/rendering-media/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/rendering-media; run supported preview/media scenarios without Godot and inspect processes, loaders, packages, GPU/media execution, and dependencies_

- [ ] 2. Prove native rendering and media ownership without Godot
  - Add hermetic preview, unsupported-runtime, device-loss, decode-failure, cancellation, cleanup, process, loader, package, and dependency checks.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/media/src/media.rs, crates/image_viewer/src/image_viewer.rs, crates/component_preview/src/component_preview.rs, crates/gpui_wgpu/src/wgpu_renderer.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/media/src/media.rs, crates/image_viewer/src/image_viewer.rs, crates/component_preview/src/component_preview.rs_
  - _Validation: execute every supported preview/media capability on a clean machine without Godot and assert no Godot process, library, server, CLI, hidden instance, or runtime dependency_
