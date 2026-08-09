# Implementation Plan: Networking and Collaboration

## Overview

Keep Godot-origin networking as native Sim boundary metadata and optional debug metadata. Runtime networking remains excluded unless represented by native Sim task/debug metadata that directly supports the target generative game engine.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: networking boundary, debug metadata, and non-migration tests pass.
- Handoff gate: unsupported runtime networking features produce explicit boundary diagnostics.
- Completion gate: no Godot-specific network runtime or protocol adapter is added; native gameplay protocol work requires G7 review and an explicit architecture decision at existing Sim owners.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: debug metadata and native task/debug integration wait for boundary policy, metadata gates, and an explicit product-enabling dependency.

## Tasks

- [ ] 1. Add networking boundary and debug metadata support
  - Encode non-migration decisions and model optional native Sim debug metadata for task/debug workflows.
  - _Requirements: 1.1, 1.2, 2.1, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/net/src/net.rs, crates/http_client/src/http_client.rs, crates/rpc/src/rpc.rs, crates/collab/src/lib.rs, crates/dap/src/dap.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/networking-collaboration/requirements.md, /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/networking-collaboration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/networking-collaboration; run supported network/debug scenarios without Godot and inspect processes, endpoints, packages, and dependencies_

- [ ] 2. Prove native networking ownership without Godot
  - Add hermetic supported, excluded, failure, timeout, cancellation, limit, cleanup, process, endpoint, and linkage validation.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/net/src/net.rs, crates/http_client/src/http_client.rs, crates/rpc/src/rpc.rs, crates/collab/src/lib.rs, crates/dap/src/dap.rs_
  - _Writes: crates/net/src/net.rs, crates/http_client/src/http_client.rs, crates/rpc/src/rpc.rs, crates/dap/src/dap.rs_
  - _Validation: execute native scenarios on a machine without Godot and assert no Godot process, library, server, CLI, hidden endpoint, or runtime dependency_
