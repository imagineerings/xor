# Tenant, identity and protocol threat model

## Scope and canonical authority

This review covers the security boundaries where a network or internal request acquires a community, authenticates a human, agent, token or service, resolves an active signing identity, parses or signs a Nostr event, evaluates authorization, and reaches tenant-bearing storage or a tenant-observable response. It implements acceptance criteria 6.3, 19.1 and 19.2 for CAP-001, CAP-003, CAP-007, CAP-008 and CAP-009.

The accepted ownership decisions remain normative:

- `crates/collaboration_domain` owns UI-free tenant, principal, identity-binding, authorization and provenance types. A wire tag, URL parameter, database UUID or compatibility event cannot construct trusted context directly.
- `crates/collab` owns request admission, current membership and role policy, tenant-fenced repositories, the signed-event log, command/outbox commit and per-community audit chain.
- `crates/nostr_compat` owns byte-exact event, filter, kind, NIP-42 and NIP-98 compatibility semantics but never authorizes a request or retrieves a signing key.
- `crates/client` and the existing Sim service authentication stack own service accounts and organization principals. Possession of a Nostr key does not imply a Sim account.
- `crates/credentials_provider` and `crates/sim_credentials_provider` own private-key custody, protected references, import, rotation, backup and recovery. Collaboration tables contain public keys and credential references only.
- The immutable signed event is the authorship authority. Identity bindings, profiles and NIP-OA attestations add authorization and provenance; none may rewrite the author.
- Redis, search, presence, caches and compatibility responses are derived tenant-scoped projections. They cannot become authentication, authorization, identity or durable event authority.
- ADR-001 fixes the final service and schema owner in Sim `collab`. ADR-002 fixes community/profile-scoped account-to-npub binding and one active signer per tuple.

The review does not declare physical timing non-interference. Shared database, CPU, cache and network contention remain measurable. It does require coarse admission, bounded work, per-tenant quotas and tests showing that content, identifiers, exact counts, response classes and private metadata do not cross tenants. Task 4.4 owns measurable operational limits and timing-class alert budgets.

## Source evidence and preserved constraints

- `projects/buzz/docs/multi-tenant-relay.md` defines host-derived `TenantContext`, host/channel agreement, label-flow non-interference, composite tenant keys, a finite error alphabet, tenant-scoped projections and per-community signing/audit separation. Its TLA+/Tamarin claims are relative to named RLS, cryptographic, resolution and deployment axioms; implementation tests must admit those axioms independently.
- `projects/buzz/crates/buzz-core/src/tenant.rs` deliberately has no `Default` or `Deserialize` for tenant context, but its public `resolved`/`from_uuid` constructors are only a lint-and-review fence. Sim must replace that with constructors inaccessible outside approved admission and repository adapters.
- `projects/buzz/crates/buzz-relay/src/tenant.rs` normalizes host names, rejects empty/unmapped/lookup-failed hosts generically and has no default-tenant fallback. Deployment-internal calls resolve the configured relay authority through the same path.
- Buzz NIP-42 uses a random challenge and a five-second unauthenticated deadline. Its connection path bounds slots, send queues and slow clients and stores the resolved tenant on the connection before frames are read.
- Buzz NIP-98 validates kind, signature, timestamp, exact URL/method and optional payload hash. Its current shared replay guard is tenant-scoped and fail-closed, but dev-mode `X-Pubkey` deliberately skips replay and must never be reachable in a production profile.
- Buzz's formal model requires RLS on every tenant table, a non-superuser `NOBYPASSRLS` request role, transaction-local community state, audited `SECURITY DEFINER` functions and community-bearing uniqueness/foreign keys. These are target migration/startup assertions, not assumed properties.
- Buzz desktop protects human and managed-agent keys in the OS keyring, verifies imports before deleting plaintext and documents an owner-only `0600` fallback. Environment key precedence and plaintext fallback enlarge the blast radius and must be explicit deployment/profile policy, never an unnoticed compatibility default.
- Buzz's audit chain is keyless and tamper-evident, not tamper-resistant. The canonical target keeps one serialized chain per community, attributes security operations and exposes this limitation rather than treating a hash chain as a database-administrator trust boundary.
- Buzz relies on TLS termination outside the relay. The consolidated deployment must validate TLS/proxy configuration and trusted forwarded-host provenance; an arbitrary forwarded host cannot become tenant authority.

