# Collaborative Workspace qualification results

Overall status: **REOPENED — NATIVE DESKTOP VALIDATION INCOMPLETE**.

The 2026-08-26 production-composition audit invalidated the prior native
desktop qualification while preserving the historical command evidence below.
The checked visual test verified reference PNG hashes and selector bounds only;
it did not render a populated native workspace or compare application raster
output. The production center region was a literal `"Timeline"` placeholder,
and several visible controls had no production owner. Epic 50 is now the
canonical corrective validation plan. No native desktop PASS may be restored
until its real-provider, cross-surface, state/failure, accessibility, restart,
and raster-comparison gates pass in the running `multiplayer-tools` product.

## 2026-08-27 canonical channel-message corrective snapshot

This is implementation evidence for Epic 51, not a restored CAP-011 or product
parity sign-off.

| Gate | Current result |
| --- | --- |
| Live PostgreSQL two-client RPC | PASS — two authenticated clients open one authorized channel, create with exact idempotent retry, receive live delivery, edit, reject a stale version, react, acknowledge, delete, disconnect, create activity while disconnected, reconnect from the authoritative cursor, reload one canonical row without duplication and deny cross-tenant/nonmember/removed-member access |
| Dense PostgreSQL pagination | PASS — three same-second messages traverse a two-row snapshot/keyset page and one-row continuation with three unique message IDs |
| Redis notification and loss recovery | PASS — the configured Redis adapter publishes/subscribes the bounded envelope; a forced unavailable Redis endpoint fails while PostgreSQL reloads the exact committed envelope and payload |
| Desktop transport build | PASS — `cargo check -p collab_ui --features multiplayer-tools` compiles the server-backed `MessageTimeline`, protected signing identity, composer operations, reconnect and channel/workspace subscription cleanup |
| Rust product build | PASS — `ZED_PRODUCT_ID=rust cargo check --package zed --no-default-features --features multiplayer-tools,rust-tools` completes |
| Standard feature boundary | PASS — `cargo check -p collab_ui --no-default-features` and `ZED_PRODUCT_ID=rust cargo check --package zed --no-default-features` complete; channel-adapter dependencies are optional edges of `multiplayer-tools` |
| Local Compose contract | PARTIAL — shell/config checks pass; an earlier local run brought PostgreSQL, Redis and object storage up healthy, but Docker Desktop could not restart for the final rerun. The Collab container and two authenticated GUI clients have not completed the manual demonstration |
| Focused Clippy | PARTIAL — warning-denied all-target/all-feature Clippy passes for `client`, `channel` and `workspace`, and production-library Clippy passes for `collab`. `collab_ui` reaches the unchanged `tests/huddle.rs:518` redundant clone; `collab` all-target reaches the unchanged `tests/buzz_object_git_metadata_import.rs:116` redundant clone; `zed` all-target reaches the pre-existing Rust-tools Cargo-action test length mismatch at `src/zed.rs:821` |
| Full workspace Clippy | BLOCKED OUTSIDE DIFF — the required command was run, but the 47 GiB pre-existing debug target plus release all-target graph exhausted the local volume while Cargo built dependencies, before a workspace source diagnostic. The disposable `target/release` cache was removed afterward |
| Epic 51 sign-off | INCOMPLETE — injected transaction rollback, revoked-authentication-session, two-process/queue fault, network-WebSocket, live-server GPUI, canonical message-thread selection, GUI restart and populated screenshot evidence remain open |

## 2026-08-26 corrective validation snapshot

This snapshot is current implementation evidence, not a parity sign-off.

