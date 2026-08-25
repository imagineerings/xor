# Collaborative workspace protocol differential gate

Status: **PASS for Task 45.1**. Every declared protocol lane completed with zero unexplained semantic or failure-frame divergence.

Captured on 2026-08-25 from source revision `fcb49ae0458962bf0e86044aedb7a6a99ec8a684`.

## Result summary

| Lane                                                        | Buzz/reference path                                                                                                         | Consolidated path                                                                                                                                                                    | Result                                                                                                         |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Signed events, heads, privacy, wire frames and relay traces | Independent `buzz-conformance` replay checker and frozen Python corpus                                                      | `nostr_compat` codecs plus Collab ingress, subscription and reconnect adapters                                                                                                       | PASS — 22 Rust checks, 25 frozen Python cases, 91 consolidated codec tests and 24 service-adapter tests passed |
| Standard Nostr behavior                                     | Frozen NIP-01 event and relay artifacts plus generated kind catalog                                                         | NIP-01 event verification, NIP-11 discovery, NIP-29 kind registration, NIP-42 authentication, NIP-45 count/subscription frames, NIP-50 filter behavior and NIP-98 HTTP authorization | PASS — supported results and rejection frames remained closed                                                  |
| Sixteen Buzz custom NIPs                                    | Frozen source documents, manifests and codec vectors for AA, AE, AM, AO, AP, CW, DV, ER, GS, IA, MP, OA, PL, PMA, RS and WP | Catalog-driven `nostr_compat::buzz_nips` codecs and registry                                                                                                                         | PASS — every document/vector pair was registered; malformed, privacy and version cases failed closed           |
| Multitenant relay observations                              | Independent adapter-neutral observation reducer                                                                             | Consolidated RPC policy gate for event, filter, count, search, notification and log probes                                                                                           | PASS — 1 differential audit passed with no content, identifier, count or timing-class leak                     |
| Git                                                         | System `git http-backend` reference server                                                                                  | Consolidated smart-HTTP read/write services, object backend, NIP-34 and Nostr signing adapters                                                                                       | PASS — 3 tests passed clone/push equivalence, signing round trip and pre-storage denial                        |
| Media                                                       | Test-owned Buzz observation oracle                                                                                          | Consolidated media authorization, object and metadata path                                                                                                                           | PASS — 2 tests passed; the checker also rejected missing and changed observations                              |
| Pairing                                                     | Frozen client/version matrix and NIP-AB vectors                                                                             | Production pair-relay session path and protected-import coordinator                                                                                                                  | PASS — 4 tests passed six desktop/mobile/CLI directions, replay, cancel, expiry, corrupt QR and recovery       |
| Standalone CLI shim                                         | 28-case frozen client manifest, including seven CLI contracts                                                               | `buzz_compat` argument/version/dispatch adapter                                                                                                                                      | PASS — 6 shim tests and all 28 frozen client contracts passed                                                  |
| Web client                                                  | Frozen Buzz web 0.1.0 routes and protocol contract                                                                          | Migrated browser-Web-API adapters with injected canonical services                                                                                                                   | PASS — 4 scenarios passed, including incompatible and read-only pre-mutation failures                          |
| Mobile client                                               | Frozen Buzz mobile 0.0.0+1 contract and NIP-AB version one                                                                  | Migrated pure-Dart data, lifecycle and pairing adapters                                                                                                                              | PASS — 4 scenarios passed under Dart 3.11.4                                                                    |
| Administration client                                       | Frozen Buzz admin 0.1.0 lifecycle and failure contract                                                                      | Migrated operator-session and resource adapters                                                                                                                                      | PASS — 5 scenarios passed, including role, tenant and version failures                                         |

## Independent-oracle boundary

The Buzz `buzz-conformance` crate has no dependency on any production Buzz crate. It owns an opaque community label and independently reimplements the transition relation; its property tests call only the public checker. The standard-library Python checker independently implements canonical event hashing, BIP-340 verification, privacy visibility, head selection and relay-trace invariants without importing either application path.

The consolidated Rust suites consume the same frozen event, head, privacy, wire, relay, custom-NIP and client manifests through production codecs and adapters. Git compares a real system `git http-backend` with the consolidated smart-HTTP services. Media compares a separately constructed Buzz observation trace with the live consolidated service objects. Pairing drives the production loopback relay while independently fixing the allowed client/version matrix and expected session outcomes.

The checkers demonstrably bite: the executed suites reject tampered signatures and identifiers, noncanonical events, stale head resurrection, privacy leaks, malformed/oversized frames, unsupported versions, cross-tenant observations, Git authorization after storage, missing or changed media observations, pairing replay/corrupt QR/expiry, and client writes that reach signing, identity, resource or storage work before compatibility admission.

## Reproduction commands

The following commands passed unchanged:

```sh
CARGO_TARGET_DIR=/tmp/buzz-conformance-target cargo test -p buzz-conformance --locked -- --nocapture
PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/collaborative-workspace/fixtures/protocol/check_fixtures.py
cargo test -q -p nostr_compat
cargo test -q -p collab --test nostr_auth_vectors --test nostr_event_ingest --test nostr_http --test nostr_ingress_version --test nostr_reconnect --test nostr_subscriptions
cargo test -q -p collab --test multitenant_conformance
cargo test -q -p collab --test git_conformance
cargo test -q -p collab --test media_conformance
cargo test -q -p collab --test pairing_interop
cargo test -q --manifest-path tools/buzz_compat/Cargo.toml
node clients/web/tests/compatibility/web_compatibility.browser.test.ts
/private/tmp/codex-dart-3.11.4/dart-sdk/bin/dart clients/mobile/test/compatibility/mobile_compatibility_e2e.dart
node --test admin-web/tests/compatibility/admin_compatibility_e2e.browser.test.ts
PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/collaborative-workspace/fixtures/clients/check_contracts.py
```

The protocol corpus checker reported `events=7 replaceable=2 privacy=7 mixed_version=1 wire=4 relay=4`. The client manifest checker reported `buzz-admin-web=4 buzz-cli=7 buzz-mobile=13 buzz-web=4 total=28`. The Git and pairing tests require only loopback listeners; their initial sandboxed attempts were denied before test traffic, and the unchanged commands passed after loopback binding was allowed.

## Environment

| Component | Version                                                                      |
| --------- | ---------------------------------------------------------------------------- |
| Host      | macOS 26.6.1 (25G76), arm64                                                  |
| Rust      | `rustc 1.97.1 (8bab26f4f 2026-07-14)`; `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Python    | 3.14.5                                                                       |
| Node.js   | 24.13.0                                                                      |
| Dart      | 3.11.4 stable, macOS arm64                                                   |
| Git       | 2.50.1 (Apple Git-155)                                                       |

No production service, tenant, credential, database or object store was contacted.

## Scope limits

- The web, mobile and administration compatibility suites use injected, test-owned service boundaries. They prove the frozen client protocol contracts and pre-mutation failure ordering, not browser rendering, physical-device behavior or deployable production HTTP routes.
- The Git and pairing listeners are local loopback services. The media path uses in-memory/test-owned service dependencies. This gate does not establish production availability, throughput or external-provider behavior.
- This report closes the independent protocol differential gate only. Security, migration/deletion, scale, orchestration and consolidated release gates remain owned by Tasks 45.2–45.6.
