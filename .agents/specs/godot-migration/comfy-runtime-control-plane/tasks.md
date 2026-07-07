# Implementation Plan: Comfy Runtime Control Plane

## Overview

Build the control plane as a protocol adapter over Sim task, HTTP, WebSocket, media, and project systems. Wave 1 defines protocol models and safety checks. Wave 2 wires queue/job behavior. Wave 3 adds realtime events and compatibility fixtures.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations for artifact references, G8 Comfy harness alignment, and no pending contradiction with `diffusion-graph-editor/`.
- Validation gate: route unit tests, WebSocket integration tests, and Comfy script fixture compatibility tests pass.
- Handoff gate: document unsupported Comfy routes and response-shape gaps in test snapshots.
- Completion gate: every public response is redacted, path confined, and backed by Sim-owned task or artifact state.

## Dependency Waves

- Global wave: W4 Authoring, graph UX, and Comfy workflows.
- Local Wave 1: Tasks 1-2 can start first and may run in parallel.
- Local Wave 2: Tasks 3-5 depend on route models and safety primitives.
- Local Wave 3: Tasks 6-7 depend on queue/job bridge and event translation.

## Tasks

- [ ] 1. Define Comfy control-plane protocol models
  - Add prompt submission, queue action, history action, job summary, feature flag, and runtime event types.
  - _Requirements: 1.1, 1.2, 2.1, 3.2_
  - _writes: crates/world_model/src/comfy_control.rs, crates/world_model/src/comfy_control_tests.rs_

- [ ] 2. Implement HTTP safety primitives
  - Add origin checks, CSP mode selection, safe content-type handling, cache-control classification, and path confinement helpers.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_
  - _writes: crates/world_model/src/comfy_http_safety.rs, crates/world_model/src/comfy_http_safety_tests.rs_

- [ ] 3. Register Comfy-compatible route aliases
  - Register legacy and `/api` paths for prompt, queue, history, jobs, features, model catalog, object info, upload, and view handlers.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 5.1_
  - _writes: crates/world_model/src/comfy_routes.rs, crates/sim/src/sim.rs, crates/world_model/src/comfy_routes_tests.rs_

- [ ] 4. Implement the Sim job bridge
  - Map prompt submission, queue snapshots, history reads, job listing, sorting, filtering, and sensitive-data redaction onto Sim jobs.
  - _Requirements: 2.2, 2.5, 5.2_
  - _writes: crates/world_model/src/comfy_jobs.rs, crates/world_model/src/comfy_jobs_tests.rs_

- [ ] 5. Implement idempotent cancellation and targeted interrupt
  - Support single and batch cancellation, terminal no-ops, unknown no-ops, pending dequeue, and running interrupt by prompt id.
  - _Requirements: 2.3, 2.4_
  - _writes: crates/world_model/src/comfy_cancellation.rs, crates/world_model/src/comfy_cancellation_tests.rs_

- [ ] 6. Implement WebSocket session and event translation
  - Add session ids, feature flag negotiation, initial queue status, status events, executing events, progress events, and preview event selection.
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _writes: crates/world_model/src/comfy_ws.rs, crates/world_model/src/comfy_events.rs, crates/world_model/src/comfy_ws_tests.rs_

- [ ] 7. Add Comfy API compatibility fixtures
  - Convert basic HTTP and WebSocket script examples into automated compatibility tests.
  - _Requirements: 1.1, 1.4, 3.1, 3.3, 5.3_
  - _writes: crates/world_model/tests/comfy_api_compat.rs, crates/world_model/fixtures/comfy/basic_api_prompt.json_

## Notes

- Do not fork `aiohttp`; this is a protocol adapter over Sim runtime services.
- Do not implement graph validation here; call the graph/node runtime validator.
