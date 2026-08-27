# Relay mesh and shared-compute threat model

## Scope and authority

This review covers authenticated relay-mesh routing and explicitly opted-in shared inference from peer discovery through admission, scheduling, execution, result acceptance, failure, rollback and cleanup. It implements acceptance criteria 16.3, 19.1 and 19.2 for CAP-035 under accepted ADR-006.

The canonical ownership split is mandatory:

- `collaboration_domain` owns community identity, membership version, provider-neutral job identity, requested capability, transferable-context policy, authorization inputs and execution-location policy.
- `agent` and ACP own the one canonical session/job, permission envelope, cancellation, activity and result lifecycle. Mesh selection never creates a second job or transcript.
- `remote::mesh` owns versioned peer protocol, attested relay membership, compute advertisements, eligibility, fair scheduling and expiring fenced executor/resource leases.
- The canonical session directory owns relay-session generation. Gossip and liveness are routing hints and cannot elect an owner.
- The admitted serving runtime owns locally enforced model, resource and sandbox limits. Its advertisement is an untrusted claim and cannot widen local policy.
- `collab_ui` displays explicit provider trust, execution location, disclosure, queue and failure state. It cannot grant eligibility by rendering a peer.

Shared compute is disabled independently at deployment, community, user and device levels. The initial trust boundary is self-hosted: same deployment or an explicitly sharing active member of the same community. Third-party, anonymous, marketplace and cross-community providers are prohibited until ADR-006's future approval gate is accepted. Task 4.4 owns numeric operational budgets; Tasks 41.5 and 45.5 own measured partition, fairness, resource and load evidence.

## Source evidence and preserved constraints

- `projects/buzz/crates/buzz-relay-mesh/src/wire.rs` freezes ALPN `buzz/mesh/1`, a one-byte wire version, a 16 MiB stream-frame ceiling, first-frame `Hello`, authenticated boot-scoped runtime IDs and a `{session_id, generation, owner_runtime_id}` fence on every session-bearing frame.
- Relay ready records expire after three refresh intervals and bind a boot runtime key to the expected relay signing identity. Unanchored or foreign relay records are rejected. Gossip uses monotonic record versions and phi suspicion only as dial/liveness hints.
- The relay runtime authenticates Iroh endpoint identity, rejects unknown inbound peers, deterministically resolves simultaneous dials, separates reliable/control streams from lossy media datagrams, and exposes bounded counters. The current Buzz mesh protocol does not itself carry a trusted community field, so Zed must bind every connection and frame to deployment and tenant context before compatibility decoding can route it.
- Buzz desktop shared compute intersects current relay membership with fresh signed member-status events, verifies owner/member/endpoint bindings, rejects missing membership snapshots, removes revoked members despite fresh status, and excludes stale status from routing.
- Desktop transport policy bounds invite tokens and transport candidates, restricts relay URLs, rejects unsafe direct targets and verifies signed bootstrap tokens. Local owner keys and endpoint identities are separate trust artifacts.
- Buzz's local mesh runtime supports serve and consume modes, model discovery, progress, health, stop/recovery and usage reporting. It does not provide the canonical resource lease, weighted fairness, job fencing or no-silent-fallback guarantees required by ADR-006; these are Zed strengthening obligations, not optional parity.
- Buzz's Kubernetes backend deliberately refuses shared-compute agents because the in-image mesh client is absent. Zed deployment must retain that fail-closed behavior until a declared execution substrate passes readiness and conformance.
- `VISION_MESH.md` accurately identifies prompt disclosure to another member's hardware and opt-in capacity, but its statement that membership is the only gate is superseded by ADR-006: membership is necessary and never sufficient consent or execution authority.

## Security invariants