| Gate | Current result |
| --- | --- |
| Rust product startup | PASS — `ZED_PRODUCT_ID=rust cargo run --package zed --no-default-features --features multiplayer-tools,rust-tools` built, launched and remained running until terminated by the validator |
| Populated native raster | PARTIAL — expanded 1930×1262: 97.07% structural and 10.90% exact pixels; collapsed 1928×1298: 96.79% structural and 21.52% exact pixels; runner exited cleanly after deterministic provider/thread/worktree/window teardown |
| Focused native composition tests | PASS — unified `workspace`, `sidebar`, `agent_ui`, `collab_ui` and `git_ui` collaborative suite passed 84 tests |
| Persistent rail pinning | PASS — seven workspace navigation tests and five sidebar pinning tests cover bounded toggle state, restart, missing/archive cleanup, ordering, activation and native empty/unavailable rendering |
| ACP composer lifecycle | PASS — the production adapter test opens a real `AgentPanel` ACP thread, reaches its canonical `MessageEditor` through landmark keyboard traversal, and proves empty-input rejection, workspace-routed submit, authoritative generating/entry state, cancel and return to idle |
| Standard changed-crate clippy | PASS — warning-denied all-target clippy without all features passed for `workspace`, `sidebar`, `agent_ui`, `collab_ui` and `git_ui` |
| Multiplayer changed-crate clippy | PARTIAL — warning-denied all-target/all-feature clippy passed for `workspace`, `sidebar`, `agent_ui` and `git_ui`; `collab_ui` production-library clippy passed, while its unchanged `tests/huddle.rs:518` redundant clone blocks the all-target command |
| Full workspace clippy | BLOCKED OUTSIDE DIFF — duplicate `tests` module in `language_models/src/provider/bedrock.rs` and stale `draw(cx).clear()` calls in `onboarding/src/workspace_choice.rs` |
| Standard application check | PASS — direct no-default-feature `zed`/`gpui_platform` check passed |
| Dual-profile harness | BLOCKED OUTSIDE DIFF — its Standard dependency deny-list rejects the existing `collaboration_domain` paths through shared `audio`, `project` and `remote` crates before profile execution |
| Specification and inventory | PASS — canonical complete validator passed with advisory legacy decomposition warnings; inventory passed 31 packages, 184 protocol records, 62 data sources, 193 surfaces, 45 capabilities, 93 requirements and 355 complete/planned leaves; seven Epic 50 leaves remain explicitly in progress |

The corrective rail now stores project/thread pins in the existing versioned
workspace navigation record and uses canonical `ChannelStore` favorites for
channels. Visible native controls toggle those owners; recent work remains
distinct from explicitly pinned work. Post-change warning-denied all-target
clippy passes for `workspace` and `sidebar` both with all features and without
all features. Warning-denied all-target `agent_ui` clippy also passes with all
features and without all features after the ACP composer lifecycle coverage.

Current external dependencies are explicit: authorized hosted community/channel
message history, remote room participants/invitations and forge CI/review state
require a configured collaboration/forge service and authenticated account.
The signed-out local run presents those dependencies as unavailable and does
not manufacture users, messages, workflow events or CI results.

Captured on 2026-08-25 from consolidated Zed revision `cde3ae4882cc23d7b079d178db3eae0f8cc30cac`. All implementation, compatibility, security, fault and approved local-load gates required before cutover passed. Requirements 20.4 and 2.4 still prohibit a completion claim because Tasks 46–48 have not executed cutover, duplicate-owner retirement, final catalog regeneration or operational/product sign-off.

## Evidence versions

| Code | Independent result | Captured source | Result and approved budget |
| --- | --- | --- | --- |
| P | `test-results/collaborative-workspace/protocol-gate.md` | `fcb49ae0458962bf0e86044aedb7a6a99ec8a684` | PASS: zero unexplained protocol/failure-frame divergence across signed events, Nostr/custom NIPs, Git, media, pairing and companion clients |
| S | `test-results/collaborative-workspace/security-gate.md` | `ae646662bc1ad6ac009eb9a6623ae02bb96df899` | PASS: 233/233 threat controls had completed negative owners; 182 Rust tests and the 55-limit/16-stop-signal observability audit passed |
| M | `test-results/collaborative-workspace/migration-gate.md` | `2daaba5496f6410668816ae1fb47c409f3f0e6cd` | PASS: 43 fault/recovery tests plus the 20-migration PostgreSQL 17 lifecycle passed without production mutation |
| L | `test-results/collaborative-workspace/collaboration-scale.md` | `c7ceee5e8082a6ad86470a1ef29da1b37052e431` | PASS: approved OL-CON, OL-DAT and OL-PUS connection, fan-out, window, search/freshness and wake-queue budgets passed |
| O | `test-results/collaborative-workspace/orchestration-scale.md` | `353fb31dd5bd65cf04d1ff7b2b67d3db7cf3ff29` | PASS: approved OL-EXE and OL-MSH workflow, delegation, provider and mesh capacity/cancellation/fairness budgets passed without duplicate execution |
| T | Completed leaf evidence in `.agents/specs/collaborative-workspace/tasks.md` | `cde3ae4882cc23d7b079d178db3eae0f8cc30cac` | PASS where cited: focused unit, GPUI, integration, visual, packaging and compatibility validation for the canonical owner |

