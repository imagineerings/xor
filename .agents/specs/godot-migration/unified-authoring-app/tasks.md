# Implementation Plan: Unified Authoring App

## Dependency Gates

- **Primary wave**: W4 Authoring and graph UX
- **Prerequisite gates**: G0 Spec consistency, G2 Shared Godot metadata, G3 Shared world-model foundations
- **Preview gates**: G4 Worker safety for interactive world-model previews; G6 Provenance for generated artifact views

## Tasks

- [ ] 1. Add unified game authoring app model
  - Register the app, define authoring items, route previews, and surface generated artifacts.
  - _Requirements: 1.1, 1.2, 2.1, 2.2_
  - _writes: crates/baymax_apps/src/game_authoring.rs, crates/baymax_apps/src/game_authoring_tests.rs_
