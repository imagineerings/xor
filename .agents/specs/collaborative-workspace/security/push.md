# Push delivery threat model

## Scope and authority

This review covers the security and reliability boundaries from an accepted collaboration event through NIP-PL lease matching, durable wake creation, installation and relay authority, last-hop provider delivery, and client reconnect. It implements acceptance criteria 9.5, 19.1 and 19.2 for CAP-016 without changing accepted ADR-005.

The canonical ownership split is mandatory:

- `collaboration_domain` owns provider-neutral notification eligibility, lease identity, generation, expiration, revocation, endpoint-generation references and sanitized wake outcomes.
- `collab` owns trusted community/origin binding, current read authorization, accepted lease state, event-to-wake matching and the durable idempotent wake outbox. A lease is never a read grant.
- `nostr_compat` owns the byte-exact kind `30350` event, NIP-44 content and NIP-PL descriptor codecs, but never selects a tenant, authorizes a match or opens platform credentials.
- The Sim-owned push gateway owns App Attest installation authority, assertion counters, encrypted APNs-token custody, relay-scoped opaque grants, NIP-98 admission, delivery replay/quota reservations, provider credentials and response sanitization.
- The mobile client owns notification permission, platform-token acquisition, installation assertions, lease publication and authenticated fetch after a fixed wake. APNs acceptance is not message delivery or read state.
- APNs production and sandbox validation are the only approved first-cutover profiles. FCM and UnifiedPush remain unavailable until the eight-part ADR-005 approval gate is satisfied; an adapter cannot silently substitute a webhook, rich payload or another transport.

The platform provider observes endpoint, timing, frequency and its own routing metadata. The design minimizes but cannot eliminate that traffic-analysis channel. Task 4.4 owns numeric alert budgets and Task 22.12 owns measured load/failure evidence.

## Source evidence and preserved constraints

- `projects/buzz/docs/nips/NIP-PL.md` makes the lease an installation-scoped, signed, expiring wake authorization. It requires a random per-origin `d`, author-only reads, exact origin agreement, bounded/allowlisted filters, current read authorization at match and send, and one fixed provider-authored reconnect body.
- Buzz persists effective leases and wake jobs under community-leading keys. Endpoint uniqueness, generation replacement, claim leases and `(community, endpoint_hash, event_id)` dedup are database constraints, not cache conventions.
- Buzz's event trigger creates match work in the accepted-event transaction and uses a per-community advisory-lock protocol when skipping lease-less communities. This covers internal producers that bypass live dispatch and closes activation-versus-event lost-wake ordering.
- The relay matcher batches at most 64 accepted events, uses community-scoped lease and membership scans, limits gift-wrap matching to the lease author and inserts wakes set-wise. Delivery claims 16 wakes per community, rechecks active generation and membership, and holds the community deletion serving fence across the provider request.
- Buzz freezes a 30-second claim, one-hour event usefulness ceiling, eight delivery attempts, 250 ms–2 s idle backoff and bounded exponential retry. These are compatibility evidence; Task 4.4 must accept or replace every numeric target with an owner, metric and alert.
- The public gateway accepts closed JSON bodies of at most 8192 bytes behind a concurrency ceiling of 256 and a 20-second request timeout. Its APNs client has a 15-second timeout, one expired-provider-token refresh, and clamps `Retry-After` to 1–3600 seconds.
- App Attest inputs are bounded, enrollment and mutations use exact domain-separated transcripts, challenges expire after 300 seconds, and assertion counters increase atomically. Apple does not attest the APNs token-to-key binding; enrollment token provenance remains an explicit bootstrap assumption.
- APNs tokens are encrypted with a keyring distinct from opaque grant sealing. The grant binds delegation, relay pubkey, app profile, endpoint epoch, generation and expiry, while the relay never receives the raw token.
- Delivery admission atomically checks current installation/delegation authority, quota, NIP-98 event replay and stable request-ID replay. Terminal results retain the request reservation; retryable/configuration results release only the request reservation while burning the auth event.
- Gateway metrics use only closed static labels on a private health router. They do not include token, endpoint, relay pubkey, request ID, event ID, installation handle or grant.

