# Buzz relay, database and pub/sub retirement manifest

Date: 2026-08-25

Status: **PREPARED — REMOVAL HOLD**

This manifest prepares retirement of the duplicate `buzz-relay`, `buzz-db` and `buzz-pubsub` service path without changing or deleting `projects/buzz`, production routing, schemas, processes or data. The canonical Collab service remains the only proposed serving and write authority. Removal requires the live usage, rollback, migration and preservation gates below.

## Frozen source baseline

| Source | SHA-256 |
| --- | --- |
| `projects/buzz/crates/buzz-relay/Cargo.toml` | `4eaea23d07c05dbe8c4cdf7a15f350eba28fcb9a5a9d8fdd3e0b75ce41aa1a37` |
| `projects/buzz/crates/buzz-relay/src/main.rs` | `85e14dfce9e2f6de7eb3b042436e30846d14450a6bf80784fbfaaf3a925a587b` |
| `projects/buzz/crates/buzz-relay/src/router.rs` | `4982f086d4068713505aa34e983f53a68bd4e9fd3591af4eeb6412d5b3596f4d` |
| `projects/buzz/crates/buzz-db/Cargo.toml` | `352e33727e7a942180e02e22a4e2ed4ccf0ba9407165e0f995b178693d90375e` |
| `projects/buzz/crates/buzz-db/src/lib.rs` | `5fcb0610f5ebbffb57f5484b71697d92cc8fea7bec5a48408e73d252ae4d9053` |
| `projects/buzz/crates/buzz-pubsub/Cargo.toml` | `58253aac14b9d8ff2859bf89b2a6e1113fa98800d21de352b765b5f934dc7b3c` |
| `projects/buzz/crates/buzz-pubsub/src/lib.rs` | `ed398f25889f1fcf3ed2546054d35383bd62386fe24505ab80e4ae78442dc8d2` |
| `projects/buzz/crates/buzz-pubsub/src/topic.rs` | `4f136a478327459ebc053c314946891c8eb5c72f312c30e99e31d8395c2d2e95` |

The supplied Buzz checkout is an external source baseline, not a Zed workspace dependency. Its 30 ordered SQL migrations and service source remain rollback, license and compatibility evidence until separately approved preservation and deletion gates complete.

## Network and process audit

| Legacy path | Canonical owner/disposition | Retirement assertion |
| --- | --- | --- |
| `buzz-relay` public TCP listener, default `0.0.0.0:3000`, serving root NIP-01 WebSocket/NIP-11 plus HTTP bridge, operator, invite, moderation, webhook, media, Git, huddle and optional web/admin routes | One Collab public service on port 8080 behind the approved Ingress or Gateway. Native RPC and the Nostr, Git, media, workflow, moderation, huddle and compatibility adapters remain in that process or their explicitly separated services. | Remove the Buzz listener and route/service targets only after adapter traffic and active-client thresholds pass. Do not leave a hidden fallback, proxy hop or loopback route to port 3000. |
| Buzz health listener, default port 8080; Prometheus exporter, default port 9102; optional Unix socket | Canonical `/healthz` and `/metrics` service surfaces and chart monitoring policy | Remove Buzz probes, scrape targets, socket mounts and service discovery together with its process. The canonical public listener's use of 8080 is not permission to bind both processes to the same deployment target. |
| Buzz outbound Postgres writer/optional reader, Redis, object store, push, pairing and optional mesh connections | Typed canonical Collab configuration and explicit dependent services under ADR-001 and Tasks 44.1–44.4 | Remove every `BUZZ_*` endpoint/credential and legacy NetworkPolicy/secret binding. The canonical boundary deliberately accepts no Buzz aliases and disabled features reject leftover configuration. |
| `buzz-db` library inside `buzz-relay` | Canonical Collab repositories and the separately privileged migration runner | No standalone process is hidden here, but no Buzz SQL function or background sweep may remain callable after relay retirement. |
| `buzz-pubsub` Redis subscriber/reconnect loops and local broadcast channels | Canonical Collab outbox/fan-out envelope and subscription bus over the configured transport | Stop all subscriber, cache-invalidation and connection-control loops after clients drain. Redis is an optional transport/performance dependency, never a write or authorization owner. |