`PASS` below means the capability's implemented/reused/compatible behavior has passing semantic evidence at the cited version. `PASS / HOLD` adds an explicit activation, cutover or removal condition; it is not a parity-complete state.

## Capability ledger

| Capability | Name | Evidence | Approved budget or oracle | Result |
| --- | --- | --- | --- | --- |
| CAP-001 | Signed event domain | P,S,T | Exact frozen encoding/signature/head/privacy semantics; zero unexplained divergence | PASS / HOLD — final catalogs and ownership audit: 47.6, 48.1–48.4 |
| CAP-002 | Nostr relay protocol | P,S,L,T | Exact supported frames/failures; bounded OL-CON admission and cleanup | PASS / HOLD — legacy relay retirement: 47.4 |
| CAP-003 | Communities and isolation | P,S,L,T | Authorization before observation; zero cross-tenant content/count/timing leak | PASS |
| CAP-004 | Relay connections and subscriptions | P,S,L,T | OL-CON connection/subscription/fan-out ceilings and deterministic reconnect | PASS / HOLD — write freeze and relay retirement: 47.1, 47.4 |
| CAP-005 | Event log and projections | S,M,L,T | Tenant-fenced authority, rebuild/drift and interruption recovery | PASS / HOLD — shadow/cutover and owner removal: 46.1–46.6, 47.1, 47.4 |
| CAP-006 | Realtime pub/sub | P,S,L,T | Scoped delivery, TTL/backpressure bounds and 0% irrelevant scoped delivery | PASS / HOLD — outbox mirroring and pub/sub retirement: 46.3, 47.4 |
| CAP-007 | Identity and profiles | P,S,M,T | Provenance, rotation/revocation/archive and protected-history semantics | PASS |
| CAP-008 | Authentication and authorization | P,S,T | Cross-transport fail-closed policy, replay and revocation negatives | PASS |
| CAP-009 | Secret and signing-key custody | S,M,T | Protected-storage round trip, redaction, fallback and rollback failures | PASS |
| CAP-010 | Channels and membership | P,S,L,T | Type/visibility/role/invite lifecycle plus tenant and queue bounds | PASS |
| CAP-011 | Messaging and threads | P,S,L,T | The new RPC, PostgreSQL authority/replay, Redis adapter and desktop channel adapter now have focused production-path evidence; live-server GPUI/Compose, message-thread selection and remaining fault/security gates are incomplete | HISTORICAL PASS — CURRENT INCOMPLETE: 51.1–51.8 |
| CAP-012 | Direct messages and privacy | P,S,T | Participant/result gates and encrypted visibility failures | PASS |
| CAP-013 | Read, unread, reminders and drafts | P,S,L,T | Bounded convergent frontier/override state and reconnect semantics | PASS |
| CAP-014 | Presence and typing | P,S,L,T | Scoped ephemeral state, expiry and cleanup | PASS |
| CAP-015 | Search and discovery | S,L,T | Authorization before ranking; approved corpus p95/p99 below 500 ms/2 s | PASS |
| CAP-016 | Notifications and push | P,S,L,T | Wake-only privacy, App Attest/provider fences and bounded claims | PASS |
| CAP-017 | Home, inbox, pulse, forum and culture | P,S,T | Canonical projection, authorization and native semantic evidence | PASS |
| CAP-018 | Project grouping | P,S,T | Cross-owner metadata without repository-authority transfer | PASS |
| CAP-019 | Git forge protocol | P,S,T | Real `git http-backend` differential, signing and pre-storage denial | PASS |
| CAP-020 | Branch collaboration and review | P,S,T | Canonical Git linkage, stale/conflict behavior and native diff reuse | PASS |
| CAP-021 | Agent ACP bridge | S,O,T | One authorized ACP session, bounded ingress, cancellation and output | PASS / HOLD — duplicate ACP retirement: 47.3 |
| CAP-022 | Agent runtime and MCP tools | S,O,T | Canonical permission/process owners, bounded tools and cleanup | PASS / HOLD — duplicate agent/MCP retirement: 47.3 |
| CAP-023 | Managed agents, personas and teams | S,T | Versioned catalogs, privacy projection and canonical lifecycle | PASS |
| CAP-024 | Agent memory, snapshots and metrics | S,M,T | Encrypted/reference-only custody, import/recovery and retention | PASS |
| CAP-025 | Agent observability and semantic activity | S,T | Exactly-one semantic mapping, in-place lifecycle and truthful fallback | PASS |
| CAP-026 | Jobs and delegation | S,O,T | OL-EXE-07, 256 active jobs and exact replay with no duplicate create/result | PASS |
| CAP-027 | Workflows and approvals | S,O,T | OL-EXE-04 caps; durable trigger/action/approval/lease replay | PASS |
| CAP-028 | Audit and usage | S,M,T | Tamper detection, redaction and content-free bounded metrics | PASS / HOLD — cutover diagnostics: 46.4 |
| CAP-029 | Moderation | P,S,T | Role/tenant enforcement, stale/ambiguous failures and audit attribution | PASS |
| CAP-030 | Retention, deletion and recovery | S,M,T | Exact-prefix retry, legal holds, ordered deletion and reversible recovery | PASS |
| CAP-031 | Media and attachments | P,S,M,T | Auth before storage, hostile-byte bounds and generation cleanup | PASS |
| CAP-032 | Voice, huddles and transcription | S,T | One canonical generation, bounded media/control and visible failures | PASS |
| CAP-033 | Pairing and identity transfer | P,S,M,T | Six-direction NIP-AB interop, replay/expiry/cancel and verified import | PASS |
| CAP-034 | Remote-agent providers | S,O,T | Pre-secret negotiation, exactly once, hostile output and process cleanup | PASS / HOLD — duplicate remote runtime retirement: 47.3 |
| CAP-035 | Relay mesh and shared compute | S,O,T | OL-MSH queue/fairness/lease/resource limits and no silent fallback | PASS |
| CAP-036 | Native collaborative desktop | T | Approved GPUI/visual/a11y geometry, state and native-owner evidence | INCOMPLETE — production composition improved, but Epic 50 hosted-channel/CI, full-state and visual-parity gates remain open |
| CAP-037 | Onboarding and workspace selection | T | Reversible shared-data presentation and Standard fallback | PASS / HOLD — Buzz desktop retirement: 47.2 |
| CAP-038 | Agent-first CLI | P,S,T | Frozen commands, links, exit/error and unsupported-version contracts | PASS / HOLD — compatibility preservation: 47.5 |
| CAP-039 | Web client | P,S,T | Frozen routes/auth/version and pre-mutation failure contract | PASS |
| CAP-040 | Mobile client | P,S,T | Frozen lifecycle/pairing/message/push contract | PASS |
| CAP-041 | Administration | P,S,M,T | Role/tenant/version failures and redacted recovery operations | PASS / HOLD — canonical operations documentation: 48.3 |
| CAP-042 | Entity/deep links and compatibility | P,T | Frozen cross-client links and fail-before-write negotiation | PASS / HOLD — compatibility preservation: 47.5 |
| CAP-043 | Build, release and deployment | S,M,L,T | Schema/config/package/profile checks and closed readiness/stop signals | PASS / HOLD — diagnostics, rollback and final operations docs: 46.4–46.5, 48.3 |
| CAP-044 | Test, conformance and formal evidence | P,S,M,L,O,T | Independent protocol, threat, fault, load and orchestration gates | PASS / HOLD — cutover rehearsal and preserved evidence: 46.6, 47.5 |
| CAP-045 | Migration and local archive | M,T | Resumable/checksummed import, schema lifecycle and rollback fixtures | PASS / HOLD — all cutover, retirement and sign-off leaves: 46.1–48.4 |

