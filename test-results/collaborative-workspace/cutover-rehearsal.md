# Aggregate cutover rehearsal

Date: 2026-08-25

Status: **PASS**

## Scope

The rehearsal exercised the public Task 46.1–46.5 boundaries for every canonical `AggregateType` variant using isolated in-memory injected stores. It performed a matching shadow comparison, switched to canonical authority, resumed the exact authority operation, applied one canonical-to-legacy mirror plus its exact duplicate, raised a legacy-only-write divergence halt, completed the pre-boundary rollback, resumed the completed rollback and verified restored legacy authority.

No production database, network, routing, deployment or source system was contacted or mutated. Presence was included for exhaustive aggregate-authority coverage even though its live Redis materialization remains derived state.

The external rehearsal exposed one integration defect: `LegacyProjectionWriter` implementors could inspect an item but could not construct the required immutable receipt. `CanonicalOutboxMirrorItem::expected_receipt` now exposes only that already-validated receipt; it adds no mutation or reverse-reconciliation capability.

## Reproduction

Temporary harness SHA-256: `f70a1c7d49b8323461d5af9c5163ecc995ed2127583e10c914321612472b8762`

```text
cargo test -p collaboration_migrate --test cutover_rehearsal -- --nocapture
```

Result: 1 passed, 0 failed. The harness was removed after recording this immutable evidence because the task authorizes the report, not a second production migration driver.

## Per-aggregate evidence

Every row has `source_count=1`, `target_count=1`, `shadow_match=true`, `mirror_count=1`, `checkpoint_resume=true`, `halt_count=1`, `rollback_count=1`, `rollback_resume=true` and `restored=legacy`.

| Aggregate | Source hash | Target hash | Mirror payload hash | Rollback-plan hash |
| --- | --- | --- | --- | --- |
| `community` | `2929292929292929292929292929292929292929292929292929292929292929` | `2929292929292929292929292929292929292929292929292929292929292929` | `4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a` | `d98bb45b69edd7edb6ef2640341e70e5e40887246ad0cc5c3dab2eced02654fa` |
| `project` | `2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a` | `2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a` | `dbc1b4c900ffe48d575b5da5c638040125f65db0fe3e24494b76ea986457d986` | `a685f79bf230734fc67179800359cd990e81a99100350576cf2267359726aa74` |
| `conversation` | `2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b` | `2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b` | `084fed08b978af4d7d196a7446a86b58009e636b611db16211b65a9aadff29c5` | `f67a2b716c922785b8c5e747f24154e728d9e6f5a47b00b1f5a20ae975a1ce7c` |
| `agent_session` | `2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c` | `2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c` | `e52d9c508c502347344d8c07ad91cbd6068afc75ff6292f062a09ca381c89e71` | `627bd3714a172765322f9c4886932fc4687ef819d2f90777c86e51067368f7d7` |
| `activity` | `2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d` | `2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d` | `e77b9a9ae9e30b0dbdb6f510a264ef9de781501d7b6b92ae89eb059c5ab743db` | `77650598a6f316d30cfa2ee25ea1fd924ce4acca668f337b34ab07a9701f6fe5` |
| `git_change` | `2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e` | `2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e` | `67586e98fad27da0b9968bc039a1ef34c939b9b8e523a8bef89d478608c5ecf6` | `302c231c5719c71007bbd1753debd478fbd178503426f5a5eef410ca90b405e6` |
| `workflow` | `2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f` | `2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f` | `ca358758f6d27e6cf45272937977a748fd88391db679ceda7dc7bf1f005ee879` | `a71b8ccaee08915bd34505504d9867041afe5c9c0b4ced83d2f490bf6a9d4e1f` |
| `identity` | `3030303030303030303030303030303030303030303030303030303030303030` | `3030303030303030303030303030303030303030303030303030303030303030` | `beead77994cf573341ec17b58bbf7eb34d2711c993c1d976b128b3188dc1829a` | `28f2d5b99e7fd4a22c752ff0b984cb7af80260558188baa34a1c3b29b0e4d45d` |
| `presence` | `3131313131313131313131313131313131313131313131313131313131313131` | `3131313131313131313131313131313131313131313131313131313131313131` | `2b4c342f5433ebe591a1da77e013d1b72475562d48578dca8b84bac6651c3cb9` | `436f1333a5dc4182ded03a1f79f9be773f519e67fd81eb8b452232efc48cd6fb` |

## Totals and invariants

- Aggregate kinds: 9
- Source records: 9; target records: 9
- Matching shadow comparisons: 9
- Canonical authority switches: 9; exact checkpoint resumes: 9
- Canonical-to-legacy mirrors: 9; exact duplicate acknowledgements: 9
- Scoped divergence halts: 9
- Completed rollbacks: 9; exact completed rollback resumes: 9
- Final legacy-authoritative aggregates: 9
- Production mutations: 0

The exercise does not authorize or claim a production cutover, a real database migration, a routing change or source retirement. Those remain controlled by the later approval and retirement gates.