## Security invariants

1. **PD-01 — wake-only noninterference.** Every provider application body equals the byte-pinned profile constant and is independent of event, lease, community, sender, channel, count, ciphertext, request and provider response.
2. **PD-02 — trusted origin first.** The authenticated connection/service route supplies community and canonical origin. Encrypted lease origin can only agree byte-for-byte; it cannot route or select authority.
3. **PD-03 — lease is not access.** Creation authentication does not confer future visibility. Current tenant, author, membership, event visibility, active generation, expiration and endpoint state are checked at match and immediately before send.
4. **PD-04 — bounded non-amplifying match.** Active subscriptions require allowlisted kinds and exact narrowing selectors, have bounded counts/strings/filters, never time-travel/search, and cannot supply callback URLs.
5. **PD-05 — one atomic effective lease.** Addressable ordering and strictly increasing generation both win before signed event, effective lease and watermark change. Any rejection leaves all three unchanged.
6. **PD-06 — endpoint authority is opaque and scoped.** Relays persist only an opaque capability/hash. Gateway custody binds one attested installation, profile, endpoint epoch, relay, generation and expiry; no field may be broadened at delivery.
7. **PD-07 — durable, idempotent wake work.** Accepted persistent events create match responsibility transactionally; wakes deduplicate by trusted community, endpoint and source event; claims recover or terminate within fixed attempt/expiry bounds.
8. **PD-08 — replay reservations survive replicas.** Challenge, assertion counter, NIP-98 event, request ID, quota and generation checks use atomic shared persistence. Store unavailability fails readiness/admission closed.
9. **PD-09 — revocation wins prospectively.** Higher-generation lease tombstones, installation/delegation revocation, endpoint rotation, membership loss and community deletion suppress unbegun sends. A send-begin already committed may finish and is documented as the race boundary.
10. **PD-10 — provider faults do not revoke identity.** Permanent endpoint errors disable only the matching endpoint generation. Transient, credential, topic and provider faults retry or surface configuration failure without revoking users, siblings or leases.
11. **PD-11 — secrets and metadata stay bounded.** Raw tokens, grants, attestations, auth events and identifiers never enter public errors, application payloads, logs, traces or metric labels. Public failures use the closed error alphabet.
12. **PD-12 — no unapproved provider fallback.** Unsupported, unavailable or misconfigured push is visible and falls back only to foreground/reconnect/manual authenticated sync; it never changes payload, attestation or endpoint trust policy.

## Threat ledger

