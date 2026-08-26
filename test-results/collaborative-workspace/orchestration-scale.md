# Workflow, agent and mesh orchestration gate

Status: **PASS for the approved Task 45.5 canonical-owner orchestration gate**. Durable workflow admission, delegated jobs, provider execution and mesh scheduling completed within their approved limits with no duplicate execution. This is not a Harbor leaderboard score or production shared-compute activation: the retained Buzz benchmark's opt-in deployment stack and external model trial were not run.

Captured on 2026-08-25 from Zed source revision `353fb31dd5bd65cf04d1ff7b2b67d3db7cf3ff29` and read-only Buzz source revision `e092824ac729a83a1bdab007ee9670f1f6756b99`.

## Approved benchmark

The deterministic benchmark uses one attempt per case and requires 100% completion, zero duplicate canonical creates/results/leases and zero accepted stale results. Its pinned workloads and ceilings are:

| Surface | Pinned workload | Approved ceiling |
| --- | --- | --- |
| Workflow | Fill and release the canonical scheduler's community, deployment and definition admission boundaries against live PostgreSQL; replay trigger and lease operations and reconstruct all recovery scenarios | OL-EXE-04: 1,000 queued/community, 10,000 queued/deployment, 16 concurrent/community and 4 concurrent/definition; trigger-to-start, queue-age and recovery observations must remain below the 30 s warning boundary |
| Delegation | Hold 16 parents and 240 direct children, replay every child create, reject the next community job, complete every child and aggregate every parent | OL-EXE-07: depth at most 8, children/job at most 16 and active jobs/community at most 256; exact replay creates no second child |
| Provider | Run the pinned Kubernetes-v1 fixture through discovery, two pre-secret negotiations, deploy, replay, hostile output, cancellation and process-tree cleanup | OL-EXE-06/OL-MSH-05: cancellation reaches the runtime within 2 s, local resources clean within 10 s (provider resources within 30 s), and one executor/session/result exists |
| Mesh | Queue 16 equal-weight requesters × 16 requests, admit every 100 ms, exercise partition/replay/revocation, false resource claims and cleanup | OL-MSH-04: 2 concurrent/requester, 16 queued/requester, 256 queued/community, p95 wait at most 30 s and normalized service ratio at most 1.25; OL-MSH-05 requires fenced leases and OL-MSH-06 requires finite resource ceilings |

The 30-second workflow observation ceiling is the accepted OL-EXE-04 queue-age warning boundary and the mesh/provider cleanup ceiling is the stricter applicable OL-EXE/OL-MSH limit. Any capacity over-admission, duplicate terminal publication, stale-result acceptance, silent provider fallback, missing finite resource bound or failure to clean the workload is a gate failure regardless of elapsed time.

## Results

| Surface | Observed result | Result |
| --- | --- | --- |
| Workflow admission and recovery | A source-identical lean target passed 6/6 recovery and 9/9 scheduler tests. The live PostgreSQL 17 scheduler suite completed in 2.13 s, filled and rejected all four OL-EXE-04 ceilings, reconnected its pool, released capacity and admitted the next run. Trigger, action, retry, approval and lease crash replays converged without a second effect or accepted stale generation. | PASS |
| Delegated jobs | The checked-in release suite passed 4/4. A temporary source-level scale extension held exactly 256 active jobs (16 parents plus 240 children), replayed all 240 child creates with `applied_creates=240`, rejected job 257, completed all children and aggregated all 16 parents. The measured orchestration section took 477 µs. | PASS |
| Agent execution | The checked-in release suite passed 4/4: one session/result, idempotent cancellation, crash recovery on the same session and stale-boundary fencing. The remote-execution library filter passed 4/4 for exactly-once launch/result replay, heartbeat expiry, cancellation shutdown and disconnect recovery. | PASS |
| Provider boundary | The release conformance target passed 3/3 in 1.11 s. It retained the same staged executable through negotiation/deploy, rejected incompatible and hostile output, performed no replay launch, and the cancellation case stopped the provider plus descendant and removed staging within its asserted 2-second cancellation bound. | PASS |
| Mesh partition and fairness | The release target passed 5/5. The full 256-entry queue rejected the next request, logical p95 wait was 24.3 s, normalized service ratio was 1.0, all 256 leases were released, and 1,024 unique gossip frames plus partition recovery/revocation/replay and false-resource scenarios remained fenced. | PASS |

Across the canonical measured workloads, 256 delegated jobs, 240 exact child-create replays, 15 workflow cases, 8 agent execution cases, 3 provider cases and 5 mesh cases produced no duplicate execution or accepted stale result. Rust build time is excluded from the wall budgets; only the measured test bodies and their owned service/process lifetimes are compared with operational limits.

## Harbor compatibility audit

The retained `harbor-buzz-orchestra` source is compatibility/workload evidence, not a second durable scheduler, job graph, provider lifecycle or mesh authority. Its two manifest files are byte-pinned at SHA-256 `546ef2e881f3886ea3085320abfcdc06d47f1081eeac5ee204300b76335e4b74` and `f4739c37d997fa9a44630edaf640ebcb4e67d5d71780c2aa2f4fbebdb696f8f8`; all four referenced persona hashes matched.

The current source discovers 59 tests rather than the 53 in the original static baseline. The root suite passed 35/35 in 3.25 s and the testbed suite passed 23/24 in 3.94 s, with only `test_create_is_idempotent_and_isolated` skipped by its declared `BUZZ_TESTBED_LIVE=1` gate. Both Ruff commands passed. Frozen lockfiles were resolved only in disposable copies by changing the unavailable package-mirror host to PyPI's hash-identical artifacts; the original Buzz checkout and lockfiles remained untouched.

The skipped case requires the legacy relay, database, owner credential and Buzz CLI. A leaderboard trial additionally requires Linux Buzz agent binaries and a configured model endpoint. Those inputs are absent, and running an external model would add cost and validate the compatibility executor rather than the canonical Zed owners. No external model request, task score, reliability claim or production service interaction is reported. The live canonical PostgreSQL scheduler and provider-process gates above supply the required execution/recovery result; deployment-stage Harbor provisioning and any leaderboard score remain explicit activation evidence, not a substitute for canonical correctness.

## Provenance and reproduction

The Rust gates ran on arm64 with `rustc 1.97.1` and `cargo 1.97.1`. The workflow database was disposable `postgres:17-alpine` at digest `sha256:18cfe3ef5e6815560c98237d6216d1e5119702fb0f3894c8785dd58b8bbe5d73`. The canonical test selections were:

```sh
cargo test -p agent --test job_delegation --release -- --nocapture
cargo test -p agent --test job_execution --release -- --nocapture
cargo test -p agent remote_execution --lib --release -- --nocapture
cargo test -p remote --test agent_provider_conformance --release -- --nocapture
cargo test -p remote --test mesh_compute --release -- --nocapture
uv run --frozen --extra dev pytest -q
uv run --frozen --extra dev ruff check .
uv run --frozen --project testbed --extra dev pytest -q testbed/tests
uv run --frozen --project testbed --extra dev ruff check testbed
```

The Harbor selections ran from frozen disposable virtual environments after the unavailable lockfile mirror host was replaced in disposable lock copies with PyPI's hash-identical artifacts. The workflow and delegation scale extensions used the exact production modules and checked-in integration sources in disposable lean targets because the full Collab dev graph includes an unrelated external LiveKit download. The extensions only added measurement scenarios: no production source was changed. All temporary Rust targets, Python environments, mirror-adjusted lock copies, test artifacts and the uniquely named PostgreSQL container were removed after capture. No production database, provider, relay, community or user data was contacted.