Capability accounting: **45/45 enumerated; 43/45 implementation-qualified;
CAP-011 and CAP-036 INCOMPLETE; 17/45 retain an explicit activation/removal hold; 0
unexplained states**.

## Acceptance-criterion ledger

The criterion result is evaluated independently from capability implementation. `QUALIFIED` means its pre-cutover semantic evidence is complete. `HOLD` names the remaining leaf that must pass before final parity; an open hold is an enforced stop-ship result, not an unexplained failure.

| Requirement | Criteria and result | Evidence or remaining owner |
| --- | --- | --- |
| 1 | 1.1 QUALIFIED; 1.2 QUALIFIED; 1.3 QUALIFIED; 1.4 HOLD | Inventory checker and T; final enumeration/catalogs 48.1–48.2 |
| 2 | 2.1 QUALIFIED; 2.2 HOLD; 2.3 HOLD; 2.4 HOLD | ADRs/T; cutover records, bridges and duplicate-owner removal 46.1–47.6 |
| 3 | 3.1 QUALIFIED; 3.2 QUALIFIED; 3.3 QUALIFIED; 3.4 QUALIFIED | T: native reversible presentation and restart evidence |
| 4 | 4.1 INCOMPLETE; 4.2 INCOMPLETE; 4.3 INCOMPLETE; 4.4 INCOMPLETE; 4.5 INCOMPLETE | Epic 50: native ACP/project state and raster evidence pass locally; hosted channel messaging, CI/review state, complete failure-state coverage and reference-density parity remain open |
| 5 | 5.1 QUALIFIED; 5.2 QUALIFIED; 5.3 QUALIFIED; 5.4 QUALIFIED | P,S: independent protocol and hostile-input gates |
| 6 | 6.1 QUALIFIED; 6.2 QUALIFIED; 6.3 QUALIFIED; 6.4 QUALIFIED | P,S: row-zero tenant and cross-transport authorization evidence |
| 7 | 7.1 QUALIFIED; 7.2 QUALIFIED; 7.3 QUALIFIED; 7.4 QUALIFIED | P,S,M,T: identity, custody and recovery evidence |
| 8 | 8.1 INCOMPLETE; 8.2 INCOMPLETE; 8.3 HOLD; 8.4 QUALIFIED | Production channel subscription/reconnect evidence is pending under 51.4–51.8; cutover divergence/operator diagnostics remain at 46.4 |
| 9 | 9.1 INCOMPLETE; 9.2 INCOMPLETE; 9.3 INCOMPLETE; 9.4 QUALIFIED; 9.5 QUALIFIED | Production channel mutation, pagination, read-state and reconnect evidence is pending under 51.3–51.8; independent search and wake gates remain valid |
| 10 | 10.1 QUALIFIED; 10.2 QUALIFIED; 10.3 QUALIFIED; 10.4 QUALIFIED | P,S,T: project authority, real-Git and native review evidence |
| 11 | 11.1 QUALIFIED; 11.2 QUALIFIED; 11.3 QUALIFIED; 11.4 QUALIFIED; 11.5 QUALIFIED | S,O,T: ACP, configuration, private state, jobs and provider equivalence |
| 12 | 12.1 QUALIFIED; 12.2 QUALIFIED; 12.3 QUALIFIED; 12.4 QUALIFIED | S,T: semantic activity mapping and lifecycle supervision evidence |
| 13 | 13.1 QUALIFIED; 13.2 QUALIFIED; 13.3 QUALIFIED; 13.4 QUALIFIED | S,O,T: durable workflow/approval recovery and audit evidence |
| 14 | 14.1 QUALIFIED; 14.2 QUALIFIED; 14.3 QUALIFIED; 14.4 QUALIFIED | P,S,M,T: media and native/compatibility huddle evidence |
| 15 | 15.1 QUALIFIED; 15.2 QUALIFIED; 15.3 QUALIFIED; 15.4 QUALIFIED | S,M,T: moderation, retention/deletion and operator recovery evidence |
| 16 | 16.1 QUALIFIED; 16.2 QUALIFIED; 16.3 QUALIFIED; 16.4 QUALIFIED | P,S,O,T: pairing, provider, mesh and client contracts |
| 17 | 17.1 QUALIFIED; 17.2 HOLD; 17.3 HOLD; 17.4 HOLD | M/T inventory; live rehearsal, rollback and bridge controls 46.1–46.6 |
| 18 | 18.1 HOLD; 18.2 QUALIFIED; 18.3 HOLD; 18.4 HOLD | P/T negotiation; final matrix, retirement gates and source state 47.1–48.4 |
| 19 | 19.1 QUALIFIED; 19.2 QUALIFIED; 19.3 HOLD; 19.4 QUALIFIED; 19.5 QUALIFIED | S,M,L,O,T; cutover diagnostics and final operations docs 46.4, 48.3 |
| 20 | 20.1 INCOMPLETE; 20.2 QUALIFIED; 20.3 HOLD; 20.4 HOLD | Epic 51 production integration, fault, GPUI, restart and local Compose evidence is pending; final cutover evidence/report/sign-off remains at 46.6, 47.6, 48.1–48.4 |
| 21 | 21.1 QUALIFIED; 21.2 QUALIFIED; 21.3 INCOMPLETE; 21.4 QUALIFIED; 21.5 QUALIFIED; 21.6 QUALIFIED; 21.7 QUALIFIED; 21.8 QUALIFIED; 21.9 QUALIFIED | The profile boundary is established, but the enabled production channel composition remains unproved under 51.5–51.8 |