The repository manifest/lockfile audit finds no Zed dependency on `buzz-relay`, `buzz-db` or `buzz-pubsub`. Remaining references are intentional source inventory, import, protocol and conformance evidence. A deployment-specific audit must still enumerate live pods/processes, Services, listeners, routes, scrape targets, jobs, sockets and environment bindings; source inspection alone cannot prove they are absent.

## Route ownership audit

| Buzz route family | Canonical disposition |
| --- | --- |
| NIP-01 WebSocket, NIP-11, NIP-98 `/events`, `/query` and `/count`, NIP-05 | Canonical `nostr_compat` codecs and Collab Nostr HTTP/subscription owners. Task 45.1 proves signed-event, query, count, subscription and reconnect semantics against the independent corpus. |
| Operator/community, membership, invites and join policy | Canonical tenant admission, membership/identity, authorization and administrative RPC owners. Compatibility clients use versioned adapters rather than a legacy service hop. |
| Moderation, audit and feedback | Canonical moderation and audit repositories/projections; no legacy database query remains exposed. |
| Git smart HTTP, hook and policy routes | Canonical Collab Git authorization/registry/review and NIP-34 compatibility owners. Repository bytes remain on the retained canonical Git volume. |
| Blossom media routes | Canonical media admission, validation/object store and Blossom adapter; Task 38.7 proves upload/download/range/alias/error parity. |
| Workflow webhook | Canonical workflow webhook, trigger, durable scheduler/admission, run, approval and audit owners. |
| Huddle, push, pairing and mesh integrations | Canonical huddle boundary and explicitly separated push gateway/pair relay/mesh configuration. Retiring Buzz does not collapse those approved service boundaries into Collab. |
| Static web/admin fallback and mesh demo echo | Retire. They are legacy presentation/testbed routes, not compatibility or data authorities. Unknown old paths must return the canonical closed response, not reach Buzz. |

The retirement route set is closed: deployment routing points a public authority to one canonical handler or to no handler. There is no path-based split that could send a mutation to both services.

## Schema and write-authority audit

Buzz's 30 migrations define its old event, community, channel/member, user, moderation, retention, push, invite, deletion, workflow and operational tables. The canonical Collab schema is a distinct ordered set of 21 reversible `*.up.sql` migrations covering identity bindings; signed events and replaceable heads; projections/outbox/search; migration checkpoints; channels/messages; push; projects/Git/review; jobs; workflows/run leases/approvals/scheduler admission; audit; and moderation.

| Data/write family | Sole canonical writer after cutover | Legacy disposition |
| --- | --- | --- |
| Signed events, replaceable heads and Nostr queries | Collab `EventRepository` and its command/outbox transaction | Freeze all Buzz `events` writes and background TTL/partition/replica-fence mutations. |
| Communities, membership, identity, channels, messages, reactions, threads and DMs | Canonical tenant/domain repositories and collaboration projections | Freeze direct Buzz projection-table writes; read only through verified migration/rollback evidence until removal. |
| Search, moderation, audit, feedback and deletion | Canonical search, moderation, audit and deletion owners | Stop Buzz index maintenance, sweepers and administrative writers. |
| Push, Git, media and service-specific metadata | Canonical push outbox, Git registry/review, media authority and object store | Preserve canonical external resources; remove only legacy rows/configuration after receipts and rollback policy permit it. |
| Jobs and workflows | Canonical job repository and workflow repositories, including durable scheduler/admission and fenced leases | Disable Buzz workflow sink/engine and database methods; never dual-enqueue or dual-complete a run. |
| Migration execution | Canonical privileged migration job over the fixed 21-migration manifest | `BUZZ_AUTO_MIGRATE`, Buzz partition creation and all Buzz DDL are disabled before service removal. No Buzz down migration is run against canonical data. |

