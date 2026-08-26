# No-duplicate owner and dependency audit

Date: 2026-08-25

Status: **PASS — REPOSITORY AND PROPOSED POST-RETIREMENT TOPOLOGY**

Source retirement status: **HOLD**

This audit closes the repository-level owner and dependency check for the retirement proposals in this directory. It finds one canonical owner for every durable aggregate family and zero unintended dependencies on the proposed retired Buzz desktop, relay, database, pub/sub, ACP, agent or development-MCP sources. It does not delete `projects/buzz`, alter routing or credentials, stop a process, migrate live data or assert that an uninspected deployment has already removed the legacy runtime.

## Manifest and executable result

The root `cargo metadata --no-deps` graph contains the canonical `collab` binary and the separately scoped `pair-relay` binary. `push_gateway` is a root-workspace library used by the separately deployed push-gateway image contract. None of `buzz-relay`, `buzz-db`, `buzz-pubsub`, `buzz-acp`, `buzz-agent` or `buzz-dev-mcp` appears in the root `Cargo.toml`, `Cargo.lock`, any root crate manifest or the resolved root package set.

`tools/buzz_compat` deliberately declares its own workspace. Its metadata contains one package, `buzz_compat`, and exactly one binary, `buzz`. That retained client shim owns no server listener, database, queue, ACP session, MCP child, provider or workflow execution. Its `http://localhost:3000` development default is a compatibility endpoint selected by a user invocation, not a Zed build dependency or a hidden relay launch path.

| Executable or component | Post-retirement role | Authority result |
| --- | --- | --- |
| `collab` | Canonical collaboration service and repository boundary | Sole server-side collaboration command/write authority. |
| `push_gateway` | Separately deployed delivery adapter over canonical push-outbox claims | Owns provider delivery attempts only; it cannot create the canonical notification or wake record. |
| `pair-relay` | Stateless encrypted pairing-frame relay | Owns transient forwarding only; it cannot decrypt, authorize or persist account/workspace state. |
| Agent `JobExecutionCoordinator` and its remote adapter | Canonical collaboration job executor | One generation-fenced executor path; remote execution is an adapter under the same claim. |
| `buzz` | Retained versioned CLI compatibility client | No durable or execution authority. |
| Buzz desktop/service/ACP/agent/MCP binaries | Retirement candidates preserved under HOLD | No root-workspace package, launch or write path. |

## Listener, route and process result

| Surface | Canonical boundary | Duplicate-owner check |
| --- | --- | --- |
| Collaboration public API, health and metrics | `collab`, deployed by `deploy/collaboration/charts/collaboration`; the declared service port is 8080 | One chart selector and one canonical handler set. No deployment manifest refers to a Buzz relay, socket, health listener or metrics exporter. |
| Push public delivery and health | Separate push-gateway deployment on 8080/8081 | Its chart, credentials and network policy are separate from Collab routing; it claims canonical outbox work and has no notification/write authority. |
| Pairing relay | `pair-relay` at the explicitly configured `PAIR_RELAY_BIND_ADDR` | Transient ciphertext forwarding is intentionally separate from the identity, grant and workspace owners. |
| Agent/provider execution | Native Agent coordinator and Remote provider adapter | No Buzz ACP, agent or MCP executable is in the workspace or deployment graph. |
| Retired Buzz relay | Legacy 3000 public, 8080 health, 9102 metrics and optional Unix-socket surfaces | No root deployment or package reference remains. Live removal is still a Task 47.1/operator gate, so this audit does not claim that an unknown deployment has stopped it. |

Production routing therefore has one destination per mutation path in the proposed topology. The retained adapters translate a versioned protocol or deliver already-authorized work; none can choose an alternate database writer. Unknown legacy routes fail closed instead of falling back to Buzz.

## Canonical aggregate and writer matrix

An aggregate is listed under the component that validates its transition and the repository or resource that performs its authoritative write. Projection, transport, import and UI code may consume these records but cannot become a second owner.

