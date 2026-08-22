# Search and push load/failure gate

Status: **PASS for the Task 22.12 local component gate**. This evidence does not approve production cutover or replace the integrated percentile, multi-host and external-provider gate owned by Task 45.4.

Captured on 2026-08-22 from source revision `f46fc41ef5c213dda8e7b6607f5ac0a99f78809f`.

## Approved budgets and results

| Probe | Approved local budget | Observed result | Result |
| --- | --- | --- | --- |
| Search document plus checkpoint indexing | At least 400 transactions/s; average at most 15 ms; at most 1% above 50 ms | 10,000/10,000; 4,912.332 transactions/s; 0.812 ms average; 0/10,000 above 50 ms | PASS |
| Authorized search plus freshness query | At least 40 transactions/s; average at most 200 ms; at most 30% above the diagnostic 100 ms threshold | 2,000/2,000; 49.187 transactions/s; 162.239 ms average; 524/2,000 (26.2%) above 100 ms | PASS |
| Push wake claim | At least 90 claim transactions/s and 1,440 wakes/s; average at most 50 ms; no transaction above 100 ms; no batch above 16 | 400/400; 100.464 claim transactions/s and 1,607.417 wakes/s; 39.543 ms average; 0/400 above 100 ms; every batch exactly 16 | PASS |
| Expired push-claim recovery | Reclaim exactly one bounded batch within 100 ms without moving deferred work | 16 expired jobs reclaimed in 20.130 ms; all moved to attempt 2 under one claim; unrelated pending jobs remained deferred | PASS |
| Search projection recovery | Divergence must be observable and fenced recovery must complete within 100 ms | One checkpoint reported divergent with 1,300,401 ms age; fenced recovery completed in 1.531 ms; the following 1.048 ms query reported zero affected checkpoints | PASS |
| Physical replica recovery | Pausing the replica must produce positive byte lag; after resume, lag must reach zero within 10 seconds and every burst row must appear | A 5,000-document burst produced 4,047,576 bytes of WAL lag; lag reached zero within a conservative 5,664 ms; replica contained all 70,000 documents and all 5,000 burst rows | PASS |
| Push dependency failure behavior | The complete executor suite must pass retry, exhaustion, revocation and provider-failure cases | `push_gateway` passed 14/14 tests, including transient retry, retry exhaustion, revocation race, permanent endpoint behavior and fixed reconnect payload | PASS |

The 100 ms search threshold is a diagnostic concurrency floor, not a production user-visible latency objective. Its 26.2% late fraction is a mandatory input to Task 45.4, which owns integrated p50/p95/p99 budgets and broad-hit query optimization.

## Environment and provenance

| Component | Value |
| --- | --- |
| Host | macOS 26.6.1 (25G76), Darwin 25.6.0, arm64 |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)`; `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Database | PostgreSQL 14.20, native `aarch64-unknown-linux-musl` container |
| Container image | `postgres:14-alpine`; `sha256:14f02666642586a64d6fae8ef42d479fd76456a77c73ae8a626b8fe323b76d22`; arm64 |
| Load driver | PostgreSQL `pgbench` 14.20, prepared statements, native container network |

Every database and container was uniquely named and disposable. The primary, physical replica, search-only rerun container, anonymous replica volume and temporary network were removed after capture. No production database, provider or deployment was contacted.

## Workload contract

The database was built by applying these checked-in migrations in order:

1. `20260820000100_collaboration_events.up.sql`
2. `20260820000300_collaboration_projections.up.sql`
3. `20260820000500_collaboration_search.up.sql`
4. `20260820000700_collaboration_channels.up.sql`
5. `20260822000200_collaboration_push.up.sql`

The synthetic tenant contained 50,000 public canonical search documents, 5,000 `authorized_restricted` documents with storage-generated search vectors left null, 10,000 projection checkpoints, 10,000 active push leases and 10,000 pending wake jobs. A controlled `cohort` token selected exactly 500 public documents and 500 otherwise-identical restricted documents. The query returned only public candidates; restricted rows stayed outside the searchable vector and candidate set. The physical-replica burst added 5,000 documents, producing a final 70,000-document replica corpus. The measured database size was 85,721,891 bytes.

The generated benchmark SQL deliberately mirrors the checked-in production statements rather than introducing a second application implementation:

