# Collaborative workspace migration and deletion fault gate

Status: **PASS for Task 45.3**. Every executed importer, schema, shadow,
retention and deletion interruption or recovery fixture converged on its
declared canonical state. No production migration, deletion, cutover or data
restore was performed.

Captured on 2026-08-25 from source revision
`2daaba5496f6410668816ae1fb47c409f3f0e6cd`.

## Executed matrix

| Boundary                                                         | Command                                                                                                                                                                                                                                                                                                                                                                | Result                                                                                                                                                             |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Import, shadow, retention and deletion fault owners              | `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test -q -p collab --test buzz_import_recovery --test retention_deletion_faults --test community_deletion_executor --test community_deletion_recovery --test retention_worker --test retention_cache_push_cleanup --test retention_search_cleanup --test retention_media_cleanup --test collaboration_projection_rebuild` | 42 passed, 0 failed                                                                                                                                                |
| Live storage interruption, replica-shadow lag and schema rebuild | `COLLAB_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55433/postgres CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test -q -p collab --test collaboration_storage_recovery` against a uniquely named disposable PostgreSQL 17 container                                                                                                                               | 1 passed, 0 failed                                                                                                                                                 |
| Migration artifact and manifest contract                         | `PYTHONDONTWRITEBYTECODE=1 python3 deploy/collaboration/migrations/check.py`                                                                                                                                                                                                                                                                                           | All 20 ordered up/down pairs, checksums, schema ceiling, required-version rejection, URL policy and chart handoff passed                                           |
| Live schema lifecycle                                            | `deploy/collaboration/migrations/tests/smoke.sh`                                                                                                                                                                                                                                                                                                                       | Staged apply/resume, idempotent replay, reversible down/up, activation sealing, checksum-drift halt and halted-resume rejection passed on disposable PostgreSQL 17 |

The Rust matrix completed **43 passing tests** with no executed test failure.
The first sandboxed Cargo attempt reached no test because the pinned WebRTC
artifact was unavailable through restricted DNS; the identical command passed
after allowing its download.

## Fault outcomes

- The desktop and agent staging importers produced identical replay output,
  preserved the last verified count/hash checkpoint across interruption, halted
  on target divergence and restored the exact pre-boundary binary,
  configuration and data fixture before entering the terminal rolled-back
  state.
- Projection rebuilds remained deterministic and tenant-scoped, surfaced
  deliberately seeded drift, and rolled back a partial replacement without
  mutating authority. The live recovery drill also held a stale
  repeatable-read observer while the canonical projection advanced, then
  rebuilt the dropped projection checkpoint from unchanged event, command and
  outbox authority.
- A transaction-level outbox interruption rolled back the canonical mutation,
  receipt and outbox together. Retry applied exactly once and replay
  deduplicated without a second authority write.
- Retention resumed from the exact committed prefix after pre-commit failure,
  recovered unknown commit outcomes without repeating authority actions,
  preserved legal holds, withheld unsafe suffixes during authority outage and
  rejected foreign or regressing batches before mutation.
- Cache/presence, push, search and media cleanup advanced only after their
  derived targets converged. Unavailable targets, unknown checkpoint outcomes,
  duplicate delivery, stale generations and foreign batches retained a
  retryable checkpoint and never weakened canonical visibility.
- Both frozen retention generations converged after before-commit and
  after-commit uncertainty. Recovery uncertainty reloaded one rolled-back
  deletion, and uncertainty after each database, search, cache, push, object
  storage and Git phase reloaded one ordered effect and one terminal deleted
  state.
- The deletion executor resumed every pre-commit and outcome-unknown phase
  without skipping or repeating irreversible work. Operator recovery restored
  every reversible state, rejected every checkpoint at or beyond the recorded
  boundary, authorized before lookup and exposed only bounded redacted status.
- The migration runner applied the first six migrations, resumed the remaining
  fourteen, performed a no-op replay, rolled the final migration down and up,
  sealed `20260824000500`, and rejected rollback across that service-activation
  floor. A second database detected changed source bytes, durably recorded
  `checksum_drift` and refused an unapproved resume.

## Environment and cleanup

Docker Desktop's daemon socket was unresponsive before the live database
fixtures. Under the existing restart and force-stop approval, the hung backend
was terminated and Docker Desktop restarted; client and server both reported
version 29.7.2 before the database runs. The migration smoke cleaned its unique
network, container, image and drift directory. The separate
`codex-collab-recovery-task-45-3` container was removed after its test, and a
final inventory found no matching disposable container or migration image.

No production database, source snapshot, tenant, migration history, retention
record, object, Git repository, queue or deployment was contacted. This gate
proves the checked-in interruption/recovery fixtures; it does not authorize the
shadow-write or cutover operations owned by Tasks 46 through 48.
