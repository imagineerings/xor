# Migration Plan: Collaborative Workspace and Buzz Consolidation

## Principles

1. Migrate authority by aggregate, never by copying an entire product at once.
2. Preserve signed events and original data until target integrity and behavior are verified.
3. A compatibility adapter may duplicate representation, not authority.
4. Dual writes require one accepted command, an outbox, deterministic idempotency and reconciliation; clients never independently write two authorities.
5. Rollback is a tested operation with an explicit last reversible checkpoint.
6. `projects/buzz` remains buildable and security-fixable until all retirement gates pass.

## Phases and gates

### Phase 0: Baseline and approval records

- Freeze a source revision and generate crate, kind, NIP, migration, UI, client and operational inventories.
- Record ADR-001 through ADR-006.
- Capture black-box protocol, CLI, desktop, mobile, web, admin, migration, performance and security baselines.
- Build the compatibility matrix and fixture corpus, including each Buzz desktop archive/config version and all 30 SQL migrations.
- Record checksums/counts for Postgres partitions/tables, object metadata, key identifiers, local archives and agent snapshots. Never export private key material into reports.

**Entry:** approved spec pack.

**Exit:** complete CAP ledger; independent baseline passes; approval gates required for Phase 2+ are recorded.

**Rollback:** not applicable; read-only discovery.

### Phase 1: Native GPUI vertical slice

- Add reversible `WorkspacePresentation` selection to existing Sim onboarding/settings.
- Compose the collaborative shell from existing project/worktree, sidebar/thread, ACP/action-log and diff entities.
- Ship behind a local presentation feature flag. No server or data authority changes.
- Validate reference geometry, accessibility and persistence.

**Entry:** Phase 0 inventory automation.

**Exit:** existing users default to unchanged editor; opted-in users can use a real project, native ACP thread and native review diff in the collaborative composition.

**Rollback:** disable presentation flag; preference remains inert; no data conversion.

### Phase 2: Canonical domain, identity and service foundations

- Extract/port pure Buzz event/tenant rules into the approved Sim collaboration-domain boundary.
- Add exact Nostr adapters and stable internal command/projection contracts.
- Implement explicit Sim-account ↔ Nostr-key binding and import Buzz keys through Sim credentials providers.
- Establish typed tenant admission and common authorization across existing Sim RPC and temporary Buzz Nostr ingress.
- Align Axum/SQLx/SeaORM dependencies before combining service processes. Until then, run the Buzz-derived Nostr ingress as a versioned sidecar using the same tenant catalog and outbox contract.
- Create projection provenance, migration checkpoints and drift observability.

**Entry:** ADR-001 and ADR-002 approved; Phase 0 differential suite green.

**Exit:** old and new ingress decode to equivalent domain commands; no new UI reads legacy Tauri state directly; identity/key imports are verified.

**Rollback:** stop new ingress, restore binding/config snapshot, resume old relay writes. No dual write is enabled yet.

### Phase 3: Communication read migration

- Backfill communities, membership, channel/message/thread/DM/read/presence/search/notification projections from signed events.
- Run shadow reads: the native GPUI client consumes canonical projections while a differential worker compares legacy Buzz query responses.
- Continue legacy relay as write authority for Nostr-authored communication.
- Reconcile by event ID/addressable coordinate and projection version; page/window comparisons include cursors and overlays, not just row content.

**Entry:** Phase 2 domain/auth and projection rebuild proven.

**Exit:** defined observation window with zero unexplained authorization, ordering, unread, search or notification divergence; mobile/web/CLI read compatibility green.

**Rollback:** native client reads legacy-compatible adapter; discard/rebuild derived projections.

### Phase 4: Communication write authority cutover

- Route accepted commands through the canonical collaboration service.
- Persist the signed event once, then use a transactional/outbox projection path for channel/member/thread/search/push/audit effects.
- Legacy relay becomes a protocol adapter over the same command path; direct legacy database writes are disabled.
- Use stable operation/event IDs so retries and mixed clients are idempotent.

**Entry:** Phase 3 exit and an approved write window.

**Exit:** all supported ingress paths produce identical authoritative events and projections; legacy direct-write counters remain zero.

**Rollback:** before the point of no return, pause writes, drain outbox, verify no divergence, switch routing to the old binary against the same authoritative event log. After new-only schema writes, rollback requires snapshot restore and coordinated write freeze.

### Phase 5: Project, Git and agent authority integration

- Map Sim projects/repositories to NIP-MP/NIP-34 identities without replacing local `Project`/`GitStore` authority.
- Cut review/CI/status events to canonical collaboration writes and link them to native diff/action IDs.
- Adapt channel mentions, jobs and NIP-AO to Sim ACP execution.
- Import personas, teams, managed agents, engrams, metrics, snapshots and local archives; preserve cryptographic coordinates and privacy gates.
- Port remote-provider execution with Sim process/permission/credential owners.