Criterion accounting: **93/93 enumerated; 67 QUALIFIED; 14 HOLD; 12
INCOMPLETE; 0 unexplained states**.

## Explained divergences and residual risks

| ID | Observation | Gate effect | Required disposition |
| --- | --- | --- | --- |
| DIV-001 | The retained Redis scale harness observes socket timeout before an already-satisfied delivery count; the source-identical temporary wrapper reversed only those observations. | Approved scoped Redis workload passed; unattended use of the original harness remains unsafe. | Correct the retained harness or replace it with an equivalent checked-in canonical load before final sign-off (48.4). |
| DIV-002 | An adverse all-matching 50,000-document search diagnostic exceeded OL-DAT-06, while the approved 500-hit corpus passed with p95/p99 30.154/75.544 ms. | Not a failure of the approved corpus; it is capacity headroom risk. | Preserve broad-hit observability/stops and reapprove the production corpus at activation (48.4). |
| DIV-003 | Harbor passed 58 hermetic tests with one declared live-stack provisioning skip; no Linux Buzz agent stack, external model trial or leaderboard score ran. | Canonical live scheduler/provider orchestration passed; no model quality/reliability claim exists. | Treat any Harbor deployment/model score as separately approved activation evidence, never as canonical execution authority (48.4). |
| DIV-004 | Web/mobile/admin compatibility uses injected test-owned service boundaries; Git/pairing use loopback and media uses test-owned stores. | Protocol contracts and fail-before-mutation ordering pass; production-route availability is not inferred. | Verify deployed routes during shadow/cutover and final operational sign-off (46.6, 48.4). |
| DIV-005 | Some focused Collab/GPUI gates used source-identical lean targets to avoid unrelated optional LiveKit dependency resolution; temporary changes were restored. | The exact production modules and test sources passed; no broad dev-graph success is invented. | Canonical release/package configurations remain the activation authority (48.4). |
| DIV-006 | Physical multi-node shared-compute and external hardware/provider saturation were not exercised; the approved deterministic scheduler, partition and process boundaries passed. | OL-MSH policy/fairness/fencing passes locally; deployment capacity is not claimed. | Keep shared compute disabled until separately approved deployment evidence and sign-off (48.4). |

