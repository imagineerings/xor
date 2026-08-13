# Implementation Plan: Collaborative Workspace and Buzz Consolidation

## Approach

The approved capability epics are parent checkboxes grouped under milestone headings. Executable work appears only as indented implementation leaves. Epic IDs are stable global integers, and leaf IDs use the `epic.leaf` form; for example, `8.1` is the first leaf of epic 8. Every leaf delivers one behavior, adapter, migration, UI component, fixture or operational artifact and is sized for 0.5–3 agent-days including focused tests.

Milestone 1 remains an end-to-end GPUI slice over existing Sim state before server authority changes. Later work moves aggregate authority behind approved adapters and migration gates. Production deployment, irreversible deletion, source removal and traffic cutover remain separately authorized operations.

## Plan summary

| Milestone | Leaf tasks | Estimated agent-days | Principal outcome |
| --- | ---: | ---: | --- |
| 0 — evidence and decisions | 23 | 35 | Reproducible inventory, ADRs, baselines and threat model |
| 1 — native vertical slice | 34 | 51 | Reversible Collaborative Workspace over existing project/ACP/Git state |
| 2 — protocol and service foundations | 53 | 95 | Canonical domain, identity, tenant, protocol, persistence and import foundations |
| 3 — communication parity | 47 | 82 | Channels, messages, DMs, awareness, search, notifications and social surfaces |
| 4 — project and Git collaboration | 27 | 49 | NIP-MP/NIP-34, branch channels, review and CI linkage |
| 5 — agent convergence | 37 | 69 | ACP/MCP, personas, memory, jobs, activity and remote execution |
| 6 — platform parity | 57 | 106 | Workflows, audit, administration, deletion, media, huddles, pairing and mesh |
| 7 — clients, operations and retirement | 52 | 95 | Client compatibility, release readiness, cutover, retirement and parity proof |
| **Total** | **330** | **582** | Complete approved migration scope |

The dependency graph has an estimated **310 agent-day critical path** from inventory and ADR approval through domain/auth/storage, messaging, agent/workflow convergence, compatibility gates, cutover and retirement. With four stable workstreams and prompt reviews, implementation work is approximately 10–16 elapsed months; required observation windows, external client certification and production approvals can extend calendar delivery. A single sequential agent is approximately 582 working days before review/rework allowance.

No leaf is intentionally larger than three agent-days. Cross-system scenarios are split into fixture construction, implementation, and execution/reporting leaves. If a leaf exceeds that bound during implementation, it must be split before code review without changing its epic scope.

## Dependency waves and parallel-safe workstreams

- **Wave 0 / Milestone 0:** inventory generation, independent fixtures and ADR evidence may proceed in parallel; the security review follows the ownership decisions and baselines.
- **Wave 1 / Milestone 1:** onboarding/settings, shell composition and activity fixtures converge into one vertical slice. Shared workspace files are serialized in numeric dependency order.
- **Wave 2 / Milestone 2:** protocol codecs, identity, tenant admission, service adapters, persistence and importers. ADR-001 gates 14.1 and 15.1; ADR-002 gates 12.1.
- **Wave 3 / Milestone 3:** channel/message foundations precede DMs and awareness. Search, desktop notifications and push are separate workstreams after projections exist.
- **Wave 4 / Milestone 4:** project binding can start after channels; forge hosting is gated by ADR-003; review presentation follows branch/Git event models.
- **Wave 5 / Milestone 5:** ACP ingress, persona state and remote-provider conformance can overlap after identity and messages; memory and jobs serialize on their owning stores.
- **Wave 6 / Milestone 6:** workflow/audit/admin/deletion form one chain. Media, pairing and mesh are parallel-safe; huddles are gated by ADR-004, push cutover by ADR-005 and mesh execution by ADR-006.
- **Wave 7 / Milestone 7:** CLI, web, mobile and admin clients are separate workstreams. Deployment and compatibility gates converge before aggregate cutover and retirement.

Parallel-safe workstreams after their stated prerequisites are:

- **Native product:** workspace/onboarding, navigation/timeline, Git review, settings/accessibility and later collaboration GPUI surfaces; serialize only on the shared-write chains below.
- **Protocol and service:** pure domain/codecs, tenant/auth, Nostr ingress, event persistence, projections and realtime/search; schema owners land before their repositories and adapters.
- **Agent and remote execution:** ACP/MCP compatibility, personas/private state, jobs/delegation and remote-provider/mesh work; never run two executor owners for one job/session.
- **Companion clients:** CLI, web, mobile and admin migrations proceed independently after the compatibility endpoint and their capability-specific server owners exist.
- **Operations and evidence:** fixtures, threat models, deployment packaging, conformance, load/fault gates and cutover rehearsals remain independent of production reducers and production mutation.

## Approval-gated leaves

| Decision | Leaves blocked pending approval |
| --- | --- |
| ADR-001 service/database topology | 2.1, 14.1, 15.1, 44.1 |
| ADR-002 account/Nostr binding | 2.2, 12.1, 12.2, 12.3 |
| ADR-003 hosted Git authority | 2.3, 25.1, 25.2, 25.3 |
| ADR-004 native huddle transport | 2.4, 39.1, 39.2, 39.3 |
| ADR-005 push platforms | 2.5, 22.9, 43.7 |
| ADR-006 shared-compute policy | 2.6, 41.1, 41.2, 41.3 |

## Shared-write sequencing

- Workspace presentation files serialize through 5.1 → 5.2 → 5.3 → 6.1 → 6.2 → 7.1 → 9.1 → 10.1.
- Activity projection files serialize through 8.1 → 8.2 → 8.3 → 8.4 → 32.1 → 32.2 → 32.3 → 32.4.
- Collaboration schemas serialize through 15.1 → 15.2 → 15.3 → 18.1 → 19.1 and then aggregate-specific migrations.
- Channel store integration serializes through 18.4 → 21.1 → 26.1.
- Git review integration serializes through 9.1 → 9.2 → 27.1 → 27.2 → 27.3.
- Agent stores serialize through 29.1 → 29.2 → 30.1 → 31.1; remote execution follows 33.1 → 33.2.
- Administrative schema and APIs serialize through 35.1 → 36.1 → 36.2 → 37.1.
- Client compatibility documents serialize through 43.1 → 43.2 → 43.3 → 43.4 → 43.5; final architecture evidence follows 48.1 → 48.4.

## Tasks

## Milestone 0 — establish evidence and decisions

