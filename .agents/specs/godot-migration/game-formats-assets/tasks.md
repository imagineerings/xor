# Implementation Plan: Game Formats and Assets

## Overview

Register generated assets through the W4 world-model artifact path first, and defer Godot text-format parsing to W7 unless a target-product import requires it.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Zed game metadata are satisfied.
- Validation gate: scene/resource parser, `.import` linker, generated asset registration, and diagnostics tests pass.
- Handoff gate: unsupported binary formats, missing imports, and generated-asset preview gaps produce actionable diagnostics.
- Completion gate: generated mesh asset registration waits for G3 shared world-model foundations and G6 provenance, and new importers/codecs/native geometry dependencies require G7 dependency review.

## Dependency Waves

- W4 Generation outputs and asset pipelines: generated mesh asset registration depends on W2 world-model primitives and G6 provenance.
- W7 Deferred Godot-origin compatibility: text format parsing and `.import` metadata linking start only when they unlock product-critical imports.

## Tasks

- [ ] 1. Implement Godot format parsing and generated asset registration
  - Classify files, parse text resources, link `.import` metadata, and register generated mesh assets.
  - Represent parsing, import linking, caches, dependencies, and generated mesh registration through existing Zed project, worktree, filesystem, preview, media, and artifact owners.
  - _Requirements: 1.1, 2.1, 3.1, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/fs/src/fs.rs, crates/image_viewer/src/image_viewer.rs, crates/media/src/media.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/game-formats-assets/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/game-formats-assets/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/game-formats-assets; run import, cache, dependency, failure, cancellation, and recovery scenarios without Godot and inspect outputs/processes/loaders/dependencies_

- [ ] 2. Prove import outputs are Zed-native without Godot
  - Add hermetic text/binary-format, unsupported-importer, cache invalidation, dependency repair, cancellation, recovery, output-type, process, loader, package, and dependency checks.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/fs/src/fs.rs, crates/image_viewer/src/image_viewer.rs, crates/media/src/media.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/fs/src/fs.rs, crates/image_viewer/src/image_viewer.rs_
  - _Validation: execute every supported importer on a machine without Godot; assert outputs are Zed-native and no Godot process, library, server, CLI, hidden instance, or dependency exists_
