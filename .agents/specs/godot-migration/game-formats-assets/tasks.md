# Implementation Plan: Game Formats and Assets

## Overview

Parse Godot text formats in W2 and defer generated mesh asset registration to W5, where world-model artifact and provenance foundations exist.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Godot metadata are satisfied.
- Validation gate: scene/resource parser, `.import` linker, generated asset registration, and diagnostics tests pass.
- Handoff gate: unsupported binary formats, missing imports, and generated-asset preview gaps produce actionable diagnostics.
- Completion gate: generated mesh asset registration waits for G3 shared world-model foundations and G6 provenance, and new importers/codecs/native geometry dependencies require G7 dependency review.

## Dependency Waves

- W2 Godot compatibility substrate: text format parsing and `.import` metadata linking can start after G2.
- W5 Generation outputs and asset pipelines: generated mesh asset registration depends on W3 world-model primitives and G6 provenance.

## Tasks

- [ ] 1. Implement Godot format parsing and generated asset registration
  - Classify files, parse text resources, link `.import` metadata, and register generated mesh assets.
  - _Requirements: 1.1, 2.1, 3.1_
  - _writes: crates/godot/src/formats.rs, crates/godot/src/imports.rs, crates/godot/src/generated_assets.rs_