1. **MS-01 — typed tenant boundary.** Every peer, advertisement, lease, job, frame and result is bound to one trusted deployment/federation and community before lookup; wire tags or gossip never select authority.
2. **MS-02 — attested boot identity.** Peer runtime keys are boot/session scoped and accepted only under the configured deployment or member trust root, supported protocol profile and current revocation state.
3. **MS-03 — hints cannot own.** Gossip, reachability, load, phi suspicion and advertisements can remove candidates but cannot grant membership, session ownership, capacity or execution.
4. **MS-04 — monotonic replay fences.** Versions, expiries, nonces and canonical session/executor generations reject stale, replayed, duplicated and wrong-owner traffic at every send, receive and completion seam.
5. **MS-05 — explicit bilateral consent.** Deployment/community policy, requester authorization and device-owner sharing consent are current at selection and execution admission; relay-mesh enablement never enables shared compute.
6. **MS-06 — one canonical job.** Local, remote and mesh attempts share one Zed job/session authority. Exactly one current executor lease may mutate it, and stale or duplicate output cannot complete it.
7. **MS-07 — local bounds are authoritative.** Signed resource claims are capped by configured policy and verified locally before and during execution; exceeding a bound cancels and cleans up rather than overcommitting.
8. **MS-08 — inference is not code execution.** Shared compute receives only explicitly classified model context and has no implicit filesystem, credential, keyring, shell, tool, environment or unrestricted network authority.
9. **MS-09 — community-local fairness.** Eligibility and weighted fair queuing operate inside one community with bounded requester/device queues, concurrency, aging and owner-reserved capacity; other tenants neither influence nor observe them.
10. **MS-10 — no silent fallback or replay.** Mesh unavailability, denial, partition and uncertain execution remain visible. Zed never changes provider, trust class, owner or attempt without an explicit authorized decision.
11. **MS-11 — content-minimized control plane.** Prompts, outputs, secrets and private memory never enter gossip, discovery, status, logs, traces or metric labels; serving-node retention is disabled beyond active bounded buffers by default.
12. **MS-12 — reversible, fail-closed operation.** Missing authority, store, identity, version, readiness, kill switch or cleanup evidence blocks admission. Disablement stops new leases and visibly drains/cancels existing work without rerouting it.

## Threat ledger