| Aggregate family | Sole canonical state/transition owner | Sole durable writer or resource owner | Retired/adapter disposition |
| --- | --- | --- | --- |
| Account binding, principal and identity custody | `collaboration_domain` identity/account-binding rules plus the existing Zed credential owner | Collab identity-binding repository; native credentials remain in the platform credential store | Buzz identity state is importer input only; compatibility identities never bypass current binding or credential policy. |
| Community, membership, role, invitation and join policy | `collaboration_domain` community/membership/authorization aggregates | Canonical Collab tenant/domain command repositories and projections | Buzz desktop/relay handlers retire; Nostr and CLI adapters call the same commands. |
| Channel, message, reaction, thread, DM, marker, read state, reminder and scheduled message | `collaboration_domain` communication aggregates | Collab channel/message/event repositories and their outbox transaction | Desktop and relay routes are clients/adapters only; Redis fan-out is not authority. |
| Signed event and replaceable head | Canonical Nostr verification and Collab event-ingest policy | Collab `EventRepository` and event/outbox transaction | `nostr_compat` is a codec/verification boundary, not a second store. Buzz event tables remain frozen rollback evidence. |
| Projection, search and subscription cursor | Collab rebuild/index/query and subscription policy | Canonical projection/search/outbox tables and cursor writes | Redis and search indexes are derived. They cannot authorize or repair source records. |
| Project, repository, branch, review and CI status | Existing native Zed project/worktree/Git owners for local resources; collaboration-domain project/Git/review rules for hosted state | Canonical Collab project/Git/review repositories; repository bytes stay in the configured Git resource | NIP-34 and CLI surfaces are compatibility adapters. Buzz project/Git handlers do not remain writers. |
| Media object and metadata | Collaboration media admission, validation and retention policy | Collab media metadata plus the configured canonical object store | Blossom and client adapters never become storage authority. |
| Push notification and wake | Canonical notification policy and Collab push admission | Collab `PushOutboxRepository`; push gateway owns only fenced delivery attempts | Buzz push tables/workers retire after drain. Provider delivery cannot synthesize a wake. |
| Agent configuration, persona/team import and private local state | Native Agent/settings/credential owners plus canonical collaboration-domain configuration rules | Existing Zed stores and verified import receipts | Buzz snapshots are staged/imported once; raw secrets and process-local sessions are not copied into a second store. |
| Collaboration job, delegation and usage | `collaboration_domain::job` and Agent `JobExecutionCoordinator` | Collab `JobRepository` and canonical audit/usage chain | Remote providers are execution adapters under one lease; Buzz queues and executors retire. |
| Workflow definition, trigger, run, step, retry, approval and audit | Canonical Collab workflow trigger/evaluator/action/approval modules | Collab `WorkflowRepository`, fenced run leases and audit/outbox transaction | Webhook/Nostr/CLI surfaces only submit to this owner. Buzz workflow engine and direct writers retire. |
| Workflow ready-queue admission | Collab `WorkflowScheduler` | `WorkflowRepository` plus `collaboration_workflow_scheduler_admission` counters in the same PostgreSQL transaction | OL-EXE-04 is enforced once: 1,000 pending/community, 10,000/deployment, 16 running/community and 4 running/definition. No adapter maintains a parallel queue or concurrency counter. |
| Moderation, feedback, archive, retention and community deletion | `collaboration_domain` moderation/feedback/retention/deletion policy and Collab administrative executors | Canonical moderation/audit/archive/checkpoint repositories | Buzz administration and sweeper code is frozen. Import and rollback reads do not grant mutation authority. |
| Presence, typing, huddle, pairing and relay-mesh state | Canonical ephemeral admission/generation owners at their documented boundaries | No durable authority for presence/typing/relay frames; durable huddle/account effects use their canonical repositories | Pair relay, media transport and mesh peers are bounded transports/adapters, not aggregate owners. |
| Desktop settings, drafts, workspace layout and platform resources | Existing Zed settings, workspace, project, worktree, terminal and platform owners | Native Zed stores/filesystem/process owners | Buzz desktop data is read-only migration input until its receipt and deletion gates pass. |
| Migration checkpoint, cutover authority and rollback evidence | Canonical migration/cutover state machines | Collab migration checkpoint/cutover repositories; DDL only through the privileged migration runner | Buzz schemas are never written by canonical runtime code and are not down-migrated into canonical data. |