- Index transactions execute the document and projection-checkpoint shapes owned by `UPSERT_DOCUMENT_SQL` and `UPSERT_CHECKPOINT_SQL` in `crates/collab/src/search/indexer.rs`, under transaction-local `app.community_id`.
- Query transactions execute the authorized community-visible candidate, rank/limit and freshness aggregate shapes owned by `SEARCH_SQL` and `FRESHNESS_SQL` in `crates/collab/src/search/repository.rs`. Visibility and the non-null search vector are predicates inside candidate selection, before rank and limit.
- Push transactions execute the `FOR UPDATE OF job SKIP LOCKED` claim shape owned by `CLAIM_WAKES_SQL` in `crates/collab/src/push/outbox.rs`, with executor constants `PUSH_WAKE_BATCH_LIMIT = 16` and `PUSH_CLAIM_MILLIS = 30_000` from `services/push_gateway/src/executor.rs`.
- Failure probes preserve the production maximum-attempt behavior (`PUSH_MAX_ATTEMPTS = 8`), expire only one prior claim, repair one version-fenced projection checkpoint, and pause only the physical replica.

## Reproduction commands

Use a uniquely named disposable PostgreSQL 14 database and provide test-only URLs without committing credentials. Apply the migration sequence and synthetic corpus above, then run the following approved bounded commands with prepared statements:

```sh
pgbench -n -U postgres -c 4 -j 4 -t 2500 -M prepared -L 50 -r -f search_index.sql load_2212
pgbench -n -U postgres -c 8 -j 4 -t 250 -M prepared -L 100 -r -f search_query.sql load_2212
pgbench -n -U postgres -c 4 -j 4 -t 100 -M prepared -L 100 -r -f push_claim.sql load_2212
```

Each script is one explicit transaction. `search_index.sql` installs the tenant, inserts one public document and its clean checkpoint, and commits. `search_query.sql` installs the tenant, selects the top 50 authorized public `cohort` candidates, computes the complete `collaboration_search` freshness aggregate, and commits. `push_claim.sql` installs the tenant, selects at most 16 eligible jobs through valid active leases with `SKIP LOCKED`, assigns one random claim for 30 seconds, increments attempts, and commits. A runner must reject any script whose constants, predicates or column set differ from the production owners named above.

The live database integration commands were:

```sh
COLLAB_TEST_DATABASE_URL="${COLLAB_TEST_DATABASE_URL}" cargo test -p collab --test search_push_privacy collaboration_search_excludes_private_content_from_vector_and_partial_index -- --nocapture
COLLAB_TEST_DATABASE_URL="${COLLAB_TEST_DATABASE_URL}" cargo test -p collab --test search_push_privacy collaboration_search_live_query_ranks_only_authorized_candidates_and_marks_lag -- --nocapture
COLLAB_TEST_DATABASE_URL="${COLLAB_TEST_DATABASE_URL}" cargo test -p collab --test search_push_privacy collaboration_push_schema_enforces_live_tenant_and_idempotency_constraints -- --nocapture
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo test -p push_gateway -- --nocapture
```

The three isolated PostgreSQL tests passed 1/1 in 0.29 s, 0.41 s and 0.29 s. The push suite passed 14/14 in 0.13 s after compilation.

## Failure and recovery procedure

1. Seed one search checkpoint as `diverged` and 120 seconds old. Verify the freshness query reports it, update only that source record with its next projection version and clean hashes, and require the next aggregate to report zero affected rows within budget.
2. Complete normal push claims, defer all remaining pending jobs, expire one exact 16-job lease and run one production-shape claim. Require one new claim identifier, attempt 2 on exactly 16 jobs and no movement of the deferred set.
3. Start a PostgreSQL physical streaming replica from the disposable primary, require `pg_is_in_recovery()` and zero initial byte lag, pause only the replica, insert 5,000 search documents on the primary, require positive WAL lag, resume the replica and poll until lag is zero. Require the replica's exact burst and total counts. The copied `pg_hba.conf` must retain PostgreSQL ownership and mode `0600` before reload.
4. Run the complete `push_gateway` suite to exercise retry scheduling, maximum-attempt exhaustion, lease revocation races, permanent endpoint generations, App Attest/APNs provider failures and the wake-only reconnect payload.

## Limits and follow-up

- This is a local single-host component floor. It does not characterize production networks, storage classes, cross-region replicas or competing workloads.
- Provider behavior is validated through the production provider interfaces and fakes; no external APNs request was sent.
- The clean search run exposes a broad-hit concurrency cost despite passing the bounded floor. Task 45.4 must set and meet production p50/p95/p99 latency and freshness objectives before cutover.
- Task 45.4 must automate the integrated workload, include multi-host failure recovery and apply deployment telemetry/SLO acceptance. This evidence cannot be used alone to enable a release gate.