## Security invariants

1. **TI-01 — row-zero tenant derivation.** A typed tenant is derived exactly once from a trusted listener/deployment route and canonical authority before authentication, authorization, parsing that allocates material resources, database access or tenant-observable lookup.
2. **TI-02 — agreement, never override.** Channel mappings, token stamps, signed URLs, event tags and body fields may agree with or narrow the row-zero tenant. Missing or conflicting values reject generically; none can select or replace it.
3. **TI-03 — authorization before observation.** Current principal, membership, role, scope, owner attestation and resource authorization are evaluated before existence lookup, ranking, limit, count, projection access or mutation. Rate limits use trusted tenant/principal keys and cannot reveal another tenant's occupancy.
4. **TI-04 — defense in depth at storage.** Every queryable tenant table, constraint, partition, projection, object pointer, cache key and audit head carries `CommunityId`. The request database role and transaction-local scope fail closed even if an application predicate is omitted.
5. **TI-05 — finite outward errors.** Authentication, authorization, not-found, duplicate, constraint, replay and internal failures map to a bounded public error taxonomy that excludes host existence, constraint names, identifiers, tuples, SQL, key material and cross-tenant distinctions.
6. **TI-06 — cryptographic verification before authority.** Canonical bytes, event ID, signature, kind, timestamp, challenge/request binding and replay are validated before a signed event or token can influence identity, policy, persistence or projections.
7. **TI-07 — one active signer, immutable authorship.** At most one verified active binding exists per community/account/profile tuple. Rotation or revocation changes future authority atomically but never rewrites historical event authorship or treats an owner attestation as owner authorship.
8. **TI-08 — protected key custody.** Private keys, backup phrases, recovery factors and reusable challenges never enter collaboration rows, signed public events, logs, telemetry or public errors. Failure to read or verify protected storage cannot synthesize or activate a replacement identity.
9. **TI-09 — bounded replay and revocation.** NIP-42 challenges, NIP-98 events, bearer tokens, invites, binding proofs and cached authorization have scoped nonce/generation, expiry and revocation behavior that works across replicas and fails closed when shared replay/current-policy state is unavailable.
10. **TI-10 — projection non-authority.** Search, count, thread metadata, presence, Redis fan-out, compatibility output and audit views inherit tenant and source provenance. Rebuild, lag, cache loss or adapter disagreement cannot grant authority or expose another tenant.
11. **TI-11 — per-community system authority.** Community system events, membership admission and audit chains use community-specific keys/heads. Compromise of one community key or head cannot authorize, sign or splice another community.
12. **TI-12 — cleanup and least privilege.** Unauthenticated/authenticated connection state, challenges, subscriptions, transaction scope and decrypted key buffers have bounded lifetime and cleanup. Service, migration, sidecar and request roles receive only their documented database, signing and network capabilities.

## Threat ledger