| Threat ID | Threat and observable failure | Fail-closed control and canonical owner | Required negative or recovery tests |
| --- | --- | --- | --- |
| T-PUSH-001 | Lease `origin`, public tag or endpoint selects another community | Row-zero community plus byte-exact origin agreement; `collab` admission | Tasks 11.11, 22.6, 22.11, 45.2 |
| T-PUSH-002 | Same installation identifier links origins | At least 128 random bits and distinct `d` per origin; client plus domain validation | Tasks 22.6, 43.7, 22.11 |
| T-PUSH-003 | Unauthenticated user creates or reads another author's lease | NIP-42 pubkey equality and author-only EVENT/REQ/COUNT policy | Tasks 14.2, 14.3, 14.4, 22.11 |
| T-PUSH-004 | Duplicate/unknown tags, fields or JSON keys smuggle endpoint/filter data | Closed bounded schema before persistence; `nostr_compat` and gateway codecs | Tasks 11.11, 22.6, 22.9 |
| T-PUSH-005 | Oversized ciphertext/plaintext/filter/endpoint exhausts memory or parser work | Descriptor and HTTP byte/depth/count/string ceilings before allocation | Tasks 11.11, 22.6, 22.9, 22.12 |
| T-PUSH-006 | Kinds-only, prefix, broad author/tag or time-travel filter becomes a firehose | Exact selectors, narrowing requirement, allowlisted kinds and no ids/time/search | Tasks 11.11, 22.6, 22.11, 22.12 |
| T-PUSH-007 | `#p` watches another user or gift-wrap timing reveals another recipient | `#p` equals author; outer gift-wrap recipient/self filter checked without decrypting | Tasks 22.6, 22.11, 45.2 |
| T-PUSH-008 | Urgent class amplifies arbitrary traffic | Descriptor class support and public-envelope urgent-kind allowlist; no downgrade | Tasks 22.4, 22.6, 22.11 |
| T-PUSH-009 | Invalid replacement poisons watermark or removes valid state | Atomic dual ordering plus generation transaction; rejection is state-neutral | Tasks 22.7, 22.13, 45.3 |
| T-PUSH-010 | Replay resurrects expired/revoked lease after cleanup | Tombstone/watermark retention through maximum replay window | Tasks 22.6, 22.7, 22.13, 37.8 |
| T-PUSH-011 | Endpoint rotation or permanent provider error disables sibling devices | Address-scoped generation and endpoint epoch; conditional generation disable only | Tasks 22.8, 22.9, 43.7 |
| T-PUSH-012 | Lease acceptance races event insert and silently loses a wake | Accepted-event transaction plus advisory-lock eligibility protocol or equivalent | Tasks 22.7, 22.13, 45.3 |
| T-PUSH-013 | Internal event producer bypasses matcher | One authoritative accepted-event/outbox seam covering every durable producer | Tasks 15.4, 22.13, 45.3 |
| T-PUSH-014 | Global matcher or dedup key crosses tenants | Community-leading leases, jobs, claims, hashes, cursors, quotas and source IDs | Tasks 22.7, 22.11, 22.13, 45.2 |
| T-PUSH-015 | Creation-time membership persists after removal | Current read and channel membership checks at match and pre-send | Tasks 22.4, 22.8, 22.11, 37.8 |
| T-PUSH-016 | Cache/projection outage becomes permissive authorization | Durable current-policy repository is authority; cache loss retries/suppresses | Tasks 22.8, 22.12, 37.8 |
| T-PUSH-017 | Duplicate matcher/worker execution causes provider amplification | Source/endpoint dedup, exclusive claims, stable request ID and replay reservation | Tasks 22.7, 22.8, 22.12, 22.13 |
| T-PUSH-018 | Poison event, lease or job retries forever | Bounded match/delivery attempts, expiry, poison reaper and terminal state | Tasks 22.8, 22.12, 22.13 |
| T-PUSH-019 | Queue growth starves tenants or exhausts storage | Per-community fairness, bounded claims/backoff/retention and alert budgets | Tasks 4.4, 22.10, 22.12, 45.4 |
| T-PUSH-020 | Relay sends event text, URL, count, ciphertext or arbitrary JSON to provider | Payload-less executor interface and provider-owned byte constant | Tasks 22.6, 22.8, 22.9, 22.11 |
| T-PUSH-021 | Attacker chooses arbitrary callback/UnifiedPush URL | Only descriptor-approved APNs profile; no client-selected network destination | Tasks 2.5, 22.6, 22.9, 45.2 |
| T-PUSH-022 | Enrollment challenge/assertion is replayed or retargeted | Single-use challenge, exact audience/domain transcript and monotonic counter | Tasks 22.9, 43.7, 45.2 |
| T-PUSH-023 | App Attest/profile/topic/environment mismatch gains endpoint authority | Pinned Apple root/app ID and profile-coherent startup/readiness; no bypass | Tasks 22.9, 22.10, 43.7, 45.2 |
| T-PUSH-024 | Submitted token is assumed Apple-bound to App Attest key | Documented bootstrap assumption plus uniqueness/rotation/revocation and abuse limits | Tasks 22.9, 22.12, 45.2 |
| T-PUSH-025 | Token, grant or sealing key leaks through relay, logs, errors or metrics | Separate encrypted custody/keyrings, opaque grant and closed redaction surfaces | Tasks 22.8, 22.9, 22.11, 44.5 |
| T-PUSH-026 | Old sealing/custody key removal invalidates live grants/tokens | Decrypt-only predecessors retained through maximum live/recovery window | Tasks 22.9, 22.10, 45.3 |
| T-PUSH-027 | Grant is used by wrong relay/profile/epoch/generation or after expiry | AEAD-bound fields plus live shared authority check at admission | Tasks 22.8, 22.9, 45.2 |
| T-PUSH-028 | Same NIP-98 auth or stable request ID sends twice across replicas | Atomic auth-event and request replay reservations with terminal/retry semantics | Tasks 22.8, 22.9, 22.12, 45.2 |
| T-PUSH-029 | Concurrent requests exceed endpoint quota | Serialized/upsert quota reservation in the same delivery-admission transaction | Tasks 22.8, 22.12, 45.4 |
| T-PUSH-030 | Provider timeout/retry duplicates delivery indefinitely | Best-effort semantics, stable ID, bounded jittered retry and explicit exhaustion | Tasks 22.8, 22.9, 22.12, 43.7 |
| T-PUSH-031 | APNs configuration/provider fault mass-disables endpoints | Closed response classifier separates invalid endpoint, retry and configuration fault | Tasks 22.8, 22.9, 22.10 |
| T-PUSH-032 | Revocation/deletion races an outbound request | Generation/membership recheck and community serving fence; documented send-begin linearization | Tasks 22.8, 37.8, 45.3 |
| T-PUSH-033 | Failure/cancellation strands sending jobs or replay fences | Recoverable claim lease, disposition bookkeeping and bounded reconciliation | Tasks 22.8, 22.12, 22.13, 45.3 |
| T-PUSH-034 | Push outage blocks foreground use or silently changes notification guarantees | Visible push unavailable state; authenticated sync/manual refresh remains canonical | Tasks 22.5, 43.7, 43.8 |
| T-PUSH-035 | Timing/frequency reveals private activity or enables wake spam | Eligibility, coalescing, per-endpoint/tenant quotas and coarse metrics/errors | Tasks 22.4, 22.11, 22.12, 45.4 |
| T-PUSH-036 | FCM/UnifiedPush is advertised without an approved fixed-body trust profile | ADR-005 configuration/type gate fails startup/descriptor negotiation | Tasks 22.9, 22.10, 43.7, 48.2 |