**Entry:** ADR-003, communication cutover stable.

**Exit:** no native agent session is executed by both Sim and Buzz runtimes; old Git/agent clients remain wire-compatible; imported agent state passes fidelity tests.

**Rollback:** stop new job admission, allow running sessions to terminate/cancel, restore runtime routing and retained configuration; never run both owners for the same session/job ID.

### Phase 6: Workflow and infrastructure capability cutover

- Port/complete workflows, approvals, audit, moderation, retention/deletion, media, huddles, push, pairing and mesh/shared compute.
- Cut each aggregate separately after shadow/differential evidence.
- Move operational dashboards, health/readiness, metrics and admin controls to Sim ownership.

**Entry:** corresponding ADRs and security reviews; core collaboration stable.

**Exit:** all CAP-027 through CAP-035 behavior has a canonical Sim owner; documented workflow/rate-limit/huddle gaps are completed or explicitly accepted.

**Rollback:** per-aggregate routing rollback before new-only writes; workflows/jobs are quiesced, not duplicated; deletion cannot roll back after its recorded irreversible checkpoint.

### Phase 7: Client and deployment migration

- Repoint CLI, web, mobile and admin clients using the published compatibility matrix.
- Replace Buzz desktop distribution with Sim Collaborative Workspace.
- Merge Compose/Helm/release ownership; retain required compatibility binaries/sidecars as versioned Sim artifacts.
- Run canary cohorts per community with automated rollback gates.

**Entry:** server capabilities and compatibility tests green.

**Exit:** supported clients use canonical endpoints; no production deployment requires the Buzz Tauri desktop or a duplicate authority.

**Rollback:** route clients to last compatible service release; desktop users can switch to Editor presentation; schema/data rollback follows the prior phase checkpoints.

### Phase 8: Retirement and final parity

- Freeze legacy writes and perform a final full reconciliation.
- Wait the approved rollback window and verify traffic/usage thresholds for legacy endpoints and binaries.
- Remove Tauri/React desktop build, duplicate agent/MCP runtimes and superseded server modules in dependency-safe commits.
- Preserve protocol fixtures, formal models, required compatibility adapters, license notices and source history.
- Mark `projects/buzz` reference-only or remove it according to repository-history approval.

**Entry:** every CAP ID green; no open critical compatibility/security defect.

**Exit:** no prohibited duplicate owner, no build-time dependency on retired sources, final evidence ledger and operator sign-off.

**Rollback:** removal occurs only after the rollback window. Restoration requires an explicit release rollback from version control; old binaries must not be pointed at a schema past their compatibility ceiling.

## Data migration matrix

| Source | Target owner | Method | Integrity evidence | Rollback |
| --- | --- | --- | --- | --- |
| Buzz `events` partitions | Canonical signed event log | Preserve bytes/IDs/signatures; attach tenant/provenance metadata without resigning | Counts by tenant/kind/month, IDs, signatures, addressable heads | Retain source partitions read-only until Phase 8 |
| Buzz channel/member/DM/thread tables | Canonical projections | Rebuild from event log plus service-issued authority records; import non-event state explicitly | Differential channel windows, memberships, counters, hide state | Drop/rebuild target projections |
| Workflows/runs/approvals | Workflow owner | Versioned import preserving run/step/approval status and secrets by reference | Counts, state-machine legality, token/hash verification | Quiesce and restore source snapshot |
| Audit log | Audit owner | Preserve entries/hash chain per community; start new chain segment only with explicit bridge entry | Full-chain verification and head records | Retain original chain and restore head |
| Moderation/retention/deletion | Admin lifecycle owner | Import state and checkpoints before enabling workers | State legality, subject/actor/community counts | Workers remain disabled until verified |
| Push leases/outbox | Push owner | Preserve encrypted events; rebuild effective lease/outbox state | Lease generation, expiry, endpoint authority and queue reconciliation | Disable gateway and rebuild from events |
| Git repo registry/object storage | Git hosting adapter | Preserve coordinates, repo names, refs and object hashes; import permission records | `git fsck`, ref/object hashes, clone/push differential | Retain source bucket/volume read-only |
| Media object store/index | Media owner | Copy or re-index by content hash; preserve URLs/aliases during compatibility | Object hash, MIME, size, tenant prefix and sampled decode | Keep original bucket and URL routing |
| Redis presence/typing/cache | Derived runtime state | Do not copy; allow expiry and repopulate from live clients/events | Empty-start and expiry behavior | Restart old service; no data restore |
| Desktop keyring/fallback keys | Sim credentials provider | Enumerate identifiers; import one at a time; read-back/sign challenge; delete source only after user/verification gate | Public-key match and signed challenge | Preserve source record until cutover confirmation |
| Desktop config, drafts, read state and local archive | Sim settings/session/collaboration stores | Versioned idempotent import with per-record source version | Fixture corpus, counts/hashes and UI readback | Keep original file/database backup |
| Personas/teams/agents/snapshots/engrams/metrics | Sim agent + collaboration owners | Import private aggregate first, then verify/rebuild public projections | Cryptographic coordinate, CAS chain, redaction and snapshot fidelity | Preserve export and old records |
| Sim existing project/Git/ACP state | Existing Sim owners | No data move; add stable collaboration bindings | Existing regression suites and binding round trip | Remove inert bindings if rolled back |

