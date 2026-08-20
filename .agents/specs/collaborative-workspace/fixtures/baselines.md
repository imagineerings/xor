# Buzz performance and known-gap baseline

## Purpose

This document freezes the reproducible evidence available in the Buzz source tree before consolidation into Sim. It is a preservation baseline, not an approval of the target architecture and not evidence that an unmeasured subsystem is fast enough for production. A missing measurement is a failing readiness condition until the owning scale-gate leaf records an approved budget and a passing result.

The baseline covers the seven subsystems required by Task 3.4: relay, Redis fan-out, search, push, workflow, relay mesh and Harbor orchestration. Each record names its command, environment, observed result, preservation budget and known defect. The downstream task references are the canonical owners of missing measurements or behavior.

## Capture environment

| Field | Captured value |
|---|---|
| Date | 2026-08-14 |
| Source revision | `4dbf73b1d36cb2e328a9a1a02aaa47b8dd59b19f` on `sim-dev-editors` |
| Host | Apple Silicon arm64, 10 logical cores, 32 GiB memory |
| Operating system | Darwin |
| Python | CPython 3.14.5 |
| Rust | `rustc 1.95.0`, `cargo 1.95.0` |
| Available infrastructure | No Docker daemon socket, live Redis, or PostgreSQL test service was available |
| Unavailable benchmark tools | `uv` and the Python `pytest` module were not installed |

No host identifier, credential, private key, endpoint secret or benchmark provider token is preserved here.

## Baseline summary

| Baseline ID | Subsystem | Captured result | Preservation budget | Known evidence gap | Downstream owner |
|---|---|---|---|---|---|
| BASE-RELAY-001 | Relay | No end-to-end load or latency harness exists under `projects/buzz/benchmarks/` or `projects/buzz/perf/` | No compatibility, authorization, bounded-frame, reconnect or graceful-drain regression; production readiness fails until numeric connection, queue, latency and freshness budgets pass | Shutdown wiring is explicitly untested; bus harness excludes relay, database, WebSocket and client work | 4.4, 15.6, 16.3, 45.4 |
| BASE-FANOUT-001 | Redis fan-out | Deterministic model reports 64.0x reduction and 0.00% scoped irrelevant delivery for 1, 2 and 4 pods; three harness tests pass | At least 95% of ideal reduction and at most 0.00% scoped irrelevant delivery in the frozen scenario | Live Redis mode was not run and measures neither database ingest nor user-visible latency | 16.1, 16.2, 16.3, 45.4 |
| BASE-SEARCH-001 | Search | Three unit tests pass; 19 PostgreSQL integration tests are ignored by the crate without infrastructure | All three unit tests and all 19 database tests must pass; zero unauthorized or excluded-kind results; no latency/throughput readiness claim until an approved load budget passes | No database execution, freshness measurement, privacy load test or latency baseline | 16.4, 16.5, 22.1–22.3, 22.11, 22.12, 45.4 |
| BASE-PUSH-001 | Push | Fifteen unit tests pass; six PostgreSQL integration tests are ignored by the crate without infrastructure | All 15 unit and six database tests must pass; wake-only payloads disclose no message content; no queue/provider readiness claim until an approved load budget passes | No database execution, provider delivery, retry-storm, queue-depth or platform-attestation measurement | 2.5, 22.4–22.13, 43.7, 45.4 |
| BASE-WORKFLOW-001 | Workflow | 154 tests pass; two PostgreSQL integration tests are ignored; no measured tests | All 154 current tests and both database tests must pass; no action may report success when skipped; approval/restart paths must pass before parity | Several actions and approvals are explicitly placeholders; no throughput, crash-recovery or replay baseline | 34.1–34.9, 35.2–35.4, 45.5 |
| BASE-MESH-001 | Relay mesh | 32 tests pass; no ignored or failed tests; no measured tests | All 32 current tests must pass; unknown versions, replay, unauthorized capacity and fail-open scheduling remain rejected; production readiness requires approved partition, fairness and resource budgets | No physical multi-node partition, hardware/provider, sustained load or shared-compute policy evidence | 2.6, 41.1–41.5, 45.5 |
| BASE-ORCH-001 | Harbor orchestration | 22 Python files parse, 53 test functions are discoverable and four prompt hashes match; no tests or live trials executed | All 53 discovered tests, lint, prompt hashes and artifact checks must pass; no score, reliability or resource readiness claim until a pinned live trial budget passes | `uv`, `pytest`, Docker stack, Linux agent binaries and model endpoints were unavailable; no leaderboard score exists | 31.6, 33.1–33.7, 45.5 |