| Threat ID | Threat and observable failure | Fail-closed control and canonical owner | Required negative or recovery tests |
| --- | --- | --- | --- |
| T-MESH-001 | Wire tenant or community tag routes into another tenant | Trusted `TenantContext` precedes compatibility decode; `collaboration_domain` | Tasks 13.3, 41.1, 45.2 |
| T-MESH-002 | Valid peer key from another deployment joins the relay mesh | Expected deployment trust root and typed federation allowlist; `remote::mesh` | Tasks 41.1, 45.2, 45.5 |
| T-MESH-003 | Long-lived deployment key collapses boot identity/fencing | Fresh runtime key plus signed binding and boot/session expiry | Tasks 41.1, 44.3, 45.2 |
| T-MESH-004 | Missing trust anchor accepts any self-signed ready record | Unconfigured identity makes listener/readiness unavailable | Tasks 41.1, 44.3, 44.4 |
| T-MESH-005 | Unknown ALPN/version is guessed or partially decoded | Closed version negotiation before allocation/state mutation | Tasks 41.1, 47.1, 48.2 |
| T-MESH-006 | Oversized stream, datagram, invite or address list exhausts resources | Pre-allocation byte/count/depth ceilings and transport MTU check | Tasks 41.1, 41.5, 45.4 |
| T-MESH-007 | Non-`Hello` first frame or role confusion reaches a handler | First-frame/role/profile state machine rejects and resets stream | Tasks 41.1, 41.5 |
| T-MESH-008 | Replayed gossip/status record resurrects stale endpoint | Monotonic version, signed timestamp, expiry and tombstone/revocation floor | Tasks 41.1, 41.2, 46.3 |
| T-MESH-009 | Membership loss races fresh compute status | Current canonical membership version rechecked at lease admission | Tasks 41.2, 41.3, 45.2 |
| T-MESH-010 | Phi suspicion or partition elects a second session owner | Canonical directory CAS generation alone grants ownership | Tasks 41.1, 41.5, 45.3 |
| T-MESH-011 | Stale relay forwards session/media after ownership changes | Fenced header checked at sender, receiver and profile handler | Tasks 41.1, 41.5, 45.3 |
| T-MESH-012 | Simultaneous dials create duplicate peer streams/work | Deterministic connection winner and idempotent install/cleanup | Tasks 41.1, 41.5 |
| T-MESH-013 | Signed owner status substitutes another member endpoint | Owner/member/endpoint transcript plus current member equality | Tasks 41.1, 41.2, 45.2 |
| T-MESH-014 | Unsafe direct address or arbitrary relay enables SSRF/routing bypass | Locally configured relay allowlist and safe direct-address policy | Tasks 41.1, 44.3, 45.2 |
| T-MESH-015 | Advertisement asserts uninstalled model or false capacity | Treat claim as hint; verify artifact/digest and local capacity on admission | Tasks 41.2, 41.3, 41.5 |
| T-MESH-016 | Advertisement widens model/context/resource policy | Clamp/reject against deployment, community, user and device ceilings | Tasks 41.2, 44.3, 45.2 |
| T-MESH-017 | Duplicate devices/endpoints inflate available capacity | Stable owner/device identity and current-advertisement uniqueness | Tasks 41.2, 41.5 |
| T-MESH-018 | Stale, draining or quarantined node remains selectable | Bounded expiry plus live eligibility recheck before lease grant | Tasks 41.2, 41.3, 41.4 |
| T-MESH-019 | Relay mesh being enabled silently exposes device compute | Independent default-off deployment/community/user/device gates | Tasks 41.2, 44.3, 48.4 |
| T-MESH-020 | Community membership alone authorizes prompt or hardware use | Explicit provider and consumer consent plus delegation policy | Tasks 33.2, 41.3, 41.4 |
| T-MESH-021 | Agent delegation permits remote inference beyond owner intent | Canonical permission/context policy checked at selection and admission | Tasks 32.4, 33.2, 41.3 |
| T-MESH-022 | Scheduler uses cross-community capacity or leaks its load | Community-leading candidate/query/metric keys and access control | Tasks 41.3, 41.5, 45.4 |
| T-MESH-023 | Stale ad causes scheduler overcommit | Atomic locally enforced resource/executor lease, not advertised count | Tasks 41.2, 41.3, 41.5 |
| T-MESH-024 | High-volume requester starves peers | Bounded weighted fair queue, requester caps, aging and reservation tests | Tasks 41.3, 41.5, 45.4 |
| T-MESH-025 | Admin weight or reservation becomes hidden unbounded priority | Role-gated, versioned, bounded policy with audit attribution | Tasks 36.3, 41.3, 45.2 |
| T-MESH-026 | One job obtains multiple concurrent executors | Canonical job-version CAS plus one fenced executor/resource lease | Tasks 33.5, 41.3, 46.3 |
| T-MESH-027 | Stale/wrong node result completes current job | Result carries job, attempt and executor generation; canonical compare-and-complete | Tasks 33.5, 41.3, 41.5 |
| T-MESH-028 | Timeout with unknown outcome is invisibly replayed elsewhere | Mark unknown/failed and require authorized linked retry after disclosure | Tasks 33.6, 41.3, 41.4 |
| T-MESH-029 | Mesh failure silently falls back to local/cloud provider | Provider selection is immutable per attempt; typed visible failure | Tasks 33.4, 41.3, 41.4 |
| T-MESH-030 | Remote inference receives credentials, tools or private memory | Explicit transferable-context classifier and secret-free provider envelope | Tasks 4.3, 33.2, 41.3 |
| T-MESH-031 | Serving runtime gains host filesystem, shell or network authority | Model-only sandbox/process identity with denied-by-default capabilities | Tasks 33.2, 41.3, 45.2 |
| T-MESH-032 | Runtime exceeds memory/GPU/disk/network/time/token bounds | Locally metered lease cancels, marks partial output incomplete and cleans resources | Tasks 41.3, 41.5, 45.4 |
| T-MESH-033 | Prompt/output leaks through discovery, logs or metrics | Content-free control records and closed bounded observability labels | Tasks 41.2, 44.5, 45.2 |
| T-MESH-034 | Opt-out, revocation or kill switch leaves active listeners/leases | Stop admissions, withdraw ads, close listeners, drain/cancel and reconcile | Tasks 41.3, 44.3, 45.3 |
| T-MESH-035 | Partition recovery trusts cached membership/job state | Re-read canonical membership and job/lease generations before work/result acceptance | Tasks 41.5, 45.3, 46.3 |
| T-MESH-036 | Unsupported deployment or third-party provider becomes usable | Startup/readiness and provider-type gates reject; no generic adapter fallback | Tasks 41.3, 44.3, 47.1, 48.4 |