All divergences are explicit and assigned. None authorizes a weaker production budget, hidden fallback, duplicate owner or completion claim.

## Stop-ship ledger

| Gate | State | Release condition |
| --- | --- | --- |
| STOP-SHIP-001 | OPEN | Tasks 46.1–46.6 pass shadow comparison, mirroring, divergence stops, rollback and aggregate cutover rehearsal. |
| STOP-SHIP-002 | OPEN | Tasks 47.1–47.6 pass write freeze, compatibility preservation, source/runtime retirement and the no-duplicate-owner audit. |
| STOP-SHIP-003 | OPEN | Tasks 48.1–48.4 regenerate catalogs, assemble final evidence/docs and record operational plus product sign-off. |
| STOP-SHIP-004 | OPEN | All 15 criterion HOLD states and all 18 capability activation/removal holds close with no new unexplained divergence. |
| STOP-SHIP-005 | OPEN | Production activation, irreversible migration and source retirement receive their separately required authorization; this qualification run grants none. |

Because every stop-ship row is open, this report deliberately rejects `COMPLETE`, `PARITY COMPLETE`, production activation, irreversible cutover and source-retirement claims. The next permitted work is the reversible cutover tooling in Task 46.1.

## 2026-08-27 native visual correction run

The visual application runner launched the Rust product composition with
`visual-tests,multiplayer-tools,rust-tools`, exercised visible expanded and
collapsed review states and captured the native GPUI raster at the two required
physical sizes. Both captures passed independent region gates; no reference
pixels, conversations or users are embedded in production or test fixtures.

