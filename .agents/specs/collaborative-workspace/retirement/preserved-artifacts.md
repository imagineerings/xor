# Buzz preserved-artifact and source-history ledger

Date: 2026-08-25

Status: **PRESERVED EVIDENCE — SOURCE RETIREMENT HOLD**

This ledger accounts for the Buzz artifacts imported into or retained by the Collaborative Workspace migration. It establishes the durable artifacts already committed to this repository and the reference-source policy required before any later source deletion. It does not delete, relocate, publish or modify the supplied Buzz tree.

## License and provenance

| Item | Evidence | Disposition |
| --- | --- | --- |
| Buzz license | Apache License 2.0; `Copyright 2026 Block, Inc.`; source `LICENSE` SHA-256 `108cb15997e51b75a8d18b0c1e2c52bd3879d051ab02118973387df1e4aab584` | Retained as `LICENSES/buzz.md`; full terms remain in root `LICENSE-APACHE`. |
| Additional source-root notices | No `NOTICE`, `COPYING` or second `LICENSE*` file was present on 2026-08-25. | No absent notice is invented. Re-run the notice scan if the baseline changes. |
| Approved reference-source location | User-supplied read-only baseline `/Users/ahmad.vegah/repos/imagineerings/zed/projects/buzz`, addressed by the specification as `projects/buzz` | Retain unchanged through all retirement gates. This local path is an inspection source, not a distributable archive. |
| Containing checkout | `/Users/ahmad.vegah/repos/imagineerings/zed`, observed HEAD `e092824ac729a83a1bdab007ee9670f1f6756b99` | Not source-history proof: its root `.gitignore` excludes `projects/`, `git ls-files projects/buzz` returns zero files and no independent Buzz `.git` exists. Do not label this commit a Buzz revision. |

The checked-in attribution resolves the license-notice requirement. Source-history preservation is intentionally still a retirement gate: before `projects/buzz` can be deleted or declared recoverable solely from an archive, a human must approve a durable repository, tag, bundle or content-addressed archive that contains the complete source baseline and its history/provenance, and the approved immutable locator and digest must replace the local-path-only record above. Until that happens, preserve the supplied directory and every source snapshot used for migration rollback.

## Complete source-accounting ledger

The canonical source inventory and drift checker account for the complete Buzz tree by source class rather than copying it into the build:

| Ledger | Count | Coverage role |
| --- | ---: | --- |
| `catalogs/buzz-packages.csv` | 31 | Every Rust workspace package and declared binary/library target |
| `catalogs/protocol.csv` | 184 | Standard/custom protocol documents, kinds, codecs and wire surfaces |
| `catalogs/data-sources.csv` | 62 | SQL, object, Redis, credentials, desktop/archive and migration data sources |
| `catalogs/surfaces.csv` | 193 | Desktop, mobile, web, administration, deployment, script, example, benchmark and test surfaces |
| `source-inventory.md` / `reuse-audit.md` | 45 capabilities | One canonical disposition for CAP-001 through CAP-045 |
| `requirements.md` / `tasks.md` | 93 acceptance criteria / 353 leaves | Requirement and executable-task traceability for every inventory row |

`script/check-collaborative-workspace-inventory` joins every catalog row through capability, requirement and leaf coverage and fails on an unclassified source path. Generated outputs, lockfiles, vendored metadata and duplicate test fixtures remain covered by their owning component as declared in `source-inventory.md`; they are not independent preservation artifacts.

## Retained protocol and formal evidence

| Artifact set | Exact contents | Count | Aggregate SHA-256 | Disposition |
| --- | --- | ---: | --- | --- |
| Custom protocol specifications | `NIP-AA`, `AE`, `AM`, `AO`, `AP`, `CW`, `DV`, `ER`, `GS`, `IA`, `MP`, `OA`, `PL`, `PMA`, `RS`, `WP`, plus the two NIP-MP JSON fixture files under `projects/buzz/docs/nips` | 18 | `a12966cd5d345ce912593232c60fc167827118fb74c5f19cf8a0b50f1ab0cc18` | Preserve in the approved reference source; implemented wire contracts remain in `crates/nostr_compat/src/buzz_nips`. |
| Formal models | `GitOnObjectStore.{tla,cfg}`, `MultiTenantRelay.{tla,cfg}` and `MultiTenantAuth.spthy` | 5 | `52dbd432866b3f0213531d77847472c3bb00bb219ef3c8f3a592804f3e206924` | Preserve independent of production code. `.gitignore` and generated model-checker outputs are not evidence artifacts. |
| Independent Buzz conformance package | `Cargo.toml`, `LIMITS.md`, `TRACE_SCHEMA.md`, three production sources, four JSONL traces and two test sources | 12 | `3b66b5734812a6a0e253d3dd4ba3b73f755129519762bad2975d27a48b328895` | Preserve as an independent oracle; never add it as a production dependency. |
| Buzz SQL migration source | Ordered `0001` through `0030` SQL files | 30 | `fff9360000beeb631b4b47e01fd6caf2fce234623b47cf98df6e38d113a7aa6e` | Preserve for schema normalization, rollback and provenance. The checked-in migration manifest records every individual checksum. |