## Boundary checklist

### PUSH-B01 — notification eligibility

- **Owner:** `collaboration_domain::notification_policy` using canonical message, membership, mute, read and device-permission state.
- **Rule:** self-authored, muted, read, revoked, unauthorized, duplicate and policy-excluded events produce no native notification or wake. Stable source IDs drive dedup; preview eligibility does not enter the push job.
- **Failure:** unavailable current policy suppresses/retries rather than treating an event as eligible.
- **Tests:** Tasks 22.4, 22.11 and 45.2.

### PUSH-B02 — lease wire admission and effective state

- **Owner:** `nostr_compat::nip_pl` for exact wire semantics; `collab::push` for trusted origin, policy and transaction.
- **Order:** bounded signed event → author authentication/signature → closed public tags → executor-key decrypt → closed plaintext → origin agreement → filter/profile/quota policy → dual ordering/generation → one signed-event/effective-state/watermark commit.
- **Rule:** author-only query; rejected replacement changes nothing; inactive minimal schema never depends on an available endpoint profile.
- **Tests:** Tasks 11.11, 22.6, 22.7 and 22.13.

### PUSH-B03 — accepted event to durable wake

- **Owner:** the canonical `collab` command/outbox transaction and push matcher.
- **Rule:** every accepted push-eligible durable event creates match responsibility exactly once at the authority seam. Matcher reads only community-labeled leases/current membership and the public accepted envelope; it never decrypts event content.
- **Bounds:** batch, scan, lease/subscription, attempts, event usefulness and idle backoff are finite and measured under Tasks 4.4/22.12.
- **Tests:** Tasks 15.4, 22.7, 22.11–22.13 and 45.4.

