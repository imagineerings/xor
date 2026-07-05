# Implementation Plan: Unified Authoring App

## Overview

Build a unified authoring workspace over Godot metadata, graph workflows, world-model requests, generated artifacts, and external run/export tasks.

## Gates

- Start gate: G0 spec consistency, G2 shared Baymax game metadata, G3 shared world-model foundations, and applicable G8 Comfy workflow alignment decisions are satisfied.
- Validation gate: app registration, authoring item routing, preview routing, and generated artifact tests pass.
- Handoff gate: unsupported previews and unavailable workers produce actionable diagnostics in the workspace model.
- Completion gate: interactive previews require G4 worker safety, generated artifact views require G6 provenance, and UI work delegates graph editing and workflow metadata to their owning specs.

## Dependency Waves

- W4 Authoring, graph UX, and Comfy workflows: authoring workspace routing depends on G2 and G3.

## Tasks

- [ ] 1. Add unified game authoring app model
  - Register the app, define authoring items, route previews, and surface generated artifacts.
  - _Requirements: 1.1, 1.2, 2.1, 2.2_
  - _writes: crates/baymax_apps/src/game_authoring.rs, crates/baymax_apps/src/game_authoring_tests.rs_