| Threat ID | Threat and observable failure | Fail-closed control and canonical owner | Required negative or recovery tests |
| --- | --- | --- | --- |
| T-TIP-001 | Missing, empty or unmapped host falls through to a default community or reveals which hosts exist | Listener constructs no `TenantContext`; one generic admission error; `collaboration_domain::tenant` plus `collab::tenant_admission` | Tasks 13.1, 13.4, 13.6, 45.2 |
| T-TIP-002 | Untrusted `Host`/forwarded-host input is accepted from an untrusted proxy hop | Deployment supplies an allowlisted trusted-proxy chain and canonical authority; direct and ambiguous forwarded hosts reject | Tasks 13.1, 14.5, 44.2, 45.2 |
| T-TIP-003 | Case, trailing-dot, default-port, IPv6 or Unicode normalization splits one tenant or aliases two | One authority parser shared by provisioning and request admission; stored normalized unique key; ambiguous/invalid forms reject | Tasks 13.1, 13.6, 44.2 |
| T-TIP-004 | An `h` tag, community field or channel ID drives a broad service into another tenant | Resolve channel under row-zero tenant and require immutable host/channel agreement before lookup or duplicate check | Tasks 13.1, 13.3, 14.4, 13.6 |
| T-TIP-005 | A token, invite, profile or workflow identifier stamped for B is presented on A's route | Stamp is an equality fence, not selector; policy and repositories key all evidence by trusted community | Tasks 13.3, 13.5, 14.5, 13.6 |
| T-TIP-006 | A direct ID, `#e`, `#a`, no-`#h` feed or auxiliary query bypasses channel membership | Common authorization executes before repository/filter evaluation; channel-less resources bind to row-zero community | Tasks 13.3, 14.3, 15.2, 13.6 |
| T-TIP-007 | Authentication, authorization, not-found or duplicate response distinguishes private resource existence | Fixed public result taxonomy and indistinguishable missing/denied behavior where privacy requires it | Tasks 13.4, 14.3, 14.4, 14.5, 13.6 |
| T-TIP-008 | Ranking or limiting before policy lets an attacker infer hidden search hits | SQL/repository applies tenant and visibility predicates before ranking/limit; excluded kinds have no search vector | Tasks 15.6, 16.4, 16.5, 13.6 |
| T-TIP-009 | COUNT, EOSE cardinality, pagination bounds or total metadata includes unauthorized rows | Count/window derives from the exact authorized relation; no global pre-count; continuation is tenant/version bound | Tasks 14.3, 16.5, 13.6, 45.1 |
| T-TIP-010 | Global event-ID uniqueness or constraint errors form a cross-tenant existence oracle | Composite community keys/foreign keys and sanitized database errors; tenant is bound before conflict lookup | Tasks 15.1, 15.2, 13.6, 45.2 |
| T-TIP-011 | Omitted application predicate reads or mutates a foreign row | RLS on every tenant table, non-owner/NOBYPASSRLS role, `SET LOCAL` per transaction and startup schema audit | Tasks 12.3, 15.1, 15.7, 13.6 |
| T-TIP-012 | `SECURITY DEFINER`, leakproof/user functions, migrations or sidecar credentials bypass RLS | Audited allowlist, no migration/projection grants for request/sidecar roles, single ADR-001 migration authority | Tasks 14.1, 15.1, 44.1, 44.4, 45.2 |
| T-TIP-013 | Projection rebuild or checkpoint reuse writes A-derived metadata into B | Provenance includes tenant/source/version; tenant-scoped checkpoint/reset; rebuild cannot serve global intermediate rows | Tasks 15.3, 15.5, 17.1, 13.6 |
| T-TIP-014 | Redis topic, local-echo dedup, presence, typing or cache keys omit tenant | Typed tenant envelope/key prefix; receiver revalidates tenant/source; Redis expires and never authorizes | Tasks 16.1, 16.2, 21.4, 21.5, 13.6 |
| T-TIP-015 | Unauthenticated NIP-11/health metadata exposes global counts, tenant names or private configuration | Public document consumes static/addressed-host-safe inputs only; metrics/health require operator boundary and redaction | Tasks 14.5, 44.5, 45.2 |
| T-TIP-016 | Reused, predictable or cross-connection NIP-42 challenge authenticates the wrong actor | CSPRNG challenge scoped to connection/tenant/relay URL with deadline, one terminal use and cleanup | Tasks 14.2, 45.1, 45.2 |
| T-TIP-017 | NIP-42 AUTH succeeds after timeout, revocation or tenant change | Recheck deadline, signature, challenge, current key/binding/membership and tenant immediately before transition | Tasks 12.7, 13.3, 14.2, 45.2 |
| T-TIP-018 | NIP-98 signature is replayed on another pod or community inside timestamp tolerance | Shared atomic seen set keyed by community/event ID, bounded TTL/capacity and fail-closed outage behavior | Tasks 13.5, 14.5, 16.1, 45.2 |
| T-TIP-019 | NIP-98 URL normalization, proxy reconstruction, method or payload mismatch widens request authority | Reconstruct expected external URL only from trusted route/proxy data; exact method and optional body digest; no alias fallback | Tasks 11.3, 14.5, 44.2, 45.1 |
| T-TIP-020 | Dev `X-Pubkey`, disabled-auth option or zero replay ID reaches production | Configuration profiles make bypass unavailable outside explicit local test mode; startup rejects insecure production combination | Tasks 14.5, 44.2, 44.4, 45.2 |
| T-TIP-021 | Malformed/oversized event, filter, tag or canonical JSON consumes resources or reaches policy partially parsed | Frame/body/depth/cardinality bounds before allocation; exact codecs; one typed rejection and no persistence | Tasks 11.2, 11.3, 11.4, 14.3, 14.4 |
| T-TIP-022 | Altered event ID, signature, author, timestamp or replacement coordinate is accepted | Recompute canonical ID, verify BIP-340 and exact kind/head rules before authorization and durable write | Tasks 11.2, 11.3, 11.4, 14.4, 45.1 |
| T-TIP-023 | A community/system signing key signs another community's membership or system event | Credential handle and signing request are tenant typed; output author/key is checked against intended community before publish | Tasks 12.4, 12.5, 13.5, 14.4, 45.2 |
| T-TIP-024 | Nostr possession is treated as Sim account, organization or billing authentication | Common principal keeps account and key claims distinct; explicit verified binding required by policy | Tasks 12.1, 13.2, 13.3, 45.2 |
| T-TIP-025 | Binding challenge is replayed, retargeted to another community/profile or approved by an administrator without key possession | Single-use domain-separated challenge bound to account/community/profile/pubkey and current policy; possession proof mandatory | Tasks 12.1, 12.7, 13.2, 45.2 |
| T-TIP-026 | Concurrent activation creates two active signers or binds one npub to conflicting owners | Tenant/account/profile uniqueness plus optimistic version transaction; conflict has no partial state | Tasks 12.1, 12.3, 12.7 |
| T-TIP-027 | Rotation, revocation, archive or recovery failure silently replaces/resurrects a key | Verify successor storage first; atomic lifecycle transition; terminal states cannot be toggled active; old source retained until confirmation | Tasks 12.4, 12.5, 12.6, 12.7 |
| T-TIP-028 | Keyring outage/corruption generates a new identity or falls back to insecure storage unnoticed | Explicit unavailable state; owner-only fallback requires approved profile and disclosure; never synthesize a key | Tasks 12.4, 12.5, 12.6, 45.2 |
| T-TIP-029 | nsec, backup, recovery factor, challenge or credential value leaks through errors/logs/telemetry | Secret-bearing types and references, zeroized temporary buffers where supported, structured redaction and negative log capture | Tasks 12.4, 12.6, 35.3, 44.5, 45.2 |
| T-TIP-030 | NIP-OA owner attestation causes agent events to be displayed or authorized as owner-authored | Preserve independent agent author; verify bounded attestation/current policy only for delegation provenance | Tasks 11.6, 12.2, 13.2, 13.3 |
| T-TIP-031 | Revoked membership/binding remains in cache, token, subscription or autocomplete | Version/generation on cached decisions; revocation invalidates active projections and disconnects/rechecks live capabilities | Tasks 12.5, 12.7, 13.3, 13.5, 14.2 |
| T-TIP-032 | Audit query/splice or global hash head reveals or corrupts another community | One serialized canonical head per community, tenant-fenced immutable entries, redacted content and stale-head rejection | Tasks 35.1, 35.2, 35.5, 35.6, 45.2 |
| T-TIP-033 | TLS or trusted-proxy misconfiguration exposes credentials or lets transport metadata select authority | Validated deployment profile, TLS in production, trusted proxy allowlist and redacted startup failure; no insecure fallback | Tasks 44.2, 44.4, 45.2 |
| T-TIP-034 | Shared rate-limit, queue or connection state reveals another tenant's exact load or permits noisy-neighbor bypass | Tenant/principal-scoped admission keys, coarse generic rejection, configured global safety ceiling and per-tenant fairness | Tasks 13.4, 14.3, 16.2, 4.4, 45.4 |
| T-TIP-035 | Cancellation or disconnect leaves tenant transaction state, challenge, subscription or decrypted key material reusable | Structured ownership, transaction-local scope, cancellation cleanup, buffer clearing and generation checks | Tasks 12.4, 14.2, 14.3, 15.7, 45.2 |
| T-TIP-036 | Compatibility adapter accepts a legacy semantic that canonical policy rejects or dual-writes authority | Decode to one canonical command, run common admission, compare differential output and keep one command/outbox commit | Tasks 14.1, 14.4, 17.8, 45.1, 46.4 |

