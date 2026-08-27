# Collaboration scale gate

Status: **PASS for the approved Task 45.4 local integrated load gate**. This is not production-cutover approval: the checked-in Nostr compatibility surface is not mounted as a production WebSocket listener, external APNs was not contacted, and production-network or cross-region latency remains an activation-stage obligation.

Captured on 2026-08-25 from source revision `c7ceee5e8082a6ad86470a1ef29da1b37052e431`.

## Approved budgets and results

| Surface | Approved budget | Observed result | Result |
| --- | --- | --- | --- |
| Connection admission | OL-CON-01/02: reject before allocating beyond the effective bounded semaphore | 512 current RPC connection guards acquired in 0.057 ms; the 513th was rejected; releasing all permits allowed immediate reacquisition. The current RPC handler semaphore is 256, below the 1,024 registry ceiling. | PASS |
| Subscription admission | OL-CON-05 and production owners: at most 1,024 Nostr subscriptions per connection and 4,096 local bus subscriptions | 4,096 concurrent bus subscriptions acquired in 13.320 ms; the 4,097th returned `CapacityExceeded`; dropping all subscriptions returned the count to zero. The six Nostr subscription regressions passed the per-connection bound and cleanup paths. | PASS |
| Fan-out queue and latency | OL-CON-03: queue at most 1,000 frames, warn at p99 depth 800; ordered delivery and cleanup | 256 subscribers received 768 events each: 196,608 exact ordered deliveries in 44.222 ms (4,445,965 deliveries/s). Publish p50/p95/p99 was 0.051/0.090/0.151 ms; maximum measured depth was 768 and cleanup returned the subscription count to zero. | PASS |
| Redis relay bus | At least 95% of ideal scoped reduction and 0% irrelevant scoped delivery | Live Redis with 64 communities × 100 events and 1/2/4 interested pods produced 64.0× reduction in every row, 98.44% irrelevant old-bus delivery and 0.00% scoped irrelevant delivery. | PASS |
| Message windows | OL-DAT-07: stable cursor pages capped at 200, no repeated/no-progress pages | On 100,000 canonical messages, 2,000 prepared 201-row `limit + 1` window transactions completed at 15,790.554 tx/s. Transaction p50/p95/p99 was 0.457/0.700/0.841 ms. Four repository regressions passed dense-cursor uniqueness, bounded recursive depth, overlay reconciliation and immutable-row rejection. | PASS |
| Read-state reconciliation | OL-DAT-08: bounded owner/frontier state and one-page reconnect convergence contract | Nine focused `collaboration_domain::read_state` tests passed merge-order convergence, incomplete-load fencing, scope/owner isolation, frontier/override bounds and counter exhaustion; the protocol gate separately passed reconnect compatibility. | PASS |
| Authorized search and freshness | OL-DAT-05/06: approved corpus/load p95 ≤500 ms, p99 ≤2 s; page ≤500; unauthorized vectors absent | On 50,000 public documents with an approved 500-hit cohort plus 5,000 otherwise-matching restricted documents, 2,000 prepared search-plus-freshness transactions completed at 485.440 tx/s. Transaction p50/p95/p99 was 13.546/30.154/75.544 ms; all 5,000 restricted vectors remained null and the checkpoint stayed clean. | PASS |
| Push wake claims | OL-PUS-02/03: batches ≤16, claim 30 s, queue oldest warning at 30 s; bounded recovery and cleanup | 400 prepared claim transactions completed at 326.380 tx/s with p50/p95/p99 12.248/16.309/18.976 ms. Exactly 6,400 wakes moved to attempt 1 in batches of 16 and 3,600 remained pending. The dependent live recovery gate reclaimed one expired batch in 20.130 ms, and current outbox/cleanup regressions passed 11/11. | PASS |
| Operational consumers | OL-CON/DAT/PUS metrics and stop conditions remain closed and bounded | The observability checker passed 4 private dashboards, 55 operational limits and 16 stop signals. | PASS |