## Boundary checklist

### MESH-B01 — deployment, tenant and peer admission

- **Owner:** Zed configuration/identity plus `remote::mesh` handshake.
- **Order:** enabled profile → typed deployment/federation and community → bounded ALPN/version → endpoint proof → trust-root attestation → current revocation/readiness → peer state.
- **Failure:** absent or ambiguous identity, wildcard trust, foreign peer or unsupported version closes the connection without resource-existence detail.
- **Tests:** Tasks 41.1, 44.3, 45.2 and 48.2.

### MESH-B02 — relay membership and session fencing

- **Owner:** canonical session directory for ownership; `remote::mesh` for hints and transport.
- **Rule:** ready/gossip state affects dialing only. Every session-bearing send, receive and profile dispatch compares the current tenant-scoped generation and owner runtime.
- **Failure:** store unavailability, stale/future ambiguity, wrong owner or partition cannot trigger takeover.
- **Tests:** Tasks 41.1, 41.5 and 45.3.

### MESH-B03 — bounded protocol and transport

- **Owner:** `remote::mesh::protocol` and approved Iroh/direct adapter.
- **Rule:** closed message types, first-frame `Hello`, role/profile separation, maximum encoded sizes, address-count limits, approved relay URLs and safe direct targets. Reliable state never rides a lossy datagram.
- **Failure:** malformed input is reset/rejected with closed counters and no partial domain mutation.
- **Tests:** Tasks 41.1, 41.5 and 45.4.

### MESH-B04 — compute identity, consent and advertisement

- **Owner:** canonical member/device policy plus `remote::mesh::advertisement`.
- **Rule:** a signed owner/member/device/endpoint binding and fresh advertisement are hints only after independent deployment, community, user and device opt-in. Model identities are immutable digests.
- **Failure:** missing membership snapshot, revoked member, stale status, duplicate endpoint or unverifiable model is unavailable, not degraded to an anonymous candidate.
- **Tests:** Tasks 41.2, 41.4, 45.2 and 48.4.

### MESH-B05 — eligibility and fair scheduling

- **Owner:** `remote::mesh::scheduler` using canonical job, tenant, delegation, trust and resource policy.
- **Order:** current consent/membership → trust class/location → model/capability/context privacy → node freshness/revocation → local capacity → community-local weighted fairness → one lease CAS.
- **Failure:** returns a typed policy-denied, queued or no-capacity outcome. No alternate provider is selected.
- **Tests:** Tasks 41.3–41.5 and 45.4.

### MESH-B06 — executor and resource lease

- **Owner:** canonical Zed job repository plus remote scheduler and serving runtime.
- **Fields:** community, job/attempt, executor generation, node/owner, model digest, resource ceilings, expiry, nonce and cancellation grace.
- **Rule:** only one current fenced lease can start or complete; serving enforcement may narrow but never widen it.
- **Tests:** Tasks 33.5, 41.3, 41.5 and 46.3.

### MESH-B07 — model inference sandbox and context release

- **Owner:** canonical agent permission/context policy and approved shared-compute runtime.
- **Rule:** after executor admission, release only selected bounded prompt/context. No credentials, environment, paths, tools, shell, private memory or general egress are implied.
- **Failure:** unknown privacy class, missing sandbox capability or model digest mismatch cancels before content disclosure.
- **Tests:** Tasks 4.3, 33.2, 41.3 and 45.2.

### MESH-B08 — output, retry and no-silent-fallback

- **Owner:** canonical agent job/session state.
- **Rule:** output is hostile, bounded and attributed to exact node/trust/location/attempt. Partial or stale output cannot be success. Cross-owner retry after possible disclosure requires an explicit policy/user action and a linked attempt.
- **Failure:** unavailable, denied, lost, cancelled, unknown and failed remain distinct visible terminal/recoverable states; provider defaults never mutate.
- **Tests:** Tasks 33.4, 33.6, 41.3, 41.4 and 45.3.

### MESH-B09 — revocation, partition and cleanup

- **Owner:** membership/job authorities, scheduler and serving runtime.
- **Rule:** policy change, membership loss, opt-out, quarantine, lease expiry, cancellation, node loss and kill switch reject new work and idempotently release queue, model, network and executor resources.
- **Recovery:** re-read canonical membership/job/lease state before reconnect, admission or accepting output; cached gossip can only suppress work.
- **Tests:** Tasks 41.5, 44.3, 45.3 and 46.3.