## Boundary checklist

Numeric limits not already frozen by Buzz are assigned by Task 4.4. Until an owner configures and validates a required limit, the relevant production path is unavailable rather than unlimited.

### TI-B01 — listener, proxy and host-to-tenant binding

- **Owner:** `collaboration_domain::tenant`, `collab::tenant_admission` and deployment routing configuration.
- **Untrusted input:** socket/listener identity, `Host`, SNI, forwarded host/proto, URL authority and client community fields.
- **Required order:** accept only a configured listener or trusted proxy hop; canonicalize one authority; resolve one active community; reject absence, ambiguity, inactive community or lookup failure before handler/auth/database work.
- **Output/error:** a typed tenant or one generic unavailable/denied class. No echo of candidate host, community ID or lookup distinction.
- **Cleanup/least privilege:** context lifetime is request/connection scoped; no global mutable current tenant. Deployment and request roles cannot create host mappings.
- **Assigned tests:** Tasks 13.1, 13.4, 13.6, 44.2 and 45.2.

### TI-B02 — NIP-42 WebSocket authentication and connection state

- **Owner:** `collab::nostr::auth` using `nostr_compat` verification and common principals.
- **Untrusted input:** AUTH frames, event bytes, challenge, pubkey, relay URL, timestamps and repeat attempts.
- **Bounds/order:** connection and frame ceiling; one CSPRNG challenge; five-second Buzz compatibility floor unless Task 4.4 approves a stricter value; exact schema/signature/challenge/URL validation; replay/current policy before authenticated transition.
- **Authorization:** authentication creates a principal only. Channel/resource policy remains TI-B05 and rechecks current revocation.
- **Cleanup/secrets:** timeout, failure, close and cancellation erase challenge/pending responders and release slots/subscriptions. Challenges and auth frames are not logged verbatim.
- **Assigned tests:** Tasks 14.2, 14.3, 13.6 and 45.2.

