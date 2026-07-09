# Implementation Plan: Game Formats and Assets

## Overview

Register generated assets through the W4 world-model artifact path first, and defer Godot text-format parsing to W7 unless a target-product import requires it.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: scene/resource parser, `.import` linker, generated asset registration, and diagnostics tests pass.
- Handoff gate: unsupported binary formats, missing imports, and generated-asset preview gaps produce actionable diagnostics.
- Completion gate: generated mesh asset registration waits for G3 shared world-model foundations and G6 provenance, and new importers/codecs/native geometry dependencies require G7 dependency review.

## Dependency Waves

- W4 Generation outputs and asset pipelines: generated mesh asset registration depends on W2 world-model primitives and G6 provenance.
- W7 Deferred Godot-origin compatibility: text format parsing and `.import` metadata linking start only when they unlock product-critical imports.

## Tasks

- [x] 1. Implement Godot format parsing and generated asset registration
  - Classify files, parse text resources, link `.import` metadata, and register generated mesh assets.
  - Represent parsing, import linking, and generated mesh registration with native Sim `SimGame*` and `SimGeneratedAsset*` records backed by `world_model` provenance metadata.
  - _Requirements: 1.1, 2.1, 3.1_
  - _writes: crates/sim_game/src/formats.rs, crates/sim_game/src/imports.rs, crates/sim_game/src/generated_assets.rs_