The matrix has no dual-write row. A compatibility adapter, background worker or separately deployed transport is retained only where its input is already authorized and its output is fenced by the canonical record.

## Schema and state-writer result

`deploy/collaboration/migrations/manifest.json` declares 21 ordered, checksummed canonical migrations through `20260825000100_collaboration_workflow_scheduler_admission`. They cover identity bindings; signed events and heads; projections, outbox and search; migration checkpoints; channels/messages; push; projects/Git/reviews; jobs; workflows, run leases, approvals and scheduler admission; audit; and moderation. Runtime repositories write only these canonical families under tenant policy. Schema changes belong exclusively to the separately privileged migration runner.

The 30 Buzz migrations remain an external, read-only migration and rollback baseline. No canonical manifest names an old Buzz table, no root package links `buzz-db`, and no production source path reads the external Buzz checkout. Importers write canonical rows and receipts; shadow readers compare results; the cutover permit prevents a compatibility route from selecting both old and new writers. Redis topics, presence, typing, connection controls and relay frames are transient and cannot advance a durable aggregate.

## Retired-source dependency classification

A repository search finds no `projects/buzz` path in Cargo manifests or `Cargo.lock`. The remaining code/script references are all intentional and non-production:

| Reference | Classification | Why it is not an unintended dependency |
| --- | --- | --- |
| `crates/cli/src/collaboration/contracts.rs` | Test-only source-drift check | Reads the approved Buzz CLI command source only inside the module's test configuration. |
| `crates/nostr_compat/src/buzz_nips/project_workflow.rs` | Unit-test fixture include | Includes the frozen NIP-MP corpus only under `#[cfg(test)]`. |
| `crates/nostr_compat/tests/buzz_nips.rs` | Integration-test oracle include | Deliberately checks canonical codecs against the approved reference documents. |
| `crates/agent_ui/tests/collaborative_activity.rs` | Provenance-string assertion | Checks that a checked-in protocol manifest records its source path; it does not read that path. |
| `script/multiplayer-build-profile` | Feature-boundary classifier | Treats any `projects/buzz` path as multiplayer-only if one is supplied; it does not build or launch Buzz. |

These checks are explicit preservation/conformance consumers, not release inputs or fallback implementations. They remain valid while the supplied source is the approved reference baseline. Task 47.5's immutable-history HOLD must be resolved before source deletion; at that gate these test fixtures must be repointed to the approved durable locator or copied artifact rather than silently removed.

## Result and remaining gates

Repository validation result: **one canonical owner per aggregate family and zero unintended retired-source dependencies**.

The result is conditional only in the operational sense: repository inspection cannot enumerate a target deployment's pods, processes, sockets, routes, scrape targets, credentials or database sessions. Before any retirement action, operators must bind this document to the exact Task 47.1 checkpoint, prove zero direct legacy writes and approved usage windows, inspect the live deployment, preserve an approved immutable Buzz source-history locator and obtain the separate source/deployment/data approvals. Until then all retirement manifests remain **HOLD**, and `projects/buzz` and legacy data remain unchanged.

## Validation commands

```text
cargo metadata --no-deps --format-version 1
cargo metadata --manifest-path tools/buzz_compat/Cargo.toml --no-deps --format-version 1
rg -n 'projects/buzz|buzz-(relay|db|pubsub|acp|agent|dev-mcp)' Cargo.toml Cargo.lock crates/**/Cargo.toml services/**/Cargo.toml tools/**/Cargo.toml
rg -n 'projects/buzz' crates services tools script Cargo.toml Cargo.lock --glob '*.rs' --glob '*.toml' --glob '*.sh' --glob 'multiplayer-build-profile'
rg -n 'BUZZ_|buzz-relay|buzz-db|buzz-pubsub|buzz-acp|buzz-agent|buzz-dev-mcp|9102' deploy/collaboration
jq -r '.migrations[] | [.version,.name,.up.file] | @tsv' deploy/collaboration/migrations/manifest.json
python3 .agents/skills/feature-spec/scripts/validate_spec.py .agents/specs/collaborative-workspace
./script/check-collaborative-workspace-inventory
./script/check-collaboration-dependencies
git diff --check
```