Task 47.1 enforces this boundary per operation: direct legacy writes are rejected and measured, while a compatibility adapter may obtain only a canonical-write permit. Cutover does not mirror canonical writes back into Buzz. Legacy tables remain read-only rollback evidence until retention/deletion approval, and canonical migration checkpoints/receipts—not table-name similarity—prove imported coverage.

## Pub/sub and transient-state audit

Buzz Redis owns community-scoped `channel`, `global`, `cache-invalidate` and `conn-control` topics plus ephemeral presence, typing, rate-limit and NIP-98 replay keys. None is a durable source of record.

- Canonical durable mutations commit to Postgres with their outbox/receipt before fan-out. The versioned, tenant-bound `FanoutEnvelope` carries an outbox sequence, provenance and payload hash; the subscription bus replays from durable authority, deduplicates and closes lagging consumers.
- Redis transport delivery cannot authorize, create or repair a record. A missing, delayed or duplicate message is recovered from the durable cursor/replay path or fails closed; it never invokes a Buzz database writer.
- Presence, typing, connection-control, cache and replay entries are transient. They expire or are rebuilt from canonical connections/policy and are not imported as durable data.
- Retirement stops new Buzz publication first, drains or closes its subscribers, then removes Buzz-prefixed keys only under an operationally approved cleanup. It never flushes a shared Redis deployment.

Thus no duplicate write authority or unintended relay-to-Buzz/Redis-to-Buzz mutation path remains in the proposed topology.

## Proposed retirement change

Once every gate below is satisfied, a separately approved retirement change may remove Buzz relay/database/pub-sub build and deployment inputs, stop the relay and its background loops, remove its routes/probes/scrapes/secrets, and later retire legacy schemas under the approved data-retention policy. It must not delete canonical Postgres, Redis, object, Git, push, pairing or mesh resources, run Buzz down migrations against canonical data, or remove frozen source/history. This manifest performs no live mutation.

Required gates:

- Task 45.1 protocol differential evidence remains passing with no unexplained route, wire or failure-frame divergence.
- Task 47.1 records zero direct legacy writes and approved relay, database and pub/sub adapter-read, adapter-write, active-client, observation-window and rollback-window thresholds pass for the exact deployment checkpoint.
- Migration receipts and aggregate cutover hashes prove every required legacy row is imported or explicitly classified; Task 46.6 rollback/rehearsal evidence remains valid.
- Canonical service readiness, migration-manifest, Postgres/Redis/object/Git dependency, multi-replica subscription replay and recovery checks pass before and after routing withdrawal.
- Live network/process inspection finds no legacy listener, route target, pod/process, migration/sweeper job, Redis subscriber or direct database credential in active use.
- Task 47.5 preserves artifacts, licenses and source history, and Task 47.6 confirms one owner and zero unintended dependencies.
- A human explicitly approves source, deployment and later data/schema retirement at their separate gates.

Until then, disposition is **HOLD**: preserve Buzz source and legacy data unchanged and do not alter routing, processes, credentials or schemas.

## Validation commands

```text
shasum -a 256 <eight frozen source files above>
find projects/buzz/migrations -maxdepth 1 -type f -name '*.sql' | sort
find crates/collab/migrations -maxdepth 1 -type f -name '*.up.sql' | sort
rg -n 'buzz-relay|buzz_relay|buzz-db|buzz_db|buzz-pubsub|buzz_pubsub' Cargo.toml Cargo.lock crates/*/Cargo.toml services/*/Cargo.toml tools/*/Cargo.toml
rg -n 'route\(|TcpListener::bind|UnixListener::bind' projects/buzz/crates/buzz-relay/src
rg -n 'buzz:|PUBLISH|SUBSCRIBE|SET|ZADD|INCR' projects/buzz/crates/buzz-pubsub/src
```