### PUSH-B04 — wake claim, revalidation and revocation race

- **Owner:** `collab::push::outbox` and executor.
- **Rule:** claim by community; recheck lease generation/expiry/endpoint, current read authorization and membership immediately before delivery; protect the final call with current community-deletion authority.
- **Race:** a committed gateway send-begin may finish after revocation. Revocation committed first prevents admission. This boundary is observable and tested; it is not described as exactly-once.
- **Tests:** Tasks 22.8, 22.13, 37.8 and 45.3.

### PUSH-B05 — App Attest installation authority

- **Owner:** approved APNs platform adapter and durable gateway authority store.
- **Input:** closed bounded challenge, enrollment, delegation, rotation and revocation JSON plus bounded Apple CBOR.
- **Rule:** exact audience/domain transcript, pinned production trust configuration, single-use 300-second challenge, strict assertion counter and atomic installation mutation. Token provenance at enrollment remains a documented bootstrap assumption.
- **Tests:** Tasks 22.9, 43.7 and 45.2.

### PUSH-B06 — endpoint custody and opaque capability

- **Owner:** gateway token-custody and grant keyrings plus shared authority database.
- **Rule:** raw APNs token exists only transiently at the mobile/gateway boundary and encrypted at rest; relay receives a sealed grant. Grant and token keys are independent, versioned and rotated without dropping live authority.
- **Failure:** decrypt/key/profile mismatch is generic unavailable/invalid-grant; it never falls back to plaintext, another profile or a regenerated endpoint.
- **Tests:** Tasks 22.8–22.10, 44.5 and 45.3.

### PUSH-B07 — relay-to-gateway admission

- **Owner:** gateway NIP-98 handler and atomic authority store.
- **Order:** 8192-byte closed body → exact HTTPS URL/method/body NIP-98 → grant AEAD → live relay/profile/epoch/generation/expiry → quota + auth replay + stable request replay transaction.
- **Output:** `invalid_auth`, `invalid_grant` or `temporarily_unavailable` never distinguishes absent installation/delegation/endpoint/quota/replay state.
- **Tests:** Tasks 22.8, 22.9, 22.11 and 45.2.

### PUSH-B08 — APNs provider transport

- **Owner:** ADR-005 APNs adapter behind the payload-less common contract.
- **Rule:** application bytes are always the registered reconnect constant. Only configured topic/environment, opaque destination, request UUID, expiry, push type and priority vary. Response body is reduced to a closed sanitized outcome.
- **Retry:** one credential-refresh attempt is permitted; transient retries are bounded/jittered; only a current matching permanent-endpoint response disables that generation.
- **Tests:** Tasks 22.8, 22.9, 22.11 and 22.12.

### PUSH-B09 — queues, failure recovery and cleanup

- **Owner:** canonical wake-outbox repository, executor and retention workers.
- **Rule:** finite claim lease, attempts, expiry and poison cleanup; terminal jobs/replay evidence retain only the approved audit/abuse window; retry release cannot erase the one-use auth-event fence.
- **Failure:** database, gateway, APNs, cancellation and process restart converge to delivered, failed, suppressed or bounded retry without unbounded `sending` state.
- **Tests:** Tasks 22.12, 22.13, 37.8 and 45.3.

### PUSH-B10 — configuration, metrics and unsupported platforms