## BASE-RELAY-001 — relay

### Source authority

- `projects/buzz/TESTING.md`
- `projects/buzz/perf/RELAY_BUS_SCALING.md`
- `projects/buzz/crates/buzz-relay/src/main.rs`
- `projects/buzz/crates/buzz-relay/src/connection.rs`
- `projects/buzz/crates/buzz-relay/src/state.rs`

### Commands and observed result

The available performance-artifact inventory was inspected with:

```sh
rg --files projects/buzz/benchmarks projects/buzz/perf
```

The inventory contains the Harbor orchestration benchmark and the Redis bus-scaling harness. It contains no full-relay benchmark that combines authenticated connections, event ingest, PostgreSQL, Redis, WebSocket delivery, reconnect and client consumption. The live-relay recipe in `projects/buzz/TESTING.md` is a functional smoke test rather than a measurement harness and was not run without its Docker services.

### Preservation budget

- Existing protocol acceptance/rejection and tenant-authorization behavior may regress by zero cases.
- Existing configured connection, frame, rate and queue bounds remain upper bounds until Task 4.4 approves replacements.
- A cutover or performance claim fails if Task 45.4 has not recorded numeric connection count, queue depth, p50/p95/p99 latency, freshness and graceful-drain budgets against the integrated stack.
- Passing the Redis fan-out model is necessary but not sufficient evidence for relay readiness.

### Known defects and missing evidence

- `projects/buzz/crates/buzz-relay/src/main.rs` explicitly records that the `serve` shutdown wiring has no automated test. Jitter selection, awaiting drain before abort and propagation to every graceful-shutdown listener are therefore unguarded at the call site.
- The bus harness explicitly excludes database ingest, WebSocket framing, relay business logic and client rendering.
- No reproducible full-stack connection, subscription, ingest, reconnect, drain or latency measurement is frozen.

## BASE-FANOUT-001 — Redis fan-out

### Source authority

- `projects/buzz/perf/RELAY_BUS_SCALING.md`
- `projects/buzz/perf/relay_bus_scaling.py`
- `projects/buzz/perf/test_relay_bus_scaling.py`
- `projects/buzz/crates/buzz-pubsub/src/topic.rs`

### Commands and observed result

```sh
python3 projects/buzz/perf/relay_bus_scaling.py --mode model
python3 -m unittest discover -s projects/buzz/perf -p 'test_*.py'
```

The deterministic scenario was 64 communities at 100 events/s, one subscribed community and 1, 2 or 4 interested pods.

| Pods | Old cluster ingress/s | Old average pod ingress/s | Scoped cluster ingress/s | Scoped average pod ingress/s | Reduction | Old irrelevant/pod | Scoped irrelevant/pod |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 6,400 | 6,400 | 100 | 100 | 64.0x | 98.44% | 0.00% |
| 2 | 12,800 | 6,400 | 200 | 100 | 64.0x | 98.44% | 0.00% |
| 4 | 25,600 | 6,400 | 400 | 100 | 64.0x | 98.44% | 0.00% |

The harness assertion passed. All three unit tests passed, including the mutant that routes irrelevant global-firehose traffic into scoped subscribers.

### Preservation budget

- Observed reduction must remain at least 95% of the ideal `communities / subscribed_communities` ratio.
- Scoped irrelevant delivery must remain at most 0.00% in the frozen scenario.
- The 1/2/4-pod rows must remain 64.0x in deterministic model mode.
- Task 45.4 must rerun the real Redis mode and add integrated relay latency/queue budgets before cutover.

### Known defects and missing evidence

Live Redis mode was not run because no Redis service was available. Model output is arithmetic contract evidence only. It does not measure Redis scheduling, network transport, duplicate delivery, database capacity, WebSocket fan-out or end-user freshness.

## BASE-SEARCH-001 — search

### Source authority

- `projects/buzz/crates/buzz-search/`
- `projects/buzz/migrations/0008_fresh_install_search_allowlist.sql`
- `projects/buzz/TESTING.md`

### Command and observed result