### MESH-B10 — visibility, observability and deployment compatibility

- **Owner:** `collab_ui` for user state and Zed deployment/operations for readiness/metrics.
- **Rule:** UI names execution location/trust, disclosure and sharing state; operators see bounded version, rejection, capacity, fairness, queue, cancellation, partition and cleanup signals with tenant access controls. Content is excluded.
- **Deployment:** listeners and serving ship disabled; unsupported clients/substrates remain unavailable. Rollback never reroutes a job, and Buzz compatibility cannot execute the same canonical attempt concurrently.
- **Tests:** Tasks 41.4, 44.3–44.5, 45.5, 47.1 and 48.4.

## Known gaps and strengthening obligations

1. Buzz relay mesh authenticates peers to one relay signing identity but its frozen frame does not carry a community. Task 41.1 must bind the compatibility connection/frame to trusted deployment and tenant context before routing; adding an untrusted tenant field alone is insufficient.
2. Buzz desktop status events offer useful membership/endpoint checks but do not provide an atomic canonical executor/resource lease. Tasks 41.2 and 41.3 must preserve status compatibility as discovery while moving execution authority into Zed.
3. Buzz capacity fields and usage status are self-reported. ADR-006 requires local admission enforcement, immutable model digests and bounded resource leases; Task 41.5 must demonstrate that false and stale claims cannot overcommit.
4. Buzz's community-member-only vision is weaker than approved bilateral consent. The migration must not infer sharing consent from existing membership or start a listener during import.
5. Buzz's local mesh SDK can restart/rearm runtimes and currently allows long management waits. Task 4.4 must assign bounded startup, stop, heartbeat, status, lease, cancellation and recovery budgets with owners and alerts.
6. Shared compute is not deployable through the current Buzz Kubernetes backend. This is a preserved fail-closed compatibility fact, not permission for a generic remote-provider fallback; Tasks 44.3 and 48.4 own explicit substrate readiness.
7. Distributed/sharded model execution described by `VISION_MESH.md` has no accepted implementation authority. It may be supported only if one canonical lease identifies every node and aggregate limits before prompt release; otherwise it remains unavailable rather than partially emulated.
8. Existing status events, identities, endpoint tokens and local model state require versioned import/shadow comparison. Imported state grants no consent or active lease, and Buzz/Zed may not execute one job concurrently during Tasks 46.1–46.6.

## Cross-cutting verification checklist

- **Authentication and replay:** Tasks 41.1, 41.5 and 45.2 cover foreign/unanchored peers, wrong endpoint binding, unknown versions, stale/future generations, replayed gossip/status and malformed bounded frames.
- **Revocation and partition:** Tasks 41.2, 41.3, 41.5 and 45.3 cover membership loss, owner opt-out, draining, quarantine, kill switch, partition, stale result and canonical-state reread.
- **Resource and isolation:** Tasks 33.2, 41.2, 41.3, 41.5 and 45.4 cover false claims, immutable models, local CPU/GPU/memory/disk/network/time/token bounds, denied capabilities and terminal cleanup.
- **Fairness:** Tasks 41.3, 41.5 and 45.4 cover per-requester concurrency/queue caps, equal default weights, bounded administrative weights, owner reservation, aging and cross-tenant noninterference.
- **No fallback:** Tasks 33.4, 33.6, 41.3, 41.4 and 45.3 cover no capacity, denial, transport loss, unknown outcome, cross-owner retry disclosure and immutable provider choice.
- **Privacy and consent:** Tasks 4.3, 41.2–41.4, 44.5 and 45.2 cover bilateral opt-in, transferable context, prompt/output exclusion from control/telemetry and visible trust/location.
- **Deployment and compatibility:** Tasks 44.3–44.5, 46.1–46.6, 47.1, 48.2 and 48.4 cover disabled defaults, readiness, kill switch, shadowing, rollback, version negotiation and no concurrent duplicate execution.

Task 4.4 must consume every numeric limit named here. Production activation remains outside this threat-model leaf and requires the security, migration, partition, load and operational evidence assigned above.