| Evidence | Result |
| --- | --- |
| `actual-screenshot-1.png` | PASS — 1930×1262; expanded native ProjectDiff; structural similarity 96.83% |
| `actual-screenshot-2.png` | PASS — 1928×1298; review fully collapsed; structural similarity 97.14% |
| Rail gates | PASS — both contrast, edge density and foreground coverage comparisons |
| Expanded top/timeline/review/composer-status gates | PASS — all four regions independently satisfy thresholds |
| Collapsed top/timeline/timeline-right/composer-status gates | PASS — all four regions independently satisfy thresholds |
| Sparse/empty-content guard | PASS — canonical rail owners and a multi-turn native ACP thread populate the scenario |
| Review-state guard | PASS — requested expanded/collapsed state is asserted before raster capture |
| Exact-pixel parity | NOT CLAIMED — 27.85% and 48.07%; content identity is intentionally different |

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `ZED_PRODUCT_ID=rust cargo run --package zed --no-default-features --features multiplayer-tools,rust-tools` | PASS — the native application reached `Running target/debug/zed`; the validation instance was then stopped |
| `cargo test -p workspace --features test-support,multiplayer-tools collaborative_` | PASS — 25 unit and 1 review integration test; the final three-test workspace integration rerun also passed |
| `cargo test -p sidebar --features multiplayer-tools collaborative_` | PASS — 23 tests |
| `cargo test -p agent_ui --features test-support,multiplayer-tools collaborative_` | PASS — 15 unit and 3 integration tests |
| focused warning-denied Clippy for `workspace`, `sidebar`, `agent_ui`, `title_bar`, `client` and `channel` with all features | PASS |
| the same focused Clippy without all features | BLOCKED outside this visual diff — the existing all-target test configuration does not cover `RemoteConnectionOptions::Mock` in `title_bar.rs:644` |
| full `./script/clippy` | BLOCKED outside this visual diff — duplicate `tests` modules in `language_models/src/provider/bedrock.rs` and three obsolete zero-argument `clear()` calls in `onboarding/src/workspace_choice.rs` |
| canonical specification validator with `--require-complete --dialect canonical` | PASS with the specification's pre-existing decomposition warnings |

The visual evidence does not close the existing Epic 51 hosted channel,
live-service GPUI, restart or two-client acceptance gaps. It also does not prove
the remaining inline ACP code/diff-card timeline composition. Full CAP-036 and
Requirement 4 parity therefore remain incomplete.

## 2026-08-27 rich native renderer validation addendum

This run replaces the prior validation note that inline ACP code/diff semantics
were absent. The production adapter now uses the canonical `ThreadView` entry
list, and the final visual runner initializes the real language registry so the
fenced Rust block and native diff use Zed's normal syntax pipeline. Test mode
truthfully refuses to start a real rust-analyzer binary; that does not bypass
rendering or register a fake language server.