## Workloads and provenance

The live relay run used `redis:7-alpine` at digest `sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf`. It ran the retained Buzz scenario with 64 communities, 100 events per community, one subscribed community and 1/2/4 pods. The unmodified harness reported a timeout after receiving the exact expected count because its polling loop checks a subscriber socket timeout before checking the already-satisfied delivery total. A temporary wrapper changed only that observation order, after which the live Redis assertion passed; the wrapper and container were removed. The three checked-in model/unit tests also passed unchanged. This harness measures the Redis bus boundary, not client rendering or database ingest.

The database runs used PostgreSQL/`pgbench` 14.20 from `postgres:14-alpine` at digest `sha256:14f02666642586a64d6fae8ef42d479fd76456a77c73ae8a626b8fe323b76d22`. All 20 checked-in up migrations were applied before seeding one tenant with 100,000 events/messages, 50,000 community-visible search documents, 5,000 `authorized_restricted` documents, one clean search checkpoint, 10,000 active push leases and 10,000 pending wake jobs. Prepared transaction logs supplied the reported nearest-rank p50/p95/p99 values. Message queries used the production channel-window index order and fetched `limit + 1`; search retained the production authorization/vector predicates, ranking, 50-row result bound and freshness aggregate; push retained the production valid-lease join, ordered `FOR UPDATE OF job SKIP LOCKED` selection, 16-job batch and 30-second claim.

The current search and push owners changed after the Task 22.12 baseline only to add retention-expiry indexing/cancellation entry points; the search query, freshness query and wake-claim SQL measured here remain unchanged. Task 22.12's disposable physical-replica test remains the current multi-host freshness evidence: 4,047,576 bytes of induced WAL lag returned to zero within 5,664 ms and all 5,000 burst rows appeared. Its expired-claim recovery moved exactly 16 jobs to attempt 2 in 20.130 ms without moving deferred work. No production database, Redis deployment, provider or user data was contacted.

## Adverse diagnostic and residual risks

An additional non-gating diagnostic deliberately made all 50,000 public documents match the same broad term. Its 2,000 transactions completed at 5.963 tx/s with p50/p95/p99 1,238.842/2,024.679/3,022.799 ms, exceeding OL-DAT-06. The normative latency objective is explicitly scoped to the approved corpus/load, so this does not invalidate the passing 500-hit gate. It does establish a release risk: broad-hit traffic must remain observable and subject to the existing unavailable/partial-result and alert behavior; production corpus approval must be revisited if common terms approach this selectivity.

The existing RPC service exposes an effective 512-connection and 256-handler ceiling rather than the larger registry maxima, and the compatibility Nostr WebSocket is not yet a routed production listener. The lower fail-closed ceilings passed, but capacity planning must advertise them and later activation/shadow gates must not infer 10,000-listener readiness from this local admission test. Likewise, the Redis harness defect should be corrected in its source owner before it becomes an unattended CI signal.

## Reproduction

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s /Users/ahmad.vegah/repos/imagineerings/zed/projects/buzz/perf -p 'test_*.py'
REDIS_URL=redis://127.0.0.1:56379/0 python3 /tmp/codex-relay-bus-scale-runner.py --mode redis --redis-timeout 30
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test -p collab --test task_45_4_scale -- --nocapture
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test -q -p collab --test collaboration_subscription_bus --test nostr_subscriptions --test message_window_repository --test collaboration_search_indexer --test collaboration_search_query --test collaboration_search_repository --test collaboration_push_outbox --test retention_cache_push_cleanup
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test -q -p collaboration_domain read_state
PYTHONDONTWRITEBYTECODE=1 python3 deploy/collaboration/observability/check.py
```

The temporary Rust scale target, Redis polling wrapper, SQL/transaction-log inputs and both uniquely named containers were removed after capture. The PostgreSQL load shapes and dependent recovery procedure are frozen in `test-results/collaborative-workspace/search-push-plan.md`; reruns must use disposable resources and reject drift from the production owners.