```sh
cargo test --manifest-path projects/buzz/Cargo.toml \
  -p buzz-search -p buzz-push-gateway -p buzz-workflow -p buzz-relay-mesh
```

For `buzz-search`, three unit tests passed and 19 integration tests were reported ignored because they require PostgreSQL. No test was marked measured. The command completed with zero executed failures.

### Preservation budget

- All three current unit tests remain passing.
- All 19 database integration tests must execute and pass in the infrastructure-backed gate; leaving them ignored is not parity evidence.
- Authorization must precede limiting and ranking, and prohibited private kinds must produce zero documents, hits, counts or existence signals.
- Task 22.12 and Task 45.4 must approve and pass indexing-queue depth, projection freshness and query-latency budgets. Until then, search performance readiness is failed rather than unbounded.

### Known defects and missing evidence

There is no frozen PostgreSQL result, corpus size, indexing throughput, query latency, stale-projection recovery or tenant-contention measurement. Unit success does not validate the database authorization boundary.

## BASE-PUSH-001 — push delivery

### Source authority

- `projects/buzz/crates/buzz-push-gateway/`
- `projects/buzz/docs/nips/NIP-PL.md`
- `projects/buzz/TESTING.md`

### Command and observed result

The shared Rust command recorded under BASE-SEARCH-001 ran `buzz-push-gateway`: 15 unit tests passed and six PostgreSQL integration tests were ignored. No test was marked measured. No APNs, FCM or other external provider call was made.

### Preservation budget

- All 15 current unit tests remain passing.
- All six database integration tests must execute and pass before data-plane cutover.
- Push remains a wake hint: zero message text, attachment data, private project data or conversation existence may enter the provider payload.
- Outbox delivery remains idempotent for a stable operation/source ID, with bounded retries and visible terminal failure.
- Task 22.12 and Task 45.4 must approve and pass queue-depth, oldest-item age, provider latency, retry-amplification and recovery budgets. ADR-005 must approve platform and attestation scope first.

### Known defects and missing evidence

There is no frozen provider-delivery result, database result, queue saturation, outage recovery, retry storm, device-lifecycle or attestation measurement. Supported platform scope remains approval-gated by ADR-005.

## BASE-WORKFLOW-001 — workflows and approvals

### Source authority

- `projects/buzz/crates/buzz-workflow/`
- `projects/buzz/crates/buzz-relay/src/workflow_sink.rs`
- `projects/buzz/VISION*.md`

### Command and observed result

The shared Rust command recorded under BASE-SEARCH-001 ran `buzz-workflow`: 154 tests passed and two PostgreSQL integration tests were ignored. No test was marked measured.

### Preservation budget

- All 154 current tests remain passing, and both database tests must execute and pass.
- A workflow action may never return a successful outcome for skipped or placeholder work.
- Trigger deduplication, durable step checkpoints, approvals, cancellation and crash/restart replay must preserve one externally visible effect per operation ID.
- Task 34.8 must pass cron, webhook, event, approval-race, retry, crash, restart and redacted-failure scenarios.
- Task 45.5 must approve and pass queue age, trigger-to-start latency, recovery time and concurrent-run budgets before workflow cutover.

### Known defects and missing evidence

- `buzz-workflow/src/executor.rs` describes action dispatch as placeholder-backed. `SendDm` and `SetChannelTopic` return `NotImplemented`.
- Without the optional HTTP feature, reaction and webhook actions return successful-looking skipped results. This violates the target truthful-outcome budget and must not survive consolidation.
- Approval creation is marked TODO; `finalize_run` converts a suspended approval into explicit failure because approval gates are not implemented.
- Long-delay scheduled resume is future work.
- No durable crash/restart, approval-race, throughput or saturation baseline is frozen.

## BASE-MESH-001 — relay mesh and shared compute

### Source authority

- `projects/buzz/crates/buzz-relay-mesh/`
- `projects/buzz/crates/buzz-relay/src/mesh_boot.rs`
- `projects/buzz/VISION_MESH.md`

### Command and observed result

The shared Rust command recorded under BASE-SEARCH-001 ran `buzz-relay-mesh`: all 32 tests passed, with no ignored or failed tests. No test was marked measured.

### Preservation budget