Each aggregate digest above is SHA-256 over the sorted, relative-path `shasum -a 256` output for the declared set. It binds both content and filenames without depending on the absolute local source path.

## Durable artifacts in this repository

| Checked-in set | Contents and purpose | Count/digest | Retirement disposition |
| --- | --- | --- | --- |
| Frozen compatibility fixtures | `fixtures/protocol` (8 files), `fixtures/migrations` (3), `fixtures/clients` (2), plus `fixtures/baselines.md` | 14 files. Manifest/baseline hashes: protocol `ac7eb19e...`, migrations `85025826...`, clients `a3ff0a40...`, baselines `ff55acc5...` | Retain permanently with their independent Python checkers. Protocol includes byte-exact copies of all four Buzz conformance traces; migration covers all 30 SQL checksums and 32 desktop fixture versions; client manifest covers all 28 frozen contracts. |
| Protocol adapters | Six tracked sources under `crates/nostr_compat/src/buzz_nips` | Included in 32-file combined digest below | Retain while any signed history/client uses the custom contracts; remove only by a separately versioned compatibility decision. |
| Migration adapters | Six tracked sources under `crates/collab/src/migration/buzz` and three under `crates/zed/src/migration/buzz` | Included in 32-file combined digest below | Retain through source/data rollback and legal retention windows; disabled from ordinary runtime ownership. |
| CLI compatibility shim | `tools/buzz_compat/Cargo.toml`, `src/buzz_compat.rs`, `src/main.rs`; shipping binary name `buzz` | Included in 32-file combined digest below | Retain and package under the supported-client matrix. Generated `target/` contents are not source artifacts. |
| Combined tracked compatibility/import set | The preceding fixtures, protocol adapters, migration adapters and shim | 32 tracked files; aggregate SHA-256 `55576467dbe87687a45c5684c507314463819030632a81217d26ab580fcfd67c` | Protected by normal Git history in this repository. |
| Visual baselines | `screenshots/screenshot-1.png` and `screenshot-2.png` | `31f179f...` and `1854c0e3...` | Retain as CAP-036 composition evidence even after desktop source retirement. |
| Compatibility policy and evidence | `docs/collaboration/compatibility.md`, the three Task 47 manifests, protocol/migration/security/scale reports and the source/catalog ledgers | Git-tracked, task-evidenced | Retain with the specification and release evidence; these documents explain when artifacts may stop shipping but are not runtime owners. |

The abbreviated hashes in the table are labels only; the authoritative full fixture and image hashes live in their checked-in manifests and `source-inventory.md`. The combined digest was computed over sorted tracked paths and file hashes. Canonical production implementation files derived from the audited behavior are not copied reference artifacts: their individual Git history, task evidence and this license notice preserve origin, while the source catalogs preserve the old-to-new mapping.

## Release and removal policy

- Every release archive that redistributes the `buzz` shim or another Buzz-derived compatibility artifact includes repository `LICENSE-APACHE`, `LICENSE-GPL`, canonical generated third-party notices and `LICENSES/buzz.md`.
- The independent source checker, formal models, protocol specifications and migration source must not become production dependencies. CI/test jobs may consume checked-in frozen fixtures without mounting `projects/buzz`; an explicit source-drift job may use the approved reference baseline.
- Protocol fixtures, signed event IDs/signatures, migration hashes and visual baselines are immutable evidence. Replacing them requires a new version and provenance record, never an in-place rewrite that disguises divergence.
- Importers and source snapshots retain no private-key plaintext in evidence. Protected credentials remain referenced through their canonical custody and receipt paths.
- Retiring a runtime does not authorize deletion of license notices, Git history, rollback snapshots, imported-record provenance or the compatibility matrix.
- The current local reference source has no durable, path-specific Git history. Therefore the final source-deletion gate is **HOLD** until an approved immutable history/archive locator exists and a restoration drill verifies its digest and completeness.

## Validation commands

```text
shasum -a 256 projects/buzz/LICENSE
find projects/buzz/docs/nips -maxdepth 1 -type f | sort
find projects/buzz/docs/spec -maxdepth 1 -type f ! -name '.gitignore' | sort
find projects/buzz/crates/buzz-conformance -type f | sort
find projects/buzz/migrations -maxdepth 1 -type f -name '*.sql' | sort
git ls-files projects/buzz
git check-ignore -v projects/buzz
git ls-files .agents/specs/collaborative-workspace/fixtures crates/nostr_compat/src/buzz_nips crates/collab/src/migration/buzz crates/zed/src/migration/buzz tools/buzz_compat
script/check-collaborative-workspace-inventory
```