- **Owner:** Sim deployment/configuration and private operational surface.
- **Rule:** public HTTPS URL, production/sandbox profile coherence, distinct keyrings, secrets, schema/privileges, resource limits and readiness are validated at startup. Metrics have a closed static label set and remain off the public listener.
- **Fallback:** missing/mismatched configuration is not ready. FCM/UnifiedPush are absent from descriptors and clients visibly use foreground/manual sync.
- **Tests:** Tasks 22.10, 22.12, 43.7, 44.5 and 48.2.

## Known gaps and strengthening obligations

1. Buzz implements the complete lease/matcher/outbox path across `buzz-relay`, `buzz-db`, migrations and the gateway; auditing the gateway crate alone would omit the authoritative match and retry semantics. Tasks 22.6–22.13 must consolidate all four owners into Sim rather than porting a standalone second queue.
2. Buzz's relay worker has several fallible wake completion/retry calls whose errors are discarded after warnings or not surfaced. The canonical executor must record disposition failures, allow claim recovery and alert on stuck age; Task 22.8 owns that strengthening.
3. A gateway delivery task is detached after send-begin so handler cancellation cannot undo replay fences, but process termination can leave a retryable request reservation until bounded request expiry. This is accepted best-effort behavior only with Task 22.12 recovery/age evidence.
4. App Attest does not prove that the submitted APNs token belongs to the attested application key. Endpoint uniqueness, rotation, rate limits and revocation bound the bootstrap assumption; Task 22.9 must preserve the explicit claim.
5. Wake timing and frequency remain visible to Apple. The fixed body prevents content leakage, not traffic analysis. Tasks 22.4, 22.12 and 45.4 must test coalescing, quotas and noisy-neighbor behavior without claiming secrecy of timing.
6. Buzz uses static numeric limits across relay and gateway code. Task 4.4 must give every retained/replaced value a canonical owner, metric, alert threshold and verification command before production readiness.
7. FCM and UnifiedPush have no approved profile or target implementation. They are not silently deferred parity; ADR-005 requires a new explicit approval satisfying its fixed-body, custody, anti-abuse, hostile-endpoint, lifecycle and conformance gate.
8. Existing encrypted leases, gateway installations, delegations, counters, grant/token key versions, epochs, replay windows and pending jobs need versioned import/reconciliation. Task 17.8 and migration Tasks 46.1–46.6 may not contact a provider or reissue a generation during dry-run import.

## Cross-cutting verification checklist

- **Payload minimization:** Tasks 22.6, 22.8, 22.9 and 22.11 vary every admitted field/provider outcome and compare exact outbound application bytes to the profile constant.
- **Lease and origin:** Tasks 11.11, 22.6, 22.7, 22.11 and 22.13 cover closed schemas, origin mismatch, author-only reads, ordering, generation, quota, revocation and cross-community negatives.
- **Amplification and fairness:** Tasks 22.4, 22.12 and 45.4 cover broad filters, duplicate producers/workers, endpoint quota concurrency, coalescing, poison work, queue age and tenant fairness.
- **Installation and capability:** Tasks 22.9, 43.7 and 45.2 cover challenge replay, assertion counter, attestation/profile mismatch, token duplication, wrong relay, grant tampering, epoch/generation and expiry.
- **Provider outcomes:** Tasks 22.8, 22.9 and 22.12 cover accepted, invalid endpoint, retry, timeout, credential refresh, configuration fault, malformed provider response and exhaustion without sibling/identity revocation.
- **Failure, rollback and cleanup:** Tasks 22.10, 22.12, 22.13, 37.8 and 45.3 cover missing secrets/schema, store/gateway/provider outage, process death, expired claim recovery, retention and rollback with no stale-generation resurrection.
- **Mobile compatibility:** Tasks 43.7, 43.8 and 48.2 cover denied permission, background/foreground lifecycle, revoked lease, unavailable push, supported-version negotiation and authoritative refetch.

Task 4.4 must consume every numeric limit named here. Final production evidence comes from Tasks 45.2–45.4 and does not replace the focused negative and recovery tests assigned above.