### TI-B03 — NIP-98 HTTP authentication, token mint and replay

- **Owner:** `collab::nostr::http`, common principal/admission policy and shared replay store.
- **Untrusted input:** authorization bytes, URL/method/body, token claims, route host and proxy metadata.
- **Bounds/order:** bound header/body before decode; verify kind, canonical ID, Schnorr signature, timestamp, exact reconstructed URL/method/payload; atomically mark tenant/event replay; then evaluate current policy.
- **Authorization:** token community/scope must agree with row-zero tenant and may only narrow. Production startup rejects dev bypass or unavailable replay authority.
- **Output/error:** generic invalid/expired/replayed/denied results without signature internals, token hashes, route mapping or resource existence.
- **Assigned tests:** Tasks 11.3, 13.5, 14.5, 44.2 and 45.2.

### TI-B04 — signed-event and filter compatibility boundary

- **Owner:** `nostr_compat` for pure bytes/semantics; `collab::nostr` for admission.
- **Untrusted input:** JSON, tags, kinds, IDs, signatures, timestamps, filters, subscription IDs and compatibility versions.
- **Bounds/order:** bound bytes/depth/tags/filter counts first; compute canonical ID and verify signature; classify kind/privacy/replacement; derive typed command; policy decides; repository writes once.
- **Output/error:** exact supported wire response from the finite public taxonomy. Unknown/malformed versions do not fabricate canonical state.
- **Cleanup/secrets:** pure codecs have no credentials, database or GPUI dependency and retain no request state.
- **Assigned tests:** Tasks 11.2–11.5, 14.1, 14.3, 14.4 and 45.1.

### TI-B05 — principal, identity binding and common authorization

- **Owner:** `collaboration_domain::{principal,account_binding,authorization}` with current records in `collab`.
- **Untrusted input:** account/session claims, npub, binding proof, owner attestation, role, scope, membership, invite and requested resource.
- **Required decision:** independently verify claims; require explicit binding where policy says; load current tenant-scoped versions; evaluate membership/role/scope/resource/delegation; return typed allow/deny plus policy version.
- **Identity rule:** account, Nostr author, profile and agent owner remain distinct. One active signer per approved tuple; history remains immutable.
- **Failure/revocation:** stale version, ambiguity, unavailable policy or conflicting owner fails closed and cannot populate a permissive cache.
- **Assigned tests:** Tasks 12.1–12.7, 13.2–13.5 and 45.2.

### TI-B06 — credential retrieval, signing and lifecycle

