# Implementation Plan: Collaborative Workspace and Buzz Consolidation

## Approach

The approved capability epics are parent checkboxes grouped under milestone headings. Executable work appears only as indented implementation leaves. Epic IDs are stable global integers, and leaf IDs use the `epic.leaf` form; for example, `8.1` is the first leaf of epic 8. Every leaf delivers one behavior, adapter, migration, UI component, fixture or operational artifact and is sized for 0.5–3 agent-days including focused tests.

Milestone 1 remains an end-to-end GPUI slice over existing Zed state before server authority changes. Later work moves aggregate authority behind approved adapters and migration gates. Production deployment, irreversible deletion, source removal and traffic cutover remain separately authorized operations.

## Plan summary

| Milestone | Leaf tasks | Estimated agent-days | Principal outcome |
| --- | ---: | ---: | --- |
| 0 — evidence and decisions | 23 | 35 | Reproducible inventory, ADRs, baselines and threat model |
| 1 — native vertical slice | 41 | 63 | Reversible Collaborative Workspace over existing project/ACP/Git state |
| 2 — protocol and service foundations | 53 | 95 | Canonical domain, identity, tenant, protocol, persistence and import foundations |
| 3 — communication parity | 47 | 82 | Channels, messages, DMs, awareness, search, notifications and social surfaces |
| 4 — project and Git collaboration | 27 | 49 | NIP-MP/NIP-34, branch channels, review and CI linkage |
| 5 — agent convergence | 37 | 69 | ACP/MCP, personas, memory, jobs, activity and remote execution |
| 6 — platform parity | 57 | 106 | Workflows, audit, administration, deletion, media, huddles, pairing and mesh |
| 7 — clients, operations and retirement | 52 | 95 | Client compatibility, release readiness, cutover, retirement and parity proof |
| **Total** | **337** | **594** | Complete approved migration scope |

The approved compile-time isolation change adds Epic 49 with 15 leaves and 24 estimated agent-days before further unrelated Collaborative Workspace work. It does not alter milestone scope or canonical ownership; it makes Standard Zed and Multiplayer Zed explicit supported build profiles. The resulting plan contains 352 leaves and 618 estimated agent-days.

The dependency graph has an estimated **316 agent-day critical path** from inventory and ADR approval through domain/auth/storage, messaging, agent/workflow convergence, compatibility gates, cutover and retirement. With four stable workstreams and prompt reviews, implementation work is approximately 10–16 elapsed months; required observation windows, external client certification and production approvals can extend calendar delivery. A single sequential agent is approximately 594 working days before review/rework allowance.

The dependency-safe decomposition of Tasks 7.5, 9.1 and 10.2 adds seven explicit leaves and recalibrates Milestone 1 by 12 agent-days. This is not added product scope: it exposes previously hidden workspace contracts, downstream adapters, workspace mounting and upper-layer registration work that cannot share one review or one crate owner without dependency cycles.

No leaf is intentionally larger than three agent-days. Cross-system scenarios are split into fixture construction, implementation, and execution/reporting leaves. If a leaf exceeds that bound during implementation, it must be split before code review without changing its epic scope.

### Implementation-discovered validation bootstrap

Tasks 1.1 through 1.4 name `check-inventory.py` in their validation metadata, but Task 1.5 creates that checker and depends on all four catalogs. During this bootstrap cycle, each catalog is validated by an equivalent source-to-catalog comparison and the specification validator; Task 1.5 must rerun the named canonical validations before the inventory epic closes. This records the dependency contradiction without changing approved scope, ownership, or task ordering.

## Dependency waves and parallel-safe workstreams

- **Wave 0 / Milestone 0:** inventory generation, independent fixtures and ADR evidence may proceed in parallel; the security review follows the ownership decisions and baselines.
- **Feature-isolation gate:** 49.1 → 49.2 → 49.3 → parallel 49.4/49.5/49.6 → parallel 49.7/49.8/49.9 → 49.10 → parallel 49.11/49.12 → 49.13 → 49.14 → 49.15. No unrelated Collaborative Workspace leaf may begin until 49.14 proves both build configurations in CI.
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

## ADR-dependent leaves

ADR-001 through ADR-006 were accepted on 2026-08-14. The leaves below remain dependency-gated by their normative decision records, not blocked on an open product choice.

| Decision | Leaves governed by the accepted decision |
| --- | --- |
| ADR-001 service/database topology | 2.1, 14.1, 15.1, 44.1 |
| ADR-002 account/Nostr binding | 2.2, 12.1, 12.2, 12.3 |
| ADR-003 hosted Git authority | 2.3, 25.1, 25.2, 25.3 |
| ADR-004 native huddle transport | 2.4, 39.1, 39.2, 39.3 |
| ADR-005 push platforms | 2.5, 22.9, 43.7 |
| ADR-006 shared-compute policy | 2.6, 41.1, 41.2, 41.3 |

## Shared-write sequencing

- Workspace presentation files serialize through 5.1 → 5.2 → 5.3 → 6.1 → 6.2 → 7.1 → 7.5 → 9.1 → 9.5 → 10.1 → 10.2 → 10.9 → 10.3 → 10.4 → 10.5. Sidebar activation follows 7.2, 7.3 and 7.4 → 7.6 without reversing the `sidebar` → `workspace` dependency.
- Activity projection files serialize through 8.1 → 8.2 → 8.3 → 8.4 → 32.1 → 32.2 → 32.3 → 32.4.
- Collaboration schemas serialize through 15.1 → 15.2 → 15.3 → 18.1 → 19.1 and then aggregate-specific migrations.
- Channel store integration serializes through 18.4 → 21.1 → 26.1.
- Git review integration serializes through workspace host 9.1 → parallel `agent_ui`/`git_ui` adapters 9.6 and 9.7 → upper mount 9.8 → 9.2 → 27.1 → 27.2 → 27.3.
- Participant/status integration serializes through workspace view data 10.2 → parallel `agent_ui` adapter 10.8 and workspace top/status mount 10.9 → upper registration 10.10. Shared `agent_ui` and `zed` crate-root writes are serialized after 9.6/9.2 and 9.8 respectively.
- Agent stores serialize through 29.1 → 29.2 → 30.1 → 31.1; remote execution follows 33.1 → 33.2.
- Administrative schema and APIs serialize through 35.1 → 36.1 → 36.2 → 37.1.
- Client compatibility documents serialize through 43.1 → 43.2 → 43.3 → 43.4 → 43.5; final architecture evidence follows 48.1 → 48.4.

## Tasks

## Milestone 0 — establish evidence and decisions

- [ ] 1. Generate and enforce the Buzz coverage ledger

  - [x] 1.1. Generate the Buzz Rust-package catalog
    - Enumerate workspace members, manifests, binaries and feature flags with stable CAP mappings.
    - _Requirements: 1.1, 1.2_
    - _Capability IDs: CAP-001, CAP-043, CAP-044_
    - _Depends on: none_
    - _Reads: projects/buzz/Cargo.toml, projects/buzz/crates/*/Cargo.toml_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/buzz-packages.csv_
    - _Validation: `python3 .agents/specs/collaborative-workspace/scripts/check-inventory.py --catalog packages` reports every workspace member_
    - _Evidence: 2026-08-13 — an independent Python `tomllib`/CSV comparison verified all 31 workspace members, manifest paths, package names, library/binary targets, declared or implicit features, unique stable package IDs, and CAP mappings; `validate_spec.py` passed with 84 acceptance criteria and 378 task records. Task 1.5 reran the canonical check and corrected explicit-versus-workspace version provenance plus the omitted `mention` and `wamp_bench` binaries. Commit: `005a5a680205df31ab53721485e7ee21beaf45b5`._

  - [x] 1.2. Generate the event-kind and NIP catalog
    - Extract registered kinds and standard/custom protocol documents into stable protocol rows.
    - _Requirements: 1.1, 1.2, 5.1_
    - _Capability IDs: CAP-001, CAP-002, CAP-044_
    - _Depends on: none_
    - _Reads: projects/buzz/crates/buzz-core/src/kind.rs, projects/buzz/docs/nips/*, projects/buzz/NOSTR.md_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/protocol.csv, .agents/specs/collaborative-workspace/source-inventory.md_
    - _Validation: inventory checker reports all registered constants and NIP files exactly once_
    - _Evidence: 2026-08-13 — an independent Python source-to-CSV comparison verified unique rows for all 137 scalar `u32` constants (133 event kinds plus four range boundaries), 28 referenced standard NIPs, all 16 custom NIP documents, both checked-in NIP-MP fixture files, and `NOSTR.md`; every source path exists and every row has valid CAP coverage. The source inventory and Task 11.5 were corrected from the stale 116-constant planning snapshot. `validate_spec.py` passed with 84 acceptance criteria and 378 task records. Task 1.5 reran the canonical protocol check. Commit: `25febd0ca7a2d5bbf6a33a0a723d3e15cc0e4ab4`._

  - [x] 1.3. Generate the data and migration catalog
    - Enumerate SQL migrations, schemas, object stores, Redis state and desktop persistence sources.
    - _Requirements: 1.1, 17.1_
    - _Capability IDs: CAP-005, CAP-030, CAP-045_
    - _Depends on: none_
    - _Reads: projects/buzz/migrations/*, projects/buzz/schema/**, projects/buzz/crates/buzz-db/src/lib.rs, projects/buzz/crates/buzz-media/src/{storage,upload,upload_record}.rs, projects/buzz/crates/buzz-pubsub/src/**, projects/buzz/crates/buzz-relay/src/api/git/{store,pack_cache}.rs, projects/buzz/desktop/src-tauri/src/{migration,event_sync,app_state,app_state_keyring,secret_store,key_backup}.rs, projects/buzz/desktop/src-tauri/src/{archive,managed_agents}/**, projects/buzz/desktop/src-tauri/src/commands/legacy_storage.rs, projects/buzz/desktop/src-tauri/src/mesh_llm/identity.rs, projects/buzz/desktop/src/{app,features,shared}/**_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/data-sources.csv_
    - _Validation: catalog check accounts for all 30 SQL migrations and every discovered durable store_
    - _Evidence: 2026-08-13 — an independent Python CSV/source comparison verified 62 unique catalog rows, exact one-to-one coverage of all 30 SQL migrations, the current schema snapshot, and 31 server, Redis, desktop, secret, cache, operational, and migration-bridge boundaries; every listed source path exists and every CAP reference is valid. The audit explicitly records the documented-but-absent Redis typing implementation as `REDIS-TYPING-GAP-001`. `validate_spec.py` passed with 84 acceptance criteria and 378 task records. Task 1.5 reran the canonical data check. Commit: `ff954e6fc7397de843ce54e588779c8d1ab8419a`._

  - [x] 1.4. Generate client, desktop and deployment catalogs
    - Enumerate Tauri modules, desktop features, client routes, charts, workflows, scripts, examples and benchmarks.
    - _Requirements: 1.1, 1.2_
    - _Capability IDs: CAP-036, CAP-038, CAP-039, CAP-040, CAP-041, CAP-043, CAP-044_
    - _Depends on: 1.2_
    - _Reads: projects/buzz/desktop/src/**, projects/buzz/desktop/src-tauri/src/**, projects/buzz/desktop/tests/**, projects/buzz/mobile/{lib,test}/**, projects/buzz/web/{src,tests}/**, projects/buzz/admin-web/**, projects/buzz/deploy/**, projects/buzz/.github/workflows/**, projects/buzz/scripts/**, projects/buzz/examples/**, projects/buzz/benchmarks/**, projects/buzz/perf/**_
    - _Writes: .agents/specs/collaborative-workspace/catalogs/surfaces.csv, .agents/specs/collaborative-workspace/source-inventory.md_
    - _Validation: inventory checker reports no unmapped feature, route, deployment component or test surface_
    - _Evidence: 2026-08-13 — an independent Python source-to-CSV audit verified 193 unique rows with exact coverage of all 39 declared Tauri modules, 29 desktop feature directories, 13 desktop routes, six web routes, ten mobile feature directories plus app/deep-link surfaces, four admin routes, four deployment components, all 18 GitHub workflows, all 61 script files, both examples, both benchmark suites, and five client test surfaces. Every source path exists and every CAP reference is valid. The source inventory was corrected from 17 to 18 workflows and from the stale Task 1.3 Tauri-catalog reference to Task 1.4. `validate_spec.py` passed with 84 acceptance criteria and 378 task records. Task 1.5 reran the canonical surfaces check. Commit: `93818e4091469e9e2f617779884861b884769ec7`._

  - [x] 1.5. Enforce inventory drift in repository checks
    - Add one checker that joins all catalogs to CAP, requirement, owner and leaf-task references and fails on omissions.
    - _Requirements: 1.2, 1.3, 1.4_
    - _Capability IDs: CAP-001, CAP-045_
    - _Depends on: 1.1, 1.2, 1.3, 1.4_
    - _Reads: .agents/specs/collaborative-workspace/catalogs/**, .agents/specs/collaborative-workspace/{source-inventory,reuse-audit,requirements,tasks}.md_
    - _Writes: .agents/specs/collaborative-workspace/scripts/check-inventory.py, script/check-collaborative-workspace-inventory, .agents/specs/collaborative-workspace/catalogs/buzz-packages.csv_
    - _Validation: a temporary unmapped fixture makes the checker fail with its exact source path and missing references_
    - _Evidence: 2026-08-13 — added `.agents/specs/collaborative-workspace/scripts/check-inventory.py` and `script/check-collaborative-workspace-inventory` to validate exact catalog schemas and stable IDs; join every row through CAP-001–CAP-045 to canonical owner/disposition, acceptance criteria, and decimal leaves; verify every catalog source path; and detect package, protocol, migration, Tauri, client, deployment, workflow, script, example, benchmark, and test-surface drift. `python3 .agents/specs/collaborative-workspace/scripts/check-inventory.py --catalog all`, each focused `--catalog` mode, and the repository wrapper passed with 31 package, 184 protocol, 62 data, 193 surface, 45 capability, 84 criterion, and 330 leaf records. `cargo metadata --manifest-path projects/buzz/Cargo.toml --no-deps --format-version 1` independently matched all 31 package versions and library/binary targets. A temporary `unmapped-surface.rs` fixture failed with exit 1 while naming its exact absolute path and all four missing reference classes. The checker exposed and corrected four stale package-version provenance values plus the omitted `mention` and `wamp_bench` binaries in `catalogs/buzz-packages.csv`. `sh -n` and `git diff --check` passed; ShellCheck was unavailable. `validate_spec.py` passed with 84 acceptance criteria and 378 task records. Commit: `3ee584249e15ef48e9647fbec2308b95b6f2c53f`._

- [ ] 2. Record canonical ownership and architecture decisions

  - [x] 2.1. Decide ADR-001 service and database topology
    - Record final process, schema and dependency-version ownership plus the bounded sidecar exit conditions.
    - _Requirements: 2.1, 2.2, 2.3_
    - _Capability IDs: CAP-003, CAP-005, CAP-043_
    - _Depends on: 1.1, 1.3_
    - _Reads: .agents/specs/collaborative-workspace/reuse-audit.md, crates/collab/**, projects/buzz/ARCHITECTURE.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-001-service-topology.md_
    - _Validation: architecture review records one migration authority and explicit sidecar removal gates_
    - _Evidence: 2026-08-14 — accepted ADR-001 records `collab` as the final collaboration service and operational owner, one Zed-owned Postgres migration authority, aggregate-specific canonical data owners, a transactional command/outbox projection path, and Redis as derived expiring state. It bounds the Buzz-derived Nostr ingress sidecar to migration Phases 2–7, denies it migration and projection-write authority, and defines measurable entry, observation, rollback and removal gates including dependency alignment, in-process route parity, supported-client compatibility and deployment-artifact cleanup. Architecture structure checks, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 2.2. Decide ADR-002 account and Nostr identity binding
    - Record binding cardinality, verification, recovery, rotation and organization policy.
    - _Requirements: 2.1, 7.1, 7.4_
    - _Capability IDs: CAP-007, CAP-008, CAP-009_
    - _Depends on: 1.2_
    - _Reads: .agents/specs/collaborative-workspace/reuse-audit.md, crates/client/src/user.rs, projects/buzz/docs/nips/NIP-OA.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-002-identity-binding.md_
    - _Validation: identity review covers create, link, rotate, revoke, archive and recovery without ambiguous authority_
    - _Evidence: 2026-08-14 — accepted ADR-002 separates Zed service accounts, Nostr signing identities, community profiles and independently authored agent identities. It permits multiple community-local npubs while enforcing one active signer per community/account/profile tuple, possession-proof linking, atomic rotation, terminal revocation, history-preserving archive, bounded verified recovery, canonical protected-key custody and organization policies that may narrow but cannot forge or replace authorship. A lifecycle table and identity-review gate cover create, link, activate, rotate, revoke, archive, restore and recover success/failure paths, cross-community isolation, replay, storage failure and NIP-OA provenance. Identity structure checks, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 2.3. Decide ADR-003 hosted Git authority
    - Choose authority boundaries between NIP-34 hosting, external providers and local Zed Git.
    - _Requirements: 2.1, 10.1, 10.2_
    - _Capability IDs: CAP-018, CAP-019, CAP-020_
    - _Depends on: 1.2_
    - _Reads: .agents/specs/collaborative-workspace/reuse-audit.md, crates/git_hosting_providers/**, projects/buzz/docs/git-on-object-storage.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-003-git-authority.md_
    - _Validation: decision table assigns one authority for working state, hosted refs, patches and review records_
    - _Evidence: 2026-08-14 — accepted ADR-003 preserves native Zed project/worktree/Git authority locally and assigns each repository exactly one versioned hosted authority: Zed NIP-34 hosting, one external provider, or none. Its decision table covers working state, hosted refs, object durability, patches, pull requests, issues, reviews, approvals, CI/status and merges; it makes Zed-hosted manifest CAS the ref commit point, treats provider responses as externally authoritative, prohibits project grouping from granting repository access, and defines write-freeze/reconciliation/rollback for authority transfers. Git-authority structure checks, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 2.4. Decide ADR-004 huddle transport
    - Select native transport and define the Buzz audio compatibility support window.
    - _Requirements: 2.1, 14.3, 14.4_
    - _Capability IDs: CAP-032_
    - _Depends on: 1.4_
    - _Reads: crates/livekit_api/**, crates/livekit_client/**, projects/buzz/crates/buzz-relay/src/audio/**_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-004-huddle-transport.md_
    - _Validation: review records lifecycle parity, platform support and adapter retirement criteria_
    - _Evidence: 2026-08-14 — accepted ADR-004 makes LiveKit the sole native realtime media/room authority beneath a transport-neutral huddle lifecycle and retains Buzz protocol v1/v2 as a bounded Opus/WebSocket gateway into the same room. It maps lifecycle and participant semantics, preserves legacy admission/frame/backpressure behavior, defines mixed-client media bridging, tenant/generation isolation, platform/device/permission gates, TTS/transcript ownership, visible failure and cleanup, and explicit Phase 8 support-floor/removal criteria with no legacy-room fallback. Huddle-decision structure checks, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 2.5. Decide ADR-005 push platform scope
    - Record required push platforms, attestation requirements and the first mobile-cutover support floor.
    - _Requirements: 2.1, 9.5, 19.2_
    - _Capability IDs: CAP-016_
    - _Depends on: 1.4_
    - _Reads: projects/buzz/crates/buzz-push-gateway/**, .agents/specs/collaborative-workspace/reuse-audit.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-005-push-scope.md_
    - _Validation: approval records supported targets, attestations, fallback and compatibility floor_
    - _Evidence: 2026-08-14 — accepted ADR-005 makes APNs production plus sandbox validation and Apple App Attest the first mobile-cutover floor, preserves NIP-PL wake-only payload noninterference and encrypted endpoint custody, and defines visible foreground/manual-sync fallback with no attestation bypass. FCM and UnifiedPush are excluded from the first floor because Buzz has no conforming profiles; a payload-free common provider contract and eight-part approval gate make later providers explicit work rather than a silent downgrade. Push-scope structure checks, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 2.6. Decide ADR-006 shared-compute policy
    - Record mesh trust, eligibility, resources, fairness, fallback and deployment policy.
    - _Requirements: 2.1, 16.3, 19.2_
    - _Capability IDs: CAP-035_
    - _Depends on: 1.4_
    - _Reads: projects/buzz/crates/buzz-relay-mesh/**, .agents/specs/collaborative-workspace/reuse-audit.md_
    - _Writes: .agents/specs/collaborative-workspace/decisions/adr-006-shared-compute.md_
    - _Validation: approval records fail-closed eligibility, resource limits, fairness and no-silent-fallback rules_
    - _Evidence: 2026-08-14 — accepted ADR-006 makes shared compute deployment/community/user/device opt-in and initially restricts providers to same-deployment or explicitly sharing active community members. It defines nine fail-closed eligibility gates, signed/fenced mesh and executor leases, locally enforced resource/sandbox bounds, community-isolated weighted fairness, prompt/privacy consent, disabled-by-default deployment and rollback policy, and a strict no-silent-fallback rule including unknown-outcome and cross-owner retry handling. Third-party compute remains ineligible behind an explicit eight-part approval gate. Shared-compute structure checks, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 3. Capture independent compatibility and behavior baselines

  - [x] 3.1. Freeze signed-event and relay protocol fixtures
    - Capture valid, malformed, replaceable, privacy-gated and mixed-version event traces without production reducers.
    - _Requirements: 5.1, 5.2, 5.3, 20.2_
    - _Capability IDs: CAP-001, CAP-002, CAP-004, CAP-044_
    - _Depends on: 1.2_
    - _Reads: projects/buzz/crates/buzz-test-client/**, projects/buzz/crates/buzz-conformance/**_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/protocol/*_
    - _Validation: independent trace checker accepts valid fixtures and rejects each malformed fixture_
    - _Evidence: 2026-08-13 — froze deterministic NIP-01 event vectors, NIP-01 WebSocket request/response traces, byte-exact copies of the four Buzz conformance traces, replaceable-head orderings, author-only/`#p`/shared privacy decisions, and simultaneous kind 9/kind 40002 compatibility in `.agents/specs/collaborative-workspace/fixtures/protocol/`. The standard-library-only `check_fixtures.py` independently reimplements canonical event hashing, BIP-340 verification, lowest-ID timestamp ties, privacy projection, wire-frame sequencing and the tenant-transition checks without importing a Buzz reducer. Its full run passed 7 event, 2 replaceable, 7 privacy, 1 mixed-version, 4 wire and 4 relay cases; focused runs accepted every valid case and classified all five malformed event vectors and all three malformed relay traces with their expected failure. A direct in-memory mutation of a valid event was rejected as `invalid_id`, and a flipped positive wire verdict was rejected as `wrong_ok_verdict`. SHA-256 comparison proved the four relay traces are byte-identical to `buzz-conformance/tests/fixtures`. `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-conformance --test replay_fixtures` passed all 6 tests. A temporary standalone verifier using `nostr 0.44.7` parsed and cryptographically verified all 10 valid frozen events. Python AST parsing, `git diff --check`, and `validate_spec.py` passed. Commit: `49189489f09cab63bc75c292be2a3027ba76b72d`._

  - [x] 3.2. Freeze CLI and companion-client contract fixtures
    - Capture command output, exit codes, routes, deep links, negotiation and background lifecycle contracts.
    - _Requirements: 16.4, 18.1, 20.1_
    - _Capability IDs: CAP-038, CAP-039, CAP-040, CAP-041, CAP-042_
    - _Depends on: 1.4_
    - _Reads: projects/buzz/crates/buzz-cli/**, projects/buzz/mobile/test/**, projects/buzz/web/**, projects/buzz/admin-web/**_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/clients/*_
    - _Validation: fixture manifest identifies client version, input, expected output and authority for every captured contract_
    - _Evidence: 2026-08-13 — added a versioned 28-row contract manifest and independent source-drift checker under `.agents/specs/collaborative-workspace/fixtures/clients/`. Every row records a stable ID, category, client/version, concrete input, observable expected output, and exact source/test authority. Coverage freezes the 22-group CLI inventory, help secrecy, local-pack boundary, JSON error envelope, exit taxonomy and entity links; all five web routes plus invite consent, app handoff and NIP-07/NIP-98 browser claim; all four admin routes plus forbidden, unavailable-content and local-status behavior; and mobile message/invite parsing, hostile-link rejection, cold-start dispatch, background grace/reconnect, pairing negotiation, CLOSED classification and retry hints. It also records the current absence of a common startup feature-negotiation endpoint as an explicit compatibility gap owned by Task 43.2. `check_contracts.py` matched all four client versions and all frozen source tokens/routes; an in-memory contract missing `expected_output` failed with its stable ID and missing field. The actual `buzz --help` exited 0 without secret values and the actual no-key `buzz channels list` exited 3 with the exact frozen JSON error, correcting an initially omitted `auth error:` prefix. `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-cli command_` passed all 3 command inventory/name/count tests, and the focused help-secret test passed. Python AST parsing, inventory validation, `git diff --check`, and `validate_spec.py` passed. Flutter and companion-client node dependencies were unavailable locally, so their behavior is validated by exact source/test authority rather than rerunning those suites in this checkpoint. Commit: `d61a718f139f5da1bc6489cb2fc4bbf288f5fe93`._

  - [x] 3.3. Freeze migration and archive fixtures
    - Build sanitized fixtures for every SQL and desktop stored-data version with counts and integrity hashes.
    - _Requirements: 17.1, 17.2, 20.1_
    - _Capability IDs: CAP-005, CAP-024, CAP-030, CAP-045_
    - _Depends on: 1.3_
    - _Reads: projects/buzz/migrations/**, projects/buzz/desktop/src-tauri/src/{migration,archive,managed?agents}/**_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/migrations/*_
    - _Validation: fixture index covers every stored version and verifies hashes without private key material_
    - _Evidence: 2026-08-14 — added a catalog-driven migration fixture corpus under `.agents/specs/collaborative-workspace/fixtures/migrations/`. `manifest.json` freezes SHA-256, byte count, line count, stable sequence, catalog name, and exact source path for all 30 ordered PostgreSQL migrations. `desktop-stores.json` provides 32 sanitized semantic versions across all 20 inventoried desktop persistence boundaries, including Sprout/release/dev app trees, inline-fallback/key-reference managed agents, persona/team folds, global/scoped retention, archive schema v0 plus all three one-shot migrations, keyring/file/backup states, WebKit stores, agent nest/receipts/logs, event sync, and mesh identity. Secret-bearing stores contain identifiers and expected migration behavior only; all key material and encrypted payload bytes are omitted. The independent standard-library `check_fixtures.py` joined the corpus to `catalogs/data-sources.csv`, verified every catalog source path, exact SQL sequence and hashes/counts, exact per-store version sets, per-fixture record counts and canonical hashes, the desktop-document hash, and explicit `contains_private_key_material: false` declarations. Its built-in negative checks rejected both a mutated fixture hash and an injected nsec-like value. The checker passed with `sql_migrations=30 desktop_stores=20 desktop_versions=32 secret_material=absent`; inventory validation and `validate_spec.py` passed. Commit: `4dbf73b1d36cb2e328a9a1a02aaa47b8dd59b19f`._

  - [x] 3.4. Freeze performance and known-gap baselines
    - Record relay, fan-out, search, push, workflow, mesh and orchestration measurements plus documented incomplete behavior.
    - _Requirements: 1.3, 20.1, 20.3_
    - _Capability IDs: CAP-006, CAP-015, CAP-016, CAP-027, CAP-035, CAP-044_
    - _Depends on: 1.1, 1.4_
    - _Reads: projects/buzz/benchmarks/**, projects/buzz/perf/**, projects/buzz/TESTING.md, projects/buzz/VISION*.md_
    - _Writes: .agents/specs/collaborative-workspace/fixtures/baselines.md_
    - _Validation: baseline document records command, environment, result budget and known defect for each subsystem_
    - _Evidence: 2026-08-14 — froze seven evidence records in `.agents/specs/collaborative-workspace/fixtures/baselines.md`, each with exact source authority, reproduction command, capture environment, observed result, explicit preservation/readiness budget, known defect and downstream owner. The deterministic Redis fan-out model produced the pinned 64.0x reduction and 0.00% scoped irrelevant delivery for 1/2/4 pods, and its three unit tests passed. A focused Rust run passed 3 search unit tests with 19 PostgreSQL tests explicitly ignored, 15 push tests with 6 PostgreSQL tests ignored, 154 workflow tests with 2 PostgreSQL tests ignored, and all 32 mesh tests; none was a measured test. A standard-library Harbor audit parsed 22 Python files, discovered 53 test functions and verified all four pinned prompt hashes. The document treats absent live relay, Redis, PostgreSQL, provider, physical mesh and Harbor measurements as failed readiness conditions owned by Tasks 22.12, 41.5, 45.4 and 45.5, rather than inventing latency or throughput targets. It also freezes the explicit relay-shutdown test gap, workflow placeholder/approval gaps and adjacent media quota, Git-store wiring and moderation gaps. Docker services, `uv` and `pytest` were unavailable, so no live-service or leaderboard result is claimed. Markdown structure checks, inventory validation, `git diff --check`, and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 4. Complete the cross-boundary threat and operations review

  - [x] 4.1. Threat-model tenant, identity and protocol boundaries
    - Enumerate host confusion, replay, signing-key, authorization-before-limit and metadata leak threats with owners.
    - _Requirements: 6.3, 19.1, 19.2_
    - _Capability IDs: CAP-001, CAP-003, CAP-007, CAP-008, CAP-009_
    - _Depends on: 2.1, 2.2, 3.1_
    - _Reads: projects/buzz/SECURITY.md, projects/buzz/docs/multi-tenant-relay.md, .agents/specs/collaborative-workspace/decisions/**_
    - _Writes: .agents/specs/collaborative-workspace/security/tenant-identity.md_
    - _Validation: review maps each threat to a fail-closed control and negative test leaf_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/tenant-identity.md` with 12 mandatory invariants, 36 stable threats and ten complete boundaries spanning trusted listener/proxy provenance, row-zero tenant binding, NIP-42, NIP-98 and shared replay, signed-event/filter compatibility, common principal/identity authorization, credential lifecycle, RLS/transaction constraints, count/search/projection privacy, Redis/realtime isolation and per-community system signing/audit. Every threat maps a user-observable failure to a canonical Zed owner, fail-closed control and focused implementation/recovery leaves; every boundary records hostile input, decision order, bounds, authorization, public error, secret/cleanup behavior and tests. The review preserves Buzz's host-derived tenant, host/channel agreement, composite-key, label-flow and per-community authority semantics while recording that public trusted constructors, RLS/crypto axioms, HA replay capacity, dev `X-Pubkey`, external TLS, environment/plaintext key fallbacks, keyless audit and physical timing are gaps or deployment obligations rather than proven target guarantees. It fixes the mandatory order as transport bound → trusted tenant → crypto/replay → principal/binding → current authorization → tenant transaction → redacted result/cleanup, including authorization before existence/ranking/limit/count. A structural audit verified 36 sequential threat IDs, ten boundary records, all three requirement traces and valid task references. `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-core tenant -- --nocapture` passed 10/10 and `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-auth nip98 -- --nocapture` passed 18/18. Inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.2. Threat-model agents, providers and MCP
    - Cover hostile provider output, subprocess cleanup, tool permissions and secret separation.
    - _Requirements: 11.1, 11.5, 19.1, 19.2_
    - _Capability IDs: CAP-021, CAP-022, CAP-034_
    - _Depends on: 3.4_
    - _Reads: projects/buzz/docs/remote-agents.md, .agents/specs/goose-migration/security-permissions/**_
    - _Writes: .agents/specs/collaborative-workspace/security/agent-workflow.md_
    - _Validation: security checklist assigns bounded input/output, cancellation and permission tests to every executor boundary_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/agent-workflow.md` with 25 stable threats, eight cross-cutting invariants and 14 enumerated executor boundaries spanning collaboration ingress, ACP, model providers, MCP, compatibility mapping, filesystem/Git tools, terminal, network, local pools, jobs/delegation, provider binaries, remote substrates, mesh and observer publication. Every executor boundary names its canonical owner, hostile input, input/output bound, permission authority, cancellation/resource-cleanup contract, secret rule and focused downstream test leaves. The review preserves Zed's existing agent/ACP/tool-permission/sandbox/credentials owners, treats content scanners as non-authoritative defense in depth, freezes Buzz's provider limits and all eight documented provider defects, and records the Buzz v1 `info`/`deploy` versus canonical inspect/terminate compatibility tension without inventing a legacy wire operation. A cross-cutting checklist assigns hostile output, permission races, process trees, pre-secret negotiation, secret echo, exactly-one execution, remote shutdown and mesh failure evidence through Tasks 28.2–45.5 and the binding Goose security-permission plan. It also documents, without silently changing the plan, the extraneous direct ADR-005 dependency on Tasks 41.2 and 41.3; their required ADR-006/Task 4.8 gates remain safe through Task 41.1's transitive dependency. A structural checker verified 14 complete boundary records, 25 sequential threat IDs and all four requirement traces; inventory validation, `git diff --check`, and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.3. Threat-model media storage and rendering
    - Cover object paths, MIME confusion, decompression, previews, credentials and orphan cleanup.
    - _Requirements: 14.1, 14.2, 19.1, 19.2_
    - _Capability IDs: CAP-031_
    - _Depends on: 3.4_
    - _Reads: projects/buzz/crates/buzz-media/**, crates/media/**_
    - _Writes: .agents/specs/collaborative-workspace/security/media.md_
    - _Validation: review maps media abuse cases and resource bounds to upload, storage and rendering tests_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/media.md` with nine security invariants, 28 stable threats and ten complete boundaries covering pre-body admission, streaming/temp files, byte/type/codec/privacy validation, derived variants, canonical metadata/object commit, authorized ranges, Blossom, native GPUI decoding, link previews and checkpointed cleanup. Each boundary names its authority, abuse cases, resource limits, secret/privacy rule, failure/cancellation behavior and focused Tasks 17.9–45.3. The review preserves shared content-addressed bytes only behind server-derived tenant bindings, freezes Buzz's safe structural floors, requires durable quota reservations and immutable/verified objects, treats active content as inert downloads, and prevents adapters/caches from turning a hash hit into cross-community visibility. It records the missing Buzz durable per-principal quota, deferred orphan GC, string-shaped body-limit fallback and the assertion-shaped low-level `crates/media` FFI surface as gaps rather than preserved behavior. A structural audit verified all ten boundary records, 28 sequential threat IDs, 20 valid task references and all four requirement traces. `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-media` passed 117 unit tests with the one live-MinIO integration test explicitly ignored; `cargo test -p media` built the native bindings and reported zero tests, confirming Task 38.5 must add fallible renderer coverage. Inventory validation, `git diff --check`, and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.4. Define operational limits and telemetry constraints
    - Set measurable connection, frame, queue, retry, freshness, migration, logging and telemetry-disabled expectations.
    - _Requirements: 8.4, 19.3, 19.5_
    - _Capability IDs: CAP-004, CAP-006, CAP-028, CAP-043_
    - _Depends on: 4.1, 4.2, 4.3, 4.5, 4.6, 4.7, 4.8_
    - _Reads: .agents/specs/collaborative-workspace/security/**, .agents/specs/telemetry-disabled-default/**, projects/buzz/deploy/**_
    - _Writes: .agents/specs/collaborative-workspace/security/operational-limits.md_
    - _Validation: every limit has an owner, metric, alert threshold and focused verification task_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/operational-limits.md` as the normative registry for 55 limits across connections/protocol, durable queues/projections/search/read state, workflows/agents/remote execution, push, media/huddles, relay mesh/shared compute and migration/compatibility/health/logging/telemetry. Every stable OL-* row defines enforcement and fail-closed behavior, one canonical owner, a bounded-cardinality metric or deterministic client signal, numeric warn/page or release-stop threshold and focused downstream verification leaves. The registry preserves Buzz's established security/compatibility ceilings while supplying previously missing readiness budgets for projection/replica freshness, queues, search, cancellation, fairness, migration checkpoints, shadow lag, rollback, logging cardinality and recovery drills; it explicitly treats unmeasured values as gates for Tasks 22.12 and 45.4–45.5 rather than achieved performance. Four private dashboard groupings and seven automatic stop/rollback classes cover every limit family. Client metrics/diagnostics remain default-off under the existing `TelemetrySettings` owner with exactly zero telemetry HTTP, while local logs and required server operational metrics remain available. A structural audit verified 55 unique limit IDs, complete owner/metric-alert/verification cells, all required limit families and task references. Inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.5. Threat-model workflow and webhook execution
    - Cover webhook authentication, SSRF/redirects, conditions, retries, actions, secrets and approval bypass.
    - _Requirements: 13.2, 13.3, 19.1, 19.2_
    - _Capability IDs: CAP-027_
    - _Depends on: 3.4_
    - _Reads: projects/buzz/crates/buzz-workflow/**, projects/buzz/crates/buzz-relay/src/{workflow_sink,webhook_secret,router}.rs, projects/buzz/crates/buzz-relay/src/api/bridge.rs, projects/buzz/crates/buzz-relay/src/handlers/command_executor.rs, projects/buzz/crates/buzz-db/src/workflow.rs_
    - _Writes: .agents/specs/collaborative-workspace/security/workflow.md_
    - _Validation: review maps each trigger/action/approval threat to a bounded negative or recovery test_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/workflow.md` with ten invariants, 36 stable threats and 11 complete boundaries spanning definition activation, event/manual triggers, schedules, inbound webhooks, condition/template evaluation, durable run leases, canonical action dispatch, outbound webhooks, approvals, retry/recovery and compatibility migration. Every threat maps to focused negative or recovery leaves and every boundary records authority, abuse cases, resource limits, secret handling, failure/recovery and test ownership. The review preserves Buzz's useful tenant/owner rechecks, scheduled-fire claim, condition bounds and DNS-pinning/no-proxy/no-redirect controls while freezing the missing definition/retry/idempotency limits, query-secret risk, loopback action path, feature-disabled false-success results and incomplete actions as gaps. It identifies a concrete approval contradiction: the executor can suspend and the database/relay contain grant/deny machinery, but the common finalizer never creates the approval record and instead fails the run; the apparent event/update transaction and detached resume path are also not atomic recovery. Task 34.6 therefore owns one canonical waiting/decision/outbox transaction rather than treating either Buzz path as parity. The factual read path was corrected from nonexistent `workflow-sink.rs` to `workflow_sink.rs` and expanded to the actual webhook/approval route, secret and database owners. A structural audit verified 36 sequential threat rows, 11 boundary records and all four requirement traces. `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-workflow` passed 154 tests with two Postgres tests explicitly ignored; inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.6. Threat-model push delivery
    - Cover lease capability, wake privacy, endpoint authority, amplification, provider errors and queue bounds.
    - _Requirements: 9.5, 19.1, 19.2_
    - _Capability IDs: CAP-016_
    - _Depends on: 2.5, 3.4_
    - _Reads: projects/buzz/crates/buzz-push-gateway/**, projects/buzz/docs/nips/NIP-PL.md_
    - _Writes: .agents/specs/collaborative-workspace/security/push.md_
    - _Validation: review proves payload minimization and assigns lease, amplification, retry and redaction tests_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/push.md` with 12 mandatory invariants, 36 stable threats and ten executor boundaries spanning notification eligibility, lease wire/effective state, accepted-event matching, durable wake claims, App Attest installation authority, encrypted endpoint custody, NIP-98 gateway admission, APNs delivery, queue recovery and deployment/platform negotiation. Every threat maps an observable failure to a canonical Zed owner, fail-closed control and focused downstream negative/recovery leaves. The review proves payload minimization by fixing a payload-less executor contract and the provider-owned byte constant, while separately constraining timing/frequency leakage; it preserves Buzz's trusted-origin agreement, narrow filters, current read authorization, transactional match responsibility, community-scoped dedup, generation/endpoint revalidation, atomic replay/quota admission and sanitized provider outcomes. It records the full relay/database/gateway ownership path, discarded disposition-error strengthening, post-send-begin crash window, App Attest token-provenance assumption, static numeric bounds, unsupported FCM/UnifiedPush profiles and encrypted migration state as obligations rather than hidden parity. A structural audit verified all 12 invariant, 36 threat and ten boundary IDs plus requirement/task references. `cargo test --manifest-path projects/buzz/Cargo.toml -p buzz-push-gateway --lib -- --nocapture` passed 15 tests with six live-PostgreSQL tests explicitly ignored; inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.7. Threat-model voice and huddles
    - Cover audio authorization, devices, transcript privacy, model files, transport failure and resource cleanup.
    - _Requirements: 14.3, 14.4, 19.1, 19.2_
    - _Capability IDs: CAP-032_
    - _Depends on: 2.4, 3.4_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-004-huddle-transport.md, projects/buzz/crates/buzz-voice/**, projects/buzz/crates/buzz-relay/src/audio/**, projects/buzz/desktop/src-tauri/src/huddle/**_
    - _Writes: .agents/specs/collaborative-workspace/security/huddle.md_
    - _Validation: review maps audio/transcript/model threats to authorization, failure and cleanup tests_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/huddle.md` with 12 mandatory invariants, 36 stable threats and ten complete boundaries spanning canonical lifecycle/policy, native LiveKit token/room authority, Buzz v1/v2 gateway admission, media/codec backpressure, devices, model acquisition, imported voices/TTS, STT consent, transcript projection and terminal cleanup/retirement. Every threat maps an observable failure to a canonical Zed owner, fail-closed control and focused downstream negative/recovery leaf. The review preserves Buzz's host/NIP-42/membership admission, frame/version/peer/heartbeat bounds, lossy media versus state-control distinction, stale-generation fencing, STT queue/speech bounds, model hash/size/safe-install checks and content-addressed voice files while applying ADR-004's one-LiveKit-room/no-legacy-fallback ownership. It records community-free legacy media datagrams, droppable state-bearing control, room version pinning, silent dead-STT handles, best-effort media cleanup, model behavior trust and imported-voice privacy as strengthening or product boundaries rather than hidden parity. A structural audit verified all 12 invariant, 36 threat and ten boundary IDs plus requirement/task references. Direct `rustc --test` runs passed all four isolated Buzz v2 audio-wire tests and the pinned April INT8 model-metadata test. A broader `buzz-relay` package build exhausted the available disk before tests, so no package-wide result is claimed; its 10 GiB regenerable `projects/buzz/target` cache was removed afterward. Inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 4.8. Threat-model relay mesh and shared compute
    - Cover peer authentication, replay, stale membership, resource claims, scheduling and unapproved fallback.
    - _Requirements: 16.3, 19.1, 19.2_
    - _Capability IDs: CAP-035_
    - _Depends on: 2.6, 3.4_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-006-shared-compute.md, projects/buzz/crates/buzz-relay-mesh/**, projects/buzz/VISION_MESH.md, projects/buzz/desktop/src-tauri/src/mesh_llm/**_
    - _Writes: .agents/specs/collaborative-workspace/security/mesh.md_
    - _Validation: review assigns replay, revocation, resource, fairness and no-fallback tests_
    - _Evidence: 2026-08-14 — added `.agents/specs/collaborative-workspace/security/mesh.md` with 12 mandatory invariants, 36 stable threats and ten complete boundaries spanning deployment/tenant peer admission, relay membership and session fences, bounded transport, signed compute discovery and consent, eligibility/fair scheduling, canonical executor/resource leases, inference sandbox/context release, hostile output/no-silent-fallback, revocation/partition cleanup and deployment visibility. Every threat maps an observable failure to a canonical Zed owner and focused downstream negative/recovery leaves. The review preserves Buzz's frozen ALPN/version/frame bounds, boot-scoped endpoint identity, relay attestation, monotonic ready/gossip hints, deterministic connection handling, signed member/owner/endpoint discovery, transport allowlists and stale/revoked-member behavior while applying ADR-006's stronger bilateral opt-in, local resource authority, one canonical job lease, community-local fairness and no-silent-fallback rules. It records the missing community field in the relay frame, non-atomic desktop capacity/status state, membership-only vision language, non-deployable Kubernetes mesh provider, long runtime-management waits and unimplemented distributed-model authority as strengthening or unavailable boundaries rather than hidden parity. A structural audit verified all 12 invariant, 36 threat and ten boundary IDs plus requirement/task references. Inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

## Milestone 1 — ship the native collaborative vertical slice

- [ ] 5. Add reversible workspace presentation selection

  - [x] 5.1. Define the workspace-presentation setting
    - Add the Editor and Collaborative enum, default and settings schema without changing current startup behavior.
    - _Requirements: 3.1, 3.4_
    - _Capability IDs: CAP-037_
    - _Depends on: 1.5_
    - _Reads: crates/workspace/src/{workspace,workspace_settings}.rs, crates/settings/**, crates/settings_content/src/workspace.rs, assets/settings/default.json_
    - _Writes: crates/workspace/src/{workspace,workspace_settings,workspace_presentation}.rs, crates/settings_content/src/workspace.rs, crates/settings/src/vscode_import.rs, assets/settings/default.json_
    - _Validation: `cargo test -p workspace workspace_presentation_setting` covers default and deserialization_
    - _Evidence: 2026-08-14 — added the canonical serialized `WorkspacePresentation::{Editor, Collaborative}` enum to the existing workspace settings schema with snake-case values and `Editor` as the default, exposed it through `crates/workspace/src/workspace_presentation.rs`, and projected it into `WorkspaceSettings` without adding any startup or composition branch. `assets/settings/default.json` explicitly selects `editor`, so existing installations retain the current presentation; VS Code import leaves the Zed-only value unset so the normal default/merge path remains authoritative. Focused tests prove the Rust default, shipped default-settings value, supported/invalid deserialization and the containing `WorkspaceSettingsContent` JSON schema field/enum. Task metadata was expanded to the discovered schema/default/import integration points. `cargo test -p workspace workspace_presentation_setting` passed all three tests (220 unrelated tests filtered out). Explicit Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 5.2. Persist workspace presentation across restart
    - Store and restore the selected presentation in existing workspace persistence without copying project state.
    - _Requirements: 3.2, 3.3_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 5.1_
    - _Reads: crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation.rs, crates/workspace/src/workspace_settings.rs, crates/workspace/src/persistence.rs, crates/workspace/src/persistence/model.rs_
    - _Writes: crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation.rs, crates/workspace/src/persistence.rs, crates/workspace/src/persistence/model.rs_
    - _Validation: `cargo test -p workspace workspace_presentation_restart` verifies both modes and unchanged project identity_
    - _Evidence: 2026-08-14 — extended the existing `WorkspaceDb` row and `SerializedWorkspace` state with a checked `workspace_presentation` discriminator whose migration defaults existing rows to `editor`; the canonical settings value initializes new workspaces, while every local, empty/session-restored and remote workspace opening path restores the row-specific value before composition. The save path updates the discriminator in the same workspace upsert/savepoint as existing layout state and does not create or mutate project, worktree, Git, identity, credential, collaboration or agent-session records. Stable string codecs reject unknown values at the schema boundary and log/fall back to Editor if a corrupted database value is encountered. `workspace_presentation_restart_preserves_project_identity` round-tripped Editor and Collaborative through both database-ID and canonical-root restoration while retaining the same workspace ID, local location, reopen paths and identity paths. `cargo test -p workspace workspace_presentation_restart` passed (1/1), `cargo test -p workspace workspace_presentation` passed (4/4), `cargo test -p workspace persistence::tests` passed (44/44), `./script/clippy -p workspace` passed with release/all-target/all-feature warnings denied, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 5.3. Add presentation switching actions
    - Register reversible actions that recompose the active workspace while retaining canonical entities.
    - _Requirements: 3.3_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 5.2_
    - _Reads: crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation.rs, crates/workspace/src/workspace_settings.rs, crates/workspace/src/persistence.rs, crates/settings/src/settings_file.rs_
    - _Writes: crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation_actions.rs_
    - _Validation: `cargo test -p workspace switch_workspace_presentation` checks entity IDs and navigation survive both transitions_
    - _Evidence: 2026-08-14 — added public `SwitchToEditorWorkspace` and `SwitchToCollaborativeWorkspace` GPUI actions in the dedicated presentation-action module and registered them on the existing `Workspace::actions` chain (the planned `crates/workspace/src/actions.rs` integration point does not exist). Each action changes only the presentation discriminator on the live `Workspace`, updates the canonical user preference through the existing settings writer, schedules the existing workspace serializer and notifies GPUI to recompose; it does not replace or copy the workspace, project, panes, items, navigation or persistence aggregates, and same-presentation dispatch remains entity-idempotent. The rendered `MultiWorkspace` action test performs Editor → Collaborative → Editor transitions and proves the workspace/project/active-pane/active-item entity IDs, backward/forward navigation state and recent-navigation history remain identical, while both the settings preference and existing `WorkspaceDb` row track each target. `cargo test -p workspace switch_workspace_presentation` passed (1/1), `cargo test -p workspace workspace_presentation` passed (5/5), `cargo test -p workspace test_pane_navigation` passed (1/1), `./script/clippy -p workspace` passed with release/all-target/all-feature warnings denied, and Rust 2024 formatting plus `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 5.4. Add onboarding workspace-choice controls
    - Render two accessible choices with concise shared-data explanation and save the chosen setting.
    - _Requirements: 3.1, 3.2_
    - _Capability IDs: CAP-037_
    - _Depends on: 5.1_
    - _Reads: crates/onboarding/src/onboarding.rs, crates/onboarding/src/basics_page.rs, crates/ui/src/components/button/**, crates/settings/src/settings_file.rs, crates/settings/src/settings_store.rs, crates/workspace/src/workspace_presentation.rs, crates/workspace/src/workspace_settings.rs_
    - _Writes: crates/onboarding/Cargo.toml, crates/onboarding/src/onboarding.rs, crates/onboarding/src/basics_page.rs, crates/onboarding/src/workspace_choice.rs_
    - _Validation: `cargo test -p onboarding workspace_choice` verifies labels, selection, keyboard activation and persisted value_
    - _Evidence: 2026-08-14 — added a native GPUI Workspace section at the start of the existing onboarding basics page with exactly “Editor Workspace” and “Collaborative Workspace,” visible descriptions, a concise same-projects-and-data explanation, stable tab order, native focus treatment, assistive labels and toggle-state semantics. Selection derives directly from canonical `WorkspaceSettings`; Enter/Space activation writes `settings.workspace.workspace_presentation` through the existing settings-file owner, and no onboarding-specific presentation store or project copy was introduced. The test-only `settings/test-support` feature enables deterministic GPUI/FakeFs coverage without changing production dependencies. `cargo test -p onboarding workspace_choice -- --nocapture` passed 1/1 and proves initial Editor selection, Space activation of Collaborative, Enter activation of Editor and both persisted JSON values; `cargo test -p onboarding` passed the crate test and doc-test targets; `cargo fmt --all -- --check`, `./script/clippy -p onboarding`, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 5.5. Cover existing-user and initialization failure behavior
    - Prove existing users remain in Editor and a failed Collaborative initialization offers a recoverable Editor fallback.
    - _Requirements: 3.3, 3.4_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 5.3, 5.4_
    - _Reads: crates/onboarding/src/workspace_choice.rs, crates/workspace/src/{collaborative_shell_state,collaborative_workspace,workspace_presentation,workspace_presentation_actions,workspace}.rs, crates/workspace/src/persistence/model.rs_
    - _Writes: crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace workspace_presentation` covers upgrade, failure, retry and explicit later switch_
    - _Evidence: 2026-08-14 — after Task 6.6 supplied the canonical shell-failure boundary, added a rendered GPUI regression that starts from the shipped/default Editor presentation, explicitly enters Collaborative, projects initialization failure, retries once without switching presentation, projects the repeated failure, and activates the visible Editor fallback. The fallback and a later explicit return to Collaborative both persist through the existing workspace/settings owners while preserving the exact Workspace, Project, active Pane and active Item entities; the recovered Collaborative shell no longer shows stale failure state. This extends, rather than duplicates, Task 5.1's default/schema upgrade checks, Task 5.2's missing-row/default migration and restart coverage, and Task 5.3's navigation-preservation coverage. The planned integration-test path does not exist because `workspace` keeps these GPUI fixtures in its library test module, so the discovered write path is `crates/workspace/src/workspace.rs`. `cargo test -p workspace workspace_presentation -- --nocapture` passed all six matching default, schema, upgrade/restart, reversible-switch and failure/retry/fallback tests; the full workspace suite passed 231/231. `./script/clippy -p workspace`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 6. Compose the collaborative GPUI shell

  - [x] 6.1. Add the CollaborativeWorkspace composition root
    - Create the native GPUI view and select it from the approved presentation setting.
    - _Requirements: 4.1_
    - _Capability IDs: CAP-036_
    - _Depends on: 5.3, 5.5_
    - _Reads: crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation_actions.rs, crates/workspace/src/pane.rs, crates/workspace/src/dock.rs, crates/workspace/src/status_bar.rs, crates/ui/src/**_
    - _Writes: crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation_actions.rs_
    - _Validation: `cargo test -p workspace collaborative_workspace_mounts` proves no React or Tauri process is launched_
    - _Evidence: 2026-08-14 — added a native `CollaborativeWorkspace` GPUI entity owned by the existing `Workspace`, holding the same canonical `Entity<Project>` rather than a copied project model. `Workspace::render` now selects the existing editor composition or this native root from the persisted `WorkspacePresentation` discriminator; the new module depends only on native Rust/GPUI/project/UI crates and has no React, Tauri or child-process integration. Presentation transitions move focus to the active presentation so the reverse switch remains reachable after the Collaborative root has painted. `cargo test -p workspace collaborative_workspace_mounts -- --nocapture` passed and proves Editor is the initial rendered composition, switching mounts the native Collaborative root with the identical project entity, and the action path switches back to Editor; `cargo test -p workspace workspace_presentation`, `cargo test -p workspace switch_workspace_presentation`, the full 226-test workspace suite, `cargo fmt --all -- --check`, `./script/clippy -p workspace` and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 6.2. Implement the collaborative top-bar layout
    - Compose title, participant region, share/invite actions and connection/layout affordances with native components.
    - _Requirements: 4.1, 4.4_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1_
    - _Reads: crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/workspace.rs, crates/title_bar/src/title_bar.rs, crates/title_bar/src/collab.rs, crates/ui/src/components/button/**_
    - _Writes: crates/workspace/src/collaborative_top_bar.rs, crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_top_bar` checks hierarchy, labels and unavailable-action states_
    - _Evidence: 2026-08-14 — added a native theme-token-driven top bar to the Collaborative composition with canonical first-visible-worktree title projection, an explicit no-active-task state, participant region, share/invite controls, connection state and review/editor layout affordances. Share, invite, connection details and review layout fail visibly unavailable until their later canonical owners are bound; each icon action has an assistive label and reason tooltip. The Editor Workspace control dispatches the existing presentation action, and the top bar introduces no participant, connection, task or project store. `cargo test -p workspace collaborative_top_bar -- --nocapture` passed and verifies all eight ordered regions render, the projected labels and action-availability model are truthful, and the enabled Editor control performs the reverse presentation transition; `cargo test -p workspace collaborative_workspace_mounts`, the full 227-test workspace suite, `cargo fmt --all -- --check`, `./script/clippy -p workspace` and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 6.3. Implement the left-rail layout container
    - Add pinned, community/project and task/thread sections with independent scrolling and native density tokens.
    - _Requirements: 4.1, 4.2_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1_
    - _Reads: crates/sidebar/src/sidebar.rs, crates/workspace/src/{collaborative_workspace,multi_workspace}.rs, crates/ui/src/{components,divider,scroll}.rs_
    - _Writes: crates/sidebar/src/collaborative_rail.rs, crates/sidebar/src/sidebar.rs, crates/sidebar/src/sidebar_tests.rs, crates/workspace/src/multi_workspace.rs_
    - _Validation: `cargo test -p sidebar collaborative_rail_layout` checks section order and bounded scrolling_
    - _Evidence: 2026-08-14 — added a native `CollaborativeRail` entity under the existing `Sidebar` owner with ordered Pinned, Communities and Projects, and Tasks and Threads regions, theme-token density, explicit empty states and three independent retained `ScrollHandle`s. `MultiWorkspace` now presents the registered sidebar on the left whenever the active workspace uses the Collaborative presentation, including when the Editor sidebar preference is closed, without changing that preference, its configured side or its persisted width; the rail receives the correct left-side divider and does not expose the Editor-only resize, close, history or recent-project controls. Resize interaction remains disabled until Task 6.5 introduces independently owned Collaborative layout state. No project, community, task, thread or navigation store was added. `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed and verifies section geometry, bounded nonzero regions, labels, independent scroll offsets and unchanged Editor sidebar state; the full 141-test sidebar suite and full 227-test workspace suite passed. `cargo fmt --all -- --check`, `./script/clippy -p sidebar`, `./script/clippy -p workspace`, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 6.4. Implement timeline and review split geometry
    - Add resizable central/review regions with minimum sizes and a full-width collapsed timeline state.
    - _Requirements: 4.1, 4.2_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1, 6.2_
    - _Reads: crates/workspace/src/{collaborative_workspace,collaborative_top_bar,dock,workspace}.rs, crates/gpui/src/elements/div.rs_
    - _Writes: crates/workspace/src/collaborative_layout.rs, crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/collaborative_top_bar.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_layout_bounds` checks expanded, collapsed and narrow constraints_
    - _Evidence: 2026-08-14 — added one native `CollaborativeLayout` entity to the existing Collaborative workspace composition, with a central timeline region, optional Review Changes region, a draggable native GPUI divider and pure geometry constraints that preserve a 480px timeline and 320px review minimum. The top-bar review affordance now toggles the same layout owner; explicit collapse produces a full-width timeline, reopening restores the retained width, and viewports below the 806px combined minimum hide the review surface without mutating requested visibility or width. The layout holds presentation geometry only and introduces no transcript, diff, project or Git state; persistence remains owned by Task 6.5. The planned `crates/ui/src/resizable.rs` integration point does not exist, so the implementation reuses the existing workspace dock/div drag idiom and records the discovered GPUI source boundary. `cargo test -p workspace collaborative_layout_bounds -- --nocapture`, `cargo test -p workspace collaborative_top_bar -- --nocapture`, `cargo test -p workspace collaborative_workspace_mounts -- --nocapture` and the full 228-test workspace suite passed. `cargo fmt --all -- --check`, `./script/clippy -p workspace` and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 6.5. Persist collaborative layout state
    - Persist review visibility, width and collaborative rail width independently of Editor layout state.
    - _Requirements: 4.2, 4.3_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.3, 6.4_
    - _Reads: crates/workspace/src/{collaborative_layout,collaborative_workspace,multi_workspace,persistence,workspace}.rs, crates/sidebar/src/{sidebar,sidebar_tests}.rs, crates/db/src/kvp.rs_
    - _Writes: crates/workspace/src/collaborative_layout_persistence.rs, crates/workspace/src/collaborative_layout.rs, crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/multi_workspace.rs, crates/workspace/src/workspace.rs, crates/sidebar/src/sidebar.rs, crates/sidebar/src/sidebar_tests.rs_
    - _Validation: `cargo test -p workspace collaborative_layout_restart` verifies round trip, bounds clamping and Editor isolation_
    - _Evidence: 2026-08-14 — added one versioned Collaborative-layout record in Zed's existing scoped key-value persistence, keyed by the canonical `WorkspaceId` and synchronously restored before the Collaborative composition is created. The record owns only requested review visibility, retained review width and Collaborative rail width; writes ride the existing throttled/flushable workspace serialization lifecycle for local, remote, empty and detached workspaces. Review widths clamp to 320–1600px and rail widths to 200–800px, missing records use native defaults, and malformed or unsupported-version records log and fail safely to defaults. `MultiWorkspace` and `Sidebar` now project and resize the retained Collaborative rail width only while that presentation is active, including double-click reset, without changing the Sidebar entity's separately serialized Editor width or its `multi_workspace_state` namespace. No schema, project record, pane model or duplicate persistence service was introduced. `cargo test -p workspace collaborative_layout_restart -- --nocapture` passed its round-trip, clamp, per-workspace, corrupt/future-version and exact Editor-state-isolation checks; `cargo test -p workspace collaborative_layout -- --nocapture` passed both persistence and rendered review geometry tests; `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed native drag-resize and Editor-width-isolation coverage; the full 229-test workspace suite and full 141-test sidebar suite passed. `cargo fmt --all -- --check`, `./script/clippy -p workspace -p sidebar` and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 6.6. Add shell loading and initialization-error states
    - Render bounded loading, unavailable-service and retry states without discarding presentation or project context.
    - _Requirements: 4.1, 8.3_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.1, 6.2, 6.3, 6.4, 6.5_
    - _Reads: crates/workspace/src/{collaborative_workspace,workspace_presentation_actions,workspace}.rs, crates/ui/src/components/{banner,button,label}.rs_
    - _Writes: crates/workspace/src/collaborative_shell_state.rs, crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_shell_state` covers loading, retry, partial failure and recovery_
    - _Evidence: 2026-08-14 — added one transient `CollaborativeShellState` entity to the native Collaborative composition with scoped Ready, Loading, PartialFailure, InitializationFailed and Retrying phases; it owns no transcript, service, project or persistence state. Theme-native banners expose the affected scope and last trustworthy state, keep the existing project and Collaborative layout mounted, and offer a retry control that emits exactly one scoped request while remaining visibly Retrying until a later canonical service binding reports recovery. Initialization failure additionally exposes an explicit “Open Editor Workspace” action through the existing presentation switch owner rather than silently changing modes. The future service-construction boundary carries a narrow release-build dead-code expectation until its planned bindings land. `cargo test -p workspace collaborative_shell_state -- --nocapture` passed 1/1, the full workspace suite passed 230/230, `./script/clippy -p workspace`, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 7. Bind collaborative navigation to existing stores

  - [x] 7.1. Define collaborative navigation row projection
    - Project existing entities into stable row IDs, groups and state badges without creating another store.
    - _Requirements: 4.3_
    - _Capability IDs: CAP-036, CAP-042_
    - _Depends on: 6.3, 6.6_
    - _Reads: crates/sidebar/src/collaborative_rail.rs, crates/project/src/{project,worktree_store}.rs, crates/channel/src/channel_store.rs, crates/agent_ui/src/thread_metadata_store.rs_
    - _Writes: Cargo.lock, crates/sidebar/Cargo.toml, crates/sidebar/src/sidebar.rs, crates/sidebar/src/collaborative_navigation.rs_
    - _Validation: `cargo test -p sidebar collaborative_navigation_projection` verifies stable IDs and one row per source entity_
    - _Evidence: 2026-08-14 — added a UI-independent, non-persisted navigation projection contract under the existing Sidebar owner. Typed canonical source identities use `ProjectGroupKey`, project-scoped `WorktreeId`, channel ID and `ThreadId`; row identity additionally includes the presentation group so one source may legitimately appear once in Pinned and once in its canonical group, while duplicate rows for the same source/group fail explicitly instead of being silently overwritten. Constructors derive directly from `Project`, `Channel` and `ThreadMetadata`, retain input ordering, and carry labels plus Unread, Running, WaitingForUser, Failed, Archived and Completed badges without observing or copying any store. The module has a narrow release dead-code expectation until Tasks 7.2–7.4 bind its consumers. Four focused tests prove stable identity across channel/thread renames, exact project-entity projection, canonical grouping, badge retention, valid pinned references and duplicate rejection; `cargo test -p sidebar collaborative_navigation_projection -- --nocapture` passed 4/4 and the full sidebar suite passed 145/145. `./script/clippy -p sidebar`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 7.2. Populate pinned and recent work groups
    - Bind existing pinned/recent project and task records with empty and unavailable states.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-036_
    - _Depends on: 7.1_
    - _Reads: crates/recent_projects/src/sidebar_recent_projects.rs, crates/workspace/src/persistence.rs, crates/agent_ui/src/thread_metadata_store.rs, crates/sidebar/src/{collaborative_navigation,collaborative_rail,sidebar}.rs_
    - _Writes: crates/sidebar/src/collaborative_pinned.rs, crates/sidebar/src/collaborative_rail.rs, crates/sidebar/src/sidebar.rs, crates/sidebar/src/sidebar_tests.rs_
    - _Validation: `cargo test -p sidebar collaborative_pinned` covers order, removal and missing targets_
    - _Evidence: 2026-08-14 — added a native `CollaborativePinned` projection under the existing rail. It loads canonical recent projects from `WorkspaceDb`, observes canonical `ThreadMetadataStore` updates, excludes archived threads, merges candidates by descending source timestamp, preserves explicit pin order ahead of up to eight recent rows and never persists a second list. Missing pinned targets remain observable, duplicate pin or recent records fail explicitly, and loading, empty and unavailable states use native theme components. Source audit found no existing Zed or Buzz project/task pin-record owner to bind; the runtime therefore supplies no synthetic pin records and truthfully renders canonical recent work or its empty/unavailable state, while the projection accepts ordered references from a future approved canonical owner without claiming one exists. Four focused tests cover ordering, removal, archived filtering, missing targets, malformed duplicates and rendered empty/unavailable states; `cargo test -p sidebar collaborative_pinned -- --nocapture` passed 4/4, `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed 1/1, and the full sidebar suite passed 149/149. `./script/clippy -p sidebar`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 7.3. Populate project, repository and worktree groups
    - Render canonical Zed project hierarchy and selection without deriving duplicate project records.
    - _Requirements: 3.3, 4.3_
    - _Capability IDs: CAP-018, CAP-036_
    - _Depends on: 7.1, 7.2_
    - _Reads: crates/workspace/src/multi_workspace.rs, crates/project/src/{project,git_store}.rs, crates/worktree/src/worktree.rs, crates/sidebar/src/{collaborative_navigation,collaborative_rail}.rs_
    - _Writes: crates/sidebar/src/collaborative_navigation.rs, crates/sidebar/src/collaborative_projects.rs, crates/sidebar/src/collaborative_rail.rs, crates/sidebar/src/sidebar.rs_
    - _Validation: `cargo test -p sidebar collaborative_projects` checks multiple repositories/worktrees and deleted worktrees_
    - _Evidence: 2026-08-14 — added a native `CollaborativeProjects` projection that reads ordered `MultiWorkspace::project_groups` and the live canonical Project repository and visible-Worktree entities on every render. Project rows retain `ProjectGroupKey`; repository identities add the canonical work-directory path; worktree identities now add their canonical absolute path to the project-scoped `WorktreeId`, preventing collisions when multiple Project entities in one linked-worktree group reuse numeric IDs. Repository and worktree rows are deterministically path-sorted, exact canonical duplicates reached through multiple workspaces collapse to one presentation row, removed worktrees disappear on the next source projection, and missing `MultiWorkspace`, empty and malformed duplicate-group states remain visible. The rail now mounts this entity in the existing independently scrolling Communities and Projects section and introduces no project/repository/worktree persistence. Three focused tests cover multiple repositories, colliding numeric worktree IDs at distinct paths, stable order, deleted-worktree removal and duplicate-group failure; `cargo test -p sidebar collaborative_projects -- --nocapture` passed 3/3, `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed 1/1, and the full sidebar suite passed 152/152. `./script/clippy -p sidebar`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 7.4. Populate task and thread history groups
    - Bind active, historical and archived ACP/thread metadata with running, waiting, failed and completed indicators.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-021, CAP-025, CAP-036_
    - _Depends on: 7.1, 7.3_
    - _Reads: crates/agent_ui/src/{thread_metadata_store,agent_panel,conversation_view}.rs, crates/acp_thread/src/acp_thread.rs, crates/sidebar/src/{collaborative_navigation,collaborative_rail,sidebar}.rs_
    - _Writes: crates/sidebar/src/collaborative_tasks.rs, crates/sidebar/src/collaborative_rail.rs, crates/sidebar/src/sidebar.rs_
    - _Validation: `cargo test -p sidebar collaborative_tasks` checks state transitions, archive and history ordering_
    - _Evidence: 2026-08-14 — added a native `CollaborativeTasks` projection that observes canonical `ThreadMetadataStore` records and derives live status through the existing AgentPanel/ACP conversation projection rather than introducing another task/session store. Archived metadata always projects Archived; live ACP Running, WaitingForConfirmation, Error and Completed map to Running, Waiting for user, Failed and Completed; drafts remain explicitly Draft without a false terminal badge; unloaded non-archived history projects Completed because Zed's canonical metadata does not persist a separate terminal outcome. Rows sort active Running/Waiting/Failed first, then drafts, completed history newest-first and archived history last. The existing rail mounts the entity in its independent Tasks and Threads scroller with native empty/unavailable states. Three focused tests exhaust live transitions and badges, active/history/archive ordering, and draft treatment; `cargo test -p sidebar collaborative_tasks -- --nocapture` passed 3/3, `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed 1/1, and the full sidebar suite passed 155/155. The lack of a persisted historical failure outcome is recorded rather than filled with duplicate state; Tasks 8.1–8.4 own the canonical lifecycle/outcome projection. `./script/clippy -p sidebar`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 7.5. Define workspace navigation targets, history and persistence
    - Own typed collaborative navigation targets, safe entity-link resolution, back/forward history and selected-context persistence without importing sidebar row types.
    - _Requirements: 4.3, 16.4_
    - _Capability IDs: CAP-036, CAP-042_
    - _Depends on: 7.2, 7.3, 7.4_
    - _Reads: crates/workspace/src/{path_link,persistence,workspace}.rs, crates/project/src/project.rs_
    - _Writes: crates/workspace/src/collaborative_navigation.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_navigation` covers back/forward, restart, missing entity and unsafe link rejection_
    - _Decomposition resolution (2026-08-14): approved split keeps the reusable target/history/persistence owner in `workspace`; Task 7.6 performs sidebar-specific activation and deep-link dispatch through that lower-level contract._
    - _Evidence: 2026-08-14 — added the UI-free `workspace::collaborative_navigation` owner with typed community, canonical project-group, repository, worktree, channel, thread, Buzz message, hosted repository/project, pull-request and issue targets. Project targets preserve Zed's path-list and non-secret remote-identity semantics; an explicit regression proves runtime SSH passwords are never serialized. The workspace owns a bounded versioned current/back/forward state under the existing `WorkspaceId` and schedules it through the existing serialization lifecycle rather than adding a project, thread or selection store. Navigation rejects invalid or unavailable targets without mutation, preserves forward history until a successful new selection, and exposes persistence-aware workspace methods for the sidebar adapter in Task 7.6. The compatibility parser accepts the documented `buzz://message|repo|project|pr|issue` forms, lowercase-normalizes cryptographic IDs, enforces exact required/single query parameters and rejects oversized URLs, credentials, ports, paths, fragments, unknown parameters, invalid hex and unsafe d-tags. Malformed or future persisted state fails closed to an empty navigation state. `cargo test -p workspace collaborative_navigation --no-fail-fast` passed 6/6, the full `cargo test -p workspace --no-fail-fast` suite passed 237/237, `./script/clippy -p workspace`, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 7.6. Bind sidebar rows to workspace navigation
    - Convert pinned, project/worktree and task/thread row activation plus supported entity links into the workspace-owned targets without persisting a second selection.
    - _Requirements: 4.3, 16.4_
    - _Capability IDs: CAP-036, CAP-042_
    - _Depends on: 7.5_
    - _Reads: crates/workspace/src/collaborative_navigation.rs, crates/sidebar/src/collaborative_{navigation,pinned,projects,tasks}.rs_
    - _Writes: crates/sidebar/src/collaborative_pinned.rs, crates/sidebar/src/collaborative_projects.rs, crates/sidebar/src/collaborative_tasks.rs_
    - _Validation: `cargo test -p sidebar collaborative_navigation_activation` covers every row type, back/forward dispatch, missing targets and unsafe entity links_
    - _Implementation contradiction (2026-08-14): the approved write set contains the pinned/project/task row surfaces but no entity-link consumer or back/forward control surface. Those two behaviors are already canonically implemented below the sidebar by Task 7.5; adding an unused sidebar parser/history wrapper would duplicate ownership and violate the architecture. Task 7.6 therefore binds every currently rendered row type and validates the existing Task 7.5 entity-link/history contract as a combined checkpoint. The first actual entity-link consumer must call `workspace::collaborative_navigation::target_from_entity_link` directly rather than introduce a sidebar copy._
    - _Evidence: 2026-08-14 — made pinned project/thread, project-group, repository, worktree and task/thread rows native clickable GPUI targets without adding selection persistence. Project-family activation re-resolves the live `MultiWorkspace` project hierarchy, activates the owning workspace and records the workspace-owned typed target only after the repository/worktree still exists. Task activation re-resolves canonical `ThreadMetadata`, selects the matching project-group workspace, loads and focuses the existing `AgentPanel` thread, then records the same canonical target. Pinned rows delegate to those same project/thread activation functions so they cannot report a false selection or drift from primary-row behavior. Missing workspaces, projects, repositories, worktrees, threads and agent surfaces produce visible themed errors and do not mutate navigation. Three focused mapping fixtures cover every rendered row family; `cargo test -p sidebar collaborative_navigation_activation --no-fail-fast` passed 3/3 and the full sidebar suite passed 158/158 in an isolated target. The combined `cargo test -p workspace collaborative_navigation --no-fail-fast` contract passed 6/6 for back/forward dispatch, missing targets, restart persistence, supported Buzz entity links, unsafe-link rejection and credential-free project identity. All-target/all-feature clippy with `--deny warnings`, Rust 2024 formatting and `git diff --check` passed. The repository release-profile clippy wrapper was also attempted, but a concurrent external workspace rebuild exhausted the shared disk; its equivalent warning policy passed against the isolated dev graph after the release-only partial artifacts were cleaned. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 8. Project existing ACP activity into the central timeline

  - [x] 8.1. Define the ActivityItem projection contract
    - Add stable source identity, semantic class, actor/verb/object/outcome, lifecycle and detail-link fields without GPUI dependencies.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-025, CAP-036_
    - _Depends on: 3.1, 6.1_
    - _Reads: crates/acp_thread/src/acp_thread.rs, crates/action_log/src/**, projects/buzz/VISION_ACTIVITY.md_
    - _Writes: crates/agent_ui/src/activity_projection.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui activity_projection_contract` covers stable identity and serializable detail handles_
    - _Evidence: 2026-08-14 — added a GPUI-free, serde-compatible `ActivityItem` contract with non-empty stable `(source_kind, source_id)` identity, independent monotonic `source_version`, all twelve semantic render classes from Buzz's activity vision, actor/verb/object/outcome, terminal-aware lifecycle, canonical context and visibility, occurrence/projection timestamps, and typed ACP/action/protocol/Git/workflow/raw detail handles plus stable action/Git/entity links. Identity remains unchanged as a source advances from running to terminal state, allowing Task 8.4 to replace rows in place; unknown semantics retain explicit Generic/Raw classes for truthful fallback. Four focused tests cover stable versioned identity, empty-ID rejection, every detail-handle JSON round trip and complete-item JSON round trip; `cargo test -p agent_ui activity_projection_contract -- --nocapture` passed 4/4 and the full agent_ui suite passed 398/398. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed. The crate-root module declaration was added to the discovered write set so downstream mapping leaves can consume the public contract. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 8.2. Map ACP messages and lifecycle events
    - Project human/agent messages, session start/stop, idle, disconnect and cancellation into one item each.
    - _Requirements: 12.1, 12.4_
    - _Capability IDs: CAP-021, CAP-025_
    - _Depends on: 8.1_
    - _Reads: crates/acp_thread/src/**, crates/agent_ui/src/activity_projection.rs_
    - _Writes: crates/agent_ui/src/activity_acp.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui activity_acp_mapping` exhausts ACP message and lifecycle fixtures_
    - _Evidence: 2026-08-14 — added an ACP adapter over canonical `UserMessage`, `AssistantMessage`, `AssistantMessageChunk`, `AcpThreadEvent`, `ThreadStatus`, `SessionId` and `StopReason` types. Human messages prefer the stable Zed client message ID so optimistic acknowledgement does not replace the row, then fall back to protocol ID or deterministic entry identity; agent message and thought chunks use protocol IDs or deterministic entry/chunk identities and each project exactly once. Canonical thread status, stopped, error and load-error events normalize directly into lifecycle inputs, while connection owners can supply the explicit disconnected state. Every lifecycle input requires a non-empty caller-owned event ID and truthfully maps started, idle, disconnected, failed, successful end-turn, token/request limits, refusal and explicit user cancellation to semantic verb/object/outcome and lifecycle values; repeated lifecycle versions retain one item ID for Task 8.4 in-place reduction. Canonical session, project, thread, actor and visibility context is preserved, and details point back to typed ACP entry handles. Four focused fixtures cover optimistic acknowledgement, message/thought cardinality, every current ACP stop reason plus connection/failure states, direct thread-event normalization, invalid lifecycle identity and stable lifecycle updates; `cargo test -p agent_ui activity_acp_mapping -- --nocapture` passed 4/4. The full agent_ui suite passed 402/402 after one unrelated remote-connection migration test transiently failed in the first full run, passed immediately in isolation, and passed in the clean rerun. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed; clippy initially exhausted disk while writing recoverable build metadata, and `cargo clean -p agent_ui` removed only 3.8 GiB of that package's rebuildable artifacts before the successful rerun. The crate-root export was added to the discovered write set. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 8.3. Map native tool and permission activity
    - Project reads, searches, edits, shell commands, tests and permission requests with truthful outcomes.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-022, CAP-025_
    - _Depends on: 8.1, 8.2_
    - _Reads: crates/action_log/src/**, crates/agent_ui/src/activity_projection.rs_
    - _Writes: crates/agent_ui/src/activity_actions.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui activity_action_mapping` maps every registered action kind or generic fallback_
    - _Evidence: 2026-08-14 — added a native action adapter over canonical ACP `ToolCallId`, `ToolKind`, `ToolCallStatus` and authorization state. Every current tool kind maps to a truthful semantic verb/object/class: reads, edits, deletes, moves, searches, shell commands, recognized test commands, thoughts, fetches, mode switches and Other; future non-exhaustive kinds use the same Generic fallback instead of fabricated semantics. Pending, running, completed, failed, rejected and cancelled states map to explicit lifecycle/outcome values, while permission grants and action choices produce distinct waiting-for-user Permission items without executing or persisting a second permission decision. Stable tool-call identity survives lifecycle versions, typed ACP details remain available, and the item links the existing canonical action ID for later diff resolution. The `ActionLog` audit confirmed that it owns aggregate buffer/diff/review state but no per-operation event registry, so the adapter does not invent duplicate ActionLog events or attach aggregate diff totals to individual calls. Four focused tests cover all ten current kinds, Generic fallback, six ordinary statuses, both permission modes, stable action links and bounded test-command recognition; `cargo test -p agent_ui activity_action_mapping -- --nocapture` passed 4/4 and the full agent_ui suite passed 406/406. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed. The first full-suite attempt could not link because the shared target filled; removing only the recoverable 24 GiB `target/debug/incremental` compiler cache restored 26 GiB free, after which the suite passed. The crate-root export was added to the discovered write set, and the dependency now serializes that shared write after Task 8.2. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 8.4. Coalesce streaming and state updates in place
    - Reduce fragments and lifecycle transitions by source ID without duplicate terminal rows.
    - _Requirements: 12.3, 12.4_
    - _Capability IDs: CAP-025_
    - _Depends on: 8.2, 8.3_
    - _Reads: crates/agent_ui/src/activity_{projection,acp,actions}.rs_
    - _Writes: crates/agent_ui/src/activity_reducer.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui activity_reducer` covers duplicate, reordered, cancelled and timed-out updates_
    - _Evidence: 2026-08-14 — added an ordered `ActivityReducer` keyed exclusively by canonical `ActivityItemId`. First observations append once; higher source versions replace the same slot; identical same-version payloads deduplicate; lower reordered versions are ignored; and different payloads at one version fail explicitly without mutating accepted state. Nonterminal lifecycle snapshots may advance or resume in place, while terminal items cannot return to nonterminal state, change terminal lifecycle, or change terminal outcome status; later versions may enrich detail for the same terminal result. Six focused tests cover duplicate delivery, reordered stale updates, streaming content replacement, cancelled-item resurrection rejection, timed-out terminal enrichment/conflict and divergent same-version payloads; `cargo test -p agent_ui activity_reducer -- --nocapture` passed 6/6 and the full agent_ui suite passed 412/412. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed. The crate-root export was added to the discovered write set, already serialized after both mapping prerequisites. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 8.5. Render the virtualized collaborative timeline
    - Render projected items with semantic summaries, progressive details and a truthful unknown-event row.
    - _Requirements: 4.1, 12.1, 12.2_
    - _Capability IDs: CAP-025, CAP-036_
    - _Depends on: 8.4_
    - _Reads: crates/agent_ui/src/activity_{projection,reducer}.rs, crates/agent_ui/src/conversation_view/**_
    - _Writes: crates/agent_ui/src/collaborative_timeline.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui collaborative_timeline_render` checks ordering, virtualization and detail disclosure_
    - _Evidence: 2026-08-14 — added a native GPUI `CollaborativeTimeline` over the canonical `ActivityReducer`, using GPUI's variable-height `ListState` with tail-following rather than materializing every feed row or persisting a second transcript. Inserts splice one row, streaming and lifecycle replacements remeasure only their stable row, and duplicate or stale updates do not disturb the list. Rows render semantic actor/verb/object summaries, lifecycle and outcome state, and typed ACP/action/protocol/Git/workflow detail handles behind an in-place disclosure control; Generic and Raw classes visibly identify unsupported activity and preserve source identity instead of inventing semantics. Empty state, separators, surfaces and text all use native Zed/GPUI components and theme tokens. Four focused tests cover reducer order, a 1,000-item virtual-list count, disclosure-state transitions with typed detail text, and truthful unknown-event fallback; `cargo test -p agent_ui collaborative_timeline_render -- --nocapture` passed 4/4 and the full agent_ui suite passed 416/416. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed. The crate-root export was added to the discovered write set, serialized after Task 8.4. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 8.6. Add ACP activity projection regression fixtures
    - Lock exactly-once mappings and empty/waiting/error behavior for the Milestone 1 source catalog.
    - _Requirements: 12.1, 12.3, 12.4, 20.1_
    - _Capability IDs: CAP-021, CAP-022, CAP-025, CAP-044_
    - _Depends on: 8.2, 8.3, 8.4, 8.5_
    - _Reads: .agents/specs/collaborative-workspace/fixtures/protocol/**, crates/agent_ui/src/activity_*.rs_
    - _Writes: crates/agent_ui/tests/collaborative_activity.rs_
    - _Validation: `cargo test -p agent_ui collaborative_activity` passes with no unmapped source kind_
    - _Evidence: 2026-08-14 — added a public-surface integration suite that enumerates the complete current Milestone 1 projection catalog: human messages, assistant messages, assistant thought summaries, all ten registered ACP tool kinds, and started/idle/completed lifecycle events. The catalog asserts every named source maps to a distinct canonical activity ID, every first delivery inserts one row, every repeat delivery deduplicates, and no expected fixture name is absent. Separate recovery fixtures verify an empty thread emits no rows and that permission waiting, explicit disconnection reasons and agent failures retain their semantic class, lifecycle and user-visible outcome. A compatibility check parses the checked-in protocol manifest, pins schema version 1, preserves mixed legacy/v2 coverage and records Buzz verification as the fixture authority. `cargo test -p agent_ui collaborative_activity -- --nocapture` passed 3/3; the full crate run passed 416 unit plus 3 integration tests. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 9. Integrate native diff review into the collaborative shell

  - [x] 9.1. Define the dependency-safe collaborative review host
    - Add a workspace-owned host and registration contract for canonical project-bound review surfaces without naming concrete `agent_ui` or `git_ui` types.
    - _Requirements: 4.1, 10.4_
    - _Capability IDs: CAP-020, CAP-036_
    - _Depends on: 6.4, 7.5, 8.5_
    - _Reads: crates/workspace/src/{collaborative_layout,workspace}.rs, crates/project/src/{project,git_store}.rs_
    - _Writes: crates/workspace/src/collaborative_review.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_review_host` verifies one project-bound provider per slot, canonical project identity, registration failure and pane collapse_
    - _Decomposition resolution (2026-08-14): approved split keeps the host contract below `agent_ui` and `git_ui`; Tasks 9.6 and 9.7 adapt concrete panes independently and Task 9.8 performs the upper registration/mount._
    - _Evidence: 2026-08-14 — added a workspace-owned `CollaborativeReviewHost` with dependency-safe agent-change and project-change slots, exact canonical `Project` entity checks, typed mismatch/occupied/unavailable failures, deterministic selection, registration-token-safe removal and collapse-aware view exposure that retains provider state. `Workspace` now constructs the host from its existing project and exposes notifying registration, selection, unregistration and visibility boundaries without naming `agent_ui` or `git_ui` types. The focused `cargo test -p workspace collaborative_review_host -- --nocapture` passed 1/1 and the full workspace suite passed 238/238; `./script/clippy -p workspace` passed with all targets, all features and denied warnings. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.6. Adapt AgentDiffPane to the collaborative review host
    - Expose the existing agent diff surface through the workspace contract while retaining its canonical ACP thread, ActionLog and project state.
    - _Requirements: 4.1, 10.4_
    - _Capability IDs: CAP-020, CAP-021, CAP-036_
    - _Depends on: 9.1_
    - _Reads: crates/agent_ui/src/{agent_diff,agent_ui}.rs, crates/workspace/src/collaborative_review.rs_
    - _Writes: crates/agent_ui/src/collaborative_review.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui collaborative_agent_review_adapter` proves the host uses the existing thread/ActionLog diff and reports unavailable or stale state_
    - _Evidence: 2026-08-14 — added `CollaborativeAgentReviewAdapter`, which accepts only an active canonical `AcpThread`, verifies the workspace's exact `Project` entity, retains the thread's existing `ActionLog` identity, constructs the existing native `AgentDiffPane`, and registers that pane in the workspace-owned agent-change slot without a second diff or transcript model. Missing-thread, canonical registration, selected-view identity and cross-project stale-state scenarios are covered. `cargo test -p agent_ui collaborative_agent_review_adapter -- --nocapture` passed 1/1 unit tests with the unrelated integration target correctly filtering all three tests; `./script/clippy -p agent_ui` passed in release mode with all targets, all features and denied warnings after obtaining the pinned WebRTC artifact. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.7. Adapt ProjectDiff to the collaborative review host
    - Expose the existing project diff surface through the workspace contract while retaining its canonical Project and GitStore state.
    - _Requirements: 4.1, 10.4_
    - _Capability IDs: CAP-020, CAP-036_
    - _Depends on: 9.1_
    - _Reads: crates/git_ui/src/{git_ui,project_diff}.rs, crates/workspace/src/collaborative_review.rs_
    - _Writes: crates/git_ui/src/collaborative_review.rs, crates/git_ui/src/git_ui.rs_
    - _Validation: `cargo test -p git_ui collaborative_project_review_adapter` proves file/diff state comes from the existing Project/GitStore and reports unavailable or stale state_
    - _Evidence: 2026-08-14 — added `CollaborativeProjectReviewAdapter`, which discovers the workspace's already-open native `ProjectDiff`, retains the exact canonical `Project` and current `GitStore` identities, and registers that same view in the workspace-owned project-change slot without creating fallback project, repository or diff state. The focused test covers explicit unavailable state before `ProjectDiff` is opened, selected-view identity after registration and fail-closed cross-project reuse; current Git-store identity is revalidated at registration. `cargo test -p git_ui collaborative_project_review_adapter -- --nocapture` passed 1/1 and the complete `git_ui` suite passed 132/132 plus doc tests. `./script/clippy -p git_ui` passed in release mode with all targets, all features and denied warnings after obtaining the pinned WebRTC artifact. Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.8. Register and mount native review adapters
    - Register the agent and project review adapters from the upper Zed composition and mount the selected native surface in the collaborative shell without a fallback copy.
    - _Requirements: 4.1, 4.2, 10.4_
    - _Capability IDs: CAP-020, CAP-036_
    - _Depends on: 9.6, 9.7_
    - _Reads: crates/agent_ui/src/collaborative_review.rs, crates/git_ui/src/collaborative_review.rs, crates/workspace/src/{collaborative_layout,collaborative_review,collaborative_workspace,workspace}.rs, crates/zed/src/{main,zed}.rs_
    - _Writes: crates/zed/src/zed.rs, crates/workspace/src/{collaborative_layout,collaborative_workspace,workspace}.rs_
    - _Validation: `cargo test -p zed collaborative_review_registration` verifies AgentDiffPane/ProjectDiff selection, shared project identity, resizable mount and collapsed full-width timeline_
    - _Discovered contradiction (2026-08-14): the approved design assigns shell composition to `workspace`, but the original Zed-only write set could register providers without replacing `CollaborativeLayout`'s placeholder review region. The dependency-safe resolution retains upper adapter registration in `zed` while expanding the leaf to the three existing workspace-owned presentation files needed to mount the host's selected `AnyView`; it adds no dependency from `workspace` to `agent_ui` or `git_ui` and does not change canonical ownership._
    - _Evidence: 2026-08-14 — upper Zed composition now observes the existing native `ProjectDiff` and active `AgentPanel` ACP thread, constructs their approved adapters outside the originating `Workspace` update lease, and replaces registrations through scoped host tokens. The workspace shell mounts the host-selected native `AnyView` without introducing `agent_ui`/`git_ui` dependencies or fallback diff state; selection preserves exact view identity, while collapse retains the provider and gives the timeline the full layout width. `cargo test -p zed collaborative_review_registration -- --nocapture`, `cargo test -p workspace collaborative_review_host -- --nocapture`, and `cargo test -p workspace collaborative_layout_bounds -- --nocapture` each passed 1/1. `./script/clippy -p workspace` and `./script/clippy -p zed` passed in release mode with all targets, all features and denied warnings; Zed lint required network access only to retrieve the pinned WebRTC archive. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.2. Add stable timeline-to-change links
    - Resolve activity action/change IDs to repository, file and hunk targets and surface stale targets.
    - _Requirements: 10.3, 10.4_
    - _Capability IDs: CAP-020, CAP-025_
    - _Depends on: 9.8_
    - _Reads: crates/action_log/src/**, crates/workspace/src/collaborative_review.rs, crates/git_ui/src/project_diff.rs_
    - _Writes: crates/agent_ui/src/activity_diff_link.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui activity_diff_link` covers valid, moved, stale and missing hunks_
    - _Discovered contradiction (2026-08-14): the planned write set named the new module but omitted the crate-root declaration required to compile it. The dependency-safe correction adds only the one-line `agent_ui.rs` module export and does not expand behavior, ownership or milestone scope._
    - _Evidence: 2026-08-14 — added an ephemeral `ActivityDiffLinkResolver` and current-target index keyed by exact `ActionLog`/`ProjectDiff` entity identities. Stable action and Git-change links bind to opaque repository/change/file/hunk IDs, while current native `ProjectPath` and hunk ranges remain projections from the canonical review state. Resolution follows a moved path only when stable file/hunk identity survives and fails closed for unsupported or empty links, duplicate/mismatched bindings, replaced sources, stale changes/files and missing hunks. `cargo test -p agent_ui activity_diff_link -- --nocapture` passed 1/1 with the unrelated integration target filtering its three tests; the complete `cargo test -p agent_ui` suite then passed 418/418 unit tests and 3/3 collaborative-activity integration tests. `./script/clippy -p agent_ui` passed in release mode with all targets, all features and denied warnings. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.3. Expose review file navigation and aggregate statistics
    - Reuse native file selection and addition/deletion totals in top and status surfaces.
    - _Requirements: 10.4_
    - _Capability IDs: CAP-020, CAP-036_
    - _Depends on: 9.8_
    - _Reads: crates/git_ui/src/project_diff.rs, crates/workspace/src/collaborative_review.rs_
    - _Writes: crates/workspace/src/collaborative_review_summary.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_review_summary` checks file changes and zero/stale states_
    - _Discovered contradiction (2026-08-14): the original write set omitted the workspace crate-root declaration required to compile the new summary module. The correction adds only that module export; later exact `workspace.rs` writers are serialized through Task 10.2's dependency on 9.3._
    - _Evidence: 2026-08-14 — added a workspace-owned `CollaborativeReviewSummary` projection keyed by the selected native review slot, exact provider entity and monotonic revision. It exposes opaque file identities, current canonical `ProjectPath` navigation targets, selected file and aggregate additions/deletions without owning Git or diff mutation. Construction rejects empty/duplicate/missing selections; navigation and selection fail closed for replaced providers, stale revisions or removed files; replacement accepts only a strictly newer projection from the same provider, including an explicit zero-change state. `cargo test -p workspace collaborative_review_summary -- --nocapture` passed 1/1, and the complete `cargo test -p workspace` suite passed 239/239. `./script/clippy -p workspace` passed in release mode with all targets, all features and denied warnings. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.4. Route valid keep, reject, stage and review actions
    - Invoke existing native actions only when their source and Git state permit them and surface failures.
    - _Requirements: 10.4_
    - _Capability IDs: CAP-020_
    - _Depends on: 9.2, 9.3, 9.8_
    - _Reads: crates/agent_ui/src/agent_diff.rs, crates/git_ui/src/project_diff.rs_
    - _Writes: crates/workspace/src/collaborative_review_actions.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_review_actions` covers valid, conflict, rejected and stale transitions_
    - _Discovered contradiction (2026-08-14): the original write set omitted the workspace crate-root declaration required to compile the new action-routing module. The correction adds only that module export; Task 9.3 is an explicit predecessor for its earlier crate-root write, and later exact `workspace.rs` work is serialized through Task 10.2's dependency on 9.4._
    - _Evidence: 2026-08-14 — added a workspace-owned authorization router for keep/reject on native agent changes and stage/review on native project changes. It requires the exact provider entity and revision, checks current conflict/rejected/stale state and provider-advertised capabilities, and invokes a caller-supplied native action only after every check succeeds; invalid slot, unavailable action and native failure remain explicit without duplicating `AgentDiffPane` or `ProjectDiff` mutations. Rejected requests are verified not to invoke their native closure. `cargo test -p workspace collaborative_review_actions -- --nocapture` passed 1/1 after the final stale-revision and unavailable-action cases, the complete workspace suite passed 240/240, and `./script/clippy -p workspace` passed in release mode with all targets, all features and denied warnings. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 9.5. Add review-pane regression scenarios
    - Exercise pane collapse/restore, file navigation, action links and canonical Git updates together.
    - _Requirements: 4.2, 10.3, 10.4, 20.1_
    - _Capability IDs: CAP-020, CAP-036, CAP-044_
    - _Depends on: 9.2, 9.3, 9.4_
    - _Reads: crates/workspace/src/collaborative_review*.rs, crates/{fs,git,project}/src/**_
    - _Writes: crates/workspace/Cargo.toml, crates/workspace/src/workspace.rs, crates/workspace/tests/collaborative_review.rs_
    - _Validation: `cargo test -p workspace collaborative_review` passes against a temporary repository fixture_
    - _Discovered contradiction (2026-08-14): adding the first auto-discovered workspace integration test compiled the normal workspace library while dev-dependency feature unification exposed test-only remote identity variants, so the original single-file write set could not pass its default validation command. The narrow correction registers the external target behind `workspace/test-support` for all-feature validation and includes the same source in the default library test build through a test-only crate alias/module. This adds only `Cargo.toml` test metadata and `workspace.rs` test registration; production behavior and milestone scope are unchanged, and Task 10.2 is sequenced after 9.5 for the repeated crate-root write._
    - _Evidence: 2026-08-14 — added a deterministic fake Git repository regression that binds the exact canonical `Project` and native review-provider entity, verifies collapse/restore preserves that provider, resolves and selects stable file targets, and routes stage through the guarded native callback. The callback updates the canonical repository index; the observed `Project` repository status transitions from unstaged to staged, a newer zero-change summary replaces the projection, and the pre-refresh action token fails stale without invocation. `cargo test -p workspace collaborative_review -- --nocapture` passed all 4 matching host/summary/action/regression tests, and the complete workspace suite passed 241/241. `./script/clippy -p workspace` passed in release mode with all targets, all features and denied warnings. Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

- [ ] 10. Finish native composer, status, accessibility and visual coverage

  - [x] 10.1. Mount the native collaborative composer
    - Reuse the existing message/prompt editor and bind submit/cancel to the active ACP thread.
    - _Requirements: 4.1, 11.1_
    - _Capability IDs: CAP-021, CAP-036_
    - _Depends on: 8.5, 9.5_
    - _Reads: crates/agent_ui/src/{agent_panel,conversation_view,message_editor}.rs, crates/workspace/src/{collaborative_workspace,workspace}.rs, crates/zed/src/zed.rs_
    - _Writes: crates/workspace/src/{collaborative_composer,collaborative_workspace,workspace}.rs, crates/agent_ui/src/{collaborative_composer,agent_ui}.rs, crates/zed/src/zed.rs_
    - _Validation: `cargo test -p workspace collaborative_composer` covers send, empty input, cancellation and unavailable thread_
    - _Discovered contradiction (2026-08-14): the original workspace-only write boundary could mount a surface but could not bind the existing `agent_ui::MessageEditor` without reversing the established `workspace` → `agent_ui` dependency prohibition. The narrow correction keeps the project-bound host and unavailable surface in `workspace`, adds a concrete adapter beside the canonical editor in `agent_ui`, and reconciles the active provider from the existing upper `zed` composition. This is one coherent native mount: it reuses the same editor entity and send/cancel events and creates no prompt, transcript or session authority. Task 10.2 now follows 10.1 to serialize their repeated `workspace.rs` write; its later adapter and upper-registration leaves already depend transitively on 10.2, so the `agent_ui.rs` and `zed.rs` writes remain ordered._
    - _Evidence: 2026-08-14 — added a generation-safe, exact-project composer host and native bottom surface; adapted the active ACP thread's existing `MessageEditor`; routed submit/cancel through its existing events; and reconciled registration on active-thread changes without copying prompt or session state. The focused workspace test passed all unavailable, project-mismatch, occupied-provider, empty-input, submit, cancel and stale-unregistration scenarios. `cargo check -p agent_ui -p zed --tests`, `./script/clippy -p workspace`, `./script/clippy -p agent_ui` and `./script/clippy -p zed` passed. Final regression-suite and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 10.2. Define participant and execution status view data
    - Define workspace-owned human/agent participant and execution-status values plus a provider contract without importing ACP UI metadata.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-007, CAP-021, CAP-036_
    - _Depends on: 6.2, 7.4, 9.1, 9.5, 10.1_
    - _Reads: crates/client/src/user.rs, crates/workspace/src/{collaborative_top_bar,status_bar,workspace}.rs_
    - _Writes: crates/workspace/src/collaborative_participants.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_participant_view_data` checks stable human/agent identity, unknown values, model/runtime/location labels and provider absence_
    - _Decomposition resolution (2026-08-14): approved split keeps dependency-safe view data in `workspace`; Task 10.8 adapts canonical `agent_ui` metadata and Task 10.9 mounts the provider into the top/status surfaces from the upper composition._
    - _Discovered sequencing resolution (2026-08-14): Tasks 9.3 through 10.1 required successive workspace crate-root writes. Depending on 10.1, which transitively follows 9.3 through 9.5, serializes all registrations without changing Task 10.2 behavior or milestone scope._
    - _Evidence: 2026-08-14 — added workspace-owned stable human/agent identities, bounded presence, execution phase, safe model/runtime/location labels and explicit ready/failed/unavailable provider states. The exact-project single-provider host uses generation-bearing tokens so stale updates or removals cannot replace current display data, while the projection owns no ACP metadata, presence or session persistence. `cargo test -p workspace collaborative_participant_view_data -- --nocapture` passed stable identity, unknown/known labels, provider absence, project mismatch, occupied provider, failure normalization, replacement and stale-token scenarios. `./script/clippy -p workspace`, Rust 2024 formatting, `git diff --check`, inventory and specification validation passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 10.8. Adapt ACP metadata to participant and execution status
    - Project the active canonical thread's agent, model, runtime and execution location into workspace view data without storing a second session record.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-007, CAP-021, CAP-036_
    - _Depends on: 8.5, 9.2, 9.6, 10.2_
    - _Reads: crates/agent_ui/src/{agent_panel,agent_ui,thread_metadata_store}.rs, crates/agent_ui/src/conversation_view/thread_view.rs, crates/workspace/src/collaborative_participants.rs_
    - _Writes: crates/agent_ui/src/collaborative_participants.rs, crates/agent_ui/src/agent_ui.rs_
    - _Validation: `cargo test -p agent_ui collaborative_participant_adapter` covers active/changed/missing threads, stable avatars, unknown model and local/remote runtime labels_
    - _Discovered contradiction (2026-08-14): Task 9.2's required crate-root export introduced a repeated `agent_ui.rs` write not present in the original decomposition. Adding 9.2 as an explicit predecessor serializes that one-line module registration without changing Task 10.8 behavior or milestone scope._
    - _Evidence: 2026-08-14 — added an exact-project `agent_ui` adapter that retains the active `ThreadView` entity ID and projects its canonical agent ID/display/HTTP avatar, current model, native-Zed-versus-ACP runtime, execution phase and existing metadata's local/named-remote/unknown location into Task 10.2 view data. Missing threads and projects or cross-project reuse fail explicitly; missing model and location remain safe unknown values; the adapter owns no session or metadata persistence. `cargo test -p agent_ui collaborative_participant_adapter -- --nocapture` passed missing, active and changed thread snapshots, stable identity/avatar, unknown and selected models, and local/remote labels. `./script/clippy -p agent_ui`, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 10.9. Mount participant and execution status surfaces
    - Render workspace participant/execution providers in the existing collaborative top bar and status bar with unavailable/failure states.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-007, CAP-021, CAP-036_
    - _Depends on: 10.2_
    - _Reads: crates/workspace/src/collaborative_participants.rs, crates/workspace/src/collaborative_top_bar.rs, crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/status_bar.rs_
    - _Writes: crates/workspace/src/collaborative_top_bar.rs, crates/workspace/src/collaborative_workspace.rs, crates/workspace/src/status_bar.rs, crates/workspace/src/workspace.rs, crates/workspace/src/workspace_presentation_actions.rs_
    - _Validation: `cargo test -p workspace collaborative_participant_status_mount` uses a fake provider to verify top/status updates, replacement and unavailable/failure states_
    - _Discovered contradiction (2026-08-14): the original three presentation-only writes could render an initial participant snapshot but could not propagate provider register/update/unregister transitions or remove the status projection when returning to the unchanged Editor presentation. The narrow correction adds workspace-owned synchronization beside the canonical provider host and invokes it after presentation transitions. This creates no participant/session authority and leaves Task 10.10 as the sole upper `agent_ui` provider-registration leaf._
    - _Evidence: 2026-08-14 — mounted the workspace-owned projection into native top/status surfaces with bounded participant avatars, execution phase/model/runtime/location labels and explicit ready, failed and unavailable states. Provider registration, updates, replacement, removal and presentation changes synchronize cloned display data from the canonical host; Editor mode receives no participant status chrome. `cargo test -p workspace collaborative_participant_status_mount -- --nocapture` and `cargo test -p workspace collaborative_top_bar -- --nocapture` passed; `./script/clippy -p workspace`, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 10.10. Register the ACP participant provider in Zed
    - Register the dependency-safe `agent_ui` adapter from the upper Zed composition and prove it supplies the workspace surfaces without duplicate participant/session persistence.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-007, CAP-021, CAP-036_
    - _Depends on: 9.8, 10.8, 10.9_
    - _Reads: crates/agent_ui/src/collaborative_participants.rs, crates/workspace/src/collaborative_participants.rs, crates/zed/src/main.rs, crates/zed/src/zed.rs_
    - _Writes: crates/zed/src/zed.rs_
    - _Validation: `cargo test -p zed collaborative_participant_provider_registration` verifies canonical active-thread updates, unknown/unavailable state and one registered provider_
    - _Implementation finding (2026-08-14): Zed's existing `cx.observe_new` workspace-composition hook and AgentPanel subscription are both defined in `zed.rs`; `main.rs` requires no initialization change. Narrowing the write set avoids a second registration path and does not change architecture, ownership or scope._
    - _Evidence: 2026-08-14 — registered Task 10.8's exact-project adapter from the existing upper workspace composition and observed the canonical active `ThreadView`. Same-thread model/runtime/phase/location changes update the sole registration in place, active-thread replacement unregisters the prior generation first, and missing or invalid threads fail closed to unavailable without participant/session persistence. `cargo test -p zed collaborative_participant_provider_registration -- --nocapture` passed unavailable, unknown metadata, occupied-provider rejection, in-place update, replacement, stale-token and cleanup scenarios. `./script/clippy -p zed` passed in release mode with all targets, all features and denied warnings; Rust 2024 formatting passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 10.3. Add project, branch, diff and task status projection
    - Compose canonical project/worktree/branch/diff and task state without new persisted authorities.
    - _Requirements: 4.1, 4.3_
    - _Capability IDs: CAP-018, CAP-020, CAP-036_
    - _Depends on: 7.3, 9.3, 10.9_
    - _Reads: crates/project/src/**, crates/workspace/src/collaborative_participants.rs, crates/workspace/src/status_bar.rs, crates/workspace/src/collaborative_review_summary.rs_
    - _Writes: crates/workspace/src/collaborative_status.rs, crates/workspace/src/status_bar.rs, crates/workspace/src/workspace.rs_
    - _Validation: `cargo test -p workspace collaborative_status` covers missing repo, dirty branch and running/waiting task_
    - _Discovered contradiction (2026-08-14): the original isolated-module write could define the projection but could not deliver the required bottom/status surface or compile without a crate-root declaration. The narrow correction mounts the projection through the existing StatusBar, observes the canonical Project/GitStore and serializes the repeated `workspace.rs` write after Task 10.9. It adds no project, Git, diff or task persistence and does not change approved ownership or milestone scope._
    - _Evidence: 2026-08-14 — added a native status projection over the canonical visible worktree and active repository, including truthful missing-repository and detached-head states, dirty state, branch label and saturating file/addition/deletion totals. A current native review summary overrides repository-derived diff totals when supplied. The active ACP execution phase maps only to a typed task presentation value; idle/unknown remains absent and no task record is stored. The existing StatusBar mounts the component only in Collaborative mode and observes canonical Project/GitStore changes. `cargo test -p workspace collaborative_status -- --nocapture` passed missing-repository, dirty named branch, native review totals and running/waiting scenarios; `cargo test -p workspace collaborative_participant_status_mount -- --nocapture` passed Collaborative mount and Editor isolation. `./script/clippy -p workspace`, Rust 2024 formatting and `git diff --check` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 10.4. Implement keyboard focus order and workspace actions
    - Define focus traversal and shortcuts across rail, timeline, composer, review and status controls.
    - _Requirements: 4.4_
    - _Capability IDs: CAP-036_
    - _Depends on: 7.6, 9.5, 10.1, 10.3_
    - _Reads: crates/workspace/src/{collaborative_composer,collaborative_layout,collaborative_workspace,status_bar,workspace}.rs, crates/gpui/**_
    - _Writes: crates/workspace/src/{collaborative_focus,collaborative_composer,collaborative_layout,collaborative_workspace,status_bar,workspace}.rs_
    - _Validation: `cargo test -p workspace collaborative_focus` verifies logical order, restoration and no focus trap_
    - _Discovered contradiction (2026-08-14): the original isolated focus-model write could define ordering but could not supply actual GPUI targets or route actions across the sidebar, central surface and existing status bar. The narrow correction gives the existing timeline, composer, review and status containers focus handles and coordinates them from `Workspace`, which already receives the canonical sidebar handle. Task 10.4 follows 10.3 to serialize the repeated `workspace.rs` and `status_bar.rs` writes. This adds no presentation persistence or alternate UI authority._
    - _Evidence: 2026-08-14 — added a typed navigation → timeline → composer → conditionally visible review → status order; native forward, reverse and restore actions; presentation-scoped Tab/Shift-Tab routing; last-landmark restoration; and terminal fallback to the window focus chain. Review focus follows actual layout geometry, so narrow-window and user-collapsed review state cannot trap focus. `cargo test -p workspace collaborative_focus -- --nocapture` passed the logical-order and mounted GPUI traversal/restoration scenarios; `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed the Collaborative navigation regression. Final clippy, formatting, inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 10.5. Add accessibility names and state announcements
    - Label navigation, participant, activity, composer, review and failure states and announce meaningful transitions.
    - _Requirements: 4.4_
    - _Capability IDs: CAP-036_
    - _Depends on: 10.4, 10.10_
    - _Reads: crates/workspace/src/{collaborative_*,status_bar,workspace}.rs, crates/agent_ui/src/collaborative_timeline.rs, crates/sidebar/src/collaborative_rail.rs_
    - _Writes: crates/workspace/src/{collaborative_accessibility,collaborative_composer,collaborative_layout,collaborative_shell_state,collaborative_status,collaborative_top_bar,collaborative_workspace,status_bar,workspace}.rs, crates/agent_ui/src/collaborative_timeline.rs, crates/sidebar/src/collaborative_rail.rs_
    - _Validation: GPUI accessibility snapshot contains named landmarks, controls and running/error announcements_
    - _Discovered contradiction (2026-08-14): the original isolated contract-module write could describe labels but could not expose them through the existing GPUI elements or label semantic activity rows owned by `agent_ui`. The narrow correction keeps label/announcement projection in dependency-safe `workspace::collaborative_accessibility`, applies it to the already canonical workspace/sidebar/status surfaces, and adds row semantics beside the existing `agent_ui` activity renderer. This is one cross-surface accessibility behavior, creates no parallel UI or execution state, and follows 10.4 to serialize repeated focus-surface writes._
    - _Evidence: 2026-08-14 — added stable names and AccessKit roles for workspace, top controls, navigation, timeline, composer, review and status landmarks; bounded participant/presence labels; semantic activity row labels and expansion state; task and retry status announcements; and alert semantics for activity, provider and shell failures. The contract snapshot covers all seven required landmarks plus running and failure transitions, while focused timeline coverage proves running/error row output. `cargo test -p workspace collaborative_accessibility -- --nocapture`, `cargo test -p agent_ui collaborative_timeline_accessibility -- --nocapture` and `cargo test -p sidebar collaborative_rail_layout -- --nocapture` passed. `./script/clippy -p workspace`, `./script/clippy -p agent_ui` and `./script/clippy -p sidebar` passed with all targets/features and denied warnings. Final formatting, inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 10.6. Add native viewport visual fixtures
    - Capture expanded and collapsed compositions at the checked-in reference dimensions using theme tokens.
    - _Requirements: 4.2, 4.5_
    - _Capability IDs: CAP-036_
    - _Depends on: 6.6, 7.6, 8.5, 9.5, 10.3, 10.5, 10.10_
    - _Reads: .agents/specs/collaborative-workspace/screenshots/*.png, crates/sidebar/src/{sidebar,sidebar_tests}.rs, crates/workspace/src/collaborative_*.rs_
    - _Writes: Cargo.lock, crates/workspace/tests/visual/collaborative_workspace.rs, crates/workspace/tests/fixtures/collaborative_workspace/visual-contract.json, crates/sidebar/src/sidebar_tests.rs, crates/sidebar/Cargo.toml_
    - _Validation: `cargo test -p sidebar collaborative_workspace_visual_fixtures -- --nocapture` renders the native expanded composition at 1930×1262 and collapsed composition at 1928×1298 against the explicitly approved reference contract_
    - _Discovered contradiction (2026-08-14): a standalone `workspace` integration target cannot compose the canonical rail because production dependency direction is `sidebar -> workspace`; a reverse dev dependency or copied rail would violate the approved architecture. Zed's existing raster runner also has no checked-in approved baseline for this surface. The narrow correction keeps the fixture and approval contract at the requested workspace paths but includes it in the existing sidebar GPUI test module, where the real Sidebar, MultiWorkspace and CollaborativeWorkspace render together. The two user-provided PNGs remain the approved reference artifacts; their exact dimensions, hashes, required regions and expanded/collapsed state are pinned while theme colors remain native Zed tokens._
    - _Evidence: 2026-08-14 — added a versioned visual contract for both approved screenshot artifacts and an exact-viewport GPUI fixture over the real native composition. The expanded case requires rail, top bar, timeline, review, composer and status regions with a bounded native review pane; the collapsed case requires the review region to be absent and the timeline to fill the layout. The fixture validates PNG identity metadata, viewport geometry, major pane adjacency, vertical hierarchy and native layout toggling without hardcoding light-theme pixels or introducing another UI owner. `cargo test -p sidebar collaborative_workspace_visual_fixtures -- --nocapture` passed 1/1. Final clippy, regression, formatting, inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 10.7. Add theme, zoom, narrow-window and restart regressions
    - Verify dark/high-contrast themes, reduced motion, zoom, narrow layout and full presentation-state restoration.
    - _Requirements: 3.2, 4.3, 4.4, 4.5, 20.1_
    - _Capability IDs: CAP-036, CAP-037, CAP-044_
    - _Depends on: 5.5, 6.5, 10.5, 10.6_
    - _Reads: crates/workspace/tests/visual/collaborative_workspace.rs, crates/workspace/src/collaborative_*.rs, crates/sidebar/src/collaborative_*.rs, crates/agent_ui/src/collaborative_*.rs, crates/theme/src/{theme,styles/colors}.rs, crates/theme_settings/src/settings.rs_
    - _Writes: crates/workspace/tests/collaborative_workspace.rs, crates/workspace/Cargo.toml_
    - _Validation: `cargo test -p workspace --features test-support --test collaborative_workspace -- --nocapture --test-threads=1` passes theme/zoom, narrow-window, restart, reduced-motion and theme-token regressions_
    - _Discovered validation correction (2026-08-14): the original command omitted the package's `test-support` feature even though the dedicated GPUI integration target consumes `AppState::test` and other existing test-support APIs. The default all-target Cargo graph also unifies `remote/test-support` without `workspace/test-support`, exposing an unrelated pre-existing `Mock` exhaustiveness mismatch. The explicit target is therefore declared with the same required feature as `collaborative_review` and validated with that feature; production identity, persistence and feature behavior are unchanged._
    - _Evidence: 2026-08-14 — added a three-scenario integration target. The display scenario installs a sentinel high-contrast `GlobalTheme`, increases native UI rem size, verifies all collaborative landmarks, collapses review automatically at 760×640 and restores it at 1400×900. The restart scenario uses one `WorkspaceId`, resizes and collapses review, selects two thread targets and navigates backward, flushes canonical settings/KVP state, replaces the root workspace and verifies presentation, Project identity, exact review width, active target and forward history. The static reduced-motion/theme contract rejects animation/timer APIs and literal color constructors across native collaborative workspace, sidebar and agent UI sources. `cargo test -p workspace --features test-support --test collaborative_workspace -- --nocapture --test-threads=1` passed 3/3; `./script/clippy -p workspace` passed all targets/features with denied warnings. Final formatting, inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

## Milestone 2 — establish canonical protocol, identity and service foundations

- [x] 11. Implement the UI-free collaboration domain and Nostr codecs

  - [x] 11.1. Define collaboration aggregate identifiers and provenance
    - Add tenant-scoped stable IDs, versions and provenance fields without I/O or GPUI dependencies.
    - _Requirements: 2.1, 2.2, 5.1_
    - _Capability IDs: CAP-001, CAP-003, CAP-005_
    - _Depends on: 2.1, 3.1_
    - _Reads: projects/buzz/crates/buzz-core/src/{event,tenant}.rs, crates/proto/**_
    - _Writes: Cargo.toml, Cargo.lock, crates/collaboration_domain/Cargo.toml, crates/collaboration_domain/src/{collaboration_domain,identity_types,provenance}.rs_
    - _Validation: `cargo test -p collaboration_domain provenance` verifies stable tenant-scoped identity and version ordering_
    - _Implementation finding (2026-08-14): the planned source modules require a crate manifest, descriptive library root and root-workspace membership before the focused package validation can compile. These are the minimal build integration paths for the approved UI/I/O-free `collaboration_domain` owner; no existing crate provides that dependency direction, and no service, protocol, persistence or GPUI dependency was introduced._
    - _Evidence: 2026-08-14 — added the UI/I/O-free `collaboration_domain` crate with opaque community, aggregate, operation and principal UUID identifiers; an explicit nine-class aggregate type; and a `ScopedAggregateId` whose equality/order always includes community, type and raw aggregate UUID. Deterministic UUIDv5 import construction preserves a stable source mapping without treating it as tenant authority. Provenance adds strictly positive overflow-safe ordered versions, a bounded source-record ID whose deserializer enforces the same 1–1024-byte invariant, closed source/integrity kinds, observation time and optional source-version/integrity fields. Five focused tests prove cross-community identity separation, deterministic source mapping, strict successor ordering/overflow, bounded construction/deserialization and lossless provenance serialization. `cargo test -p collaboration_domain provenance -- --nocapture` passed 5/5; `./script/clippy -p collaboration_domain`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.2. Port canonical event serialization and identifiers
    - Implement exact canonical JSON, event-ID and signature-input encoding behind the compatibility boundary.
    - _Requirements: 5.1, 5.4_
    - _Capability IDs: CAP-001_
    - _Depends on: 11.1_
    - _Reads: projects/buzz/crates/buzz-core/src/event.rs, .agents/specs/collaborative-workspace/fixtures/protocol/**_
    - _Writes: Cargo.toml, Cargo.lock, crates/nostr_compat/Cargo.toml, crates/nostr_compat/src/{nostr_compat,event}.rs_
    - _Validation: `cargo test -p nostr_compat event_vectors` matches frozen byte and ID fixtures_
    - _Implementation finding (2026-08-14): the first compatibility module requires a crate manifest, descriptive library root and workspace registration before its exact fixture test can compile. These are the minimal integration paths for the approved protocol-adapter boundary; the crate has no dependency on key custody, authorization, persistence, services or GPUI._
    - _Evidence: 2026-08-14 — added the UI/service-free `nostr_compat` crate and an exact NIP-01 signature-input encoder for `[0,pubkey,created_at,kind,tags,content]` using compact UTF-8 JSON plus SHA-256 event identifiers. Public key and event-ID types accept only exact-length lowercase hexadecimal text on constructors and deserialization; timestamps and kinds are bounded by `u64`/`u16`, while tags preserve ordered string arrays and content preserves Unicode and JSON escaping. Four focused tests match all 12 structurally hashable frozen event IDs, pin the legacy event's exact preimage bytes, reproduce the frozen tampered-content ID mismatch, prove Unicode/escape behavior and reject uppercase, invalid and wrong-length identifier encodings. Signature and curve validity remain intentionally owned by Task 11.3. `cargo test -p nostr_compat event_vectors -- --nocapture` passed 4/4; `./script/clippy -p nostr_compat`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.3. Port signing and verification rules
    - Verify Schnorr signatures, event IDs, timestamps and malformed inputs without accessing key storage.
    - _Requirements: 5.1, 5.4, 19.2_
    - _Capability IDs: CAP-001, CAP-009_
    - _Depends on: 11.2_
    - _Reads: projects/buzz/crates/buzz-core/src/verification.rs, crates/nostr_compat/src/event.rs, .agents/specs/collaborative-workspace/fixtures/protocol/events.json_
    - _Writes: Cargo.toml, Cargo.lock, crates/nostr_compat/Cargo.toml, crates/nostr_compat/src/{nostr_compat,verification}.rs_
    - _Validation: `cargo test -p nostr_compat verification` covers valid, altered, oversized and invalid-key fixtures_
    - _Implementation finding (2026-08-14): exact Buzz-compatible BIP-340 verification requires the same audited `secp256k1` 0.29 line used by Buzz's Nostr dependency, registered once in root workspace dependencies and consumed only by `nostr_compat`. The public module/root and crate manifest must expose that boundary. This does not add signing, key custody, authorization, persistence, async runtime or I/O to the adapter._
    - _Evidence: 2026-08-14 — added strict 128-character lowercase signature encoding, a pure `SignedEvent` verification boundary and explicit `TimestampPolicy::{Historical, Bounded}` so frozen/imported history is not rejected by a live freshness rule. Verification enforces the 256 KiB content and 512 KiB canonical-preimage ceilings before cryptography, applies saturating timestamp windows, compares the recomputed SHA-256 event ID, parses the x-only public key and verifies the BIP-340 Schnorr signature over the exact 32-byte ID. Five focused tests accept all ten frozen valid signatures and reject the frozen tampered ID, zero signature and invalid curve key; they also cover the 900-second bounded timestamp edge, oversized content before crypto and malformed signature constructor/deserialization. The complete nine-test `nostr_compat` suite preserves Task 11.2 vectors. `cargo test -p nostr_compat verification -- --nocapture` passed 5/5, `cargo test -p nostr_compat -- --nocapture` passed 9/9 plus doc tests, and `./script/clippy -p nostr_compat`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.4. Port filter and replaceable-head semantics
    - Implement bounded filters and exact replaceable/addressable selection rules as pure functions.
    - _Requirements: 5.1, 5.4, 8.4_
    - _Capability IDs: CAP-001, CAP-002_
    - _Depends on: 11.2_
    - _Reads: projects/buzz/crates/buzz-core/src/filter.rs, projects/buzz/crates/buzz-core/src/kind.rs_
    - _Writes: Cargo.lock, crates/nostr_compat/Cargo.toml, crates/nostr_compat/src/{nostr_compat,filter,head}.rs_
    - _Validation: property tests match Buzz selection for permutations, ties, deletes and invalid limits_
    - _Implementation finding (2026-08-15): permutation coverage requires the existing workspace-pinned `proptest` package as a test-only dependency, which updates the crate manifest and lockfile; the library root must expose both pure modules. Task 11.4 is directly sequenced after Task 11.2's event types and Task 11.3's lockfile write, so no overlapping source or dependency mutation is concurrent._
    - _Evidence: 2026-08-15 — added bounded NIP-01 filter types and pure matching with at most ten OR-ed filters; exact author/kind, inclusive time, canonical lowercase ID-prefix and AND-ed generic-tag behavior; 1,024-value/64-tag/1,024-byte tag-value ceilings; and Buzz's `#h` stored-channel fallback only when no explicit signed `h` tag exists. Added regular, NIP-16 replaceable, ephemeral and NIP-33 parameterized persistence classification; exact replacement coordinates with one required `d` tag for NIP-33; mixed-coordinate rejection; greatest-timestamp/lowest-ID head selection; and an owned deletion/tombstone order floor that blocks stale resurrection while accepting a truly newer value. Three filter tests cover AND/OR/prefix/time/tag semantics, strict `#h` precedence and invalid/excessive limits. Four head tests match both frozen timestamp/tie vectors, reject missing/duplicate discriminators, prove delete-floor behavior and run 256 property cases showing selection is input-order invariant across arbitrary timestamps and IDs. `cargo test -p nostr_compat filter -- --nocapture` passed 3/3, `cargo test -p nostr_compat head -- --nocapture` passed 4/4, and `./script/clippy -p nostr_compat`, Rust 2024 formatting, inventory validation, `git diff --check` and `validate_spec.py` passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.5. Generate the standard and Buzz kind registry
    - Generate typed kind metadata, persistence class, privacy gate and replacement behavior from the frozen catalog.
    - _Requirements: 1.2, 5.1, 5.3_
    - _Capability IDs: CAP-001, CAP-044_
    - _Depends on: 1.2, 11.4_
    - _Reads: .agents/specs/collaborative-workspace/catalogs/protocol.csv, projects/buzz/crates/buzz-core/src/kind.rs_
    - _Writes: crates/nostr_compat/src/{nostr_compat,generated_kinds}.rs, .agents/specs/collaborative-workspace/scripts/generate-kind-registry.py_
    - _Validation: `python3 .agents/specs/collaborative-workspace/scripts/generate-kind-registry.py --check --verify-unclassified-guard` fails on an unclassified kind and matches all 133 frozen event-kind constants; the four range-boundary constants remain accounted for by the protocol catalog_
    - _Implementation finding (2026-08-15): deterministic drift enforcement requires a checked generator beside the generated module, and the crate root must expose that module. Buzz's `KIND_MEDIA_UPLOAD` catalog row is intentionally typed as `InternalNotRelayEvent`; flattening it into `Registered` would erase the audited distinction. Privacy is a composable bit set because result-level and recipient gates overlap for agent turn metrics._
    - _Evidence: 2026-08-15 — generated a numeric-sorted registry for all 133 frozen event kinds plus named constants for all four range boundaries, with typed NIP-16/NIP-33/ephemeral persistence, replacement behavior, protocol references, catalog status and the four Buzz privacy-gate families. The generator joins the authoritative protocol CSV to every scalar `u32` constant and privacy array in Buzz, rejects missing/extra/divergent constants, duplicate values, unknown privacy members and unsupported statuses, exercises an injected uncataloged-kind failure, and emits rustfmt-stable source. Three Rust tests prove sorted unique lookup, all range-boundary storage rules and overlapping privacy semantics. Generator drift/guard validation, focused Rust tests, clippy, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.6. Implement membership and identity NIP codecs
    - Add exact parsing and encoding for NIP-AA, NIP-IA and NIP-OA.
    - _Requirements: 5.3, 5.4, 7.1_
    - _Capability IDs: CAP-001, CAP-007, CAP-008_
    - _Depends on: 11.3, 11.5_
    - _Reads: projects/buzz/docs/nips/NIP-{AA,IA,OA}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/nostr_compat.rs, crates/nostr_compat/src/buzz_nips.rs, crates/nostr_compat/src/buzz_nips/identity.rs_
    - _Validation: `cargo test -p nostr_compat buzz_nips::identity -- --nocapture` round-trips identity NIP vectors and rejects malformed membership/attestation/archive tags_
    - _Implementation finding (2026-08-15): native module exposure requires the crate root and a non-`mod.rs` `buzz_nips.rs` namespace in addition to the planned codec file. NIP-OA conditions cannot have one generic evaluator: event provenance evaluates every clause, NIP-AA ignores `kind=` during connection admission, request-borne NIP-IA proof evaluates only time clauses against the request, and published-profile NIP-IA proof evaluates no clauses. The codec exposes these contexts separately and performs no membership or archive-state mutation._
    - _Evidence: 2026-08-15 — added strict canonical parsing and encoding for NIP-OA owner attestations, NIP-AA agent authentication presentations, and all five NIP-IA request/delta/snapshot wire shapes. Lowercase fixed-width keys/signatures, canonical decimal clauses, exact attestation preimages, self-attestation rejection, context-specific condition evaluation, one protected marker/target, 64-byte reason and 64-KiB content bounds, archive-only replacement pointers, consent actors, unmarked request references and marked profile-proof references are enforced. Five focused tests cryptographically verify and round-trip the published OA vector; round-trip AA and reject duplicate credentials; reproduce the published IA 9035, 8002 and 13535 event IDs; and reject missing/duplicate/malformed and action-incompatible tags. `cargo test -p nostr_compat buzz_nips::identity -- --nocapture`, `./script/clippy -p nostr_compat`, Rust formatting, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.7. Implement persona and managed-agent NIP codecs
    - Add exact parsing and encoding for NIP-AP and NIP-PMA.
    - _Requirements: 5.3, 5.4, 11.2_
    - _Capability IDs: CAP-001, CAP-023_
    - _Depends on: 11.3, 11.5, 11.6_
    - _Reads: projects/buzz/docs/nips/NIP-{AP,PMA}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: Cargo.lock, crates/nostr_compat/Cargo.toml, crates/nostr_compat/src/buzz_nips.rs, crates/nostr_compat/src/buzz_nips/agent_config.rs_
    - _Validation: `cargo test -p nostr_compat buzz_nips::agent_config -- --nocapture` covers versions, CAS predecessors, privacy gates and malformed projections_
    - _Implementation finding (2026-08-15): strict PMA RFC3339 validation requires the existing workspace `chrono` dependency, which updates the crate manifest and its lockfile dependency edge; the non-`mod.rs` NIP namespace must expose the codec. The dependency list now includes Task 11.6 because this leaf reuses its OA verifier and serializes the shared namespace file. Task 11.7 owns exact AP/PMA wire and decrypted-payload validation only: it leaves NIP-44 encryption/decryption, key custody, persistence and transactional aggregate submission outside this compatibility crate, and keeps `PRIVATE_MANAGED_AGENT_INGEST_ENABLED` false until the ordered PMA deployment gates are implemented._
    - _Evidence: 2026-08-15 — added NIP-AP codecs for bounded persona definitions, exact persona/team coordinates, absent-or-exact shared tags, recursive team secret-field rejection and both slim and legacy-fat kind-30177 projections. Added an inert NIP-PMA signed-envelope codec with owner verification, ciphertext bounds, exact permitted tags, canonical positive safe generations, mandatory generation-two predecessor and lifecycle state; strict duplicate/unknown-field JSON decoding for decrypted v1 active/tombstone payloads; namespaced extension and portable configuration bounds; checksum-validated nsec-to-agent derivation; unconditional OA owner proof; and complete signed 30175/30177 recovery binding verification by ID, signature, author, kind, coordinate and content hash. Five focused tests cover AP/team privacy failure, slim/fat compatibility, signed CAS envelopes with ingest disabled, supported/unsupported payload versions plus duplicate keys and tombstones, and malformed projection coordinates/content bindings. `cargo test -p nostr_compat buzz_nips::agent_config -- --nocapture`, `./script/clippy -p nostr_compat`, Rust formatting, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.8. Implement agent activity and memory NIP codecs
    - Add exact parsing and encoding for NIP-AE, NIP-AM and NIP-AO.
    - _Requirements: 5.3, 5.4, 11.3, 12.1_
    - _Capability IDs: CAP-001, CAP-024, CAP-025_
    - _Depends on: 11.3, 11.5, 11.7_
    - _Reads: projects/buzz/docs/nips/NIP-{AE,AM,AO}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips.rs, crates/nostr_compat/src/buzz_nips/agent_activity.rs_
    - _Validation: `cargo test -p nostr_compat buzz_nips::agent_activity -- --nocapture` covers encrypted agent coordinates, versions, observer frames and privacy failures_
    - _Implementation finding (2026-08-15): the codec namespace write is serialized after Task 11.7 and its existing `chrono` dependency is reused for RFC3339 payload validation. This leaf derives the exact NIP-44 v2 conversation key and validates already-decrypted payload bytes but deliberately does not implement NIP-44 content encryption/decryption; Task 30.1 remains the approved encryption/key/zeroization boundary. Unknown NIP-AO frames remain structurally valid but are typed as ignored, matching the forward-compatible relay/client contract._
    - _Evidence: 2026-08-15 — added NIP-AE core/memory slug grammar, strict duplicate-key body parsing, tombstones, raw-x ECDH plus `nip44-v2` HKDF-extract conversation keys, versioned HMAC-blinded coordinates and signed agent/owner encrypted-envelope validation before body-coordinate matching. Added owner-only NIP-AM envelopes with no channel leakage and decrypted turn-metric parsing that preserves null/zero semantics, rejects explicit-null cache/pricing fields, requires cumulative session/sequence, bounds costs, restricts billing authorities and tolerates future stop reasons. Added ephemeral NIP-AO telemetry/control envelopes with exact direction tags, optional channel scope, recipient-only visibility, forward-compatible ignored types, bounded strict payloads and redacted debug output. Five focused tests reproduce the published symmetric conversation key, blinded `mem/example` coordinate and signed encrypted event; reject duplicate body keys and mismatched coordinates; exercise metric privacy/null rules; exercise telemetry/control direction and debug redaction; and ignore unknown observer frames while rejecting malformed control routing. `cargo test -p nostr_compat buzz_nips::agent_activity -- --nocapture`, `./script/clippy -p nostr_compat`, Rust formatting, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.9. Implement communication-state NIP codecs
    - Add exact parsing and encoding for NIP-CW, NIP-DV, NIP-ER and NIP-RS.
    - _Requirements: 5.3, 5.4, 9.1, 9.2, 9.3_
    - _Capability IDs: CAP-001, CAP-011, CAP-012, CAP-013_
    - _Depends on: 11.3, 11.5, 11.8_
    - _Reads: projects/buzz/docs/nips/NIP-{CW,DV,ER,RS}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: crates/nostr_compat/src/buzz_nips.rs, crates/nostr_compat/src/buzz_nips/communication.rs_
    - _Validation: communication vectors cover cursors, wraps, reminders, read frontiers and malformed tags_
    - _Implementation finding (2026-08-15): the shared codec namespace is serialized after Task 11.8. This leaf owns only wire/filter parsing, deterministic encoding and pure read-state register operations. Relay-side channel-window queries and overlays, reminder scheduling, NIP-RS full-state enumeration/barriers, persistence, delivery policy and NIP-44 encryption/decryption remain with their approved later owners; accepting ciphertext here never grants authority or makes decrypted state durable._
    - _Evidence: 2026-08-15 — added exact NIP-CW opt-in filter parsing, composite cursors, row-budget clamping, deterministic request bindings and signature/relay-identity/tag/content validation for thread-summary and window-bounds overlays. Added relay-signed, owner-result-gated NIP-DV snapshots whose hidden channels remain set-valued presentation state. Added signed author-only NIP-ER envelopes with canonical safe-integer schedules, expiration ordering, opaque NIP-44 bounds and strict decrypted target/status/note validation. Added NIP-RS stable-coordinate envelopes, forward-compatible version handling, last-value duplicate contexts, namespace escaping, complete live/tombstone override groups, primary-coordinate confinement, max-register merging, hierarchical frontier evaluation, clear-wins canonical encoding and checked non-wrapping mark-read/mark-unread counters. Five focused tests cover cursor/request binding and malformed half cursors; DM privacy/set behavior and malformed tags; reminder schedule/privacy/duplicate JSON failures; read-frontier duplicate/escape/partial-group behavior; monotone merges, tombstones, future versions and counter exhaustion. `cargo test -p nostr_compat buzz_nips::communication -- --nocapture`, `./script/clippy -p nostr_compat`, Rust formatting, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.10. Implement project and workflow NIP codecs
    - Add exact parsing and encoding for NIP-GS, NIP-MP and NIP-WP.
    - _Requirements: 5.3, 5.4, 10.1, 10.2, 13.1_
    - _Capability IDs: CAP-001, CAP-018, CAP-019, CAP-027_
    - _Depends on: 11.3, 11.5, 11.9_
    - _Reads: projects/buzz/docs/nips/NIP-{GS,MP,WP}.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: Cargo.lock, crates/nostr_compat/Cargo.toml, crates/nostr_compat/src/buzz_nips.rs, crates/nostr_compat/src/buzz_nips/project_workflow.rs_
    - _Validation: project/workflow vectors cover signed coordinates, versions and malformed cross-references_
    - _Implementation finding (2026-08-15): the shared codec namespace is serialized after Task 11.9, and the existing workspace `base64` dependency is added directly to `nostr_compat` for exact NIP-GS armor handling. This leaf owns signed/canonical representation and verification only. Git CLI/key custody, project collection folding and pagination, workspace-profile role admission/persistence/NIP-11 serving, Git authorization and workflow execution remain with their approved owners; a project membership reference is never an authorization input._
    - _Evidence: 2026-08-15 — added byte-exact NIP-GS three-line LF armor, strict duplicate/unknown-field JSON parsing, canonical field-order reconstruction, standard padded base64, size ceilings, domain-separated timestamp/OA-bound hashes and BIP-340 commit verification with a separately reported NIP-OA result. Added signed NIP-MP project parsing/encoding with exact project identity and metadata cardinality/length bounds, 64-member raw-tag cap, kind-30617 split-on-two-colons coordinates, opaque relay hints, duplicate rejection and listed-by-default visibility. Added signed NIP-WP image-sink validation and explicit/absent clear handling without assuming role authority. Five focused tests reproduce the published NIP-GS hash, armor, commit signature and owner-attestation vectors; reject noncanonical order and duplicate JSON; run all 31 authoritative NIP-MP ingest fixtures; preserve colon-bearing cross-owner coordinates without granting authority; and reject unsafe workspace icon schemes. `cargo test -p nostr_compat buzz_nips::project_workflow -- --nocapture`, `./script/clippy -p nostr_compat`, Rust formatting, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.11. Implement push-lease NIP codec
    - Add exact parsing and encoding for NIP-PL without notification policy or provider behavior.
    - _Requirements: 5.3, 5.4, 9.5_
    - _Capability IDs: CAP-001, CAP-016_
    - _Depends on: 11.3, 11.5, 11.10_
    - _Reads: projects/buzz/docs/nips/NIP-PL.md, projects/buzz/crates/buzz-sdk/**_
    - _Writes: Cargo.lock, crates/nostr_compat/Cargo.toml, crates/nostr_compat/src/buzz_nips.rs, crates/nostr_compat/src/buzz_nips/push_lease.rs_
    - _Validation: push-lease vectors cover generation, capabilities, expiry and malformed encrypted values_
    - _Implementation finding (2026-08-15): the shared codec namespace is serialized after Task 11.10, and the existing workspace `uuid` dependency is added directly to `nostr_compat` for the descriptor-registered lowercase UUID-v4 channel grammar. This leaf owns signed-envelope, decrypted-schema and bounded filter grammar only. NIP-44 decryption/key custody, dual-order replacement watermarks, persistence, match-time authorization, notification policy, fixed wake payloads, gateways, provider behavior and delivery remain with their approved later owners._
    - _Evidence: 2026-08-15 — added a signed author-bound NIP-PL kind-30350 envelope with closed exact public tags, canonical expiration, descriptor size/TTL/skew limits and deterministic encoding. Added strict duplicate/unknown-field v1 parsing; byte-exact canonical-origin confirmation; positive safe generations; complete active and minimal inactive/tombstone schemas; app-profile/transport/endpoint bounds; registered priority classes; bounded positive and subtractive filters; exact authors/self-`#p`/event IDs; descriptor-selected lowercase UUID-v4 channels; eligible/urgent kind checks; and suppression bounds. Five focused tests cover authentication, TTL and public-tag privacy failures; active round trips with ignore/suppression; minimal revocation and generation failure; duplicate/unknown/cross-user rejection; and time-travel, malformed-channel and ineligible-urgency rejection. `cargo test -p nostr_compat buzz_nips::push_lease -- --nocapture`, `./script/clippy -p nostr_compat`, Rust formatting, inventory validation, spec validation and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.12. Add custom NIP catalog conformance
    - Run every custom NIP golden/malformed fixture independently of production reducers.
    - _Requirements: 5.3, 5.4, 20.2_
    - _Capability IDs: CAP-001, CAP-044_
    - _Depends on: 11.6, 11.7, 11.8, 11.9, 11.10, 11.11_
    - _Reads: crates/nostr_compat/src/buzz_nips/**, projects/buzz/docs/nips/NIP-*.md, .agents/specs/collaborative-workspace/fixtures/protocol/**_
    - _Writes: crates/nostr_compat/tests/buzz_nips.rs_
    - _Validation: `cargo test -p nostr_compat buzz_nips` passes every registered custom NIP fixture_
    - _Implementation finding (2026-08-15): the authoritative custom-NIP set consists of 16 named Markdown documents, while their executable golden and malformed vectors are intentionally colocated with the six dependency-safe codec modules from Tasks 11.6 through 11.11. The integration catalog therefore registers the exact document/module/vector relationship and runs under the same `buzz_nips` test filter; it does not reproduce protocol parsing or import any relay/product reducer. The frozen cross-protocol corpus is additionally exercised only through public `nostr_compat` verification, replacement-head and generated privacy-registry APIs._
    - _Evidence: 2026-08-15 — added a closed 16-entry NIP-AA/AE/AM/AO/AP/CW/DV/ER/GS/IA/MP/OA/PL/PMA/RS/WP catalog that requires every source document plus a registered golden and malformed codec vector. Added independent frozen-manifest execution for accepted and malformed signed events, deterministic replacement winners, privacy visibility, simultaneous legacy/v2 kinds, wire references and all four relay artifacts; content hashes fail on fixture drift. `cargo test -p nostr_compat buzz_nips -- --nocapture` passed 30 codec vectors and 4 catalog/corpus tests. The independent Python protocol checker passed 7 event, 2 replacement, 7 privacy, 1 mixed-version, 4 wire and 4 relay cases. `./script/clippy -p nostr_compat`, Rust formatting, inventory validation and specification validation passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 11.13. Enforce the domain dependency boundary
    - Wire manifests and a dependency check so collaboration-domain cannot depend on GPUI, storage or transports.
    - _Requirements: 2.1, 2.4_
    - _Capability IDs: CAP-001, CAP-036_
    - _Depends on: 11.1, 11.12_
    - _Reads: Cargo.toml, crates/collaboration_domain/**, crates/nostr_compat/**_
    - _Writes: crates/collaboration_domain/Cargo.toml, crates/nostr_compat/Cargo.toml, script/check-collaboration-dependencies_
    - _Validation: dependency checker and `cargo check -p collaboration_domain -p nostr_compat` pass_
    - _Implementation finding (2026-08-15): both lower-level crates require an explicit manifest contract rather than only a package-name denylist. Each manifest now declares its architectural layer, exact allowed non-dev direct dependencies and all three forbidden capability classes. The checker treats normal and build edges as production, excludes dev-only fixtures, compares the direct allowlist exactly and then walks Cargo's locked resolved graph for known GPUI, persistence and transport packages. This keeps future dependency additions approval-visible while still detecting a forbidden capability hidden behind an allowed crate feature._
    - _Evidence: 2026-08-15 — added domain/protocol-compatibility boundary metadata to `collaboration_domain` and `nostr_compat` and an executable repository-root-independent checker. It rejects missing/duplicate boundary packages, absent or incomplete metadata, stale or unexpected direct dependencies, and forbidden GPUI/storage/transport packages anywhere in the non-dev closure. `./script/check-collaboration-dependencies` passed against locked Cargo metadata; `cargo check -p collaboration_domain -p nostr_compat`, `./script/clippy -p collaboration_domain`, `./script/clippy -p nostr_compat`, `bash -n script/check-collaboration-dependencies` and Rust formatting passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

- [x] 12. Consolidate identity binding and signing-key custody

  - [x] 12.1. Implement approved account-to-Nostr binding records
    - Add binding creation, verification method, community scope and version state from ADR-002.
    - _Requirements: 7.1, 7.4_
    - _Capability IDs: CAP-007, CAP-008_
    - _Depends on: 2.2, 11.1, 11.3_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-002-identity-binding.md, crates/client/src/user.rs_
    - _Writes: crates/collaboration_domain/src/{account_binding,collaboration_domain}.rs_
    - _Validation: `cargo test -p collaboration_domain account_binding` covers verified, conflicting, revoked and historical bindings_
    - _Discovered contradiction (2026-08-15): the original single-file write set could define the record but Rust cannot compile or test an unregistered module. The narrow correction adds only the crate-root module declaration and public domain exports beside the new file. It introduces no client, protocol, storage, credential or GPUI dependency and preserves the approved lower-layer ownership._
    - _Evidence: 2026-08-15 — added validated, serde-safe ADR-002 records with distinct community, service-account, profile, Nostr-author and binding identifiers; optimistic record and policy versions; bounded verification-evidence references and method; predecessor/successor version links; lifecycle timestamps; actor and audit references; and pending/verified/active/rotated/revoked/archived states. Only active state can sign. Migrated evidence can preserve historical records but cannot authorize a live signer. Record hydration rejects invalid timestamp/state/link combinations, and active-set validation enforces one signer per community/account/profile plus one human account owner per community key while permitting the same key in another community. `cargo test -p collaboration_domain account_binding -- --nocapture` passed verified-evidence, profile/owner conflict, community-fence and revoked/historical scenarios; the full 9-test crate suite, `./script/clippy -p collaboration_domain`, the collaboration dependency checker and Rust formatting passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 12.2. Add human and agent profile domain records
    - Model profiles, status, owner attestations, social lists and archival without conflating account and signing identity.
    - _Requirements: 7.1, 7.4_
    - _Capability IDs: CAP-007, CAP-023_
    - _Depends on: 11.6, 12.1_
    - _Reads: projects/buzz/docs/nips/NIP-{OA,IA}.md, projects/buzz/crates/buzz-db/src/user.rs, projects/buzz/desktop/src-tauri/src/models.rs, projects/buzz/crates/buzz-core/src/{kind,presence}.rs_
    - _Writes: crates/collaboration_domain/src/{profile,collaboration_domain}.rs_
    - _Validation: domain tests preserve historical authorship and reject unattested agent-owner changes_
    - _Discovered contradiction (2026-08-15): the planned `projects/buzz/crates/buzz-core/src/identity.rs` source does not exist, and a standalone Rust module cannot run its tests without crate-root registration. Discovery found the actual legacy profile projection in `buzz-db/src/user.rs`, client surface fields in desktop `models.rs`, and status/social kind semantics in `buzz-core/{presence,kind}.rs`; NIP-OA and NIP-IA remain normative for provenance and archival. The narrow write correction adds only the dependency-free module/export. No approved identity ownership, protocol, persistence or milestone scope changes._
    - _Evidence: 2026-08-15 — added validated human and agent profile records whose mandatory community/profile scope and immutable Nostr author are independent from service accounts and optional owner provenance. Agent owner claims require a bounded matching owner-to-agent attestation; unattested, self or mismatched claims fail. Metadata, NIP-38-style status and typed contact/mute/pin/bookmark/emoji/named-follow lists retain their signed source author and apply bounded, list-specific entry grammar. Relay archive state is separately keyed and relay-authored, validates target/consent/replacement shape, affects only active visibility on that relay and never rewrites the profile author. Profile updates require the same profile, community, author and human/agent kind plus the next optimistic version. `cargo test -p collaboration_domain profile -- --nocapture` passed human/agent authorship, unattested owner, relay-local archive and authored social/status scenarios; the full 13-test crate suite, `./script/clippy -p collaboration_domain`, the collaboration dependency checker, Rust formatting and diff hygiene passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 12.3. Add the identity-binding persistence migration
    - Create versioned tenant-fenced bindings and revocations with no private key columns.
    - _Requirements: 6.1, 7.1, 7.4_
    - _Capability IDs: CAP-005, CAP-007_
    - _Depends on: 12.1_
    - _Reads: crates/collab/src/db/**, crates/collaboration_domain/src/account_binding.rs_
    - _Writes: crates/collab/migrations/20260815000100_collaboration_identity_bindings.{up,down}.sql, crates/collab/tests/identity_binding_migration.rs, crates/collab/tests/integration/db_tests/migrations.rs_
    - _Validation: migration tests cover forward/down paths, tenant fences and absence of secret columns_
    - _Discovered contradiction (2026-08-15): the planned unversioned migration filename is not a SQLx migration and therefore would never be resolved or applied. A reversible timestamped up/down pair is required by the existing migration authority. Resolving that pair also exposed that the integration-test helper attempted to execute reversible down files as forward migrations; it now skips down migrations, matching `sqlx::migrate::Migrator::run`. The repository's baseline migration states that production schema rollout is coordinated through the Cloud repository, so this task creates and validates the migration artifacts but does not apply them to a production deployment. That approval boundary remains unchanged._
    - _Evidence: 2026-08-15 — added a reversible SQLx migration for append-only, version-addressable identity bindings with a unique current record, tenant-composite primary/foreign/index keys, lifecycle and timestamp checks, optimistic policy version, actor/audit references, and restrictive forced row-level security keyed only by trusted `app.community_id`. The schema stores a 32-byte public key and bounded evidence reference but contains no private key, raw challenge, seed or recovery secret. The focused migration test resolves both reversible files through SQLx, verifies forward/down DDL, tenant fences and the forbidden-secret vocabulary; the shared database migration helper now ignores reversible down entries during normal forward setup. No database or deployment was mutated._

  - [x] 12.4. Implement protected signing-key import
    - Import Buzz key identifiers into Zed credentials, verify a signing challenge and retain the source until confirmation.
    - _Requirements: 7.2, 7.3, 17.2_
    - _Capability IDs: CAP-009, CAP-045_
    - _Depends on: 11.3, 12.1_
    - _Reads: crates/credentials_provider/**, crates/zed_credentials_provider/**, projects/buzz/desktop/src-tauri/src/{secret_store,identity_storage}.rs_
    - _Writes: Cargo.{toml,lock}, crates/zed_credentials_provider/Cargo.toml, crates/zed_credentials_provider/src/{zed_credentials_provider,nostr_import}.rs_
    - _Validation: credential tests cover success, corrupt source, unavailable keyring, challenge mismatch and source preservation_
    - _Discovered contradiction (2026-08-15): the planned Buzz source filenames used hyphens, but the actual Rust modules are `secret_store.rs` and `identity_storage.rs`. A standalone importer also cannot compile or be exercised without crate-root registration and its cryptographic/encoding dependencies; the narrow write correction registers the module, adds the already-canonical secp256k1/SHA-256/UUID/zeroization dependencies, and adds `bech32` for NIP-19 `nsec` compatibility. The importer deliberately does not link the Buzz desktop crate or embed its keyring implementation: later migration adapters implement the read-only source trait while Zed's existing `CredentialsProvider` remains the only production destination._
    - _Evidence: 2026-08-15 — added a provider-only production import entry point with a bounded read-only Buzz source adapter, deterministic community/account/profile/public-key credential identifier, five-minute domain-separated challenge, trusted system clock, expected x-only public-key binding, and NSEC/hex/raw compatibility parsing. Source material and decoded buffers are zeroized where owned. Existing matching destinations are idempotent; conflicts are never overwritten. New values are stored as canonical 32-byte secrets, read back through the protected provider, challenged again, and removed if verification fails; source deletion is not exposed. Sanitized errors distinguish corrupt/missing/unavailable source, invalid/expired challenge, protected-store failure, conflict, read-back mismatch and cleanup failure without carrying secret material. `cargo test -p zed_credentials_provider nostr_import --release -- --nocapture` passed all five success, corrupt-source, unavailable-store, mismatch, idempotency and source-preservation scenarios; `./script/clippy -p zed_credentials_provider` and Rust formatting passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 12.5. Implement key generation, rotation and archive transitions
    - Route generation, rotation, revocation and archive through canonical credentials and identity records.
    - _Requirements: 7.2, 7.3, 7.4_
    - _Capability IDs: CAP-007, CAP-009_
    - _Depends on: 12.2, 12.4_
    - _Reads: crates/zed_credentials_provider/src/nostr_import.rs, crates/collaboration_domain/src/profile.rs_
    - _Writes: Cargo.{toml,lock}, crates/zed_credentials_provider/Cargo.toml, crates/zed_credentials_provider/src/{zed_credentials_provider,nostr_import,nostr_lifecycle}.rs_
    - _Validation: lifecycle tests prove old authorship remains, active signing changes and failures never synthesize a key_
    - _Discovered contradiction (2026-08-15): rotating a key by changing the author on the existing `IdentityProfile` would violate Task 12.2's approved immutable-authorship invariant. Rotation therefore preserves the old profile and every signed projection unchanged, creates a distinct successor profile for the new author, and atomically commits the old binding's rotated version with the successor active binding/profile. Agent successors begin without inherited owner provenance because an attestation for the old key cannot authorize the new author. Implementing this safely also requires the existing import storage kernel, canonical domain dependency, OS entropy dependency, crate-root registration and lockfile to join the planned write set. These are narrow dependency/registration changes; canonical identity, credential and repository ownership is unchanged._
    - _Evidence: 2026-08-15 — added generation, rotation, revocation, archive and active-credential resolution through Zed's credentials provider and an optimistic identity-lifecycle repository contract. Generation/rotation probes protected storage before requesting entropy, uses fallible OS randomness, challenge-verifies provider read-back and emits validated domain records only after storage succeeds. Definite repository rejection/unavailability removes the uncommitted new key; an unknown commit outcome retains the key and returns its secret-free credential identifier for reconciliation. Rotation versions and links the predecessor/successor together, preserves the historical profile author and creates an empty same-kind successor profile; old, revoked and archived bindings cannot resolve a signer. Signer resolution re-reads the canonical current binding by community/binding ID, preventing a stale pre-rotation `Active` object from signing. Terminal transitions retain protected material for historical Nostr decryption while canonical binding state forbids new signatures. The import storage kernel now also cleans a potentially partial write before returning failure. `cargo test -p zed_credentials_provider nostr_ --release -- --nocapture` passed all 11 import/lifecycle scenarios, including storage-before-entropy failure, repository rollback, unknown-outcome retention, historical authorship and active-signer movement; `./script/clippy -p zed_credentials_provider` and Rust formatting passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 12.6. Add backup and restore compatibility
    - Preserve approved Buzz backup formats with redacted diagnostics and verified restore into canonical storage.
    - _Requirements: 7.2, 16.1_
    - _Capability IDs: CAP-009, CAP-033_
    - _Depends on: 12.4, 12.5_
    - _Reads: projects/buzz/desktop/src-tauri/src/{key_backup,key_backup_tests}.rs, projects/buzz/desktop/src-tauri/src/commands/{identity,identity_key_backup_tests}.rs, crates/zed_credentials_provider/src/{nostr_import,nostr_lifecycle}.rs_
    - _Writes: Cargo.{toml,lock}, crates/zed_credentials_provider/Cargo.toml, crates/zed_credentials_provider/src/{zed_credentials_provider,nostr_import,nostr_backup}.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: round-trip, wrong-password, truncated-backup and log-redaction tests pass_
    - _Discovered contradiction (2026-08-20): the planned Buzz read path used the nonexistent hyphenated `key-backup.rs`; the actual Rust module is `key_backup.rs`, with command-level behavior and concurrency gates in `commands/identity.rs`. A new Rust module also requires crate-root registration, and exact NIP-49 compatibility requires the same bounded `nostr` 0.44.7 codec used by Buzz. The narrow correction registers the module, adds only that workspace dependency, and exposes read-only accessors on the existing internal protected-record wrapper so backup reuses the canonical credential kernel. It does not port Buzz file writing, Tauri commands or a second secret store, and it does not change approved identity ownership or pairing scope._
    - _Evidence: 2026-08-20 — added canonical NIP-49 export and restore around the existing lifecycle repository and Zed credentials provider. Export re-resolves the active binding, verifies protected key identity and runs the production-cost KDF off the GPUI foreground executor. Restore bounds the backup/password and advertised scrypt cost before work, decrypts on the background executor, verifies the expected public key, refuses conflicting destinations and verifies protected-store read-back with cleanup on failure. Seven focused tests pass for the frozen Buzz vector, encrypted round trip through canonical storage, wrong password, truncated backup, excessive KDF cost, idempotent restore, conflicting destination preservation and diagnostic redaction. `cargo check -p zed_credentials_provider`, `cargo test -p zed_credentials_provider nostr_backup --release -- --nocapture`, `./script/clippy -p zed_credentials_provider`, `./script/check-collaboration-dependencies` and Rust formatting passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 12.7. Implement the identity-binding repository
    - Read and write binding versions/revocations through typed tenant inputs and optimistic concurrency.
    - _Requirements: 6.1, 7.1, 7.4_
    - _Capability IDs: CAP-005, CAP-007_
    - _Depends on: 12.3_
    - _Reads: crates/collab/migrations/20260815000100_collaboration_identity_bindings.{up,down}.sql, crates/collaboration_domain/src/account_binding.rs, crates/collab/src/{lib,db}.rs_
    - _Writes: Cargo.lock, crates/collab/Cargo.toml, crates/collab/src/{lib,identity}.rs, crates/collab/src/identity/binding_repository.rs, crates/collab/tests/identity_binding_repository.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab identity_binding_repository` covers tenant isolation, revoke and version conflict_
    - _Discovered contradiction (2026-08-20): the planned unversioned migration read path does not exist because Task 12.3 correctly created a reversible timestamped SQLx pair. The single planned source-file write also could not register the repository or exercise PostgreSQL transaction/RLS SQL through the existing `collab` test setup. The narrow correction registers one identity module, adds the already-canonical domain/error dependencies, enables SeaORM's mock test feature and adds one focused integration test. It does not add a store, schema, protocol, credential owner or service, and no database or deployment is mutated._
    - _Evidence: 2026-08-20 — added a PostgreSQL-only SeaORM repository over the canonical append-only identity-binding table. Every operation begins a transaction, installs the typed community as transaction-local RLS state and retains explicit community predicates. Reads hydrate and revalidate the complete domain aggregate. Appends lock the current head, enforce caller-supplied optimistic version and exact successor ordering, clear only the selected head, map uniqueness races to a closed version conflict and roll back all failures. The repository stores no credential material and exposes closed tenant, conflict, invalid-record and unavailable errors. `cargo test -p collab identity_binding_repository --test identity_binding_repository --no-default-features -- --nocapture` passed all three tenant-isolation, revocation and rollback scenarios; `cargo check -p collab`, `cargo clippy -p collab --lib --no-default-features -- --deny warnings`, Rust formatting and diff hygiene passed. The mandated release/all-target `./script/clippy -p collab` reached the repository cleanly but could not complete because this host lacks Xcode's optional Metal Toolchain; an equivalent debug all-target run hit the same external build prerequisite. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

- [ ] 13. Add typed tenant admission and common authorization

  - [x] 13.1. Define trusted TenantContext construction
    - Construct tenant context only from approved host, listener or deployment routing and reject payload-derived values.
    - _Requirements: 6.1, 6.3_
    - _Capability IDs: CAP-003, CAP-008_
    - _Depends on: 4.1, 11.1_
    - _Reads: projects/buzz/crates/buzz-core/src/tenant.rs, projects/buzz/crates/buzz-relay/src/tenant.rs_
    - _Writes: crates/collaboration_domain/src/{collaboration_domain,tenant}.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collaboration_domain tenant_context` rejects absent, conflicting and event-tag tenants_
    - _Discovered contradiction (2026-08-20): a new Rust domain module cannot compile, expose its opaque types or run the planned focused test without crate-root registration. The narrow correction adds only the module declaration/public type exports and living-spec trace beside `tenant.rs`. It adds no transport, persistence, codec, GPUI or authorization dependency and leaves trusted route resolution with the later admission adapter._
    - _Evidence: 2026-08-20 — added an immutable tenant context with private fields and no `Default`, serde or raw-community constructor. Context establishment requires one explicit bounded trusted route branded as direct host, trusted forwarded host, listener or deployment provenance. Channel mappings, token stamps, signed URLs, event tags and body fields are separate untrusted claims: matching claims can only corroborate, while an absent route, an event-tag-only attempt or any conflicting claim fails without a context and with the same outward error text. Route references reject empty, surrounding-whitespace, control-character and over-limit values. `cargo test -p collaboration_domain tenant_context -- --nocapture` passed all six route-class, absent, event-tag-only, agreement, conflict/privacy and route-bound scenarios; the full 19-test crate suite, `./script/clippy -p collaboration_domain`, `./script/check-collaboration-dependencies`, Rust formatting and diff hygiene passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 13.2. Define common authenticated principals
    - Normalize Zed accounts, Nostr keys, owner-attested agents, scoped tokens and services into typed principals.
    - _Requirements: 6.2, 7.1_
    - _Capability IDs: CAP-007, CAP-008, CAP-023_
    - _Depends on: 12.1, 13.1_
    - _Reads: crates/collab/src/auth.rs, projects/buzz/crates/buzz-auth/**_
    - _Writes: crates/collaboration_domain/src/{collaboration_domain,principal}.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: principal tests reject unverified bindings and preserve service/token scopes_
    - _Discovered contradiction (2026-08-20): the planned isolated Rust file cannot compile, expose the common principal types or execute unit tests without crate-root registration. The narrow correction adds only the domain module declaration/public exports and living-spec trace. It introduces no authentication transport, database, token issuer, policy evaluator or GPUI dependency; later tasks still own verification adapters and authorization decisions._
    - _Evidence: 2026-08-20 — added a tenant-bound, non-deserializable authenticated-principal envelope with distinct Zed account, direct/account-bound Nostr identity, owner-attested agent, scoped-token and service kinds. Direct Nostr authentication retains NIP-42/NIP-98 provenance without implying a Zed account. Binding metadata can be attached only from a same-community active binding; verified-but-inactive and cross-community records fail. Agent construction requires a validated agent profile with matching owner proof and retains the agent as author. Explicit scope sets preserve known and bounded extension scopes for tokens/services, deterministically deduplicate them, reject invalid values and bound presented entries before collection. `cargo test -p collaboration_domain authenticated_principal -- --nocapture` passed all six identity-separation, inactive/cross-tenant binding, active binding, owner-attestation, scope-preservation and bounded-input scenarios; the full 25-test crate suite, `./script/clippy -p collaboration_domain`, `./script/check-collaboration-dependencies`, Rust formatting and diff hygiene passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 13.3. Implement membership, role and resource authorization policy
    - Evaluate membership versions, roles, channel access, ownership, scopes and delegation from typed inputs.
    - _Requirements: 6.2, 6.4_
    - _Capability IDs: CAP-003, CAP-008, CAP-010, CAP-023_
    - _Depends on: 13.2_
    - _Reads: projects/buzz/crates/buzz-auth/**, crates/collaboration_domain/src/{tenant,principal}.rs_
    - _Writes: crates/collaboration_domain/src/{authorization,collaboration_domain}.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: authorization table tests cover every principal/resource/role decision and stale membership_
    - _Discovered contradiction (2026-08-20): the planned standalone domain file cannot compile, expose the policy types or run its table tests without crate-root registration. The narrow correction adds only the authorization module declaration/public exports and living-spec trace. It does not add repositories, membership persistence, transport admission, invitation/delegation verification or UI behavior; those remain with their planned leaves._
    - _Evidence: 2026-08-20 — added a pure tenant-first authorization decision over explicit scopes, community/channel membership state and versions, role, resource ownership and exact delegation. The policy distinguishes every principal kind, treats scoped tokens as their recorded subject, applies owner/admin/member/guest/bot role semantics, requires active current community membership, requires separate active current channel membership for channel-bound resources and rejects a malformed channel without a coordinate. Ownership cannot bypass membership/scope or community/administration gates. Delegation is exact to tenant, delegate, resource, action, unexpired state and current membership version; revoked or mismatched grants confer nothing. `cargo test -p collaboration_domain authorization_policy -- --nocapture` passed all six principal, role, resource/ownership, channel, stale/scope and delegation tables; the full 31-test crate suite, `./script/clippy -p collaboration_domain`, `./script/check-collaboration-dependencies`, Rust formatting and diff hygiene passed. Inventory and specification-validator evidence is recorded in the enclosing checkpoint commit._

  - [x] 13.4. Enforce tenant and policy at Zed RPC admission
    - Bind existing RPC requests to TenantContext and common authorization before handler or database access.
    - _Requirements: 6.1, 6.2, 6.3_
    - _Capability IDs: CAP-003, CAP-008_
    - _Depends on: 13.1, 13.3_
    - _Reads: crates/collab/src/{auth,rpc}.rs, crates/collaboration_domain/src/authorization.rs_
    - _Writes: crates/collab/src/{lib,tenant_admission}.rs, crates/collab/tests/tenant_admission_rpc.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab tenant_admission_rpc` proves authorization precedes database queries_
    - _Discovered contradiction (2026-08-20): current legacy editor RPC sessions contain only `rpc::Principal::User`; they have no trusted community route, membership snapshot or collaboration resource coordinate. Applying tenant policy globally would either invent a forbidden default tenant or break unchanged Editor Workspace behavior. The narrow correction registers a mandatory admission/token boundary for every new tenant-scoped collaborative RPC and leaves editor-only handlers unchanged until their planned explicit mappings exist. A standalone module also requires crate-root registration and a focused integration test. This does not weaken the final requirement: no Collaborative Workspace RPC may use the legacy path._
    - _Evidence: 2026-08-20 — added a fail-closed Zed RPC admission boundary that binds only trusted tenant routes, collapses missing/payload-selected/conflicting tenant failures to one denial, invokes the common policy before issuing an owned authorization token and permits handler/database work only through that token's once-owned operation closure. Denied requests execute zero query closures; an allowed request carries the exact tenant/principal and executes once. `cargo test -p collab tenant_admission_rpc --test tenant_admission_rpc --no-default-features -- --nocapture` passed authorization-order and tenant-conflict scenarios. Focused compile/clippy, Rust formatting, dependency, inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 13.5. Add scoped tokens, invites and virtual-agent membership
    - Implement API scopes, replay controls, invite evidence and NIP-AA virtual membership through the common policy.
    - _Requirements: 6.2, 6.4_
    - _Capability IDs: CAP-008, CAP-010, CAP-023_
    - _Depends on: 13.3_
    - _Reads: projects/buzz/crates/buzz-auth/**, projects/buzz/crates/buzz-db/src/{api_token,relay_invite}.rs, projects/buzz/crates/buzz-relay/src/invite_token.rs, projects/buzz/docs/nips/NIP-AA.md, crates/collaboration_domain/src/{authorization,principal}.rs_
    - _Writes: crates/collaboration_domain/src/{admission_evidence,collaboration_domain}.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: tests cover scope narrowing, invite exhaustion/revocation, replay and unattested virtual agents_
    - _Discovered contradiction (2026-08-20): the planned domain-only file cannot verify bearer hashes/signatures or durably serialize concurrent invite/replay consumption without reversing the approved separation between canonical domain state, protocol adapters and persistence. It also requires crate-root registration to compile and expose its policy inputs. The narrow correction models only already-verified evidence and explicit next-version results; transport adapters remain responsible for cryptographic verification and repositories must atomically commit consumption before admitting work. NIP-AA produces a request/connection-lifetime policy snapshot and explicitly never a persistent agent-membership row._
    - _Evidence: 2026-08-20 — added typed scoped-token, replay-challenge, bounded-invite and NIP-AA virtual-membership evidence. Token admission preserves explicit requested scopes only when they are a subset of the grant and consumes a tenant/token-bound, current, unexpired one-time challenge. Invite redemption rejects stale, revoked, expired or exhausted evidence, advances count/version only for a new member and preserves existing-member idempotency. Virtual membership requires an owner-attested agent plus a current active owner membership, retains owner identity for connection controls and projects only transient member access for the agent without owner role inheritance. Six focused tests cover narrowing/escalation, replay/cross-tenant denial, bounded use/idempotency, exhaustion/revocation, restricted virtual access and unattested/revoked-owner denial. Focused and full crate tests, clippy, dependency, formatting, diff, inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 13.6. Add independent cross-tenant negative traces
    - Exercise RPC, Nostr, database, cache, search, object, Git and count paths across two communities.
    - _Requirements: 6.1, 6.2, 6.3, 20.2_
    - _Capability IDs: CAP-003, CAP-008, CAP-044_
    - _Depends on: 13.4, 13.5_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/collab/src/tenant_admission.rs_
    - _Writes: crates/collab/tests/multitenant_conformance.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab multitenant_conformance` reports no content, ID, count or timing-class leaks_
    - _Discovered contradiction (2026-08-20): the approved dependency order places this conformance leaf before the Nostr, cache, search, object and tenant-scoped Git adapters it names. Live end-to-end traces for absent production seams would require implementing later leaves early or adding forbidden placeholder services. The narrow correction establishes an adapter-neutral independent trace contract now, drives the real tenant/RPC policy gate for all probes and requires each later adapter leaf to emit the same closed observations when its live seam lands. The test harness shares no production reducer, key helper or storage implementation._
    - _Evidence: 2026-08-20 — added an independent two-community trace audit over RPC, Nostr, database, cache, search, object, Git and count seam labels. Every record path exercises own, foreign-ID, missing-ID and foreign-tenant probes through the real common policy/RPC authorization token; denied foreign tenants execute zero operation closures, while allowed lookups are keyed by the admitted tenant. The checker independently rejects content/community/opaque-ID crossover, foreign-inclusive counts, coverage omissions, distinct outward absence errors and distinct timing classes. The fixture proves symmetric isolation for both communities and records different tenant-local counts so a combined count cannot pass accidentally. The focused integration test passed. Release and focused clippy both reached the `collab` crate but were blocked while compiling the unrelated GPUI dev-dependency because the host lacks Xcode's Metal Toolchain; formatting, diff, dependency, inventory and specification validations passed. Later live adapter leaves retain their required E2E negative tests._

- [ ] 14. Add Nostr WebSocket and HTTP adapters

  - [x] 14.1. Establish the versioned Nostr ingress boundary
    - Add the ADR-001-approved listener/sidecar boundary and route accepted operations to domain commands.
    - _Requirements: 2.3, 5.2, 18.2_
    - _Capability IDs: CAP-002, CAP-004, CAP-043_
    - _Depends on: 2.1, 11.13, 13.4_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-001-service-topology.md, crates/collab/src/{main,tenant_admission}.rs_
    - _Writes: crates/collab/src/{collaboration_command,lib,nostr}.rs, crates/collab/src/nostr/ingress.rs, crates/collab/tests/nostr_ingress_version.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab nostr_ingress_version` rejects unsupported versions before a write_
    - _Discovered contradiction (2026-08-20): the approved topology requires this ingress to target a shared versioned domain-command contract, while transactional command/outbox persistence is not implemented until Task 15.4 and the planned single-file write cannot register a Rust module or run an integration target. Implementing a listener-specific store now would create the forbidden second authority. The narrow correction defines only the Zed-owned in-memory command/sink interface, registers the Nostr module and adds a focused integration test. Task 15.4 must implement the same sink transactionally; no route, database, projection writer, sidecar process or framework-version merge lands here._
    - _Evidence: 2026-08-20 — added a versioned Nostr ingress that accepts an already-authorized tenant request, validates its adapter and peer-minimum versions before any sink invocation and translates current requests into one shared command envelope carrying stable operation ID, exact tenant/principal, optimistic versions, typed payload and explicit in-process/temporary-sidecar origin. A narrow async sink returns authoritative operation/version receipts and has no migration or storage implementation. The focused test proves a future version produces zero writes and a current temporary-sidecar request submits exactly one correctly bound command. Focused test, formatting and diff checks passed. Full release clippy remains blocked by the host's missing Xcode Metal Toolchain while compiling the unrelated GPUI dependency; dependency, inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 14.2. Implement NIP-42 WebSocket authentication
    - Preserve challenge, response, timeout, replay and reauthentication behavior under common principals.
    - _Requirements: 5.2, 6.2, 8.1_
    - _Capability IDs: CAP-002, CAP-004, CAP-008_
    - _Depends on: 11.3, 13.2, 14.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/{connection,handlers/auth}.rs, projects/buzz/crates/buzz-auth/**, crates/nostr_compat/src/{event,verification}.rs, crates/nostr_compat/src/buzz_nips/identity.rs, crates/collaboration_domain/src/principal.rs_
    - _Writes: Cargo.toml, Cargo.lock, crates/collab/Cargo.toml, crates/collab/src/nostr.rs, crates/collab/src/nostr/auth.rs, crates/collab/tests/nostr_auth_vectors.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: old test-client auth vectors cover success, timeout, replay, wrong tenant and revoked key_
    - _Discovered contradiction (2026-08-20): the planned single auth file cannot compile against the approved pure event verifier, register the module or run old-client-shaped integration vectors without manifest, crate-root and test writes. Shared Redis replay and authoritative membership/revocation repositories are scheduled after this leaf, so a permissive or process-local production default here would weaken the approved multi-replica security boundary. The narrow correction defines mandatory injected replay/resolver traits with no default; Task 16.1 and the canonical membership repositories must implement them before a live route is enabled. This leaf adds no AUTH persistence, listener route, Redis implementation or duplicate identity store._
    - _Evidence: 2026-08-20 — added a NIP-42 terminal connection state machine with redacted CSPRNG challenge generation, exact AUTH challenge frame, five-second timeout, kind/signature/event-ID and ±60-second freshness verification, unique challenge/relay tags with Buzz-compatible localhost/trailing-slash normalization, optional verified NIP-AA proof, tenant/event replay claiming and a mandatory current-principal resolver. Resolver output must remain in the trusted tenant, match the signed author/owner proof and retain NIP-42 provenance; revoked or cross-tenant results fail closed. AUTH events never enter the command sink or persistence. Safe protocol dispositions preserve accepted, timeout-close, verification-failed, restricted, internal, already-authenticated and already-failed behavior. Six signed old-client-shaped vectors pass for success plus reauthentication, timeout before crypto/replay, tenant-scoped replay, wrong tenant, revoked key, wrong challenge and wrong relay. Formatting and diff checks passed. Full release clippy remains blocked by the host's missing Xcode Metal Toolchain while compiling the unrelated GPUI dependency; dependency, inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 14.3. Implement bounded REQ, COUNT and subscription frames
    - Parse filters, enforce limits and emit EOSE/CLOSED/COUNT frames with cancellation cleanup.
    - _Requirements: 5.2, 8.1, 8.4_
    - _Capability IDs: CAP-002, CAP-004_
    - _Depends on: 11.4, 14.2_
    - _Reads: projects/buzz/crates/buzz-relay/src/{protocol,subscription}.rs, crates/nostr_compat/src/filter.rs_
    - _Writes: crates/collab/src/nostr.rs, crates/collab/src/nostr/subscriptions.rs, crates/collab/tests/nostr_subscriptions.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test nostr_subscriptions --no-default-features -- --nocapture` covers limits, EOSE, close, count privacy, cancellation and resource release_
    - _Discovered contradiction (2026-08-20): the planned single implementation file cannot register the Rust module or supply the required conformance target. The approved dependency order also places this leaf before the canonical event repository and shared pub/sub resource implementation. Adding a process-local store or fanout default would create temporary authority without the required migration controls. The narrow correction registers the module, adds a focused integration target and exposes mandatory tenant-bound query and resource interfaces with no default; Tasks 15.3 and 16.1 must implement those interfaces at their canonical boundaries before the listener is enabled._
    - _Evidence: 2026-08-20 — added bounded REQ, COUNT and CLOSE parsing with 512 KiB frame, 256-byte identifier, ten-filter, per-filter field/value, 1,000-result and 1,024-active-subscription limits. An authenticated session fixes the admitted tenant, principal and connection ID; mandatory injected query/resource interfaces receive that scope. REQ generation-replaces resources, emits historical EVENT frames followed by EOSE and cleans up before CLOSED on query failure. COUNT remains tenant-private and allocates no subscription. CLOSE, replacement and connection cancellation release exact resource tokens, with cleanup failure retained as an observable failure. Six focused conformance scenarios pass for bounds, EVENT/EOSE/replacement/CLOSED, two-tenant COUNT isolation, cancellation, query-failure cleanup and the active ceiling. Formatting and diff checks passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain; dependency, inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 14.4. Implement signed EVENT ingest and OK responses
    - Validate, authorize and idempotently submit events while preserving exact success and rejection frames.
    - _Requirements: 5.1, 5.2, 5.4, 8.1_
    - _Capability IDs: CAP-001, CAP-002, CAP-004_
    - _Depends on: 11.3, 13.3, 14.2_
    - _Reads: projects/buzz/crates/buzz-relay/src/handlers/{event,ingest}.rs, projects/buzz/crates/buzz-relay/src/protocol.rs, .agents/specs/collaborative-workspace/fixtures/protocol/{events,wire-traces}.json, crates/nostr_compat/src/**_
    - _Writes: crates/collab/src/collaboration_command.rs, crates/collab/src/nostr.rs, crates/collab/src/nostr/{event_ingest,subscriptions}.rs, crates/collab/tests/nostr_event_ingest.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test nostr_event_ingest --no-default-features -- --nocapture` differentially matches accepted, duplicate, malformed and unauthorized Buzz behavior; `cargo test -p collab --test nostr_ingress_version --no-default-features -- --nocapture` preserves the shared ingress contract_
    - _Discovered contradiction (2026-08-20): the planned single adapter file cannot register its module, add the differential test target or express duplicate success through the existing domain-command receipt. Authoritative operation deduplication and event persistence are not implemented until Tasks 15.3–15.4, so an adapter-local replay cache would become forbidden duplicate authority. The narrow correction adds an applied-or-duplicate disposition to the existing generic receipt, derives a stable community-plus-event operation ID and leaves the mandatory command sink responsible for atomic deduplication. This leaf adds no event table, cache, route or fanout implementation._
    - _Evidence: 2026-08-20 — added bounded signed EVENT parsing, normalized signed-field wire payloads and pure `nostr_compat` verification on Tokio's blocking executor with Buzz's ±15-minute freshness and content limits. Structurally malformed and cryptographically invalid events fail before submission; the frozen tampered-content vector emits the byte-exact Buzz negative OK response. AUTH replay through EVENT is rejected, normal events require the authenticated direct/bound Nostr or agent author, and NIP-59 gift wraps retain Buzz's distinct envelope-author behavior. Accepted commands traverse the existing versioned ingress with one deterministic community-and-event operation ID. Applied and duplicate receipts emit exact positive empty/`duplicate:` OK frames; rejected and unavailable sinks emit bounded restricted/error frames without leaking internals. Four focused tests pass accepted, duplicate, frozen malformed, wrong-author, malformed-envelope, domain rejection and unavailable-service scenarios, while the pre-existing ingress-version regression also passes. Formatting and diff checks passed. Full release clippy, dependency, inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 14.5. Implement NIP-11, NIP-05 and NIP-98 HTTP routes
    - Expose relay metadata, identity resolution and authenticated HTTP with tenant-bound policy.
    - _Requirements: 5.2, 6.1, 6.2_
    - _Capability IDs: CAP-002, CAP-008_
    - _Depends on: 13.2, 14.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/{nip11,router}.rs, projects/buzz/crates/buzz-relay/src/api/{bridge,nip05}.rs, projects/buzz/crates/buzz-auth/**, crates/collab/src/nostr/{auth,event_ingest,subscriptions}.rs_
    - _Writes: Cargo.lock, crates/collab/Cargo.toml, crates/collab/src/nostr.rs, crates/collab/src/nostr/{event_ingest,http}.rs, crates/collab/tests/nostr_http.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test nostr_http --no-default-features -- --nocapture` covers host binding, signatures, expiry, replay and metadata redaction; `cargo test -p collab --test nostr_event_ingest --no-default-features -- --nocapture` preserves shared signed-event parsing_
    - _Discovered contradiction (2026-08-20): the planned single HTTP file cannot register an Axum module, decode the standard base64 authorization scheme, add integration coverage or share Task 14.4's signed-event parser without manifest, module-root, parser and test writes. Production host, profile, replay and current-principal repositories land in later service/storage leaves, so mounting these routes now would require a default tenant or process-local replay/directory state and violate the approved fail-closed topology. The narrow correction supplies a real but unmounted public router and mandatory-injection NIP-98 boundary; the canonical service composition must provide those traits before mounting it._
    - _Evidence: 2026-08-20 — added content-negotiated `/` and `/info` NIP-11 routes, a tenant-scoped `/.well-known/nostr.json` NIP-05 route and a reusable NIP-98 authenticator. Public routes derive tenancy only from a canonical direct/trusted-forwarded Host, ignore untrusted forwarded headers, expose the same generic NIP-11 document for unmapped hosts and redact optional tenant metadata on lookup failure. Advertised frame/subscription/filter/result/identifier limits use the same adapter constants. NIP-05 bounds/canonicalizes names, performs one admitted tenant lookup, returns empty maps for malformed/missing/foreign records and advertises the bound host. NIP-98 bounds and decodes the standard authorization header, shares normalized signed-event parsing, verifies kind/signature on the blocking executor, enforces ±60-second expiry, exact tenant URL/method, optional-or-required body hash, tenant-scoped atomic replay and current same-tenant NIP-98 principal provenance. Replay/backend outages, revoked identities and mismatched tenants fail closed. Four HTTP integration tests pass NIP-11 negotiation/redaction, two-tenant NIP-05 resolution/empty behavior, valid signature/payload plus replay, and wrong-host, invalid-signature, wrong-method/body, expiry, wrong-tenant, revoked and unavailable scenarios. The Task 14.4 parser regression and focused library check also pass. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain; formatting, diff, dependency, inventory and specification validations passed._

  - [x] 14.6. Add reconnect and local-echo compatibility tests
    - Verify reauthentication, head/window refetch, subscription rearm and optimistic event reconciliation.
    - _Requirements: 8.2, 8.3, 20.2_
    - _Capability IDs: CAP-004, CAP-006, CAP-044_
    - _Depends on: 14.3, 14.4, 14.5_
    - _Reads: projects/buzz/crates/buzz-ws-client/**, crates/collab/src/nostr/**_
    - _Writes: crates/collab/tests/nostr_reconnect.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test nostr_reconnect --no-default-features -- --nocapture` proves no duplicate echo and exposes partial freshness_
    - _Discovered contradiction (2026-08-20): this test leaf precedes the authoritative transactional event sink in Task 15.4 and shared cross-replica subscription resources in Task 16.1, so it cannot truthfully prove the complete production no-local-echo guarantee or add a process-local substitute without violating canonical ownership. The dependency-safe correction freezes the adapter compatibility contract over the existing mandatory query/resource and command-sink interfaces; Tasks 15.4 and 16.1 remain responsible for satisfying it with canonical persistence and fan-out._
    - _Evidence: 2026-08-20 — added three adapter-level reconnect scenarios. A replacement connection signs a fresh NIP-42 challenge, cancels the prior connection's resource, refetches separate authoritative head and older-window filters and rearms the same live subscription under the new connection ID. A partial-freshness scenario keeps the successful head live through EOSE while an unavailable older window emits CLOSED and releases only its own resource. An uncertain optimistic publish reuses the same signed event and deterministic operation ID; applied then duplicate positive acknowledgements plus event-ID replacement of the historical echo leave one authoritative local item through both the in-process consolidated path and temporary-sidecar compatibility path. The focused test target passes all three scenarios. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

- [ ] 15. Establish authoritative event storage and projections

  - [x] 15.1. Add the authoritative signed-event schema
    - Create tenant-fenced event partitions, immutable bytes, signature state and addressable-head indexes under ADR-001.
    - _Requirements: 2.1, 5.1, 17.1_
    - _Capability IDs: CAP-001, CAP-005_
    - _Depends on: 2.1, 11.3, 13.6_
    - _Reads: projects/buzz/crates/buzz-db/**, projects/buzz/migrations/**, crates/collab/src/db/**_
    - _Writes: crates/collab/migrations/20260820000100_collaboration_events.{up,down}.sql, crates/collab/tests/collaboration_event_migration.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test collaboration_event_migration --no-default-features -- --nocapture` verifies checksums, partitions, tenant fences, immutability and rollback_
    - _Discovered contradiction (2026-08-20): the planned unversioned `collaboration_events.sql` path is not a runnable migration under the existing SQLx reversible migration convention and cannot represent or test rollback. A schema-only file also cannot satisfy the named migration-test validation. The dependency-safe correction adds one versioned up/down pair and one static migration-contract test; it does not run the migration, implement the Task 15.2 repository or introduce an event authority outside `collab`._
    - _Evidence: 2026-08-20 — added the sole Zed-owned signed-event table with a community-leading primary key and sixteen community-hash partitions. Forced restrictive row-level policies cover the parent and direct partition access. Exact 32-byte IDs/authors, full unsigned 64-bit timestamps, 16-bit kinds, array tags, 256-KiB content, 512-KiB canonical signature-input bytes, 64-byte signatures and live/historical verification state are bounded in PostgreSQL. Ephemeral events have no valid persistence class. A trigger rejects all row updates while leaving later authorized retention deletion possible. Community-leading chronological, kind, author-kind and addressable indexes preserve greatest timestamp then lowest event-ID head order. The reversible down migration names only the owned table/function. Three focused tests pass SQLx SHA-384 checksum reproduction, reversible discovery, 16-partition and tenant-policy shape, field bounds, immutable trigger, exact index order, ephemeral exclusion and rollback ownership. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No production database was mutated._

  - [x] 15.2. Implement the event repository
    - Store verified events once, deduplicate by ID and query exact heads and bounded filters.
    - _Requirements: 2.1, 5.1, 8.1_
    - _Capability IDs: CAP-001, CAP-005_
    - _Depends on: 15.1_
    - _Reads: crates/collab/migrations/20260820000100_collaboration_events.up.sql, crates/nostr_compat/src/{filter,head}.rs_
    - _Writes: crates/collab/src/db.rs, crates/collab/src/db/collaboration.rs, crates/collab/src/db/collaboration/event_repository.rs, crates/collab/migrations/20260820000200_collaboration_event_heads.{up,down}.sql, crates/collab/tests/event_repository.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test event_repository --no-default-features -- --nocapture` covers duplicate, head, delete, ephemeral and tenant cases_
    - _Discovered contradiction (2026-08-20): the planned single repository file cannot register its dependency-safe module or supply the named integration target. More importantly, immutable event bytes plus an addressable lookup index cannot by themselves preserve the approved deletion watermark: physically deleting the current head would expose an older row, while updating it would violate Task 15.1 immutability. The narrow correction adds a separate tenant-fenced head-watermark migration and reversible rollback. It remains part of the one `collab` event authority and introduces neither a projection store nor adapter-local state._
    - _Evidence: 2026-08-20 — added a Postgres-only event repository over the Task 15.1 table and an additive coordinate-watermark table. Typed input construction verifies event ID, signature and size through `nostr_compat` before storage and requires historical/live provenance to match a historical/bounded timestamp policy. Every database transaction sets the admitted row-zero community; cross-tenant inputs fail before I/O and foreign result rows roll back. Regular events insert once under `(community_id,event_id)`. Ephemeral events return `EphemeralNotPersisted` without opening a transaction. Replaceable and parameterized events atomically advance greatest-time/lowest-ID watermarks, treat exact live replays as duplicates and reject older or tombstoned heads before insertion. Deletion clears only a matching live pointer before removing immutable bytes, retaining the order floor. Bounded queries accept at most ten validated OR filters and 1,000 rows, push ID-prefix, author, kind, inclusive time and generic-tag predicates into SQL, treat empty generic-tag value sets as match-none, expose only regular rows or current live heads and order by timestamp descending/event ID ascending. Exact and coordinate-head reads reconstruct and revalidate community, canonical bytes and event ID. Five focused tests cover inserted/duplicate, invalid and valid ephemeral behavior, verification-policy mismatch, all filter classes and bounds, exact head reconstruction, rollback-owned head schema, delete-before-payload ordering, stale resurrection, and cross-tenant rollback. The Task 15.1 migration regression, focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No database was mutated._

  - [x] 15.3. Define projection provenance and rebuild checkpoints
    - Persist source kind/ID/version, projection version, cursor and drift state for derived tables.
    - _Requirements: 2.2, 17.2_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 15.1_
    - _Reads: crates/collaboration_domain/src/provenance.rs, .agents/specs/collaborative-workspace/migration-plan.md_
    - _Writes: crates/collab/migrations/20260820000300_collaboration_projections.{up,down}.sql, crates/collab/tests/projection_migration.rs, .agents/specs/collaborative-workspace/{design,tasks}.md_
    - _Validation: `cargo test -p collab --test projection_migration --no-default-features -- --nocapture` covers checkpoint resume, version conflict and per-tenant reset_
    - _Discovered contradiction (2026-08-20): the planned unversioned single SQL file cannot be discovered by Zed's reversible SQLx migration loader, prove rollback or provide the named migration test. Columns and positive checks alone also do not make version conflicts observable under concurrent updates. The narrow correction adds a versioned up/down pair, one update-guard trigger and one static migration-contract target. It creates no projection writer or rebuild implementation before Tasks 15.4–15.5._
    - _Evidence: 2026-08-20 — added a tenant-scoped checkpoint keyed by community, projection, source system and bounded source record ID. It preserves optional source version, observation time and paired SHA-256/Nostr/Git integrity provenance; full unsigned projection version and reset generation; a 64-KiB resume cursor; projected/reset timestamps; and clean, suspect, diverged, rebuilding or reset-pending drift state with bounded hashes/errors. Diverged rows require distinct 32-byte authoritative/projection hashes, clean rows cannot retain errors and reset-pending rows cannot retain cursors. A database trigger makes source identity immutable, requires every update to advance the projection version exactly once with serialization-failure conflicts and permits reset generation to hold or advance once; an advancing reset must atomically clear the cursor, enter reset-pending and stamp reset time. Both indexes and forced restrictive RLS lead with community. Three focused tests pass SQLx SHA-384 discovery/checksums, exact rollback ownership, provenance bounds, resume fields, optimistic conflict fencing, reset fencing, drift invariants and tenant-leading indexes/policy. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No rebuild ran and no database was mutated._

  - [x] 15.4. Implement transactional command and outbox persistence
    - Persist accepted commands, authoritative records and one ordered outbox operation under a stable idempotency key.
    - _Requirements: 2.2, 2.3, 8.1_
    - _Capability IDs: CAP-005, CAP-006_
    - _Depends on: 15.2, 15.3_
    - _Reads: crates/collab/src/db/**, crates/collaboration_domain/src/provenance.rs_
    - _Writes: crates/collab/src/db/collaboration/outbox.rs_
    - _Validation: `cargo test -p collab collaboration_outbox` covers retry, crash boundary, duplicate and ordering_
    - _Discovered contradiction (2026-08-20): the planned single Rust write path cannot create the durable command receipt and ordered outbox tables required by the approved atomicity, migration and rollback requirements. The narrow correction adds one reversible SQLx migration pair and one focused integration target beside the named repository module. Both tables remain inside the canonical `collab` Postgres authority, and no adapter-local replay or second aggregate store is introduced._
    - _Evidence: 2026-08-20 — added a Postgres-only `DomainCommandSink` implementation whose owned transaction installs the admitted community, reserves the stable operation ID with contract/principal/adapter/kind/fingerprint metadata, invokes one injected canonical mutation on that same transaction, inserts exactly one bounded provenance-bearing outbox operation and completes the authoritative receipt before commit. A committed duplicate returns its stored authoritative version only when all command identity fields and the SHA-256 payload fingerprint match and its outbox row exists; operation-ID content collisions reject, cross-tenant principals fail before I/O and every mutation/enqueue/completion error explicitly rolls back. The reversible schema gives receipts and globally monotonic outbox identities community-leading keys, one outbox row per operation, a receipt foreign key, bounded payload/provenance/delivery fields, delivery indexes and forced restrictive community RLS. Six focused tests cover first-apply/retry without reexecution, mutation-before-outbox ordering, interrupted enqueue rollback, collision rejection, pre-I/O tenant rejection and SQLx migration checksums/rollback. Focused library check and formatting passed; dependency, inventory and specification gates are recorded in the checkpoint validation. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No production migration ran and no database was mutated._

  - [x] 15.5. Implement projection rebuild and drift comparison
    - Rebuild one tenant/aggregate from authority and compare source/version/count hashes without mutating authority.
    - _Requirements: 2.2, 8.3, 17.2_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 15.3, 15.4_
    - _Reads: crates/collab/src/db/collaboration/{event_repository,outbox}.rs_
    - _Writes: crates/collab/src/db/collaboration/rebuild.rs_
    - _Validation: rebuild twice yields identical projections and a seeded drift produces a scoped diagnostic_
    - _Discovered contradiction (2026-08-20): the planned production module alone cannot provide the named executable two-pass/drift scenario or prove that an adapter failure rolls back a partial projection replacement. The narrow correction adds one focused integration target beside the named module. It introduces no projection table or second authority; aggregate adapters remain responsible for their existing derived tables._
    - _Evidence: 2026-08-20 — added a Postgres-only projection rebuild orchestrator over the Task 15.3 checkpoint schema. A bounded source identifies one projection and canonical provenance; bounded projection rows are sorted by stable key and reject duplicates, oversized rows, excessive counts and excessive aggregate payload. Under one tenant-fenced transaction the injected aggregate adapter reads authority, replaces only the selected derived aggregate and reads it back. SHA-256 comparisons length-prefix source system/record, source version, row count, keys and payloads, yielding a scoped clean/diverged diagnostic with both counts, versions and hashes. The same transaction upserts source provenance, equal or unequal hashes, a monotonically advanced checkpoint version and a bounded content-free drift summary; adapter/storage failure explicitly rolls back projection and checkpoint, and no authority mutation API exists. Three focused tests prove two rebuilds produce identical sorted materialization/hashes without authoritative writes, seeded version/count/row drift yields the exact community/projection/source diagnostic and partial replacement failure rolls back before checkpointing. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No production projection or checkpoint was mutated._

  - [x] 15.6. Enforce ephemeral non-persistence and privacy exclusions
    - Reject durable storage/indexing for ephemeral or privacy-disallowed kinds at the repository boundary.
    - _Requirements: 5.1, 5.3, 6.3_
    - _Capability IDs: CAP-001, CAP-005, CAP-014, CAP-025_
    - _Depends on: 11.5, 15.2_
    - _Reads: crates/nostr_compat/src/generated_kinds.rs, crates/collab/src/db/collaboration/event_repository.rs_
    - _Writes: crates/collab/src/db/collaboration/persistence_policy.rs_
    - _Validation: privacy tests prove prohibited kinds never reach SQL, search or logs_
    - _Discovered contradiction (2026-08-20): a standalone policy module cannot enforce a repository boundary or prove the named SQL/search/log exclusion unless the existing event writer consumes its opaque decision and a focused integration target observes the pre-I/O path. The narrow correction updates that existing `EventRepository::store` seam and its tests; it introduces no search index, event store or privacy authority._
    - _Evidence: 2026-08-20 — added a catalog-driven event persistence policy with kind-bound opaque decisions and no permissive default. Cataloged registered and deliberately defined-unused relay kinds retain their declared persistence class; unclassified kinds and the internal non-relay media kind reject. Every ephemeral kind resolves to transient-only/search-excluded before privacy or database work. Durable private kinds require each declared author-only, recipient, result-reader and author-or-explicit-share gate independently; accepted private material is eligible only for an authorization-scoped index, while community-visible kinds alone may enter community search. The existing repository now requires and revalidates the exact decision kind before opening a transaction, preventing public-decision substitution. Four focused privacy tests prove ephemeral and mismatched-private records produce no transaction log or content marker, overlapping gates cannot be weakened, community search excludes private decisions, and unknown/internal kinds fail with content- and kind-neutral errors. All five prior event-repository tests pass with explicit decisions. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No production event, index or log was written._

  - [x] 15.7. Add Postgres failure and rollback integration tests
    - Exercise transaction abort, outbox interruption, replica lag and schema rollback with authoritative data intact.
    - _Requirements: 8.3, 17.3, 20.1_
    - _Capability IDs: CAP-005, CAP-043, CAP-044_
    - _Depends on: 15.4, 15.5, 15.6_
    - _Reads: crates/collab/src/db/collaboration/**, crates/collab/migrations/collaboration_*.sql_
    - _Writes: crates/collab/tests/collaboration_storage_recovery.rs_
    - _Validation: `cargo test -p collab collaboration_storage_recovery` passes against isolated Postgres_
    - _Discovered contradiction (2026-08-20): the planned migration glob `crates/collab/migrations/collaboration_*.sql` matches none of Zed's approved timestamp-prefixed reversible SQLx artifacts, and mock-database tests cannot validate PostgreSQL function parameter types, MVCC lag or transactional trigger failures. The narrow correction includes the exact Task 15.1–15.4 migration files in the named live test and fixes the two transaction-local tenant setters whose UUID bind was rejected by PostgreSQL `set_config(text,text,bool)`. No schema, ownership or migration strategy changes._
    - _Evidence: 2026-08-20 — added one self-cleaning live recovery drill that creates a UUID-named database from `COLLAB_TEST_DATABASE_URL`, applies the exact event/head/checkpoint/outbox migrations and always terminates sessions and drops the database before reporting its result. A PostgreSQL trigger interrupts outbox insertion after the canonical mutation; real counts prove mutation, receipt and outbox all remain zero, then trigger removal permits one apply and one duplicate retry with exactly one row in each table. A repeatable-read read-only transaction remains on projection version 1 while primary authority and the production rebuilder reach version 2 cleanly, proving lag is observable without overwriting authority. Rolling the derived checkpoint schema down/up preserves one signed event, command authority and outbox operation; a subsequent authoritative rebuild recreates one clean checkpoint. The first live run exposed and corrected `set_config` UUID binding in both command/outbox and projection transactions; canonical UUID text now passes real PostgreSQL while existing mock suites remain green. The final target passed on disposable PostgreSQL 14 in 0.45s, and its container/database were removed. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No persistent external database or container remains._

- [ ] 16. Consolidate realtime, presence infrastructure and search foundations

  - [x] 16.1. Implement tenant-scoped Redis fan-out envelopes
    - Publish source ID/version and tenant-bound payload references without making Redis authoritative.
    - _Requirements: 8.1, 8.4_
    - _Capability IDs: CAP-006_
    - _Depends on: 15.4_
    - _Reads: projects/buzz/crates/buzz-pubsub/**, crates/collab/src/**_
    - _Writes: crates/collab/src/pubsub/envelope.rs_
    - _Validation: pub/sub tests reject wrong-tenant envelopes and deduplicate local source IDs_
    - _Discovered contradiction (2026-08-20): the planned nested envelope file cannot be registered without Zed's required non-`mod.rs` module roots, and a production module alone cannot provide the named hostile-wire and deduplication scenarios. The narrow correction adds `crates/collab/src/pubsub.rs`, exports it from the existing library root and adds one focused integration target. Redis transport remains unimplemented until Task 16.2._
    - _Evidence: 2026-08-20 — added a strict 16-KiB v1 fan-out envelope that carries only community, positive Postgres-compatible outbox sequence, canonical lowercase topic, bounded source system/record/version/observation provenance and a lowercase SHA-256 payload reference. Encoding contains no authoritative payload or content; decoding rejects unknown fields, future versions, missing source versions, malformed hashes, invalid topics and oversized frames. A tenant-fixed local deduplicator rejects either caller/envelope tenant mismatch before state mutation, keys local and Redis echoes by canonical source system/ID/version rather than delivery sequence, admits later source versions and evicts oldest keys at a caller-selected capacity bounded to 65,536. Redis remains a lossy notification hint; ordered replay and payload authority stay in Postgres. Four focused tests cover reference-only round trip, pre-mutation cross-tenant rejection, local/Redis duplicate suppression plus bounded eviction and malformed/version/size failures. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No Redis connection or publish occurred._

  - [x] 16.2. Implement cross-replica subscription fan-out
    - Connect outbox delivery to bounded local/Redis subscriptions with cancellation and replay cursors.
    - _Requirements: 8.1, 8.2, 8.4_
    - _Capability IDs: CAP-004, CAP-006_
    - _Depends on: 14.3, 16.1_
    - _Reads: crates/collab/src/{nostr/subscriptions,pubsub/envelope}.rs_
    - _Writes: crates/collab/src/pubsub/subscription_bus.rs_
    - _Validation: two-replica test covers ordering, reconnect replay, duplicate suppression and shutdown cleanup_
    - _Discovered contradiction (2026-08-20): the named bus file cannot prove a two-replica/reconnect/cancellation contract without an executable integration target, while adding a Buzz Redis client directly would violate the approved adapter boundary and introduce an unplanned dependency owner. The narrow correction adds one focused test and injects bounded authoritative replay and transport interfaces; deployment-specific Redis/Postgres implementations can bind them without changing the bus or making Redis authoritative._
    - _Evidence: 2026-08-20 — added a tenant-fixed subscription bus with injected authoritative cursor replay and encoded transport publication. Subscribe registers an initializing receiver before awaiting replay, buffers concurrent live work, validates tenant/topic/cursor, merges replay and buffer by strictly increasing outbox sequence and suppresses duplicate source system/ID/version pairs. Limits cap 4,096 subscriptions, 1,024 queued deliveries, 1,000 replay rows and the initialization buffer. Active local delivery is nonblocking; full or closed consumers are removed. Authoritative publication delivers locally and always attempts transport, so a Redis failure can be retried even after local dedup; strict remote decode plus the Task 16.1 deduplicator suppresses the publishing replica's echo. Drop/cancel removes registrations synchronously and shutdown drains all registrations, emits a marker when capacity permits and closes every receiver. Two focused async tests cover ordered two-replica replay/live delivery, local echo suppression, cursor-2 reconnect replay of 3–4, cancellation, terminal shutdown, foreign-tenant pre-registration rejection, slow-subscriber removal and transport-failure retry. Focused library check, formatting, diff, dependency, inventory and specification validation passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain. No Redis or production outbox was contacted._

  - [x] 16.3. Add replica freshness and partial-service state
    - Track heartbeat, projection lag, pub/sub availability and last trustworthy cursors for clients/operators.
    - _Requirements: 8.3, 19.3_
    - _Capability IDs: CAP-004, CAP-006, CAP-043_
    - _Depends on: 15.5, 16.2_
    - _Reads: projects/buzz/migrations/0026-*.sql, crates/collab/src/pubsub/subscription_bus.rs_
    - _Writes: crates/collab/src/freshness.rs_
    - _Validation: integration test distinguishes healthy, lagging, disconnected and recovering replicas_
    - _Discovered contradiction (2026-08-20): the planned library file can define the freshness projection but cannot satisfy the named executable integration scenario by itself. The narrow correction adds one focused integration target; it does not add persistence, transport or a second health service._
    - _Evidence: 2026-08-20 — added a tenant-fixed replica freshness tracker that combines epoch-scoped monotonic heartbeat tokens, heartbeat age, authoritative/projection cursor lag and pub/sub availability into healthy, lagging, disconnected and recovering states. Missing/stale heartbeats, unavailable pub/sub, same-epoch token regression, epoch replacement, foreign tenants and invalid cursor/future-time observations fail closed. Repeated tokens cannot renew heartbeat age. Disconnects retain the last trustworthy cursor and require two consecutive fully healthy samples before returning from recovering to healthy; lagging/disconnected observations cannot advance that cursor. Two focused integration tests exercise all four states, cursor retention/recovery, stale repeated tokens, regression, epoch replacement and tenant rejection. Focused formatting, clippy, test, library-check, dependency, inventory and specification gates passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

  - [x] 16.4. Add privacy-aware collaboration search schema
    - Create tenant-scoped searchable projections and database-level exclusions for private kinds.
    - _Requirements: 6.3, 9.4_
    - _Capability IDs: CAP-015_
    - _Depends on: 15.3, 15.6_
    - _Reads: projects/buzz/crates/buzz-search/**, projects/buzz/migrations/0008-*.sql_
    - _Writes: crates/collab/migrations/collaboration_search.sql_
    - _Validation: migration test proves excluded content produces no searchable vector or index entry_
    - _Discovered contradiction (2026-08-20): a single irreversible `collaboration_search.sql` would violate Zed's reversible migration convention, and copying signed-event bodies into the named search projection would create prohibited duplicate transcript state. The narrow correction adds one reversible migration pair plus one focused migration target. Signed-event vectors are generated on immutable event authority; only non-event canonical resources occupy the rebuildable projection table._
    - _Evidence: 2026-08-20 — added a tenant-fenced search migration that stores Buzz's positive event-kind allowlist `(0, 9, 40002, 45001, 45003)` as a generated vector directly on `collaboration_events`, with a partial GIN index restricted to non-null vectors. All other event kinds therefore have neither copied search text nor an index-eligible vector. Added a provenance/version-bearing projection for non-event profiles, communities, projects, repositories, tasks, agents, workflows and media; only community-visible rows generate vectors, while authorized-restricted and excluded rows remain null under forced restrictive RLS. Rollback removes the derived table, index and event vector without cascade. Static tests pin privacy, tenant, rollback and checksum invariants. A live PostgreSQL 14 test proves a public kind-9 event and community project receive vectors, a kind-1059 ciphertext event and restricted project receive null vectors, only the public event matches FTS, and the event GIN index predicate is exactly `search_tsv IS NOT NULL`; the disposable container was stopped and removed after the 0.21-second run. Focused formatting, test, library-check, dependency, inventory, diff and specification gates passed. Full clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

  - [x] 16.5. Implement authorized search repository primitives
    - Apply tenant/visibility policy before ranking and limit and expose projection freshness.
    - _Requirements: 9.4, 8.3_
    - _Capability IDs: CAP-015_
    - _Depends on: 13.3, 16.4_
    - _Reads: crates/collab/migrations/collaboration_search.sql, projects/buzz/crates/buzz-search/**_
    - _Writes: crates/collab/src/search/repository.rs_
    - _Validation: search tests cover authorization-before-limit, ranking, excluded kinds and lag markers_
    - _Discovered contradiction (2026-08-20): the nested repository file requires Zed's non-`mod.rs` `crates/collab/src/search.rs` module root, and the planned production file cannot alone prove pre-I/O authorization or live Postgres ranking/privacy/freshness behavior. The narrow correction adds that module root, the existing crate export and one focused test target; Task 22.2 retains higher-level filter/query orchestration._
    - _Evidence: 2026-08-20 — added a PostgreSQL-only collaboration search repository that requires the canonical `collaboration:search` scope plus active community-read authorization before beginning a transaction. Authorized work binds the typed tenant through RLS, normalizes and caps input at 4,096 characters, bounds pages/results, supports Buzz-compatible full-text and trailing-token prefix modes, and unions only non-null signed-event vectors with community-visible canonical vectors before rank, stable ordering, limit and offset. Hits contain only canonical event IDs/kinds or bounded provenance/document references, never copied content. An aggregate checkpoint read exposes current, lagging with affected count, or unavailable projection freshness. Mock tests prove missing scope performs zero database work, visibility predicates precede rank/limit, ordered references decode, lag is surfaced and hostile/oversized input is bounded. A live PostgreSQL 14 test proves full-text and prefix queries, nonincreasing rank order, exclusion of higher-frequency kind-1059 ciphertext and restricted task candidates, inclusion of the public kind-9 event/project, and a diverged checkpoint lag marker; the final run passed in 0.36 seconds and its disposable container was stopped and removed. Focused formatting, warning-denied library clippy, tests, library check, dependency, inventory, diff and specification gates passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

- [ ] 17. Build resumable Buzz data importers

  - [x] 17.1. Define migration checkpoint and integrity records
    - Persist tenant/shard, source/target cursors, counts, hashes, status and rollback boundary.
    - _Requirements: 17.1, 17.2, 17.3_
    - _Capability IDs: CAP-005, CAP-045_
    - _Depends on: 15.3_
    - _Reads: .agents/specs/collaborative-workspace/migration-plan.md, crates/collab/src/db/**_
    - _Writes: crates/collab/src/migration/buzz/checkpoint.rs_
    - _Validation: checkpoint tests cover interruption, monotonic resume and rejected cross-tenant reuse_
    - _Discovered contradiction (2026-08-20): the listed Rust file cannot persist or database-fence migration progress, and its nested path requires non-`mod.rs` module roots. The narrow correction adds one reversible run/checkpoint migration pair, the two required module roots and one focused test target. This implements the approved checkpoint strategy without starting an importer or crossing any production rollback boundary._
    - _Evidence: 2026-08-20 — added globally unique migration-run assignments bound to one community/source revision and forced-RLS checkpoints keyed by tenant, run, stream and shard. Checkpoints persist optimistic version, lifecycle, independent monotonic source/target cursor sequence plus bounded opaque token, four monotonic counts, 32-byte source/target hashes, bounded errors and a named reversible or timestamped irreversible boundary. Rust and PostgreSQL both reject identity/version/status regression, same-sequence token substitution, same-progress hash substitution, count/cursor rollback, mutable irreversible evidence and rolled-back state after the point of no return. The repository creates, loads and atomically saves typed tenant-bound checkpoints; cross-tenant reuse rejects before transaction creation. Pure tests cover pending→running→interrupted→exact resume, cursor/token regression and irreversible rollback denial; schema/checksum tests pin RLS, integrity and reversible removal. A live PostgreSQL 14 test creates and reloads the initial record, persists running/interrupted state, reconnects a repository, resumes at the exact cursor/count/hash and rejects a stale-version writer; the final run passed in 0.16 seconds and the disposable container was stopped and removed. Focused formatting, warning-denied library clippy, tests, library check, dependency, inventory, diff and specification gates passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

  - [x] 17.2. Import signed events and addressable heads
    - Preserve original bytes, IDs and signatures while attaching verified tenant/provenance metadata.
    - _Requirements: 17.1, 17.2_
    - _Capability IDs: CAP-001, CAP-005, CAP-045_
    - _Depends on: 15.2, 17.1_
    - _Reads: projects/buzz/crates/buzz-db/**, .agents/specs/collaborative-workspace/fixtures/migrations/**_
    - _Writes: crates/collab/src/migration/buzz/events.rs_
    - _Validation: fixture import preserves byte/hash/signature/head counts and is idempotent after interruption_
    - _Implementation finding (2026-08-20): the live event repository correctly rejects stale addressable writes, while Buzz's migration source retains superseded and deleted signed rows. The dependency-safe boundary is therefore a migration-only writer into the same immutable canonical tables. It verifies historical signatures, inserts retained rows regardless of current-head visibility and rebuilds the existing head watermark without exposing a second runtime write authority. Buzz stores signed fields rather than a raw wire document, so “original bytes” means the exact reproducible NIP-01 signature-input bytes whose SHA-256 is the preserved event ID; the preserved signature is compared independently._
    - _Evidence: 2026-08-20 — added a PostgreSQL-only, tenant-fixed importer for batches of at most 1,000 strictly ordered Buzz rows. Source construction verifies canonical bytes, event ID, Schnorr signature, timestamp representation, replacement coordinate and Buzz's no-ephemeral-storage invariant before mutation. Each transaction installs row-zero RLS, inserts immutable historical rows idempotently, reads every row back and rejects an ID collision if canonical bytes or signature differ. Addressable rows rebuild the canonical greatest-time/lowest-ID watermark; a deleted winner retains a null-live tombstone floor. Ordered source/read-back SHA-256 values cover source sequence, ID, canonical bytes and signature, and the result exposes inserted/duplicate/coordinate counts plus the exact final source sequence for checkpoint updates. Frozen signed-event fixtures prove a partial batch, exact replay after simulated interruption, an overlapping completion window, five preserved rows, byte/signature equality, one addressable coordinate and the deleted lower-ID same-second winner. A cross-tenant batch rejects before database I/O. The live PostgreSQL 14 run passed in 0.26 seconds and its disposable container was stopped and removed. Focused formatting, warning-denied library clippy, tests, library check, dependency, inventory, diff and specification gates passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

  - [x] 17.3. Import community, membership and channel state
    - Import service-issued community, membership, invite and channel records with explicit provenance.
    - _Requirements: 17.1, 17.2_
    - _Capability IDs: CAP-003, CAP-005, CAP-010, CAP-045_
    - _Depends on: 17.1, 17.2, 18.1_
    - _Reads: projects/buzz/migrations/**, projects/buzz/crates/buzz-db/src/channel.rs_
    - _Writes: crates/collab/src/migration/buzz/community_state.rs_
    - _Validation: importer rejects unknown versions, preserves membership versions and is idempotent after interruption_
    - _Discovered contradiction (2026-08-20): this importer was ordered before Task 18.1 even though Zed's existing integer channel tables are semantically incompatible and the canonical community/member/channel/invite targets did not yet exist. Task 18.1 is therefore an explicit prerequisite and is executed first. This avoids a temporary staging store and preserves the approved one-owner architecture._
    - _Implementation finding (2026-08-20): Buzz's current community tables have no shared aggregate-version column, and older database shapes are already represented by the 30 checksummed source migrations. The importer therefore accepts only a frozen, normalized schema-v30 snapshot and requires the source reader to supply stable positive community/channel/membership/invite versions plus resolved canonical principal IDs. Original Buzz public keys, row keys and the complete normalized row remain covered by source provenance and integrity; an older deployment is upgraded on a copy while its original stays untouched until verification._
    - _Evidence: 2026-08-20 — added a PostgreSQL-only community-state importer over the canonical Task 18.1 tables. Batches accept at most 1,000 strictly increasing rows for one tenant and reject unknown schema versions, invalid roles/statuses/types/visibility, nil IDs, malformed lowercase public keys/policy hashes, inconsistent timestamps/TTL pairs, invalid invite use limits and cross-tenant input before I/O. Community, relay membership, join-policy acceptance, channel, relay invite and channel-membership records retain exact Buzz table/key identity, schema version, observation time and a SHA-256 over the full normalized source record; target conflicts succeed only when that stored integrity matches. Source/read-back batch hashes bind row sequence and integrity, so overlap after interruption is observable and idempotent without updates or a staging authority. A live PostgreSQL 14 run imported five dependency-ordered rows, replayed all five as duplicates, completed two remaining rows, preserved community membership versions 7/12 and channel membership version 15, and rejected a changed-host retry as divergence; the final run passed in 0.15 seconds and the disposable container was stopped and removed. Pure tests reject schema v31 and a foreign tenant before database I/O. Focused formatting, warning-denied library clippy, tests, library check, dependency, inventory, diff and specification gates passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._

  - [x] 17.4. Import object and Git metadata by content identity
    - Inventory object keys, hashes, repository coordinates and refs without copying bytes prematurely.
    - _Requirements: 17.1, 17.2_
    - _Capability IDs: CAP-019, CAP-031, CAP-045_
    - _Depends on: 17.1_
    - _Reads: projects/buzz/crates/buzz-media/**, projects/buzz/crates/buzz-relay/src/git/**_
    - _Writes: crates/collab/src/migration/buzz/object_git_metadata.rs_
    - _Validation: fixture import matches object/ref hashes and reports missing objects without advancing checkpoint_
    - _Implementation finding (2026-08-20): Buzz's bucket deliberately mixes three identity classes: media blobs, Git packs and manifests whose keys are their byte SHA-256; derived thumbnails and Git indexes whose keys name source content; and tenant bindings/metadata (sidecars, upload records and repository pointers) whose own bytes are identified by an observed SHA-256 and, for mutable pointers, an ETag. The importer preserves those distinctions instead of treating an S3 ETag as a content digest or copying large object bodies. Tenant-reachable inventory begins at media bindings and repository pointers, then follows blob/thumbnail, current and ancestor manifest, pack and optional index references; fleet probes and unreachable immutable CAS objects are not attributed to a tenant._
    - _Evidence: 2026-08-20 — added a bounded, strictly ordered, tenant-fixed metadata importer that accepts streamed object hashes plus only bounded sidecar/upload/manifest/pointer bodies. It validates exact Buzz key taxonomies, metadata body length and digest, content-addressed key digests, canonical manifest v1 bytes, ref names/OIDs, repository coordinates, pointer ETags, media size/type agreement and manifest ancestry without writing or copying object bodies. Its successful result contains sorted reachable object identities, media bindings, repository coordinates, refs, packs, deterministic source/target/ref-state hashes and the sole checkpoint-progress token. Missing blobs, thumbnails, manifests, ancestors or packs instead produce a sorted diagnostic report whose API cannot expose checkpoint progress; unknown keys, digest divergence, duplicate listings, ancestry cycles and foreign-tenant bindings fail closed. The focused fixture preserved five exact object identities and the Git branch/ref hash, a missing pack withheld progress at source sequence five, and a foreign-community pointer was rejected. Focused formatting, warning-denied library clippy and the dedicated test target passed._

  - [x] 17.5. Import desktop settings, drafts, read state and archive
    - Version and import general configuration, drafts, read state and transcript archive while preserving source files.
    - _Requirements: 9.3, 17.1, 17.2_
    - _Capability IDs: CAP-013, CAP-045_
    - _Depends on: 12.4, 17.1_
    - _Reads: projects/buzz/desktop/src-tauri/src/{migration,archive,event?sync}/**, projects/buzz/desktop/src/features/{messages,channels,local-archive,notifications,presence}/**_
    - _Writes: crates/zed/src/migration/buzz/desktop_state.rs_
    - _Validation: every desktop fixture version imports twice identically and source files remain unchanged_
    - _Discovered contradiction (2026-08-20): the original read set named only Tauri modules, but Buzz drafts, NIP-RS local read caches, manual-unread state and most device preferences are actually versioned WebKit `localStorage` records implemented in TypeScript. The read set is widened to those authoritative sources without changing the approved Zed settings/session/collaboration ownership. The nested Rust write path also requires `migration.rs` and `migration/buzz.rs` module roots, plus `sha2`/`thiserror` dependencies already present in the workspace lock graph._
    - _Implementation finding (2026-08-20): direct mutation of a live WebKit database or Buzz archive would violate the rollback requirement and couple Zed to browser/SQLite implementation details. The importer therefore consumes a bounded, versioned read-only snapshot produced from those sources and emits a deterministic owner-neutral write batch. Zed settings, native draft/read-state caches and collaboration archive stores remain the only final owners; this module neither creates a second runtime store nor deletes source state. Credential-shaped general configuration is rejected for Task 12.4's verified credential path, pure timeline caches are intentionally skipped, and the full raw signed archive event remains available to the canonical event verifier._
    - _Evidence: 2026-08-20 — added v1/v2 desktop snapshot and v1-v4 archive-schema validation; deterministic imports for general configuration and device settings, legacy and relay-scoped drafts, mention/attachment metadata, NIP-RS contexts, publishable/source-time metadata, legacy/current manual-unread entries, raw archived events, scope membership and save subscriptions. Import validates resource bounds, public keys, relay URLs, event/raw-JSON agreement, archive foreign keys and unique keys, exact migration-marker sets, timestamps, context bounds and secret exclusion. Source and normalized target SHA-256 values support repeat verification; settings, drafts, read state and archive rows are sorted so a second import is byte-equivalent. Both frozen desktop fixture versions imported twice identically, legacy missing draft fields upgraded without loss, current archive rows retained, cache-only state was counted but excluded, a file-backed fixture remained byte-identical and nested secret material failed closed. A lightweight harness compiled the actual module and passed all three tests plus warning-denied Clippy. The native Zed test build passed the WebRTC dependency after disk recovery but remains blocked in the unrelated `gpui_macos` build because the host lacks Xcode's Metal Toolchain._

  - [x] 17.6. Add migration rollback and verification harness
    - Compare counts/hashes, halt on divergence and restore pre-boundary binary/config/data fixtures.
    - _Requirements: 17.2, 17.3, 17.4, 20.1_
    - _Capability IDs: CAP-005, CAP-043, CAP-044, CAP-045_
    - _Depends on: 17.2, 17.3, 17.4, 17.5, 17.7, 17.8, 17.9_
    - _Reads: crates/collab/src/migration/buzz/**, crates/zed/src/migration/buzz/**_
    - _Writes: crates/collab/tests/buzz_import_recovery.rs_
    - _Validation: isolated harness demonstrates resume, idempotency, divergence halt and pre-boundary rollback_
    - _Evidence: 2026-08-20 — added an isolated recovery harness over the canonical migration checkpoint state machine and the real desktop-state and agent-state staging importers. The harness advances count/hash checkpoints only after cross-stream agreement, preserves counts on interruption/resume and replay of the same source window, fails and permanently halts on a target-hash divergence, and restores the exact pre-boundary binary, configuration and data fixture before transitioning to `RolledBack`. The focused integration target passed all eight recovery/importer tests; warning-denied Clippy and the collab library check passed with GPUI's existing runtime-shader feature because this host lacks the offline Metal compiler. Formatting, collaboration dependency boundaries, the exhaustive inventory gate and the feature-spec validator passed._

  - [x] 17.7. Import workflow, moderation and lifecycle state
    - Stage workflow/run/approval, moderation, retention and deletion checkpoints with workers disabled.
    - _Requirements: 15.1, 15.2, 15.3, 17.1, 17.2_
    - _Capability IDs: CAP-027, CAP-029, CAP-030, CAP-045_
    - _Depends on: 17.1, 17.2_
    - _Reads: projects/buzz/migrations/**, projects/buzz/crates/buzz-db/src/{moderation,workflow}.rs_
    - _Writes: crates/collab/src/migration/buzz/lifecycle_state.rs_
    - _Validation: importer preserves legal state/checkpoints and leaves workflow/deletion/retention workers disabled_
    - _Evidence: 2026-08-20 — added a schema-versioned, tenant-fenced Buzz lifecycle staging importer for workflow definitions/runs/approval gates, moderation reports/restrictions/actions, NIP-RS replay-retention watermarks, and whole-community deletion state/requests/approvals/checkpoints. The importer preserves deterministic source/staged hashes, rejects duplicate identities, broken parent links, definition-hash drift, cross-tenant records, impossible workflow/moderation states, mismatched frozen-inventory approvals, forward deletion checkpoints and stale lease generations. Workflow scheduler/executor, retention and deletion worker activation is represented by an immutable all-disabled policy; active source workflows and terminal abort evidence cannot start work. Buzz deletion runtime inspection was additionally required to preserve its approved/fenced abort recovery semantics; this narrows validation but does not change the approved architecture or scope. Four focused lifecycle tests, warning-denied collab library clippy, collab library check, collaboration dependency validation and collaborative-workspace inventory validation passed. Commit: enclosing leaf commit, reported after creation._

  - [x] 17.8. Import push leases and wake outbox state
    - Stage encrypted leases, generations and pending wake records without contacting providers.
    - _Requirements: 9.5, 17.1, 17.2_
    - _Capability IDs: CAP-016, CAP-045_
    - _Depends on: 17.1, 17.2_
    - _Reads: projects/buzz/migrations/0022-*.sql, projects/buzz/migrations/0023-*.sql_
    - _Writes: crates/collab/src/migration/buzz/push_state.rs_
    - _Validation: importer preserves encrypted values/generations, rejects unknown version and sends no wake_
    - _Evidence: 2026-08-20 — added a versioned, tenant-fenced push-state staging importer for effective NIP-PL leases, durable wake-outbox rows and event-match jobs. It preserves source event references, monotonic generations, endpoint enablement, opaque endpoint grants, bounded subscription JSON, endpoint/event dedup identities and queue attempts; rejects unknown format versions, cross-tenant rows, incomplete active/tombstone tuples, duplicate addresses/events, future-generation wakes, current-generation endpoint drift and invalid claim state. Nonportable `sending`/`matching` claims are deterministically reconciled to `pending` with attempts retained and claim fences cleared, producing separate source/staged hashes and a recovery count. Matcher, wake dispatcher and provider contact remain immutable-disabled, so staging sends no wake. The task's listed 0022 migration is unrelated TTL refresh and 0023 contains only the push gate; implementing the approved leaf required the actual 0012/0013 lease/outbox and 0018 match-queue migrations plus `buzz-db/src/push.rs`. Encrypted signed lease events remain canonically owned by Task 17.3 and are referenced by event ID here instead of duplicated. Four focused push-state tests and warning-denied collab library clippy passed. Commit: enclosing leaf commit, reported after creation._

  - [x] 17.9. Import managed-agent, team and snapshot staging records
    - Stage versioned private agent/team/persona/snapshot records for later canonical agent import.
    - _Requirements: 11.2, 11.3, 17.1, 17.2_
    - _Capability IDs: CAP-023, CAP-024, CAP-045_
    - _Depends on: 12.4, 17.1_
    - _Reads: projects/buzz/desktop/src-tauri/src/managed-agents/**, projects/buzz/desktop/src-tauri/src/archive/**_
    - _Writes: crates/zed/src/migration/buzz/agent_staging.rs_
    - _Validation: every agent fixture stages idempotently with version/privacy hashes and source preservation_
    - _Evidence: 2026-08-20 — added a local, owner-profile-scoped staging format for managed-agent instances, personas, teams, plain agent snapshots, NIP-44 v2 encrypted snapshots, team snapshots and hash-only references to archived observer/turn-metric evidence. Source JSON is bounded and hashed before transformation, exact format/version discriminators and nested team members are validated, memory-bearing artifacts receive a private-memory classification, encrypted artifacts remain opaque, and archive payloads stay owned by the Task 17.5 desktop archive import instead of being duplicated. Inline nsecs and provider/environment secrets must resolve through explicit protected-credential bindings; staged JSON contains only typed credential references and value hashes, while the borrowed source bytes remain unchanged. Stable source, staged, privacy and idempotency hashes make replay deterministic. Agent execution, automatic start and credential use are immutable-disabled. The task's `managed-agents/**` read path does not exist; Buzz uses `managed_agents/**`, which was inspected together with the listed archive source. A minimal harness compiled the actual Zed module and passed all four source-preservation, idempotency, privacy, owner-scope and version-failure tests plus warning-denied all-target clippy; the full native Zed target remains unavailable on this host because the unrelated Xcode Metal Toolchain is not installed. Commit: enclosing leaf commit, reported after creation._

## Milestone 3 — communication and awareness parity

- [ ] 18. Extend canonical channels, communities and membership

  - [x] 18.1. Add community and channel projection schema
    - Create tenant-fenced community, membership and channel projection tables with source provenance.
    - _Requirements: 2.2, 6.1, 9.1_
    - _Capability IDs: CAP-003, CAP-005, CAP-010_
    - _Depends on: 15.3, 15.7_
    - _Reads: projects/buzz/crates/buzz-db/src/channel.rs, crates/collab/src/db/queries/channels.rs_
    - _Writes: crates/collab/migrations/collaboration_channels.sql_
    - _Validation: migration tests cover tenant fences, provenance indexes and down migration_
    - _Discovered contradiction (2026-08-20): one non-reversible SQL file cannot satisfy the required down-migration gate, and Task 17.3 cannot preserve Buzz relay invites or join-policy acceptance evidence unless their canonical persistence exists before import. The narrow dependency-safe correction uses the repository's reversible timestamped migration pair, includes invite and policy-evidence tables in the same community/channel ownership boundary, and adds one focused schema test. It does not implement lifecycle commands or redemption behavior reserved for Tasks 18.2 through 18.5._
    - _Evidence: 2026-08-20 — added canonical community, community-membership, join-policy-acceptance, channel, invite and channel-membership tables. Tenant-data primary, foreign and query-index paths are community-leading while the trusted routing host remains globally unique; principal-bound composite foreign keys reject cross-tenant creators, memberships and invite issuers. Bounded roles, statuses, channel types, visibility, TTL/expiry pairs, invite use counts, aggregate versions and full source provenance are database-enforced. All six tables enable and force RLS with an explicit permissive candidate policy plus a restrictive transaction-local community policy, avoiding PostgreSQL's deny-all behavior for a restrictive-only policy set while retaining the tenant fence. The down migration drops six tables in dependency order without cascade, and SQLx checksums pin both directions. A live PostgreSQL 14 NOBYPASSRLS request role inserted tenant A, observed zero tenant-A rows as tenant B, failed a foreign-tenant insert and rolled the schema down to no remaining community table; the final run passed in 0.11 seconds and the disposable container was stopped and removed. Focused formatting, warning-denied library clippy, migration tests, library check, dependency, inventory, diff and specification gates passed. Full release clippy reached `collab` but remains blocked while compiling the unrelated GPUI dependency because the host lacks Xcode's Metal Toolchain._
    - _Follow-up correction (2026-08-20): the least-privilege Task 18.1 test exposed that the earlier identity, event/head, projection, receipt/outbox, search and migration-checkpoint schemas also had restrictive-only policy sets. Each now has an explicit permissive candidate paired with its unchanged forced restrictive tenant predicate, including every event partition. A catalog test requires a pair for every restrictive policy. A live PostgreSQL 14 NOBYPASSRLS role applied all seven repaired migrations, wrote within tenant A, observed zero rows from tenant B and failed a foreign write in 0.44 seconds; the disposable container was removed. This changes admission from accidental deny-all to the designed tenant predicate and does not weaken SQL privileges or cross-tenant isolation._

  - [x] 18.2. Implement community lifecycle commands
    - Add create, update, archive and join-policy transitions with version and authorization checks.
    - _Requirements: 6.2, 6.4, 9.1_
    - _Capability IDs: CAP-003, CAP-010_
    - _Depends on: 13.3, 18.1_
    - _Reads: projects/buzz/crates/buzz-core/src/community.rs, crates/collaboration_domain/src/authorization.rs_
    - _Writes: crates/collaboration_domain/src/community.rs_
    - _Validation: domain tests cover legal transitions, stale versions and unauthorized archive_
    - _Discovered contradiction (2026-08-20): `projects/buzz/crates/buzz-core/src/community.rs` does not exist. Buzz's authoritative community lifecycle behavior is split across `buzz-db/src/lib.rs` create/archive/unarchive transactions, `buzz-relay/src/api/operator.rs`, `buzz-relay/src/handlers/community_provisioning.rs`, and the join-policy configuration and invite APIs. Those sources were used without changing the approved canonical Zed domain owner or this leaf's scope._
    - _Evidence: 2026-08-20 — added bounded canonical community host/icon/join-policy values, a hydratable versioned community aggregate, and pure create, metadata-update, join-policy, archive and restore commands. Every command enters the common tenant/principal/scope/membership/delegation authorization policy before checking version or mutating state; create/archive/restore require delete authority while metadata and policy changes require manage authority. Successful changes increment exactly one aggregate version, no-op retries preserve the version, stale versions preserve the complete prior aggregate, and quiescing/fenced/tombstone states reject ordinary lifecycle changes. Tests cover owner creation, metadata and versioned policy changes, stale rollback-free rejection, admin archive denial, active/archive restoration and protected-state failure. All 41 collaboration-domain tests, warning-denied all-target Clippy, formatting and diff checks passed._

  - [x] 18.3. Implement membership, roles and revocation
    - Project NIP-29 membership, role changes, virtual membership and revocation into common policy inputs.
    - _Requirements: 6.2, 6.4, 9.1_
    - _Capability IDs: CAP-008, CAP-010_
    - _Depends on: 13.5, 18.1, 18.2_
    - _Reads: projects/buzz/crates/buzz-db/src/channel.rs, crates/collaboration_domain/src/admission_evidence.rs_
    - _Writes: crates/collaboration_domain/src/membership.rs_
    - _Validation: membership tests cover invite, role, removal, archive and stale authorization cache_
    - _Evidence: 2026-08-20 — added one versioned membership aggregate for community and NIP-29 channel scopes, projecting persistent community/channel records and owner-attested virtual-agent evidence into the existing common authorization inputs without a second policy model. Verified invite redemptions retain joined-versus-idempotent-retry disposition; virtual inputs remain explicitly distinguishable and cannot become persistence records through this API. Add, role-change, revoke, archive and restore commands validate the exact authorization resource shape, run the shared tenant/scope/current-membership policy first, derive actor attribution rather than trusting it, reject self-mutation, protect community ownership, restrict admins to lower-role membership changes and increment exactly one version per mutation. Revocation is terminal, archive is reversible, and stale cached membership snapshots fail against the new current version. All 46 collaboration-domain tests, warning-denied all-target Clippy, formatting and diff checks passed._

  - [x] 18.4. Implement channel types and lifecycle
    - Add open, private, DM, ephemeral, forum and huddle channel types with archive and expiry semantics.
    - _Requirements: 9.1, 15.2_
    - _Capability IDs: CAP-010, CAP-030, CAP-032_
    - _Depends on: 18.1, 18.3_
    - _Reads: projects/buzz/crates/buzz-db/src/channel.rs, crates/channel/src/channel_store.rs_
    - _Writes: crates/collaboration_domain/src/channel.rs_
    - _Validation: state-transition tests cover each channel type, visibility, archive and ephemeral expiry_
    - _Evidence: 2026-08-20 — added bounded canonical channel names/descriptions, stream/forum/direct-message/workflow/ephemeral/huddle types, open/private visibility, active/archived/expired/deleted lifecycle state and paired nonzero TTL/deadline values. Creation uses community-manage authorization and derives scoped-token attribution from its subject; existing-channel archive/restore/delete commands require the exact common channel authorization resource and current community/channel memberships. Direct messages, workflows and huddles fail closed unless private; explicit ephemeral channels require TTL while Buzz-compatible TTL remains valid on other types. Activity renews TTL with checked time arithmetic, due expiry is deterministic and versioned, restore renews an expired/archived deadline, deletion is terminal, no-op transitions do not advance versions and stale commands cannot mutate. Tests cover every type's legal visibility, invalid DM/ephemeral shapes, archive/restore for all six types, early/due expiry, activity renewal and expired recovery. Warning-denied all-target Clippy, focused tests, formatting and diff checks passed. Zed's legacy integer-ID `ChannelStore` remains unchanged and will consume this domain only through Task 18.6's adapter._

  - [x] 18.5. Implement channel invite lifecycle
    - Add use-limited invites, redemption evidence, expiry and revocation under membership policy.
    - _Requirements: 6.4, 9.1_
    - _Capability IDs: CAP-008, CAP-010_
    - _Depends on: 18.3, 18.4_
    - _Reads: projects/buzz/migrations/0025-*.sql, crates/collaboration_domain/src/membership.rs_
    - _Writes: crates/collaboration_domain/src/channel_invite.rs_
    - _Validation: tests cover invite exhaustion, expiry, revocation, replay and unauthorized redemption_
    - _Evidence: 2026-08-20 — added a canonical hash-only invite aggregate supporting Buzz-compatible community invites and optional native channel targets without storing reusable bearer codes. Mint validates the exact community/channel manage-policy resource, derives scoped-token attribution from its subject, restricts grants to member/guest, enforces 60-second through 30-day lifetimes and the 10,000-use ceiling, and carries explicit active/revoked/exhausted/expired state. Redemption binds tenant plus token hash, checks expiry before existing membership or capacity, rejects foreign/inactive policy inputs, emits canonical community/channel membership records, increments only a genuinely new member, marks the final slot exhausted and preserves use count/version for an existing-member replay. Revocation and expiry are versioned; stale or mismatched bearers fail closed. Focused tests cover final-slot exhaustion, replay without double consumption, expiry ordering, revocation, wrong bearer and foreign-channel membership. All focused tests, warning-denied all-target Clippy, formatting and diff checks passed. Plaintext token generation and serialized `FOR UPDATE` persistence remain adapter/repository responsibilities rather than a second domain authority._

  - [x] 18.6. Integrate canonical channels with native stores
    - Project community/channel/member records into existing ChannelStore and collab UI without a second authority.
    - _Requirements: 2.1, 9.1_
    - _Capability IDs: CAP-010, CAP-036_
    - _Depends on: 18.2, 18.3, 18.4, 18.5, 18.7_
    - _Reads: crates/channel/src/channel_store.rs, crates/collab_ui/src/**, crates/collaboration_domain/src/{community,membership,channel}.rs_
    - _Writes: crates/channel/src/collaboration_store.rs_
    - _Validation: `cargo test -p channel collaboration_store` proves one canonical ID and correct type/role projections_
    - _Evidence: 2026-08-20 — extended the existing native `ChannelStore` with one version-fenced, rebuildable collaboration projection rather than another authority or legacy channel table. Each projected channel is keyed only by its exact `(community UUID, channel UUID)` canonical identity; all six channel types, visibility, lifecycle, topic/canvas metadata, community memberships and channel memberships remain typed domain records. Snapshot replacement is atomic, idempotent at equal version and rejects stale, conflicting, duplicate, unknown-channel and cross-community input without changing the last trustworthy view. Open/private visibility maps to the existing native public/member presentation, owner/admin/member/guest roles map only where Zed's legacy role is semantically safe, and Buzz's separate bot designation deliberately returns an unsupported compatibility result instead of acquiring legacy authority. The focused tests passed 3/3 and the complete channel suite passed 6/6; warning-denied all-target Clippy, all-target check, formatting, collaboration dependency and inventory gates passed. Integration required the narrow `channel` crate dependency/export and `ChannelStore` field/accessor plus corresponding lockfile edges in addition to the planned new adapter file._

  - [x] 18.7. Implement channel templates, topics and canvas metadata
    - Add validated templates plus versioned topic/canvas records under channel write policy.
    - _Requirements: 9.1_
    - _Capability IDs: CAP-010_
    - _Depends on: 18.3, 18.4_
    - _Reads: projects/buzz/desktop/src/features/channel-templates/**, crates/collaboration_domain/src/channel.rs_
    - _Writes: crates/collaboration_domain/src/channel_metadata.rs_
    - _Validation: tests cover template validation, version conflict and unauthorized topic/canvas writes_
    - _Evidence: 2026-08-20 — added bounded canonical channel metadata and validated Buzz-compatible stream/forum templates with open/private visibility, canvas placeholders and deduplicated persona/team references retaining runtime, model, role and local/provider backend selection. Unknown or unmatched placeholders, unsupported channel types, empty/control/oversized reference fields and excess references fail before construction. Versioned topic and canvas writes require the exact channel-shaped common authorization policy; stale versions, insufficient channel roles, timestamp regression and version exhaustion fail without partial mutation, while no-op retries preserve the version. Focused metadata tests passed 2/2 and all collaboration-domain tests passed 54/54; warning-denied all-target Clippy, all-target check, formatting, collaboration dependency, inventory, diff and specification gates passed. The existing Zed `ChannelStore` remains untouched until Task 18.6 projects these values without becoming a second authority._

- [ ] 19. Port messages, threads, reactions and stable channel windows

  - [x] 19.1. Add message and auxiliary-event projection schema
    - Persist messages, edits, deletes, reactions, pins, bookmarks and schedules with provenance and stable sort keys.
    - _Requirements: 9.1, 9.2_
    - _Capability IDs: CAP-005, CAP-011_
    - _Depends on: 18.1_
    - _Reads: projects/buzz/crates/buzz-db/src/{event,thread,reaction}.rs, projects/buzz/migrations/**_
    - _Writes: crates/collab/migrations/collaboration_messages.sql_
    - _Validation: migration test covers same-second keys, uniqueness, tombstones and tenant fences_
    - _Evidence: 2026-08-20 — added a reversible PostgreSQL message projection and one immutable auxiliary-event projection for edits, deletes, reaction add/remove, pin/unpin, bookmark/unbookmark and schedule/create/cancel/publish records. Both tables reference the canonical signed-event log and deliberately store no message content; community-leading channel/principal/event foreign keys, forced RLS with paired permissive/restrictive policies and bounded provenance preserve tenant and source ownership. Message windows use `(message_created_at DESC, source_event_id ASC)` and auxiliary histories use `(event_created_at, auxiliary_event_id)` so tied seconds remain lossless. Lifecycle shape checks retain deletion and removal tombstones, accept delete-before-target projection, bound custom emoji to 4 KiB and require exact schedule and related-event fields. Static/checksum tests passed 3/3. A disposable PostgreSQL 14 test passed in 0.35 seconds, proving tied-second order, source uniqueness, out-of-order tombstone retention, false-tombstone rejection, tenant invisibility/denial and clean rollback; the container was stopped and removed. Focused warning-denied Clippy and check passed with GPUI's existing runtime-shader feature because this host lacks the Xcode Metal Toolchain; formatting, dependency and inventory gates passed. The planned generic write path resolved to timestamped reversible up/down migrations plus the focused migration test._

  - [x] 19.2. Implement message command and edit/delete rules
    - Add authorized create, edit and delete transitions with immutable source history.
    - _Requirements: 9.1_
    - _Capability IDs: CAP-011_
    - _Depends on: 18.4, 19.1_
    - _Reads: projects/buzz/desktop/src/features/messages/**, crates/collaboration_domain/src/channel.rs_
    - _Writes: crates/collaboration_domain/src/{collaboration_domain,message}.rs_
    - _Validation: domain tests cover author/moderator rights, stale edits, delete visibility and retries_
    - _Evidence: 2026-08-20 — added the canonical message aggregate with bounded content and moderation metadata, signed-event source provenance, versioned immutable mutation history, and fail-closed hydration validation. Create, edit and delete commands require exact conversation/channel authorization shapes; owner-attested agent authors require matching proof provenance; only authors or their attested owners can edit; self-deletes require write access while moderation deletes require manage access. Duplicate delivery is idempotent only after authentication, stale or rejected commands remain atomic, deleted content is hidden through the projection API, and moderation reasons remain available for audit. Focused message tests passed 3/3, the complete collaboration-domain suite passed 57/57, warning-denied all-target Clippy passed, and repository formatting passed._

  - [x] 19.3. Implement message reactions
    - Add authorized reaction add/remove and target-deletion behavior, including long custom emoji values.
    - _Requirements: 9.1_
    - _Capability IDs: CAP-011, CAP-017_
    - _Depends on: 19.2_
    - _Reads: projects/buzz/crates/buzz-db/src/reaction.rs, crates/collaboration_domain/src/message.rs_
    - _Writes: .agents/specs/collaborative-workspace/design.md, crates/collaboration_domain/src/{collaboration_domain,message,reaction}.rs_
    - _Validation: tests cover add/remove, long custom emoji, duplicate delivery and target deletion_
    - _Evidence: 2026-08-20 — added a canonical reaction set that reuses message authorization and identity resolution rather than introducing a parallel policy path. Per-actor/per-value add, remove and reactivation transitions retain immutable signed-event sources, contiguous aggregate versions and exact removal references; authenticated duplicate deliveries and already-active adds are no-ops, while stale, malformed, cross-target and unauthorized commands cannot mutate state. Reaction values preserve Buzz's 64-character standard payload limit and canonical wrapped 64-byte shortcode compatibility (66 characters total). Deleted messages hide all active groups and reject new adds without deleting audit history; removals remain representable for reconciliation. Focused reaction tests passed 4/4, the complete collaboration-domain suite passed 61/61, warning-denied all-target Clippy passed, and repository formatting passed._

  - [x] 19.4. Implement NIP-CW thread graph and summaries
    - Build reply ancestry, auxiliary closure, summary and bounded continuation rules from stable IDs.
    - _Requirements: 5.3, 9.1, 9.2_
    - _Capability IDs: CAP-011_
    - _Depends on: 11.9, 19.2, 19.3, 19.8, 19.9_
    - _Reads: projects/buzz/docs/nips/NIP-CW.md, projects/buzz/crates/buzz-db/src/thread.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,thread}.rs_
    - _Validation: golden thread fixtures cover deep replies, deleted roots, aux closure and malformed references_
    - _Discovered contradiction (2026-08-21): the planned standalone domain file cannot compile, expose its graph types or run the required golden fixtures without crate-root registration. The narrow correction adds only the module declaration/public exports and living-spec trace; it adds no transport, persistence, access-scoping, relay codec or UI owner and leaves stable channel-window querying with Task 19.5._
    - _Evidence: 2026-08-21 — added a canonical UI-free NIP-CW graph over immutable event IDs with order-independent ancestry resolution, same-channel parent validation, optional-root agreement, cycle rejection and the Buzz depth-100 ceiling. Deleted events remain structural ancestors while active summaries count direct and nested replies, newest descendant activity and at most ten distinct recent Nostr authors. Thread reads clamp row budgets and continue strictly by `(created_at ASC, event_id ASC)` so tied seconds remain lossless. The auxiliary closure accepts only stable same-channel references, deduplicates IDs, orders deterministically and caps both the reaction/edit/delete row hop and delete-of-aux hop at 1,000 events. Four golden fixtures passed for depth-four ancestry and tied-second paging, deleted roots/replies, two-hop bounded closure, and missing/cross-channel/root/cycle/depth failures; the full collaboration-domain suite passed 72/72, warning-denied all-target/all-feature release Clippy passed, and repository formatting passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 19.5. Implement stable channel and thread query windows
    - Query immutable keyset pages plus live overlay with exact continuation under concurrent writes.
    - _Requirements: 9.2, 8.2_
    - _Capability IDs: CAP-011_
    - _Depends on: 19.1, 19.4_
    - _Reads: crates/collab/migrations/20260820000800_collaboration_messages.up.sql, crates/collaboration_domain/src/thread.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/Cargo.toml, crates/collab/src/{lib,messages}.rs, crates/collab/src/messages/window_repository.rs, crates/collab/src/db/queries/rooms.rs, crates/collab/tests/message_window_repository.rs_
    - _Validation: dense-second and concurrent-live tests return every authorized row exactly once_
    - _Discovered contradiction (2026-08-21): the planned migration read path omits the repository's timestamped reversible migration name, and the standalone repository file cannot compile or run its focused validation without crate-root/module registration and a test target. The existing focused repository tests also import SeaORM's mock harness without enabling its `mock` dev feature, while `rooms.rs` referenced the removed `RequiresSimCla` model column instead of the persisted `RequiresZedCla` column; both pre-existing build blockers surfaced before this leaf's tests could run. The narrow correction registers only the message repository, enables the already-used test harness, corrects that model-column typo and adds the required living-spec/test evidence without taking transport, relay codec, optimistic reconciliation or UI ownership._
    - _Evidence: 2026-08-21 — added authorization-first PostgreSQL channel and recursive thread windows bound to a microsecond snapshot, tenant-scoped transaction and immutable composite cursor. Channel history orders tied seconds by `(message_created_at DESC, source_event_id ASC)`, includes only top-level messages and protocol broadcast replies, reconstructs edit/delete state as of the captured snapshot and probes one extra row for exact continuation. Thread history follows NIP-CW reply tags with a cycle guard and depth-100 ceiling, omits deleted replies while retaining deleted structural roots, and continues strictly by `(message_created_at ASC, source_event_id ASC)`. `StableChannelWindow` keeps live rows outside immutable history, rejects conflicting stable IDs and out-of-chain pages, and removes overlay rows only when a reauthenticated authoritative head or continuation contains them. The focused integration target passed 4/4 fixtures covering authorization before database work, dense same-second channel and recursive-thread pages, exact row-once collection and concurrent live/reconnect reconciliation. Production-feature warning-denied release Clippy, the focused library check, repository formatting, diff hygiene and both canonical specification validators passed. The mandated all-target/all-feature Clippy first stopped on unrelated existing unused imports in `language_model::fake_provider`; the narrower all-feature library probe then exposed pre-existing `collab/test-support` feature wiring that does not enable GPUI/RPC test APIs, while production `collab` completed cleanly._

  - [x] 19.6. Implement optimistic message reconciliation
    - Reconcile stable client operation IDs with accepted, rejected and replaced authoritative events.
    - _Requirements: 8.2, 9.2_
    - _Capability IDs: CAP-011_
    - _Depends on: 15.4, 19.2, 19.5_
    - _Reads: crates/collab_ui/src/**, crates/collab/src/messages/window_repository.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab_ui/Cargo.toml, crates/collab_ui/src/{collab_ui,message_reconciliation}.rs, crates/zed/Cargo.toml_
    - _Validation: tests cover retry, rejection, reconnect, server replacement and no duplicate local echo_
    - _Discovered contradiction (2026-08-21): the planned standalone source file cannot compile, expose the reconciler or run its unit tests without crate-root registration. Compiling it unconditionally would also violate the approved Standard/Multiplayer feature isolation. The narrow correction adds an internal `collab_ui` feature and forwards it from Zed's sole public `multiplayer-tools` feature; the implementation remains a generic client state machine with no collaboration-domain, server, transport, persistence or timeline-rendering dependency._
    - _Evidence: 2026-08-21 — added a stable-operation reconciler with explicit pending, accepted, rejected and reconciled states. Retries reuse one operation row and increment a checked attempt counter; rejection is locally terminal but can be superseded by an authoritative reconnect result; accepted receipts replace optimistic state; and retained old/new event aliases suppress historical and live echoes after server replacement without duplicating the local row. Five focused unit tests passed for retry, rejection recovery, receipt acceptance, reconnect replacement and cross-operation event ownership. Standard and `multiplayer-tools` `collab_ui` library checks passed, as did release all-target/all-feature warning-denied Clippy, repository formatting and diff hygiene. The normal Cargo unit-test target remains blocked before reaching `collab_ui` by the pre-existing test-only `RemoteConnectionOptions::Mock` exhaustiveness failure in `remote_connection`, so the pure `std` module's same five tests were also compiled and run directly with `rustc --test`._

  - [x] 19.7. Render native message and thread timelines
    - Add human/agent messages, replies, edits, reactions and pagination to the common timeline projection.
    - _Requirements: 4.1, 9.1, 9.2, 12.1_
    - _Capability IDs: CAP-011, CAP-025, CAP-036_
    - _Depends on: 19.5, 19.6_
    - _Reads: crates/agent_ui/src/collaborative_timeline.rs, crates/collab_ui/src/message_reconciliation.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, Cargo.lock, crates/collab_ui/Cargo.toml, crates/collab_ui/src/{collab_ui,message_timeline}.rs, crates/workspace/src/status_bar.rs_
    - _Validation: GPUI tests cover pages, live insert, replies, edits, deletion and failed optimistic items_
    - _Discovered contradiction (2026-08-22): the planned standalone file cannot compile, expose a GPUI entity or reuse `CollaborativeTimeline` without crate-root registration plus an optional `agent_ui`/`chrono` feature edge at the first shared consumer. Enabling the already-approved Multiplayer path also exposed a pre-existing duplicate `FocusHandle`/`Role` import in `workspace::status_bar` that made `workspace/multiplayer-tools` uncompilable. The narrow correction extends only `collab_ui/multiplayer-tools`, retains a Standard build with no `agent_ui` dependency, registers the adapter and removes the redundant import without changing status-bar behavior._
    - _Evidence: 2026-08-22 — added a native `MessageTimeline` GPUI entity that transactionally projects immutable newest-first cursor pages, versioned live overlay entries and stable optimistic operations into one chronological `agent_ui::CollaborativeTimeline`. Cursor gaps, repeated history IDs, non-advancing continuations, invalid authors/messages/reactions and conflicting or stale event versions fail before visible state changes. Human and agent messages retain explicit actors; replies retain target links; edits and deterministic reaction groups remain visible semantic detail; deletions replace content in place; and rejected optimistic rows remain failed and visible until authority replaces them without a duplicate echo. Three focused GPUI tests passed for two-page ordering plus live insertion, agent reply/edit/reaction/deletion projection and failed optimistic reconciliation. Standard and Multiplayer release library checks passed; Standard's normal dependency tree excludes `agent_ui`; warning-denied release all-target/all-feature Clippy, repository formatting, diff hygiene and the canonical feature-spec validator passed._

  - [x] 19.8. Implement message pins and private bookmarks
    - Add role-gated pins and viewer-private bookmarks with independent removal behavior.
    - _Requirements: 9.1, 9.3_
    - _Capability IDs: CAP-011, CAP-013_
    - _Depends on: 19.2_
    - _Reads: projects/buzz/desktop/src/features/messages/**, crates/collaboration_domain/src/message.rs_
    - _Writes: .agents/specs/collaborative-workspace/design.md, crates/collaboration_domain/src/{collaboration_domain,message_marker}.rs_
    - _Validation: tests cover pin permissions, bookmark privacy, removal, target deletion and retries_
    - _Evidence: 2026-08-20 — added a canonical per-message marker aggregate with immutable pin, unpin, bookmark and unbookmark source history, exact removal references, contiguous versions and fail-closed hydration. Pins and unpins reuse the common message boundary with manage authority; bookmarks reuse write authority and remain keyed to the authenticated actor. The authorized read projection exposes shared pin state but computes bookmark state only for its authenticated viewer, so another member's private bookmark cannot appear. Pin and bookmark removal remain independent, authenticated duplicate deliveries are idempotent, stale or denied commands are atomic, and deleted targets hide marker state and reject new markers without erasing reconciliation history. Focused marker tests passed 3/3, including role denial, independent viewers/removals, retry and target-deletion cases; the complete collaboration-domain suite passed 64/64, warning-denied all-target Clippy and repository formatting passed, and the feature-spec validator retained 84 acceptance criteria and 385 tasks._

  - [x] 19.9. Implement scheduled-message lifecycle
    - Add create, update, cancel and one-shot due execution under author permissions and bounded recovery.
    - _Requirements: 9.1, 9.3_
    - _Capability IDs: CAP-011, CAP-013_
    - _Depends on: 19.2_
    - _Reads: projects/buzz/desktop/src/features/messages/**, crates/collaboration_domain/src/message.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,message,scheduled_message}.rs_
    - _Validation: timer tests cover edit/cancel, duplicate due, clock skew, restart and denied actor_
    - _Evidence: 2026-08-20 — added the canonical scheduled-message aggregate with author-controlled create, edit and cancellation, bounded clock skew and scheduling horizon, durable one-shot execution claims, finite lease recovery and exact claim/result idempotency. Record hydration validates signed source uniqueness, author/owner mutation authority, contiguous author versions and the exact version implied by claim attempts, so a restarted executor can reclaim only an expired lease and a superseded worker cannot complete it. Message creation authentication is shared with immediate messages, including exact owner-attested-agent proof matching. Focused scheduled-message tests passed 4/4; message-related regressions passed 10/10; the complete collaboration-domain suite passed 68/68; warning-denied all-target Clippy, repository formatting and diff checks passed; the feature-spec validator retained 84 acceptance criteria and 385 tasks with advisory warnings only._

- [x] 20. Port encrypted DMs and visibility projections

  - [x] 20.1. Implement gift-wrap DM codec and privacy gates
    - Parse, validate and emit supported encrypted DM envelopes without exposing plaintext to indexing/logging paths.
    - _Requirements: 5.3, 9.1, 19.2_
    - _Capability IDs: CAP-012_
    - _Depends on: 11.9, 12.6_
    - _Reads: projects/buzz/docs/nips/NIP-DV.md, projects/buzz/crates/buzz-db/src/dm.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/nostr_compat/src/{dm,nostr_compat}.rs_
    - _Validation: codec tests cover round trip, wrong recipient, malformed wrap and plaintext redaction_
    - _Discovered contradiction (2026-08-22): the planned standalone `dm.rs` cannot compile or expose its codec without crate-root registration, and the leaf's required living-documentation evidence adds the specification paths. The named Buzz sources define DM persistence and NIP-DV's gift-wrap privacy relationship but do not implement NIP-44 key custody; Buzz's published compatibility posture supports only the opaque NIP-59 kind-1059 outer envelope. The narrow implementation therefore validates and emits that signed outer envelope, keeps NIP-44 ciphertext opaque and redacted, and supplies filter/result/search gates without claiming encryption, decryption, membership or persistence authority._
    - _Evidence: 2026-08-22 — added a pure `nostr_compat::dm` boundary that verifies the signed kind-1059 event before inspecting it, requires exactly one canonical `p` recipient, validates canonical bounded NIP-44 v2 ciphertext and round-trips the opaque outer envelope without accepting plaintext or keys. Kindless and gift-wrap-capable filters require exactly one authenticated self-`#p`; parsed results independently reject a different reader; indexing is unconditionally excluded; and custom debug output redacts both the encoded ciphertext and a decoded marker. Five focused codec/privacy tests passed for round trip, wrong recipient across filter/result gates, malformed tags/ciphertext/signature and plaintext/ciphertext redaction. The complete `nostr_compat` package passed 54 library tests, four independent Buzz-NIP integration tests and doc tests; warning-denied release all-target/all-feature Clippy, the collaboration dependency-boundary checker, repository formatting and diff hygiene passed; the canonical feature-spec validator retained 84 acceptance criteria and 385 tasks with advisory warnings only. The external Buzz checkout was linked only temporarily to satisfy the existing compile-time fixture path and was removed after validation._

  - [x] 20.2. Implement DM group lifecycle
    - Add open, participant add/remove, leave and reopen transitions with participant-only authority.
    - _Requirements: 6.2, 9.1_
    - _Capability IDs: CAP-010, CAP-012_
    - _Depends on: 18.3, 20.1_
    - _Reads: projects/buzz/crates/buzz-db/src/dm.rs, crates/collaboration_domain/src/membership.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,dm}.rs_
    - _Validation: state tests cover legal membership changes, stale versions and outsider denial_
    - _Discovered contradiction (2026-08-22): the planned standalone `dm.rs` cannot compile, expose its aggregate or run focused tests without crate-root registration, and the living-documentation requirement adds the specification paths. Buzz persists each changed participant set as a new immutable compatibility channel, while the approved leaf explicitly requires canonical add/remove/leave/reopen transitions. The canonical aggregate therefore retains versioned participant history without mutating Buzz source rows; the later protocol/service adapter remains responsible for mapping compatible participant-set representations, and Task 20.3 separately owns per-viewer hide/reopen presentation state._
    - _Evidence: 2026-08-22 — added a UI- and I/O-free `DirectMessage` aggregate with two-to-nine-member open validation, active/left/removed participant states, open/closed lifecycle derivation and a bounded immutable mutation history. Initial open passes the common community write policy and requires the authenticated subject in the participant set; add, remove and leave pass the common channel policy plus an exact active-participant check; self-removal is forced through leave; a voluntary leaver may reopen under current community authorization, while an involuntarily removed participant cannot self-rejoin. Applied commands advance exactly one version, stale or denied commands leave state byte-for-byte unchanged, and hydration rejects noncontiguous, illegal or state-divergent history. Five focused tests passed for legal add/remove/leave/reopen transitions, contiguous versions and hydration, stale-command atomicity, outsider/removed-participant denial, participant bounds, self-removal, idempotent active-add behavior and persisted-state/history divergence. The complete collaboration-domain suite passed 77/77; warning-denied release all-target/all-feature Clippy, the collaboration dependency-boundary checker, repository formatting and diff hygiene passed; the canonical feature-spec validator retained 84 acceptance criteria and 385 tasks with advisory warnings only._

  - [x] 20.3. Persist per-viewer DM visibility
    - Store relay-signed hide/reopen state separately from message deletion and enforce it before counts/results.
    - _Requirements: 6.3, 9.3_
    - _Capability IDs: CAP-012, CAP-013_
    - _Depends on: 19.1, 20.2_
    - _Reads: projects/buzz/docs/nips/NIP-DV.md, crates/collab/migrations/collaboration_messages.sql_
    - _Writes: crates/collab/src/messages/dm_visibility.rs_
    - _Validation: repository tests cover hide, reopen, participant removal and no ID/count leakage_
    - _Discovered contradiction (2026-08-22): the planned unversioned `crates/collab/migrations/collaboration_messages.sql` does not exist; Task 19.1 created the canonical timestamped channel/message migrations, and the channel-membership schema already contains the required tenant-scoped `hidden_at` column. The planned standalone repository cannot compile or expose its API without `messages.rs` registration, its integration-only crate configuration requires a separate test target, and living-documentation evidence adds the specification paths. No new migration is needed: this leaf uses the existing membership field and leaves relay signing/publication to the NIP-DV adapter while producing its complete owner-scoped hidden set._
    - _Evidence: 2026-08-22 — added a PostgreSQL `DmVisibilityRepository` over the existing tenant-fenced channel-membership projection. Hide and reopen reject delegated/private-preference access, pass the common authorization policy before starting a transaction, bind the authenticated participant and expected membership version, require an active canonical DM and respectively set or clear only `hidden_at`; message projections and deletion tombstones are never queried or mutated. Snapshot reads derive the viewer from the authorized community membership, select only that viewer's active hidden DM memberships in deterministic ID order and expose count only as the length of the authorized set, so removed participants and denied callers receive neither IDs nor a separately observable count. Five focused integration tests passed for hide, reopen, removed-participant rejection, active owner-scoped snapshot filtering and pre-database denial with an empty transaction log. Warning-denied release Clippy passed for the `collab` library, the collaboration dependency boundary, 31-package/184-protocol/62-data-source/193-surface inventory, Rust formatting, diff hygiene and the canonical feature-spec validator passed. The required all-target Clippy wrapper additionally reached two pre-existing unused imports in `crates/language_model/src/fake_provider.rs`; those unrelated base warnings remain untouched._

  - [x] 20.4. Render native DM navigation and timeline
    - Add authorized DM rows, participants and encrypted-message failure states using canonical records.
    - _Requirements: 4.1, 9.1, 9.3_
    - _Capability IDs: CAP-012, CAP-036_
    - _Depends on: 19.7, 20.3_
    - _Reads: crates/collab_ui/src/message_timeline.rs, crates/sidebar/src/collaborative_navigation.rs_
    - _Writes: crates/collab_ui/src/dm_view.rs_
    - _Validation: GPUI tests cover open, hide, decrypt failure, removed participant and reconnect_
    - _Discovered contradiction (2026-08-22): the planned standalone `dm_view.rs` cannot compile, expose its view or use the canonical DM aggregate without crate-root registration plus a feature-gated `collaboration_domain` dependency and lockfile update; focused GPUI tests also require their existing crate test-support feature and a test-only UUID dependency, while living-documentation evidence adds the specification paths. The named sidebar source supplies navigation conventions but this leaf does not authorize sidebar composition writes. The narrow correction therefore exposes an authorized navigation-row model from the native DM view and leaves placement in the application shell to its owning composition task, while reusing rather than duplicating the canonical message timeline._
    - _Evidence: 2026-08-22 — added a feature-gated native `DmView` that constructs only for an active canonical `DirectMessage` participant, derives the complete participant list and authorized navigation row from that record, and hides or reopens only the navigation projection without discarding timeline state. The view delegates page ordering and reconciliation to `MessageTimeline`; reconnect builds and validates a replacement authoritative timeline before swapping it, while a removed viewer atomically loses navigation, participants, decrypt failures and timeline access. Closed missing-key, malformed-envelope and unsupported-version failures retain only event, sender and time metadata and never accept ciphertext or plaintext. `cargo test -p collab_ui --lib --features multiplayer-tools,test-support dm_view -- --nocapture` passed all five open, hide/reopen, decrypt-failure, removed-participant and atomic-reconnect GPUI scenarios. Warning-denied release/all-target/all-feature `./script/clippy -p collab_ui --features multiplayer-tools,test-support`, production and feature-disabled library checks, the collaboration dependency boundary, 31-package/184-protocol/62-data-source/193-surface inventory, Rust formatting, diff hygiene and the canonical feature-spec validator passed._

  - [x] 20.5. Add independent DM privacy conformance tests
    - Probe event, filter, count, search, notification and logs as participant and nonparticipant.
    - _Requirements: 6.3, 9.3, 20.2_
    - _Capability IDs: CAP-012, CAP-015, CAP-016, CAP-044_
    - _Depends on: 20.3, 20.4_
    - _Reads: projects/buzz/crates/buzz-conformance/**, projects/buzz/{crates/buzz-push-gateway,docs/nips/NIP-PL.md}, crates/nostr_compat/src/{dm,filter}.rs, crates/collab/src/messages/dm_visibility.rs, crates/collab/migrations/20260820000500_collaboration_search.up.sql_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/tests/dm_privacy_conformance.rs_
    - _Validation: two-user/two-community suite reports no plaintext, existence, count or search leak_
    - _Discovered contradiction (2026-08-22): notification eligibility, native dispatch and the canonical push gateway are intentionally owned by Tasks 22.4–22.9, while live search queries land in Tasks 22.1–22.2, so this earlier conformance leaf cannot truthfully drive nonexistent production notification/search adapters or add test-local authorities. The dependency-safe correction exercises the existing signed-event, filter, visibility repository, indexing, storage-schema and redaction boundaries, and freezes Buzz's byte-exact recipient-only APNs reconnect body as an independent observation. Task 22.11 remains responsible for driving the same oracle from the completed live search and push paths._
    - _Evidence: 2026-08-22 — added a checker-owned DM privacy trace covering all 24 combinations of two communities, participant/nonparticipant and event/filter/count/search/notification/log seams. Alice is the authorized recipient in one community and the nonparticipant in the other, with Bob reversed; real signed kind-1059 envelopes accept only their exact recipient, filters require one exact self `p` tag, and owner-scoped `DmVisibilityRepository` snapshots return deliberately different authorized counts while the other member's own empty set yields zero IDs and zero count. The audit also requires the production query to retain community/viewer predicates and derive count without a separate `COUNT` query. The search probe combines unconditional gift-wrap indexing exclusion with the generated-column kind allowlist, logs exercise codec/ciphertext debug redaction plus repository traces, and participant notifications remain byte-identical wake-only bodies while nonparticipants receive none. Independent mutation tests reject plaintext, existence, count/ID, search and wake leaks plus missing/duplicate seam coverage. The focused test passed 3/3, warning-denied no-dependency Clippy passed for the new target, Rust formatting and diff hygiene passed. The required all-target Clippy wrapper additionally reached two pre-existing unused imports in `crates/language_model/src/fake_provider.rs`; those unrelated base warnings remain untouched._

- [x] 21. Merge read state, reminders, drafts, presence and typing

  - [x] 21.1. Implement encrypted read and manual-unread state
    - Merge cross-device frontiers and manual overrides under NIP-RS ordering and privacy rules.
    - _Requirements: 9.3_
    - _Capability IDs: CAP-013_
    - _Depends on: 11.9, 19.5_
    - _Reads: projects/buzz/docs/nips/NIP-RS.md, crates/channel/src/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, Cargo.lock, crates/collaboration_domain/Cargo.toml, crates/collaboration_domain/src/{collaboration_domain,read_state}.rs_
    - _Validation: property tests cover monotonic frontier, override, tombstone and concurrent devices_
    - _Discovered contradiction (2026-08-22): the planned standalone domain file cannot compile, expose its public aggregate or run genuine property tests without crate-root registration and the workspace's existing test-only `proptest` dependency, so the narrow write set includes those manifest/lock/root changes and living-spec trace. NIP-44 encryption/decryption, encrypted wire parsing, relay full-state enumeration/fencing, persistence and key custody remain with their approved adapter and service owners; the domain accepts only explicitly owner-bound decrypted replicas and does not add a second codec or crypto implementation._
    - _Evidence: 2026-08-22 — added a community-and-owner-scoped read-state aggregate whose debug surfaces redact contexts and whose reads, mutations and replica joins reject foreign principals or communities. Owner-decrypted replicas normalize duplicate frontiers/registers by componentwise maximum, require manual overrides to remain co-located with their frontiers and enforce the NIP-RS context bounds. Complete-load state advances monotone channel/thread/message frontiers, folds optional parent frontiers, applies clear-wins manual-unread actions, preserves natural frontier progress when counters are exhausted, and emits only live triples or permanent counter-floor tombstones; potentially incomplete loads remain readable but cannot mutate or publish. Four generated property families cover frontier monotonicity, override semilattice laws, tombstone resurrection resistance and concurrent-device delivery order, with focused unit cases for completeness, privacy, hierarchy, canonical publication, exhaustion and debug redaction. Eight focused tests and the full 85-test collaboration-domain suite passed, as did warning-denied release all-target/all-feature Clippy, the focused crate check, Rust formatting, dependency-boundary and inventory validation, spec validation and diff hygiene. Commit: enclosing leaf commit, reported after creation._

  - [x] 21.2. Persist local drafts without server authority
    - Key drafts by canonical community/channel/thread identity and retain them through offline/restart transitions.
    - _Requirements: 9.3_
    - _Capability IDs: CAP-013_
    - _Depends on: 18.6, 19.7_
    - _Reads: crates/db/**, crates/collab_ui/src/message_timeline.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab_ui/Cargo.toml, crates/collab_ui/src/{collab_ui,draft_store}.rs_
    - _Validation: draft tests cover restart, channel deletion, account switch and no cross-community reuse_
    - _Discovered contradiction (2026-08-22): the planned standalone file cannot be compiled or tested without registering it at the feature-gated crate root and enabling the existing database test-support feature for `collab_ui` tests, so the narrow write set includes those manifest/root and living-spec changes. The current timeline presentation context exposes display strings rather than Task 18.6's canonical channel identity; making those strings persistence authority would violate this task, so the store accepts a separately typed canonical location and leaves composer-awareness wiring to Task 21.6._
    - _Evidence: 2026-08-22 — added a local-only draft store fixed to one canonical principal and scoped by typed community, channel and optional thread-root event identities. Strict versioned JSON lives only in Zed's native SQLite key-value store; no client, RPC, outbox, event or relay path is imported. Synchronous loads restore drafts through offline/restart transitions, asynchronous writes propagate failures, whitespace clears the exact draft and channel deletion removes only that account/community/channel namespace. Unsent content is redacted from debug output. Five focused tests passed for channel and thread restart retention, channel deletion isolation, account switching and switch-back recovery, cross-community channel-ID isolation, blank clearing and diagnostic redaction. The no-default-feature library check and warning-denied release Clippy with multiplayer/test-support features passed, as did Rust formatting, diff hygiene, collaboration dependency-boundary validation, Buzz inventory validation and canonical spec validation._

  - [x] 21.3. Implement reminder lifecycle and due recovery
    - Add create, update, dismiss and due-after-offline behavior under NIP-ER privacy/retention rules.
    - _Requirements: 9.3, 15.2_
    - _Capability IDs: CAP-013, CAP-030_
    - _Depends on: 11.9, 19.3_
    - _Reads: projects/buzz/docs/nips/NIP-ER.md, projects/buzz/desktop/src/features/reminders/**, crates/nostr_compat/src/buzz_nips/communication.rs, crates/collaboration_domain/src/{read_state,scheduled_message}.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,reminder}.rs_
    - _Validation: timer tests cover clock skew, restart, duplicate due and expired target_
    - _Discovered contradiction (2026-08-22): the planned standalone domain file cannot compile or expose its public aggregate without crate-root registration and living-spec trace, so the narrow write set includes those files. `collaboration_domain` is prohibited from importing protocol, storage, transport or GPUI owners; the aggregate therefore consumes explicitly owner-bound replicas after Task 11.9's signature/tag/plaintext checks, evaluates deterministic timestamps supplied by its caller and leaves encryption, persistence, timer wakeups, notifications and terminal-event publication to their approved adapters. Target expiry is supplied by the canonical visibility/retention owner rather than inferred from private reminder text or used as authorization._
    - _Evidence: 2026-08-22 — added a community-and-owner-scoped NIP-ER reminder aggregate with create, pending update/snooze, done/cancelled dismissal, strict author-only reads and mutations, lowest-event-id replacement tie breaking and idempotent replica convergence. Local due evaluation never fires before `not_before`, persists the handled source head across record reconstruction, suppresses duplicate deliveries and expired reminder/target state, retries temporarily unavailable targets and clears delivery state only for a newer winning replacement. Terminal local transitions require Buzz's 30–90 day cleanup window and cannot reuse the old reminder address; corrupted handled records and conflicting same-event replicas fail closed, while diagnostics redact private identifiers, targets, notes, schedules and handled event IDs. Six focused timer/lifecycle/privacy tests and the full 91-test collaboration-domain suite passed, as did warning-denied release all-target/all-feature Clippy, Rust formatting, diff hygiene, collaboration dependency-boundary validation, Buzz inventory validation and canonical spec validation._

  - [x] 21.4. Implement canonical presence projection
    - Merge signed and room presence by source/expiry without allowing presence to grant authorization.
    - _Requirements: 9.3, 11.5_
    - _Capability IDs: CAP-014, CAP-034_
    - _Depends on: 16.3, 18.3_
    - _Reads: projects/buzz/NOSTR.md, projects/buzz/docs/remote-agents.md, projects/buzz/crates/buzz-pubsub/src/presence.rs, projects/buzz/crates/buzz-relay/src/api/bridge.rs, projects/buzz/crates/buzz-relay/src/handlers/event.rs, projects/buzz/crates/buzz-sdk/src/builders.rs, crates/collab/src/freshness.rs, crates/collab/src/db/tables/room_participant.rs, crates/collab/src/db/tables/channel_chat_participant.rs, crates/collaboration_domain/src/authorization.rs, crates/collaboration_domain/src/membership.rs_
    - _Writes: .agents/specs/collaborative-workspace/design.md, .agents/specs/collaborative-workspace/tasks.md, crates/collaboration_domain/src/collaboration_domain.rs, crates/collaboration_domain/src/presence.rs_
    - _Validation: presence tests cover forged state, TTL expiry, multiple sources and revoked membership_
    - _Discovered contradiction (2026-08-22): the planned standalone domain file cannot compile or expose its public projection without crate-root registration and living-spec trace, so the narrow write set includes those files. Signature verification and account-binding resolution remain adapter responsibilities represented by the verified-event constructor; room authentication, clocks, storage, fan-out and UI timers likewise remain outside the I/O-free domain. Both source types use explicit bounded expiry, and presence consumes current membership but exposes no authorization decision._
    - _Evidence: 2026-08-22 — added a community/principal/key-bound canonical presence projection that accepts only exact-author adapter-verified signed events and same-scope authenticated room observations. Independent signed-event ordering and per-room sequences retain offline/disconnect tombstones, reject conflicts and ignore delayed resurrection; online wins over away across active sources, clearing or expiry affects only that source, and the earliest remaining expiry drives freshness refresh. Signed and room liveness are capped at Buzz's 180-second TTL. Inactive membership rejects updates, archive/revocation clears all sources and terminal revocation cannot reactivate; the projection never creates membership or returns an authorization decision. Five focused tests cover forged/cross-tenant state, exact TTL boundaries and excess TTL, multiple-source merge/clear/expiry, stale resurrection and revoked membership. The full 96-test collaboration-domain suite passed, as did warning-denied release all-target/all-feature Clippy, Rust formatting, diff hygiene, collaboration dependency-boundary validation, Buzz inventory validation through the supplied external checkout and canonical spec validation._

  - [x] 21.5. Implement bounded typing indicators
    - Accept signed typing events, enforce channel access and expire them without persistence.
    - _Requirements: 5.1, 8.4, 9.3_
    - _Capability IDs: CAP-006, CAP-014_
    - _Depends on: 16.2, 18.4, 21.4_
    - _Reads: projects/buzz/crates/buzz-pubsub/src/lib.rs, projects/buzz/crates/buzz-pubsub/src/rate_limiter.rs, projects/buzz/crates/buzz-acp/src/relay.rs, projects/buzz/crates/buzz-test-client/tests/conformance_multitenant.rs, .agents/specs/collaborative-workspace/security/operational-limits.md, .agents/specs/collaborative-workspace/security/tenant-identity.md, crates/collab/src/nostr/event_ingest.rs, crates/collab/src/tenant_admission.rs, crates/collab/src/db/collaboration/persistence_policy.rs, crates/collaboration_domain/src/authorization.rs, crates/collaboration_domain/src/presence.rs_
    - _Writes: .agents/specs/collaborative-workspace/design.md, .agents/specs/collaborative-workspace/tasks.md, crates/collab/src/lib.rs, crates/collab/src/presence.rs, crates/collab/src/presence/typing.rs, crates/collab/tests/collaboration_typing.rs_
    - _Validation: typing tests cover rate limit, unauthorized sender, expiry, reconnect and zero durable rows_
    - _Discovered contradiction (2026-08-22): the planned nested module needs the minimal crate-root and parent-module registrations plus a focused integration-test target to compile, expose and verify the contract, so the actual write set includes those files and the living design trace. Buzz advertises Redis typing state but contains no typing module and the authoritative data catalog records `REDIS-TYPING-GAP-001`; this leaf therefore implements the approved collab-owned in-memory derived state and relies on the existing generated ephemeral persistence policy to prove zero SQL/search rows. Cross-replica live transport remains an adapter concern and cannot turn typing into replayable authority._
    - _Evidence: 2026-08-22 — added an in-memory, tenant/channel/principal/connection-generation keyed typing store for verified kind-20002 events. Admission requires an exact authenticated author, one canonical channel tag, current direct channel membership through the common authorization policy, a current connection generation and a valid bounded signature timestamp; delegated authority, forged/tampered events, inactive membership, stale generations and stale event order fail closed. Independent device sources collapse by principal on reads, reconnect replaces only that connection generation, disconnect removes the exact source, duplicates cannot refresh expiry, and active state expires at the normative 60-second boundary. A per-principal token bucket enforces two updates per second with burst ten, while explicit connection/retained-entry caps and 120-second replay-floor pruning bound memory. Five focused tests passed for burst/refill limiting, unauthorized and tampered senders, expiry/duplicate/reconnect/disconnect behavior, two-device independence and zero durable database transactions with transient/search-excluded classification. `cargo clippy -p collab --lib --release -- --deny warnings`, Rust formatting, collaboration dependency boundaries, Buzz inventory validation through the supplied external checkout and diff hygiene passed. The required all-target repository Clippy wrapper remains blocked before this leaf by pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`; an all-features library probe separately reaches pre-existing collab test-support methods unavailable in that feature combination._

  - [x] 21.6. Integrate awareness state into native navigation
    - Render unread, reminder, presence and typing state with offline/freshness indicators in existing rows.
    - _Requirements: 4.3, 8.3, 9.3_
    - _Capability IDs: CAP-013, CAP-014, CAP-036_
    - _Depends on: 21.1, 21.2, 21.3, 21.4, 21.5_
    - _Reads: crates/sidebar/src/collaborative_navigation.rs, crates/sidebar/src/collaborative_rail.rs, crates/sidebar/src/collaborative_pinned.rs, crates/sidebar/src/collaborative_projects.rs, crates/sidebar/src/collaborative_tasks.rs, crates/collab_ui/src/draft_store.rs, crates/collaboration_domain/src/read_state.rs, crates/collaboration_domain/src/reminder.rs, crates/collaboration_domain/src/presence.rs, crates/collab/src/presence/typing.rs, .agents/specs/collaborative-workspace/design.md_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/sidebar/src/sidebar.rs, crates/sidebar/src/collaborative_awareness.rs, crates/sidebar/src/collaborative_pinned.rs, crates/sidebar/src/collaborative_projects.rs, crates/sidebar/src/collaborative_tasks.rs_
    - _Validation: GPUI tests cover multi-device updates, offline stale state, reconnect and expiry_
    - _Discovered contradiction (2026-08-22): the planned standalone awareness file cannot compile, expose its adapter API or render in existing navigation rows without crate-root registration and the three existing pinned/project/task render call sites, so the narrow write set includes those files and the living design trace. The focused multiplayer build also exposed a pre-existing `Vec`/`and_then` mismatch in task activation; the correction selects the first projected workspace with `into_iter().next()` and does not change ownership. The sidebar projection accepts only already-authoritative, adapter-supplied durable and ephemeral state and adds no domain, protocol, authorization or persistence dependency._
    - _Evidence: 2026-08-22 — added a bounded GPUI awareness store keyed by existing navigation source identity, with redacted target, participant, token and update diagnostics. Per-source generations and sequences fence stale reconnects and conflicting duplicates; newer durable revisions win across devices while presence and typing merge by participant, online wins away, exact-source disconnect clears ephemeral state and reconnect renders recovery without discarding last trustworthy unread/reminder state. Pinned, project/worktree/repository and task/thread rows render capped unread, reminder, online/away and typing badges plus fresh, stale, reconnecting or offline retry state under a semantic status/accessibility label. GPUI timers drive 180-second-bounded presence, 60-second-bounded typing and reminder-due transitions. Four focused awareness tests passed, the expiry test passed across 20 scheduler seeds, and related navigation/pinned/projects/tasks suites passed 7/5/4/4 tests. `cargo check -p sidebar --features multiplayer-tools --lib`, warning-denied release library Clippy with `--no-deps`, Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory validation through the supplied external checkout and canonical spec validation passed. The required all-target repository Clippy wrapper remains blocked before this leaf by pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`._

- [ ] 22. Integrate search, native notifications and NIP-PL push

  - [x] 22.1. Project authorized collaboration content into search
    - Consume authoritative outbox records and update only policy-approved search documents.
    - _Requirements: 9.4, 15.2_
    - _Capability IDs: CAP-015, CAP-030_
    - _Depends on: 16.4, 19.1, 20.3_
    - _Reads: crates/collab/src/db/collaboration/outbox.rs, crates/collab/migrations/20260820000500_collaboration_search.up.sql, projects/buzz/crates/buzz-search/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/src/search.rs, crates/collab/src/search/indexer.rs, crates/collab/tests/collaboration_search_indexer.rs_
    - _Validation: indexer tests cover edit/delete/retention, DM exclusion and idempotent replay_
    - _Discovered contradiction (2026-08-22): the planned `collaboration_search.sql` path is stale; Task 16.4 already established the reversible timestamped search migration and prohibited duplicating signed-event transcript content into a second projection. The narrow correction consumes only a strict canonical-document outbox topic into that existing table and includes the minimal module registration, focused integration target and living-spec trace needed to compile and verify the new indexer. Signed-event search remains generated on immutable event authority, and delivery scheduling remains owned by the existing outbox/fan-out adapters._
    - _Evidence: 2026-08-22 — added a PostgreSQL-only search indexer that resolves the exact authoritative outbox sequence under transaction-local tenant RLS, rejects malformed or unsupported projection contracts and ignores unrelated topics. A closed version-one payload accepts bounded community-visible canonical documents or content-free excluded tombstones for delete, retention expiry, restricted visibility and direct-message cases. The durable outbox sequence fences each document update and its big-endian projection cursor; conditional upserts suppress duplicate and delayed replay, preserve deletion floors and commit a clean `collaboration_search` checkpoint atomically with each accepted change. Four focused integration tests cover ordered edits, delete and retention tombstones, direct-message exclusion without private content and idempotent replay without a second checkpoint. The collab library check, Rust formatting, diff hygiene, Buzz inventory validation through the supplied external checkout and canonical spec validation passed. The required all-target release Clippy wrapper remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`._

  - [x] 22.2. Implement collaboration search queries
    - Query authorized community, channel, member, project and message result classes with freshness metadata.
    - _Requirements: 6.3, 9.4_
    - _Capability IDs: CAP-015_
    - _Depends on: 16.5, 22.1_
    - _Reads: crates/collab/src/search/{repository,indexer}.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/migrations/20260822000100_collaboration_search_channels.{up,down}.sql, crates/collab/src/search.rs, crates/collab/src/search/{query,repository,indexer}.rs, crates/collab/tests/{collaboration_search_query,collaboration_search_repository}.rs_
    - _Validation: query tests apply policy before rank/limit and return stable result identities_
    - _Discovered contradiction (2026-08-22): Task 16.4's canonical-document schema and Task 22.1's strict outbox contract omitted `channel`, so a query-only facade could declare but never return one of this leaf's required result classes. Signed profile hits also lacked the author key needed for a stable replaceable-member identity. The narrow correction adds one reversible derived-projection constraint extension, admits channel documents through the existing strict indexer contract and carries the already-authoritative event author through repository references; it does not create a channel, profile or message authority or copy indexed content into results._
    - _Evidence: 2026-08-22 — added a typed query facade over the existing policy-first repository for community, channel, member, project, message and additional canonical resource classes. Canonical identities bind class plus source system/record while deliberately ignoring source version, replaceable signed profiles bind the author public key and messages bind the immutable event ID; results retain only refetch references, rank, observation time, page and current/lagging/unavailable projection freshness. A reversible derived-schema extension admits channel documents, with rollback temporarily relaxing owner bypass only long enough to remove rebuildable channel rows before restoring forced RLS and the prior constraint. Three focused query tests prove denied requests perform zero database work, visibility policy precedes rank and limit, all five required classes are returned, canonical/member identity survives version/event replacement and freshness is preserved. Adjacent repository and indexer suites passed 4/4 each, and the collab library check, warning-denied focused library Clippy, Rust formatting, dependency boundary, Buzz inventory, diff hygiene and canonical spec gates passed. The required all-target release Clippy wrapper remains blocked before this leaf by pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`._

  - [x] 22.3. Compose collaboration results in native search UI
    - Add typed collaboration groups alongside existing file/project search with scope and freshness labels.
    - _Requirements: 4.4, 9.4_
    - _Capability IDs: CAP-015, CAP-036_
    - _Depends on: 22.2_
    - _Reads: crates/search/src/**, crates/collab/src/search/query.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/search/src/search.rs, crates/search/src/collaboration_search.rs, crates/search/tests/collaboration_search.rs_
    - _Validation: `cargo test -p search --test collaboration_search -- --nocapture` covers keyboard flow, empty, stale and unauthorized results_
    - _Discovered contradiction (2026-08-22): a standalone source file cannot compile, expose its GPUI boundary or run the named scenarios without crate-root registration and an integration target. Making desktop `search` depend directly on server-oriented `collab` would also pull persistence and infrastructure dependencies across the approved boundary. The narrow correction keeps canonical refetch, authorization and query execution in the upper collaboration adapter and accepts only reference-safe presentation rows; native search owns grouping and interaction but no collaboration authority or content store. The generic crate-filter command is blocked before selecting this leaf's tests by two pre-existing `tests` modules in `crates/search/src/text_finder/delegate.rs`, so the exact isolated target proves this leaf without modifying unrelated code._
    - _Evidence: 2026-08-22 — added a native GPUI collaboration-search view that stably groups existing file/project rows with typed community, channel, member, collaborative-project, message, repository, task, agent, workflow and media rows. Authorized presentation state renders an explicit community scope and current, lagging or unavailable freshness; empty and unauthorized states expose no collaboration rows, with malformed cross-class inputs filtered closed. Next/previous actions wrap through the combined result order, confirmation emits the stable native or collaboration identity and refresh preserves selection across label/version changes. Four focused tests passed, and the keyboard flow passed 20 deterministic GPUI scheduler seeds. The search library check, warning-denied focused library Clippy, Rust formatting, dependency boundary, Buzz inventory, diff hygiene and canonical spec gates passed. The required all-target release Clippy wrapper remains blocked before this leaf's test target by the pre-existing duplicate `tests` module in `crates/search/src/text_finder/delegate.rs`._

  - [x] 22.4. Define notification eligibility and deduplication policy
    - Decide native/push eligibility from mentions, membership, mute, read state, device permissions and stable source IDs.
    - _Requirements: 9.5_
    - _Capability IDs: CAP-016_
    - _Depends on: 18.3, 21.1_
    - _Reads: projects/buzz/desktop/src/features/notifications/**, crates/notifications/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/collaboration_domain.rs, crates/collaboration_domain/src/notification_policy.rs_
    - _Validation: policy table tests cover self, mute, read, duplicate, revoked and private events_
    - _Discovered contradiction (2026-08-22): the planned standalone policy file cannot compile or expose its domain API without crate-root registration, and Buzz's bounded local-storage seen set cannot be adopted as canonical cross-device deduplication authority. The narrow correction defines a pure stable-key policy with a caller-supplied delivered lookup; native and later durable push adapters retain their approved storage and record only successful deliveries. The policy accepts current typed membership/privacy/read/permission snapshots and owns no content, persistence, transport or permission prompt._
    - _Evidence: 2026-08-22 — added a content-free notification candidate and stable delivery identity keyed by community, canonical source system/record, recipient and native/push surface. Common gates require current active community and applicable channel membership bound to the exact tenant, recipient and channel, then require private participation, suppress self-authored and already-read work and fail closed when read state is unavailable. Personal mute suppresses ordinary activity while preserving Buzz's mention override only after access checks. Native and push permissions resolve independently across granted, disabled, denied, revoked and unsupported states; a caller-supplied canonical lookup suppresses only the already-delivered surface, and source records are redacted from diagnostics. Six focused tests passed, including a policy table for self, mute, read, duplicate, revoked and private events plus stable-source identity and permission behavior; the full collaboration-domain suite passed 102/102. The library check, required all-target release Clippy wrapper with warnings denied, Rust formatting, diff hygiene, collaboration dependency boundary, Buzz inventory and canonical spec validation passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.5. Dispatch native desktop notifications
    - Convert eligible records to existing native notifications and navigate safely to canonical entities.
    - _Requirements: 9.5, 16.4_
    - _Capability IDs: CAP-016, CAP-042_
    - _Depends on: 22.4_
    - _Reads: crates/notifications/**, crates/collaboration_domain/src/notification_policy.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, Cargo.lock, crates/notifications/Cargo.toml, crates/notifications/src/notifications.rs, crates/notifications/src/collaboration.rs_
    - _Validation: `cargo test -p notifications --features multiplayer-tools collaboration_notification -- --nocapture` covers permission denial, deduplication, redacted preview and missing deep-link target_
    - _Discovered contradiction (2026-08-22): the planned file cannot compile or expose its adapter without crate-root registration and optional dependency wiring, while the canonical `CollaborativeNavigationTarget` is deliberately gated behind `multiplayer-tools`; enabling it in default notifications would violate the compile-time capability boundary. The narrow correction gates the adapter and its domain/hash dependencies behind the same feature. GPUI's system-notification API is intentionally fire-and-forget and exposes no operating-system delivery receipt, so the adapter reports an application post and leaves canonical successful-delivery recording to the caller rather than fabricating confirmation. Entity availability remains owned by the injected canonical navigator, not the notification crate._
    - _Evidence: 2026-08-22 — added a multiplayer-gated native dispatcher that consumes Task 22.4 policy candidates and caller-owned delivered lookups, posts only eligible native decisions through GPUI `SystemNotification` and returns the stable delivery identity for canonical recording. Public titles/bodies are nonempty, control-safe and bounded to 128/512 bytes; private records accept no caller preview and always display fixed redacted text. SHA-256 tags expose no raw community, recipient or source record. A bounded 500-entry one-shot activation map carries the existing typed collaborative-navigation target to an injected resolver, ignores unknown actions and treats a disappeared entity as a safe no-op. Four focused GPUI tests passed for denied permission, stable-key duplicate suppression, redacted private preview and missing-target activation; the missing-target path passed 20 deterministic scheduler seeds. Default and multiplayer library checks, focused warning-denied Clippy, the required all-target release Clippy wrapper, Rust formatting, diff hygiene, the collaboration dependency boundary, Buzz inventory and canonical spec validation passed. The initially full filesystem was recovered by removing only 46.1 GiB of this worktree's regenerable Rust `target` cache before rebuilding. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.6. Define the canonical push-lease domain
    - Model capability-bound device leases, generations, expiry and revocation independently of wire/provider code.
    - _Requirements: 9.5_
    - _Capability IDs: CAP-016_
    - _Depends on: 22.4_
    - _Reads: projects/buzz/docs/nips/NIP-PL.md, projects/buzz/crates/buzz-push-gateway/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/collaboration_domain.rs, crates/collaboration_domain/src/push_lease.rs_
    - _Validation: `cargo test -p collaboration_domain push_lease -- --nocapture` covers generation, expiry, revocation, wrong capability and the wake-only payload invariant_
    - _Discovered contradiction (2026-08-22): the planned standalone file cannot compile, expose its domain API or run its tests without crate-root registration. NIP-PL wire parsing already belongs to `nostr_compat`, while dual addressable-event ordering and atomic watermark persistence belong to later `collab` leaves; duplicating either here would create competing authority. The narrow correction registers only a provider-neutral aggregate over adapter-validated installation identity, opaque capability digest, lease/endpoint generations and time, with no signed-event, storage, matcher, raw grant or provider dependency._
    - _Evidence: 2026-08-22 — added a community/owner/installation-scoped push lease with redacted installation and capability diagnostics, nonzero typed lease and endpoint generations, strict newer-generation replacement, expiry fencing, higher-generation revocation and replay-safe watermark retention. Wake admission rechecks active state, inclusive expiry, exact lease generation, endpoint generation and capability reference before returning a routing record whose only serializable application state is the fixed `reconnect` signal. A tombstone retains the last active expiry and a higher generation may reactivate without affecting sibling addresses. Six focused tests passed for community scope, generation replacement, expiry, revocation/reactivation, wrong capability and the wake-only payload invariant; the complete collaboration-domain suite passed 108/108. The library check, focused warning-denied Clippy, required all-target release Clippy wrapper, Rust formatting, diff hygiene, collaboration dependency boundary, Buzz inventory and canonical spec validation passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.7. Add push-lease and wake-outbox schema
    - Create tenant/device-scoped encrypted lease and idempotent wake-job tables.
    - _Requirements: 9.5, 17.2_
    - _Capability IDs: CAP-005, CAP-016_
    - _Depends on: 22.6_
    - _Reads: projects/buzz/migrations/{0012_push_leases,0013_push_endpoint_state,0018_push_match_queue,0022_event_ttl_refresh,0023_push_match_gate}.sql_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/migrations/20260822000200_collaboration_push.{up,down}.sql, crates/collab/tests/collaboration_push_migration.rs_
    - _Validation: `cargo test -p collab --test collaboration_push_migration -- --nocapture` covers encryption columns, generation uniqueness, tenant fences and rollback_
    - _Discovered contradiction (2026-08-22): the listed Buzz `0022` migration is unrelated event-TTL refresh and `0023` contains only the later lease-aware match gate; the authoritative lease/outbox schema is `0012`, endpoint disablement is `0013` and durable match work starts in `0018`. The planned single unversioned SQL file would also bypass this crate's SQLx reversible migration convention. The narrow correction reads those actual owners and adds one timestamped up/down pair plus a focused migration test. The schema references the already-canonical signed event instead of duplicating its NIP-44 ciphertext, while encrypting the effective capability and subscription material needed by later matching/delivery leaves._
    - _Evidence: 2026-08-22 — added an effective push-lease table keyed by community, owner and installation, with immutable signed-event provenance, safe positive lease/endpoint generations, mutually exclusive active/tombstone shapes, retained last-active expiry, endpoint disablement state, bounded encrypted capability/subscription envelopes and a redacted capability digest. Added wake jobs with stable request and capability/source-event uniqueness, exact lease/event foreign keys, closed pending/leased/delivered/failed/suppressed state, finite claim fields and no application payload, content, preview, URL or ciphertext column. Both tables use tenant-leading indexes plus forced restrictive RLS; the exact rollback removes wake jobs before leases without `CASCADE`. Five focused tests passed for encryption/no-payload shape, generation and idempotency constraints, RLS/FKs, reversible checksums and rollback; the optional isolated PostgreSQL execution branch was skipped because `COLLAB_PUSH_MIGRATION_TEST_DATABASE_URL` is unset. Focused warning-denied Clippy, Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical spec validation passed; the first sandboxed build required the pinned LiveKit WebRTC artifact and a Metal cache write, both of which succeeded after approved scoped escalation. The required all-target release Clippy wrapper remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.8. Implement the push-gateway executor
    - Consume wake jobs with bounded retry, endpoint authority and no event-content access.
    - _Requirements: 9.5, 19.2, 19.3_
    - _Capability IDs: CAP-016_
    - _Depends on: 4.6, 22.13_
    - _Reads: projects/buzz/crates/{buzz-push-gateway/**,buzz-relay/src/push_runtime.rs}, crates/collab/src/push/outbox.rs, .agents/specs/collaborative-workspace/security/{push,operational-limits}.md_
    - _Writes: Cargo.{toml,lock}, .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/src/push/outbox.rs, crates/collab/tests/collaboration_push_outbox.rs, services/push_gateway/{Cargo.toml,src/{push_gateway,executor}.rs}_
    - _Validation: `cargo test -p push_gateway -- --nocapture` covers transient/permanent failure, revocation race, redaction and retry exhaustion; `cargo test -p collab --test collaboration_push_outbox -- --nocapture` covers executor-facing claim persistence_
    - _Discovered contradiction (2026-08-22): the planned Buzz gateway read omits its relay-owned delivery worker, where claim revalidation, retry and terminal disposition actually live. The planned Zed service directory also did not exist, so one executor file could neither form a workspace target nor run its tests. Task 22.13 deliberately stopped at claim/complete persistence and therefore lacked the final revalidate, retry-release and generation-conditional endpoint-disable operations this executor must call without reaching into SQL. The narrow correction reads the actual Buzz worker, registers one library-only service crate and adds only those three claim-fenced repository operations plus their focused regression. APNs bytes, App Attest, token/grant custody, HTTP admission, provider credentials and deployment remain with Tasks 22.9–22.10._
    - _Evidence: 2026-08-22 — added a generic Zed push executor over injected canonical wake storage, current authorization, provider and fresh clock boundaries. Every sweep claims at most 16 community jobs for 30 seconds, rejects foreign/malformed claim output, refreshes time per send, suppresses expiry or lost read/lease/endpoint authority before provider contact and exposes only stable request ID, opaque capability digest, lease/endpoint generations, expiration and the fixed reconnect signal. Exact matching invalid-endpoint results conditionally disable only that current generation; stale provider generations cannot disable. Transient, provider-unavailable, authorization-unavailable and configuration results use stable request-derived bounded jitter with a 1–3,600-second base/final ceiling, then claim-fenced retry release; attempt eight terminates visibly. Added repository operations that rejoin the exact current lease for final revalidation, release only an unexpired current claim and disable only its still-current endpoint generation. Six gateway tests passed for transient retry, exact and stale permanent failure, revocation race, redacted fixed-payload boundary and retry exhaustion; the expanded collaboration push repository suite passed 6/6. Focused warning-denied Clippy and the required all-target release Clippy wrapper passed for `push_gateway`; the leaf's focused warning-denied `collab` integration target also passed, while the required full `./script/clippy -p collab` wrapper remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`. Rust formatting, diff, dependency, inventory and canonical spec gates passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.9. Implement approved platform push adapters
    - Add only ADR-005-approved APNs, App Attest and other platform adapters behind the common executor.
    - _Requirements: 9.5, 18.2_
    - _Capability IDs: CAP-016, CAP-040_
    - _Depends on: 2.5, 11.11, 22.8_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-005-push-scope.md, projects/buzz/crates/buzz-push-gateway/src/**_
    - _Writes: Cargo.lock, services/push_gateway/Cargo.toml, services/push_gateway/src/{push_gateway,executor,platform}.rs, services/push_gateway/src/platform/{apns,app_attest,grant}.rs_
    - _Validation: `cargo test -p push_gateway -- --nocapture` uses closed provider/authority fakes for token custody, attestation replay, expiry, provider-error mapping and unsupported-provider behavior_
    - _Discovered contradiction (2026-08-22): the phrase "other platform adapters" conflicts with ADR-005's explicit prohibition on advertising FCM, UnifiedPush, webhooks or any transport lacking a separately approved fixed-body profile. The planned nested-only write path also cannot register the platform module, add the audited cryptographic dependencies or construct a provider request for crate-internal contract tests. The narrow correction adds only the two existing Buzz APNs profile identifiers, their common APNs/App Attest/grant-token implementation, crate registration and a test-only common-request constructor. HTTP admission, durable gateway authority schema, replay/quota reservation, configuration/readiness and deployment remain with Tasks 22.10, 22.12 and 45.2._
    - _Evidence: 2026-08-22 — implemented an ADR-005-only platform layer behind the Task 22.8 provider contract. `buzz-ios-production` and `buzz-ios-sandbox` are the only accepted profiles; FCM, UnifiedPush, webhook, desktop and renamed-profile inputs fail closed without fallback. A Buzz-compatible AES-256-GCM grant/token authority preserves the existing AEAD domains, closed relay-delegation grant schema and plain SHA-256 endpoint-grant reference, while current keys mint, bounded predecessors decrypt, grant and token key material cannot overlap, raw tokens are exactly 32 bytes and all secret-bearing diagnostics are redacted. Resolution rechecks the exact lease generation, endpoint epoch, capability and expiration before yielding a token. The Apple verifier pins the accepted root certificate hash and configured application identifier, bounds attestation/assertion CBOR and UTF-8 transcripts, and uses an injected store contract to atomically consume challenges and advance counters; replay, counter rollback, profile mismatch and expiry reject generically. The APNs adapter has separate non-fallthrough production/sandbox routes, topics and credentials, a 15-second no-proxy/no-redirect client, cached ES256 tokens, fixed alert push type/priority, floored non-extending expiry, the exact registered reconnect body, a 4 KiB provider-error ceiling, one expired-token refresh and closed accepted/retry/invalid-endpoint/configuration/request-fault mapping. Eight new platform contract tests plus the six executor regressions passed (14/14); package check, focused all-target warning-denied Clippy, the required release/all-target wrapper, Rust formatting and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.10. Add push-gateway deployment artifacts
    - Add configuration, migrations, health/readiness and bounded resources without production deployment.
    - _Requirements: 19.3, 19.4_
    - _Capability IDs: CAP-016, CAP-043_
    - _Depends on: 22.7, 22.8, 22.9_
    - _Reads: projects/buzz/deploy/charts/buzz-push-gateway/**, deploy/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, deploy/collaboration/push-gateway/{Chart.yaml,README.md,values.yaml,values-production.yaml,values-rollback.yaml,values.schema.json,templates/*,tests/render.sh}_
    - _Validation: `deploy/collaboration/push-gateway/tests/render.sh` runs Helm lint/render and configuration contracts for default-off, enabled, production, missing-secret, bounded-limit and rollback cases_
    - _Discovered contradiction (2026-08-22): Zed had no existing `deploy/` convention or runnable push-gateway image to extend, while the Buzz chart defaults to a mutable runnable image and live Buzz hostname. Copying that posture would contradict this leaf's explicit "without production deployment" boundary and would claim readiness before the later HTTP/authority-store executable work. The narrow correction introduces a Zed-owned Helm contract that renders no resources by default and makes checked-in production/rollback values intentionally invalid until a release system injects immutable images and environment-owned authority. It reuses only the already-canonical collaboration schema floor from Task 22.7; new gateway authority schema, public HTTP admission, signed-image publication and actual environment rollout remain with Tasks 45.2 and 48.1–48.4._
    - _Evidence: 2026-08-22 — added a strict disabled-by-default Helm chart for the collaboration push gateway. Enabled renders pin the public Service to port 8080 while liveness, dependency-aware readiness and optional metrics remain on pod-private 8081; probes enforce the 3-second/5-second/10-second/120-second operational contract. Runtime and migration pods use separate labels, non-root/read-only security contexts and Secret references, with DML runtime and DDL migration database identities forbidden from aliasing. App Attest and the exact `buzz-ios-production`/`buzz-ios-sandbox` APNs profiles use environment-owned certificate, credential and configuration references; unsupported/renamed profiles and shared production/sandbox secrets fail configuration. Fixed 8 KiB body, 256-request concurrency, 20-second request, 16-job claim, 30-second claim and eight-attempt ceilings are schema-validated and rendered with explicit CPU, memory and ephemeral-storage bounds, a disruption budget and DNS/APNs/PostgreSQL-only egress. Production requires an immutable digest, explicit Gateway attachment and database network. Forward migration is deadline/backoff bounded at schema `20260822000200`; rollback requires a compatible previous digest and renders neither migration Job nor migration NetworkPolicy. Helm lint and Ruby manifest assertions passed for default, enabled, production and rollback renders; negative cases passed for missing runtime/APNs secrets, aliased DDL credentials, mutable production image, missing/incompatible rollback data, expanded body limit and unknown configuration. JSON schema, shell syntax and diff hygiene passed. Commit: enclosing checkpoint commit, reported after creation._

  - [x] 22.11. Add search and notification privacy conformance
    - Probe authorization-before-limit, private indexing, previews and wake payloads using mixed-version clients.
    - _Requirements: 6.3, 9.4, 9.5, 20.2_
    - _Capability IDs: CAP-015, CAP-016, CAP-044_
    - _Depends on: 11.11, 22.2, 22.5, 22.9_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/search/src/collaboration_search.rs, services/push_gateway/src/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, Cargo.lock, crates/collab/Cargo.toml, crates/collab/tests/search_push_privacy.rs_
    - _Validation: conformance returns no private content, count, preview or payload across community boundaries_
    - _Discovered contradiction (2026-08-22): the approved single `collab` integration-test owner could not exercise the existing `search` presentation crate or standalone `push_gateway` executor through `collab`'s prior dev-dependency graph, and its notification dev dependency did not enable the already-implemented collaboration surface. Moving the conformance leaf into one of those lower crates would omit at least two production boundaries and contradict the required mixed search/notification/push trace. The narrow correction adds only test-time `search` and `push_gateway` dependencies and enables notifications' existing `multiplayer-tools` test surface; no normal dependency, production feature, owner or runtime path changes._
    - _Evidence: 2026-08-22 — added an independent 16-observation privacy trace that drives both Buzz-compatibility and consolidated-Zed provenance through production search repository/indexer/presentation, native notification and push-executor boundaries. Foreign search is rejected before a database transaction; authorized SQL applies visibility before ranking/limit and returns references without event content; direct-message indexing emits a content-free exclusion and rejects a seeded content-bearing exclusion without logging it. Unauthorized native presentation preserves only the local result plus a generic unavailable status. Private notifications expose exactly `New private activity` / `Open Zed to view it.` and neither private content nor source identifiers; a cross-community membership mismatch returns the exact inactive-membership suppression without posting. Authorized provider requests contain only the byte-exact `"reconnect"` payload, while a foreign wake is rejected before provider contact. The checker requires exact client/surface coverage and independently rejects private text, both private record identifiers, counts/provider operations and non-generic previews/payloads. `cargo test -p collab --test search_push_privacy -- --nocapture` passed 3/3; focused warning-denied Clippy passed; `script/check-collaboration-dependencies`, formatting and diff hygiene passed. Two existing unused-import warnings from `crates/language_model/src/fake_provider.rs` were emitted while dependencies compiled and remain outside this leaf._

  - [x] 22.12. Add search and push load/failure tests
    - Measure indexing/query and wake throughput, queue bounds, replica lag and recovery under dependency failure.
    - _Requirements: 8.3, 8.4, 19.3, 20.1_
    - _Capability IDs: CAP-006, CAP-015, CAP-016, CAP-044_
    - _Depends on: 22.10, 22.11_
    - _Reads: projects/buzz/perf/**, crates/collab/src/search/**, services/push_gateway/src/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, test-results/collaborative-workspace/search-push-plan.md_
    - _Validation: approved load command meets bounded queue/freshness budgets and records recovery evidence_
    - _Discovered contradiction (2026-08-22): Buzz has no search/push performance harness, and the planned plan-only write cannot itself execute a load or failure probe. Adding a new production or CI harness in this leaf would exceed its approved ownership and duplicate the integrated gate reserved for Task 45.4. The narrow correction uses disposable native PostgreSQL 14/`pgbench` environments, freezes the exact production-SQL workload contracts and bounded commands in the evidence plan, and adds only living-spec traceability. No production code, external provider, database or deployment changes._
    - _Evidence: 2026-08-22 — the approved local component gate passed. A clean prepared indexing run completed 10,000/10,000 document-plus-checkpoint transactions at 4,912.332 transactions/s, 0.812 ms average and zero above 50 ms. Authorized search plus freshness completed 2,000/2,000 at 49.187 transactions/s and 162.239 ms average; exactly 500 public controlled hits were searchable while 500 otherwise-identical restricted documents remained vector-null, and the 26.2% diagnostic late fraction stays a mandatory Task 45.4 input rather than a production SLA. Push claims completed 400 bounded transactions at 100.464 transactions/s, 1,607.417 wakes/s, 39.543 ms average, zero above 100 ms and exact batches of 16. One expired batch recovered to attempt 2 in 20.130 ms without moving deferred work; one deliberately divergent search checkpoint became clean in 1.531 ms; and a paused physical replica accumulated 4,047,576 WAL bytes before reaching zero within a conservative 5,664 ms and exposing all 5,000 burst rows. Three live PostgreSQL privacy/schema tests passed and the complete `push_gateway` suite passed 14/14 retry, exhaustion, revocation and provider-failure cases. Exact provenance, budgets, commands, cleanup and production-gate limits are recorded in `test-results/collaborative-workspace/search-push-plan.md`._

  - [x] 22.13. Implement push-lease and wake-outbox persistence
    - Read/write encrypted leases and consume idempotent wake jobs under canonical device authority.
    - _Requirements: 9.5, 17.2_
    - _Capability IDs: CAP-005, CAP-016_
    - _Depends on: 22.7_
    - _Reads: crates/collab/migrations/20260822000200_collaboration_push.up.sql, crates/collaboration_domain/src/push_lease.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab/src/{lib.rs,push.rs,push/outbox.rs}, crates/collab/tests/collaboration_push_outbox.rs_
    - _Validation: `cargo test -p collab --test collaboration_push_outbox -- --nocapture` covers replacement, revoke, crash retry, duplicate wake and tenant isolation_
    - _Discovered contradiction (2026-08-22): Task 22.7 correctly replaced the planned unversioned migration path with the crate's timestamped reversible SQLx pair, and the planned single nested Rust file cannot compile or expose its repository without crate/module registration or run the required repository suite without an integration target. The narrow correction reads the canonical timestamped schema and adds only those registrations plus one focused test. Provider contact, raw endpoint grants, matching policy, retry scheduling and APNs payload construction remain with later gateway leaves._
    - _Evidence: 2026-08-22 — added a PostgreSQL-only push repository that installs transaction-local tenant RLS and defensively rejects foreign typed inputs or rows. Effective lease upserts require strictly newer safe generations, round-trip active authority through bounded encrypted capability/subscription envelopes and a custody key ID, and atomically persist revocation tombstones with retained active expiry and no device secrets. Wake admission selects through the exact current enabled lease generation, endpoint generation, capability digest and expiry; an exact stored identity returns a duplicate while key collisions and unavailable authority fail closed. Bounded `SKIP LOCKED` claims rejoin current device authority, reclaim pending or expired leased rows, increment attempts and fence terminal completion by the current unexpired claim. Five focused tests passed for replacement/encrypted read/stale suppression, revoke/secret clearing, crash recovery and completion, duplicate enqueue and pre-write plus defensive-read tenant isolation. The collab library check and focused warning-denied Clippy passed; formatting, diff, dependency, inventory and specification gates are recorded in the checkpoint validation. The required all-target release Clippy wrapper remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`. Commit: enclosing checkpoint commit, reported after creation._

- [x] 23. Port inbox, pulse, forum, custom emoji and feedback

  - [x] 23.1. Implement the canonical inbox projection
    - Derive mentions, replies, reminders and activity from message/read records without a second message store.
    - _Requirements: 2.2, 9.1, 9.3_
    - _Capability IDs: CAP-013, CAP-017_
    - _Depends on: 19.3, 19.8, 19.9, 21.1, 21.3_
    - _Reads: projects/buzz/desktop/src/features/home/**, crates/collaboration_domain/src/{message,thread,message_marker,read_state,reminder}.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,inbox}.rs_
    - _Validation: projection fixtures cover mention, reminder, read, deletion and duplicate events_
    - _Discovered contradiction (2026-08-22): the planned `message_aux.rs` and `read-state.rs` paths do not exist; their approved canonical owners are `message.rs`, `thread.rs`/`message_marker.rs` and `read_state.rs`. A standalone `inbox.rs` also cannot compile, expose its projection API or run its fixtures without crate-root registration. The narrow correction updates those read/write paths, registers one pure domain module and keeps the focused fixtures inside it. It adds no message/reminder persistence, protocol parsing, authorization decision, timer, notification or UI owner._
    - _Evidence: 2026-08-22 — added a bounded tenant-and-owner-scoped inbox projection over borrowed canonical `Message` records, one canonical `ReadState` and owner-readable `Reminder` records. Approved adapters supply stable conversation/read contexts and already-resolved mention/reply relationships; the projection validates their scope and bounds, rejects conflicting identifiers, deduplicates exact records and retains only stable IDs, categories, counts and timestamps. Self-authored and deleted messages never become rows; active external messages group by conversation, select the oldest unread representative and conservatively preserve read-state completeness. Pending reminders enrich an existing target conversation without duplication or become stable standalone rows, while private target/note content is never retained. Focused mention/reply, reminder, read, deletion and duplicate fixtures passed 3/3; the full `collaboration_domain` suite passed 111/111; release all-target/all-feature warning-denied `./script/clippy -p collaboration_domain`, the collaboration dependency boundary, formatting and diff hygiene passed. Inventory and specification validation are recorded in the enclosing checkpoint commit._

  - [x] 23.2. Render native inbox and pulse lists
    - Add filterable, paged GPUI lists over canonical inbox/activity projections.
    - _Requirements: 4.4, 9.1, 9.3_
    - _Capability IDs: CAP-017, CAP-036_
    - _Depends on: 23.1_
    - _Reads: projects/buzz/desktop/src/features/{home,pulse}/**, crates/collab_ui/src/**_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab_ui/src/{collab_ui,inbox_pulse}.rs_
    - _Validation: GPUI tests cover unread, filters, pagination, empty and stale states_
    - _Discovered contradiction (2026-08-22): the planned standalone source file cannot compile or expose the view without crate-root registration, and its canonical inbox/activity dependencies are intentionally available only behind `collab_ui`'s existing `multiplayer-tools` boundary. Focused GPUI tests also require the package's existing `test-support` feature so transitive remote mock variants and their guarded consumers are enabled consistently. The narrow correction adds only the feature-gated module registration and living-spec trace; it introduces no dependency, persistence, authorization, refetch or application-shell composition owner._
    - _Evidence: 2026-08-22 — added a native GPUI inbox/pulse view over one revisioned canonical `InboxProjection` plus canonical activity items. It rejects stale revisions, foreign scopes, invalid or conflicting pulse rows and oversized activity snapshots without replacing trusted state. Inbox filters cover activity, unread, mentions, replies and reminders; pulse filters cover people, agents and system/service actors. Mode/filter changes reset bounded 1–200-row paging, the current page renders through GPUI `ListState`, loading/empty states remain distinct and stale/retrying states preserve cached rows with an accessible warning. Focused GPUI fixtures passed 3/3 for filters, pagination/list bounds, empty snapshots, stale retention and revision rejection using `cargo test -p collab_ui --no-default-features --features multiplayer-tools,test-support inbox_pulse -- --nocapture`; warning-denied production and test Clippy targets passed. The collaboration dependency boundary, inventory, specification, formatting and diff-hygiene checks are recorded in the enclosing checkpoint commit._

  - [x] 23.3. Implement forum post, vote and comment domain rules
    - Model forum records as channel/message projections with authorized voting and stable thread links.
    - _Requirements: 9.1, 9.2_
    - _Capability IDs: CAP-011, CAP-017_
    - _Depends on: 19.2, 19.4_
    - _Reads: projects/buzz/{desktop/src/features/forum/**,desktop/src-tauri/src/commands/messages/forum.rs,mobile/lib/features/forum/**,crates/buzz-sdk/src/builders.rs,crates/buzz-relay/src/handlers/ingest.rs,crates/buzz-cli/src/commands/messages.rs}, crates/collaboration_domain/src/{channel,message,thread}.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,forum}.rs_
    - _Validation: domain tests cover vote replacement, comment deletion, pagination and visibility_
    - _Discovered contradiction (2026-08-22): the planned desktop/mobile forum directories implement post/comment presentation and relay fetching but contain no vote command or replacement rule. Buzz's actual `+`/`-` vote construction is in `buzz-sdk` and the CLI, while relay ingest verifies that votes target a same-channel forum post/comment; those read paths are required to preserve behavior. A standalone domain file also cannot compile or expose its projection without crate-root registration. The narrow correction registers one borrowed projection over the existing canonical channel/message/thread owners and treats ordinary signed votes as deterministic per-voter/target observations. It adds no protocol kind, message copy, persistence, membership authority or UI owner._
    - _Evidence: 2026-08-22 — added a canonical forum projection that requires current channel-read authorization, accepts only same-community/forum-channel canonical messages, preserves original source event IDs as stable post/comment ancestry and reuses `ThreadGraph` for deletion-safe summaries and exact comment continuation. Post paging is bounded to 1–200 rows and orders descending timestamp/ascending event ID; a cursor excludes concurrently arriving newer rows while retaining every older same-second row. Vote construction requires an active forum, live post/comment target and the canonical write policy; projection input is bounded and duplicate-source-safe, and one vote per voter/target resolves by greatest timestamp then lowest event ID before counts, score and viewer direction are derived. Archived forums remain readable but reject votes, while deleted/expired forums fail closed. Focused vote-replacement/authorization, deleted-comment/thread-pagination and post-pagination/visibility fixtures passed 3/3; the full `collaboration_domain` suite passed 114/114; release all-target/all-feature warning-denied `./script/clippy -p collaboration_domain`, formatting and diff hygiene passed. The collaboration dependency boundary, inventory and canonical specification validation are recorded in the enclosing checkpoint commit._

  - [x] 23.4. Render native forum surfaces
    - Add post list/detail/composer views using canonical channel, thread and forum records.
    - _Requirements: 4.4, 9.1_
    - _Capability IDs: CAP-017, CAP-036_
    - _Depends on: 19.7, 23.3_
    - _Reads: crates/collaboration_domain/src/forum.rs, crates/collab_ui/src/{message_timeline,dm_view}.rs, crates/agent_ui/src/{message_editor,collaborative_composer}.rs, crates/workspace/src/collaborative_composer.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab_ui/src/{collab_ui,forum}.rs_
    - _Validation: GPUI tests cover create, vote, comment, permission denial and archived forum_
    - _Discovered contradiction (2026-08-22): the planned standalone view cannot compile or expose its API without feature-gated crate-root registration, so the narrow write set includes that registration and the living-spec trace. `ForumProjection` deliberately borrows canonical messages and therefore cannot be retained in a `'static` GPUI entity; the view snapshots bounded presentation rows and retains exact domain cursors while adapters apply subsequent projection pages. The existing agent `MessageEditor` is bound to agent-thread project/workspace authority and cannot safely become a forum command owner, so the forum composer emits typed create/comment/vote requests for an authorized adapter to execute and never constructs canonical messages, votes or optimistic authority itself._
    - _Evidence: 2026-08-22 — added a native GPUI forum surface with bounded virtual post paging, canonical post/thread identities, accessible list/detail/archived states and thread detail rendered through the existing `MessageTimeline`. Author presentation is explicitly adapter-resolved, missing or conflicting identities fail closed, borrowed projections are scope-checked on every page, and exact forum/thread cursors drive continuation without a second store. The composer trims and validates canonical `MessageContent` then emits scoped post/comment requests; vote controls emit scoped directions without changing projected counts, and adapter permission plus archived state gates every write. Focused GPUI fixtures passed 3/3 for post creation, canonical comment detail/submission, voting, projected permission denial and archived read-only behavior; warning-denied production and test Clippy targets passed. Feature-boundary, collaboration dependency, inventory, specification, formatting and diff-hygiene checks are recorded in the enclosing checkpoint commit._

  - [x] 23.5. Implement custom emoji records and reaction resolution
    - Validate community emoji identifiers/assets and resolve long reaction values without changing message authority.
    - _Requirements: 9.1, 14.1_
    - _Capability IDs: CAP-011, CAP-017, CAP-031_
    - _Depends on: 19.3_
    - _Reads: projects/buzz/desktop/src/{features/custom-emoji/**,shared/api/customEmoji.ts,features/messages/lib/formatTimelineMessages.ts}, projects/buzz/mobile/lib/shared/custom_emoji/custom_emoji.dart, projects/buzz/crates/{buzz-sdk/src/builders.rs,buzz-relay/src/handlers/ingest.rs}, projects/buzz/migrations/0028_long_reaction_payloads.sql, crates/collaboration_domain/src/reaction.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,custom_emoji}.rs_
    - _Validation: tests cover duplicate names, invalid media, removal and historical reaction rendering_
    - _Discovered contradiction (2026-08-22): the planned migration glob uses a hyphen but Buzz's actual long-reaction migration is `0028_long_reaction_payloads.sql`; the desktop feature delegates canonical shortcode, asset and long-reaction admission to `buzz-sdk` and relay ingest, while historical image rendering reads the matching asset tag from each reaction event. Those sources and the existing canonical reaction aggregate are therefore required reads. A standalone domain file also cannot compile or expose its resolver without crate-root registration and living-spec trace. NIP-30 records carry an HTTP(S) asset reference but no trusted MIME type, so upload-time MIME inspection remains an adapter gate rather than invented domain authority._
    - _Evidence: 2026-08-22 — added bounded community custom-emoji set records with canonical lowercase 1–64 byte shortcodes, Buzz-compatible HTTP(S) asset-reference validation, per-set duplicate rejection and exact-source conflict handling. Per-owner replaceable heads select greatest timestamp then lowest event ID; the community union collapses shortcode collisions by greatest set timestamp then lexicographically smallest asset URL, matching Buzz, while a newer set omission removes only that owner's contribution. Reaction resolution consumes the existing immutable `ReactionGroup`, validates tags against active reaction event IDs and deterministically prefers the earliest active embedded asset, so a 66-character wrapped reaction remains renderable after palette removal without mutating message or reaction authority; long reactions without their required event asset fail closed. Four focused duplicate/media/removal/history fixtures passed, as did the adjacent 64-byte reaction boundary fixture, the full 118-test domain suite and release all-target/all-feature warning-denied Clippy. Dependency, inventory, specification, formatting and diff-hygiene checks are recorded in the enclosing checkpoint commit._

  - [x] 23.6. Implement feedback event flow
    - Add authorized feedback submission and operator-safe status projection without exposing private context.
    - _Requirements: 9.1, 15.4_
    - _Capability IDs: CAP-017, CAP-029_
    - _Depends on: 18.2, 19.2_
    - _Reads: projects/buzz/desktop/src/features/settings/{hooks/useSendFeedback.ts,ui/SendFeedbackDialog.tsx}, projects/buzz/crates/{buzz-relay/src/handlers/{product_feedback,ingest}.rs,buzz-db/src/{product_feedback,admin_moderation}.rs}, projects/buzz/migrations/0017_product_feedback.sql, projects/buzz/VISION_MODERATION.md, crates/collaboration_domain/src/authorization.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,feedback}.rs_
    - _Validation: tests cover submit, redact, status update, unauthorized read and tenant isolation_
    - _Discovered contradiction (2026-08-22): the planned Pulse directory contains activity-feed UI and no product-feedback flow. Discovery found the actual desktop submission surface under settings and the private relay/database sidecar handlers and migration; `VISION_MODERATION.md` describes moderation but no product-feedback status workflow. Buzz accepts private kind-42000 feedback outside ordinary event storage and exposes deployment-admin detail, but has no feedback workflow status field or transition command. The narrow correction therefore registers one pure canonical aggregate with a closed content-free status projection while leaving protocol/tag/attachment validation, sidecar persistence and operator tooling to later adapters; it does not turn feedback into message or timeline authority._
    - _Evidence: 2026-08-22 — added exact-scope, current-member feedback submission with a bounded trimmed private body, signed source provenance and resolved submitter identity. Feedback status is a closed optimistic-versioned workflow whose monotonic operation IDs make exact retries idempotent and conflicting reuse fail closed; read and manage authorization shapes are distinct, and only current owners/admins can obtain the content-free status view or mutate it. Debug and outward error text redact body, submitter and private context, while the status view contains only category, state/reason, event identity, version and update time. Five focused fixtures passed for authorized submit, debug/projection redaction, versioned/idempotent status updates, unauthorized member read/update and cross-tenant submit/read denial; the full domain suite passed 123/123. Remaining formatting, Clippy, dependency, inventory, specification and diff gates are recorded in the enclosing checkpoint commit._

  - [x] 23.7. Add social-surface projection regressions
    - Verify inbox, pulse, forum, emoji and feedback rebuild from canonical records and recover after reconnect.
    - _Requirements: 8.2, 9.1, 9.3, 20.1_
    - _Capability IDs: CAP-013, CAP-017, CAP-044_
    - _Depends on: 23.2, 23.4, 23.5, 23.6_
    - _Reads: crates/collaboration_domain/src/{inbox,forum,custom_emoji,feedback}.rs, crates/collab_ui/src/{inbox_pulse,forum}.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collab_ui/tests/social_surfaces.rs_
    - _Validation: `cargo test -p collab_ui --no-default-features --features multiplayer-tools,test-support --test social_surfaces -- --nocapture` passes rebuild, offline, reconnect and failure fixtures_
    - _Discovered contradiction (2026-08-22): the planned `custom-emoji.rs` path uses a hyphen while the canonical module is `custom_emoji.rs`, and the planned default-feature test command cannot compile the feature-gated native social modules. The focused integration target therefore runs under the existing `multiplayer-tools,test-support` test profile. Forum projections deliberately borrow canonical records and the native forum view deliberately owns a bounded snapshot, so reconnect recovery reconstructs a replacement view from a fresh projection rather than adding mutable borrowed authority or a second store. The test-only correction adds no protocol, persistence, authorization, refetch or application-shell owner._
    - _Evidence: 2026-08-22 — added two cross-surface GPUI integration fixtures over canonical inbox/read/message, activity, forum/channel/thread, custom-emoji and private-feedback records. The reconnect fixture proves independent rebuilds are deterministic, inbox/pulse retains its accepted revision while offline and retrying, a strictly newer snapshot incorporates reconnected inbox and pulse records, a fresh forum snapshot incorporates new canonical posts, replaceable emoji resolves the new owner head and feedback rehydrates its versioned operator status without exposing its body. The failure fixture proves foreign inbox scope, foreign pulse context, missing forum presentation, foreign emoji records and unauthorized feedback status reads fail without replacing trusted rows, posts, palette or feedback state. The focused target passed 2/2 after downloading Cargo's pinned LiveKit/WebRTC build artifact; warning-denied Clippy, dependency, inventory, specification, formatting and diff gates are recorded in the enclosing checkpoint commit._

## Milestone 4 — projects, Git and review collaboration

- [x] 24. Bind Zed projects and repositories to NIP-MP metadata

  - [x] 24.1. Define signed project-group metadata
    - Model NIP-MP project identity, visibility and repository coordinates without local filesystem authority.
    - _Requirements: 10.1_
    - _Capability IDs: CAP-018_
    - _Depends on: 11.10, 18.2_
    - _Reads: projects/buzz/docs/nips/NIP-MP.md, crates/nostr_compat/src/buzz_nips/project_workflow.rs, crates/project/src/project.rs_
    - _Writes: .agents/specs/collaborative-workspace/{design,tasks}.md, crates/collaboration_domain/src/{collaboration_domain,project_group}.rs_
    - _Validation: domain tests cover multi-repository, cross-owner, visibility and invalid coordinate cases_
    - _Discovered contradiction (2026-08-22): Task 11.10 already installed the signed NIP-MP event parser and exact wire grammar in `nostr_compat`, so duplicating signature/event/tag parsing in the domain would create a second protocol owner. The narrow domain boundary instead consumes adapter-verified signed metadata, while a standalone module requires crate-root registration and living-spec trace to compile and expose it. NIP-MP is explicitly global-only and permits unresolved cross-owner coordinates, whereas Zed's existing `Project` owns local worktrees and filesystem state; the model therefore retains signed global container metadata without a community, local path or repository authority and leaves tenant persistence/binding to Task 24.3._
    - _Evidence: 2026-08-22 — added a pure signed project-group model whose stable identity is the Nostr signer plus bounded nonempty slug and whose source retains the signed event identity/time. Bounded name/description/channel metadata, exact unlisted-only visibility and zero-member groups match NIP-MP. Up to 64 NIP-34 coordinates preserve each repository owner, verbatim colon-bearing discriminator and opaque relay hint, sort deterministically and reject duplicate identity even with different hints; malformed kind, owner grammar, empty discriminator and oversized values fail closed. Cross-owner fixtures prove the project signer remains distinct from each member owner, and the public model exposes no local filesystem, worktree, remote, push or permission authority. Three focused fixtures and the full 126-test domain suite passed, as did release all-target/all-feature warning-denied Clippy. Dependency, inventory, specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 24.2. Add stable collaboration repository identity
    - Map local repository identity to hosted coordinates and preserve remotes/worktrees as Zed-owned state.
    - _Requirements: 2.1, 10.1_
    - _Capability IDs: CAP-018, CAP-019_
    - _Depends on: 24.1_
    - _Reads: crates/project/src/{project,worktree-store}.rs, crates/project/src/git_store.rs_
    - _Writes: crates/project/src/collaboration_repository.rs_
    - _Validation: `cargo test -p project collaboration_repository_identity` covers reopen, remote change and missing repo_
    - _Discovered contradiction (2026-08-22): the planned standalone source file cannot compile or prove its contract without registering the module, declaring its lightweight collaboration-domain dependency and registering a focused project integration fixture. Zed already owns the correct stable disk identity through the common Git directory, including linked-worktree convergence and independent submodule handling, while `RepositoryId` and remote URLs are intentionally transient. The implementation therefore resolves only a currently live repository through `Project`, stores neither transient handle nor remote/worktree state, and leaves durable project-group persistence to Task 24.3._
    - _Evidence: 2026-08-22 — added a read-only local repository identity and hosted-coordinate binding that reuse Zed's canonical repository identity path, keep NIP-34 coordinate validation in `collaboration_domain` and fail explicitly when the repository is absent. The focused GPUI fixture reopened the same repository through a new `Project`, observed an origin URL change, proved the local identity and hosted binding remained stable and rejected an unknown repository ID. The exact planned test command passed 1/1; warning-denied Clippy, dependency, inventory, specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 24.3. Persist project-group bindings
    - Store versioned project/repository/channel bindings separately from local project persistence.
    - _Requirements: 2.2, 10.1_
    - _Capability IDs: CAP-005, CAP-018_
    - _Depends on: 24.1, 24.2_
    - _Reads: crates/collab/src/db/**, crates/collaboration_domain/src/project_group.rs_
    - _Writes: crates/collab/migrations/collaboration_projects.sql_
    - _Validation: migration tests cover tenant fences, cross-owner grouping and binding deletion_
    - _Discovered contradiction (2026-08-22): the planned unversioned `collaboration_projects.sql` path is incompatible with the crate's SQLx reversible migration source, which requires a unique numeric version plus paired `.up.sql` and `.down.sql` files; proving real DDL and rollback also requires a focused migration test target not listed in the planned writes. The signed NIP-MP head is global canonical metadata, but its community association is a tenant-local projection, so the schema retains signed source/version provenance and binding history without copying Zed-owned local project, worktree, remote or filesystem state._
    - _Evidence: 2026-08-22 — added reversible tenant-fenced tables for immutable signed project-group versions and independently versioned repository/channel bindings. Current-row indexes and explicit active/deleted tombstones preserve replacement and deletion history; exact group-version foreign keys bind each projection to its signed source, and repository owner keys remain independent from project signer keys. A focused PostgreSQL 14 run applied the channel prerequisite and project migration, exercised least-privilege forced RLS isolation, stored a cross-owner NIP-34 coordinate, replaced the signed project head, retired both bindings into current deletion tombstones and rolled all project tables down; all 3 tests passed. The focused target also passed without a database for deterministic schema/checksum coverage. Dependency, inventory, canonical specification, Rust 2024 formatting and diff-hygiene gates passed. The broader filtered Collab test command remains blocked by a pre-existing duplicate `test_rejoining_channel_after_stale_connection_cleanup_connects_livekit` definition, and warning-denied Clippy was stopped when its unrelated full application graph reduced free disk to 1.5 GB; neither blocker produced a task-local diagnostic._

  - [x] 24.4. Integrate project and channel navigation bindings
    - Resolve signed project/channel bindings into existing native project and collaborative navigation entities.
    - _Requirements: 4.3, 10.1_
    - _Capability IDs: CAP-010, CAP-018, CAP-036_
    - _Depends on: 18.6, 24.3_
    - _Reads: crates/sidebar/src/collaborative_projects.rs, crates/project/src/collaboration_repository.rs_
    - _Writes: crates/project/src/collaboration_navigation.rs_
    - _Validation: navigation tests cover missing local clone, multiple worktrees and archived group_
    - _Discovered contradiction (2026-08-22): `workspace` already depends on `project`, so importing `workspace::CollaborativeNavigationTarget` into the planned project adapter would create a dependency cycle and a second navigation owner. The stable integration seam is the existing native values those targets consume: `ProjectGroupKey`, repository work directories, `WorktreeId` plus path and canonical community/channel IDs. Compiling and proving the new public adapter also requires project module registration and a focused integration fixture beyond the single planned write file. Workspace remains the sole owner of navigation mutation/history persistence, while this adapter only resolves current signed/persisted bindings against live native entities._
    - _Evidence: 2026-08-22 — added a deterministic read-only project navigation resolver that rejects archived groups, unexpected/duplicate repository bindings and mismatched channel bindings before producing a target. Signed repository coordinates preserve their independent owners and resolve through stable common-Git-directory identity to every matching live checkout; an absent or stale local binding remains an explicit unavailable member. Exact active channel bindings project to canonical community/channel IDs, while absent or inactive channels produce no broken link. Three focused GPUI integration fixtures passed for a stale binding whose clone is missing, a main checkout plus an open linked worktree resolving to two native repository/worktree targets and one active channel, and an archived group that cannot produce navigation. `./script/clippy -p project` passed release all-target/all-feature warning-denied checks; dependency, inventory, specification, formatting and diff gates are recorded in the enclosing checkpoint commit._

  - [x] 24.5. Prove grouping never grants Git authority
    - Add negative integration tests for push, filesystem and external-host operations by project signers.
    - _Requirements: 6.2, 10.1, 20.3_
    - _Capability IDs: CAP-018, CAP-019, CAP-044_
    - _Depends on: 24.3, 24.4_
    - _Reads: crates/project/src/collaboration_*.rs, crates/git/**_
    - _Writes: crates/project/tests/project_group_permissions.rs_
    - _Validation: `cargo test -p project project_group_permissions` denies every authority not separately granted_
    - _Discovered contradiction (2026-08-22): directly invoking native `GitStore::push`, filesystem writes or an HTTP client in a negative grouping test would exercise the already-authorized local Zed owners and could not attribute the operation to the NIP-MP signer. The security property is instead that no signer-derived grouping/navigation value is a capability handle for any of those owners. The regression therefore combines compile-time negative trait assertions against the canonical Git, filesystem and HTTP interfaces with a cross-owner runtime snapshot proving resolution itself performs no file or remote mutation; it adds no parallel authorization policy or test-only denial shim._
    - _Evidence: 2026-08-22 — added a dedicated integration target that statically rejects `GitRepository`, `Fs` and `HttpClient` implementation on every public project-group, repository, local-repository, worktree and channel navigation target. A runtime fixture signs a project with a key distinct from the member repository owner, resolves the cross-owner binding and proves the protected file bytes and configured external origin remain unchanged. The exact planned command passed 2/2, and `./script/clippy -p project` passed release all-target/all-feature warning-denied checks. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

- [ ] 25. Consolidate NIP-34 forge and Git signing/authentication

  - [x] 25.1. Implement NIP-34 repository and ref codecs
    - Encode and validate repository announcements, state, refs and status events under ADR-003.
    - _Requirements: 5.1, 10.2_
    - _Capability IDs: CAP-019_
    - _Depends on: 2.3, 11.10, 24.2_
    - _Reads: .agents/specs/collaborative-workspace/decisions/adr-003-git-authority.md, projects/buzz/crates/buzz-core/src/git.rs_
    - _Writes: crates/nostr_compat/src/nip34_repository.rs_
    - _Validation: golden fixtures round-trip refs, clone URLs, maintainers and malformed coordinates_
    - _Discovered contradiction (2026-08-22): the approved Buzz read path `projects/buzz/crates/buzz-core/src/git.rs` does not exist. The registered Git kind constants remain in `buzz-core/src/kind.rs`, while the actual repository announcement, coordinate and status builders plus their validation fixtures live in `buzz-sdk/src/builders.rs`; Buzz has no repository-state builder. The port therefore reads those actual sources and the canonical NIP-34 grammar without changing the task's approved ownership or introducing a second Git/ref authority._
    - _Evidence: 2026-08-22 — added a pure `nostr_compat` codec for kind-30617 repository announcements, exact kind-30617 coordinates and subordinate references, kind-30618 repository ref state and kinds 1630–1633 status events. Frozen vectors round-trip multi-value web/clone/relay tags, maintainers, SHA-1 refs, symbolic HEAD, opaque future tags, repository/status coordinates and applied/merged references; negative vectors reject malformed coordinates, unsafe refs and status-only metadata on the wrong kind. The full crate suite passed 59 unit and four integration tests, and `./script/clippy -p nostr_compat` passed release all-target/all-feature warning-denied checks. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.2. Implement NIP-34 patch, PR and issue codecs
    - Encode and validate patches, pull requests, issues, comments and status references.
    - _Requirements: 5.1, 10.2_
    - _Capability IDs: CAP-019, CAP-020_
    - _Depends on: 25.1_
    - _Reads: projects/buzz/crates/buzz-core/src/git.rs, projects/buzz/docs/nips/NIP-GS.md_
    - _Writes: crates/nostr_compat/src/nip34_collaboration.rs_
    - _Validation: golden fixtures cover patch series, revisions, issue links and invalid ancestry_
    - _Discovered contradiction (2026-08-22): `projects/buzz/crates/buzz-core/src/git.rs` does not exist, and the listed NIP-GS document specifies local commit/tag signing rather than NIP-34 forge events. The actual Buzz patch, issue, status, pull-request and PR-update builders plus their tests live in `buzz-sdk/src/builders.rs`; Buzz has no Git-scoped NIP-22 comment builder. The port therefore uses those real builders with canonical NIP-34/NIP-22 grammars, while leaving NIP-GS execution to its separately planned signing-helper leaf 25.9._
    - _Evidence: 2026-08-22 — added pure kind-1617 patch, kind-1618 pull-request, kind-1619 update, kind-1621 issue and Git-scoped kind-1111 comment codecs over the canonical repository coordinate/object-ID types. Frozen vectors round-trip patch roots and revisions, SHA-1/SHA-256 commit metadata, PR fetch locations/branch/channel/base, PR-update roots, issues, top-level and nested comment links and bounded future tags. Negative vectors reject conflicting patch ancestry, mismatched comment roots, missing repository-owner links and stale status targets; status linkage checks exact root, author recipient and optional repository identity without deciding authorization. The full crate suite passed 64 unit and four integration tests, and `./script/clippy -p nostr_compat` passed release all-target/all-feature warning-denied checks. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.3. Add hosted repository and permission schema
    - Create hosted coordinates, storage handles and explicit read/write/admin grant tables under ADR-003.
    - _Requirements: 6.2, 10.2_
    - _Capability IDs: CAP-005, CAP-019_
    - _Depends on: 2.3, 24.3, 25.1_
    - _Reads: projects/buzz/crates/buzz-relay/src/git/**, crates/git_hosting_providers/**_
    - _Writes: crates/collab/migrations/collaboration_git.sql_
    - _Validation: migration tests cover tenant fences, grant uniqueness, archive and rollback_
    - _Discovered contradiction (2026-08-22): the planned unversioned `collaboration_git.sql` path is incompatible with the existing SQLx reversible-migration authority, whose immutable history requires a unique timestamped `.up.sql`/`.down.sql` pair. The write set also omitted the focused integration fixture needed to execute tenant RLS, uniqueness and rollback rather than inspect SQL text. The narrow correction adds `20260822000400_collaboration_git.{up,down}.sql` and one migration-only test target; it changes no runtime repository or authorization behavior reserved for Task 25.10._
    - _Evidence: 2026-08-22 — added community-fenced hosted repository, opaque Zed-hosted storage-handle and exact read/write/admin grant tables under ADR-003. Stable repository IDs preserve rename identity; kind-30617 owner/discriminator uniqueness, one complete Sim-hosted-or-external authority tuple, explicit grantor/grantee memberships, optimistic versions, lifecycle timestamps and non-cascading rollback are enforced without project/channel authority, credentials, remotes or local paths. Four focused static/checksum tests passed, and an isolated PostgreSQL 14 run passed the live non-bypass-RLS scenario for cross-tenant invisibility and insert denial, exact grant uniqueness, valid revoke/archive transitions and complete rollback. Warning-denied release Clippy passed for the focused target with dependencies excluded; the full crate script remains blocked by two pre-existing unused imports in `language_model/src/fake_provider.rs`, which this task did not modify. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.4. Implement content-addressed Git object storage adapter
    - Read/write objects and refs with tenant/repository fencing, atomic ref updates and integrity verification.
    - _Requirements: 10.2, 19.2_
    - _Capability IDs: CAP-019_
    - _Depends on: 25.10_
    - _Reads: projects/buzz/docs/git-on-object-storage.md, projects/buzz/crates/buzz-relay/src/git/**_
    - _Writes: crates/collab/src/git/object_store.rs_
    - _Validation: object-store tests cover hash mismatch, concurrent ref update, missing object and cross-tenant path_
    - _Discovered contradiction (2026-08-22): Buzz has no `crates/buzz-relay/src/git/**`; the approved protocol implementation lives under `crates/buzz-relay/src/api/git/{store,manifest,cas_publish}.rs`. The planned production file also cannot prove backend races or corruption behavior by itself. The narrow correction reads those actual sources and adds one focused integration target without introducing smart-HTTP hydration, Git subprocesses, push policy or response construction owned by Tasks 25.5 and 25.6._
    - _Evidence: 2026-08-22 — added the bounded Sim-hosted object adapter over Task 25.10's authorized active repository/storage handle. Versioned community/repository/opaque-handle namespaces exclude caller paths and credentials; packed objects and canonical manifests are create-only SHA-256 values, verified on every read and on idempotent collision, while the sole ref pointer advances by create-only or exact opaque-ETag CAS with SDK retries disabled. Closed manifest/pointer decoding, streaming byte caps, exact parent snapshots, pre-publication verification of every named object, external/archive rejection and generic failures preserve the integrity and least-privilege boundaries. Five focused tests passed deterministic hash corruption, malformed manifest, missing object/pointer, unsafe ref, size, foreign snapshot/path and exactly-one-winner races. The same production AWS adapter passed an isolated MinIO live run for create/read, real conditional-write contention, cross-tenant isolation and injected corruption; its disposable bucket and container were removed. Compilation, warning-denied Clippy, dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.5. Implement Git smart-HTTP read paths
    - Serve discovery and fetch from the hosted adapter with bounded request/response and authorization.
    - _Requirements: 10.2, 19.2_
    - _Capability IDs: CAP-019, CAP-039_
    - _Depends on: 25.4, 25.10_
    - _Reads: projects/buzz/crates/buzz-relay/src/git/**, crates/collab/src/git/object_store.rs_
    - _Writes: crates/collab/src/git/smart_http_read.rs_
    - _Validation: clone/fetch tests cover authorized, private, missing and oversized requests_
    - _Discovered contradiction (2026-08-23): Buzz has no `crates/buzz-relay/src/git/**`; its smart-HTTP hydration and transport implementation lives under `crates/buzz-relay/src/api/git/{hydrate,transport}.rs`. The planned production-only write set also omitted the existing workspace dependencies required for bounded gzip decoding and ephemeral bare repositories, plus the focused integration target needed to exercise a real Git client. The narrow correction reads those actual sources, adds the two existing dependencies and one focused test target without adding receive-pack, push policy or global route wiring owned by later leaves._
    - _Evidence: 2026-08-23 — added a bounded upload-pack service that resolves canonical Task 25.10 read authorization before request decoding, object-store access or subprocess work; denied/private and missing repositories share a generic unavailable result. Authorized snapshots hydrate into ephemeral bare repositories pack-first, verify indexed object connectivity before refs are exposed and disappear after each request. Strict content type and Git protocol validation, compressed/decoded request limits, repository and advertisement/result caps, a non-blocking process semaphore, scrubbed Git environment, timeout/kill handling and bounded stdout collection that continues draining enforce the transport boundary. Three focused tests passed private and missing repositories, pre-I/O oversized denial, malformed content, gzip expansion and advertisement limits, plus real local `git clone` and advancing `git fetch` round trips through Axum. Compilation and warning-denied focused Clippy passed; the unchanged dependency build still reports two pre-existing unused imports in `language_model/src/fake_provider.rs`. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.6. Implement Git smart-HTTP write paths
    - Accept push updates through permission checks and atomic ref/object persistence with audit IDs.
    - _Requirements: 10.2, 13.4, 19.2_
    - _Capability IDs: CAP-019, CAP-028_
    - _Depends on: 25.4, 25.5_
    - _Reads: crates/collab/src/git/{object_store,smart-http-read}.rs_
    - _Writes: crates/collab/src/git/smart_http_write.rs_
    - _Validation: push tests cover fast-forward, force policy, missing object, concurrent update and denied writer_
    - _Discovered contradiction (2026-08-23): the planned read path uses the nonexistent hyphenated `smart-http-read.rs` spelling instead of `smart_http_read.rs`, and the production-only write set omits the Git module registration and focused integration target required to expose the service and prove real receive-pack behavior. The narrow correction reads the actual underscore path, registers one module and adds one test target without global route wiring, provider hosting, branch-protection workflows or the durable audit-chain repository owned by later leaves._
    - _Evidence: 2026-08-23 — added a bounded receive-pack service that resolves canonical Task 25.10 `git:write` authorization before request validation, decoding, object-store access or subprocess work. Each request carries a non-nil canonical `OperationId`; applied and rejected receipts retain that audit attribution plus the exact parent/published manifest digests for the later audit-chain writer. Authorized pushes hydrate one verified parent snapshot, enforce configured fast-forward-only or force-allowed policy inside receive-pack, snapshot bounded post-push refs, verify connectivity, capture one complete reachable pack and expose no success response until immutable-object persistence and the exact parent-pointer CAS both succeed. Missing/corrupt parent objects fail closed, rejected/no-op pushes publish nothing, a lost CAS returns conflict and temporary repositories are removed per request. Four focused tests passed pre-I/O writer denial, real first and fast-forward pushes, denied and explicitly allowed force pushes, non-nil applied/rejected operation receipts, missing stored objects and deterministic CAS loss with the prior ref preserved. Compilation and warning-denied focused Clippy passed; the unchanged dependency build still reports two pre-existing unused imports in `language_model/src/fake_provider.rs`. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.7. Port the NIP-98 Git credential helper
    - Adapt credential lookup to canonical key storage with compatible stdin/stdout and rejection contracts.
    - _Requirements: 7.2, 10.2, 16.4_
    - _Capability IDs: CAP-009, CAP-019, CAP-038_
    - _Depends on: 12.5, 25.10_
    - _Reads: projects/buzz/crates/git-credential-nostr/**, crates/zed_credentials_provider/**_
    - _Writes: tools/git_credential_nostr/Cargo.toml, tools/git_credential_nostr/src/*_
    - _Validation: helper tests cover lookup, locked keyring, denied host, redaction and exact exit codes_
    - _Discovered contradiction (2026-08-23): Git invokes credential helpers out of process, while Zed's canonical `CredentialsProvider` API requires an in-process GPUI `AsyncApp`; directly linking that API cannot retrieve a key for the helper. The planned new-tool-only write set also omits workspace registration and lockfile resolution required to compile it. The narrow correction adds the tool as a workspace member and implements read-only platform adapters for the exact macOS internet-password, Linux Secret Service and Windows Credential Manager layouts already owned by GPUI/Zed, keyed only by the canonical credential identifier. It adds no plaintext/environment/keyfile fallback, credential write/delete path, lifecycle authority or host auto-trust._
    - _Evidence: 2026-08-23 — added a bounded Git credential-protocol helper that acts only on `get`, requires Git's `authtype` capability plus a parseable Nostr method challenge, reconstructs the repository-root HTTP(S) URL and signs kind 27235 only when the exact request host appears in the repository's `nostr.allowedHost` list. A single validated `nostr.credentialIdentifier` selects the canonical Zed record; the read-only platform adapter returns zeroized owned bytes, accepts only the canonical 32-byte secret and requires its derived public key to match the stored public-key username before signing. Optional NIP-OA auth tags remain inside the signature. Nonmatching challenges and non-`get` actions fall through silently; request/host/config rejection exits 1, locked/unavailable protected storage exits 2, a missing credential exits 3 and invalid/signing material exits 4. Successful stdout contains only Git's ephemeral Nostr credential fields, while every error is fixed and secret/identifier-free. Five release tests passed canonical lookup and signature/URL verification, locked keyring, denied-host pre-I/O behavior, redaction and exact success/failure exits; repository-standard warning-denied all-target/all-feature Clippy, formatting and diff hygiene passed. Dependency, inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 25.8. Add NIP-34 and Git-hosting conformance suite
    - Run legacy and consolidated clone, push, patch, sign and permission scenarios against independent fixtures.
    - _Requirements: 10.2, 20.1, 20.2_
    - _Capability IDs: CAP-019, CAP-044_
    - _Depends on: 25.2, 25.6, 25.7, 25.9_
    - _Reads: projects/buzz/crates/buzz-conformance/**, crates/collab/src/git/**, tools/git_*_nostr/**_
    - _Writes: crates/collab/tests/git_conformance.rs_
    - _Validation: `cargo test -p collab git_conformance` passes old/new server and external-provider cases_
    - _Discovered contradiction (2026-08-23): the planned test-only write cannot compile while directly exercising both retained helper libraries without adding their existing packages and `zeroize` as Collab dev-dependencies, so the narrow write set includes `crates/collab/Cargo.toml` and the corresponding lockfile dependency edges without changing production dependencies or runtime wiring. The exact package-filter command also compiles the unrelated `collab_tests` integration target, whose pre-existing `channel_tests.rs` defines `test_rejoining_channel_after_stale_connection_cleanup_connects_livekit` twice; the direct `--test git_conformance` target is therefore the executable acceptance gate for this leaf._
    - _Evidence: 2026-08-23 — added one independent conformance target that drives a real Git client through two consecutive fast-forward pushes and a fresh clone against both Git's reference `git http-backend` CGI and the consolidated read/write smart-HTTP services, then proves identical head object IDs and checked-out bytes. A protocol fixture round-trips a canonical NIP-34 patch, validates the retained NIP-98 helper's kind-27235 event and public-key binding, and signs plus verifies a canonical NIP-GS envelope while proving verification performs zero credential-store reads. Negative fixtures prove denied reads and writes reject malformed input before object access and that external-provider records cannot enter Sim-hosted storage, again before any backend read. `cargo test -p collab --test git_conformance git_conformance -- --nocapture` passed all 3 scenarios; warning-denied target Clippy passed with dependencies excluded, while unchanged dependencies still report the two known `language_model/src/fake_provider.rs` unused-import warnings. The broader planned command was attempted and stopped before this target at the recorded unrelated duplicate-symbol baseline. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.9. Port the Nostr commit and tag signing helper
    - Adapt commit/tag signing and verification to canonical key storage with compatible Git contracts.
    - _Requirements: 7.2, 10.2, 16.4_
    - _Capability IDs: CAP-009, CAP-019, CAP-038_
    - _Depends on: 12.5, 25.10_
    - _Reads: projects/buzz/crates/git-sign-nostr/**, crates/zed_credentials_provider/**_
    - _Writes: tools/git_sign_nostr/Cargo.toml, tools/git_sign_nostr/src/*_
    - _Validation: helper tests cover sign/verify, locked keyring, altered object, redaction and exact exit codes_
    - _Discovered contradiction (2026-08-23): like Git's credential helper, the signing program runs outside GPUI and cannot call the `AsyncApp`-bound `CredentialsProvider` directly. The planned new-tool-only write set also omits workspace/lockfile registration and the minimal public read access needed to reuse Task 25.7's exact platform storage adapter instead of cloning a second set of keychain layouts. The narrow correction registers the helper, exposes only the already-held credential username and zeroizing secret slices and depends on the existing read-only adapter; it adds no credential mutation, environment secret, plaintext keyfile, identity-lifecycle or trust-root authority._
    - _Evidence: 2026-08-23 — added a bounded Git x509 signing backend that accepts the compatible `--status-fd`, `-bsau` and `--verify <file> -` contracts and uses the canonical `nostr_compat` NIP-GS codec for the domain-separated hash, exact compact envelope, three-line armor and verification. Signing resolves one configured canonical protected record, requires its raw 32-byte key to derive both the stored public-key username and Git's requested hex/npub key ID, verifies any configured NIP-OA owner attestation before signing and erases the secret/keypair guards on every return path. Verification reads only a bounded regular signature file, never touches protected storage, emits exact `GOODSIG`/`VALIDSIG`/advisory trust/owner-notation status on success and `BADSIG` for an altered object. Invocation/I/O exits 1, locked/unavailable storage exits 2, a missing credential exits 3, invalid/signing material exits 4 and failed cryptographic verification exits 5; diagnostics are fixed and secret-, identifier- and key-free. Five focused release tests passed canonical sign/verify, locked storage, altered object, redaction and every exit class; warning-denied all-target/all-feature Clippy passed. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 25.10. Implement hosted repository registry and permission checks
    - Read/write hosted repository records and evaluate explicit grants before object or HTTP access.
    - _Requirements: 6.2, 10.2_
    - _Capability IDs: CAP-005, CAP-019_
    - _Depends on: 25.3_
    - _Reads: crates/collab/migrations/collaboration_git.sql, crates/collaboration_domain/src/authorization.rs_
    - _Writes: crates/collab/src/git/repository_registry.rs_
    - _Validation: repository tests cover tenant, permission, rename, archive and external-host coexistence_
    - _Discovered contradiction (2026-08-22): Task 25.3 established the approved Git schema as the timestamped reversible `20260822000400_collaboration_git.{up,down}.sql` pair, so the unversioned migration read path does not exist. The planned nested implementation file also cannot compile without Zed's required non-`mod.rs` module root, and the runtime behaviors cannot be proven by that production file alone. The narrow correction reads the canonical migration pair, adds `crates/collab/src/git.rs` plus the existing library export and adds one focused integration target; it introduces no object storage, HTTP or credential behavior owned by later leaves._
    - _Evidence: 2026-08-22 — added a PostgreSQL hosted repository registry that admits only exact repository-shaped `git:read`, `git:write` or `git:admin` requests allowed by the common authorization policy, resolves scoped tokens to their canonical subject and performs the active-grant check in the same tenant-bound transaction as each read or mutation. Creation atomically installs the creator's explicit admin grant; permission hierarchy, active-member grant/regrant, revocation, optimistic rename and atomic archive are closed operations, while external-provider records retain independent authority without a storage handle or credential. Two focused tests passed pre-I/O tenant/scope denial and the full live lifecycle; an isolated PostgreSQL 14 run under a non-bypass RLS login proved exact permission denial, stable-ID rename, stale-version rejection, grant revocation, archive denial and Sim-hosted/external-provider coexistence. Warning-denied release Clippy passed for the focused target with dependencies excluded; the known unchanged `language_model/src/fake_provider.rs` imports remain the full-script baseline blocker recorded in Task 25.3. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

- [ ] 26. Implement branch-as-channel linkage

  - [x] 26.1. Define branch collaboration identity and state
    - Model repository/branch/commit identity and create, update, merge and archive transitions.
    - _Requirements: 10.3_
    - _Capability IDs: CAP-020_
    - _Depends on: 18.4, 25.1_
    - _Reads: projects/buzz/VISION_PROJECTS.md, crates/project/src/git_store.rs_
    - _Writes: crates/collaboration_domain/src/branch_activity.rs_
    - _Validation: state tests cover branch recreation, force update, merge and stale commit_
    - _Discovered contradiction (2026-08-23): the planned standalone domain file cannot compile or expose its public state API without crate-root module registration, and living-documentation traceability adds the specification paths. The canonical Zed and hosted Git owners alone can inspect ancestry or accept ref mutations, so the pure domain state consumes an explicit fast-forward/force classification after acceptance rather than importing Git, storage, transport or permission behavior._
    - _Evidence: 2026-08-23 — added tenant/repository-scoped branch identity with a safe full heads ref, positive recreation generation and exact lowercase SHA-1/SHA-256 commit links. Create establishes an active first generation; accepted head updates retain the prior/current commits and explicit fast-forward or force classification; merge retains source head, target branch and resulting commit; delete/merge archival preserves those records; and recreation derives a new generation without mutating archived history. Every update, merge, archive and recreation requires both the current aggregate version and expected head, so stale commands fail atomically, while invalid refs, commits, nil scopes and inconsistent hydrated records fail closed. Five focused branch-state tests passed recreation, force update, merge/archive retention, stale commit/version atomicity and identifier rejection; the complete `collaboration_domain` suite passed 131/131 and repository-standard release all-target/all-feature warning-denied Clippy passed. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 26.2. Create and bind branch channels idempotently
    - Create one canonical channel per approved branch binding and reuse it on retries/reconnect.
    - _Requirements: 9.1, 10.3_
    - _Capability IDs: CAP-010, CAP-020_
    - _Depends on: 18.6, 26.1_
    - _Reads: crates/collaboration_domain/src/{channel,branch-activity}.rs_
    - _Writes: crates/collab/src/git/branch_channel.rs_
    - _Validation: tests prove one channel per binding under duplicate and concurrent create_
    - _Discovered contradiction (2026-08-23): the planned hyphenated `branch-activity.rs` read path is the nonexistent spelling of Task 26.1's registered `branch_activity.rs`. The production file also cannot compile without Git module registration, and a real duplicate/concurrency proof requires a focused integration target plus living-documentation traceability. The existing tenant-RLS `collaboration_channels` primary key is already the canonical linearization point, so the narrow correction adds no competing table or migration._
    - _Evidence: 2026-08-23 — added a PostgreSQL branch-channel adapter that rejects inactive branches and runs the canonical channel create authorization before any transaction. It derives one stable UUID from the complete tenant/repository/full-ref/generation identity, proposes a conservative private stream, stores a full SHA-256 binding fingerprint and performs insert-or-read under the existing tenant RLS and channel primary key. A mismatched fingerprint, type, visibility, expiry shape or inactive lifecycle fails closed; a valid existing row is reused without replacing its creator or mutable presentation. The focused target passed pre-storage member/inactive denial and, against disposable PostgreSQL 14, 16 simultaneous creates plus a reconnect all returned the same ID while exactly one channel row existed. `cargo test -p collab --test branch_channel branch_channel -- --nocapture` passed 2/2 with the live race enabled; warning-denied focused Clippy passed with dependencies excluded, while unchanged dependencies still report the known `language_model/src/fake_provider.rs` imports. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 26.3. Project ref updates into immutable activity events
    - Emit stable branch/ref/commit activity records from accepted Git updates.
    - _Requirements: 10.3, 12.1_
    - _Capability IDs: CAP-020, CAP-025_
    - _Depends on: 25.6, 26.1, 26.2_
    - _Reads: crates/collab/src/git/smart_http_write.rs, crates/collab/src/git/branch_channel.rs_
    - _Writes: crates/collab/src/git/branch_activity.rs_
    - _Validation: tests cover retry deduplication, commit links, force update and missing channel recovery_
    - _Discovered contradiction (2026-08-23): the planned production file cannot compile or expose the projector without Git module registration, and the required retry/recovery proof needs a focused integration target plus living-documentation traceability. Task 25.6's receipt intentionally binds the operation only to the parent/published manifest commit point rather than copying every ref; this projector therefore also requires the exact repository-scoped push context and consecutive Task 26.1 before/after branch snapshots. It does not modify receive-pack or make its derived event a second ref authority._
    - _Evidence: 2026-08-23 — added an immutable branch-activity projector that admits only a non-nil applied receive-pack receipt with a published manifest, verifies the exact tenant/repository write shape and classifies either an active first-version creation or one consecutive active head transition. Each emitted record carries a deterministic operation-and-branch event ID, push actor, generation-scoped branch and channel IDs, branch version, exact prior/current commit plus fast-forward/force classification and parent/published manifest digests. The projector resolves the canonical channel before append, so a missing binding recovers through Task 26.2; the sink contract inserts or returns an identical event and rejects conflicts, while rejected/no-op receipts and inconsistent snapshots stop before either dependency. Three focused tests passed stable retry deduplication, missing-channel recovery, initial commit linkage, force-update linkage and pre-dependency rejection. `cargo test -p collab --test branch_activity branch_activity -- --nocapture` passed 3/3 and warning-denied focused Clippy passed with dependencies excluded; unchanged dependencies still report the known `language_model/src/fake_provider.rs` imports. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 26.4. Apply merge and archive channel lifecycle
    - Transition branch channels on merge/delete while preserving immutable conversation and review history.
    - _Requirements: 9.1, 10.3_
    - _Capability IDs: CAP-010, CAP-020_
    - _Depends on: 26.2, 26.3_
    - _Reads: crates/collab/src/git/{branch_channel,branch-activity}.rs_
    - _Writes: crates/collab/src/git/branch_lifecycle.rs_
    - _Validation: lifecycle tests cover merge, delete, reopen, stale events and retained history_
    - _Discovered contradiction (2026-08-23): the planned hyphenated `branch-activity.rs` read path is the nonexistent spelling of the registered `branch_activity.rs`, and the new production lifecycle owner cannot compile without Git module registration. Reusing Task 26.2's generation fingerprint and row hydration without weakening their visibility required narrow sibling visibility changes in `branch_channel.rs`; proving history retention and compare-and-set behavior required the focused PostgreSQL integration target and living-documentation traceability. The existing channel row and canonical channel authorization remain authoritative, so no lifecycle table, competing history store or migration was added._
    - _Evidence: 2026-08-23 — added a PostgreSQL branch lifecycle service that accepts only consecutive Task 26.1 active-to-merged, active-to-deleted or merged-to-merged-archive transitions, locks the exact Task 26.2 generation channel under tenant RLS and applies canonical channel management authorization plus optimistic channel-version fencing. Merge and delete update only lifecycle, version and timestamp on the existing row; provenance, creator, conversation memberships and downstream immutable review/history references are never replaced or deleted. A merged archive replay is an authorized no-op, while stale channel versions and inconsistent branch transitions fail closed. Reopening first verifies the prior generation channel is archived, then binds the next active generation to a distinct canonical channel while retaining the old row. The focused target passed against disposable PostgreSQL 14, covering merge, delete, stale replay, idempotent merged archival, generation reopen, three retained rows and byte-for-byte preservation of the original source record, fingerprint and creator. `cargo test -p collab --test branch_lifecycle -- --nocapture` passed 1/1 with the live database enabled; warning-denied focused Clippy passed with dependencies excluded, while unchanged dependencies still report the known `language_model/src/fake_provider.rs` imports. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 26.5. Add branch-channel reconnect regressions
    - Verify ref activity and channel state converge after duplicate, delayed and disconnected delivery.
    - _Requirements: 8.2, 10.3, 20.1_
    - _Capability IDs: CAP-006, CAP-020, CAP-044_
    - _Depends on: 26.3, 26.4_
    - _Reads: crates/collab/src/git/branch_*.rs_
    - _Writes: crates/collab/tests/branch_channel_recovery.rs_
    - _Validation: `cargo test -p collab branch_channel_recovery` passes reordered and reconnect traces_
    - _Discovered contradiction (2026-08-23): the planned test-only write set could not satisfy delayed delivery after lifecycle closure because Task 26.3 reused the interactive Task 26.2 bind path, which correctly rejects an archived channel even when a delayed immutable event targets that exact validated generation. The narrow production correction adds an activity-only resolution path that permits an existing archived row after the same fingerprint checks while leaving ordinary branch binding active-only. The exact package-wide validation command also exposed two byte-identical definitions of `test_rejoining_channel_after_stale_connection_cleanup_connects_livekit` already present in `channel_tests.rs`; the newer duplicate was removed while retaining the original regression. Living-documentation traceability expands the write set further without adding a second recovery owner._
    - _Evidence: 2026-08-23 — added a live recovery trace spanning the Task 26.2 channel service, Task 26.3 activity projector and Task 26.4 lifecycle service. It archives a channel before delivering its update and creation activity out of order, duplicates the update, rejects a stale lifecycle replay, reconstructs every service connection, replays both events, closes the merged branch, reopens the same ref as the next generation and repeats disconnect/replay. The converged result retains exactly three immutable activity events, one archived generation-one channel at version two and one distinct active generation-two channel at version one; every replay leaves the event set and channel count unchanged. The activity-only resolver still runs canonical channel-create authorization, tenant RLS and complete generation fingerprint validation, and it rejects deleted/expired or mismatched rows. The focused live PostgreSQL 14 target passed 1/1, and the exact `cargo test -p collab branch_channel_recovery` package-wide filtered command passed after building every `collab` target. Warning-denied focused Clippy, dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

- [ ] 27. Complete review, CI and approval timeline integration

  - [x] 27.1. Define canonical review and approval records
    - Model patch revision, review comment, approval and merge readiness linked to repository/commit IDs.
    - _Requirements: 10.3, 10.4_
    - _Capability IDs: CAP-019, CAP-020_
    - _Depends on: 25.2, 26.3_
    - _Reads: projects/buzz/VISION_PROJECTS.md, crates/collaboration_domain/src/branch_activity.rs_
    - _Writes: crates/collaboration_domain/src/review.rs_
    - _Validation: state tests cover stale revision, superseded approval, comment anchor and merge eligibility_
    - _Discovered contradiction (2026-08-23): the planned standalone domain file cannot compile or expose its canonical records without crate-root registration, and review mutations that affect Requirements 10.3/10.4 require living-documentation traceability. The narrow write expansion registers and re-exports the module from `collaboration_domain.rs` and records the state boundary in `design.md`; it adds no persistence, protocol verification, authorization, native diff owner or merge executor._
    - _Evidence: 2026-08-23 — added a bounded, version-fenced review aggregate scoped to the exact generation-specific branch, repository and review identity. Sequential patch revisions retain base/head commits and author; inline comments retain validated relative file, content-derived hunk, diff side and nonempty line range against the exact revision-side commit; approval/change-request decisions retain their approver, revision and head. Stable record IDs make exact retries no-ops and conflicting reuse fail closed. A new revision supersedes every old approval, a later same-approver decision replaces only that current decision, and merge readiness deterministically returns the exact distinct current approval IDs while blocking insufficient approvals or live change requests and rejecting stale revisions/commits. The four focused state regressions passed, the complete `collaboration_domain` suite passed 135/135, and warning-denied all-target Clippy passed with dependencies excluded. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 27.2. Define CI result and workflow-link records
    - Model check suites, runs, statuses and artifact links with bounded untrusted text.
    - _Requirements: 10.3, 19.2_
    - _Capability IDs: CAP-020, CAP-027_
    - _Depends on: 27.1_
    - _Reads: projects/buzz/VISION_PROJECTS.md, crates/collaboration_domain/src/review.rs_
    - _Writes: crates/collaboration_domain/src/ci_status.rs_
    - _Validation: tests cover pending/success/failure/cancel, stale commit and malicious output truncation_
    - _Discovered contradiction (2026-08-23): as with Task 27.1, the planned standalone domain file cannot compile or expose its records without crate-root registration, while Requirement 19.2 output/link behavior and Requirements 10.3 traceability require living-documentation updates. The narrow write expansion registers and re-exports `ci_status` from `collaboration_domain.rs` and records the boundary in `design.md`; it adds no workflow executor, provider transport, artifact fetcher, persistence or CI process owner._
    - _Evidence: 2026-08-23 — added a bounded CI suite aggregate scoped to the exact Task 27.1 review identity, patch revision and head commit. Canonical workflow definition/run links, check runs and unique artifact references retain stable IDs and bounded HTTPS presentation links without fetch authority. Runs move from pending to running or directly to success/failure/cancel, terminal states cannot reopen, exact additions/completions replay idempotently and suite/run versions fence races. Suite status remains pending/running until all runs terminate, then resolves conservatively to failure, cancellation or success. Terminal writes reject stale commits atomically. Untrusted labels and output cap both allocation and inspection, remove non-display controls, truncate only at UTF-8 boundaries and retain sanitization/truncation flags. Four focused regressions passed every planned status, artifact/workflow linkage, stale-commit atomicity and hostile control/oversize output; the complete `collaboration_domain` suite passed 139/139, and warning-denied all-target Clippy passed with dependencies excluded. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 27.3. Persist review, approval and CI projections
    - Add provenance-aware projection tables and repositories without duplicating Git working state.
    - _Requirements: 2.2, 10.3_
    - _Capability IDs: CAP-005, CAP-020_
    - _Depends on: 27.1, 27.2_
    - _Reads: crates/collab/migrations/collaboration_git.sql, crates/collaboration_domain/src/{review,ci-status}.rs_
    - _Writes: crates/collab/src/git/review_repository.rs_
    - _Validation: repository tests cover revision replacement, provenance rebuild and tenant isolation_
    - _Discovered contradiction (2026-08-23): the planned unversioned `collaboration_git.sql` input does not exist; the implemented Git schema is the already validated immutable `20260822000400_collaboration_git.{up,down}.sql` pair. The planned `ci-status.rs` spelling also differs from the canonical `ci_status.rs`. The narrow correction adds a new reversible timestamped projection migration rather than altering existing migration history, registers the repository module, adds the named integration target and exposes bounded CI record hydration so persisted sanitization/truncation flags round-trip without reprocessing. Living-documentation traceability expands the write set further. This leaf adds no patch bytes, diff, index, worktree, ref mutation or duplicate Git working-state owner._
    - _Evidence: 2026-08-23 — added community-keyed review and generation-scoped CI projection tables with forced row-level security, hosted-repository foreign keys, bounded JSONB payloads, complete source provenance and canonical SHA-256 integrity hashes. The repository rejects cross-tenant input before I/O, fences stale or conflicting aggregate versions, atomically replaces a newer review snapshot and rebuilds derived CI rows on exact replay while retaining the aggregate's complete revision, comment and approval history. Load reconstructs bounded domain records and verifies suite count, identity metadata and every payload/projection hash. The focused PostgreSQL 14 integration target passed 2/2, covering initial round-trip, revision replacement, stale rejection, an injected missing-CI-row provenance rebuild and both repository/API and direct-RLS tenant isolation. Warning-denied Clippy, domain hydration regressions, dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 27.4. Project Git, review and CI events into ActivityItem
    - Map all collaboration code events to verb/object/outcome classes and truthful fallbacks.
    - _Requirements: 10.3, 12.1, 12.2_
    - _Capability IDs: CAP-020, CAP-025_
    - _Depends on: 8.4, 26.3, 27.3_
    - _Reads: crates/agent_ui/src/activity_projection.rs, crates/collaboration_domain/src/{branch_activity,review,ci-status}.rs_
    - _Writes: crates/agent_ui/src/activity_git.rs_
    - _Validation: activity fixture test maps each Git/review/CI kind exactly once_
    - _Discovered contradiction (2026-08-23): the planned standalone projector file cannot compile or become reachable because `agent_ui` did not depend on `collaboration_domain` and its crate root did not register the module; the planned `ci-status.rs` spelling again differs from canonical `ci_status.rs`. The narrow correction adds the domain dependency only to `agent_ui`'s existing `multiplayer-tools` feature, updates the lockfile and registers the feature-gated module. Living-documentation traceability expands the write set further. No standard-build dependency, event store, Git owner, review owner or workflow executor is introduced._
    - _Evidence: 2026-08-23 — added one typed projection boundary for branch create, fast-forward, force-update, merge and delete; patch submission; inline review comments; approval and change requests; and pending, running, successful, failed and cancelled CI suites. Known items derive tenant, repository, branch, review, revision, commit, actor and workflow identities from canonical domain records, expose verb/object/outcome semantics and carry stable entity plus Git-change links for later native-diff resolution. A bounded generic fallback preserves unknown Git/workflow/CI activity with an explicit unknown outcome and raw detail rather than dropping it. The reducer regression proves live CI advances in place while immutable approval history deduplicates instead of changing a terminal outcome. The focused Multiplayer fixture passed 3/3 with all 15 explicit kinds mapped once, and warning-denied focused Clippy passed with dependencies excluded. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 27.5. Resolve review events to native diff state
    - Map stable repository/revision/file/hunk identities and expose stale/conflict outcomes.
    - _Requirements: 10.3, 10.4_
    - _Capability IDs: CAP-020_
    - _Depends on: 9.2, 27.1, 27.3_
    - _Reads: crates/agent_ui/src/activity_diff_link.rs, crates/git_ui/src/project_diff.rs_
    - _Writes: crates/git_ui/src/collaborative_review.rs_
    - _Validation: diff tests cover exact, moved, stale, deleted and conflicting anchors_
    - _Discovered contradiction (2026-08-23): the planned single-file resolver cannot consume the canonical review/revision/file/hunk types because `git_ui` had no `collaboration_domain` dependency. The narrow correction adds that dependency only to `git_ui`'s existing `multiplayer-tools` feature, adds the test-only UUID fixture dependency and updates the lockfile. Living-documentation traceability expands the write set further. The resolver remains an in-memory adapter over the exact existing `Project`, `GitStore` and `ProjectDiff` identities and introduces no standard-build dependency, Git object store, patch-byte copy, diff computation or mutation owner._
    - _Evidence: 2026-08-23 — canonical review comments now produce anchors bound to repository, review, patch revision, selected base/head commit, bounded stable file identity, hunk identity and diff side. A source-fenced native index accepts current `ProjectPath`/point ranges, explicit deletions and native conflict state while rejecting duplicate or contradictory file facts. Resolution returns exact and moved native targets, explicit deleted/conflicting outcomes, or typed staleness for replaced sources and repository/review/revision/commit/file/hunk drift. The focused Multiplayer Git UI fixture passed 2/2, including all five required exact, moved, stale, deleted and conflicting outcomes, and warning-denied focused Clippy passed with dependencies excluded. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 27.6. Render collaborative review and CI cards
    - Add native cards, actions and progressive details to timeline/review surfaces using canonical records.
    - _Requirements: 4.1, 10.3, 10.4, 12.2_
    - _Capability IDs: CAP-020, CAP-025, CAP-036_
    - _Depends on: 27.4, 27.5_
    - _Reads: crates/agent_ui/src/activity_git.rs, crates/git_ui/src/collaborative_review.rs_
    - _Writes: crates/collab_ui/src/git_activity.rs_
    - _Validation: GPUI tests cover pending CI, approval, conflict, stale review and valid native actions_
    - _Discovered contradiction (2026-08-23): the planned standalone card file cannot compile or become reachable because `collab_ui` did not depend on `git_ui` and its crate root did not register the module. The narrow correction adds `git_ui` only to `collab_ui`'s existing `multiplayer-tools` feature, registers the feature-gated module and updates the lockfile. Living-documentation traceability expands the write set further. The card remains an ephemeral presentation adapter over canonical activity and native review records and introduces no standard-build dependency, Git/workflow/review owner or mutation path._
    - _Evidence: 2026-08-23 — added a native GPUI card for canonical Git, review, workflow and CI activity with explicit pending, running, terminal, conflict, stale and deleted states, immutable approval presentation and progressively disclosed source/native-hunk details. Exact and moved reviews expose only advertised stage/review actions; requests retain the exact project-change source revision and route through the existing workspace authorization boundary, while conflicting, stale and deleted reviews expose no action. Five focused GPUI regressions passed for pending CI, approval, conflict, stale review and valid native action routing; warning-denied focused Clippy and a clean no-default-features library build passed. The initially sandboxed WebRTC archive download required a network-enabled retry, and the focused test uses `collab_ui`'s declared `test-support` feature to avoid a known repository feature-unification mismatch in `remote_connection`. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

  - [x] 27.7. Add end-to-end patch-to-merge scenario
    - Exercise human/agent patch, CI, review, approval and merge with timeline-to-hunk navigation.
    - _Requirements: 10.2, 10.3, 10.4, 20.1_
    - _Capability IDs: CAP-019, CAP-020, CAP-044_
    - _Depends on: 25.8, 26.5, 27.6_
    - _Reads: crates/collab_ui/src/git_activity.rs, crates/collab/tests/git_conformance.rs_
    - _Writes: crates/collab_ui/tests/patch_review_merge.rs_
    - _Validation: end-to-end test completes valid merge and visibly blocks stale/conflicting variants_
    - _Discovered contradiction (2026-08-23): `collab_ui` exposes the Task 27 timeline, native-review and domain APIs only through its optional `multiplayer-tools` feature, and the GPUI integration fixture requires its existing `test-support` feature. The planned test file is therefore feature-gated and its executable acceptance command names both existing features; no package dependency, production source or runtime owner is added. As with Task 25.8, a package-filtered command that omits the required feature set would not execute the intended integration case._
    - _Evidence: 2026-08-23 — added one GPUI integration scenario that opens an agent-authored canonical patch, records an anchored human review comment, projects the patch/comment into native activity cards and routes the exact source-tokened project review action to the resolved hunk. The same trace projects CI as pending, running and successful, records an immutable human approval, proves current review readiness plus green CI and applies the canonical branch merge while retaining source/result commits. A replacement patch revision produces the typed stale-readiness failure; stale and conflicting review cards expose their explicit statuses, advertise no actions and reject direct action requests while the blocked branch remains active and unmerged. The focused end-to-end target passed 1/1 and warning-denied target Clippy passed with dependencies excluded. Dependency, inventory, canonical specification, formatting and diff-hygiene gates are recorded in the enclosing checkpoint commit._

## Milestone 5 — agent platform convergence

- [ ] 28. Adapt Buzz channel and observer ingress to Zed ACP/MCP

  - [x] 28.1. Implement NIP-AO control and observer codecs
    - Parse encrypted control/observer frames, versions and privacy gates independently of ACP execution.
    - _Requirements: 5.3, 11.1, 12.1_
    - _Capability IDs: CAP-021, CAP-025_
    - _Depends on: 11.8, 12.5_
    - _Reads: projects/buzz/docs/nips/NIP-AO.md, projects/buzz/crates/buzz-acp/**_
    - _Writes: crates/nostr_compat/src/agent_observer.rs_
    - _Validation: golden codec tests cover versions, encryption, malformed frames and unauthorized observers_
    - _Discovered contradiction (2026-08-23): Task 11.8 already owns the pure NIP-AO envelope and decrypted-payload codec, while Task 30.1 explicitly retains NIP-44 encryption, decryption and zeroization. This leaf therefore adds the missing authenticated ingress-policy adapter instead of duplicating either boundary. The planned standalone source file also requires the narrow crate-root module registration in `crates/nostr_compat/src/nostr_compat.rs` to compile and expose the codec. The Buzz checkout supplied outside this worktree was made visible through a temporary, untracked symlink only while running compile-time fixture and inventory checks; no Buzz source was written._
    - _Evidence: 2026-08-23 — added a signed NIP-AO ingress adapter that accepts only exact kind-24200 tag shapes with canonical opaque NIP-44 v2 ciphertext, derives and checks telemetry/control direction, binds the authenticated recipient and caller-proven current agent owner, freshness-gates controls to ±300 seconds and authorizes only recipient-exact live filters. Caller-decrypted content is bounded and typed into the four known telemetry kinds or UUID-bound `cancel_turn`; malformed frames and payloads fail closed while future frames and telemetry kinds remain ignored without ACP execution. Four focused golden/privacy tests passed, including v2 version rejection, malformed and extra tags, wrong recipient, unauthorized owner, stale control, broad/history filters and forward-compatible unknown values. The complete `nostr_compat` suite passed 72/72 tests (68 unit plus four integration), and warning-denied release all-target/all-feature Clippy passed. Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 28.2. Map collaboration threads to native ACP sessions
    - Resolve channel/thread/job identities to exactly one native session and preserve cancellation ownership.
    - _Requirements: 2.1, 11.1_
    - _Capability IDs: CAP-021_
    - _Depends on: 19.7, 28.1_
    - _Reads: crates/agent/src/**, crates/acp_thread/src/**, crates/collaboration_domain/src/message.rs_
    - _Writes: crates/agent/src/collaboration_session.rs_
    - _Validation: `cargo test -p agent collaboration_session` proves idempotent create/resume and exactly-one executor_
    - _Discovered contradiction (2026-08-23): the planned standalone source must be registered in `crates/agent/src/agent.rs`, and executable isolation requires `crates/agent/tests/collaboration_session.rs`. The exact package-filtered validation command compiles all existing `agent` unit-test code before applying the name filter and is blocked there by five pre-existing `LanguageModelToolUseInput` versus `serde_json::Value` mismatches in `agent.rs` and `thread.rs`; the dedicated integration target executes the intended cases without compiling those unrelated unit fixtures. The all-target repository Clippy wrapper likewise remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`._
    - _Evidence: 2026-08-23 — added an in-memory coordination registry over community-scoped channel, thread and job identities and native `acp::SessionId` values. Resolution reserves before creation, returns the same lease for a repeated create, returns the existing native ID for same-executor resume and rejects a competing executor. Activation is idempotent for the same binding, rejects both a second native session for one identity and reuse of one native session by another identity, while checked generations fence aborted reservations and authorize cancellation only for the active owner before exact binding cleanup. Invalid/nil identities and blank native session IDs fail closed. The dedicated integration target passed 4/4 tests, `cargo check -p agent --lib` passed and focused warning-denied release Clippy passed with dependencies excluded; the exact broader test and Clippy blockers are recorded above. Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 28.3. Route authorized mentions into ACP prompts
    - Convert supported human/agent mentions to native prompt requests after membership and permission checks.
    - _Requirements: 6.2, 11.1_
    - _Capability IDs: CAP-011, CAP-021_
    - _Depends on: 19.7, 28.2_
    - _Reads: crates/collaboration_domain/src/authorization.rs, crates/agent/src/collaboration_session.rs_
    - _Writes: crates/agent/src/collaboration_mention.rs_
    - _Validation: tests cover direct/team mention, duplicate event, unauthorized actor and busy session_
    - _Discovered contradiction (2026-08-23): the adapter must call the common authorization policy with canonical `Message` and principal evidence, so `agent` requires a direct `collaboration_domain` dependency and the corresponding `Cargo.lock` edge. It also needs crate-root registration, a lease-validating accessor on Task 28.2's registry and an isolated integration target; these narrow supporting writes are `crates/agent/{Cargo.toml,src/agent.rs,src/collaboration_session.rs,tests/collaboration_mention.rs}`. The required all-target repository Clippy wrapper remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`._
    - _Evidence: 2026-08-23 — added an authorization-first mention router that accepts canonical visible messages, exact conversation/write authorization, resolved direct or bounded/deduplicated team targets and only the active generation-fenced session lease. Human, scoped-token and owner-attested-agent subject shapes are rechecked before common policy; tenant, community, channel, membership, role, scope and resource mismatches collapse to a closed unauthorized outcome. Successful routing returns one native `acp::PromptRequest` without executing or copying a transcript. Per-session dispatch generations reject stale completion, a busy session rejects a second event, abort permits retry and a bounded 4,096-event completion window suppresses duplicate source events. Four focused tests passed for direct/team conversion, in-flight/completed duplicates, unauthorized membership and busy/retry behavior; all four Task 28.2 session regressions, `cargo check -p agent --lib` and focused warning-denied release Clippy also passed. Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 28.4. Publish ACP lifecycle through observer adapters
    - Translate native session/action outcomes to bounded NIP-AO frames without creating a second transcript.
    - _Requirements: 11.1, 12.1, 12.3_
    - _Capability IDs: CAP-021, CAP-025_
    - _Depends on: 28.1, 28.2_
    - _Reads: crates/acp_thread/src/**, crates/nostr_compat/src/agent_observer.rs_
    - _Writes: crates/acp_thread/src/collaboration_observer.rs_
    - _Validation: observer tests cover streaming, terminal outcomes, cancellation, redaction and retry deduplication_
    - _Discovered contradiction (2026-08-23): the planned standalone adapter source must be registered by `crates/acp_thread/src/acp_thread.rs`, consume the canonical `ObserverTelemetry` and plaintext ceiling through a direct `nostr_compat` dependency recorded in `crates/acp_thread/Cargo.toml` and `Cargo.lock`, and expose its focused cases through `crates/acp_thread/tests/collaboration_observer.rs`. These narrow supporting writes compile and exercise the specified adapter without moving signing, encryption, publication, persistence or transcript authority into `acp_thread`._
    - _Evidence: 2026-08-23 — added a channel/session-bound native observer adapter that emits monotonic `turn_started`, `acp_read`, `acp_write` and terminal `session_resolved` telemetry. Raw ACP frames are reduced to bounded method/shape metadata before serialization, so request IDs, params, results, errors, prompts and credentials cannot form a second transcript; native action IDs remain stable across monotonic pending-to-terminal updates, cancellation maps to a single terminal outcome, and resolved turns reject further mutation. Bounded 800-entry source, action and resolved-turn windows suppress transport and semantic retries without consuming sequence numbers. The focused integration target passed 4/4 streaming, action-update, completion, cancellation, redaction and retry cases; `cargo check -p acp_thread --lib`, focused warning-denied Clippy and `./script/clippy -p acp_thread` in release/all-target/all-feature mode passed. Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 28.5. Add Buzz MCP tool compatibility mappings
    - Map shell/read/edit/search/tree/image/todo requests to native tools and existing permission prompts.
    - _Requirements: 11.1, 19.2_
    - _Capability IDs: CAP-022_
    - _Depends on: 4.2, 28.2_
    - _Reads: projects/buzz/crates/buzz-dev-mcp/**, crates/agent/src/tools/**_
    - _Writes: crates/agent/src/buzz_tool_compat.rs_
    - _Validation: tool-by-tool tests cover success, denial, invalid path, bounded output and cancellation_
    - _Discovered contradiction (2026-08-23): the planned standalone mapper cannot compile or expose its contract without crate-root registration, and executable tool-by-tool coverage requires the focused `crates/agent/tests/buzz_tool_compat.rs` target. Buzz has no native Zed todo tool, so its session list maps to canonical ACP plan state instead of creating another store. Buzz's `replace_all`, recursive tree rendering and remote/data-URL image acquisition cannot be truthfully represented by the current native owners without bypassing stable unique-edit or project path/SSRF policy; these cases fail closed and remain on the temporary legacy boundary for Task 28.6's differential decision. The all-target repository Clippy wrapper remains blocked before this leaf by the pre-existing unused `EmptyView` and `AppContext` imports in `crates/language_model/src/fake_provider.rs`._
    - _Evidence: 2026-08-23 — added a strict compatibility mapper from Buzz shell, read, edit, search, tree, local-image and todo request shapes to the existing native terminal, read-file, edit-file, grep, list-directory and ACP plan owners. The mapper emits only native tool names and schemas so execution retains the native registry's availability, permission, sandbox and cancellation paths; denied tools and pre-dispatch cancellation fail before a native call. Project-relative path normalization rejects absolute paths, traversal, controls and remote image sources. Command/edit/path/todo inputs, timeouts, line windows and output tails are bounded; search globs are normalized, todo text retains Buzz's character, duplicate and spoofing checks, and debug output redacts arguments and plan content. Seven focused regressions passed every supported owner plus denial, traversal/absolute/remote paths, unsupported semantic drift, UTF-8 tail bounds, redaction and cancellation. `cargo check -p agent --lib` and focused warning-denied release Clippy passed; Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 28.6. Add ACP/MCP lifecycle conformance suite
    - Differentially test legacy harness and native runtime for prompts, tools, queues, cleanup and observer output.
    - _Requirements: 11.1, 11.5, 20.2_
    - _Capability IDs: CAP-021, CAP-022, CAP-044_
    - _Depends on: 28.3, 28.4, 28.5_
    - _Reads: projects/buzz/crates/{buzz-acp,buzz-agent,buzz-dev-mcp}/**, crates/agent/src/collaboration_*.rs_
    - _Writes: crates/agent/tests/buzz_acp_conformance.rs_
    - _Validation: `cargo test -p agent buzz_acp_conformance` passes reentrancy, crash and resource-cleanup cases_
    - _Discovered contradiction (2026-08-23): the retained Buzz crates are an external source checkout rather than Zed workspace members; adding them as path dev-dependencies would couple the consolidated test graph to the second runtime and violate Requirement 20.2's independent checker boundary. The narrow test therefore freezes the observable Buzz session/pool/MCP lifecycle rules in a test-owned reference harness and drives the real native adapters without importing either production reducer. The exact package-filtered command compiles unrelated `agent` unit-test fixtures before applying its name filter and remains blocked there by five pre-existing `LanguageModelToolUseInput` versus `serde_json::Value` mismatches in `agent.rs` and `thread.rs`; the direct `--test buzz_acp_conformance` target is the executable acceptance gate._
    - _Evidence: 2026-08-23 — added three differential scenarios over the frozen Buzz contract and the Task 28.2–28.5 native adapters. A first prompt emits sequence one, a reentrant event reports busy without being consumed, crash aborts its generation-fenced dispatch, rejects stale completion, emits exactly one cancelled terminal frame, releases both native session indexes and permits the same source event after a fresh executor/session generation. All shell/read/edit/search/tree/local-image/todo surfaces resolve to the expected native owner or ACP plan state, while denied and pre-cancelled requests stop before dispatch. Observer action retries consume no sequence and resolved turns reject later activity. The dedicated conformance target passed 3/3; the complete focused adapter matrix passed 22/22 across conformance, tool, mention, session and observer targets; `cargo check -p agent --lib` and warning-denied target Clippy passed. The exact broader-command baseline blocker is recorded above. Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

- [ ] 29. Port personas, teams and private managed-agent state

  - [x] 29.1. Port persona pack parsing and merge rules
    - Parse persona metadata/content and deterministic inheritance without runtime or UI concerns.
    - _Requirements: 11.2_
    - _Capability IDs: CAP-023_
    - _Depends on: 11.7_
    - _Reads: projects/buzz/crates/buzz-persona/**, projects/buzz/docs/nips/NIP-AP.md_
    - _Writes: crates/agent_settings/src/persona.rs_
    - _Validation: parser fixtures cover valid, inherited, conflicting and malformed packs_
    - _Discovered contradiction (2026-08-23): the planned standalone source file cannot compile as a public settings boundary without crate-root registration and production JSON/YAML dependency declarations, so the narrow supporting writes are `crates/agent_settings/src/agent_settings.rs`, `crates/agent_settings/Cargo.toml` and `Cargo.lock`. Buzz's legacy loader accepts uppercase persona names, while the already-approved consolidated design and NIP-AP require `^[a-z0-9][a-z0-9_-]{0,63}$`; the canonical parser intentionally applies the stricter public grammar._
    - _Evidence: 2026-08-23 — added a pure in-memory persona-pack parser with bounded manifest/frontmatter/body/instructions, safe pack-relative paths, strict YAML frontmatter, manifest-ordered output and redacted diagnostics. Pack version, model, temperature, context, subscription and reply defaults inherit deterministically; explicit empty subscription arrays override defaults; persona trigger objects shallow-replace pack triggers and fill only built-in trigger defaults. Pack instructions remain separate from prompts, and parsed MCP/hook declarations remain inert metadata. Four focused fixtures passed valid, inherited/overridden, conflicting and malformed packs; the complete `agent_settings` suite passed 44/44. Rust formatting, warning-denied crate Clippy, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 29.2. Define agent-team and catalog records
    - Model team membership, roles, catalogs and public share records with owner attestations.
    - _Requirements: 7.1, 11.2_
    - _Capability IDs: CAP-007, CAP-023_
    - _Depends on: 12.2, 29.1_
    - _Reads: projects/buzz/docs/nips/NIP-AP.md, projects/buzz/desktop/src/features/agents/**_
    - _Writes: crates/agent_settings/src/team.rs_
    - _Validation: tests cover duplicate member, revoked identity, public catalog and owner change_
    - _Discovered contradiction (2026-08-23): the planned standalone settings record cannot compile or expose its public API without `crates/agent_settings/src/agent_settings.rs` registration. `agent_settings` cannot import the multiplayer-only identity/protocol crates without violating the Standard Zed dependency boundary, so the record consumes canonical lowercase public-key/event-id values plus upstream-verified attestation evidence and active/revoked status; signature, membership and revocation verification remain with the existing identity owners. Buzz teams contain ordered persona IDs but no roles or spawned identities, while Requirement 11.2 and the validation cases require the consolidated record to add bounded roles, active identity binding and exact re-attestation on owner change._
    - _Evidence: 2026-08-23 — added pure bounded records for Nostr identities, proof references, owner-attestation evidence with exact conditions, active/revoked agent identity, local/published persona references, roles and ordered team membership. Team construction rejects duplicate members, revoked identities and owner/agent mismatches; ownership changes require an exact complete replacement-attestation set and apply atomically. Explicit public persona, embedded team-member/team and owner-scoped catalog records retain NIP-AP's separate persona-slug and team-coordinate grammars while excluding filesystem, environment, credential and runtime-process fields. Four focused tests passed duplicate member, revoked identity, embedded public catalog and complete/incomplete owner change; the complete `agent_settings` suite passed 48/48. Rust formatting, warning-denied crate Clippy, diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 29.3. Define private managed-agent configuration
    - Model runtime, model, provider, environment references and PMA expected-version transitions without secret values.
    - _Requirements: 11.2, 19.2_
    - _Capability IDs: CAP-023_
    - _Depends on: 29.1, 29.2_
    - _Reads: projects/buzz/docs/nips/NIP-PMA.md, projects/buzz/desktop/src-tauri/src/managed-agents/**_
    - _Writes: crates/agent_settings/src/managed_agent.rs_
    - _Validation: tests cover CAS, invalid provider/model, secret-reference-only storage and stale update_
    - _Discovered contradiction (2026-08-23): the planned `managed-agents/**` source path does not exist; Buzz uses `managed_agents/**`, which was inspected. The standalone settings module also requires narrow crate-root registration in `crates/agent_settings/src/agent_settings.rs`. Buzz's private PMA payload can carry literal nsec, environment and backend secret material, while this leaf and Requirement 19.2 explicitly prohibit secret values at the settings boundary, so the consolidated record exposes only typed process-environment and protected-credential references; encryption and secret custody remain with their existing owners._
    - _Evidence: 2026-08-23 — added pure bounded runtime, provider and opaque model identifiers, POSIX-shaped environment target/source names, protected-credential references and a managed-agent configuration with no literal-value variant. Diagnostics redact credential and source-environment identifiers. The private PMA record binds distinct owner/agent keys, current event and JavaScript-safe generation, validates the generation-one predecessor exception during construction and hydration, and performs exact generation-plus-event compare-and-swap replacement or tombstoning while retaining the predecessor. Stale updates are non-mutating; duplicate event IDs, generation exhaustion and post-delete resurrection fail closed. Four focused tests passed CAS/predecessor/hydration, invalid provider/model, secret-reference-only storage/redaction and stale-update atomicity; the complete `agent_settings` suite passed 52/52. Rust formatting and warning-denied crate Clippy passed; diff hygiene, collaboration dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 29.4. Implement public/private agent projection rules
    - Derive redacted public persona/team/catalog records from private runnable state and reject secret projection.
    - _Requirements: 11.2, 19.2_
    - _Capability IDs: CAP-023_
    - _Depends on: 29.2, 29.3_
    - _Reads: crates/agent_settings/src/{team,managed-agent}.rs, projects/buzz/docs/nips/NIP-PMA.md_
    - _Writes: crates/collaboration_domain/src/agent_config.rs_
    - _Validation: exhaustive redaction test proves credentials/environment values never enter public events_
    - _Discovered contradiction (2026-08-23): the planned settings read path spells `managed-agent.rs`, while Task 29.3 created the Rust-canonical `managed_agent.rs`. More importantly, `collaboration_domain` cannot depend on `agent_settings`: doing so would pull GPUI, settings, project and filesystem dependencies through the enforced domain boundary. The projection therefore defines a transport-neutral private-source/public-output contract using the domain's canonical identity and attestation types; later approved adapters translate settings records at the composition layer. Narrow supporting writes register/re-export the module in `collaboration_domain.rs` and synchronize this design record._
    - _Evidence: 2026-08-23 — added a one-way bounded projection from private persona, team, catalog and managed-agent source records to a static public schema. Standalone personas retain their validated public slug, while embedded team personas omit local/publication identity; teams revalidate owner, exact member attestation binding, role, coordinate, duplicate and size invariants. Public output has no managed-agent, PMA-version, environment, credential, backend, local-path or response-allowlist field. A closed exhaustive field classifier rejects all six private field classes, and compile-time exhaustive destructuring covers every source field before serialized-output assertions prove four distinct private sentinels and all private field names are absent. Four focused projection/redaction tests and the complete `collaboration_domain` suite passed 143/143; Rust formatting and warning-denied crate Clippy passed. Diff hygiene, dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 29.5. Persist managed-agent state and snapshots
    - Store private versions and public projection provenance through canonical agent/settings owners.
    - _Requirements: 2.2, 11.2_
    - _Capability IDs: CAP-005, CAP-023, CAP-024_
    - _Depends on: 29.3, 29.4_
    - _Reads: crates/agent_settings/src/**, crates/agent/src/db.rs_
    - _Writes: crates/agent/src/managed_agents.rs_
    - _Validation: repository tests cover CAS, restart, projection rebuild and corrupt snapshot_
    - _Discovered contradiction (2026-08-23): `crates/agent/src/db.rs` is the legacy thread database rather than the shared application database owner. The repository therefore registers a narrow `db::Domain` migration over `db::AppDatabase`, and the standalone module requires crate-root registration in `crates/agent/src/agent.rs`. The exact package-filtered unit-test command compiles unrelated `agent` fixtures and remains blocked by five pre-existing `LanguageModelToolUseInput` versus `serde_json::Value` mismatches in `agent.rs` and `thread.rs`; a dedicated integration target is the executable acceptance gate, requiring the narrow supporting write `crates/agent/tests/managed_agents_repository.rs`._
    - _Evidence: 2026-08-23 — added strict shared-SQLite persistence for versioned private managed-agent snapshots and derived public projection provenance. Snapshot encoding stores only the validated settings record and typed environment/credential references; decoding re-enters every bounded constructor and rejects malformed, unknown-version or column/document mismatches without deleting recovery evidence. Exact generation-plus-event CAS and public-projection invalidation share one savepoint; stale writes are non-mutating. Projection rebuilds require the current active source record, matching owner and redaction-safe JSON, retain source generation/event, deterministic revision and projection time, and return stale when the private source advances. The dedicated integration target passed all 4 CAS, restart, projection-rebuild and corrupt-snapshot tests; `cargo check -p agent --lib` passed. Rust formatting, focused warning-denied Clippy, diff hygiene, dependency boundaries, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 29.6. Add native persona and team management UI
    - Render catalog, persona, team and managed-agent editing with privacy and validation feedback.
    - _Requirements: 4.4, 11.2_
    - _Capability IDs: CAP-023, CAP-036_
    - _Depends on: 29.5_
    - _Reads: crates/agent_ui/**, crates/agent/src/managed_agents.rs_
    - _Writes: crates/agent_ui/src/collaborative_agent_settings.rs_
    - _Validation: GPUI tests cover create, share, private edit, conflict, revoked owner and validation error_
    - _Discovered contradiction (2026-08-23): the standalone UI source requires narrow crate-root registration in `crates/agent_ui/src/agent_ui.rs`, and its canonical agent/projection dependencies are available only under `agent_ui`'s existing `multiplayer-tools` feature, so the surface is gated with that feature. The settings record carries lifecycle status for the owner-attested agent identity but no distinct owner-account lifecycle field; the requested revoked-owner feedback therefore applies when that upstream owner-attested agent identity is revoked, while canonical owner authority remains with the identity owner._
    - _Evidence: 2026-08-23 — added a focusable, theme-token-based native GPUI surface with narrow-window scrolling and persona, team and managed-agent catalog sections. Native controls emit explicit create/edit/share requests; validated drafts create and edit settings records, private managed-agent changes use the Task 29.5 repository's exact generation-plus-event CAS, stale writes refresh canonical state and remain visibly conflicted, and sharing uses the Task 29.4 one-way projection before persistence. The UI never renders environment names or credential references, shows only a private-reference count and explicit privacy guidance, and distinguishes saved, shared, conflict, revoked owner-attested identity, validation and unavailable states. Three focused GPUI tests passed create/share/private edit with serialized redaction assertions, external stale-write conflict and refresh, and revoked/invalid feedback. `cargo check -p agent_ui --features multiplayer-tools`, Zed's warning-denied release/all-targets `./script/clippy -p agent_ui --no-all-features --features multiplayer-tools --no-deps`, Rust formatting and diff hygiene passed; canonical specification, dependency and Buzz inventory gates are recorded in the enclosing checkpoint commit._

- [ ] 30. Consolidate engrams, snapshots, archives and metrics

  - [x] 30.1. Implement NIP-AE engram coordinate and encryption codecs
    - Preserve encrypted coordinates, relay scope and owner-read privacy independently of storage.
    - _Requirements: 5.3, 11.3_
    - _Capability IDs: CAP-024_
    - _Depends on: 11.8, 12.5_
    - _Reads: projects/buzz/docs/nips/NIP-AE.md, projects/buzz/desktop/src/features/agent-memory/**_
    - _Writes: crates/nostr_compat/src/agent_memory.rs_
    - _Validation: codec tests cover round trip, wrong owner, rotation and malformed coordinate_
    - _Discovered contradiction (2026-08-23): the standalone codec requires narrow registration in `crates/nostr_compat/src/nostr_compat.rs`, and interoperable NIP-44 v2 encryption must use the workspace-pinned `nostr` implementation with its `nip44` feature, requiring the direct dependency allowlist, `crates/nostr_compat/Cargo.toml` and `Cargo.lock` to advance together. The crate's existing compile-time Buzz conformance fixtures also require a temporary worktree-local `projects/buzz` symlink to the separately supplied source root during tests; the link is not committed and no Buzz file is changed._
    - _Evidence: 2026-08-23 — added exact canonical `30174:<agent>:<blinded-d>` coordinates with explicit owner scope, agent-only NIP-44 v2 encryption and symmetric agent/owner decryption that derives and verifies the supplied reader identity before ciphertext work. Decrypted bodies re-enter strict Task 11.8 parsing and blinded-slug validation; ciphertext and error diagnostics expose no plaintext or wire payload. Added bounded NIP-65 write-relay/fallback resolution with canonical WebSocket equality, retained advertised connection URLs and an explicit old/new union for safe head republication during relay rotation. Four focused tests passed owner/agent round trip, wrong-owner denial and redaction, owner-plus-relay rotation, malformed coordinates and unusable relay scope; the complete `nostr_compat` library suite passed 72/72. Repository-standard warning-denied release/all-target/all-feature Clippy, dependency boundaries, Rust formatting and diff hygiene passed; Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 30.2. Implement canonical encrypted memory storage
    - Persist engram metadata/ciphertext and retention state without service-side plaintext access.
    - _Requirements: 11.3, 15.2_
    - _Capability IDs: CAP-005, CAP-024, CAP-030_
    - _Depends on: 30.1_
    - _Reads: crates/agent/src/db.rs, crates/nostr_compat/src/agent_memory.rs_
    - _Writes: crates/agent/src/memory.rs_
    - _Validation: storage tests cover owner read, ciphertext integrity, expiry and key rotation_
    - _Discovered contradiction (2026-08-23): the shared application database and existing managed-agent repository pattern live outside the legacy private `crates/agent/src/db.rs` store. A usable feature therefore requires narrow registration in `crates/agent/src/agent.rs`, an integration target, optional `nostr_compat`/`sha2` dependencies and lockfile metadata. Because NIP-AE is multiplayer-only, `agent` owns a matching non-default feature and `agent_ui` forwards its existing `multiplayer-tools` feature; Standard builds do not compile the new module._
    - _Evidence: 2026-08-23 — added a strict shared-SQLite repository that persists only exact owner/agent/blinded-slug coordinates, signed-event ID/time, canonical NIP-44 ciphertext, SHA-256 integrity and generation-fenced expiry state. Owner reads reject mismatched authentication before I/O, hide records at the exact expiry boundary and reparse/verify ciphertext without deleting corrupt evidence. Addressable updates retain greatest-time/lowest-event-ID head ordering; expiry is an exact retention CAS; owner-key rotation atomically expires the old coordinate and inserts the caller-supplied re-encrypted replacement, with exact retry idempotency and closed stale/conflict outcomes. Four focused tests passed restart-safe owner read/decryption and diagnostic redaction, ciphertext corruption/evidence retention, expiry/stale update and atomic/idempotent key rotation. A Standard no-default-feature `agent` check and repository-standard warning-denied release/all-target Clippy with `multiplayer-tools,test-support` passed. Rust formatting and diff hygiene passed; dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 30.3. Implement managed-agent snapshot lifecycle
    - Create, compare, restore and compact persona/team/runtime snapshots with stable provenance.
    - _Requirements: 11.3, 17.2_
    - _Capability IDs: CAP-023, CAP-024_
    - _Depends on: 29.5, 30.2_
    - _Reads: projects/buzz/desktop/src-tauri/src/managed-agents/**, crates/agent/src/managed_agents.rs_
    - _Writes: crates/agent/src/snapshot.rs_
    - _Validation: snapshot tests cover fidelity, stale restore, partial corruption and compaction_
    - _Discovered contradiction (2026-08-23): the planned Buzz `managed-agents/**` path does not exist; the inspected implementation uses `managed_agents/**`. The standalone lifecycle file also cannot compile or expose its API without multiplayer-gated crate-root registration, and fidelity requires reusing Task 29.5's private runtime codec rather than creating a second unchecked representation; the narrow correction makes that codec crate-visible, adds a focused integration target and synchronizes living design traceability. Buzz's portable agent/team exports intentionally omit machine-local secrets while its spawn snapshot is runtime-only. The canonical lifecycle therefore remains owner-scoped private local storage, persists exact caller-supplied canonical persona/team documents plus the already-validated private runtime record, and leaves portable disclosure/import policy to Task 30.5 rather than silently treating local snapshots as shareable files._
    - _Evidence: 2026-08-23 — added a strict shared-SQLite lifecycle with stable idempotent snapshot IDs, closed source-version/predecessor provenance, canonical bounded persona/team documents, the validated private managed-agent runtime encoding and per-component plus aggregate SHA-256 integrity. Owner-scoped load/compare exposes only component-change flags; restore is fenced by the caller's exact current runtime version and returns verified historical state for the existing CAS owner. Head-fenced compaction verifies every row, writes and reads back an owner-scoped removed-ID/chain-digest journal, then deletes only older history in the same savepoint while retaining the head; stale or corrupt input is non-mutating. Four release integration tests passed restart fidelity/idempotency/redacted diagnostics, exact compare and stale restore, partial-corruption evidence retention, and stale/successful/idempotent compaction. Standard no-default-feature release compilation, warning-denied focused Clippy, Rust formatting and diff hygiene passed; dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 30.4. Implement private per-turn usage metrics
    - Preserve NIP-AM encrypted metrics and local accounting without enabling client telemetry.
    - _Requirements: 11.3, 19.5_
    - _Capability IDs: CAP-024, CAP-028_
    - _Depends on: 11.8, 28.2_
    - _Reads: projects/buzz/docs/nips/NIP-AM.md, crates/agent/src/db.rs_
    - _Writes: crates/agent/src/usage.rs_
    - _Validation: tests cover aggregation, encryption, retention, export and telemetry-disabled behavior_
    - _Discovered contradiction (2026-08-23): Task 11.8 intentionally stops at the signed NIP-AM envelope and strict decrypted-payload codec, so preserving encrypted metrics requires the workspace-pinned `nostr` NIP-44 implementation and zeroizing plaintext buffers at this storage boundary. The standalone planned file also requires multiplayer-gated crate-root registration, direct optional dependency edges, lockfile metadata and a focused integration target. NIP-AM's durable encrypted metrics are distinct from Zed client telemetry: this repository must retain private local accounting while exposing no telemetry emitter or outbound client path._
    - _Evidence: 2026-08-23 — added owner/agent-bound NIP-44 v2 encryption and decryption over the strict Task 11.8 payload, exact canonical kind-44200 `p`/`agent` envelopes and redacted diagnostics. Added append-only owner-scoped shared-SQLite storage with independent local-payload/ciphertext SHA-256 integrity, exact retry idempotency, closed conflict handling, generation-fenced expiry and owner-only ciphertext export. Aggregation recomputes ordered cumulative session deltas, preserves unknown values, rejects duplicate sequences and detects numeric overflow; a reliable reported turn supplies only a missing initial baseline. Four focused release tests passed encryption/owner-agent readers and wrong-owner denial, cumulative aggregation and counter resets, restart-safe ciphertext-only export with retention/purge, and private local accounting plus corruption failure while client telemetry integration remains disabled by design. Standard no-default-feature release compilation, Rust formatting and diff hygiene passed; warning-denied Clippy, dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 30.5. Import archives, memories, snapshots and metrics
    - Map previously staged desktop records into the canonical memory/snapshot/usage stores with source retention.
    - _Requirements: 11.3, 17.2, 17.3_
    - _Capability IDs: CAP-024, CAP-045_
    - _Depends on: 17.9, 30.2, 30.3, 30.4_
    - _Reads: crates/zed/src/migration/buzz/desktop_state.rs, crates/agent/src/{memory,snapshot,usage}.rs_
    - _Writes: crates/zed/src/migration/buzz/agent_state.rs_
    - _Validation: every archive fixture imports idempotently with content/privacy hashes and rollback evidence_
    - _Discovered contradiction (2026-08-23): Task 17.9 deliberately stages sanitized JSON and hash-only kind-24200/44200 archive evidence, so it does not retain the missing plaintext, NIP-44 keys or canonical ciphertext needed to construct memories, snapshots or usage records autonomously. The importer therefore uses an explicit materialization request containing already-validated/re-encrypted canonical records, verifies them through their owning repositories and never guesses state from hashes. The planned standalone Zed file also requires multiplayer-gated module registration, direct feature forwarding to `agent`, a direct optional `nostr_compat` edge and test-support forwarding; these narrow manifest and lockfile changes are required for the leaf to compile without leaking the migration into Standard builds._
    - _Evidence: 2026-08-23 — added an owner-profile-scoped shared-SQLite importer that recomputes the complete Task 17.9 staging plan before I/O, validates exact canonical owner/agent/event and historical-retention bounds, accepts usage only for its matching archived kind-44200 event, retains kind-24200 evidence without inventing a second observer store and keeps opaque/unmaterialized sources on the disabled staging boundary. Every canonical write is read back before a per-record receipt binds the source content/privacy hashes to a redacted target manifest; existing verified receipts short-circuit before target mutation, partial batches resume from the first missing receipt and a final source/staged/privacy-to-target hash receipt supplies rollback evidence while execution, automatic start and credential use remain disabled. Three focused release GPUI tests passed full snapshot/memory/24200/44200 import and restart replay, partial failure with checkpointed resume, cross-owner rejection and tampered-plan rejection. The multiplayer release product check and repository-standard warning-denied release/all-target Clippy passed; Standard compilation, dependency, inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 30.6. Add agent-state privacy and fidelity conformance
    - Verify old/new export, relay rotation, compaction, retention and unauthorized access behavior.
    - _Requirements: 11.3, 20.2, 20.3_
    - _Capability IDs: CAP-024, CAP-044_
    - _Depends on: 30.5_
    - _Reads: crates/agent/src/{memory,snapshot,usage}.rs, .agents/specs/collaborative-workspace/fixtures/migrations/**_
    - _Writes: crates/agent/tests/agent_state_conformance.rs_
    - _Validation: `cargo test -p agent agent_state_conformance` passes legacy/new and unauthorized-reader cases_
    - _Discovered contradiction (2026-08-23): the frozen migration fixture is intentionally metadata-only and proves that secret material is absent, so legacy/current conformance cannot decrypt fixture payloads that do not exist. The test instead binds each frozen schema-era expectation to locally constructed canonical encrypted records and exercises those records through their owning repositories. The planned integration test also depends on multiplayer-only agent-state APIs; an explicit required-feature test target is necessary to keep Standard builds from auto-discovering and compiling it without those APIs._
    - _Evidence: 2026-08-23 — added three focused release GPUI conformance tests that bind frozen archive/cache/pricing, retention and persona/team fixture eras to canonical encrypted usage, memory and snapshot records. The suite proves ciphertext-only owner export across legacy/current usage schemas, exact cache and pricing fidelity, retention at the expiry boundary, owner and relay-set rotation without cross-scope reads, latest-version snapshot compaction, foreign-owner load/restore denial and redacted diagnostics. All three release tests passed; Standard no-default-feature release compilation, warning-denied focused Clippy, Rust formatting and diff hygiene passed. The frozen-fixture verifier confirmed 30 SQL migrations, 20 desktop stores, 32 desktop versions and no secret material; dependency, Buzz inventory and canonical specification gates passed with only the specification validator's existing manual-review warnings._

- [x] 31. Implement signed jobs and delegation

  - [x] 31.1. Define the canonical job state machine
    - Model request, accept, progress, result, cancel and error transitions with idempotency/version invariants.
    - _Requirements: 11.4_
    - _Capability IDs: CAP-026_
    - _Depends on: 11.13, 29.2_
    - _Reads: projects/buzz/crates/buzz-core/src/kind.rs, crates/task/**_
    - _Writes: crates/collaboration_domain/src/job.rs_
    - _Validation: property tests enumerate legal transitions and reject duplicates/out-of-order terminal updates_
    - _Discovered contradiction (2026-08-23): Buzz's inspected job implementation is only the six-kind `43001`–`43006` registry and feed classification; it supplies no state machine to port. Zed's structured task lifecycle is process-local, unversioned and intentionally returns booleans for invalid or terminal updates, so it cannot itself serve as signed distributed job authority. The canonical domain therefore adds a transport-free versioned command aggregate that later adapters bind to those six kinds and native tasks. The planned standalone file also requires crate-root module registration and re-exports, plus living design synchronization for Requirement 11.4._
    - _Evidence: 2026-08-23 — added a tenant-scoped canonical job aggregate whose first request fixes requester and target executor, whose accept/progress/result path preserves executor continuity and whose cancel/error transitions retain any accepted executor for later authorization and audit. Complete ordered command history provides exact reconstruction and operation-level idempotency at any historical version; conflicting operation reuse, same-version mutation, stale/gapped versions, regressing timestamps, wrong executors, result-before-accept and every non-replay terminal update fail closed without mutation. Two 256-case property tests cover the complete six-state/six-command legality matrix, exact replay and stale/future terminal fencing; focused history/executor and idempotency-conflict tests also passed. The complete domain suite passed 147/147, the four focused release tests passed, and repository-standard warning-denied release/all-target Clippy passed; formatting, diff, dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 31.2. Add job and executor-lease schema
    - Create job versions, delegation ancestry and exactly-one executor lease tables with recovery timestamps.
    - _Requirements: 2.1, 11.4, 11.5_
    - _Capability IDs: CAP-005, CAP-026_
    - _Depends on: 31.1_
    - _Reads: crates/collaboration_domain/src/job.rs, crates/collab/src/db/**_
    - _Writes: crates/collab/migrations/collaboration_jobs.sql_
    - _Validation: migration tests cover tenant fences, lease uniqueness, ancestry indexes and rollback_
    - _Discovered contradiction (2026-08-23): the planned unversioned `collaboration_jobs.sql` path does not match `collab`'s SQLx reversible migration convention, in which every current collaboration schema has a timestamped `.up.sql`/`.down.sql` pair and checksum-discovery tests. The narrow correction uses the next ordered reversible migration pair and a focused integration test. The canonical state machine permits owner-attested agents whose community access may be transient, so principal UUIDs remain tenant-scoped identifiers for Task 31.3 authorization rather than incorrectly requiring every executor to have a durable membership row._
    - _Evidence: 2026-08-23 — added a forced-RLS, community-keyed current job head and immutable version history with unique operation attribution, closed command/executor shapes and full unsigned-version bounds. Added a depth-1..8 ancestry closure with one ancestor per child/depth, self-link rejection and descendant/direct-child indexes. Added generation-preserving executor lease history bound to an exact job version, with a partial unique index admitting at most one active lease per job and explicit acquire/heartbeat/expiry/recovery/release fences. The four-row reverse migration drops dependents in exact order without cascade. Six focused release migration tests passed schema invariants, SQLx discovery/SHA-384 checksums and rollback shape; the optional isolated live PostgreSQL exercise is present but skipped when `COLLAB_JOB_MIGRATION_TEST_DATABASE_URL` is unset. Warning-denied focused-target Clippy, Rust formatting and diff hygiene passed. Repository-standard all-target Clippy was attempted and is blocked only by pre-existing `redundant_clone` and `needless_lifetimes` warnings in unrelated tests; dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 31.3. Enforce job and delegation authorization
    - Apply owner, team, role, scope and delegation-depth/resource policy to every transition.
    - _Requirements: 6.2, 11.4_
    - _Capability IDs: CAP-008, CAP-023, CAP-026_
    - _Depends on: 13.3, 29.2, 31.1_
    - _Reads: crates/collaboration_domain/src/{authorization,job}.rs_
    - _Writes: crates/collaboration_domain/src/job_authorization.rs_
    - _Validation: policy tests cover owner/team/service, revoked member, cycle, excessive depth and scope_
    - _Discovered contradiction (2026-08-23): `collaboration_domain` cannot import the GPUI-backed `agent_settings::team::TeamRole` without violating its enforced lower-layer dependency contract, and the canonical job aggregate intentionally retains no resource payload. The policy therefore consumes explicit current team-membership evidence and separately resolved bounded canonical resource IDs, while their settings, storage and payload owners remain outside the authorization module. The planned standalone file also requires crate-root module registration and re-exports plus living design synchronization._
    - _Evidence: 2026-08-23 — added a pure fail-closed authorization boundary that first validates the exact tenant, legal versioned state transition, `jobs:write`, effective scoped-token subject, command actor and a canonical resource subset. Current active community roles admit owners/admins or narrowly constrain members; explicit current team roles constrain coordinators, executors and observers; owner-attested agents require team or delegation evidence. Expiring revocable service grants bind one principal, job version and transition set. Delegation grants bind current membership version, exact parent/child/delegate, transition set and a further-narrowed resource set; ancestry rejects foreign tenants, cycles and duplicates, caps depth at eight, and request admission enforces 16 active children and 256 active community jobs while allowing terminal cleanup. Six focused debug and release tests cover owner/team/service authority, revoked memberships, cycles, excessive depth, actor/scope failures, resource narrowing and cleanup limits. The complete release domain suite passed 153/153 and repository-standard warning-denied release/all-target Clippy passed; formatting, diff, dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 31.4. Implement signed job Nostr adapter
    - Translate kinds 43001–43006 to canonical commands and exact compatibility responses.
    - _Requirements: 5.1, 11.4_
    - _Capability IDs: CAP-001, CAP-026_
    - _Depends on: 14.4, 31.1, 31.3_
    - _Reads: projects/buzz/crates/buzz-core/src/kind.rs, crates/collaboration_domain/src/job.rs_
    - _Writes: crates/nostr_compat/src/jobs.rs_
    - _Validation: golden job traces cover all transitions, duplicates, cancellation and malformed ancestry_
    - _Discovered contradiction (2026-08-23): Buzz registers kinds 43001–43006, retention and feed classification but contains no job tag/content schema, reducer or compatibility response implementation to port; its sole nearby comment names an unimplemented depth-3/breadth-10 “auth chain”, while the approved operational policy fixes depth 8 and active-child admission 16. The adapter therefore defines the narrow canonical Zed wire boundary around the six immutable kind numbers and the already-approved domain/policy, without claiming a nonexistent Buzz payload schema. Translating into canonical commands also requires the intended one-way `nostr_compat` → UI/I/O-free `collaboration_domain` dependency, its exact dependency allowlist update, crate-root registration, lockfile update and living design synchronization beyond the planned standalone file._
    - _Evidence: 2026-08-23 — added a pure signed-job codec that verifies signature/event ID/content/timestamp policy before field use; accepts only exact canonical tenant, job, positive-version, request-target and ordered-parent tags; injects signer/target principal resolution; derives stable operation IDs from community plus signed event ID; checks Nostr-second conversion; and rejects unknown kinds, tag smuggling, tenant disagreement, unknown principals, duplicate/self/foreign/over-depth ancestry and noncanonical UUID/version text. The inverse encoder emits byte-stable tag order for request, accept, progress, result, cancel and error response events, requires lossless whole-second timestamps and keeps opaque payload content out of the state machine and redacted from diagnostics. Three focused release golden tests cover all six kind/command mappings, exact tags and payloads, request→accept→progress→result application, exact duplicate replay, cancellation, failure, malformed ancestry, tenant mismatch and signature tampering. The complete release protocol suite passed 75 unit and four integration tests; repository-standard warning-denied release/all-target Clippy and the updated dependency boundary passed. Formatting, diff, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 31.5. Bind accepted jobs to native task/session execution
    - Acquire an executor lease, create/resume one Zed task/ACP session and publish terminal outcome.
    - _Requirements: 11.4, 11.5_
    - _Capability IDs: CAP-021, CAP-026_
    - _Depends on: 28.2, 31.4, 31.7_
    - _Reads: crates/agent/src/collaboration_session.rs, crates/collab/src/jobs/repository.rs_
    - _Writes: crates/agent/src/jobs.rs_
    - _Validation: execution tests cover accept, progress, cancel, crash, lease expiry and exactly-one result_
    - _Discovered contradiction (2026-08-23): `collab::JobRepository` is a server-side PostgreSQL owner and importing it directly into the desktop `agent` crate would collapse the approved service/RPC boundary. The native coordinator therefore consumes a mandatory tenant-bound execution-authority capability whose contract matches Task 31.7's job, transition and generation-fenced lease behavior; production composition must adapt the canonical repository or RPC client without a second store. The standalone file also requires crate-root registration, the existing workspace `async-trait` dependency and a dedicated integration target._
    - _Evidence: 2026-08-23 — added a native job execution coordinator that accepts only the assigned executor on accepted/in-progress jobs, acquires an exact job-version lease, maps the job onto Task 28.2's single ACP session registry and creates or resumes that session before dispatching one bounded native prompt. It publishes a deterministic lease-derived progress transition, validates the current generation and recovery deadline before result/error/cancel side effects, uses the native cancellation capability, releases terminal leases and cleans up surviving leases on terminal retries. Crashes retain the in-progress state and active lease until timed recovery, after which a new generation resumes the same native session; stable operation IDs and canonical idempotency ensure exactly one result. Four focused release integration tests pass accept/progress/result with terminal retry, cancellation, crash plus pre-expiry rejection and post-expiry resume, and recovery-boundary fencing with zero result. Release library checking, focused warning-denied Clippy, Rust formatting, diff, dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

  - [x] 31.6. Implement delegated child-job orchestration
    - Create authorized child jobs, preserve ancestry and aggregate outcomes without recursive duplicate execution.
    - _Requirements: 11.4_
    - _Capability IDs: CAP-026_
    - _Depends on: 31.3, 31.5_
    - _Reads: crates/agent/src/jobs.rs, projects/buzz/benchmarks/harbor-buzz-orchestra/**_
    - _Writes: crates/agent/src/job_delegation.rs_
    - _Validation: orchestration tests cover tree completion, partial failure, parent cancel, retry and cycle rejection_
    - _Discovered contradiction (2026-08-23): Buzz's Harbor orchestra coordinates one-step assignments, reports and verification through channel messages over its relay/PostgreSQL deployment, but contains no durable child-job graph, ancestry, retry, cancellation or outcome-aggregation contract to port. The desktop `agent` crate also cannot own job persistence or import the server PostgreSQL repository without violating Task 31.5's approved service/RPC boundary. The narrow correction extends that mandatory authority capability, consumes already-authorized child requests from Task 31.3 and adds crate-root registration plus a focused integration target beyond the planned standalone file._
    - _Evidence: 2026-08-23 — added a delegated-job orchestrator that accepts only canonical authorized initial child commands with bounded same-tenant acyclic ancestry, verifies a nonterminal direct parent, delegates exact creation to canonical authority and reloads the persisted history/ancestry before success. Exact retries return the existing child without another creation, leaving Task 31.5's generation-fenced lease as the sole execution gate. Direct-child aggregation waits for every terminal child, completes only an all-success tree and fails on any failed or cancelled child; parent cancellation uses stable per-job operation IDs, skips terminal children and is idempotent on retry. Four focused release integration tests passed full tree completion, partial failure, cancellation propagation and retry, exact child-create retry and cycle rejection; focused and repository-standard release/all-target/all-feature warning-denied Clippy passed. Rust formatting, diff hygiene, collaboration dependency boundaries, Buzz inventory and the canonical specification validator passed with only its existing manual-review warnings._

  - [x] 31.7. Implement the job and executor-lease repository
    - Read/write job transitions, ancestry and executor leases with idempotency and optimistic concurrency.
    - _Requirements: 2.1, 11.4, 11.5_
    - _Capability IDs: CAP-005, CAP-026_
    - _Depends on: 31.2_
    - _Reads: crates/collab/migrations/collaboration_jobs.sql, crates/collaboration_domain/src/job.rs_
    - _Writes: crates/collab/src/jobs/repository.rs_
    - _Validation: repository tests cover concurrent accept, expired lease, retry, transition conflict and tenant isolation_
    - _Discovered contradiction (2026-08-23): the migration is the timestamped `20260823000200_collaboration_jobs` pair rather than the planned unversioned path, and a usable repository requires crate-root/module registration plus an integration target beyond the standalone implementation file. Exact replay, transition serialization and lease recovery also require complete history/ancestry reads and transaction-local tenant setup inside the same write transaction; a write-only abstraction would not preserve the approved domain and forced-RLS invariants._
    - _Evidence: 2026-08-23 — added the PostgreSQL-only canonical job repository with transaction-local tenant RLS, atomic request/head/version/ancestry creation, complete history reconstruction with denormalized-head verification, exact command/ancestry retry handling, locked domain transitions and optimistic head compare-and-swap. Executor leases are admitted only for the exact accepted/in-progress version and assigned executor, allocate monotonic generations, preserve and release recoverable expired generations, and fence heartbeat/release by tenant, job, version, generation, lease and executor. Seven deterministic release integration tests pass serialized competing accepts, exact retry, CAS rollback conflict, ordered ancestry persistence/reconstruction, expired-lease recovery, heartbeat/release fencing and pre-database tenant rejection. Focused warning-denied release Clippy, library checking, formatting, diff, dependency, Buzz inventory and canonical specification gates are recorded in the enclosing checkpoint commit._

- [ ] 32. Enrich the semantic activity feed

  - [x] 32.1. Map NIP-AO observer events to ActivityItem
    - Convert supported observer states to existing semantic classes with generic fallback.
    - _Requirements: 12.1, 12.2_
    - _Capability IDs: CAP-025_
    - _Depends on: 8.6, 28.4_
    - _Reads: crates/agent_ui/src/activity_projection.rs, crates/nostr_compat/src/agent_observer.rs_
    - _Writes: crates/agent_ui/src/activity_observer.rs_
    - _Validation: fixture tests map every NIP-AO kind exactly once and redact raw encrypted content_
    - _Discovered contradiction (2026-08-24): the planned standalone file cannot compile or become reachable because the typed NIP-AO ingress is Multiplayer-only and `agent_ui` had no `nostr_compat` dependency. The narrow correction registers the module and adds that dependency only to `agent_ui`'s existing non-default `multiplayer-tools` feature, updating lockfile metadata without adding a Standard edge. NIP-AO explicitly requires future telemetry/control kinds to be ignored, so the general D8 generic-fallback rule applies to recognized ACP read/write frames while unknown NIP-AO kinds remain absent. The mapper consumes only already-authorized decrypted ingress and cannot accept the encrypted event body or raw ACP transcript content._
    - _Evidence: 2026-08-24 — added a Multiplayer-only NIP-AO projector for all four recognized telemetry kinds. Turn start and session resolution share one agent/session/turn activity identity and advance by monotonic observer sequence; closed end-turn, cancellation, limit, refusal and error reasons map to terminal outcomes while unknown reasons remain redacted and explicitly unknown. ACP read/write frames become generic running protocol activity without method, parameters, prompts, results or arbitrary payload values. Typed channel/session links and outer protocol-event details retain provenance; control and future frames remain ignored. Four focused release fixtures passed all-kind cardinality and serialized secret-sentinel redaction, start-to-resolution reduction, terminal reason semantics and control/future/mismatch handling. Multiplayer and Standard release library checks, focused and repository-standard all-target warning-denied Clippy, Rust formatting, diff hygiene and collaboration dependency boundaries passed. The Standard dependency-tree gate remains blocked before this leaf by existing unconditional `nostr_compat` through `acp_thread` and `collaboration_domain` through `project`/`agent`; this leaf's optional edge is inactive in that reverse tree._

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

- [ ] 33. Merge remote-agent providers with Zed remote execution

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

  - [ ] 36.6. Port moderation commands to Zed CLI
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
    - Generate bounded variants and render image/audio/video/link attachments through existing Zed media components.
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
    - Map lifecycle and participants to the ADR-selected Zed audio transport with bounded cleanup.
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

  - [ ] 40.3. Import paired identities into Zed credentials
    - Verify received identity, store it canonically and preserve prior credentials on any failure.
    - _Requirements: 7.2, 7.3, 16.1_
    - _Capability IDs: CAP-009, CAP-033_
    - _Depends on: 12.4, 40.1, 40.2_
    - _Reads: crates/zed_credentials_provider/src/nostr_import.rs, crates/nostr_compat/src/pairing.rs_
    - _Writes: crates/zed_credentials_provider/src/pairing.rs_
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

- [ ] 42. Merge agent-first collaboration commands into Zed CLI

  - [ ] 42.1. Define canonical CLI command and error contracts
    - Map Buzz command groups, global options, compact output and exit classes to Zed-owned operations.
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
    - Forward legacy syntax to Zed commands, preserve stdout/stderr/retry semantics and emit minimum-version errors.
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
    - Translate services, migrations, ingress, autoscaling, disruption, storage and network policy to Zed ownership.
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
    - Move service/client/package artifacts to Zed conventions with signed manifests and compatibility metadata.
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
    - _Reads: projects/buzz/desktop/**, crates/zed/src/migration/buzz/**, .agents/specs/collaborative-workspace/validation-results.md_
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

## Milestone prerequisite — isolate Standard and Multiplayer Zed builds

- [x] 49. Establish the canonical `multiplayer-tools` compile-time boundary

  - [x] 49.1. Inventory and classify the feature boundary
    - Record current and planned shared, multiplayer-only, compatibility-shim and deployment-only surfaces without changing canonical ownership.
    - _Requirements: 21.1, 21.2, 21.7, 21.8_
    - _Capability IDs: CAP-001, CAP-002, CAP-005, CAP-009, CAP-021, CAP-036, CAP-037, CAP-043, CAP-045_
    - _Depends on: none_
    - _Reads: .agents/specs/collaborative-workspace/{source-inventory,reuse-audit,design,migration-plan,tasks}.md, Cargo.toml, crates/*/Cargo.toml_
    - _Writes: .agents/specs/collaborative-workspace/{requirements,design,migration-plan,reuse-audit,tasks}.md_
    - _Validation: feature-spec validator reports all Requirement 21 criteria traced and manual audit finds no shared canonical owner reclassified_
    - _Evidence: 2026-08-20 — classified persisted presentation as the dependency-light disabled-build shim; Editor/project/worktree/Git/ACP/base credentials/settings/collaboration as always-shared owners; native collaborative GPUI and Buzz domain/protocol extensions as multiplayer-only; and exclusive relay/migration/assets as deployment-only. Added nine stable Requirement 21 criteria, D13 dependency/state boundaries, binary rollback rules and fifteen dependency-ordered leaves. Manual ownership review retained every CAP-001 through CAP-045 disposition, and the feature-spec validator passed with 93 acceptance criteria and 401 parsed tasks; remaining warnings are advisory legacy granularity/overlap findings documented by the decomposition audit._

  - [x] 49.2. Define the public application feature
    - Add non-default `zed/multiplayer-tools` as the only release-facing Cargo feature; Task 49.3 fills its forwarding list after the target features exist.
    - _Requirements: 21.1, 21.3, 21.7_
    - _Capability IDs: CAP-036, CAP-037_
    - _Depends on: 49.1_
    - _Reads: Cargo.toml, crates/zed/Cargo.toml_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/zed/Cargo.toml_
    - _Validation: Cargo metadata shows `zed/multiplayer-tools` exists outside `default` and enables no dependency before Task 49.3_
    - _Evidence: 2026-08-20 — added the canonical `zed/multiplayer-tools` public feature outside the empty default feature set. The dependency-safe shell initially enables no crate or dependency until Task 49.3 declares its forwarding targets. Cargo metadata asserted both feature arrays exactly, diff checks passed and the feature-spec validator retained 93 acceptance criteria and 401 parsed tasks._

  - [x] 49.3. Declare internal composition features
    - Add non-default internal features to the existing UI and shared consumer crates without enabling dependencies yet.
    - _Requirements: 21.1, 21.7_
    - _Capability IDs: CAP-009, CAP-021, CAP-036, CAP-037_
    - _Depends on: 49.2_
    - _Reads: crates/{workspace,onboarding,sidebar,agent_ui,git_ui,channel,zed_credentials_provider}/Cargo.toml_
    - _Writes: crates/zed/Cargo.toml, crates/{workspace,onboarding,sidebar,agent_ui,git_ui,channel,zed_credentials_provider}/Cargo.toml_
    - _Validation: Cargo metadata shows every internal `multiplayer-tools` feature is non-default and reachable only from the explicit application feature_
    - _Evidence: 2026-08-20 — declared non-default internal `multiplayer-tools` features on the seven existing UI/shared consumer crates and forwarded the public Zed feature to that exact list. Cargo metadata assertions passed for the ordered forwarding set and for exclusion from every internal default set. An additional application check reached platform compilation but could not execute the macOS Metal compiler because the local Xcode Metal Toolchain is absent; no Rust or feature-graph error was reported before that environment boundary._

  - [x] 49.4. Isolate the canonical channel projection dependency
    - Make `collaboration_domain` optional in `channel` and compile its projection store only for multiplayer builds.
    - _Requirements: 21.2, 21.3, 21.7_
    - _Capability IDs: CAP-001, CAP-010, CAP-011, CAP-013_
    - _Depends on: 49.3_
    - _Reads: crates/channel/src/{channel,channel_store,collaboration_store}.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/channel/{Cargo.toml,src/{channel,channel_store}.rs}_
    - _Validation: channel tests pass in default mode without `collaboration_domain` and multiplayer projection tests pass with the feature_
    - _Evidence: 2026-08-20 — made `collaboration_domain` optional at the shared `channel` consumer and gated the canonical projection module, store field and APIs as one crate boundary. Default channel tests passed 3/3, flagged tests passed 6/6 including all projection cases, and exact `cargo tree --prefix none` assertions proved `collaboration_domain` absent unflagged and present flagged._

  - [x] 49.5. Isolate Nostr credential extensions
    - Make `collaboration_domain` optional in `zed_credentials_provider` and compile Nostr lifecycle/backup modules only for multiplayer builds.
    - _Requirements: 21.2, 21.3, 21.7_
    - _Capability IDs: CAP-001, CAP-007, CAP-009_
    - _Depends on: 49.3_
    - _Reads: crates/zed_credentials_provider/src/{zed_credentials_provider,nostr_import,nostr_lifecycle,nostr_backup}.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/zed_credentials_provider/{Cargo.toml,src/zed_credentials_provider.rs}_
    - _Validation: base credential-provider tests pass unflagged and Nostr import/lifecycle/backup tests pass flagged_
    - _Evidence: 2026-08-20 — gated Nostr import, lifecycle and backup modules at the credentials-provider crate root and made all eleven extension-only dependencies optional under its internal feature. The unflagged provider suite built and passed with zero extension tests; the flagged suite passed 18/18. Exact dependency-tree assertions proved `collaboration_domain` absent unflagged and present flagged._

  - [x] 49.6. Gate native workspace composition and preserve effective-mode fallback
    - Compile Collaborative GPUI modules only for Multiplayer Zed while retaining the lightweight presentation value and non-destructive effective Editor fallback.
    - _Requirements: 21.2, 21.3, 21.4, 21.7_
    - _Capability IDs: CAP-020, CAP-025, CAP-036, CAP-037_
    - _Depends on: 49.3_
    - _Reads: crates/workspace/src/{workspace,workspace_presentation,workspace_presentation_actions,multi_workspace,persistence,status_bar}.rs, crates/workspace/src/collaborative_*.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/workspace/Cargo.toml, crates/workspace/src/{workspace,workspace_presentation,workspace_presentation_actions,multi_workspace,status_bar}.rs_
    - _Validation: workspace tests prove unflagged effective Editor without preference mutation and flagged mount/switch/restart restoration_
    - _Evidence: 2026-08-20 — gated the native collaborative shell, rail/layout persistence, navigation, composer, participant, review, focus, status and top-bar modules at the workspace crate boundary. Standard Workspace now retains the dependency-light serialized presentation value but resolves Collaborative to an effective Editor presentation without rewriting the preference; Multiplayer Workspace mounts and persists the existing surfaces unchanged. Both no-default-feature crate checks passed; presentation-focused tests passed 5/5 unflagged and 7/7 flagged; and the flagged collaborative restart/theme/narrow-window integration suite passed 3/3._

  - [x] 49.7. Gate the onboarding choice
    - Render only Editor Workspace in Standard Zed and both approved choices in Multiplayer Zed without rewriting the saved preference.
    - _Requirements: 21.2, 21.3, 21.5, 21.7_
    - _Capability IDs: CAP-037_
    - _Depends on: 49.6_
    - _Reads: crates/onboarding/src/{onboarding,workspace_choice}.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/onboarding/src/workspace_choice.rs, crates/workspace/src/workspace.rs_
    - _Validation: onboarding tests show one accessible Editor choice unflagged and both persisted choices flagged_
    - _Evidence: 2026-08-20 — compiled the onboarding catalog as one Editor-only choice in Standard Zed and both Editor and Collaborative choices in Multiplayer Zed. The selector consumes Workspace's dependency-light effective-presentation shim, so a disabled build renders Editor selected while retaining a stored Collaborative preference for later restoration. Focused GPUI tests passed in both configurations, proving the collaborative selector absent unflagged, present flagged, keyboard accessible, and persisted only after an explicit selection._

  - [x] 49.8. Gate agent and Git collaboration adapters
    - Compile adapter modules only for multiplayer while leaving canonical ACP and Git owners unchanged.
    - _Requirements: 21.2, 21.3, 21.7_
    - _Capability IDs: CAP-020, CAP-021, CAP-025_
    - _Depends on: 49.6_
    - _Reads: crates/{agent_ui,git_ui}/src/{agent_ui,git_ui}.rs, crates/{agent_ui,git_ui}/src/collaborative_*.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/agent_ui/src/agent_ui.rs, crates/git_ui/src/git_ui.rs_
    - _Validation: crate builds prove adapters absent unflagged and their focused tests pass flagged_
    - _Evidence: 2026-08-20 — gated the composer, participant, agent-review, semantic-timeline and project-review adapter modules at the `agent_ui` and `git_ui` crate roots while leaving ACP, AgentPanel, project diff and Git owners shared. Both crates passed unflagged and flagged no-default-feature checks. With GPUI's supported runtime-shader test feature compensating for the locally absent Xcode Metal Toolchain, seven agent adapter/timeline unit tests, three collaborative activity compatibility tests and the focused Git review adapter test passed._

  - [x] 49.9. Gate the collaborative sidebar rail
    - Compile and mount collaborative rail/navigation modules only for multiplayer while retaining the normal Sidebar unchanged.
    - _Requirements: 21.2, 21.3, 21.5, 21.7_
    - _Capability IDs: CAP-036_
    - _Depends on: 49.6_
    - _Reads: crates/sidebar/src/sidebar.rs, crates/sidebar/src/collaborative_*.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/sidebar/src/{sidebar,sidebar_tests}.rs_
    - _Validation: sidebar tests prove standard rendering has no collaborative rail and flagged rendering preserves navigation badges_
    - _Evidence: 2026-08-20 — gated all five collaborative navigation/rail modules, the rail entity, width projection and live-status adapter at the Sidebar crate boundary while leaving the existing thread Sidebar compiled unchanged. Both no-default-feature crate checks passed after fetching the pre-existing audio/WebRTC artifact. Runtime-shader GPUI tests proved Standard Sidebar mounts at its canonical width with no collaborative rail, Multiplayer Sidebar preserves the rail geometry/section contract, and seven navigation projection, identity, duplicate-rejection and activation tests pass._

  - [x] 49.10. Gate application reconciliation registration
    - Register collaborative composer, participant and review reconciliation only in Multiplayer Zed.
    - _Requirements: 21.2, 21.3, 21.5, 21.7_
    - _Capability IDs: CAP-020, CAP-021, CAP-025, CAP-036_
    - _Depends on: 49.4, 49.5, 49.7, 49.8, 49.9_
    - _Reads: crates/zed/src/{zed,migration}.rs, crates/zed/src/migration/buzz.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/zed/src/{zed,migration}.rs, crates/zed/src/migration/buzz.rs_
    - _Validation: Zed tests show no multiplayer registrations unflagged and composer/participant/review registration flagged_
    - _Evidence: 2026-08-20 — gated the application-level composer, participant and project/agent review reconciliation state, observers, subscriptions and workspace event handlers so Standard Zed has no registration path while Multiplayer Zed retains automatic reconciliation. The same semantic ownership boundary gates the otherwise exclusive Buzz migration module; explicit module paths preserve its staged importer layout without exposing it to Standard Zed. Both application configurations passed no-default-feature checks with runtime shaders, and the flagged focused reconciliation tests passed for composer, participant and review registration._

  - [x] 49.11. Add closed capability negotiation and unsupported-operation behavior
    - Expose build capability without tenant/resource data and reject retained multiplayer entry points before lookup when unavailable.
    - _Requirements: 21.5, 21.6_
    - _Capability IDs: CAP-002, CAP-038, CAP-039, CAP-040, CAP-041, CAP-042_
    - _Depends on: 49.10_
    - _Reads: crates/workspace/src/workspace_presentation.rs, crates/zed/src/zed.rs, crates/proto/src/**_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, crates/workspace/src/{multiplayer_capability,workspace_presentation,workspace}.rs_
    - _Validation: unit tests prove explicit advertised availability and identical closed rejection for missing, foreign and denied multiplayer targets in Standard Sim_
    - _Evidence: 2026-08-20 — added an always-compiled, dependency-light capability advertisement whose serialized `multiplayer_tools` value explicitly reflects the build profile, plus one closed `NotIncludedInBuild` admission error. Standard-profile tests prove missing, foreign and denied target classes receive the identical error and never invoke resource lookup; Multiplayer-profile tests prove resolution begins only after successful capability admission. Workspace presentation now consumes the same canonical build-capability owner._

  - [x] 49.12. Define feature-aware packaging and deployment profiles
    - Add reproducible Standard/Multiplayer build profiles and artifact-content checks; keep base collab deployment shared and exclusive Buzz components explicit.
    - _Requirements: 21.2, 21.3, 21.6, 21.8_
    - _Capability IDs: CAP-002, CAP-005, CAP-016, CAP-035, CAP-043, CAP-045_
    - _Depends on: 49.10_
    - _Reads: script/bundle-{mac,linux,freebsd}, script/bundle-windows.ps1, .github/workflows/{release,deploy_collab}.yml_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, script/multiplayer-build-profile, docs/src/{SUMMARY,development/multiplayer-tools}.md_
    - _Validation: profile dry runs emit explicit feature/artifact metadata and reject multiplayer payloads in Standard mode_
    - _Evidence: 2026-08-20 — added deterministic `standard` and `multiplayer` release profiles that emit the exact Zed feature arguments, explicit artifact capability, migration policy and shared-collab disposition. Artifact inspection rejects known Buzz-owned service, protocol, migration and deployment payload names only in Standard mode while Multiplayer mode reports them. Development documentation defines build/run commands, packaging metadata, inspection, rollback and the semantic-ownership exception for shared Zed collaboration infrastructure._

  - [x] 49.13. Add a local dual-configuration verification harness
    - Check, test, warning-denied lint and smoke both application configurations and inspect the unflagged dependency tree.
    - _Requirements: 21.2, 21.3, 21.4, 21.5, 21.8, 21.9_
    - _Capability IDs: CAP-036, CAP-037, CAP-043, CAP-044_
    - _Depends on: 49.11, 49.12_
    - _Reads: script/clippy, crates/zed/Cargo.toml, crates/workspace/tests/collaborative_workspace.rs_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, script/{check-multiplayer-tools,clippy}_
    - _Validation: script/check-multiplayer-tools runs the default and enabled quick matrices and fails a seeded dependency-leak fixture_
    - _Evidence: 2026-08-20 — added a dual-profile harness whose quick matrix checks the Zed application, tests the always-compiled capability admission boundary, validates the package profile and denies all audited Buzz/Collaborative-exclusive packages in Standard Zed's locked normal dependency tree. Full mode additionally runs application tests and warning-denied release Clippy for each profile. The shared Clippy wrapper now accepts the internal `--no-all-features` control needed to prevent profile lint from unifying unrelated features. Both quick profiles passed and a seeded `collaboration_domain` dependency-tree fixture failed closed._

  - [x] 49.14. Add the required CI feature matrix
    - Run the local harness in CI with separate Standard and Multiplayer jobs so neither profile can regress through feature unification.
    - _Requirements: 21.2, 21.3, 21.8, 21.9_
    - _Capability IDs: CAP-043, CAP-044_
    - _Depends on: 49.13_
    - _Reads: tooling/xtask/src/tasks/workflows/run_tests.rs, .github/workflows/run_tests.yml, script/check-multiplayer-tools_
    - _Writes: .agents/specs/collaborative-workspace/tasks.md, tooling/xtask/src/tasks/workflows/run_tests.rs, .github/workflows/run_tests.yml_
    - _Validation: workflow YAML parses and matrix/job command assertions cover feature-off, feature-on and dependency denial_
    - _Evidence: 2026-08-20 — added separate generated `multiplayer_tools_standard` and `multiplayer_tools_multiplayer` Linux jobs at the canonical xtask workflow source. Each job runs the full local harness for exactly one profile, including isolated build, tests, warning-denied Clippy, package smoke and the shared Standard dependency denial; both are required by the aggregate `tests_pass` job. Workflow regeneration, YAML parsing and command/profile assertions passed._

  - [x] 49.15. Verify documentation, cleanup and future-leaf enforcement
    - Publish exact commands and audit every completed/planned collaborative leaf for a shared/gated/compatibility/deployment classification.
    - _Requirements: 21.1, 21.2, 21.3, 21.7, 21.8, 21.9_
    - _Capability IDs: CAP-001, CAP-036, CAP-037, CAP-043, CAP-044, CAP-045_
    - _Depends on: 49.14_
    - _Reads: .agents/specs/collaborative-workspace/**, docs/src/development/multiplayer-tools.md, script/check-multiplayer-tools_
    - _Writes: .agents/specs/collaborative-workspace/{design,reuse-audit,tasks}.md, crates/zed/src/zed.rs, docs/src/development/multiplayer-tools.md_
    - _Validation: feature-spec validator passes, dual-configuration harness passes, task audit has no unclassified Buzz-derived write path or perpetual deferred bucket_
    - _Evidence: 2026-08-20 — added a semantic-ownership ledger covering every descendant leaf of all 49 epics with explicit shared, gated, disabled-build compatibility and deployment classifications. Mixed epics preserve canonical Zed owners while gating only Collaborative Workspace-exclusive adapters, registrations and service artifacts; future leaves must split independently reviewable owners and update dependency/payload denial in the introducing change. The developer guide now publishes exact quick, full, profile-specific and captured-tree commands plus the same classification rule. Standard warning-denied Clippy identified three multiplayer-test imports left unconditional; the cleanup now admits those imports only with their owning tests. Specification validation, the task-ledger coverage audit and both supported build-profile verification gates passed._

## Decomposition audit notes

- No approved requirement, architecture decision, capability ownership, migration phase or milestone scope was changed.
- Milestones are headings, the 48 approved capability epics are parent checkboxes, and all executable work is represented by nested `epic.leaf` implementation units with metadata only on leaves.
- All 93 acceptance criteria appear in the approved design traceability table and in at least one leaf task. All CAP-001 through CAP-045 identifiers appear in at least one implementation, compatibility, validation or verified-reuse leaf.
- The final plan contains 49 populated epic parents and 352 implementation leaves. Every leaf includes requirement, capability, dependency, read, write and validation metadata.
- The decomposition audit reviewed compound titles, multi-path writes and cross-boundary outcomes. Each leaf retains one primary implementation or evidence boundary, concrete scope, and focused validation; independently reviewable domain, persistence, transport, UI, client, migration, deployment and test outcomes remain separate leaves.
- The validator's remaining granularity warnings belong to completed Milestone 1 integration checkpoints or one mechanical internal-feature declaration: Task 5.1 owns one presentation-setting schema/default/consumer contract; Tasks 6.3 and 6.5 own one cross-crate rail-layout and persistence composition each; Task 7.1 includes its manifest/lock exports for one sidebar projection; and Task 49.3 declares the same empty non-default feature across existing consumer manifests without changing code owners. Reopening landed checkpoints would change reviewed implementation history rather than resolve the approved contradictions. Every behavioral Epic 49 leaf has one primary crate or operational boundary.
- Repeated writes are explicitly sequenced by dependencies. The approved Milestone 1 splits add upper composition leaves that intentionally mount lower-owner registrations; their narrow cross-crate writes are listed rather than hidden inside the workspace contract leaves.
- All leaf dependencies were recomputed after renumbering. Validator checks confirm that every dependency names an existing implementation leaf and the explicit shared-write chains sequence intentional overlap.
- No perpetual deferred bucket exists. ADR-001 through ADR-006 are accepted; their implementation leaves remain concrete and dependency-gated. The three implementation-discovered dependency contradictions are resolved by Tasks 7.5–7.6, 9.1/9.6–9.8 and 10.2/10.8–10.10 without changing product scope or dependency direction.
- No leaf is estimated above three agent-days. The compatibility/load/cutover gates are execution-and-evidence leaves over fixtures built earlier, not requests to construct those systems inside the gate task.
- Production cutover, source deletion and irreversible operations are represented as tooling, rehearsal, evidence and separately authorized handoff work; this plan does not authorize those mutations.
