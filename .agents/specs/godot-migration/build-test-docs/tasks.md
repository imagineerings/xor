# Implementation Plan: Build, Test, and Documentation

## Overview

Add shared documentation, fixture attribution, and dependency review helpers before feature specs copy source fixtures or introduce heavy dependencies.

## Gates

- Start gate: G0 spec consistency passes for the umbrella and grouped specs.
- Validation gate: docs metadata, fixture attribution, and dependency review helper tests pass.
- Handoff gate: dependency review records include license, maintenance, security, binary-size, and platform-impact fields.
- Completion gate: G7 dependency review is available before any task adds vendored, native, codec, model, media, or mesh dependencies.

## Dependency Waves

- W1 Shared foundations: fixture attribution and dependency review helpers land first when another task needs them.
- W7 Deferred Godot-origin compatibility: docs and compatibility metadata integrations depend on W1 helpers and start only when they unlock target-product work.

## Tasks

- [ ] 1. Add docs, fixture attribution, and dependency review helpers
  - Implement native Zed docs metadata ingestion, fixture attribution validation, and dependency review records.
  - Recreate Comfy-era generated fixture attribution as native Zed generated asset metadata, not as a thin compatibility label or pass-through.
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 3.1, 3.2, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: tooling/compliance/src/lib.rs, script/check-licenses, script/generate-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/build-test-docs/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/build-test-docs/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/build-test-docs; run build/test/docs/license/dependency checks without Godot and inspect process, package, loader, and dependency graphs_

- [ ] 2. Prove Godot remains evidence-only and unlinked
  - Add hermetic scans for Godot executables, libraries, generators, servers, copied source, vendor material, process invocation, linkage, package contents, and unapproved fixture/doc/assets; verify every approved copy has a review record.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: projects/godot/COPYRIGHT.txt, projects/godot/thirdparty/README.md, Cargo.toml, Cargo.lock, deny.toml, script/check-licenses, tooling/compliance/src/lib.rs_
  - _Writes: tooling/compliance/src/lib.rs, .agents/specs/godot-migration/godot-full-port-coverage/findings.md, .agents/specs/godot-migration/godot-full-port-coverage/decisions.md_
  - _Validation: execute Zed build/test/docs/compliance/package checks on a machine without Godot and assert zero unapproved copied material, Godot process, library, server, CLI, linkage, or runtime dependency_