- **Owner:** existing Sim credential providers and `sim_credentials_provider` Nostr adapters.
- **Untrusted input:** imported nsec/hex/NIP-49 data, backup, recovery input, key reference, signing preimage and lifecycle request.
- **Bounds/order:** bounded parsing/KDF; protected write and round-trip signature before activation; tenant/binding/key agreement before every sign; verify signed output before release.
- **Secret rule:** collaboration state stores only public key and protected reference. Raw values never reach command args, public events, logs, telemetry or errors; old source survives until confirmed.
- **Failure/cleanup:** unavailable/corrupt storage has no generated fallback identity; clear transient material and leave prior active binding unchanged.
- **Assigned tests:** Tasks 12.4–12.7, 35.3, 44.5 and 45.2.

### TI-B07 — tenant-fenced database transaction and constraints

- **Owner:** ADR-001 Sim migration authority and `collab` repositories.
- **Required role:** non-superuser, non-owner, `NOBYPASSRLS`; transaction-local community set before statements; no pooled-connection tenant residue.
- **Schema rule:** all tenant tables, partitions, indexes, uniqueness, foreign keys, audit heads and projection checkpoints include community. No request path uses an unscoped lookup as an authorization oracle.
- **Error rule:** rollback the whole command/outbox transaction and translate database errors to the public taxonomy before logging/responding.
- **Startup proof:** enumerate policies, roles, functions, constraints and grants; refuse readiness on drift. Migration/sidecar roles are separate and not request identities.
- **Assigned tests:** Tasks 12.3, 15.1–15.7, 44.1, 44.4 and 45.2.

### TI-B08 — authorized queries, counts, search and projections

- **Owner:** authoritative repository plus aggregate-specific projection/search owners.
- **Untrusted input:** IDs, tags, filters, cursor, query text, ranking, limit and requested projection version.
- **Order:** tenant and current visibility relation first; then matching/ranking/limit/count; return tenant/version-bound cursors and freshness. Privacy-excluded or ephemeral content never enters durable projection/search/log storage.
- **Rebuild:** replay only labeled source rows into tenant-scoped derived rows; intermediate/global scan state is operator-only and not tenant-served.
- **Failure:** lag is explicit and cannot widen reads; unavailable policy/projection does not fall back to a global query.
- **Assigned tests:** Tasks 13.6, 15.3, 15.5, 15.6, 16.3–16.5 and 45.2.

### TI-B09 — Redis, realtime fan-out and authorization caches

- **Owner:** `collab::pubsub` and canonical current-policy repositories; Redis remains derived.
- **Envelope/key:** community, source ID/version and bounded payload reference are mandatory. Presence/typing/cache/dedup keys also include community and generation/expiry.
- **Delivery:** subscriber and current membership are revalidated where required; wrong-tenant, stale, duplicate and unknown-version envelopes drop without local mutation.
- **Failure/cleanup:** Redis outage exposes partial freshness and reconnect/refetch; it never fails open or supplies durable membership. Cancellation removes local registrations and bounded buffers.
- **Assigned tests:** Tasks 16.1–16.3, 21.4, 21.5 and 45.2.

### TI-B10 — community system signing and audit observation

- **Owner:** community-scoped credential binding, `collaboration_domain::audit` and one `collab::audit` repository writer.
- **Input:** security/admin operation, redacted outcome, actor, tenant, stable operation ID, prior head and intended system-event preimage.
- **Rules:** one key/head per community; compare-and-swap serialized append; verify signer/tenant and prior head; immutable retained entries; exported segment remains tenant authorized.
- **Limit:** the keyless hash chain detects ordinary mutation but does not defeat a database administrator who recomputes the chain. External anchoring is not silently claimed.
- **Failure/secrets:** stale/cross-tenant head rejects; audit failure follows the operation's declared atomicity and is visible; private payloads and key material are excluded before hash/log/export.
- **Assigned tests:** Tasks 35.1–35.6 and 45.2.

## Mandatory admission sequence

Every network and internal compatibility path follows this dependency order:

1. Enforce transport-level connection/header/body ceilings that require no tenant data.
2. Establish trusted listener/proxy provenance and derive exactly one row-zero `TenantContext`.
3. Parse only the bounded authentication envelope and verify cryptographic request binding and replay.
4. Construct independently verified principal claims and resolve the active community/profile binding.
5. Load current tenant-scoped membership, role, scope, owner-attestation and resource state.
6. Authorize before existence lookup, detailed validation that depends on private state, ranking, limit, count or mutation.
7. Begin a tenant-scoped least-privilege transaction, recheck write preconditions and commit authority plus one outbox operation atomically.
8. Project a redacted result through the finite public error/response taxonomy, then clean up transaction, challenge, subscription and transient key state.