| Validation | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo test -p workspace --features test-support,multiplayer-tools collaborative_reference_and_responsive_geometry` | PASS — both reference sizes plus narrow and wide layouts |
| `cargo test -p agent_ui --features test-support,multiplayer-tools collaborative_` | PASS — 14 unit and 3 integration tests |
| `./script/clippy -p gpui -p acp_thread -p agent_ui -p workspace -p platform_title_bar` | PASS — release, all targets, all features, warnings denied |
| `./script/clippy --no-all-features --no-default-features -p gpui -p acp_thread -p agent_ui -p workspace -p platform_title_bar` | PASS after placing the shared collaborative renderer behind `multiplayer-tools` |
| `ZED_PRODUCT_ID=rust cargo check --package zed --no-default-features --features rust-tools` | PASS |
| `ZED_PRODUCT_ID=rust cargo check --package zed --no-default-features --features multiplayer-tools,rust-tools` | PASS |
| `ZED_PRODUCT_ID=rust cargo run --package zed --no-default-features --features multiplayer-tools,rust-tools` | PASS — reached `Running target/debug/zed`; validation instance stopped |
| `COLLABORATIVE_VISUAL_ONLY=1 cargo run -p zed --bin zed_visual_test_runner --features visual-tests,multiplayer-tools,rust-tools` | PASS — both exact captures and all semantic/region gates |
| Full `./script/clippy` | BLOCKED by unrelated existing errors: duplicate `tests` modules at `language_models/src/provider/bedrock.rs:99,2977`; obsolete zero-argument `clear()` calls at `onboarding/src/workspace_choice.rs:188,225,267` |

Final raster measurements:

| Capture | Review geometry | Structural/exact result | Semantic gates |
| --- | --- | --- | --- |
| 1930×1262 expanded | split x=1221.0; reference x=1221; tolerance ±2 | 96.61% / 12.77% | Markdown code, inline diff, content output, terminal, failed tool, authoritative avatar and non-sparse timeline present |
| 1928×1298 collapsed | review absent; timeline right edge x=1928.0 | 96.88% / 24.93% | rich ACP content present; released right-side timeline and compact composer/status pass |

The host-owned macOS controls are intentionally absent from deterministic GPUI
content rasters. A permission-granted native-window file may be supplied with
`COLLABORATIVE_NATIVE_WINDOW_CAPTURE`; otherwise the generated host-window
evidence document remains the truthful manual Screen Recording step. Epic 51
hosted-channel production evidence remains incomplete and was not inferred from
this populated visual scenario.

## 2026-08-27 native ownership reuse validation addendum

| Validation | Result |
| --- | --- |
| `cargo test -p workspace --all-features --lib collaborative_` | PASS — 28 tests, including ownership scanning, status behavior and responsive layout |
| `cargo test -p workspace --all-features --test collaborative_review` | PASS — native review routing |
| `cargo test -p agent_ui --features test-support,multiplayer-tools --lib collaborative_` | PASS — 14 tests, including canonical participant-reader lifecycle and native ACP composition |
| explicit unified Git UI/Editor native ProjectDiff test | PASS — Collaborative adapter and regular Git UI reference the same `Entity<ProjectDiff>` |
| `cargo test -p collab_ui --all-features --test patch_review_merge` | PASS — native review-action context converges and rejects unsafe variants |
| `./script/clippy -p workspace -p agent_ui -p sidebar -p git_ui` | PASS — release, all targets, all features, warnings denied |
| same focused Clippy with `--no-all-features --no-default-features` | PASS |
| `./script/clippy -p collab_ui` | BLOCKED outside this refactor by `clippy::redundant_clone` in `crates/collab_ui/tests/huddle.rs:518` after production Collab UI compiled |
| `ZED_PRODUCT_ID=rust cargo check --release --package zed --no-default-features --features rust-tools` | PASS |
| same Rust-product check with `multiplayer-tools,rust-tools` | PASS |
| `ZED_PRODUCT_ID=rust cargo run --package zed --no-default-features --features multiplayer-tools,rust-tools` | PASS — reached `Running target/debug/zed`; validation instance stopped |
| exact-size visual runner | PASS — 1930×1262 expanded boundary x=1221 and 96.63% structural similarity; 1928×1298 collapsed timeline edge x=1928 and 96.89%; all semantic/region gates pass |
| final `cargo fmt --all -- --check` and `git diff --check` | PASS |
| canonical specification validator with `--require-complete --dialect canonical` | PASS with the specification's pre-existing decomposition warnings |
| full `./script/clippy` | BLOCKED outside this refactor — duplicate `tests` modules at `crates/language_models/src/provider/bedrock.rs:99,2977` and obsolete zero-argument `clear()` calls at `crates/onboarding/src/workspace_choice.rs:188,225,267` |
| extra Sidebar release-test rerun | INCONCLUSIVE — exhausted disk while rebuilding a duplicate optimized test graph; only recoverable Cargo release artifacts were removed. Sidebar already passes both focused warning-denied Clippy matrices and the ownership source audit. |

The runner regenerated `actual`, `native`, `side-by-side`, `diff`,
`amplified-diff` and per-region metric artifacts. The pre-audit actual captures
are preserved as `before-reuse-audit-screenshot-{1,2}.png`. Unrelated or
external gaps remain separate rather than weakening an ownership invariant.