- All 32 current tests remain passing.
- Unknown wire versions, stale generations, replay, invalid signatures, unauthorized membership and unapproved capacity must fail closed.
- A mesh partition or provider failure may not silently move a job onto untrusted or ineligible compute.
- Task 41.5 must approve and pass partition recovery, revocation propagation, resource-cap and scheduler-fairness budgets.
- Task 45.5 must approve and pass sustained orchestration and recovery budgets after ADR-006 sets trust and eligibility policy.

### Known defects and missing evidence

The unit suite does not provide physical multi-node partition, packet-loss, hardware capability, provider execution, resource exhaustion, fairness under sustained load or recovery-time evidence. Shared-compute policy is intentionally unresolved until ADR-006.

## BASE-ORCH-001 — Harbor Buzz Orchestra

### Source authority

- `projects/buzz/benchmarks/harbor-buzz-orchestra/README.md`
- `projects/buzz/benchmarks/harbor-buzz-orchestra/harbor_buzz_orchestra/`
- `projects/buzz/benchmarks/harbor-buzz-orchestra/testbed/`
- `projects/buzz/benchmarks/harbor-buzz-orchestra/manifests/`
- `projects/buzz/benchmarks/harbor-buzz-orchestra/scripts/`

### Commands and observed result

The Python sources and pinned prompts were checked without third-party packages:

```sh
python3 - <<'PY'
# Parse every benchmark Python file with ast, count test_* definitions,
# and compare each manifest prompt's declared SHA-256 with its file bytes.
PY
```

The check passed: 22 Python files parsed, 53 test functions were discovered and all four manifest prompt hashes matched their source bytes. The full documented commands were not run because `uv`, `pytest`, the benchmark Docker stack, Linux agent binaries and model endpoints were unavailable:

```sh
cd projects/buzz/benchmarks/harbor-buzz-orchestra
uv run --extra dev pytest -q
uv run --extra dev ruff check .
cd testbed
uv run --extra dev pytest -q
uv run --extra dev ruff check .
```

No Harbor task, leaderboard trial or external model request was executed, and no score was produced.

### Preservation budget

- All 53 discovered tests and both Ruff commands must pass in the pinned benchmark environment.
- All manifest prompt hashes must match before a trial starts.
- Trial isolation, unique identities/channels, bounded agent budgets, artifact retention and cancellation cleanup may regress by zero cases.
- Static parsing is not orchestration evidence. Task 45.5 must define a pinned task set, attempt count, pass/reliability floor, wall-clock/resource ceilings and partial-failure recovery budget, then record a passing live result.

### Known defects and missing evidence

No current score, success rate, duration distribution, token/resource result, multi-agent contention result, model/provider failure result or cancellation cleanup result is frozen. The harness also depends on external package availability for some graders, so a reproducible environment and artifact provenance are part of the missing gate evidence.

## Adjacent documented incomplete behavior

These gaps were discovered while reviewing the required Buzz performance and vision sources. They do not change approved scope; they prevent later work from treating the current Buzz implementation as complete merely because a similarly named component exists.

| Gap ID | Evidence | Frozen incomplete behavior | Canonical follow-up |
|---|---|---|---|
| GAP-BUZZ-001 | `projects/buzz/crates/buzz-relay/src/api/media.rs` | Durable media bytes have active-work admission bounds but no persistent per-pubkey storage quota | 38.2–38.8, 45.3 |
| GAP-BUZZ-002 | `projects/buzz/crates/buzz-relay/src/api/git/store.rs` | Object-store Git backend is marked as not yet wired into the push path | 25.1–25.9, 45.2 |
| GAP-BUZZ-003 | `projects/buzz/VISION_MODERATION.md` | Escalation has durable records but no platform inbox; notices are best-effort; automod and a moderator tier are intentionally absent | 36.1–36.8, 45.3 |

## Reproduction and update rules

1. Run commands from the repository root at the recorded source revision unless a command explicitly changes directory.
2. Do not compare a model-mode result with a live-service result as if they measured the same boundary.
3. If ignored infrastructure tests become executable, append their exact environment and result; do not replace the fact that they were ignored in this capture.
4. A later approved numeric budget supersedes only the corresponding provisional readiness blocker. Compatibility, privacy, fail-closed and truthful-outcome budgets remain mandatory.
5. Record tool versions, configuration, dataset/fixture identity and raw result artifact path for every later measurement. Redact credentials and host identifiers.
6. Any regression against a preservation budget requires an approved requirements/design change; it is not accepted as migration drift.