An adapter may reject earlier for syntax, size, version or cryptographic invalidity, but it may not consult tenant data or emit a tenant-distinguishing result before steps 2–6. Rate limiting before full authorization may use only row-zero tenant plus already verified principal/network classes and emits a coarse result; resource-specific limits are applied after authorization.

## Known gaps and required strengthening

1. Buzz's public trusted-tenant constructors are intentionally enforced by lint/review rather than visibility. Task 13.1 must make construction dependency-safe and inaccessible to codecs, payload deserializers and ordinary handlers.
2. The formal multi-tenant proof admits RLS, constraint, role and cryptographic axioms. Tasks 13.6, 15.1 and 45.2 must run mutation/negative checks against the real target schema and request roles; the proof document alone is not release evidence.
3. NIP-98 replica safety depends on a shared atomic seen set and sufficient capacity/TTL. Process-local caches or best-effort Redis are not compatible fallbacks. Tasks 14.5, 16.1 and 4.4 own the executable bound and outage gate.
4. Buzz dev `X-Pubkey` auth skips replay by using a zero event ID. Task 44.2 must make that route impossible in production configuration and Task 45.2 must try to enable/use it.
5. Buzz's environment-key precedence and owner-only plaintext fallback are explicit compatibility behaviors, not universal target defaults. ADR-002 policy plus Tasks 12.4–12.6 decide availability per profile and must surface weaker custody.
6. TLS is external to the Buzz relay. Tasks 44.2 and 44.4 must validate TLS/trusted-proxy production profiles and fail readiness rather than relying on operator prose.
7. The audit chain is tamper-evident only. Tasks 35.1–35.6 must preserve the honest claim, serialize each community head and test deletion/reorder/mutation/cross-head behavior.
8. Physical timing channels are not eliminated by logical isolation. Task 4.4 must define bounded work, per-tenant fairness and coarse public outcomes; Task 45.4 must exercise noisy-neighbor behavior without claiming constant time.
9. Historical events authored before revocation remain valid history. Current authorization, discovery and signing must change immediately, while retention/deletion rules own later visibility. Tests must not assert retroactive signature invalidation.
10. Compatibility routes may remain during migration but cannot retain independent tenant, key, membership or event authority. ADR-001 sidecar grants, divergence metrics and removal gates apply to every path in this review.

## Cross-cutting verification checklist

- **Tenant construction:** Tasks 13.1 and 13.6 inject absent, malformed, conflicting and payload-derived tenants across RPC, WebSocket, HTTP, database, cache, search, object, Git and count paths.
- **Authorization order:** Tasks 13.4, 13.6 and 16.5 instrument repository/query calls and prove denied requests never execute existence, ranking, limit or count work.
- **Protocol and replay:** Tasks 11.2–11.5, 14.2–14.5 and 45.1 cover canonical bytes, invalid signatures, challenge misuse, NIP-98 URL/body mismatch, HA replay and bounded filters.
- **Storage isolation:** Tasks 12.3, 15.1, 15.2, 15.7 and 45.2 mutate application predicates, transaction scope, constraints, request roles and tenant keys and require the storage fence to hold.
- **Identity/key lifecycle:** Tasks 12.1–12.7 and 45.2 cover possession proof, active uniqueness, conflicting owners, storage outage, rotation failure, revoked recovery, source preservation and secret-negative logs.
- **Metadata/error privacy:** Tasks 13.6, 14.3–14.5, 16.4, 16.5, 35.3, 44.5 and 45.2 compare response class/body, exact count, cursor, search results, logs and metrics across two communities.
- **Replica/cache behavior:** Tasks 16.1–16.3 and 45.2 cover wrong-tenant envelope, replay, stale membership, Redis outage, reconnect and last-trustworthy-state behavior.
- **Audit/system authority:** Tasks 35.1–35.6 and 45.2 cover per-community signer/head, stale append, cross-chain splice, redaction and the documented database-administrator limitation.
- **Deployment least privilege:** Tasks 14.1, 44.1–44.5 and 45.2 inspect sidecar/request/migration grants, trusted proxies, TLS, insecure feature combinations, redacted configuration and telemetry-disabled behavior.

Task 4.4 must ingest every numeric-bound placeholder in this document. Task 45.2 is the final independent execution gate; it does not replace the focused leaf tests named in the ledger.