## Temporary bridges

| Bridge | Allowed phase | Direction | Reconciliation | Removal criterion |
| --- | --- | --- | --- | --- |
| Nostr ingress sidecar → Sim domain service | 2-7 | Commands/events inward; protocol responses outward | Operation/event ID, domain version and response fixture | Axum/SQLx alignment complete and Nostr routes hosted by final service deployment |
| Legacy Buzz reads versus canonical projections | 3-4 | Read-only shadow comparison | Tenant/query/cursor keyed differential metrics | Observation window has zero unexplained divergence |
| Canonical outbox → legacy projections | 4-6 only if needed | One-way derived write | Outbox sequence, source ID/version and drift scan | Every supported client reads canonical projection/adapter |
| Buzz ACP/provider shim → Sim agent runtime | 5-7 | Job/session command inward; observer/activity outward | Stable session/job ID and exactly-one executor lease | Remote images/providers use Sim-owned runtime directly |
| `buzz` CLI shim | 2-8 and optional long-term | CLI syntax to canonical APIs | Golden stdout/stderr/exit-code tests | Approved usage threshold and documented replacement |
| Buzz Opus huddle adapter | 6-8 or approved long-term | Audio/lifecycle compatibility | Huddle/session/participant IDs and event parity | ADR-004 criterion and supported-client floor reached |
| Old URLs/deep links | 1-8 and optional long-term | Alias to canonical navigator | Normalized entity IDs and telemetry-free local counters | Explicit compatibility policy permits removal |

No bridge may perform bidirectional last-writer-wins reconciliation. Conflicts stop the affected aggregate and require the precedence rule below.

## Precedence and reconciliation

1. A verified signed event is deduplicated by event ID; addressable/replaceable heads use the exact protocol ordering.
2. A canonical domain command is deduplicated by tenant and idempotency key.
3. Local project/Git/ACP state wins for its canonical aggregate; a stale collaboration projection is rebuilt, not merged into local state.
4. During shadow reads, legacy is serving authority and canonical results are diagnostic only.
5. After write cutover, canonical authority wins and legacy direct writes are rejected. Any observed legacy-only write triggers an automatic cutover halt.
6. User-visible conflicts retain both candidate records in diagnostic storage, expose a resolution state and never silently overwrite security-sensitive data.

## Observability and automatic gates

Per tenant/aggregate, record without private content:

- accepted/rejected commands by adapter and reason class;
- event/projection/outbox sequence lag and drift counts;
- differential read result hashes, cursor/overlay differences and authorization mismatches;
- import checkpoints, record/object counts, integrity failures and elapsed time;
- legacy direct-write attempts and traffic by client/version;
- connection/subscription/queue/backpressure/replica freshness;
- job/workflow/provider/huddle/push/mesh lifecycle outcomes;
- rollback readiness and last reversible checkpoint.

Any cross-tenant mismatch, signature/authorization disagreement, data-loss count, unbounded queue or legacy-only write after cutover is a stop-ship and automatic rollback trigger where rollback remains safe.

## Removal criteria

### Buzz React/Tauri desktop

- GPUI visual/product parity for CAP-036-037 and all desktop-only state imported.
- Supported desktop update path and rollback release tested.
- No server capability depends on a Tauri command.

### Buzz agent, ACP and MCP binaries

- Tool-by-tool and ACP lifecycle parity, remote image/provider migration, snapshot/persona/memory fidelity and exactly-one execution evidence.
- Compatibility shim retained if external ACP users still require the binary name.

### Buzz relay/database/pubsub service modules

- Nostr and old-client conformance against final service; all writes enter the canonical command/event path.
- Projection and authorization differential windows clean; schema owner and operations moved.

### Legacy client and operator surfaces

- Published support window expired or explicit long-term ownership accepted.
- Replacement commands/URLs documented; usage below approved threshold; security update path remains.

### `projects/buzz`

- Every CAP ID satisfies Requirement 20.4.
- Required protocol/formal/test sources have a permanent Sim-owned location.
- Apache notices and history policy approved.
- Workspace builds, packages and deployments contain no unintended path dependency on retired sources.