- [ ] 1. Generate and enforce the Buzz coverage ledger

  - [ ] 1.1. Generate the Buzz Rust-package catalog
    - Enumerate workspace members, manifests, binaries and feature flags with stable CAP mappings.
    - _Requirements: 1.1, 1.2_
    - _Capability IDs: CAP-001, CAP-043, CAP-044_
    - _Depends on: none_
    - _Reads: projects/buzz/Cargo.toml, projects/buzz/crates/*/Cargo.toml_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/buzz-packages.csv_
    - _Validation: `python3 .agents/specs/collaborative-workspace/scripts/check-inventory.py --catalog packages` reports every workspace member_

  - [ ] 1.2. Generate the event-kind and NIP catalog
    - Extract registered kinds and standard/custom protocol documents into stable protocol rows.
    - _Requirements: 1.1, 1.2, 5.1_
    - _Capability IDs: CAP-001, CAP-002, CAP-044_
    - _Depends on: none_
    - _Reads: projects/buzz/crates/buzz-core/src/kind.rs, projects/buzz/docs/nips/*, projects/buzz/NOSTR.md_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/protocol.csv_
    - _Validation: inventory checker reports all registered constants and NIP files exactly once_

  - [ ] 1.3. Generate the data and migration catalog
    - Enumerate SQL migrations, schemas, object stores, Redis state and desktop persistence sources.
    - _Requirements: 1.1, 17.1_
    - _Capability IDs: CAP-005, CAP-030, CAP-045_
    - _Depends on: none_
    - _Reads: projects/buzz/migrations/*, projects/buzz/schema/**, projects/buzz/desktop/src-tauri/src/{migration,archive,event?sync}/**_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/data-sources.csv_
    - _Validation: catalog check accounts for all 30 SQL migrations and every discovered durable store_

  - [ ] 1.4. Generate client, desktop and deployment catalogs
    - Enumerate Tauri modules, desktop features, client routes, charts, workflows, scripts, examples and benchmarks.
    - _Requirements: 1.1, 1.2_
    - _Capability IDs: CAP-036, CAP-038, CAP-039, CAP-040, CAP-041, CAP-043, CAP-044_
    - _Depends on: none_
    - _Reads: projects/buzz/desktop/src/**, projects/buzz/mobile/lib/**, projects/buzz/web/src/**, projects/buzz/admin-web/**, projects/buzz/deploy/**, projects/buzz/.github/workflows/**_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/surfaces.csv_
    - _Validation: inventory checker reports no unmapped feature, route, deployment component or test surface_

  - [ ] 1.5. Enforce inventory drift in repository checks
    - Add one checker that joins all catalogs to CAP, requirement, owner and leaf-task references and fails on omissions.
    - _Requirements: 1.2, 1.3, 1.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 1.1, 1.2, 1.3, 1.4_
    - _Reads: .agents/specs/collaborative-workspace/catalogs/**, .agents/specs/collaborative-workspace/{source-inventory,reuse-audit,requirements,tasks}.md_
    - _Writes: .agents/specs/collaborative-workspace/scripts/check-inventory.py, script/check-collaborative-workspace-inventory_
    - _Validation: a temporary unmapped fixture makes the checker fail with its exact source path and missing references_

- [ ] 2. Record canonical ownership and architecture decisions

  - [ ] 2.1. Decide ADR-001 service and database topology
    - Record final process, schema and dependency-version ownership plus the bounded sidecar exit conditions.
    - _Requirements: 2.1, 2.2, 2.3_
    - _Capability IDs: CAP-003, CAP-005, CAP-043_
    - _Depends on: 1.1, 1.3_
    - _Reads: .agents/specs/collaborative-workspace/reuse-audit.md, crates/collab/**, projects/buzz/ARCHITECTURE.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-001-service-topology.md_
    - _Validation: architecture review records one migration authority and explicit sidecar removal gates_

  - [ ] 2.2. Decide ADR-002 account and Nostr identity binding
    - Record binding cardinality, verification, recovery, rotation and organization policy.
    - _Requirements: 2.1, 7.1, 7.4_
    - _Capability IDs: CAP-007, CAP-008, CAP-009_
    - _Depends on: 1.2_
    - _Reads: .agents/specs/collaborative-workspace/reuse-audit.md, crates/client/src/user.rs, projects/buzz/docs/nips/NIP-OA.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-002-identity-binding.md_
    - _Validation: identity review covers create, link, rotate, revoke, archive and recovery without ambiguous authority_

  - [ ] 2.3. Decide ADR-003 hosted Git authority
    - Choose authority boundaries between NIP-34 hosting, external providers and local Sim Git.
    - _Requirements: 2.1, 10.1, 10.2_
    - _Capability IDs: CAP-018, CAP-019, CAP-020_
    - _Depends on: 1.2_
    - _Reads: .agents/specs/collaborative-workspace/reuse-audit.md, crates/git_hosting_providers/**, projects/buzz/docs/git-on-object-storage.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-003-git-authority.md_
    - _Validation: decision table assigns one authority for working state, hosted refs, patches and review records_

  - [ ] 2.4. Decide ADR-004 huddle transport
    - Select native transport and define the Buzz audio compatibility support window.
    - _Requirements: 2.1, 14.3, 14.4_
    - _Capability IDs: CAP-032_
    - _Depends on: 1.4_
    - _Reads: crates/livekit_api/**, crates/livekit_client/**, projects/buzz/crates/buzz-relay/src/audio/**_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-004-huddle-transport.md_
    - _Validation: review records lifecycle parity, platform support and adapter retirement criteria_

  - [ ] 2.5. Decide ADR-005 push platform scope
    - Record required push platforms, attestation requirements and the first mobile-cutover support floor.
    - _Requirements: 2.1, 9.5, 19.2_
    - _Capability IDs: CAP-016_
    - _Depends on: 1.4_
    - _Reads: projects/buzz/crates/buzz-push-gateway/**, .agents/specs/collaborative-workspace/reuse-audit.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-005-push-scope.md_
    - _Validation: approval records supported targets, attestations, fallback and compatibility floor_

  - [ ] 2.6. Decide ADR-006 shared-compute policy
    - Record mesh trust, eligibility, resources, fairness, fallback and deployment policy.
    - _Requirements: 2.1, 16.3, 19.2_
    - _Capability IDs: CAP-035_
    - _Depends on: 1.4_
    - _Reads: projects/buzz/crates/buzz-relay-mesh/**, .agents/specs/collaborative-workspace/reuse-audit.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-006-shared-compute.md_
    - _Validation: approval records fail-closed eligibility, resource limits, fairness and no-silent-fallback rules_

- [ ] 3. Capture independent compatibility and behavior baselines

  - [ ] 3.1. Freeze signed-event and relay protocol fixtures
    - Capture valid, malformed, replaceable, privacy-gated and mixed-version event traces without production reducers.
    - _Requirements: 5.1, 5.2, 5.3, 20.2_
    - _Capability IDs: CAP-001, CAP-002, CAP-004, CAP-044_
    - _Depends on: 1.2_
    - _Reads: projects/buzz/crates/buzz-test-client/**, projects/buzz/crates/buzz-conformance/**_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/protocol/*_
    - _Validation: independent trace checker accepts valid fixtures and rejects each malformed fixture_

  - [ ] 3.2. Freeze CLI and companion-client contract fixtures
    - Capture command output, exit codes, routes, deep links, negotiation and background lifecycle contracts.
    - _Requirements: 16.4, 18.1, 20.1_
    - _Capability IDs: CAP-038, CAP-039, CAP-040, CAP-041, CAP-042_
    - _Depends on: 1.4_
    - _Reads: projects/buzz/crates/buzz-cli/**, projects/buzz/mobile/test/**, projects/buzz/web/**, projects/buzz/admin-web/**_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/clients/*_
    - _Validation: fixture manifest identifies client version, input, expected output and authority for every captured contract_

  - [ ] 3.3. Freeze migration and archive fixtures
    - Build sanitized fixtures for every SQL and desktop stored-data version with counts and integrity hashes.
    - _Requirements: 17.1, 17.2, 20.1_
    - _Capability IDs: CAP-005, CAP-024, CAP-030, CAP-045_
    - _Depends on: 1.3_
    - _Reads: projects/buzz/migrations/**, projects/buzz/desktop/src-tauri/src/{migration,archive,managed?agents}/**_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/migrations/*_
    - _Validation: fixture index covers every stored version and verifies hashes without private key material_

  - [ ] 3.4. Freeze performance and known-gap baselines
    - Record relay, fan-out, search, push, workflow, mesh and orchestration measurements plus documented incomplete behavior.
    - _Requirements: 1.3, 20.1, 20.3_
    - _Capability IDs: CAP-006, CAP-015, CAP-016, CAP-027, CAP-035, CAP-044_
    - _Depends on: 1.1, 1.4_
    - _Reads: projects/buzz/benchmarks/**, projects/buzz/perf/**, projects/buzz/TESTING.md, projects/buzz/VISION*.md_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/baselines.md_
    - _Validation: baseline document records command, environment, result budget and known defect for each subsystem_

- [ ] 4. Complete the cross-boundary threat and operations review

  - [ ] 4.1. Threat-model tenant, identity and protocol boundaries
    - Enumerate host confusion, replay, signing-key, authorization-before-limit and metadata leak threats with owners.
    - _Requirements: 6.3, 19.1, 19.2_
    - _Capability IDs: CAP-001, CAP-003, CAP-007, CAP-008, CAP-009_
    - _Depends on: 2.1, 2.2, 3.1_
    - _Reads: projects/buzz/SECURITY.md, projects/buzz/docs/multi-tenant-relay.md, .agents/specs/collaborative-workspace/decisions/**_
    - _Writes: .agents/specs/collaborative-workspace/security/tenant-identity.md_
    - _Validation: review maps each threat to a fail-closed control and negative test leaf_

  - [ ] 4.2. Threat-model agents, providers and MCP
    - Cover hostile provider output, subprocess cleanup, tool permissions and secret separation.
    - _Requirements: 11.1, 11.5, 19.1, 19.2_
    - _Capability IDs: CAP-021, CAP-022, CAP-034_
    - _Depends on: 3.4_
    - _Reads: projects/buzz/docs/remote-agents.md, .agents/specs/goose-migration/security-permissions/**_
    - _Writes: .agents/specs/collaborative-workspace/security/agent-workflow.md_
    - _Validation: security checklist assigns bounded input/output, cancellation and permission tests to every executor boundary_

  - [ ] 4.3. Threat-model media storage and rendering
    - Cover object paths, MIME confusion, decompression, previews, credentials and orphan cleanup.
    - _Requirements: 14.1, 14.2, 19.1, 19.2_
    - _Capability IDs: CAP-031_
    - _Depends on: 3.4_
    - _Reads: projects/buzz/crates/buzz-media/**, crates/media/**_
    - _Writes: .agents/specs/collaborative-workspace/security/media.md_
    - _Validation: review maps media abuse cases and resource bounds to upload, storage and rendering tests_

  - [ ] 4.4. Define operational limits and telemetry constraints
    - Set measurable connection, frame, queue, retry, freshness, migration, logging and telemetry-disabled expectations.
    - _Requirements: 8.4, 19.3, 19.5_
    - _Capability IDs: CAP-004, CAP-006, CAP-028, CAP-043_
    - _Depends on: 4.1, 4.2, 4.3, 4.5, 4.6, 4.7, 4.8_
    - _Reads: .agents/specs/collaborative-workspace/security/**, .agents/specs/telemetry-disabled-default/**, projects/buzz/deploy/**_
    - _Writes: .agents/specs/collaborative-workspace/security/operational-limits.md_
    - _Validation: every limit has an owner, metric, alert threshold and focused verification task_

  - [ ] 4.5. Threat-model workflow and webhook execution
    - Cover webhook authentication, SSRF/redirects, conditions, retries, actions, secrets and approval bypass.
    - _Requirements: 13.2, 13.3, 19.1, 19.2_
    - _Capability IDs: CAP-027_
    - _Depends on: 3.4_
    - _Reads: projects/buzz/crates/buzz-workflow/**, projects/buzz/crates/buzz-relay/src/workflow-sink.rs_
    - _Writes: .agents/specs/collaborative-workspace/security/workflow.md_
    - _Validation: review maps each trigger/action/approval threat to a bounded negative or recovery test_

  - [ ] 4.6. Threat-model push delivery
    - Cover lease capability, wake privacy, endpoint authority, amplification, provider errors and queue bounds.
    - _Requirements: 9.5, 19.1, 19.2_
    - _Capability IDs: CAP-016_
    - _Depends on: 2.5, 3.4_
    - _Reads: projects/buzz/crates/buzz-push-gateway/**, projects/buzz/docs/nips/NIP-PL.md_
    - _Writes: .agents/specs/collaborative-workspace/security/push.md_
    - _Validation: review proves payload minimization and assigns lease, amplification, retry and redaction tests_

  - [ ] 4.7. Threat-model voice and huddles
    - Cover audio authorization, devices, transcript privacy, model files, transport failure and resource cleanup.
    - _Requirements: 14.3, 14.4, 19.1, 19.2_
    - _Capability IDs: CAP-032_
    - _Depends on: 2.4, 3.4_
    - _Reads: projects/buzz/crates/buzz-voice/**, projects/buzz/crates/buzz-relay/src/audio/**_
    - _Writes: .agents/specs/collaborative-workspace/security/huddle.md_
    - _Validation: review maps audio/transcript/model threats to authorization, failure and cleanup tests_

  - [ ] 4.8. Threat-model relay mesh and shared compute
    - Cover peer authentication, replay, stale membership, resource claims, scheduling and unapproved fallback.
    - _Requirements: 16.3, 19.1, 19.2_
    - _Capability IDs: CAP-035_
    - _Depends on: 2.6, 3.4_
    - _Reads: projects/buzz/crates/buzz-relay-mesh/**, projects/buzz/VISION_MESH.md_
    - _Writes: .agents/specs/collaborative-workspace/security/mesh.md_
    - _Validation: review assigns replay, revocation, resource, fairness and no-fallback tests_

## Milestone 1 — ship the native collaborative vertical slice

- [ ] 5. Add reversible workspace presentation selection

  - [ ] 5.1. Define the workspace-presentation setting
    - Add the Editor and Collaborative enum, default and settings schema without changing current startup behavior.
    - _Requirements: 3.1, 3.4_
    - _Capability IDs: CAP-037_
    - _Depends on: 1.5_
    - _Reads: crates/workspace/src/workspace_settings.rs, crates/settings/**_
    - _Writes: crates/workspace/src/workspace_presentation.rs_
    - _Validation: `cargo test -p workspace workspace_presentation_setting` covers default and deserialization_

  - [ ] 5.2. Persist workspace presentation across restart
    - Store and restore the selected presentation in existing workspace persistence without copying project state.
    - _Requirements: 3.2, 3.3_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 5.1_
    - _Reads: crates/workspace/src/{persistence,workspace-presentation}.rs_
    - _Writes: crates/workspace/src/persistence.rs_
    - _Validation: `cargo test -p workspace workspace_presentation_restart` verifies both modes and unchanged project identity_

  - [ ] 5.3. Add presentation switching actions
    - Register reversible actions that recompose the active workspace while retaining canonical entities.
    - _Requirements: 3.3_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 5.2_
    - _Reads: crates/workspace/src/{workspace,workspace-presentation}.rs, crates/workspace/src/actions.rs_
    - _Writes: crates/workspace/src/workspace_presentation_actions.rs, crates/workspace/src/actions.rs_
    - _Validation: `cargo test -p workspace switch_workspace_presentation` checks entity IDs and navigation survive both transitions_

  - [ ] 5.4. Add onboarding workspace-choice controls
    - Render two accessible choices with concise shared-data explanation and save the chosen setting.
    - _Requirements: 3.1, 3.2_
    - _Capability IDs: CAP-037_
    - _Depends on: 5.1_
    - _Reads: crates/onboarding/src/{onboarding,basics-page}.rs, crates/ui/src/**_
    - _Writes: crates/onboarding/src/workspace_choice.rs_
    - _Validation: `cargo test -p onboarding workspace_choice` verifies labels, selection, keyboard activation and persisted value_

  - [ ] 5.5. Cover existing-user and initialization failure behavior
    - Prove existing users remain in Editor and a failed Collaborative initialization offers a recoverable Editor fallback.
    - _Requirements: 3.3, 3.4_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 5.3, 5.4_
    - _Reads: crates/onboarding/src/workspace_choice.rs, crates/workspace/src/workspace_presentation.rs_
    - _Writes: crates/workspace/tests/workspace_presentation.rs_
    - _Validation: `cargo test -p workspace workspace_presentation` covers upgrade, failure, retry and explicit later switch_

- [ ] 6. Compose the collaborative GPUI shell

  - [ ] 6.1. Add the CollaborativeWorkspace composition root
    - Create the native GPUI view and select it from the approved presentation setting.
    - _Requirements: 4.1_
    - _Capability IDs: CAP-036_
    - _Depends on: 5.3_
    - _Reads: crates/workspace/src/{workspace,pane,dock,status-bar}.rs, crates/ui/src/**_
    - _Writes: crates/workspace/src/collaborative_workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_workspace_mounts` proves no React or Tauri process is launched_

  - [ ] 6.2. Implement the collaborative top-bar layout
    - Compose title, participant region, share/invite actions and connection/layout affordances with native components.
    - _Requirements: 4.1, 4.4_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1_
    - _Reads: crates/workspace/src/collaborative_workspace.rs, crates/title_bar/**, crates/ui/src/**_
    - _Writes: crates/workspace/src/collaborative_top_bar.rs_
    - _Validation: `cargo test -p workspace collaborative_top_bar` checks hierarchy, labels and unavailable-action states_

  - [ ] 6.3. Implement the left-rail layout container
    - Add pinned, community/project and task/thread sections with independent scrolling and native density tokens.
    - _Requirements: 4.1, 4.2_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1_
    - _Reads: crates/sidebar/src/sidebar.rs, crates/workspace/src/collaborative_workspace.rs, crates/ui/src/**_
    - _Writes: crates/sidebar/src/collaborative_rail.rs_
    - _Validation: `cargo test -p sidebar collaborative_rail_layout` checks section order and bounded scrolling_

  - [ ] 6.4. Implement timeline and review split geometry
    - Add resizable central/review regions with minimum sizes and a full-width collapsed timeline state.
    - _Requirements: 4.1, 4.2_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1_
    - _Reads: crates/workspace/src/{collaborative_workspace,pane}.rs, crates/ui/src/resizable.rs_
    - _Writes: crates/workspace/src/collaborative_layout.rs_
    - _Validation: `cargo test -p workspace collaborative_layout_bounds` checks expanded, collapsed and narrow constraints_

  - [ ] 6.5. Persist collaborative layout state
    - Persist review visibility, width and collaborative rail width independently of Editor layout state.
    - _Requirements: 4.2, 4.3_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.4_
    - _Reads: crates/workspace/src/{collaborative_layout,persistence}.rs_
    - _Writes: crates/workspace/src/collaborative_layout_persistence.rs_
    - _Validation: `cargo test -p workspace collaborative_layout_restart` verifies round trip, bounds clamping and Editor isolation_

  - [ ] 6.6. Add shell loading and initialization-error states
    - Render bounded loading, unavailable-service and retry states without discarding presentation or project context.
    - _Requirements: 4.1, 8.3_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1, 6.2, 6.3, 6.4_
    - _Reads: crates/workspace/src/collaborative_workspace.rs, crates/ui/src/**_
    - _Writes: crates/workspace/src/collaborative_shell_state.rs_
    - _Validation: `cargo test -p workspace collaborative_shell_state` covers loading, retry, partial failure and recovery_

- [ ] 7. Bind collaborative navigation to existing stores

  - [ ] 7.1. Define collaborative navigation row projection
    - Project existing entities into stable row IDs, groups and state badges without creating another store.
    - _Requirements: 4.3_
    - _Capability IDs: CAP-036, CAP-042_
    - _Depends on: 6.3_
    - _Reads: crates/sidebar/src/collaborative_rail.rs, crates/project/src/**, crates/channel/src/channel_store.rs, crates/agent_ui/src/thread_metadata_store.rs_
    - _Writes: crates/sidebar/src/collaborative_navigation.rs_
    - _Validation: `cargo test -p sidebar collaborative_navigation_projection` verifies stable IDs and one row per source entity_

  - [ ] 7.2. Populate pinned and recent work groups
    - Bind existing pinned/recent project and task records with empty and unavailable states.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-036_
    - _Depends on: 7.1_
    - _Reads: crates/recent_projects/src/**, crates/sidebar/src/collaborative_navigation.rs_
    - _Writes: crates/sidebar/src/collaborative_pinned.rs_
    - _Validation: `cargo test -p sidebar collaborative_pinned` covers order, removal and missing targets_

  - [ ] 7.3. Populate project, repository and worktree groups
    - Render canonical Sim project hierarchy and selection without deriving duplicate project records.
    - _Requirements: 3.3, 4.3_
    - _Capability IDs: CAP-018, CAP-036_
    - _Depends on: 7.1_
    - _Reads: crates/project/src/**, crates/worktree/src/**, crates/sidebar/src/collaborative_navigation.rs_
    - _Writes: crates/sidebar/src/collaborative_projects.rs_
    - _Validation: `cargo test -p sidebar collaborative_projects` checks multiple repositories/worktrees and deleted worktrees_

  - [ ] 7.4. Populate task and thread history groups
    - Bind active, historical and archived ACP/thread metadata with running, waiting, failed and completed indicators.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-021, CAP-025, CAP-036_
    - _Depends on: 7.1_
    - _Reads: crates/agent_ui/src/thread_metadata_store.rs, crates/task/**, crates/sidebar/src/collaborative_navigation.rs_
    - _Writes: crates/sidebar/src/collaborative_tasks.rs_
    - _Validation: `cargo test -p sidebar collaborative_tasks` checks state transitions, archive and history ordering_

  - [ ] 7.5. Add selection, history and deep-link navigation
    - Route row activation and supported entity links through existing workspace navigation and persist selected context.
    - _Requirements: 4.3, 16.4_
    - _Capability IDs: CAP-036, CAP-042_
    - _Depends on: 7.2, 7.3, 7.4_
    - _Reads: crates/workspace/src/path_link.rs, crates/sidebar/src/collaborative_{pinned,projects,tasks}.rs_
    - _Writes: crates/workspace/src/collaborative_navigation.rs_
    - _Validation: `cargo test -p workspace collaborative_navigation` covers back/forward, restart, missing entity and unsafe link rejection_

- [ ] 8. Project existing ACP activity into the central timeline

  - [ ] 8.1. Define the ActivityItem projection contract
    - Add stable source identity, semantic class, actor/verb/object/outcome, lifecycle and detail-link fields without GPUI dependencies.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-025, CAP-036_
    - _Depends on: 3.1, 6.1_
    - _Reads: crates/acp_thread/src/acp_thread.rs, crates/action_log/src/**, projects/buzz/VISION_ACTIVITY.md_
    - _Writes: crates/agent_ui/src/activity_projection.rs_
    - _Validation: `cargo test -p agent_ui activity_projection_contract` covers stable identity and serializable detail handles_

  - [ ] 8.2. Map ACP messages and lifecycle events
    - Project human/agent messages, session start/stop, idle, disconnect and cancellation into one item each.
    - _Requirements: 12.1, 12.4_
    - _Capability IDs: CAP-021, CAP-025_
    - _Depends on: 8.1_
    - _Reads: crates/acp_thread/src/**, crates/agent_ui/src/activity_projection.rs_
    - _Writes: crates/agent_ui/src/activity_acp.rs_
    - _Validation: `cargo test -p agent_ui activity_acp_mapping` exhausts ACP message and lifecycle fixtures_

  - [ ] 8.3. Map native tool and permission activity
    - Project reads, searches, edits, shell commands, tests and permission requests with truthful outcomes.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-022, CAP-025_
    - _Depends on: 8.1_
    - _Reads: crates/action_log/src/**, crates/agent_ui/src/activity_projection.rs_
    - _Writes: crates/agent_ui/src/activity_actions.rs_
    - _Validation: `cargo test -p agent_ui activity_action_mapping` maps every registered action kind or generic fallback_

  - [ ] 8.4. Coalesce streaming and state updates in place
    - Reduce fragments and lifecycle transitions by source ID without duplicate terminal rows.
    - _Requirements: 12.3, 12.4_
    - _Capability IDs: CAP-025_
    - _Depends on: 8.2, 8.3_
    - _Reads: crates/agent_ui/src/activity_{projection,acp,actions}.rs_
    - _Writes: crates/agent_ui/src/activity_reducer.rs_
    - _Validation: `cargo test -p agent_ui activity_reducer` covers duplicate, reordered, cancelled and timed-out updates_

  - [ ] 8.5. Render the virtualized collaborative timeline
    - Render projected items with semantic summaries, progressive details and a truthful unknown-event row.
    - _Requirements: 4.1, 12.1, 12.2_
    - _Capability IDs: CAP-025, CAP-036_
    - _Depends on: 8.4_
    - _Reads: crates/agent_ui/src/activity_{projection,reducer}.rs, crates/agent_ui/src/conversation_view/**_
    - _Writes: crates/agent_ui/src/collaborative_timeline.rs_
    - _Validation: `cargo test -p agent_ui collaborative_timeline_render` checks ordering, virtualization and detail disclosure_

  - [ ] 8.6. Add ACP activity projection regression fixtures
    - Lock exactly-once mappings and empty/waiting/error behavior for the Milestone 1 source catalog.
    - _Requirements: 12.1, 12.3, 12.4, 20.1_
    - _Capability IDs: CAP-021, CAP-022, CAP-025, CAP-044_
    - _Depends on: 8.2, 8.3, 8.4, 8.5_
    - _Reads: .agents/specs/collaborative-workspace/fixtures/protocol/**, crates/agent_ui/src/activity_*.rs_
    - _Writes: crates/agent_ui/tests/collaborative_activity.rs_
    - _Validation: `cargo test -p agent_ui collaborative_activity` passes with no unmapped source kind_

- [ ] 9. Integrate native diff review into the collaborative shell

  - [ ] 9.1. Mount the existing native review pane
    - Compose AgentDiffPane and ProjectDiff in the shell without cloning Git or diff state.
    - _Requirements: 4.1, 10.4_
    - _Capability IDs: CAP-020, CAP-036_
    - _Depends on: 6.4, 8.5_
    - _Reads: crates/agent_ui/src/agent_diff.rs, crates/git_ui/src/project_diff.rs, crates/workspace/src/collaborative_layout.rs_
    - _Writes: crates/workspace/src/collaborative_review.rs_
    - _Validation: `cargo test -p workspace collaborative_review_mount` verifies shared GitStore and pane collapse_

  - [ ] 9.2. Add stable timeline-to-change links
    - Resolve activity action/change IDs to repository, file and hunk targets and surface stale targets.
    - _Requirements: 10.3, 10.4_
    - _Capability IDs: CAP-020, CAP-025_
    - _Depends on: 9.1_
    - _Reads: crates/action_log/src/**, crates/workspace/src/collaborative_review.rs, crates/git_ui/src/project_diff.rs_
    - _Writes: crates/agent_ui/src/activity_diff_link.rs_
    - _Validation: `cargo test -p agent_ui activity_diff_link` covers valid, moved, stale and missing hunks_

  - [ ] 9.3. Expose review file navigation and aggregate statistics
    - Reuse native file selection and addition/deletion totals in top and status surfaces.
    - _Requirements: 10.4_
    - _Capability IDs: CAP-020, CAP-036_
    - _Depends on: 9.1_
    - _Reads: crates/git_ui/src/project_diff.rs, crates/workspace/src/collaborative_review.rs_
    - _Writes: crates/workspace/src/collaborative_review_summary.rs_
    - _Validation: `cargo test -p workspace collaborative_review_summary` checks file changes and zero/stale states_

  - [ ] 9.4. Route valid keep, reject, stage and review actions
    - Invoke existing native actions only when their source and Git state permit them and surface failures.
    - _Requirements: 10.4_
    - _Capability IDs: CAP-020_
    - _Depends on: 9.1, 9.2_
    - _Reads: crates/agent_ui/src/agent_diff.rs, crates/git_ui/src/project_diff.rs_
    - _Writes: crates/workspace/src/collaborative_review_actions.rs_
    - _Validation: `cargo test -p workspace collaborative_review_actions` covers valid, conflict, rejected and stale transitions_

  - [ ] 9.5. Add review-pane regression scenarios
    - Exercise pane collapse/restore, file navigation, action links and canonical Git updates together.
    - _Requirements: 4.2, 10.3, 10.4, 20.1_
    - _Capability IDs: CAP-020, CAP-036, CAP-044_
    - _Depends on: 9.2, 9.3, 9.4_
    - _Reads: crates/workspace/src/collaborative_review*.rs_
    - _Writes: crates/workspace/tests/collaborative_review.rs_
    - _Validation: `cargo test -p workspace collaborative_review` passes against a temporary repository fixture_

- [ ] 10. Finish native composer, status, accessibility and visual coverage

  - [ ] 10.1. Mount the native collaborative composer
    - Reuse the existing message/prompt editor and bind submit/cancel to the active ACP thread.
    - _Requirements: 4.1, 11.1_
    - _Capability IDs: CAP-021, CAP-036_
    - _Depends on: 8.5, 9.5_
    - _Reads: crates/agent_ui/src/message_editor.rs, crates/workspace/src/collaborative_workspace.rs_
    - _Writes: crates/workspace/src/collaborative_composer.rs_
    - _Validation: `cargo test -p workspace collaborative_composer` covers send, empty input, cancellation and unavailable thread_

  - [ ] 10.2. Add participant and execution status projection
    - Project current human, agent, model, runtime and execution location into top/status surfaces.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-007, CAP-021, CAP-036_
    - _Depends on: 6.2, 7.4_
    - _Reads: crates/client/src/user.rs, crates/agent_ui/src/thread_metadata_store.rs, crates/workspace/src/status_bar.rs_
    - _Writes: crates/workspace/src/collaborative_participants.rs_
    - _Validation: `cargo test -p workspace collaborative_participants` checks stable avatars, unknown identity and local/remote runtime labels_

  - [ ] 10.3. Add project, branch, diff and task status projection
    - Compose canonical project/worktree/branch/diff and task state without new persisted authorities.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-018, CAP-020, CAP-036_
    - _Depends on: 7.3, 9.3_
    - _Reads: crates/project/src/**, crates/workspace/src/status_bar.rs, crates/workspace/src/collaborative_review_summary.rs_
    - _Writes: crates/workspace/src/collaborative_status.rs_
    - _Validation: `cargo test -p workspace collaborative_status` covers missing repo, dirty branch and running/waiting task_

  - [ ] 10.4. Implement keyboard focus order and workspace actions
    - Define focus traversal and shortcuts across rail, timeline, composer, review and status controls.
    - _Requirements: 4.4_
    - _Capability IDs: CAP-036_
    - _Depends on: 7.5, 9.5, 10.1_
    - _Reads: crates/workspace/src/collaborative_*.rs, crates/gpui/**_
    - _Writes: crates/workspace/src/collaborative_focus.rs_
    - _Validation: `cargo test -p workspace collaborative_focus` verifies logical order, restoration and no focus trap_

  - [ ] 10.5. Add accessibility names and state announcements
    - Label navigation, participant, activity, composer, review and failure states and announce meaningful transitions.
    - _Requirements: 4.4_
    - _Capability IDs: CAP-036_
    - _Depends on: 10.4_
    - _Reads: crates/workspace/src/collaborative_*.rs, crates/agent_ui/src/collaborative_timeline.rs_
    - _Writes: crates/workspace/src/collaborative_accessibility.rs_
    - _Validation: GPUI accessibility snapshot contains named landmarks, controls and running/error announcements_

  - [ ] 10.6. Add native viewport visual fixtures
    - Capture expanded and collapsed compositions at the checked-in reference dimensions using theme tokens.
    - _Requirements: 4.2, 4.5_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.6, 7.5, 8.5, 9.5, 10.3_
    - _Reads: .agents/specs/collaborative-workspace/screenshots/*.png, crates/workspace/src/collaborative_*.rs_
    - _Writes: crates/workspace/tests/visual/collaborative_workspace.rs, crates/workspace/tests/fixtures/collaborative_workspace/*_
    - _Validation: visual comparison passes at 1930×1262 expanded and 1928×1298 collapsed with explicit baseline approval_

  - [ ] 10.7. Add theme, zoom, narrow-window and restart regressions
    - Verify dark/high-contrast themes, reduced motion, zoom, narrow layout and full presentation-state restoration.
    - _Requirements: 3.2, 4.3, 4.4, 4.5, 20.1_
    - _Capability IDs: CAP-036, CAP-037, CAP-044_
    - _Depends on: 5.5, 6.5, 10.5, 10.6_
    - _Reads: crates/workspace/tests/visual/collaborative_workspace.rs, crates/workspace/src/collaborative_*.rs_
    - _Writes: crates/workspace/tests/collaborative_workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_workspace` passes all accessibility, visual, persistence and failure fixtures_

## Milestone 2 — establish canonical protocol, identity and service foundations

- [ ] 11. Implement the UI-free collaboration domain and Nostr codecs

  - [ ] 11.1. Define collaboration aggregate identifiers and provenance
    - Add tenant-scoped stable IDs, versions and provenance fields without I/O or GPUI dependencies.
    - _Requirements: 2.1, 2.2, 5.1_
    - _Capability IDs: CAP-001, CAP-003, CAP-005_
    - _Depends on: 2.1, 3.1_
    - _Reads: projects/buzz/crates/buzz-core/src/{event,tenant}.rs, crates/proto/**_
    - _Writes: crates/collaboration_domain/src/identity_types.rs, crates/collaboration_domain/src/provenance.rs_
    - _Validation: `cargo test -p collaboration_domain provenance` verifies stable tenant-scoped identity and version ordering_

  - [ ] 11.2. Port canonical event serialization and identifiers
    - Implement exact canonical JSON, event-ID and signature-input encoding behind the compatibility boundary.
    - _Requirements: 5.1, 5.4_
    - _Capability IDs: CAP-001_
    - _Depends on: 11.1_
    - _Reads: projects/buzz/crates/buzz-core/src/event.rs, .agents/specs/collaborative-workspace/fixtures/protocol/**_
    - _Writes: crates/nostr_compat/src/event.rs_
    - _Validation: `cargo test -p nostr_compat event_vectors` matches frozen byte and ID fixtures_

  - [ ] 11.3. Port signing and verification rules
    - Verify Schnorr signatures, event IDs, timestamps and malformed inputs without accessing key storage.
    - _Requirements: 5.1, 5.4, 19.2_
    - _Capability IDs: CAP-001, CAP-009_
    - _Depends on: 11.2_
    - _Reads: projects/buzz/crates/buzz-core/src/verification.rs, crates/nostr_compat/src/event.rs_
    - _Writes: crates/nostr_compat/src/verification.rs_
    - _Validation: `cargo test -p nostr_compat verification` covers valid, altered, oversized and invalid-key fixtures_

  - [ ] 11.4. Port filter and replaceable-head semantics
    - Implement bounded filters and exact replaceable/addressable selection rules as pure functions.
    - _Requirements: 5.1, 5.4, 8.4_
    - _Capability IDs: CAP-001, CAP-002_
    - _Depends on: 11.2_
    - _Reads: projects/buzz/crates/buzz-core/src/filter.rs, projects/buzz/crates/buzz-core/src/kind.rs_
    - _Writes: crates/nostr_compat/src/filter.rs, crates/nostr_compat/src/head.rs_
    - _Validation: property tests match Buzz selection for permutations, ties, deletes and invalid limits_

  - [ ] 11.5. Generate the standard and Buzz kind registry
    - Generate typed kind metadata, persistence class, privacy gate and replacement behavior from the frozen catalog.
    - _Requirements: 1.2, 5.1, 5.3_
    - _Capability IDs: CAP-001, CAP-044_
    - _Depends on: 1.2, 11.4_
    - _Reads: .agents/specs/collaborative-workspace/catalogs/protocol.csv, projects/buzz/crates/buzz-core/src/kind.rs_
    - _Writes: crates/nostr_compat/src/generated_kinds.rs_
    - _Validation: generator check fails on an unclassified kind and matches all 116 frozen constants_

  - [ ] 11.6. Implement membership and identity NIP codecs
    - Add exact parsing and encoding for NIP-AA, NIP-IA and NIP-OA.
    - _Requirements: 5.3, 5.4, 7.1_
    - _Capability IDs: CAP-001, CAP-007, CAP-008_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-{AA,IA,OA}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips/identity.rs_
    - _Validation: identity NIP vectors round-trip and reject malformed membership/attestation/archive tags_

  - [ ] 11.7. Implement persona and managed-agent NIP codecs
    - Add exact parsing and encoding for NIP-AP and NIP-PMA.
    - _Requirements: 5.3, 5.4, 11.2_
    - _Capability IDs: CAP-001, CAP-023_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-{AP,PMA}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips/agent_config.rs_
    - _Validation: agent-config vectors cover versions, CAS predecessors, privacy gates and malformed projections_

  - [ ] 11.8. Implement agent activity and memory NIP codecs
    - Add exact parsing and encoding for NIP-AE, NIP-AM and NIP-AO.
    - _Requirements: 5.3, 5.4, 11.3, 12.1_
    - _Capability IDs: CAP-001, CAP-024, CAP-025_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-{AE,AM,AO}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips/agent_activity.rs_
    - _Validation: encrypted agent vectors cover coordinates, versions, observer frames and privacy failures_

  - [ ] 11.9. Implement communication-state NIP codecs
    - Add exact parsing and encoding for NIP-CW, NIP-DV, NIP-ER and NIP-RS.
    - _Requirements: 5.3, 5.4, 9.1, 9.2, 9.3_
    - _Capability IDs: CAP-001, CAP-011, CAP-012, CAP-013_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-{CW,DV,ER,RS}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips/communication.rs_
    - _Validation: communication vectors cover cursors, wraps, reminders, read frontiers and malformed tags_

  - [ ] 11.10. Implement project and workflow NIP codecs
    - Add exact parsing and encoding for NIP-GS, NIP-MP and NIP-WP.
    - _Requirements: 5.3, 5.4, 10.1, 10.2, 13.1_
    - _Capability IDs: CAP-001, CAP-018, CAP-019, CAP-027_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-{GS,MP,WP}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips/project_workflow.rs_
    - _Validation: project/workflow vectors cover signed coordinates, versions and malformed cross-references_

  - [ ] 11.11. Implement push-lease NIP codec
    - Add exact parsing and encoding for NIP-PL without notification policy or provider behavior.
    - _Requirements: 5.3, 5.4, 9.5_
    - _Capability IDs: CAP-001, CAP-016_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-PL.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips/push_lease.rs_
    - _Validation: push-lease vectors cover generation, capabilities, expiry and malformed encrypted values_

  - [ ] 11.12. Add custom NIP catalog conformance
    - Run every custom NIP golden/malformed fixture independently of production reducers.
    - _Requirements: 5.3, 5.4, 20.2_
    - _Capability IDs: CAP-001, CAP-044_
    - _Depends on: 11.6, 11.7, 11.8, 11.9, 11.10, 11.11_
    - _Reads: crates/nostr_compat/src/buzz_nips/**, .agents/specs/collaborative-workspace/fixtures/protocol/**_
    - _Writes: crates/nostr_compat/tests/buzz_nips.rs_
    - _Validation: `cargo test -p nostr_compat buzz_nips` passes every registered custom NIP fixture_

  - [ ] 11.13. Enforce the domain dependency boundary
    - Wire manifests and a dependency check so collaboration-domain cannot depend on GPUI, storage or transports.
    - _Requirements: 2.1, 2.4_
    - _Capability IDs: CAP-001, CAP-036_
    - _Depends on: 11.1, 11.12_
    - _Reads: Cargo.toml, crates/collaboration_domain/**, crates/nostr_compat/**_
    - _Writes: crates/collaboration_domain/Cargo.toml, crates/nostr_compat/Cargo.toml, script/check-collaboration-dependencies_
    - _Validation: dependency checker and `cargo check -p collaboration_domain -p nostr_compat` pass_

- [ ] 12. Consolidate identity binding and signing-key custody

  - [ ] 12.1. Implement approved account-to-Nostr binding records
    - Add binding creation, verification method, community scope and version state from ADR-002.
    - _Requirements: 7.1, 7.4_
    - _Capability IDs: CAP-007, CAP-008_
    - _Depends on: 2.2, 11.1, 11.3_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-002-identity-binding.md, crates/client/src/user.rs_
    - _Writes: crates/collaboration_domain/src/account_binding.rs_
    - _Validation: `cargo test -p collaboration_domain account_binding` covers verified, conflicting, revoked and historical bindings_

  - [ ] 12.2. Add human and agent profile domain records
    - Model profiles, status, owner attestations, social lists and archival without conflating account and signing identity.
    - _Requirements: 7.1, 7.4_
    - _Capability IDs: CAP-007, CAP-023_
    - _Depends on: 11.6, 12.1_
    - _Reads: projects/buzz/docs/nips/NIP-{OA,IA}.md, projects/buzz/crates/buzz-core/src/identity.rs_
    - _Writes: crates/collaboration_domain/src/profile.rs_
    - _Validation: domain tests preserve historical authorship and reject unattested agent-owner changes_

  - [ ] 12.3. Add the identity-binding persistence migration
    - Create versioned tenant-fenced bindings and revocations with no private key columns.
    - _Requirements: 6.1, 7.1, 7.4_
    - _Capability IDs: CAP-005, CAP-007_
    - _Depends on: 12.1_
    - _Reads: crates/collab/src/db/**, crates/collaboration_domain/src/account_binding.rs_
    - _Writes: crates/collab/migrations/collaboration_identity_bindings.sql_
    - _Validation: migration tests cover forward/down paths, tenant fences and absence of secret columns_

  - [ ] 12.4. Implement protected signing-key import
    - Import Buzz key identifiers into Sim credentials, verify a signing challenge and retain the source until confirmation.
    - _Requirements: 7.2, 7.3, 17.2_
    - _Capability IDs: CAP-009, CAP-045_
    - _Depends on: 11.3, 12.1_
    - _Reads: crates/credentials_provider/**, crates/sim_credentials_provider/**, projects/buzz/desktop/src-tauri/src/{secret-store,identity-storage}.rs_
    - _Writes: crates/sim_credentials_provider/src/nostr_import.rs_
    - _Validation: credential tests cover success, corrupt source, unavailable keyring, challenge mismatch and source preservation_

  - [ ] 12.5. Implement key generation, rotation and archive transitions
    - Route generation, rotation, revocation and archive through canonical credentials and identity records.
    - _Requirements: 7.2, 7.3, 7.4_
    - _Capability IDs: CAP-007, CAP-009_
    - _Depends on: 12.2, 12.4_
    - _Reads: crates/sim_credentials_provider/src/nostr_import.rs, crates/collaboration_domain/src/profile.rs_
    - _Writes: crates/sim_credentials_provider/src/nostr_lifecycle.rs_
    - _Validation: lifecycle tests prove old authorship remains, active signing changes and failures never synthesize a key_

  - [ ] 12.6. Add backup and restore compatibility
    - Preserve approved Buzz backup formats with redacted diagnostics and verified restore into canonical storage.
    - _Requirements: 7.2, 16.1_
    - _Capability IDs: CAP-009, CAP-033_
    - _Depends on: 12.4, 12.5_
    - _Reads: projects/buzz/desktop/src-tauri/src/key-backup.rs, crates/sim_credentials_provider/src/nostr_lifecycle.rs_
    - _Writes: crates/sim_credentials_provider/src/nostr_backup.rs_
    - _Validation: round-trip, wrong-password, truncated-backup and log-redaction tests pass_

  - [ ] 12.7. Implement the identity-binding repository
    - Read and write binding versions/revocations through typed tenant inputs and optimistic concurrency.
    - _Requirements: 6.1, 7.1, 7.4_
    - _Capability IDs: CAP-005, CAP-007_
    - _Depends on: 12.3_
    - _Reads: crates/collab/migrations/collaboration_identity_bindings.sql, crates/collaboration_domain/src/account_binding.rs_
    - _Writes: crates/collab/src/identity/binding_repository.rs_
    - _Validation: `cargo test -p collab identity_binding_repository` covers tenant isolation, revoke and version conflict_

- [ ] 13. Add typed tenant admission and common authorization

  - [ ] 13.1. Define trusted TenantContext construction
    - Construct tenant context only from approved host, listener or deployment routing and reject payload-derived values.
    - _Requirements: 6.1, 6.3_
    - _Capability IDs: CAP-003, CAP-008_
    - _Depends on: 4.1, 11.1_
    - _Reads: projects/buzz/crates/buzz-core/src/tenant.rs, projects/buzz/crates/buzz-relay/src/tenant.rs_
    - _Writes: crates/collaboration_domain/src/tenant.rs_
    - _Validation: `cargo test -p collaboration_domain tenant_context` rejects absent, conflicting and event-tag tenants_

  - [ ] 13.2. Define common authenticated principals
    - Normalize Sim accounts, Nostr keys, owner-attested agents, scoped tokens and services into typed principals.
    - _Requirements: 6.2, 7.1_
    - _Capability IDs: CAP-007, CAP-008, CAP-023_
    - _Depends on: 12.1, 13.1_
    - _Reads: crates/collab/src/auth.rs, projects/buzz/crates/buzz-auth/**_
    - _Writes: crates/collaboration_domain/src/principal.rs_
    - _Validation: principal tests reject unverified bindings and preserve service/token scopes_

  - [ ] 13.3. Implement membership, role and resource authorization policy
    - Evaluate membership versions, roles, channel access, ownership, scopes and delegation from typed inputs.
    - _Requirements: 6.2, 6.4_
    - _Capability IDs: CAP-003, CAP-008, CAP-010, CAP-023_
    - _Depends on: 13.2_
    - _Reads: projects/buzz/crates/buzz-auth/**, crates/collaboration_domain/src/{tenant,principal}.rs_
    - _Writes: crates/collaboration_domain/src/authorization.rs_
    - _Validation: authorization table tests cover every principal/resource/role decision and stale membership_

  - [ ] 13.4. Enforce tenant and policy at Sim RPC admission
    - Bind existing RPC requests to TenantContext and common authorization before handler or database access.
    - _Requirements: 6.1, 6.2, 6.3_
    - _Capability IDs: CAP-003, CAP-008_
    - _Depends on: 13.1, 13.3_
    - _Reads: crates/collab/src/{auth,rpc}.rs, crates/collaboration_domain/src/authorization.rs_
    - _Writes: crates/collab/src/tenant_admission.rs_
    - _Validation: `cargo test -p collab tenant_admission_rpc` proves authorization precedes database queries_

  - [ ] 13.5. Add scoped tokens, invites and virtual-agent membership
    - Implement API scopes, replay controls, invite evidence and NIP-AA virtual membership through the common policy.
    - _Requirements: 6.2, 6.4_
    - _Capability IDs: CAP-008, CAP-010, CAP-023_
    - _Depends on: 13.3_
    - _Reads: projects/buzz/crates/buzz-auth/**, projects/buzz/docs/nips/NIP-AA.md_
    - _Writes: crates/collaboration_domain/src/admission_evidence.rs_
    - _Validation: tests cover scope narrowing, invite exhaustion/revocation, replay and unattested virtual agents_

  - [ ] 13.6. Add independent cross-tenant negative traces
    - Exercise RPC, Nostr, database, cache, search, object, Git and count paths across two communities.
    - _Requirements: 6.1, 6.2, 6.3, 20.2_
    - _Capability IDs: CAP-003, CAP-008, CAP-044_
    - _Depends on: 13.4, 13.5_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/collab/src/tenant_admission.rs_
    - _Writes: crates/collab/tests/multitenant_conformance.rs_
    - _Validation: `cargo test -p collab multitenant_conformance` reports no content, ID, count or timing-class leaks_

- [ ] 14. Add Nostr WebSocket and HTTP adapters

  - [ ] 14.1. Establish the versioned Nostr ingress boundary
    - Add the ADR-001-approved listener/sidecar boundary and route accepted operations to domain commands.
    - _Requirements: 2.3, 5.2, 18.2_
    - _Capability IDs: CAP-002, CAP-004, CAP-043_
    - _Depends on: 2.1, 11.13, 13.4_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-001-service-topology.md, crates/collab/src/main.rs_
    - _Writes: crates/collab/src/nostr/ingress.rs_
    - _Validation: `cargo test -p collab nostr_ingress_version` rejects unsupported versions before a write_

  - [ ] 14.2. Implement NIP-42 WebSocket authentication
    - Preserve challenge, response, timeout, replay and reauthentication behavior under common principals.
    - _Requirements: 5.2, 6.2, 8.1_
    - _Capability IDs: CAP-002, CAP-004, CAP-008_
    - _Depends on: 11.3, 13.2, 14.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/connection.rs, projects/buzz/crates/buzz-auth/**_
    - _Writes: crates/collab/src/nostr/auth.rs_
    - _Validation: old test-client auth vectors cover success, timeout, replay, wrong tenant and revoked key_

  - [ ] 14.3. Implement bounded REQ, COUNT and subscription frames
    - Parse filters, enforce limits and emit EOSE/CLOSED/COUNT frames with cancellation cleanup.
    - _Requirements: 5.2, 8.1, 8.4_
    - _Capability IDs: CAP-002, CAP-004_
    - _Depends on: 11.4, 14.2_
    - _Reads: projects/buzz/crates/buzz-relay/src/{protocol,subscription}.rs, crates/nostr_compat/src/filter.rs_
    - _Writes: crates/collab/src/nostr/subscriptions.rs_
    - _Validation: conformance covers limits, EOSE, close, count privacy, cancellation and resource release_

  - [ ] 14.4. Implement signed EVENT ingest and OK responses
    - Validate, authorize and idempotently submit events while preserving exact success and rejection frames.
    - _Requirements: 5.1, 5.2, 5.4, 8.1_
    - _Capability IDs: CAP-001, CAP-002, CAP-004_
    - _Depends on: 11.3, 13.3, 14.2_
    - _Reads: projects/buzz/crates/buzz-relay/src/handlers.rs, crates/nostr_compat/src/**_
    - _Writes: crates/collab/src/nostr/event_ingest.rs_
    - _Validation: differential EVENT/OK suite matches accepted, duplicate, malformed and unauthorized Buzz behavior_

  - [ ] 14.5. Implement NIP-11, NIP-05 and NIP-98 HTTP routes
    - Expose relay metadata, identity resolution and authenticated HTTP with tenant-bound policy.
    - _Requirements: 5.2, 6.1, 6.2_
    - _Capability IDs: CAP-002, CAP-008_
    - _Depends on: 13.2, 14.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/{nip11,router}.rs, projects/buzz/crates/buzz-auth/**_
    - _Writes: crates/collab/src/nostr/http.rs_
    - _Validation: HTTP integration tests cover host binding, signatures, expiry, replay and metadata redaction_

  - [ ] 14.6. Add reconnect and local-echo compatibility tests
    - Verify reauthentication, head/window refetch, subscription rearm and optimistic event reconciliation.
    - _Requirements: 8.2, 8.3, 20.2_
    - _Capability IDs: CAP-004, CAP-006, CAP-044_
    - _Depends on: 14.3, 14.4, 14.5_
    - _Reads: projects/buzz/crates/buzz-ws-client/**, crates/collab/src/nostr/**_
    - _Writes: crates/collab/tests/nostr_reconnect.rs_
    - _Validation: `cargo test -p collab nostr_reconnect` proves no duplicate echo and exposes partial freshness_

- [ ] 15. Establish authoritative event storage and projections

  - [ ] 15.1. Add the authoritative signed-event schema
    - Create tenant-fenced event partitions, immutable bytes, signature state and addressable-head indexes under ADR-001.
    - _Requirements: 2.1, 5.1, 17.1_
    - _Capability IDs: CAP-001, CAP-005_
    - _Depends on: 2.1, 11.3, 13.6_
    - _Reads: projects/buzz/crates/buzz-db/**, projects/buzz/migrations/**, crates/collab/src/db/**_
    - _Writes: crates/collab/migrations/collaboration_events.sql_
    - _Validation: migration tests verify checksums, partitions, tenant fences, immutability and rollback_

  - [ ] 15.2. Implement the event repository
    - Store verified events once, deduplicate by ID and query exact heads and bounded filters.
    - _Requirements: 2.1, 5.1, 8.1_
    - _Capability IDs: CAP-001, CAP-005_
    - _Depends on: 15.1_
    - _Reads: crates/collab/migrations/collaboration_events.sql, crates/nostr_compat/src/{filter,head}.rs_
    - _Writes: crates/collab/src/db/collaboration/event_repository.rs_
    - _Validation: `cargo test -p collab event_repository` covers duplicate, head, delete, ephemeral and tenant cases_

  - [ ] 15.3. Define projection provenance and rebuild checkpoints
    - Persist source kind/ID/version, projection version, cursor and drift state for derived tables.
    - _Requirements: 2.2, 17.2_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 15.1_
    - _Reads: crates/collaboration_domain/src/provenance.rs, .agents/specs/collaborative-workspace/migration-plan.md_
    - _Writes: crates/collab/migrations/collaboration_projections.sql_
    - _Validation: migration tests cover checkpoint resume, version conflict and per-tenant reset_

  - [ ] 15.4. Implement transactional command and outbox persistence
    - Persist accepted commands, authoritative records and one ordered outbox operation under a stable idempotency key.
    - _Requirements: 2.2, 2.3, 8.1_
    - _Capability IDs: CAP-005, CAP-006_
    - _Depends on: 15.2, 15.3_
    - _Reads: crates/collab/src/db/**, crates/collaboration_domain/src/provenance.rs_
    - _Writes: crates/collab/src/db/collaboration/outbox.rs_
    - _Validation: `cargo test -p collab collaboration_outbox` covers retry, crash boundary, duplicate and ordering_

  - [ ] 15.5. Implement projection rebuild and drift comparison
    - Rebuild one tenant/aggregate from authority and compare source/version/count hashes without mutating authority.
    - _Requirements: 2.2, 8.3, 17.2_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 15.3, 15.4_
    - _Reads: crates/collab/src/db/collaboration/{event_repository,outbox}.rs_
    - _Writes: crates/collab/src/db/collaboration/rebuild.rs_
    - _Validation: rebuild twice yields identical projections and a seeded drift produces a scoped diagnostic_

  - [ ] 15.6. Enforce ephemeral non-persistence and privacy exclusions
    - Reject durable storage/indexing for ephemeral or privacy-disallowed kinds at the repository boundary.
    - _Requirements: 5.1, 5.3, 6.3_
    - _Capability IDs: CAP-001, CAP-005, CAP-014, CAP-025_
    - _Depends on: 11.5, 15.2_
    - _Reads: crates/nostr_compat/src/generated_kinds.rs, crates/collab/src/db/collaboration/event_repository.rs_
    - _Writes: crates/collab/src/db/collaboration/persistence_policy.rs_
    - _Validation: privacy tests prove prohibited kinds never reach SQL, search or logs_

  - [ ] 15.7. Add Postgres failure and rollback integration tests
    - Exercise transaction abort, outbox interruption, replica lag and schema rollback with authoritative data intact.
    - _Requirements: 8.3, 17.3, 20.1_
    - _Capability IDs: CAP-005, CAP-043, CAP-044_
    - _Depends on: 15.4, 15.5, 15.6_
    - _Reads: crates/collab/src/db/collaboration/**, crates/collab/migrations/collaboration_*.sql_
    - _Writes: crates/collab/tests/collaboration_storage_recovery.rs_
    - _Validation: `cargo test -p collab collaboration_storage_recovery` passes against isolated Postgres_

- [ ] 16. Consolidate realtime, presence infrastructure and search foundations

  - [ ] 16.1. Implement tenant-scoped Redis fan-out envelopes
    - Publish source ID/version and tenant-bound payload references without making Redis authoritative.
    - _Requirements: 8.1, 8.4_
    - _Capability IDs: CAP-006_
    - _Depends on: 15.4_
    - _Reads: projects/buzz/crates/buzz-pubsub/**, crates/collab/src/**_
    - _Writes: crates/collab/src/pubsub/envelope.rs_
    - _Validation: pub/sub tests reject wrong-tenant envelopes and deduplicate local source IDs_

  - [ ] 16.2. Implement cross-replica subscription fan-out
    - Connect outbox delivery to bounded local/Redis subscriptions with cancellation and replay cursors.
    - _Requirements: 8.1, 8.2, 8.4_
    - _Capability IDs: CAP-004, CAP-006_
    - _Depends on: 14.3, 16.1_
    - _Reads: crates/collab/src/{nostr/subscriptions,pubsub/envelope}.rs_
    - _Writes: crates/collab/src/pubsub/subscription_bus.rs_
    - _Validation: two-replica test covers ordering, reconnect replay, duplicate suppression and shutdown cleanup_

  - [ ] 16.3. Add replica freshness and partial-service state
    - Track heartbeat, projection lag, pub/sub availability and last trustworthy cursors for clients/operators.
    - _Requirements: 8.3, 19.3_
    - _Capability IDs: CAP-004, CAP-006, CAP-043_
    - _Depends on: 15.5, 16.2_
    - _Reads: projects/buzz/migrations/0026-*.sql, crates/collab/src/pubsub/subscription_bus.rs_
    - _Writes: crates/collab/src/freshness.rs_
    - _Validation: integration test distinguishes healthy, lagging, disconnected and recovering replicas_

  - [ ] 16.4. Add privacy-aware collaboration search schema
    - Create tenant-scoped searchable projections and database-level exclusions for private kinds.
    - _Requirements: 6.3, 9.4_
    - _Capability IDs: CAP-015_
    - _Depends on: 15.3, 15.6_
    - _Reads: projects/buzz/crates/buzz-search/**, projects/buzz/migrations/0008-*.sql_
    - _Writes: crates/collab/migrations/collaboration_search.sql_
    - _Validation: migration test proves excluded content produces no searchable vector or index entry_

  - [ ] 16.5. Implement authorized search repository primitives
    - Apply tenant/visibility policy before ranking and limit and expose projection freshness.
    - _Requirements: 9.4, 8.3_
    - _Capability IDs: CAP-015_
    - _Depends on: 13.3, 16.4_
    - _Reads: crates/collab/migrations/collaboration_search.sql, projects/buzz/crates/buzz-search/**_
    - _Writes: crates/collab/src/search/repository.rs_
    - _Validation: search tests cover authorization-before-limit, ranking, excluded kinds and lag markers_

- [ ] 17. Build resumable Buzz data importers

  - [ ] 17.1. Define migration checkpoint and integrity records
    - Persist tenant/shard, source/target cursors, counts, hashes, status and rollback boundary.
    - _Requirements: 17.1, 17.2, 17.3_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 15.3_
    - _Reads: .agents/specs/collaborative-workspace/migration-plan.md, crates/collab/src/db/**_
    - _Writes: crates/collab/src/migration/buzz/checkpoint.rs_
    - _Validation: checkpoint tests cover interruption, monotonic resume and rejected cross-tenant reuse_

  - [ ] 17.2. Import signed events and addressable heads
    - Preserve original bytes, IDs and signatures while attaching verified tenant/provenance metadata.
    - _Requirements: 17.1, 17.2_
    - _Capability IDs: CAP-001, CAP-005, CAP-045_
    - _Depends on: 15.2, 17.1_
    - _Reads: projects/buzz/crates/buzz-db/**, .agents/specs/collaborative-workspace/fixtures/migrations/**_
    - _Writes: crates/collab/src/migration/buzz/events.rs_
    - _Validation: fixture import preserves byte/hash/signature/head counts and is idempotent after interruption_

  - [ ] 17.3. Import community, membership and channel state
    - Import service-issued community, membership, invite and channel records with explicit provenance.
    - _Requirements: 17.1, 17.2_
    - _Capability IDs: CAP-003, CAP-005, CAP-010, CAP-045_
    - _Depends on: 17.1, 17.2_
    - _Reads: projects/buzz/migrations/**, projects/buzz/crates/buzz-db/src/channel.rs_
    - _Writes: crates/collab/src/migration/buzz/community_state.rs_
    - _Validation: importer rejects unknown versions, preserves membership versions and is idempotent after interruption_

  - [ ] 17.4. Import object and Git metadata by content identity
    - Inventory object keys, hashes, repository coordinates and refs without copying bytes prematurely.
    - _Requirements: 17.1, 17.2_
    - _Capability IDs: CAP-019, CAP-031, CAP-045_
    - _Depends on: 17.1_
    - _Reads: projects/buzz/crates/buzz-media/**, projects/buzz/crates/buzz-relay/src/git/**_
    - _Writes: crates/collab/src/migration/buzz/object_git_metadata.rs_
    - _Validation: fixture import matches object/ref hashes and reports missing objects without advancing checkpoint_

  - [ ] 17.5. Import desktop settings, drafts, read state and archive
    - Version and import general configuration, drafts, read state and transcript archive while preserving source files.
    - _Requirements: 9.3, 17.1, 17.2_
    - _Capability IDs: CAP-013, CAP-045_
    - _Depends on: 12.4, 17.1_
    - _Reads: projects/buzz/desktop/src-tauri/src/{migration,archive,event?sync}/**_
    - _Writes: crates/sim/src/migration/buzz/desktop_state.rs_
    - _Validation: every desktop fixture version imports twice identically and source files remain unchanged_

  - [ ] 17.6. Add migration rollback and verification harness
    - Compare counts/hashes, halt on divergence and restore pre-boundary binary/config/data fixtures.
    - _Requirements: 17.2, 17.3, 17.4, 20.1_
    - _Capability IDs: CAP-005, CAP-043, CAP-044, CAP-045_
    - _Depends on: 17.2, 17.3, 17.4, 17.5, 17.7, 17.8, 17.9_
    - _Reads: crates/collab/src/migration/buzz/**, crates/sim/src/migration/buzz/**_
    - _Writes: crates/collab/tests/buzz_import_recovery.rs_
    - _Validation: isolated harness demonstrates resume, idempotency, divergence halt and pre-boundary rollback_

  - [ ] 17.7. Import workflow, moderation and lifecycle state
    - Stage workflow/run/approval, moderation, retention and deletion checkpoints with workers disabled.
    - _Requirements: 15.1, 15.2, 15.3, 17.1, 17.2_
    - _Capability IDs: CAP-027, CAP-029, CAP-030, CAP-045_
    - _Depends on: 17.1, 17.2_
    - _Reads: projects/buzz/migrations/**, projects/buzz/crates/buzz-db/src/{moderation,workflow}.rs_
    - _Writes: crates/collab/src/migration/buzz/lifecycle_state.rs_
    - _Validation: importer preserves legal state/checkpoints and leaves workflow/deletion/retention workers disabled_

  - [ ] 17.8. Import push leases and wake outbox state
    - Stage encrypted leases, generations and pending wake records without contacting providers.
    - _Requirements: 9.5, 17.1, 17.2_
    - _Capability IDs: CAP-016, CAP-045_
    - _Depends on: 17.1, 17.2_
    - _Reads: projects/buzz/migrations/0022-*.sql, projects/buzz/migrations/0023-*.sql_
    - _Writes: crates/collab/src/migration/buzz/push_state.rs_
    - _Validation: importer preserves encrypted values/generations, rejects unknown version and sends no wake_

  - [ ] 17.9. Import managed-agent, team and snapshot staging records
    - Stage versioned private agent/team/persona/snapshot records for later canonical agent import.
    - _Requirements: 11.2, 11.3, 17.1, 17.2_
    - _Capability IDs: CAP-023, CAP-024, CAP-045_
    - _Depends on: 12.4, 17.1_
    - _Reads: projects/buzz/desktop/src-tauri/src/managed-agents/**, projects/buzz/desktop/src-tauri/src/archive/**_
    - _Writes: crates/sim/src/migration/buzz/agent_staging.rs_
    - _Validation: every agent fixture stages idempotently with version/privacy hashes and source preservation_

## Milestone 3 — communication and awareness parity

- [ ] 18. Extend canonical channels, communities and membership

  - [ ] 18.1. Add community and channel projection schema
    - Create tenant-fenced community, membership and channel projection tables with source provenance.
    - _Requirements: 2.2, 6.1, 9.1_
    - _Capability IDs: CAP-003, CAP-005, CAP-010_
    - _Depends on: 15.3, 15.7_
    - _Reads: projects/buzz/crates/buzz-db/src/channel.rs, crates/collab/src/db/queries/channels.rs_
    - _Writes: crates/collab/migrations/collaboration_channels.sql_
    - _Validation: migration tests cover tenant fences, provenance indexes and down migration_

  - [ ] 18.2. Implement community lifecycle commands
    - Add create, update, archive and join-policy transitions with version and authorization checks.
    - _Requirements: 6.2, 6.4, 9.1_
    - _Capability IDs: CAP-003, CAP-010_
    - _Depends on: 13.3, 18.1_
    - _Reads: projects/buzz/crates/buzz-core/src/community.rs, crates/collaboration_domain/src/authorization.rs_
    - _Writes: crates/collaboration_domain/src/community.rs_
    - _Validation: domain tests cover legal transitions, stale versions and unauthorized archive_

  - [ ] 18.3. Implement membership, roles and revocation
    - Project NIP-29 membership, role changes, virtual membership and revocation into common policy inputs.
    - _Requirements: 6.2, 6.4, 9.1_
    - _Capability IDs: CAP-008, CAP-010_
    - _Depends on: 13.5, 18.1, 18.2_
    - _Reads: projects/buzz/crates/buzz-db/src/channel.rs, crates/collaboration_domain/src/admission_evidence.rs_
    - _Writes: crates/collaboration_domain/src/membership.rs_
    - _Validation: membership tests cover invite, role, removal, archive and stale authorization cache_

  - [ ] 18.4. Implement channel types and lifecycle
    - Add open, private, DM, ephemeral, forum and huddle channel types with archive and expiry semantics.
    - _Requirements: 9.1, 15.2_
    - _Capability IDs: CAP-010, CAP-030, CAP-032_
    - _Depends on: 18.1, 18.3_
    - _Reads: projects/buzz/crates/buzz-db/src/channel.rs, crates/channel/src/channel_store.rs_
    - _Writes: crates/collaboration_domain/src/channel.rs_
    - _Validation: state-transition tests cover each channel type, visibility, archive and ephemeral expiry_

  - [ ] 18.5. Implement channel invite lifecycle
    - Add use-limited invites, redemption evidence, expiry and revocation under membership policy.
    - _Requirements: 6.4, 9.1_
    - _Capability IDs: CAP-008, CAP-010_
    - _Depends on: 18.3, 18.4_
    - _Reads: projects/buzz/migrations/0025-*.sql, crates/collaboration_domain/src/membership.rs_
    - _Writes: crates/collaboration_domain/src/channel_invite.rs_
    - _Validation: tests cover invite exhaustion, expiry, revocation, replay and unauthorized redemption_

  - [ ] 18.6. Integrate canonical channels with native stores
    - Project community/channel/member records into existing ChannelStore and collab UI without a second authority.
    - _Requirements: 2.1, 9.1_
    - _Capability IDs: CAP-010, CAP-036_
    - _Depends on: 18.2, 18.3, 18.4, 18.5, 18.7_
    - _Reads: crates/channel/src/channel_store.rs, crates/collab_ui/src/**, crates/collaboration_domain/src/{community,membership,channel}.rs_
    - _Writes: crates/channel/src/collaboration_store.rs_
    - _Validation: `cargo test -p channel collaboration_store` proves one canonical ID and correct type/role projections_

  - [ ] 18.7. Implement channel templates, topics and canvas metadata
    - Add validated templates plus versioned topic/canvas records under channel write policy.
    - _Requirements: 9.1_
    - _Capability IDs: CAP-010_
    - _Depends on: 18.3, 18.4_
    - _Reads: projects/buzz/desktop/src/features/channel-templates/**, crates/collaboration_domain/src/channel.rs_
    - _Writes: crates/collaboration_domain/src/channel_metadata.rs_
    - _Validation: tests cover template validation, version conflict and unauthorized topic/canvas writes_

- [ ] 19. Port messages, threads, reactions and stable channel windows

  - [ ] 19.1. Add message and auxiliary-event projection schema
    - Persist messages, edits, deletes, reactions, pins, bookmarks and schedules with provenance and stable sort keys.
    - _Requirements: 9.1, 9.2_
    - _Capability IDs: CAP-005, CAP-011_
    - _Depends on: 18.1_
    - _Reads: projects/buzz/crates/buzz-db/src/{event,thread,reaction}.rs, projects/buzz/migrations/**_
    - _Writes: crates/collab/migrations/collaboration_messages.sql_
    - _Validation: migration test covers same-second keys, uniqueness, tombstones and tenant fences_

  - [ ] 19.2. Implement message command and edit/delete rules
    - Add authorized create, edit and delete transitions with immutable source history.
    - _Requirements: 9.1_
    - _Capability IDs: CAP-011_
    - _Depends on: 18.4, 19.1_
    - _Reads: projects/buzz/desktop/src/features/messages/**, crates/collaboration_domain/src/channel.rs_
    - _Writes: crates/collaboration_domain/src/message.rs_
    - _Validation: domain tests cover author/moderator rights, stale edits, delete visibility and retries_

  - [ ] 19.3. Implement message reactions
    - Add authorized reaction add/remove and target-deletion behavior, including long custom emoji values.
    - _Requirements: 9.1_
    - _Capability IDs: CAP-011, CAP-017_
    - _Depends on: 19.2_
    - _Reads: projects/buzz/crates/buzz-db/src/reaction.rs, crates/collaboration_domain/src/message.rs_
    - _Writes: crates/collaboration_domain/src/reaction.rs_
    - _Validation: tests cover add/remove, long custom emoji, duplicate delivery and target deletion_

  - [ ] 19.4. Implement NIP-CW thread graph and summaries
    - Build reply ancestry, auxiliary closure, summary and bounded continuation rules from stable IDs.
    - _Requirements: 5.3, 9.1, 9.2_
    - _Capability IDs: CAP-011_
    - _Depends on: 11.9, 19.2, 19.3, 19.8, 19.9_
    - _Reads: projects/buzz/docs/nips/NIP-CW.md, projects/buzz/crates/buzz-db/src/thread.rs_
    - _Writes: crates/collaboration_domain/src/thread.rs_
    - _Validation: golden thread fixtures cover deep replies, deleted roots, aux closure and malformed references_

  - [ ] 19.5. Implement stable channel and thread query windows
    - Query immutable keyset pages plus live overlay with exact continuation under concurrent writes.
    - _Requirements: 9.2, 8.2_
    - _Capability IDs: CAP-011_
    - _Depends on: 19.1, 19.4_
    - _Reads: crates/collab/migrations/collaboration_messages.sql, crates/collaboration_domain/src/thread.rs_
    - _Writes: crates/collab/src/messages/window_repository.rs_
    - _Validation: dense-second and concurrent-live tests return every authorized row exactly once_

  - [ ] 19.6. Implement optimistic message reconciliation
    - Reconcile stable client operation IDs with accepted, rejected and replaced authoritative events.
    - _Requirements: 8.2, 9.2_
    - _Capability IDs: CAP-011_
    - _Depends on: 15.4, 19.2, 19.5_
    - _Reads: crates/collab_ui/src/**, crates/collab/src/messages/window_repository.rs_
    - _Writes: crates/collab_ui/src/message_reconciliation.rs_
    - _Validation: tests cover retry, rejection, reconnect, server replacement and no duplicate local echo_

  - [ ] 19.7. Render native message and thread timelines
    - Add human/agent messages, replies, edits, reactions and pagination to the common timeline projection.
    - _Requirements: 4.1, 9.1, 9.2, 12.1_
    - _Capability IDs: CAP-011, CAP-025, CAP-036_
    - _Depends on: 19.5, 19.6_
    - _Reads: crates/agent_ui/src/collaborative_timeline.rs, crates/collab_ui/src/message_reconciliation.rs_
    - _Writes: crates/collab_ui/src/message_timeline.rs_
    - _Validation: GPUI tests cover pages, live insert, replies, edits, deletion and failed optimistic items_

  - [ ] 19.8. Implement message pins and private bookmarks
    - Add role-gated pins and viewer-private bookmarks with independent removal behavior.
    - _Requirements: 9.1, 9.3_
    - _Capability IDs: CAP-011, CAP-013_
    - _Depends on: 19.2_
    - _Reads: projects/buzz/desktop/src/features/messages/**, crates/collaboration_domain/src/message.rs_
    - _Writes: crates/collaboration_domain/src/message_marker.rs_
    - _Validation: tests cover pin permissions, bookmark privacy, removal, target deletion and retries_

  - [ ] 19.9. Implement scheduled-message lifecycle
    - Add create, update, cancel and one-shot due execution under author permissions and bounded recovery.
    - _Requirements: 9.1, 9.3_
    - _Capability IDs: CAP-011, CAP-013_
    - _Depends on: 19.2_
    - _Reads: projects/buzz/desktop/src/features/messages/**, crates/collaboration_domain/src/message.rs_
    - _Writes: crates/collaboration_domain/src/scheduled_message.rs_
    - _Validation: timer tests cover edit/cancel, duplicate due, clock skew, restart and denied actor_

- [ ] 20. Port encrypted DMs and visibility projections

  - [ ] 20.1. Implement gift-wrap DM codec and privacy gates
    - Parse, validate and emit supported encrypted DM envelopes without exposing plaintext to indexing/logging paths.
    - _Requirements: 5.3, 9.1, 19.2_
    - _Capability IDs: CAP-012_
    - _Depends on: 11.9, 12.6_
    - _Reads: projects/buzz/docs/nips/NIP-DV.md, projects/buzz/crates/buzz-db/src/dm.rs_
    - _Writes: crates/nostr_compat/src/dm.rs_
    - _Validation: codec tests cover round trip, wrong recipient, malformed wrap and plaintext redaction_

  - [ ] 20.2. Implement DM group lifecycle
    - Add open, participant add/remove, leave and reopen transitions with participant-only authority.
    - _Requirements: 6.2, 9.1_
    - _Capability IDs: CAP-010, CAP-012_
    - _Depends on: 18.3, 20.1_
    - _Reads: projects/buzz/crates/buzz-db/src/dm.rs, crates/collaboration_domain/src/membership.rs_
    - _Writes: crates/collaboration_domain/src/dm.rs_
    - _Validation: state tests cover legal membership changes, stale versions and outsider denial_

  - [ ] 20.3. Persist per-viewer DM visibility
    - Store relay-signed hide/reopen state separately from message deletion and enforce it before counts/results.
    - _Requirements: 6.3, 9.3_
    - _Capability IDs: CAP-012, CAP-013_
    - _Depends on: 19.1, 20.2_
    - _Reads: projects/buzz/docs/nips/NIP-DV.md, crates/collab/migrations/collaboration_messages.sql_
    - _Writes: crates/collab/src/messages/dm_visibility.rs_
    - _Validation: repository tests cover hide, reopen, participant removal and no ID/count leakage_

  - [ ] 20.4. Render native DM navigation and timeline
    - Add authorized DM rows, participants and encrypted-message failure states using canonical records.
    - _Requirements: 4.1, 9.1, 9.3_
    - _Capability IDs: CAP-012, CAP-036_
    - _Depends on: 19.7, 20.3_
    - _Reads: crates/collab_ui/src/message_timeline.rs, crates/sidebar/src/collaborative_navigation.rs_
    - _Writes: crates/collab_ui/src/dm_view.rs_
    - _Validation: GPUI tests cover open, hide, decrypt failure, removed participant and reconnect_

  - [ ] 20.5. Add independent DM privacy conformance tests
    - Probe event, filter, count, search, notification and logs as participant and nonparticipant.
    - _Requirements: 6.3, 9.3, 20.2_
    - _Capability IDs: CAP-012, CAP-015, CAP-016, CAP-044_
    - _Depends on: 20.3, 20.4_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/collab/src/messages/dm_visibility.rs_
    - _Writes: crates/collab/tests/dm_privacy_conformance.rs_
    - _Validation: two-user/two-community suite reports no plaintext, existence, count or search leak_

- [ ] 21. Merge read state, reminders, drafts, presence and typing

  - [ ] 21.1. Implement encrypted read and manual-unread state
    - Merge cross-device frontiers and manual overrides under NIP-RS ordering and privacy rules.
    - _Requirements: 9.3_
    - _Capability IDs: CAP-013_
    - _Depends on: 11.9, 19.5_
    - _Reads: projects/buzz/docs/nips/NIP-RS.md, crates/channel/src/**_
    - _Writes: crates/collaboration_domain/src/read_state.rs_
    - _Validation: property tests cover monotonic frontier, override, tombstone and concurrent devices_

  - [ ] 21.2. Persist local drafts without server authority
    - Key drafts by canonical community/channel/thread identity and retain them through offline/restart transitions.
    - _Requirements: 9.3_
    - _Capability IDs: CAP-013_
    - _Depends on: 18.6, 19.7_
    - _Reads: crates/db/**, crates/collab_ui/src/message_timeline.rs_
    - _Writes: crates/collab_ui/src/draft_store.rs_
    - _Validation: draft tests cover restart, channel deletion, account switch and no cross-community reuse_

  - [ ] 21.3. Implement reminder lifecycle and due recovery
    - Add create, update, dismiss and due-after-offline behavior under NIP-ER privacy/retention rules.
    - _Requirements: 9.3, 15.2_
    - _Capability IDs: CAP-013, CAP-030_
    - _Depends on: 11.9, 19.3_
    - _Reads: projects/buzz/docs/nips/NIP-ER.md, projects/buzz/desktop/src/features/reminders/**_
    - _Writes: crates/collaboration_domain/src/reminder.rs_
    - _Validation: timer tests cover clock skew, restart, duplicate due and expired target_

  - [ ] 21.4. Implement canonical presence projection
    - Merge signed and room presence by source/expiry without allowing presence to grant authorization.
    - _Requirements: 9.3, 11.5_
    - _Capability IDs: CAP-014, CAP-034_
    - _Depends on: 16.3, 18.3_
    - _Reads: projects/buzz/crates/buzz-pubsub/src/presence.rs, crates/collab/src/**_
    - _Writes: crates/collaboration_domain/src/presence.rs_
    - _Validation: presence tests cover forged state, TTL expiry, multiple sources and revoked membership_

  - [ ] 21.5. Implement bounded typing indicators
    - Accept signed typing events, enforce channel access and expire them without persistence.
    - _Requirements: 5.1, 8.4, 9.3_
    - _Capability IDs: CAP-006, CAP-014_
    - _Depends on: 16.2, 18.4, 21.4_
    - _Reads: projects/buzz/crates/buzz-pubsub/**, crates/collaboration_domain/src/presence.rs_
    - _Writes: crates/collab/src/presence/typing.rs_
    - _Validation: typing tests cover rate limit, unauthorized sender, expiry, reconnect and zero durable rows_

  - [ ] 21.6. Integrate awareness state into native navigation
    - Render unread, reminder, presence and typing state with offline/freshness indicators in existing rows.
    - _Requirements: 4.3, 8.3, 9.3_
    - _Capability IDs: CAP-013, CAP-014, CAP-036_
    - _Depends on: 21.1, 21.2, 21.3, 21.4, 21.5_
    - _Reads: crates/sidebar/src/collaborative_navigation.rs, crates/collab_ui/src/draft_store.rs_
    - _Writes: crates/sidebar/src/collaborative_awareness.rs_
    - _Validation: GPUI tests cover multi-device updates, offline stale state, reconnect and expiry_

- [ ] 22. Integrate search, native notifications and NIP-PL push

  - [ ] 22.1. Project authorized collaboration content into search
    - Consume authoritative outbox records and update only policy-approved search documents.
    - _Requirements: 9.4, 15.2_
    - _Capability IDs: CAP-015, CAP-030_
    - _Depends on: 16.4, 19.1, 20.3_
    - _Reads: crates/collab/src/db/collaboration/outbox.rs, crates/collab/migrations/collaboration_search.sql_
    - _Writes: crates/collab/src/search/indexer.rs_
    - _Validation: indexer tests cover edit/delete/retention, DM exclusion and idempotent replay_

  - [ ] 22.2. Implement collaboration search queries
    - Query authorized community, channel, member, project and message result classes with freshness metadata.
    - _Requirements: 6.3, 9.4_
    - _Capability IDs: CAP-015_
    - _Depends on: 16.5, 22.1_
    - _Reads: crates/collab/src/search/{repository,indexer}.rs_
    - _Writes: crates/collab/src/search/query.rs_
    - _Validation: query tests apply policy before rank/limit and return stable result identities_

  - [ ] 22.3. Compose collaboration results in native search UI
    - Add typed collaboration groups alongside existing file/project search with scope and freshness labels.
    - _Requirements: 4.4, 9.4_
    - _Capability IDs: CAP-015, CAP-036_
    - _Depends on: 22.2_
    - _Reads: crates/search/src/**, crates/collab/src/search/query.rs_
    - _Writes: crates/search/src/collaboration_search.rs_
    - _Validation: `cargo test -p search collaboration_search` covers keyboard flow, empty, stale and unauthorized results_

  - [ ] 22.4. Define notification eligibility and deduplication policy
    - Decide native/push eligibility from mentions, membership, mute, read state, device permissions and stable source IDs.
    - _Requirements: 9.5_
    - _Capability IDs: CAP-016_
    - _Depends on: 18.3, 21.1_
    - _Reads: projects/buzz/desktop/src/features/notifications/**, crates/notifications/**_
    - _Writes: crates/collaboration_domain/src/notification_policy.rs_
    - _Validation: policy table tests cover self, mute, read, duplicate, revoked and private events_

  - [ ] 22.5. Dispatch native desktop notifications
    - Convert eligible records to existing native notifications and navigate safely to canonical entities.
    - _Requirements: 9.5, 16.4_
    - _Capability IDs: CAP-016, CAP-042_
    - _Depends on: 22.4_
    - _Reads: crates/notifications/**, crates/collaboration_domain/src/notification_policy.rs_
    - _Writes: crates/notifications/src/collaboration.rs_
    - _Validation: notification tests cover permission denial, deduplication, redacted preview and missing deep-link target_

  - [ ] 22.6. Define the canonical push-lease domain
    - Model capability-bound device leases, generations, expiry and revocation independently of wire/provider code.
    - _Requirements: 9.5_
    - _Capability IDs: CAP-016_
    - _Depends on: 22.4_
    - _Reads: projects/buzz/docs/nips/NIP-PL.md, projects/buzz/crates/buzz-push-gateway/**_
    - _Writes: crates/collaboration_domain/src/push_lease.rs_
    - _Validation: domain tests cover generation, expiry, revocation, wrong capability and wake-only payload invariant_

  - [ ] 22.7. Add push-lease and wake-outbox schema
    - Create tenant/device-scoped encrypted lease and idempotent wake-job tables.
    - _Requirements: 9.5, 17.2_
    - _Capability IDs: CAP-005, CAP-016_
    - _Depends on: 22.6_
    - _Reads: projects/buzz/migrations/0022-*.sql, projects/buzz/migrations/0023-*.sql_
    - _Writes: crates/collab/migrations/collaboration_push.sql_
    - _Validation: migration tests cover encryption columns, generation uniqueness, tenant fences and rollback_

  - [ ] 22.8. Implement the push-gateway executor
    - Consume wake jobs with bounded retry, endpoint authority and no event-content access.
    - _Requirements: 9.5, 19.2, 19.3_
    - _Capability IDs: CAP-016_
    - _Depends on: 4.6, 22.13_
    - _Reads: projects/buzz/crates/buzz-push-gateway/**, crates/collab/src/push/outbox.rs_
    - _Writes: services/push_gateway/src/executor.rs_
    - _Validation: gateway tests cover transient/permanent failure, revocation race, redaction and retry exhaustion_

  - [ ] 22.9. Implement approved platform push adapters
    - Add only ADR-005-approved APNs, App Attest and other platform adapters behind the common executor.
    - _Requirements: 9.5, 18.2_
    - _Capability IDs: CAP-016, CAP-040_
    - _Depends on: 2.5, 11.11, 22.8_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-005-push-scope.md, projects/buzz/crates/buzz-push-gateway/src/**_
    - _Writes: services/push_gateway/src/platform/*_
    - _Validation: contract tests use provider sandboxes/fakes for token, attestation, expiry and provider-error mapping_

  - [ ] 22.10. Add push-gateway deployment artifacts
    - Add configuration, migrations, health/readiness and bounded resources without production deployment.
    - _Requirements: 19.3, 19.4_
    - _Capability IDs: CAP-016, CAP-043_
    - _Depends on: 22.7, 22.8, 22.9_
    - _Reads: projects/buzz/deploy/charts/buzz-push-gateway/**, deploy/**_
    - _Writes: deploy/collaboration/push-gateway/*_
    - _Validation: chart/render and configuration tests pass with missing-secret and rollback cases_

  - [ ] 22.11. Add search and notification privacy conformance
    - Probe authorization-before-limit, private indexing, previews and wake payloads using mixed-version clients.
    - _Requirements: 6.3, 9.4, 9.5, 20.2_
    - _Capability IDs: CAP-015, CAP-016, CAP-044_
    - _Depends on: 11.11, 22.2, 22.5, 22.9_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/search/src/collaboration_search.rs, services/push_gateway/src/**_
    - _Writes: crates/collab/tests/search_push_privacy.rs_
    - _Validation: conformance returns no private content, count, preview or payload across community boundaries_

  - [ ] 22.12. Add search and push load/failure tests
    - Measure indexing/query and wake throughput, queue bounds, replica lag and recovery under dependency failure.
    - _Requirements: 8.3, 8.4, 19.3, 20.1_
    - _Capability IDs: CAP-006, CAP-015, CAP-016, CAP-044_
    - _Depends on: 22.10, 22.11_
    - _Reads: projects/buzz/perf/**, crates/collab/src/search/**, services/push_gateway/src/**_
    - _Writes: test-results/collaborative-workspace/search-push-plan.md_
    - _Validation: approved load command meets bounded queue/freshness budgets and records recovery evidence_

  - [ ] 22.13. Implement push-lease and wake-outbox persistence
    - Read/write encrypted leases and consume idempotent wake jobs under canonical device authority.
    - _Requirements: 9.5, 17.2_
    - _Capability IDs: CAP-005, CAP-016_
    - _Depends on: 22.7_
    - _Reads: crates/collab/migrations/collaboration_push.sql, crates/collaboration_domain/src/push_lease.rs_
    - _Writes: crates/collab/src/push/outbox.rs_
    - _Validation: repository tests cover replacement, revoke, crash retry, duplicate wake and tenant isolation_

- [ ] 23. Port inbox, pulse, forum, custom emoji and feedback

  - [ ] 23.1. Implement the canonical inbox projection
    - Derive mentions, replies, reminders and activity from message/read records without a second message store.
    - _Requirements: 2.2, 9.1, 9.3_
    - _Capability IDs: CAP-013, CAP-017_
    - _Depends on: 19.3, 19.8, 19.9, 21.1, 21.3_
    - _Reads: projects/buzz/desktop/src/features/home/**, crates/collaboration_domain/src/{message_aux,read-state,reminder}.rs_
    - _Writes: crates/collaboration_domain/src/inbox.rs_
    - _Validation: projection fixtures cover mention, reminder, read, deletion and duplicate events_

  - [ ] 23.2. Render native inbox and pulse lists
    - Add filterable, paged GPUI lists over canonical inbox/activity projections.
    - _Requirements: 4.4, 9.1, 9.3_
    - _Capability IDs: CAP-017, CAP-036_
    - _Depends on: 23.1_
    - _Reads: projects/buzz/desktop/src/features/{home,pulse}/**, crates/collab_ui/src/**_
    - _Writes: crates/collab_ui/src/inbox_pulse.rs_
    - _Validation: GPUI tests cover unread, filters, pagination, empty and stale states_

  - [ ] 23.3. Implement forum post, vote and comment domain rules
    - Model forum records as channel/message projections with authorized voting and stable thread links.
    - _Requirements: 9.1, 9.2_
    - _Capability IDs: CAP-011, CAP-017_
    - _Depends on: 19.2, 19.4_
    - _Reads: projects/buzz/desktop/src/features/forum/**, projects/buzz/mobile/lib/features/forum/**_
    - _Writes: crates/collaboration_domain/src/forum.rs_
    - _Validation: domain tests cover vote replacement, comment deletion, pagination and visibility_

  - [ ] 23.4. Render native forum surfaces
    - Add post list/detail/composer views using canonical channel, thread and forum records.
    - _Requirements: 4.4, 9.1_
    - _Capability IDs: CAP-017, CAP-036_
    - _Depends on: 19.7, 23.3_
    - _Reads: crates/collaboration_domain/src/forum.rs, crates/collab_ui/src/message_timeline.rs_
    - _Writes: crates/collab_ui/src/forum.rs_
    - _Validation: GPUI tests cover create, vote, comment, permission denial and archived forum_

  - [ ] 23.5. Implement custom emoji records and reaction resolution
    - Validate community emoji identifiers/assets and resolve long reaction values without changing message authority.
    - _Requirements: 9.1, 14.1_
    - _Capability IDs: CAP-011, CAP-017, CAP-031_
    - _Depends on: 19.3_
    - _Reads: projects/buzz/desktop/src/features/custom-emoji/**, projects/buzz/migrations/0028-*.sql_
    - _Writes: crates/collaboration_domain/src/custom_emoji.rs_
    - _Validation: tests cover duplicate names, invalid media, removal and historical reaction rendering_

  - [ ] 23.6. Implement feedback event flow
    - Add authorized feedback submission and operator-safe status projection without exposing private context.
    - _Requirements: 9.1, 15.4_
    - _Capability IDs: CAP-017, CAP-029_
    - _Depends on: 18.2, 19.2_
    - _Reads: projects/buzz/desktop/src/features/pulse/**, projects/buzz/VISION_MODERATION.md_
    - _Writes: crates/collaboration_domain/src/feedback.rs_
    - _Validation: tests cover submit, redact, status update, unauthorized read and tenant isolation_

  - [ ] 23.7. Add social-surface projection regressions
    - Verify inbox, pulse, forum, emoji and feedback rebuild from canonical records and recover after reconnect.
    - _Requirements: 8.2, 9.1, 9.3, 20.1_
    - _Capability IDs: CAP-013, CAP-017, CAP-044_
    - _Depends on: 23.2, 23.4, 23.5, 23.6_
    - _Reads: crates/collaboration_domain/src/{inbox,forum,custom-emoji,feedback}.rs, crates/collab_ui/src/{inbox_pulse,forum}.rs_
    - _Writes: crates/collab_ui/tests/social_surfaces.rs_
    - _Validation: `cargo test -p collab_ui social_surfaces` passes rebuild, offline, reconnect and failure fixtures_

## Milestone 4 — projects, Git and review collaboration

- [ ] 24. Bind Sim projects and repositories to NIP-MP metadata

  - [ ] 24.1. Define signed project-group metadata
    - Model NIP-MP project identity, visibility and repository coordinates without local filesystem authority.
    - _Requirements: 10.1_
    - _Capability IDs: CAP-018_
    - _Depends on: 11.10, 18.2_
    - _Reads: projects/buzz/docs/nips/NIP-MP.md, crates/project/src/project.rs_
    - _Writes: crates/collaboration_domain/src/project_group.rs_
    - _Validation: domain tests cover multi-repository, cross-owner, visibility and invalid coordinate cases_

  - [ ] 24.2. Add stable collaboration repository identity
    - Map local repository identity to hosted coordinates and preserve remotes/worktrees as Sim-owned state.
    - _Requirements: 2.1, 10.1_
    - _Capability IDs: CAP-018, CAP-019_
    - _Depends on: 24.1_
    - _Reads: crates/project/src/{project,worktree-store}.rs, crates/project/src/git_store.rs_
    - _Writes: crates/project/src/collaboration_repository.rs_
    - _Validation: `cargo test -p project collaboration_repository_identity` covers reopen, remote change and missing repo_

  - [ ] 24.3. Persist project-group bindings
    - Store versioned project/repository/channel bindings separately from local project persistence.
    - _Requirements: 2.2, 10.1_
    - _Capability IDs: CAP-005, CAP-018_
    - _Depends on: 24.1, 24.2_
    - _Reads: crates/collab/src/db/**, crates/collaboration_domain/src/project_group.rs_
    - _Writes: crates/collab/migrations/collaboration_projects.sql_
    - _Validation: migration tests cover tenant fences, cross-owner grouping and binding deletion_

  - [ ] 24.4. Integrate project and channel navigation bindings
    - Resolve signed project/channel bindings into existing native project and collaborative navigation entities.
    - _Requirements: 4.3, 10.1_
    - _Capability IDs: CAP-010, CAP-018, CAP-036_
    - _Depends on: 18.6, 24.3_
    - _Reads: crates/sidebar/src/collaborative_projects.rs, crates/project/src/collaboration_repository.rs_
    - _Writes: crates/project/src/collaboration_navigation.rs_
    - _Validation: navigation tests cover missing local clone, multiple worktrees and archived group_

  - [ ] 24.5. Prove grouping never grants Git authority
    - Add negative integration tests for push, filesystem and external-host operations by project signers.
    - _Requirements: 6.2, 10.1, 20.3_
    - _Capability IDs: CAP-018, CAP-019, CAP-044_
    - _Depends on: 24.3, 24.4_
    - _Reads: crates/project/src/collaboration_*.rs, crates/git/**_
    - _Writes: crates/project/tests/project_group_permissions.rs_
    - _Validation: `cargo test -p project project_group_permissions` denies every authority not separately granted_

- [ ] 25. Consolidate NIP-34 forge and Git signing/authentication

  - [ ] 25.1. Implement NIP-34 repository and ref codecs
    - Encode and validate repository announcements, state, refs and status events under ADR-003.
    - _Requirements: 5.1, 10.2_
    - _Capability IDs: CAP-019_
    - _Depends on: 2.3, 11.10, 24.2_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-003-git-authority.md, projects/buzz/crates/buzz-core/src/git.rs_
    - _Writes: crates/nostr_compat/src/nip34_repository.rs_
    - _Validation: golden fixtures round-trip refs, clone URLs, maintainers and malformed coordinates_

  - [ ] 25.2. Implement NIP-34 patch, PR and issue codecs
    - Encode and validate patches, pull requests, issues, comments and status references.
    - _Requirements: 5.1, 10.2_
    - _Capability IDs: CAP-019, CAP-020_
    - _Depends on: 25.1_
    - _Reads: projects/buzz/crates/buzz-core/src/git.rs, projects/buzz/docs/nips/NIP-GS.md_
    - _Writes: crates/nostr_compat/src/nip34_collaboration.rs_
    - _Validation: golden fixtures cover patch series, revisions, issue links and invalid ancestry_

  - [ ] 25.3. Add hosted repository and permission schema
    - Create hosted coordinates, storage handles and explicit read/write/admin grant tables under ADR-003.
    - _Requirements: 6.2, 10.2_
    - _Capability IDs: CAP-005, CAP-019_
    - _Depends on: 2.3, 24.3, 25.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/git/**, crates/git_hosting_providers/**_
    - _Writes: crates/collab/migrations/collaboration_git.sql_
    - _Validation: migration tests cover tenant fences, grant uniqueness, archive and rollback_

  - [ ] 25.4. Implement content-addressed Git object storage adapter
    - Read/write objects and refs with tenant/repository fencing, atomic ref updates and integrity verification.
    - _Requirements: 10.2, 19.2_
    - _Capability IDs: CAP-019_
    - _Depends on: 25.10_
    - _Reads: projects/buzz/docs/git-on-object-storage.md, projects/buzz/crates/buzz-relay/src/git/**_
    - _Writes: crates/collab/src/git/object_store.rs_
    - _Validation: object-store tests cover hash mismatch, concurrent ref update, missing object and cross-tenant path_

  - [ ] 25.5. Implement Git smart-HTTP read paths
    - Serve discovery and fetch from the hosted adapter with bounded request/response and authorization.
    - _Requirements: 10.2, 19.2_
    - _Capability IDs: CAP-019, CAP-039_
    - _Depends on: 25.4, 25.10_
    - _Reads: projects/buzz/crates/buzz-relay/src/git/**, crates/collab/src/git/object_store.rs_
    - _Writes: crates/collab/src/git/smart_http_read.rs_
    - _Validation: clone/fetch tests cover authorized, private, missing and oversized requests_

  - [ ] 25.6. Implement Git smart-HTTP write paths
    - Accept push updates through permission checks and atomic ref/object persistence with audit IDs.
    - _Requirements: 10.2, 13.4, 19.2_
    - _Capability IDs: CAP-019, CAP-028_
    - _Depends on: 25.4, 25.5_
    - _Reads: crates/collab/src/git/{object_store,smart-http-read}.rs_
    - _Writes: crates/collab/src/git/smart_http_write.rs_
    - _Validation: push tests cover fast-forward, force policy, missing object, concurrent update and denied writer_

  - [ ] 25.7. Port the NIP-98 Git credential helper
    - Adapt credential lookup to canonical key storage with compatible stdin/stdout and rejection contracts.
    - _Requirements: 7.2, 10.2, 16.4_
    - _Capability IDs: CAP-009, CAP-019, CAP-038_
    - _Depends on: 12.5, 25.10_
    - _Reads: projects/buzz/crates/git-credential-nostr/**, crates/sim_credentials_provider/**_
    - _Writes: tools/git_credential_nostr/Cargo.toml, tools/git_credential_nostr/src/*_
    - _Validation: helper tests cover lookup, locked keyring, denied host, redaction and exact exit codes_

  - [ ] 25.8. Add NIP-34 and Git-hosting conformance suite
    - Run legacy and consolidated clone, push, patch, sign and permission scenarios against independent fixtures.
    - _Requirements: 10.2, 20.1, 20.2_
    - _Capability IDs: CAP-019, CAP-044_
    - _Depends on: 25.2, 25.6, 25.7, 25.9_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/collab/src/git/**, tools/git_*_nostr/**_
    - _Writes: crates/collab/tests/git_conformance.rs_
    - _Validation: `cargo test -p collab git_conformance` passes old/new server and external-provider cases_

  - [ ] 25.9. Port the Nostr commit and tag signing helper
    - Adapt commit/tag signing and verification to canonical key storage with compatible Git contracts.
    - _Requirements: 7.2, 10.2, 16.4_
    - _Capability IDs: CAP-009, CAP-019, CAP-038_
    - _Depends on: 12.5, 25.10_
    - _Reads: projects/buzz/crates/git-sign-nostr/**, crates/sim_credentials_provider/**_
    - _Writes: tools/git_sign_nostr/Cargo.toml, tools/git_sign_nostr/src/*_
    - _Validation: helper tests cover sign/verify, locked keyring, altered object, redaction and exact exit codes_

  - [ ] 25.10. Implement hosted repository registry and permission checks
    - Read/write hosted repository records and evaluate explicit grants before object or HTTP access.
    - _Requirements: 6.2, 10.2_
    - _Capability IDs: CAP-005, CAP-019_
    - _Depends on: 25.3_
    - _Reads: crates/collab/migrations/collaboration_git.sql, crates/collaboration_domain/src/authorization.rs_
    - _Writes: crates/collab/src/git/repository_registry.rs_
    - _Validation: repository tests cover tenant, permission, rename, archive and external-host coexistence_

- [ ] 26. Implement branch-as-channel linkage

  - [ ] 26.1. Define branch collaboration identity and state
    - Model repository/branch/commit identity and create, update, merge and archive transitions.
    - _Requirements: 10.3_
    - _Capability IDs: CAP-020_
    - _Depends on: 18.4, 25.1_
    - _Reads: projects/buzz/VISION_PROJECTS.md, crates/project/src/git_store.rs_
    - _Writes: crates/collaboration_domain/src/branch_activity.rs_
    - _Validation: state tests cover branch recreation, force update, merge and stale commit_

  - [ ] 26.2. Create and bind branch channels idempotently
    - Create one canonical channel per approved branch binding and reuse it on retries/reconnect.
    - _Requirements: 9.1, 10.3_
    - _Capability IDs: CAP-010, CAP-020_
    - _Depends on: 18.6, 26.1_
    - _Reads: crates/collaboration_domain/src/{channel,branch-activity}.rs_
    - _Writes: crates/collab/src/git/branch_channel.rs_
    - _Validation: tests prove one channel per binding under duplicate and concurrent create_

  - [ ] 26.3. Project ref updates into immutable activity events
    - Emit stable branch/ref/commit activity records from accepted Git updates.
    - _Requirements: 10.3, 12.1_
    - _Capability IDs: CAP-020, CAP-025_
    - _Depends on: 25.6, 26.1, 26.2_
    - _Reads: crates/collab/src/git/smart_http_write.rs, crates/collab/src/git/branch_channel.rs_
    - _Writes: crates/collab/src/git/branch_activity.rs_
    - _Validation: tests cover retry deduplication, commit links, force update and missing channel recovery_

  - [ ] 26.4. Apply merge and archive channel lifecycle
    - Transition branch channels on merge/delete while preserving immutable conversation and review history.
    - _Requirements: 9.1, 10.3_
    - _Capability IDs: CAP-010, CAP-020_
    - _Depends on: 26.2, 26.3_
    - _Reads: crates/collab/src/git/{branch_channel,branch-activity}.rs_
    - _Writes: crates/collab/src/git/branch_lifecycle.rs_
    - _Validation: lifecycle tests cover merge, delete, reopen, stale events and retained history_

  - [ ] 26.5. Add branch-channel reconnect regressions
    - Verify ref activity and channel state converge after duplicate, delayed and disconnected delivery.
    - _Requirements: 8.2, 10.3, 20.1_
    - _Capability IDs: CAP-006, CAP-020, CAP-044_
    - _Depends on: 26.3, 26.4_
    - _Reads: crates/collab/src/git/branch_*.rs_
    - _Writes: crates/collab/tests/branch_channel_recovery.rs_
    - _Validation: `cargo test -p collab branch_channel_recovery` passes reordered and reconnect traces_

- [ ] 27. Complete review, CI and approval timeline integration

  - [ ] 27.1. Define canonical review and approval records
    - Model patch revision, review comment, approval and merge readiness linked to repository/commit IDs.
    - _Requirements: 10.3, 10.4_
    - _Capability IDs: CAP-019, CAP-020_
    - _Depends on: 25.2, 26.3_
    - _Reads: projects/buzz/VISION_PROJECTS.md, crates/collaboration_domain/src/branch_activity.rs_
    - _Writes: crates/collaboration_domain/src/review.rs_
    - _Validation: state tests cover stale revision, superseded approval, comment anchor and merge eligibility_

  - [ ] 27.2. Define CI result and workflow-link records
    - Model check suites, runs, statuses and artifact links with bounded untrusted text.
    - _Requirements: 10.3, 19.2_
    - _Capability IDs: CAP-020, CAP-027_
    - _Depends on: 27.1_
    - _Reads: projects/buzz/VISION_PROJECTS.md, crates/collaboration_domain/src/review.rs_
    - _Writes: crates/collaboration_domain/src/ci_status.rs_
    - _Validation: tests cover pending/success/failure/cancel, stale commit and malicious output truncation_

  - [ ] 27.3. Persist review, approval and CI projections
    - Add provenance-aware projection tables and repositories without duplicating Git working state.
    - _Requirements: 2.2, 10.3_
    - _Capability IDs: CAP-005, CAP-020_
    - _Depends on: 27.1, 27.2_
    - _Reads: crates/collab/migrations/collaboration_git.sql, crates/collaboration_domain/src/{review,ci-status}.rs_
    - _Writes: crates/collab/src/git/review_repository.rs_
    - _Validation: repository tests cover revision replacement, provenance rebuild and tenant isolation_

  - [ ] 27.4. Project Git, review and CI events into ActivityItem
    - Map all collaboration code events to verb/object/outcome classes and truthful fallbacks.
    - _Requirements: 10.3, 12.1, 12.2_
    - _Capability IDs: CAP-020, CAP-025_
    - _Depends on: 8.4, 26.3, 27.3_
    - _Reads: crates/agent_ui/src/activity_projection.rs, crates/collaboration_domain/src/{branch_activity,review,ci-status}.rs_
    - _Writes: crates/agent_ui/src/activity_git.rs_
    - _Validation: activity fixture test maps each Git/review/CI kind exactly once_

  - [ ] 27.5. Resolve review events to native diff state
    - Map stable repository/revision/file/hunk identities and expose stale/conflict outcomes.
    - _Requirements: 10.3, 10.4_
    - _Capability IDs: CAP-020_
    - _Depends on: 9.2, 27.1, 27.3_
    - _Reads: crates/agent_ui/src/activity_diff_link.rs, crates/git_ui/src/project_diff.rs_
    - _Writes: crates/git_ui/src/collaborative_review.rs_
    - _Validation: diff tests cover exact, moved, stale, deleted and conflicting anchors_

  - [ ] 27.6. Render collaborative review and CI cards
    - Add native cards, actions and progressive details to timeline/review surfaces using canonical records.
    - _Requirements: 4.1, 10.3, 10.4, 12.2_
    - _Capability IDs: CAP-020, CAP-025, CAP-036_
    - _Depends on: 27.4, 27.5_
    - _Reads: crates/agent_ui/src/activity_git.rs, crates/git_ui/src/collaborative_review.rs_
    - _Writes: crates/collab_ui/src/git_activity.rs_
    - _Validation: GPUI tests cover pending CI, approval, conflict, stale review and valid native actions_

  - [ ] 27.7. Add end-to-end patch-to-merge scenario
    - Exercise human/agent patch, CI, review, approval and merge with timeline-to-hunk navigation.
    - _Requirements: 10.2, 10.3, 10.4, 20.1_
    - _Capability IDs: CAP-019, CAP-020, CAP-044_
    - _Depends on: 25.8, 26.5, 27.6_
    - _Reads: crates/collab_ui/src/git_activity.rs, crates/collab/tests/git_conformance.rs_
    - _Writes: crates/collab_ui/tests/patch_review_merge.rs_
    - _Validation: end-to-end test completes valid merge and visibly blocks stale/conflicting variants_

## Milestone 5 — agent platform convergence

- [ ] 28. Adapt Buzz channel and observer ingress to Sim ACP/MCP

  - [ ] 28.1. Implement NIP-AO control and observer codecs
    - Parse encrypted control/observer frames, versions and privacy gates independently of ACP execution.
    - _Requirements: 5.3, 11.1, 12.1_
    - _Capability IDs: CAP-021, CAP-025_
    - _Depends on: 11.8, 12.5_
    - _Reads: projects/buzz/docs/nips/NIP-AO.md, projects/buzz/crates/buzz-acp/**_
    - _Writes: crates/nostr_compat/src/agent_observer.rs_
    - _Validation: golden codec tests cover versions, encryption, malformed frames and unauthorized observers_

  - [ ] 28.2. Map collaboration threads to native ACP sessions
    - Resolve channel/thread/job identities to exactly one native session and preserve cancellation ownership.
    - _Requirements: 2.1, 11.1_
    - _Capability IDs: CAP-021_
    - _Depends on: 19.7, 28.1_
    - _Reads: crates/agent/src/**, crates/acp_thread/src/**, crates/collaboration_domain/src/message.rs_
    - _Writes: crates/agent/src/collaboration_session.rs_
    - _Validation: `cargo test -p agent collaboration_session` proves idempotent create/resume and exactly-one executor_

  - [ ] 28.3. Route authorized mentions into ACP prompts
    - Convert supported human/agent mentions to native prompt requests after membership and permission checks.
    - _Requirements: 6.2, 11.1_
    - _Capability IDs: CAP-011, CAP-021_
    - _Depends on: 19.7, 28.2_
    - _Reads: crates/collaboration_domain/src/authorization.rs, crates/agent/src/collaboration_session.rs_
    - _Writes: crates/agent/src/collaboration_mention.rs_
    - _Validation: tests cover direct/team mention, duplicate event, unauthorized actor and busy session_

  - [ ] 28.4. Publish ACP lifecycle through observer adapters
    - Translate native session/action outcomes to bounded NIP-AO frames without creating a second transcript.
    - _Requirements: 11.1, 12.1, 12.3_
    - _Capability IDs: CAP-021, CAP-025_
    - _Depends on: 28.1, 28.2_
    - _Reads: crates/acp_thread/src/**, crates/nostr_compat/src/agent_observer.rs_
    - _Writes: crates/acp_thread/src/collaboration_observer.rs_
    - _Validation: observer tests cover streaming, terminal outcomes, cancellation, redaction and retry deduplication_

  - [ ] 28.5. Add Buzz MCP tool compatibility mappings
    - Map shell/read/edit/search/tree/image/todo requests to native tools and existing permission prompts.
    - _Requirements: 11.1, 19.2_
    - _Capability IDs: CAP-022_
    - _Depends on: 4.2, 28.2_
    - _Reads: projects/buzz/crates/buzz-dev-mcp/**, crates/agent/src/tools/**_
    - _Writes: crates/agent/src/buzz_tool_compat.rs_
    - _Validation: tool-by-tool tests cover success, denial, invalid path, bounded output and cancellation_

  - [ ] 28.6. Add ACP/MCP lifecycle conformance suite
    - Differentially test legacy harness and native runtime for prompts, tools, queues, cleanup and observer output.
    - _Requirements: 11.1, 11.5, 20.2_
    - _Capability IDs: CAP-021, CAP-022, CAP-044_
    - _Depends on: 28.3, 28.4, 28.5_
    - _Reads: projects/buzz/crates/{buzz-acp,buzz-agent,buzz-dev-mcp}/**, crates/agent/src/collaboration_*.rs_
    - _Writes: crates/agent/tests/buzz_acp_conformance.rs_
    - _Validation: `cargo test -p agent buzz_acp_conformance` passes reentrancy, crash and resource-cleanup cases_

- [ ] 29. Port personas, teams and private managed-agent state

  - [ ] 29.1. Port persona pack parsing and merge rules
    - Parse persona metadata/content and deterministic inheritance without runtime or UI concerns.
    - _Requirements: 11.2_
    - _Capability IDs: CAP-023_
    - _Depends on: 11.7_
    - _Reads: projects/buzz/crates/buzz-persona/**, projects/buzz/docs/nips/NIP-AP.md_
    - _Writes: crates/agent_settings/src/persona.rs_
    - _Validation: parser fixtures cover valid, inherited, conflicting and malformed packs_

  - [ ] 29.2. Define agent-team and catalog records
    - Model team membership, roles, catalogs and public share records with owner attestations.
    - _Requirements: 7.1, 11.2_
    - _Capability IDs: CAP-007, CAP-023_
    - _Depends on: 12.2, 29.1_
    - _Reads: projects/buzz/docs/nips/NIP-AP.md, projects/buzz/desktop/src/features/agents/**_
    - _Writes: crates/agent_settings/src/team.rs_
    - _Validation: tests cover duplicate member, revoked identity, public catalog and owner change_

  - [ ] 29.3. Define private managed-agent configuration
    - Model runtime, model, provider, environment references and PMA expected-version transitions without secret values.
    - _Requirements: 11.2, 19.2_
    - _Capability IDs: CAP-023_
    - _Depends on: 29.1, 29.2_
    - _Reads: projects/buzz/docs/nips/NIP-PMA.md, projects/buzz/desktop/src-tauri/src/managed-agents/**_
    - _Writes: crates/agent_settings/src/managed_agent.rs_
    - _Validation: tests cover CAS, invalid provider/model, secret-reference-only storage and stale update_

  - [ ] 29.4. Implement public/private agent projection rules
    - Derive redacted public persona/team/catalog records from private runnable state and reject secret projection.
    - _Requirements: 11.2, 19.2_
    - _Capability IDs: CAP-023_
    - _Depends on: 29.2, 29.3_
    - _Reads: crates/agent_settings/src/{team,managed-agent}.rs, projects/buzz/docs/nips/NIP-PMA.md_
    - _Writes: crates/collaboration_domain/src/agent_config.rs_
    - _Validation: exhaustive redaction test proves credentials/environment values never enter public events_

  - [ ] 29.5. Persist managed-agent state and snapshots
    - Store private versions and public projection provenance through canonical agent/settings owners.
    - _Requirements: 2.2, 11.2_
    - _Capability IDs: CAP-005, CAP-023, CAP-024_
    - _Depends on: 29.3, 29.4_
    - _Reads: crates/agent_settings/src/**, crates/agent/src/db.rs_
    - _Writes: crates/agent/src/managed_agents.rs_
    - _Validation: repository tests cover CAS, restart, projection rebuild and corrupt snapshot_

  - [ ] 29.6. Add native persona and team management UI
    - Render catalog, persona, team and managed-agent editing with privacy and validation feedback.
    - _Requirements: 4.4, 11.2_
    - _Capability IDs: CAP-023, CAP-036_
    - _Depends on: 29.5_
    - _Reads: crates/agent_ui/**, crates/agent/src/managed_agents.rs_
    - _Writes: crates/agent_ui/src/collaborative_agent_settings.rs_
    - _Validation: GPUI tests cover create, share, private edit, conflict, revoked owner and validation error_

- [ ] 30. Consolidate engrams, snapshots, archives and metrics

  - [ ] 30.1. Implement NIP-AE engram coordinate and encryption codecs
    - Preserve encrypted coordinates, relay scope and owner-read privacy independently of storage.
    - _Requirements: 5.3, 11.3_
    - _Capability IDs: CAP-024_
    - _Depends on: 11.8, 12.5_
    - _Reads: projects/buzz/docs/nips/NIP-AE.md, projects/buzz/desktop/src/features/agent-memory/**_
    - _Writes: crates/nostr_compat/src/agent_memory.rs_
    - _Validation: codec tests cover round trip, wrong owner, rotation and malformed coordinate_

  - [ ] 30.2. Implement canonical encrypted memory storage
    - Persist engram metadata/ciphertext and retention state without service-side plaintext access.
    - _Requirements: 11.3, 15.2_
    - _Capability IDs: CAP-005, CAP-024, CAP-030_
    - _Depends on: 30.1_
    - _Reads: crates/agent/src/db.rs, crates/nostr_compat/src/agent_memory.rs_
    - _Writes: crates/agent/src/memory.rs_
    - _Validation: storage tests cover owner read, ciphertext integrity, expiry and key rotation_

  - [ ] 30.3. Implement managed-agent snapshot lifecycle
    - Create, compare, restore and compact persona/team/runtime snapshots with stable provenance.
    - _Requirements: 11.3, 17.2_
    - _Capability IDs: CAP-023, CAP-024_
    - _Depends on: 29.5, 30.2_
    - _Reads: projects/buzz/desktop/src-tauri/src/managed-agents/**, crates/agent/src/managed_agents.rs_
    - _Writes: crates/agent/src/snapshot.rs_
    - _Validation: snapshot tests cover fidelity, stale restore, partial corruption and compaction_

  - [ ] 30.4. Implement private per-turn usage metrics
    - Preserve NIP-AM encrypted metrics and local accounting without enabling client telemetry.
    - _Requirements: 11.3, 19.5_
    - _Capability IDs: CAP-024, CAP-028_
    - _Depends on: 11.8, 28.2_
    - _Reads: projects/buzz/docs/nips/NIP-AM.md, crates/agent/src/db.rs_
    - _Writes: crates/agent/src/usage.rs_
    - _Validation: tests cover aggregation, encryption, retention, export and telemetry-disabled behavior_

  - [ ] 30.5. Import archives, memories, snapshots and metrics
    - Map previously staged desktop records into the canonical memory/snapshot/usage stores with source retention.
    - _Requirements: 11.3, 17.2, 17.3_
    - _Capability IDs: CAP-024, CAP-045_
    - _Depends on: 17.9, 30.2, 30.3, 30.4_
    - _Reads: crates/sim/src/migration/buzz/desktop_state.rs, crates/agent/src/{memory,snapshot,usage}.rs_
    - _Writes: crates/sim/src/migration/buzz/agent_state.rs_
    - _Validation: every archive fixture imports idempotently with content/privacy hashes and rollback evidence_

  - [ ] 30.6. Add agent-state privacy and fidelity conformance
    - Verify old/new export, relay rotation, compaction, retention and unauthorized access behavior.
    - _Requirements: 11.3, 20.2, 20.3_
    - _Capability IDs: CAP-024, CAP-044_
    - _Depends on: 30.5_
    - _Reads: crates/agent/src/{memory,snapshot,usage}.rs, .agents/specs/collaborative-workspace/fixtures/migrations/**_
    - _Writes: crates/agent/tests/agent_state_conformance.rs_
    - _Validation: `cargo test -p agent agent_state_conformance` passes legacy/new and unauthorized-reader cases_

- [ ] 31. Implement signed jobs and delegation

  - [ ] 31.1. Define the canonical job state machine
    - Model request, accept, progress, result, cancel and error transitions with idempotency/version invariants.
    - _Requirements: 11.4_
    - _Capability IDs: CAP-026_
    - _Depends on: 11.13, 29.2_
    - _Reads: projects/buzz/crates/buzz-core/src/kind.rs, crates/task/**_
    - _Writes: crates/collaboration_domain/src/job.rs_
    - _Validation: property tests enumerate legal transitions and reject duplicates/out-of-order terminal updates_

  - [ ] 31.2. Add job and executor-lease schema
    - Create job versions, delegation ancestry and exactly-one executor lease tables with recovery timestamps.
    - _Requirements: 2.1, 11.4, 11.5_
    - _Capability IDs: CAP-005, CAP-026_
    - _Depends on: 31.1_
    - _Reads: crates/collaboration_domain/src/job.rs, crates/collab/src/db/**_
    - _Writes: crates/collab/migrations/collaboration_jobs.sql_
    - _Validation: migration tests cover tenant fences, lease uniqueness, ancestry indexes and rollback_

  - [ ] 31.3. Enforce job and delegation authorization
    - Apply owner, team, role, scope and delegation-depth/resource policy to every transition.
    - _Requirements: 6.2, 11.4_
    - _Capability IDs: CAP-008, CAP-023, CAP-026_
    - _Depends on: 13.3, 29.2, 31.1_
    - _Reads: crates/collaboration_domain/src/{authorization,job}.rs_
    - _Writes: crates/collaboration_domain/src/job_authorization.rs_
    - _Validation: policy tests cover owner/team/service, revoked member, cycle, excessive depth and scope_

  - [ ] 31.4. Implement signed job Nostr adapter
    - Translate kinds 43001–43006 to canonical commands and exact compatibility responses.
    - _Requirements: 5.1, 11.4_
    - _Capability IDs: CAP-001, CAP-026_
    - _Depends on: 14.4, 31.1, 31.3_
    - _Reads: projects/buzz/crates/buzz-core/src/kind.rs, crates/collaboration_domain/src/job.rs_
    - _Writes: crates/nostr_compat/src/jobs.rs_
    - _Validation: golden job traces cover all transitions, duplicates, cancellation and malformed ancestry_

  - [ ] 31.5. Bind accepted jobs to native task/session execution
    - Acquire an executor lease, create/resume one Sim task/ACP session and publish terminal outcome.
    - _Requirements: 11.4, 11.5_
    - _Capability IDs: CAP-021, CAP-026_
    - _Depends on: 28.2, 31.4, 31.7_
    - _Reads: crates/agent/src/collaboration_session.rs, crates/collab/src/jobs/repository.rs_
    - _Writes: crates/agent/src/jobs.rs_
    - _Validation: execution tests cover accept, progress, cancel, crash, lease expiry and exactly-one result_

  - [ ] 31.6. Implement delegated child-job orchestration
    - Create authorized child jobs, preserve ancestry and aggregate outcomes without recursive duplicate execution.
    - _Requirements: 11.4_
    - _Capability IDs: CAP-026_
    - _Depends on: 31.3, 31.5_
    - _Reads: crates/agent/src/jobs.rs, projects/buzz/benchmarks/harbor-buzz-orchestra/**_
    - _Writes: crates/agent/src/job_delegation.rs_
    - _Validation: orchestration tests cover tree completion, partial failure, parent cancel, retry and cycle rejection_

  - [ ] 31.7. Implement the job and executor-lease repository
    - Read/write job transitions, ancestry and executor leases with idempotency and optimistic concurrency.
    - _Requirements: 2.1, 11.4, 11.5_
    - _Capability IDs: CAP-005, CAP-026_
    - _Depends on: 31.2_
    - _Reads: crates/collab/migrations/collaboration_jobs.sql, crates/collaboration_domain/src/job.rs_
    - _Writes: crates/collab/src/jobs/repository.rs_
    - _Validation: repository tests cover concurrent accept, expired lease, retry, transition conflict and tenant isolation_

- [ ] 32. Enrich the semantic activity feed

  - [ ] 32.1. Map NIP-AO observer events to ActivityItem
    - Convert supported observer states to existing semantic classes with generic fallback.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-025_
    - _Depends on: 8.6, 28.4_
    - _Reads: crates/agent_ui/src/activity_projection.rs, crates/nostr_compat/src/agent_observer.rs_
    - _Writes: crates/agent_ui/src/activity_observer.rs_
    - _Validation: fixture tests map every NIP-AO kind exactly once and redact raw encrypted content_

  - [ ] 32.2. Map message, presence and job events to ActivityItem
    - Add human/agent message, reply, waiting, presence and delegated-work outcomes without duplicate transcript rows.
    - _Requirements: 12.1, 12.3, 12.4_
    - _Capability IDs: CAP-011, CAP-014, CAP-026_
    - _Depends on: 19.7, 21.4, 31.5_
    - _Reads: crates/agent_ui/src/activity_projection.rs, crates/collaboration_domain/src/{message,presence,job}.rs_
    - _Writes: crates/agent_ui/src/activity_collaboration.rs_
    - _Validation: tests cover same-source reconciliation, wait/resume, terminal outcome and no duplicate message_

  - [ ] 32.3. Map Git, workflow, moderation and system events
    - Register consequence-weighted mappings while retaining a truthful raw fallback for unknown versions.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-020, CAP-025, CAP-027, CAP-029_
    - _Depends on: 27.4, 32.1_
    - _Reads: crates/agent_ui/src/{activity_projection,activity-git}.rs_
    - _Writes: crates/agent_ui/src/activity_platform.rs_
    - _Validation: mapping catalog test fails when a registered event lacks semantic or generic handling_

  - [ ] 32.4. Reconcile cross-source activity updates
    - Merge streaming, compatibility and authoritative lifecycle records by provenance without collapsing distinct actions.
    - _Requirements: 12.3, 12.4_
    - _Capability IDs: CAP-025_
    - _Depends on: 8.4, 32.1, 32.2, 32.3_
    - _Reads: crates/agent_ui/src/activity_{reducer,observer,collaboration,platform}.rs_
    - _Writes: crates/agent_ui/src/activity_reconciliation.rs_
    - _Validation: property tests cover reordered sources, duplicate frames, timeout, disconnect and late terminal state_

  - [ ] 32.5. Render complete semantic cards and interventions
    - Add progressive detail, raw protocol rail, permission/error actions and waiting-for-user treatment.
    - _Requirements: 4.4, 12.2, 12.4_
    - _Capability IDs: CAP-025, CAP-036_
    - _Depends on: 32.4_
    - _Reads: crates/agent_ui/src/collaborative_timeline.rs, crates/agent_ui/src/activity_reconciliation.rs_
    - _Writes: crates/agent_ui/src/collaborative_activity_cards.rs_
    - _Validation: GPUI tests cover thought, plan, search, edit, command, test, permission, error and intervention states_

  - [ ] 32.6. Add complete activity-catalog conformance
    - Verify every ACP, NIP-AO, message, Git, job and platform fixture maps exactly once and never goes blank.
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 20.1_
    - _Capability IDs: CAP-025, CAP-044_
    - _Depends on: 32.5_
    - _Reads: crates/agent_ui/src/activity_*.rs, .agents/specs/collaborative-workspace/fixtures/**_
    - _Writes: crates/agent_ui/tests/activity_catalog.rs_
    - _Validation: `cargo test -p agent_ui activity_catalog` reports zero unmapped or duplicate semantic records_

- [ ] 33. Merge remote-agent providers with Sim remote execution

  - [ ] 33.1. Port the remote-provider discovery contract
    - Discover versioned provider executables/configurations without loading provider code into the client process.
    - _Requirements: 11.5, 16.2_
    - _Capability IDs: CAP-034_
    - _Depends on: 4.2, 12.5_
    - _Reads: projects/buzz/docs/remote-agents.md, projects/buzz/crates/buzz-backend-kubernetes/**_
    - _Writes: crates/remote/src/agent_provider_discovery.rs_
    - _Validation: discovery tests cover supported, duplicate, missing, incompatible and untrusted providers_

  - [ ] 33.2. Parse and bound hostile provider output
    - Validate structured output, redact secrets and cap stdout/stderr/time/resources before state updates.
    - _Requirements: 16.2, 19.2_
    - _Capability IDs: CAP-034_
    - _Depends on: 33.1_
    - _Reads: projects/buzz/docs/remote-agents.md, crates/remote/src/agent_provider_discovery.rs_
    - _Writes: crates/remote/src/agent_provider_protocol.rs_
    - _Validation: fuzz/fixture tests cover malformed, oversized, secret-bearing and hanging providers_

  - [ ] 33.3. Implement provider deploy/inspect/terminate lifecycle
    - Run bounded provider commands and map results to one canonical remote-agent lifecycle.
    - _Requirements: 11.5, 16.2_
    - _Capability IDs: CAP-034_
    - _Depends on: 33.2_
    - _Reads: crates/remote/src/agent_provider_protocol.rs, crates/remote_connection/**_
    - _Writes: crates/remote/src/agent_provider_lifecycle.rs_
    - _Validation: lifecycle tests cover deploy, inspect, at-most-one instance, cancel, timeout and termination_

  - [ ] 33.4. Bind provider secrets and project configuration
    - Resolve canonical credential references at execution time and reject missing/empty identity without persisting values.
    - _Requirements: 7.3, 16.2, 19.2_
    - _Capability IDs: CAP-009, CAP-034_
    - _Depends on: 12.5, 33.3_
    - _Reads: crates/credentials_provider/**, crates/remote/src/agent_provider_lifecycle.rs_
    - _Writes: crates/agent/src/remote_provider_config.rs_
    - _Validation: tests cover locked keyring, missing key, redacted diagnostics and config/secret separation_

  - [ ] 33.5. Bind remote instances to jobs, sessions and presence
    - Acquire job/session ownership, surface substrate capabilities and expire presence without using it for control.
    - _Requirements: 11.4, 11.5, 16.2_
    - _Capability IDs: CAP-014, CAP-026, CAP-034_
    - _Depends on: 21.4, 31.5, 33.3, 33.4_
    - _Reads: crates/agent/src/{jobs,remote-provider-config}.rs, crates/collaboration_domain/src/presence.rs_
    - _Writes: crates/agent/src/remote_execution.rs_
    - _Validation: tests cover launch, heartbeat stale, result, cancellation, disconnect and exactly-one execution_

  - [ ] 33.6. Add provider L1–L3 conformance and cleanup tests
    - Validate discovery, deployment, job execution, hostile output and resource cleanup against Kubernetes fixtures.
    - _Requirements: 11.5, 16.2, 20.2_
    - _Capability IDs: CAP-034, CAP-044_
    - _Depends on: 33.5_
    - _Reads: projects/buzz/crates/buzz-backend-kubernetes/**, crates/agent/src/remote_execution.rs_
    - _Writes: crates/remote/tests/agent_provider_conformance.rs_
    - _Validation: `cargo test -p remote agent_provider_conformance` passes L1–L3, malicious and cleanup scenarios_

## Milestone 6 — platform and operational capability parity

- [ ] 34. Port and complete workflows and approvals

  - [ ] 34.1. Port workflow definition parsing and validation
    - Parse versioned YAML definitions, conditions, steps, secrets references and bounded retry policy.
    - _Requirements: 13.1, 13.3_
    - _Capability IDs: CAP-027_
    - _Depends on: 11.10, 31.3_
    - _Reads: projects/buzz/crates/buzz-workflow/**, projects/buzz/desktop/src/features/workflows/**_
    - _Writes: crates/collaboration_workflow/src/definition.rs_
    - _Validation: parser fixtures cover supported versions, invalid actions, secret literals and unbounded retries_

  - [ ] 34.2. Add workflow definition, run and step schema
    - Create tenant/project-scoped definition versions, runs, steps and retry records with provenance.
    - _Requirements: 2.2, 13.1_
    - _Capability IDs: CAP-005, CAP-027_
    - _Depends on: 15.3, 34.1_
    - _Reads: projects/buzz/crates/buzz-workflow/**, crates/collaboration_workflow/src/definition.rs_
    - _Writes: crates/collab/migrations/collaboration_workflows.sql_
    - _Validation: migration tests cover version keys, run/step relations, tenant fences and rollback_

  - [ ] 34.3. Implement cron and event trigger admission
    - Evaluate schedules and authorized collaboration events into stable run requests with bounded catch-up.
    - _Requirements: 13.1, 13.3_
    - _Capability IDs: CAP-027_
    - _Depends on: 34.1, 34.9_
    - _Reads: projects/buzz/crates/buzz-workflow/src/**, crates/collab/src/workflows/repository.rs_
    - _Writes: crates/collab/src/workflows/triggers.rs_
    - _Validation: trigger tests cover clock skew, missed interval, duplicate event, condition false and unauthorized source_

  - [ ] 34.4. Implement secure webhook trigger admission
    - Authenticate requests and enforce body, timeout, DNS/private-range and redirect policy before creating a run.
    - _Requirements: 13.1, 13.3, 19.2_
    - _Capability IDs: CAP-027_
    - _Depends on: 4.5, 34.9_
    - _Reads: projects/buzz/crates/buzz-workflow/**, .agents/specs/collaborative-workspace/security/agent-workflow.md_
    - _Writes: crates/collab/src/workflows/webhook.rs_
    - _Validation: webhook tests cover signature, SSRF, redirect, oversize, timeout and replay_

  - [ ] 34.5. Implement workflow action execution
    - Execute message, DM, channel-topic, agent-job and supported actions through canonical commands and permissions.
    - _Requirements: 13.1, 13.3_
    - _Capability IDs: CAP-011, CAP-026, CAP-027_
    - _Depends on: 19.2, 31.5, 34.9_
    - _Reads: projects/buzz/crates/buzz-workflow/**, crates/collaboration_domain/src/**_
    - _Writes: crates/collab/src/workflows/actions.rs_
    - _Validation: action tests complete formerly stubbed send-DM/topic actions and surface permission/failure outcomes_

  - [ ] 34.6. Implement durable approval suspend and resume
    - Atomically create one request, accept one authorized grant/deny and resume or terminate the waiting run.
    - _Requirements: 13.2, 13.3_
    - _Capability IDs: CAP-027_
    - _Depends on: 31.3, 34.5, 34.9_
    - _Reads: projects/buzz/crates/buzz-workflow/**, crates/collab/src/workflows/repository.rs_
    - _Writes: crates/collab/src/workflows/approval.rs_
    - _Validation: tests cover grant/deny race, stale approver, restart while waiting and duplicate response_

  - [ ] 34.7. Render native workflow and approval state
    - Render definitions, runs, steps, approval requests and scoped failures in the native task surface.
    - _Requirements: 8.3, 13.1, 13.2_
    - _Capability IDs: CAP-027, CAP-036_
    - _Depends on: 34.3, 34.4, 34.5, 34.6_
    - _Reads: crates/collab/src/workflows/**, crates/tasks_ui/**_
    - _Writes: crates/tasks_ui/src/workflows.rs_
    - _Validation: GPUI tests cover running, waiting approval, grant, deny, retry, redacted failure and unavailable service_

  - [ ] 34.8. Add workflow replay and crash-recovery scenarios
    - Verify deterministic trigger/action/approval recovery after executor, database or dependency failure.
    - _Requirements: 8.3, 13.1, 13.2, 13.3, 20.1_
    - _Capability IDs: CAP-027, CAP-044_
    - _Depends on: 34.3, 34.4, 34.5, 34.6, 34.7_
    - _Reads: crates/collab/src/workflows/**, crates/tasks_ui/src/workflows.rs_
    - _Writes: crates/collab/tests/workflow_recovery.rs_
    - _Validation: workflow E2E covers cron/webhook/event, approval race, retry, crash, restart and redacted failure_

  - [ ] 34.9. Implement the workflow repository
    - Read/write versioned definitions, runs, steps, retries and waiting approvals with idempotency.
    - _Requirements: 2.2, 13.1, 13.2_
    - _Capability IDs: CAP-005, CAP-027_
    - _Depends on: 34.2_
    - _Reads: crates/collab/migrations/collaboration_workflows.sql, crates/collaboration_workflow/src/definition.rs_
    - _Writes: crates/collab/src/workflows/repository.rs_
    - _Validation: repository tests cover definition version, run restart, duplicate trigger, approval wait and tenant fence_

- [ ] 35. Port audit chains and usage accounting

  - [ ] 35.1. Implement canonical audit-entry hashing
    - Canonicalize redacted entries and link per-community hashes under a single writer invariant.
    - _Requirements: 13.4, 19.2_
    - _Capability IDs: CAP-028_
    - _Depends on: 4.4, 11.1_
    - _Reads: projects/buzz/crates/buzz-audit/**, projects/buzz/crates/buzz-datastore-tracing/**_
    - _Writes: crates/collaboration_domain/src/audit.rs_
    - _Validation: hash vectors cover canonical ordering, redaction, mutation detection and chain bridge_

  - [ ] 35.2. Add audit-chain persistence schema
    - Create immutable entry, per-community head and export-cursor tables.
    - _Requirements: 6.1, 13.4_
    - _Capability IDs: CAP-005, CAP-028_
    - _Depends on: 15.7, 35.1_
    - _Reads: crates/collaboration_domain/src/audit.rs, crates/collab/src/db/**_
    - _Writes: crates/collab/migrations/collaboration_audit.sql_
    - _Validation: migration tests cover immutable fields, one head per community, tenant fences and rollback_

  - [ ] 35.3. Record security and administrative operations
    - Attach stable operation IDs to auth, membership, moderation, workflow and migration audit events.
    - _Requirements: 13.4, 15.1_
    - _Capability IDs: CAP-008, CAP-027, CAP-028, CAP-029, CAP-045_
    - _Depends on: 34.6, 35.6_
    - _Reads: crates/collab/src/{tenant_admission,workflows}/**, crates/collab/src/audit/repository.rs_
    - _Writes: crates/collab/src/audit/events.rs_
    - _Validation: audit integration test records attributable success/failure without private payloads_

  - [ ] 35.4. Consolidate agent and workflow usage accounting
    - Aggregate canonical job/turn/workflow records by tenant while preserving private NIP-AM semantics.
    - _Requirements: 11.3, 13.4, 19.5_
    - _Capability IDs: CAP-024, CAP-027, CAP-028_
    - _Depends on: 30.4, 34.9, 35.6_
    - _Reads: crates/agent/src/usage.rs, crates/collab/src/workflows/repository.rs_
    - _Writes: crates/collab/src/audit/usage.rs_
    - _Validation: usage tests cover retry deduplication, redaction, export and disabled client telemetry_

  - [ ] 35.5. Add audit verification, export and tamper tests
    - Verify full/segment chains, bridge imported heads and emit operator-safe diagnostics on corruption.
    - _Requirements: 13.4, 19.3, 20.1_
    - _Capability IDs: CAP-028, CAP-044_
    - _Depends on: 35.3, 35.4, 35.6_
    - _Reads: crates/collab/src/audit/**, .agents/specs/collaborative-workspace/fixtures/migrations/**_
    - _Writes: crates/collab/tests/audit_chain.rs_
    - _Validation: `cargo test -p collab audit_chain` detects deletion, reorder, mutation and wrong imported head_

  - [ ] 35.6. Implement the audit-chain repository
    - Append one serialized per-community chain, read/export segments and reject stale or cross-tenant heads.
    - _Requirements: 6.1, 13.4_
    - _Capability IDs: CAP-005, CAP-028_
    - _Depends on: 35.2_
    - _Reads: crates/collab/migrations/collaboration_audit.sql, crates/collaboration_domain/src/audit.rs_
    - _Writes: crates/collab/src/audit/repository.rs_
    - _Validation: concurrent writer tests preserve one valid chain and reject stale/cross-community heads_

- [ ] 36. Port moderation and administration

  - [ ] 36.1. Define report, mute, ban and timeout state machines
    - Model personal mute separately from role-gated reports, bans, timeouts and resolution.
    - _Requirements: 15.1, 15.4_
    - _Capability IDs: CAP-029_
    - _Depends on: 18.3, 35.1_
    - _Reads: projects/buzz/VISION_MODERATION.md, projects/buzz/crates/buzz-db/src/moderation.rs_
    - _Writes: crates/collaboration_domain/src/moderation.rs_
    - _Validation: state tests cover report, resolve, timeout expiry, ban, personal mute and stale actor_

  - [ ] 36.2. Persist moderation and archive records
    - Add tenant-fenced reports, actions, resolutions and identity/community archive state with provenance.
    - _Requirements: 2.2, 15.1_
    - _Capability IDs: CAP-005, CAP-029, CAP-030_
    - _Depends on: 35.2, 36.1_
    - _Reads: projects/buzz/migrations/0006-*.sql, crates/collaboration_domain/src/moderation.rs_
    - _Writes: crates/collab/migrations/collaboration_moderation.sql_
    - _Validation: migration tests cover active uniqueness, history, tenant fences and rollback_

  - [ ] 36.3. Enforce moderation at common authorization boundaries
    - Apply bans, timeouts and archives consistently before reads/writes while retaining historical attribution.
    - _Requirements: 6.2, 7.4, 15.1, 15.4_
    - _Capability IDs: CAP-007, CAP-008, CAP-029_
    - _Depends on: 13.3, 36.1, 36.2_
    - _Reads: crates/collaboration_domain/src/{authorization,moderation}.rs_
    - _Writes: crates/collaboration_domain/src/moderation_policy.rs_
    - _Validation: policy tests cover timeout expiry, ban, archive, historical read and fail-closed ambiguity_

  - [ ] 36.4. Implement operator moderation APIs
    - Expose least-privilege list/report/resolve/ban/timeout/archive operations with audit attribution.
    - _Requirements: 15.1, 15.4_
    - _Capability IDs: CAP-028, CAP-029, CAP-041_
    - _Depends on: 35.3, 36.3_
    - _Reads: projects/buzz/crates/buzz-admin/**, crates/collab/src/audit/events.rs_
    - _Writes: crates/collab/src/admin/moderation.rs_
    - _Validation: API tests cover role matrix, tenant mismatch, stale action and redacted partial failure_

  - [ ] 36.5. Add native moderation queue UI
    - Render reports, evidence summaries, actions and resolution state with accessible confirmation/failure flows.
    - _Requirements: 4.4, 15.1, 15.4_
    - _Capability IDs: CAP-029, CAP-036_
    - _Depends on: 36.4_
    - _Reads: projects/buzz/desktop/src/features/moderation/**, crates/collab_ui/src/**_
    - _Writes: crates/collab_ui/src/moderation.rs_
    - _Validation: GPUI tests cover empty queue, resolve, denied action, stale report and partial service failure_

  - [ ] 36.6. Port moderation commands to Sim CLI
    - Add report/list/resolve/ban/timeout/archive commands with compatible machine output and exit classes.
    - _Requirements: 15.1, 16.4_
    - _Capability IDs: CAP-029, CAP-038, CAP-041_
    - _Depends on: 36.4_
    - _Reads: projects/buzz/crates/buzz-admin/**, projects/buzz/crates/buzz-cli/**, crates/cli/**_
    - _Writes: crates/cli/src/collaboration_moderation.rs_
    - _Validation: golden CLI tests cover output, denied/stale errors, redaction and exit codes_

  - [ ] 36.7. Add moderation and administration security scenarios
    - Exercise personal mute, role enforcement, operator API, native UI and CLI across tenants and archived identities.
    - _Requirements: 6.3, 15.1, 15.4, 20.1_
    - _Capability IDs: CAP-029, CAP-041, CAP-044_
    - _Depends on: 36.5, 36.6_
    - _Reads: crates/collab/src/admin/moderation.rs, crates/collab_ui/src/moderation.rs_
    - _Writes: crates/collab/tests/moderation_administration.rs_
    - _Validation: E2E suite reports no cross-tenant evidence, role bypass or unaudited action_

- [ ] 37. Port retention, deletion and recovery state machines

  - [ ] 37.1. Define canonical retention and expiry policy
    - Resolve event kind, community policy, legal hold, ephemeral and archive rules into one disposition.
    - _Requirements: 15.2, 19.2_
    - _Capability IDs: CAP-030_
    - _Depends on: 11.5, 36.3_
    - _Reads: projects/buzz/crates/buzz-deletion/**, projects/buzz/migrations/0007-*.sql_
    - _Writes: crates/collaboration_domain/src/retention.rs_
    - _Validation: policy tests cover TTL, archive, legal hold, mixed version and retry_

  - [ ] 37.2. Implement authoritative retention worker
    - Apply policy to event/projection authority in bounded tenant batches with resumable checkpoints.
    - _Requirements: 15.2, 17.2_
    - _Capability IDs: CAP-005, CAP-030_
    - _Depends on: 17.1, 37.1_
    - _Reads: crates/collaboration_domain/src/retention.rs, crates/collab/src/db/collaboration/**_
    - _Writes: crates/collab/src/retention/worker.rs_
    - _Validation: worker tests cover interruption, retry, hold, partial batch and idempotent resume_

  - [ ] 37.3. Converge search projections after retention
    - Remove or hide expired authoritative records from collaboration search using source/version checkpoints.
    - _Requirements: 15.2, 17.4_
    - _Capability IDs: CAP-015, CAP-030_
    - _Depends on: 22.1, 37.2_
    - _Reads: crates/collab/src/search/**, crates/collab/src/retention/worker.rs_
    - _Writes: crates/collab/src/retention/search_cleanup.rs_
    - _Validation: search cleanup tests converge after delayed, duplicate and interrupted delivery_

  - [ ] 37.4. Define durable whole-community deletion transitions
    - Model requested, verified, reversible, irreversible, completed and failed states with authority evidence.
    - _Requirements: 15.3, 15.4_
    - _Capability IDs: CAP-030, CAP-041_
    - _Depends on: 36.3, 37.1_
    - _Reads: projects/buzz/crates/buzz-deletion/**, projects/buzz/migrations/0029-*.sql_
    - _Writes: crates/collaboration_domain/src/community_deletion.rs_
    - _Validation: state tests reject partial tenant reuse, stale authority and invalid rollback transitions_

  - [ ] 37.5. Implement checkpointed community deletion executor
    - Execute database, search, cache, push, object and Git phases with recorded irreversible boundary.
    - _Requirements: 15.3, 17.2, 17.3_
    - _Capability IDs: CAP-030, CAP-041, CAP-045_
    - _Depends on: 17.4, 37.3, 37.4, 37.8, 37.9_
    - _Reads: projects/buzz/migrations/0030-*.sql, crates/collaboration_domain/src/community_deletion.rs_
    - _Writes: crates/collab/src/deletion/executor.rs_
    - _Validation: executor tests fault every phase and resume without skipping or repeating irreversible work_

  - [ ] 37.6. Implement pre-irreversible recovery and operator status
    - Restore reversible checkpoints and expose redacted progress, halt reason and recovery action.
    - _Requirements: 8.3, 15.3, 15.4, 17.3_
    - _Capability IDs: CAP-030, CAP-041_
    - _Depends on: 37.5_
    - _Reads: crates/collab/src/deletion/executor.rs, crates/collab/src/admin/**_
    - _Writes: crates/collab/src/deletion/recovery.rs_
    - _Validation: recovery tests restore every reversible phase and refuse recovery beyond the recorded boundary_

  - [ ] 37.7. Add mixed-version retention and deletion fault suite
    - Exercise legacy/new workers, retries, archive, expiry, recovery and irreversible completion independently.
    - _Requirements: 15.2, 15.3, 15.4, 20.1_
    - _Capability IDs: CAP-030, CAP-044_
    - _Depends on: 37.3, 37.6_
    - _Reads: crates/collab/src/{retention,deletion}/**, .agents/specs/collaborative-workspace/fixtures/migrations/**_
    - _Writes: crates/collab/tests/retention_deletion_faults.rs_
    - _Validation: isolated fault suite reaches one correct final state for every injected failure_

  - [ ] 37.8. Converge Redis cache and push queues after retention
    - Invalidate derived cache/presence keys and cancel obsolete wake jobs without treating either as authority.
    - _Requirements: 15.2, 17.4_
    - _Capability IDs: CAP-006, CAP-016, CAP-030_
    - _Depends on: 22.13, 37.2_
    - _Reads: crates/collab/src/{pubsub,push}/**, crates/collab/src/retention/worker.rs_
    - _Writes: crates/collab/src/retention/cache_push_cleanup.rs_
    - _Validation: cleanup tests cover unavailable Redis/gateway, retry, duplicate invalidation and final visibility_

  - [ ] 37.9. Clean retained media references and verified orphans
    - Remove expired attachment metadata and delete bytes only after content-reference verification.
    - _Requirements: 14.1, 15.2, 17.4_
    - _Capability IDs: CAP-030, CAP-031_
    - _Depends on: 37.2, 38.4_
    - _Reads: crates/collab/src/media/object_store.rs, crates/collab/src/retention/worker.rs_
    - _Writes: crates/collab/src/retention/media_cleanup.rs_
    - _Validation: tests preserve shared content, remove true orphans and resume safely after object-store failure_

- [ ] 38. Merge media storage and Blossom compatibility

  - [ ] 38.1. Define canonical media and attachment metadata
    - Model content hash, type, size, tenant owner, variants and message links without object credentials.
    - _Requirements: 14.1, 14.2_
    - _Capability IDs: CAP-031_
    - _Depends on: 11.1, 19.2_
    - _Reads: projects/buzz/crates/buzz-media/**, crates/media/**_
    - _Writes: crates/collaboration_domain/src/media.rs_
    - _Validation: metadata tests cover hash identity, attachment link, variant and invalid tenant path_

  - [ ] 38.2. Implement authenticated upload admission
    - Authorize tenant/user, bound request size and issue an upload operation without exposing storage credentials.
    - _Requirements: 6.2, 14.1, 19.2_
    - _Capability IDs: CAP-031_
    - _Depends on: 13.3, 38.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/api/media.rs, crates/http_client/**_
    - _Writes: crates/collab/src/media/upload_admission.rs_
    - _Validation: tests cover unauthorized, wrong tenant, oversize, replay and expired admission_

  - [ ] 38.3. Implement MIME, magic-byte and content validation
    - Stream bounded uploads, verify declared/observed type, hash and supported decoding before commit.
    - _Requirements: 14.1, 19.2_
    - _Capability IDs: CAP-031_
    - _Depends on: 4.3, 38.2_
    - _Reads: projects/buzz/crates/buzz-media/**, crates/media/**_
    - _Writes: crates/collab/src/media/validation.rs_
    - _Validation: corpus tests cover polyglot, truncated, oversized, hash mismatch and supported files_

  - [ ] 38.4. Implement tenant-scoped object storage and cleanup
    - Store validated bytes by content identity, serve authorized ranges and remove verified orphans.
    - _Requirements: 14.1, 15.2_
    - _Capability IDs: CAP-030, CAP-031_
    - _Depends on: 38.1, 38.3_
    - _Reads: projects/buzz/crates/buzz-media/**, crates/collab/src/media/validation.rs_
    - _Writes: crates/collab/src/media/object_store.rs_
    - _Validation: tests cover range, duplicate hash, tenant fence, missing object and safe orphan cleanup_

  - [ ] 38.5. Implement thumbnail and native rendering integration
    - Generate bounded variants and render image/audio/video/link attachments through existing Sim media components.
    - _Requirements: 14.2, 4.4_
    - _Capability IDs: CAP-031, CAP-036_
    - _Depends on: 38.4_
    - _Reads: crates/media/**, crates/collab/src/media/object_store.rs_
    - _Writes: crates/media/src/collaboration.rs_
    - _Validation: renderer tests cover thumbnail failure, unsupported media, accessibility label and missing variant_

  - [ ] 38.6. Implement the Blossom compatibility adapter
    - Preserve authenticated Blossom upload/download and URL-alias contracts over canonical media storage.
    - _Requirements: 5.2, 14.2_
    - _Capability IDs: CAP-002, CAP-031_
    - _Depends on: 38.2, 38.4, 38.5_
    - _Reads: projects/buzz/crates/buzz-media/**, crates/collab/src/media/**_
    - _Writes: crates/nostr_compat/src/blossom.rs_
    - _Validation: adapter tests cover signed upload, download/range, alias, authorization and protocol errors_

  - [ ] 38.7. Add old/new media compatibility conformance
    - Compare Buzz and consolidated media paths for uploads, downloads, rendering metadata and failure behavior.
    - _Requirements: 14.1, 14.2, 20.2_
    - _Capability IDs: CAP-031, CAP-044_
    - _Depends on: 38.5, 38.6_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/nostr_compat/src/blossom.rs_
    - _Writes: crates/collab/tests/media_conformance.rs_
    - _Validation: old/new clients match content hash, ranges, URL aliases, authorization and errors_

- [ ] 39. Consolidate huddles, voice, TTS and transcription

  - [ ] 39.1. Define transport-neutral huddle lifecycle
    - Model start, join, leave, end, participant, reaction and transcript references under ADR-004.
    - _Requirements: 14.3_
    - _Capability IDs: CAP-032_
    - _Depends on: 2.4, 18.4_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-004-huddle-transport.md, projects/buzz/crates/buzz-relay/src/audio/**_
    - _Writes: crates/collaboration_domain/src/huddle.rs_
    - _Validation: lifecycle tests cover duplicate join/leave, owner disconnect, end and transcript linkage_

  - [ ] 39.2. Implement the approved native huddle transport adapter
    - Map lifecycle and participants to the ADR-selected Sim audio transport with bounded cleanup.
    - _Requirements: 14.3, 19.2_
    - _Capability IDs: CAP-032_
    - _Depends on: 2.4, 4.7, 39.1_
    - _Reads: crates/livekit_api/**, crates/livekit_client/**, crates/collaboration_domain/src/huddle.rs_
    - _Writes: crates/audio/src/collaboration_huddle.rs_
    - _Validation: transport tests cover connect, reconnect, participant sync, end and resource cleanup_

  - [ ] 39.3. Implement the Buzz audio compatibility adapter
    - Translate supported Opus/WebSocket lifecycle and audio controls to the canonical huddle domain.
    - _Requirements: 5.2, 14.3, 18.2_
    - _Capability IDs: CAP-032_
    - _Depends on: 2.4, 4.7, 39.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/audio/**, projects/buzz/desktop/src-tauri/src/huddle/**_
    - _Writes: crates/collab/src/huddle/buzz_audio.rs_
    - _Validation: differential tests cover join/leave/end, invalid frame, version error and compatibility-window behavior_

  - [ ] 39.4. Implement microphone, speaker, mute and push-to-talk controls
    - Bind device selection and audio controls to native transport while preserving huddle state on device failure.
    - _Requirements: 14.3, 14.4_
    - _Capability IDs: CAP-032_
    - _Depends on: 39.2_
    - _Reads: crates/audio/**, crates/audio/src/collaboration_huddle.rs_
    - _Writes: crates/collab_ui/src/huddle_controls.rs_
    - _Validation: GPUI/device tests cover mute, PTT, device loss/switch, permission denial and safe retry_

  - [ ] 39.5. Integrate local TTS voice models
    - Port model selection and bounded synthesis behind a cancellable native service with visible failure.
    - _Requirements: 14.4, 19.2_
    - _Capability IDs: CAP-032_
    - _Depends on: 4.7, 39.1_
    - _Reads: projects/buzz/crates/buzz-voice/**, projects/buzz/desktop/src-tauri/src/huddle/**_
    - _Writes: crates/audio/src/collaboration_tts.rs_
    - _Validation: tests cover missing model, invalid voice, cancellation, output bound and recovery_

  - [ ] 39.6. Implement transcription-to-channel projection
    - Convert bounded transcript segments into canonical channel records with provenance and retention.
    - _Requirements: 14.3, 14.4, 15.2_
    - _Capability IDs: CAP-011, CAP-030, CAP-032_
    - _Depends on: 19.2, 39.1_
    - _Reads: projects/buzz/crates/buzz-voice/**, crates/collaboration_domain/src/{huddle,message}.rs_
    - _Writes: crates/collab/src/huddle/transcription.rs_
    - _Validation: tests cover partial/final segments, failure, retry, redaction and retention expiry_

  - [ ] 39.7. Add the native huddle workspace UI
    - Render lifecycle, participants, reactions, controls, transcript and scoped retry/fallback states.
    - _Requirements: 4.4, 14.3, 14.4_
    - _Capability IDs: CAP-032, CAP-036_
    - _Depends on: 39.3, 39.4, 39.5, 39.6_
    - _Reads: crates/collab_ui/src/huddle_controls.rs, crates/collab/src/huddle/**_
    - _Writes: crates/collab_ui/src/huddle.rs_
    - _Validation: GPUI tests cover start/join/leave/end, device/model/network failures and transcript display_

  - [ ] 39.8. Add native and Buzz huddle interoperability scenarios
    - Exercise equivalent lifecycle, audio controls, failures and transcripts through both supported transports.
    - _Requirements: 14.3, 14.4, 20.1, 20.2_
    - _Capability IDs: CAP-032, CAP-044_
    - _Depends on: 39.3, 39.7_
    - _Reads: crates/collab_ui/src/huddle.rs, crates/collab/src/huddle/**_
    - _Writes: crates/collab_ui/tests/huddle.rs_
    - _Validation: E2E produces equivalent canonical events and safe failure/recovery for native and Buzz clients_

- [ ] 40. Port pairing into canonical credential storage

  - [ ] 40.1. Port NIP-AB pairing cryptography and session codec
    - Implement QR/session, expiry, replay and encrypted transfer primitives independently of transport/storage.
    - _Requirements: 5.1, 16.1_
    - _Capability IDs: CAP-033_
    - _Depends on: 11.3, 12.6_
    - _Reads: projects/buzz/crates/buzz-core/src/pairing/**, projects/buzz/docs/nips/NIP-AB.md_
    - _Writes: crates/nostr_compat/src/pairing.rs_
    - _Validation: vectors cover round trip, wrong secret, expiry, replay, corrupted QR and version mismatch_

  - [ ] 40.2. Implement the ephemeral pairing relay
    - Relay bounded opaque session frames with expiry and no durable identity authority.
    - _Requirements: 8.4, 16.1, 19.2_
    - _Capability IDs: CAP-033_
    - _Depends on: 40.1_
    - _Reads: projects/buzz/crates/buzz-pair-relay/**, crates/nostr_compat/src/pairing.rs_
    - _Writes: services/pair_relay/src/main.rs_
    - _Validation: relay tests cover expiry, capacity, replay, disconnect cleanup and zero persisted key material_

  - [ ] 40.3. Import paired identities into Sim credentials
    - Verify received identity, store it canonically and preserve prior credentials on any failure.
    - _Requirements: 7.2, 7.3, 16.1_
    - _Capability IDs: CAP-009, CAP-033_
    - _Depends on: 12.4, 40.1, 40.2_
    - _Reads: crates/sim_credentials_provider/src/nostr_import.rs, crates/nostr_compat/src/pairing.rs_
    - _Writes: crates/sim_credentials_provider/src/pairing.rs_
    - _Validation: credential tests cover verified import, interruption, locked keyring and source preservation_

  - [ ] 40.4. Port the pairing CLI
    - Preserve create, receive, cancel and status syntax, output and exit contracts over canonical sessions.
    - _Requirements: 16.1, 16.4_
    - _Capability IDs: CAP-033, CAP-038_
    - _Depends on: 40.2, 40.3_
    - _Reads: projects/buzz/crates/buzz-pairing-cli/**, crates/cli/**_
    - _Writes: crates/cli/src/pairing.rs_
    - _Validation: golden CLI tests cover create, receive, expiry, cancel, error output and no secret logging_

  - [ ] 40.5. Add desktop/mobile/CLI pairing interoperability
    - Exercise every supported sender/receiver combination and interrupted recovery with frozen clients.
    - _Requirements: 16.1, 18.1, 20.2_
    - _Capability IDs: CAP-033, CAP-040, CAP-044_
    - _Depends on: 40.4, 40.6_
    - _Reads: .agents/specs/collaborative-workspace/fixtures/clients/**, services/pair_relay/src/main.rs_
    - _Writes: crates/collab/tests/pairing_interop.rs_
    - _Validation: interoperability matrix passes expiry, replay, corrupt QR, cancel and successful verified import_

  - [ ] 40.6. Add the native pairing QR flow
    - Render accessible QR display, scan/import confirmation, expiry, cancellation and safe failure states.
    - _Requirements: 4.4, 16.1_
    - _Capability IDs: CAP-033, CAP-036, CAP-040_
    - _Depends on: 40.2, 40.3_
    - _Reads: projects/buzz/mobile/lib/features/pairing/**, crates/collab_ui/src/**_
    - _Writes: crates/collab_ui/src/pairing.rs_
    - _Validation: GPUI tests cover display/scan, expiry, corrupt QR, cancel, confirmation and locked keyring_

- [ ] 41. Port relay mesh and shared-compute scheduling

  - [ ] 41.1. Port fenced mesh wire and membership protocol
    - Implement ADR-006-approved version, community membership, peer identity and replay rules over Iroh.
    - _Requirements: 5.1, 16.3, 19.2_
    - _Capability IDs: CAP-035_
    - _Depends on: 2.6, 4.8, 13.3_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-006-shared-compute.md, projects/buzz/crates/buzz-relay-mesh/**_
    - _Writes: crates/remote/src/mesh/protocol.rs_
    - _Validation: protocol tests cover version, replay, revoked membership, partition and malformed gossip_

  - [ ] 41.2. Implement compute advertisements and expiry
    - Validate approved capabilities/resources and fence stale advertisements without granting execution.
    - _Requirements: 16.3, 19.2_
    - _Capability IDs: CAP-035_
    - _Depends on: 2.5, 41.1_
    - _Reads: projects/buzz/desktop/src/features/mesh-compute/**, crates/remote/src/mesh/protocol.rs_
    - _Writes: crates/remote/src/mesh/advertisement.rs_
    - _Validation: tests cover spoofed resource, expiry, revoke, duplicate peer and bounded advertisement size_

  - [ ] 41.3. Integrate approved mesh scheduling with remote jobs
    - Select only eligible providers under trust/resource policy and acquire canonical job/executor leases.
    - _Requirements: 11.5, 16.3_
    - _Capability IDs: CAP-026, CAP-034, CAP-035_
    - _Depends on: 2.5, 33.5, 41.2_
    - _Reads: crates/agent/src/remote_execution.rs, crates/remote/src/mesh/advertisement.rs_
    - _Writes: crates/remote/src/mesh/scheduler.rs_
    - _Validation: scheduler tests cover eligibility, fairness, capacity, revoked peer and no silent fallback_

  - [ ] 41.4. Add shared-compute availability and failure UI
    - Render eligible capacity, execution location, no-capacity and unapproved-provider states without enabling policy changes.
    - _Requirements: 4.4, 16.3_
    - _Capability IDs: CAP-035, CAP-036_
    - _Depends on: 41.3_
    - _Reads: crates/remote/src/mesh/scheduler.rs, crates/collab_ui/src/**_
    - _Writes: crates/collab_ui/src/mesh_compute.rs_
    - _Validation: GPUI tests cover available, stale, revoked, no capacity and provider failure states_

  - [ ] 41.5. Add mesh partition, security and load scenarios
    - Exercise gossip partitions, replay, revocation, resource caps, scheduler fairness and recovery under load.
    - _Requirements: 8.3, 16.3, 19.3, 20.1_
    - _Capability IDs: CAP-035, CAP-044_
    - _Depends on: 41.3, 41.4_
    - _Reads: projects/buzz/perf/**, crates/remote/src/mesh/**_
    - _Writes: crates/remote/tests/mesh_compute.rs_
    - _Validation: approved mesh test command meets partition-recovery, resource and fairness budgets_

## Milestone 7 — client cutover, operations, retirement and parity

- [ ] 42. Merge agent-first collaboration commands into Sim CLI

  - [ ] 42.1. Define canonical CLI command and error contracts
    - Map Buzz command groups, global options, compact output and exit classes to Sim-owned operations.
    - _Requirements: 16.4, 18.1, 18.2_
    - _Capability IDs: CAP-038, CAP-042_
    - _Depends on: 3.2, 14.1_
    - _Reads: projects/buzz/crates/buzz-cli/**, crates/cli/**, .agents/specs/collaborative-workspace/fixtures/clients/**_
    - _Writes: crates/cli/src/collaboration/contracts.rs_
    - _Validation: contract manifest accounts for every frozen command, option, output stream and exit code_

  - [ ] 42.2. Port identity and community CLI commands
    - Implement profile, social, community, member and invite commands through canonical APIs.
    - _Requirements: 6.4, 7.1, 16.4_
    - _Capability IDs: CAP-007, CAP-008, CAP-038_
    - _Depends on: 18.6, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/community.rs_
    - _Validation: golden tests cover profile, membership, invite, permission, compact output and exit class_

  - [ ] 42.3. Port message and thread CLI commands
    - Implement message, reply, edit, delete, reaction, pin, bookmark and scheduled-message commands.
    - _Requirements: 9.1, 9.2, 16.4_
    - _Capability IDs: CAP-011, CAP-038_
    - _Depends on: 20.5, 23.7, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/messages.rs_
    - _Validation: golden tests cover paging, reply, edit/delete, reactions, schedules and stable errors_

  - [ ] 42.4. Port project and repository CLI commands
    - Implement project, repository, ref, clone and hosted-repository operations through canonical APIs.
    - _Requirements: 10.1, 10.2, 16.4_
    - _Capability IDs: CAP-018, CAP-019, CAP-038_
    - _Depends on: 25.8, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/git.rs_
    - _Validation: golden CLI suite covers project grouping, clone coordinates, refs, hosting and permission errors_

  - [ ] 42.5. Port agent, persona, memory and job CLI commands
    - Implement agent/team/persona/memory/snapshot/job commands through canonical agent owners.
    - _Requirements: 11.2, 11.3, 11.4, 16.4_
    - _Capability IDs: CAP-023, CAP-024, CAP-026, CAP-038_
    - _Depends on: 30.6, 31.6, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/agents.rs_
    - _Validation: golden tests cover lifecycle, private-state redaction, delegation, cancellation and stable exit codes_

  - [ ] 42.6. Implement and verify the versioned buzz CLI shim
    - Forward legacy syntax to Sim commands, preserve stdout/stderr/retry semantics and emit minimum-version errors.
    - _Requirements: 18.1, 18.2, 18.3_
    - _Capability IDs: CAP-038, CAP-045_
    - _Depends on: 42.2, 42.3, 42.4, 42.5, 42.7, 42.8, 42.9, 42.10, 42.11, 42.12_
    - _Reads: crates/cli/src/collaboration/**, .agents/specs/collaborative-workspace/fixtures/clients/**_
    - _Writes: tools/buzz_compat/Cargo.toml, tools/buzz_compat/src/*_
    - _Validation: frozen automation scripts produce approved output/exit codes against old and consolidated endpoints_

  - [ ] 42.7. Port workflow CLI commands
    - Implement definition, trigger, run, approval and cancellation commands through canonical workflow APIs.
    - _Requirements: 13.1, 13.2, 13.3, 16.4_
    - _Capability IDs: CAP-027, CAP-038_
    - _Depends on: 34.6, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/workflows.rs_
    - _Validation: golden tests cover trigger, waiting approval, grant/deny, retry failure, cancellation and exit codes_

  - [ ] 42.8. Consolidate moderation CLI commands
    - Integrate the focused moderation commands into the common collaboration command/output contract.
    - _Requirements: 15.1, 15.4, 16.4_
    - _Capability IDs: CAP-029, CAP-038, CAP-041_
    - _Depends on: 36.6, 42.1_
    - _Reads: crates/cli/src/collaboration_moderation.rs, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/moderation.rs_
    - _Validation: golden tests preserve role errors, redaction, output schemas and exit codes_

  - [ ] 42.9. Port media CLI commands
    - Implement authenticated upload, download, metadata and attachment commands through canonical media APIs.
    - _Requirements: 14.1, 14.2, 16.4_
    - _Capability IDs: CAP-031, CAP-038_
    - _Depends on: 38.7, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/media.rs_
    - _Validation: golden tests cover upload/download, unsupported media, permission denial, progress and exit codes_

  - [ ] 42.10. Port channel CLI commands
    - Implement channel create/update/archive, membership, template, topic and canvas commands.
    - _Requirements: 6.4, 9.1, 16.4_
    - _Capability IDs: CAP-010, CAP-038_
    - _Depends on: 18.7, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/channels.rs_
    - _Validation: golden tests cover types, roles, invites, template/topic/canvas, archive and exit codes_

  - [ ] 42.11. Port DM, read-state and social CLI commands
    - Implement encrypted DM, read/unread, reminder, forum, emoji and feedback commands through privacy-aware APIs.
    - _Requirements: 9.1, 9.3, 16.4_
    - _Capability IDs: CAP-012, CAP-013, CAP-017, CAP-038_
    - _Depends on: 20.5, 23.7, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/social.rs_
    - _Validation: golden tests cover encrypted DM, unread, reminders, forum/emoji, privacy-safe errors and exit codes_

  - [ ] 42.12. Port patch, pull-request, issue and review CLI commands
    - Implement collaboration records, CI status, review and approval commands against canonical Git records.
    - _Requirements: 10.2, 10.3, 10.4, 16.4_
    - _Capability IDs: CAP-019, CAP-020, CAP-038_
    - _Depends on: 27.7, 42.1_
    - _Reads: projects/buzz/crates/buzz-cli/src/**, crates/cli/src/collaboration/contracts.rs_
    - _Writes: crates/cli/src/collaboration/review.rs_
    - _Validation: golden tests cover patch/PR/issue/status/review, stale conflict, denied approval and exit codes_

- [ ] 43. Repoint web, mobile and admin clients with version negotiation

  - [ ] 43.1. Publish the compatibility matrix and negotiation contract
    - Record desktop, mobile, web, CLI, admin, service, protocol and schema minimum/maximum versions.
    - _Requirements: 18.1, 18.2_
    - _Capability IDs: CAP-038, CAP-039, CAP-040, CAP-041, CAP-042_
    - _Depends on: 42.6_
    - _Reads: .agents/specs/collaborative-workspace/fixtures/clients/**, .agents/specs/collaborative-workspace/migration-plan.md_
    - _Writes: docs/collaboration/compatibility.md_
    - _Validation: matrix checker rejects gaps, ambiguous write compatibility and unsupported schema combinations_

  - [ ] 43.2. Implement common client feature negotiation endpoints
    - Return supported features/minimum versions before incompatible writes across Nostr and HTTP clients.
    - _Requirements: 5.4, 18.2_
    - _Capability IDs: CAP-002, CAP-039, CAP-040, CAP-041_
    - _Depends on: 14.5, 43.1_
    - _Reads: docs/collaboration/compatibility.md, crates/collab/src/nostr/http.rs_
    - _Writes: crates/collab/src/compatibility.rs_
    - _Validation: endpoint tests cover supported, read-only, upgrade-required and unknown-feature clients_

  - [ ] 43.3. Repoint web invite and authentication flows
    - Use canonical community/invite/NIP-98 endpoints while preserving routes and clear upgrade errors.
    - _Requirements: 6.4, 16.4, 18.2_
    - _Capability IDs: CAP-008, CAP-039, CAP-042_
    - _Depends on: 18.5, 43.2_
    - _Reads: projects/buzz/web/src/**, crates/collab/src/compatibility.rs_
    - _Writes: clients/web/src/auth/*_
    - _Validation: browser tests cover redeem, expired/exhausted invite, signer denial and minimum-version response_

  - [ ] 43.4. Repoint web repository browsing and downloads
    - Preserve repository/detail/blob/download URLs against canonical hosted Git authorization.
    - _Requirements: 10.2, 16.4, 18.1_
    - _Capability IDs: CAP-019, CAP-039, CAP-042_
    - _Depends on: 25.5, 43.3_
    - _Reads: projects/buzz/web/src/**, crates/collab/src/git/smart_http_read.rs_
    - _Writes: clients/web/src/repositories/*_
    - _Validation: browser tests cover public/private browse, blob, range download, missing object and old URLs_

  - [ ] 43.5. Add web client compatibility E2E
    - Run frozen and migrated web versions through invite, authentication, browse, deep-link and upgrade flows.
    - _Requirements: 16.4, 18.1, 18.2, 20.1_
    - _Capability IDs: CAP-039, CAP-042, CAP-044_
    - _Depends on: 43.3, 43.4_
    - _Reads: clients/web/**, .agents/specs/collaborative-workspace/fixtures/clients/**_
    - _Writes: clients/web/tests/compatibility/*_
    - _Validation: web E2E matrix passes every supported version and fails incompatible writes before mutation_

  - [ ] 43.6. Repoint mobile auth, community and collaboration APIs
    - Migrate endpoint/storage bindings while preserving account, community and canonical entity identifiers.
    - _Requirements: 16.4, 18.1, 18.2_
    - _Capability IDs: CAP-040, CAP-042_
    - _Depends on: 23.7, 43.2_
    - _Reads: projects/buzz/mobile/lib/**, docs/collaboration/compatibility.md_
    - _Writes: clients/mobile/lib/data/collaboration/*_
    - _Validation: mobile tests cover upgrade, account/community switch and no duplicated local records_

  - [ ] 43.7. Migrate mobile background reconnect, push and pairing
    - Preserve lifecycle reconnect, wake-only fetch and NIP-AB flows for ADR-005-approved platforms.
    - _Requirements: 8.2, 9.5, 16.1, 18.2_
    - _Capability IDs: CAP-016, CAP-033, CAP-040_
    - _Depends on: 2.5, 22.9, 40.5, 43.6_
    - _Reads: projects/buzz/mobile/lib/**, services/push_gateway/src/**, services/pair_relay/src/main.rs_
    - _Writes: clients/mobile/lib/platform/collaboration_lifecycle/*_
    - _Validation: mobile lifecycle tests cover background/foreground, revoked lease, reconnect, pairing expiry and upgrade error_

  - [ ] 43.8. Add mobile collaboration compatibility E2E
    - Exercise channels, messages, DMs, read state, search, media, push and pairing across supported versions.
    - _Requirements: 9.1, 9.3, 9.4, 9.5, 16.4, 20.1_
    - _Capability IDs: CAP-012, CAP-013, CAP-015, CAP-016, CAP-031, CAP-040, CAP-044_
    - _Depends on: 38.7, 43.6, 43.7_
    - _Reads: clients/mobile/**, .agents/specs/collaborative-workspace/fixtures/clients/**_
    - _Writes: clients/mobile/test/compatibility/*_
    - _Validation: device/simulator matrix passes offline, background, reconnect, privacy and minimum-version scenarios_

  - [ ] 43.9. Repoint admin web resources to canonical APIs
    - Migrate community/member/invite/moderation/archive/deletion/metrics resources independently of mobile/web clients.
    - _Requirements: 15.1, 15.3, 16.4, 18.2_
    - _Capability IDs: CAP-029, CAP-030, CAP-041_
    - _Depends on: 36.7, 37.6, 43.2_
    - _Reads: projects/buzz/admin-web/**, crates/collab/src/admin/**_
    - _Writes: admin-web/src/data/collaboration/*_
    - _Validation: resource tests cover least privilege, stale writes, deletion status and minimum-version error_

  - [ ] 43.10. Migrate admin authentication and failure UX
    - Bind operator identity/scopes and render redacted partial-failure/retry states for canonical APIs.
    - _Requirements: 6.2, 15.4, 18.2_
    - _Capability IDs: CAP-008, CAP-041_
    - _Depends on: 43.9_
    - _Reads: projects/buzz/admin-web/**, admin-web/src/data/collaboration/**_
    - _Writes: admin-web/src/auth/collaboration/*_
    - _Validation: browser tests cover denied role, expired session, partial service, retry and no tenant metadata leak_

  - [ ] 43.11. Add admin web compatibility E2E
    - Exercise provisioning, invites, moderation, archive, deletion recovery and metrics across supported server versions.
    - _Requirements: 15.1, 15.3, 15.4, 18.1, 20.1_
    - _Capability IDs: CAP-029, CAP-030, CAP-041, CAP-044_
    - _Depends on: 43.9, 43.10_
    - _Reads: admin-web/**, .agents/specs/collaborative-workspace/fixtures/clients/**_
    - _Writes: admin-web/tests/compatibility/*_
    - _Validation: browser E2E passes role, tenant, recovery, upgrade and redaction scenarios_

- [ ] 44. Consolidate deployment, configuration and release pipelines

  - [ ] 44.1. Define canonical collaboration service configuration
    - Translate ADR-001 endpoints, database, Redis, object, Git, push, pair and mesh settings with validation/redaction.
    - _Requirements: 19.2, 19.4_
    - _Capability IDs: CAP-003, CAP-043_
    - _Depends on: 2.1, 41.5_
    - _Reads: projects/buzz/deploy/**, crates/collab/src/main.rs, .agents/specs/collaborative-workspace/decisions/adr-001-service-topology.md_
    - _Writes: crates/collab/src/collaboration_config.rs_
    - _Validation: config tests cover valid, missing secret, incompatible feature and redacted diagnostic_

  - [ ] 44.2. Consolidate local and Compose environments
    - Add canonical service dependencies, health ordering, volumes and rollback selection for development/self-hosting.
    - _Requirements: 19.3, 19.4_
    - _Capability IDs: CAP-043_
    - _Depends on: 22.10, 40.2, 44.1_
    - _Reads: projects/buzz/deploy/compose/**, projects/buzz/deploy/local/**, deploy/**_
    - _Writes: deploy/collaboration/compose/*_
    - _Validation: Compose configuration renders, starts healthy in isolation and supports documented prior-image rollback_

  - [ ] 44.3. Consolidate Helm and Kubernetes resources
    - Translate services, migrations, ingress, autoscaling, disruption, storage and network policy to Sim ownership.
    - _Requirements: 19.3, 19.4_
    - _Capability IDs: CAP-016, CAP-034, CAP-035, CAP-043_
    - _Depends on: 44.1, 44.2_
    - _Reads: projects/buzz/deploy/charts/**, deploy/**_
    - _Writes: deploy/collaboration/charts/*_
    - _Validation: Helm lint/render checks production-like values, missing secrets, migration hook and rollback version_

  - [ ] 44.4. Add canonical schema migration jobs and contracts
    - Package ordered forward/backward migration jobs with checksum, compatibility ceiling and halt behavior.
    - _Requirements: 17.2, 17.3, 19.4_
    - _Capability IDs: CAP-005, CAP-043, CAP-045_
    - _Depends on: 17.6, 37.7, 44.3_
    - _Reads: crates/collab/migrations/**, projects/buzz/scripts/cutover/**_
    - _Writes: deploy/collaboration/migrations/*_
    - _Validation: dry run applies, resumes, detects checksum drift and rolls back before compatibility boundary_

  - [ ] 44.5. Add health, metrics and redacted logging dashboards
    - Expose readiness, queue, projection drift, replica freshness, compatibility and migration state without content.
    - _Requirements: 19.3, 19.5_
    - _Capability IDs: CAP-006, CAP-028, CAP-043_
    - _Depends on: 16.3, 35.5, 44.1_
    - _Reads: crates/collab/src/{freshness,audit}/**, projects/buzz/deploy/**_
    - _Writes: deploy/collaboration/observability/*_
    - _Validation: smoke tests assert every stop/rollback signal is exported and private fixture content is absent_

  - [ ] 44.6. Consolidate build, signing and release workflows
    - Move service/client/package artifacts to Sim conventions with signed manifests and compatibility metadata.
    - _Requirements: 18.1, 19.4_
    - _Capability IDs: CAP-036, CAP-038, CAP-039, CAP-040, CAP-043_
    - _Depends on: 43.5, 43.8, 43.11, 44.3_
    - _Reads: projects/buzz/.github/workflows/**, .github/workflows/**, script/**_
    - _Writes: .github/workflows/collaboration-release.yml, script/collaboration-release-contract_
    - _Validation: release-contract dry run verifies packages, signatures, versions, notices and no production publish_

  - [ ] 44.7. Document operational rollout and rollback
    - Publish configuration, migration, canary, alert, rollback and incident procedures with authorization boundaries.
    - _Requirements: 17.3, 17.4, 19.3, 19.4_
    - _Capability IDs: CAP-043, CAP-045_
    - _Depends on: 44.4, 44.5, 44.6_
    - _Reads: .agents/specs/collaborative-workspace/migration-plan.md, deploy/collaboration/**_
    - _Writes: docs/collaboration/operations.md_
    - _Validation: tabletop review follows each rollback path and identifies last reversible checkpoint_

- [ ] 45. Run full compatibility, security and scale gates

  - [ ] 45.1. Run independent protocol differential gates
    - Execute signed-event, relay, custom NIP, Git, media, pairing and client protocol fixtures against both paths.
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 20.2_
    - _Capability IDs: CAP-001, CAP-002, CAP-019, CAP-031, CAP-033, CAP-044_
    - _Depends on: 38.7, 40.5, 42.6, 43.5, 43.8, 43.11_
    - _Reads: projects/buzz/crates/buzz-conformance/**, .agents/specs/collaborative-workspace/fixtures/**_
    - _Writes: test-results/collaborative-workspace/protocol-gate.md_
    - _Validation: independent oracle reports no unexplained semantic or failure-frame divergence_

  - [ ] 45.2. Run cross-boundary security gates
    - Execute tenant, auth, key, provider, webhook, media, push, huddle, mesh and logging negative suites.
    - _Requirements: 6.3, 19.1, 19.2, 20.1_
    - _Capability IDs: CAP-003, CAP-008, CAP-009, CAP-027, CAP-031, CAP-032, CAP-034, CAP-035, CAP-044_
    - _Depends on: 34.8, 38.7, 39.8, 41.5, 44.5_
    - _Reads: .agents/specs/collaborative-workspace/security/**, crates/**/tests/**_
    - _Writes: test-results/collaborative-workspace/security-gate.md_
    - _Validation: all threat-model controls have passing negative evidence and no critical/open bypass_

  - [ ] 45.3. Run migration and deletion fault gates
    - Execute every importer, schema, shadow, retention and deletion interruption/recovery fixture.
    - _Requirements: 15.2, 15.3, 17.2, 17.3, 20.1_
    - _Capability IDs: CAP-030, CAP-045, CAP-044_
    - _Depends on: 17.6, 37.7, 44.4_
    - _Reads: crates/collab/tests/{buzz_import_recovery,retention-deletion-faults}.rs, deploy/collaboration/migrations/**_
    - _Writes: test-results/collaborative-workspace/migration-gate.md_
    - _Validation: every injected failure resumes or rolls back to its declared canonical state_

  - [ ] 45.4. Run relay, messaging, search and push scale gates
    - Measure connections, subscriptions, fan-out, windows, search and wake queues against approved budgets.
    - _Requirements: 8.1, 8.4, 19.3, 20.1_
    - _Capability IDs: CAP-004, CAP-006, CAP-011, CAP-015, CAP-016, CAP-044_
    - _Depends on: 22.12, 44.3, 44.5_
    - _Reads: projects/buzz/perf/**, crates/collab/tests/**_
    - _Writes: test-results/collaborative-workspace/collaboration-scale.md_
    - _Validation: approved load runs remain within queue, latency, freshness and cleanup budgets_

  - [ ] 45.5. Run workflow, agent and mesh orchestration gates
    - Measure durable workflows, multi-agent delegation, remote providers and mesh partitions/resource fairness.
    - _Requirements: 11.4, 11.5, 13.1, 16.3, 20.1_
    - _Capability IDs: CAP-026, CAP-027, CAP-034, CAP-035, CAP-044_
    - _Depends on: 31.6, 33.6, 34.8, 41.5_
    - _Reads: projects/buzz/benchmarks/harbor-buzz-orchestra/**, crates/agent/tests/**, crates/remote/tests/**_
    - _Writes: test-results/collaborative-workspace/orchestration-scale.md_
    - _Validation: approved benchmark completes within resource/cancellation/fairness budgets without duplicate execution_

  - [ ] 45.6. Publish consolidated gate results and blockers
    - Aggregate independent results by CAP, criterion, version and approved budget without hiding known failures.
    - _Requirements: 1.4, 20.1, 20.3, 20.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 45.1, 45.2, 45.3, 45.4, 45.5_
    - _Reads: test-results/collaborative-workspace/*.md_
    - _Writes: .agents/specs/collaborative-workspace/validation-results.md_
    - _Validation: result checker rejects any missing CAP/criterion, unexplained divergence or unmet stop-ship gate_

- [ ] 46. Execute shadow reads and authoritative cutover tooling

  - [ ] 46.1. Implement aggregate cutover checkpoint records
    - Persist phase, tenant/aggregate, authority, cursors, hashes and last reversible boundary.
    - _Requirements: 2.2, 2.3, 17.4_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 17.1, 45.6_
    - _Reads: .agents/specs/collaborative-workspace/migration-plan.md, crates/collab/src/migration/buzz/checkpoint.rs_
    - _Writes: crates/collab/src/migration/cutover_checkpoint.rs_
    - _Validation: tests reject authority advancement without required hashes/gates and permit idempotent resume_

  - [ ] 46.2. Implement read-only shadow comparison
    - Compare legacy and canonical results by tenant/query/cursor/overlay without influencing served responses.
    - _Requirements: 2.2, 17.4_
    - _Capability IDs: CAP-005, CAP-011, CAP-045_
    - _Depends on: 46.1_
    - _Reads: crates/collab/src/migration/cutover_checkpoint.rs, crates/collab/src/messages/window_repository.rs_
    - _Writes: crates/collab/src/migration/shadow_read.rs_
    - _Validation: seeded content/order/auth divergence is detected while legacy remains serving authority_

  - [ ] 46.3. Implement bounded canonical outbox mirroring
    - Mirror one accepted command to temporary legacy projections using stable operation/source IDs and one-way precedence.
    - _Requirements: 2.3, 17.4_
    - _Capability IDs: CAP-005, CAP-006, CAP-045_
    - _Depends on: 15.4, 46.1, 46.2_
    - _Reads: crates/collab/src/db/collaboration/outbox.rs, .agents/specs/collaborative-workspace/migration-plan.md_
    - _Writes: crates/collab/src/migration/legacy_mirror.rs_
    - _Validation: tests cover retry, duplicate, delayed projection and prohibition on reverse last-writer-wins_

  - [ ] 46.4. Implement divergence stops and operator diagnostics
    - Halt the affected tenant/aggregate on authorization, signature, count/hash or legacy-only write divergence.
    - _Requirements: 8.3, 17.3, 17.4, 19.3_
    - _Capability IDs: CAP-028, CAP-043, CAP-045_
    - _Depends on: 46.2, 46.3_
    - _Reads: crates/collab/src/migration/{shadow_read,legacy-mirror}.rs, crates/collab/src/audit/events.rs_
    - _Writes: crates/collab/src/migration/divergence.rs_
    - _Validation: each stop condition halts only the scoped aggregate and emits redacted rollback guidance_

  - [ ] 46.5. Implement pre-boundary rollback commands
    - Quiesce writes, drain outbox, verify no divergence and restore prior routing/configuration before the recorded boundary.
    - _Requirements: 17.3, 17.4_
    - _Capability IDs: CAP-043, CAP-045_
    - _Depends on: 46.1, 46.4_
    - _Reads: crates/collab/src/migration/{cutover_checkpoint,divergence}.rs, docs/collaboration/operations.md_
    - _Writes: tools/collaboration_migrate/src/rollback.rs_
    - _Validation: isolated rollback restores prior authority and rejects unsafe rollback past boundary_

  - [ ] 46.6. Rehearse aggregate-by-aggregate cutover
    - Run shadow, bounded mirror, authority switch, divergence halt and rollback for every persisted aggregate without production mutation.
    - _Requirements: 2.3, 17.2, 17.3, 17.4, 20.1_
    - _Capability IDs: CAP-005, CAP-045, CAP-044_
    - _Depends on: 46.3, 46.4, 46.5_
    - _Reads: tools/collaboration_migrate/**, .agents/specs/collaborative-workspace/migration-plan.md_
    - _Writes: test-results/collaborative-workspace/cutover-rehearsal.md_
    - _Validation: rehearsal records exact counts/hashes, resume, halt and rollback for each aggregate_

- [ ] 47. Retire superseded implementations

  - [ ] 47.1. Enforce legacy write freeze and usage gates
    - Reject direct legacy writes, measure remaining traffic and require rollback-window thresholds before removal.
    - _Requirements: 2.4, 18.3_
    - _Capability IDs: CAP-004, CAP-005, CAP-045_
    - _Depends on: 46.6_
    - _Reads: crates/collab/src/migration/**, docs/collaboration/compatibility.md_
    - _Writes: crates/collab/src/migration/legacy_freeze.rs_
    - _Validation: tests reject legacy-only writes and removal gate fails above approved traffic/rollback thresholds_

  - [ ] 47.2. Prepare Buzz React/Tauri desktop retirement change
    - Remove build/package dependencies only after GPUI parity and desktop-state import evidence; preserve source until deletion approval.
    - _Requirements: 2.4, 18.3, 18.4_
    - _Capability IDs: CAP-036, CAP-037, CAP-045_
    - _Depends on: 10.7, 30.5, 47.1_
    - _Reads: projects/buzz/desktop/**, crates/sim/src/migration/buzz/**, .agents/specs/collaborative-workspace/validation-results.md_
    - _Writes: .agents/specs/collaborative-workspace/retirement/buzz-desktop.md_
    - _Validation: retirement manifest proves no server capability or unimported state depends on a Tauri command_

  - [ ] 47.3. Prepare duplicate ACP, agent and MCP retirement change
    - Remove duplicate execution ownership after tool/session/provider conformance while retaining required external shims.
    - _Requirements: 2.4, 18.3, 18.4_
    - _Capability IDs: CAP-021, CAP-022, CAP-034_
    - _Depends on: 28.6, 33.6, 47.1_
    - _Reads: projects/buzz/crates/{buzz-acp,buzz-agent,buzz-dev-mcp}/**, tools/buzz_compat/**_
    - _Writes: .agents/specs/collaborative-workspace/retirement/buzz-agent-runtime.md_
    - _Validation: process/state audit identifies one executor owner and lists every retained compatibility binary_

  - [ ] 47.4. Prepare relay, database and pub/sub retirement change
    - Remove superseded orchestration/direct writes only after final service, projection and protocol gates.
    - _Requirements: 2.4, 18.3, 18.4_
    - _Capability IDs: CAP-002, CAP-004, CAP-005, CAP-006_
    - _Depends on: 45.1, 47.1_
    - _Reads: projects/buzz/crates/{buzz-relay,buzz-db,buzz-pubsub}/**, crates/collab/**_
    - _Writes: .agents/specs/collaborative-workspace/retirement/buzz-service.md_
    - _Validation: network/schema/process audit finds no unintended path or duplicate write authority_

  - [ ] 47.5. Preserve compatibility artifacts, licenses and source history
    - Inventory retained protocols, formal models, fixtures, shims, notices and approved reference-source location.
    - _Requirements: 18.3, 18.4_
    - _Capability IDs: CAP-038, CAP-042, CAP-044, CAP-045_
    - _Depends on: 47.2, 47.3, 47.4_
    - _Reads: projects/buzz/LICENSE*, projects/buzz/docs/spec/**, projects/buzz/crates/buzz-conformance/**_
    - _Writes: LICENSES/buzz.md, .agents/specs/collaborative-workspace/retirement/preserved-artifacts.md_
    - _Validation: license/source-history review accounts for every imported or retained Buzz artifact_

  - [ ] 47.6. Execute no-duplicate owner and dependency audit
    - Scan manifests, processes, ports, schemas and state writers after proposed retirements; do not delete source without authorization.
    - _Requirements: 2.4, 18.4, 20.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 47.5_
    - _Reads: Cargo.toml, crates/**/Cargo.toml, .agents/specs/collaborative-workspace/retirement/**_
    - _Writes: .agents/specs/collaborative-workspace/retirement/no-duplicate-audit.md_
    - _Validation: audit reports one canonical owner per aggregate and zero unintended retired-source dependencies_

- [ ] 48. Publish complete parity and ownership evidence

  - [ ] 48.1. Regenerate final source and ownership catalogs
    - Re-run every inventory and join CAP rows to canonical owner, disposition, implementation and retained boundary.
    - _Requirements: 1.4, 20.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 47.6_
    - _Reads: .agents/specs/collaborative-workspace/catalogs/**, .agents/specs/collaborative-workspace/retirement/**_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/final-coverage.csv_
    - _Validation: inventory checker reports all CAP IDs complete with no unexplained or deferred component_

  - [ ] 48.2. Assemble the final parity evidence report
    - Link every acceptance criterion and CAP ID to passing reuse, implementation, compatibility and migration evidence.
    - _Requirements: 1.4, 20.1, 20.3, 20.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 45.6, 46.6, 48.1_
    - _Reads: .agents/specs/collaborative-workspace/**, test-results/collaborative-workspace/**_
    - _Writes: .agents/specs/collaborative-workspace/parity-report.md_
    - _Validation: parity checker rejects missing evidence, open known gap, unresolved ADR or prohibited duplicate owner_

  - [ ] 48.3. Publish canonical collaboration architecture and operations docs
    - Document final owners, adapters, data flows, compatibility, migration history and operator runbooks.
    - _Requirements: 18.1, 18.3, 19.3, 20.4_
    - _Capability IDs: CAP-041, CAP-043, CAP-045_
    - _Depends on: 44.7, 47.5, 48.2_
    - _Reads: .agents/specs/collaborative-workspace/{design,migration-plan,parity-report}.md, docs/collaboration/operations.md_
    - _Writes: docs/collaboration/architecture.md_
    - _Validation: documentation review resolves every canonical owner, supported boundary, rollback ceiling and retained artifact_

  - [ ] 48.4. Record final operational and product sign-off
    - Capture approved parity, security, compatibility, migration, rollback-window and source-retirement decisions.
    - _Requirements: 18.3, 18.4, 20.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 48.2, 48.3_
    - _Reads: .agents/specs/collaborative-workspace/parity-report.md, docs/collaboration/architecture.md_
    - _Writes: .agents/specs/collaborative-workspace/final-signoff.md_
    - _Validation: sign-off checklist has named approvals, dates and no open stop-ship, ADR, deferred or duplicate-owner item_

## Decomposition audit notes

- No approved requirement, architecture decision, capability ownership, migration phase or milestone scope was changed.
- Milestones are headings, the 48 approved capability epics are parent checkboxes, and all executable work is represented by nested `epic.leaf` implementation units with metadata only on leaves.
- All 84 acceptance criteria appear in the approved design traceability table and in at least one leaf task. All CAP-001 through CAP-045 identifiers appear in at least one implementation, compatibility, validation or verified-reuse leaf.
- The final plan contains 48 populated epic parents and 330 implementation leaves. Every leaf includes requirement, capability, dependency, read, write and validation metadata.
- The decomposition audit reviewed compound titles, multi-path writes and cross-boundary outcomes. Each leaf retains one primary implementation or evidence boundary, concrete scope, and focused validation; independently reviewable domain, persistence, transport, UI, client, migration, deployment and test outcomes remain separate leaves.
- The validator reports only four cross-root write warnings: the inventory checker and repository entry point, the shared domain-boundary enforcement check, the release workflow and its local runner, and the preservation manifest with its license notice. Each pair is one coherent enforcement or preservation artifact; splitting it would leave an independently unusable wrapper or manifest. No exact `_Writes:` path is repeated.
- All leaf dependencies were recomputed after renumbering. Validator checks confirm that every dependency names an existing implementation leaf and the explicit shared-write chains sequence intentional overlap.
- No perpetual deferred bucket exists. Approval-gated work names its ADR and remains assigned to a concrete leaf after approval.
- No leaf is estimated above three agent-days. The compatibility/load/cutover gates are execution-and-evidence leaves over fixtures built earlier, not requests to construct those systems inside the gate task.
- Production cutover, source deletion and irreversible operations are represented as tooling, rehearsal, evidence and separately authorized handoff work; this plan does not authorize those mutations.
