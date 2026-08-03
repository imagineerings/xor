# Artifact consistency audit

## Result

**PASS — no material artifact-consistency blocker remains in the frozen
native-Rust/GPUI specification snapshot.**

This is an independent structural, regeneration, reference, count, source
closure, native-boundary, and validator audit. It does not assert that Sim has
implemented the plan or achieved runtime parity. The master ledger correctly
reports zero equivalent and zero partial rows; unavailable runtime, hardware,
platform, account, and provider observations remain uncertainty rather than
being promoted by this result.

## 2026-07-21 Task 315 implementation addendum

The planning-snapshot qualification above remains applicable to the catalog
status ledger, but Task 315 now also has fresh implementation evidence for the
completed workspace-ownership slice. A whole-repository definition and
call-site audit found one `CpuWorkspaceAuthority`, one one-use
`PlannedWorkspaceAuthorization` issuer, one consuming `WorkerSession` adapter,
and one backend bind path. It found no production zero-scratch constructor,
raw-byte worker authorizer, legacy workspace context, alternate scratch issuer,
or superseded allocation wrapper. The 99-row ownership catalog is closed, and
`VAL-OWNERSHIP-001` passed twice with SHA-256
`6ea17a1cb984281e0415b81c3990663457b34e7898ec1564cc2e9f74bd8051d4`.

Both locked and ordinary all-target suites passed for `comfy_tensor`,
`comfy_model`, `comfy_sampler`, `comfy_media`, `comfy_runtime`, `comfy_worker`,
and `comfy_test_support`; the feature-enabled `comfy_ui` suite passed as well.
Native image passed twice, and native diffusion passed twice in 198.34 and
205.06 seconds with artifact SHA-256
`fca775c4c07d7e6876ac32dd0c76ff147cd63cb42fdada9e4b46d3cf3c296f80`.
All nineteen mapped validators, exact seven-package clippy, repository-wide
release/all-target/all-feature clippy with warnings denied, and formatting
passed on macOS aarch64 CPU. Optional accelerator hardware is not claimed by
this addendum.

## Commands and fixed-point result

The required validator was run exactly against the deliverable:

```sh
python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/comfy-parity --require-complete
```

It exited `0` and printed exactly:

```text
Validated spec pack: .agents/specs/comfy-parity
```

The pack was then copied to an isolated `/tmp` workspace with read-only links
to the pinned source trees. The canonical pipeline was run there so that this
audit did not mutate the deliverable:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice
```

It exited `0`. Therefore the checked-in artifacts equal one canonical
regeneration, a second regeneration is byte-stable, and
`catalogs/source-snapshot-manifest.json` agrees before and after the pipeline.
All 11 Python generators parse as Python AST, all 11 JSON files parse, and all
85 CSV catalogs have a uniform row width. The CSV catalogs contain 68,860 data
rows in total.

## Baseline and source-tree closure

The documented recipes independently reproduced every baseline fingerprint:

| Source | Files / inputs | Reproduced SHA-256 |
| --- | ---: | --- |
| ComfyUI 0.27.1 | 949 | `21de8fece20d8d5bfa94daaa52d6ccfe2db6726ca0803ca3b383ad164cbd1d5f` |
| ComfyUI-Frontend 1.48.2 | 4,697 | `aeb208b759effdacf2ea3b1929f0a3e583201f0b7b3cb006f36f1007364b8ca3` |
| Comfy-Desktop 1.0.28 | 735 | `2442854931f3a5a80e68aa55eab21a26dcefe868b4e875251a5b4d811668e448` |
| comfy-cli 0.0.0 CI placeholder | 312 | `09d0b5f262bce3105f83777a310f1e391c4624f95142da5e3230626b68a276e6` |
| Comfy documentation | 5,800 | `1f4c9c460b8f5b35e30eb4d2d64bc201a958f247ab21af6c68743cce28c33931` |
| Comfy embedded docs 0.5.7 | 10,298 | `5aebf925cf36fe7b8df3c89466ad96ffa42110542a392ec6156b88fc807ec956` |
| Sim 1.10.2 manifest | 3,310 | `99ceb40a1cc3359cde6e0865fe1b6138a06317d5fbd892f1595de10a96b07e9a` |

For all six Comfy source trees, the filesystem path set exactly equals its
source ledger: no missing path, extra path, or duplicate path. The backend and
comfy-cli per-file SHA-256 and byte counts match source; Frontend byte counts
also match. Production-like source rows all resolve to master feature IDs:
476 backend, 1,800 Frontend, 292 Desktop, and 137 comfy-cli rows. The docs and
embedded-docs ledgers have no blank disposition or reason. The 970 backend test
rows and 2,295 comfy-cli test rows have no unresolved coverage mapping according
to the independently recomputed master closure.

The filesystem contains 14 applicable nested source instruction files: one
Desktop `AGENTS.md`, the Frontend root plus ten scoped `AGENTS.md` files, the
docs `AGENTS.md`, and embedded-docs `.cursorrules`. `baseline.md` names all 14
and records that comfy-cli has no nested instruction file.

## Master catalog and derived-artifact agreement

`catalogs/features.csv` has 12,712 rows, 12,712 nonblank IDs, and no duplicate
master ID. Every one of its 12,712 `(source_catalog, source_row)` pointers
resolves, no two master features claim the same pointer, and every available
subordinate `sim_status`, `target_status`, or `current_sim_status` equals the
master status.

The master counters independently reproduce
`catalogs/master-reconciliation.json`:

- Status: 9,667 missing, 835 conflicting, 2,123 deferred, 87 uncertain,
  0 equivalent, and 0 partial.
- Evidence: 7,042 code-inferred, 3,215 test-backed, 2,352 documented-only,
  and 103 observed.
- Product totals: 3,590 ComfyUI; 3,628 main Frontend; 48 Frontend desktop-ui;
  282 Frontend website; 1,268 Desktop; 1,348 CLI; 1,599 documentation;
  855 embedded documentation; and 94 cross-product.
- Runtime validation: 103 observed of 12,659 independently testable rows,
  or 0.8137 percent.
- Registry reconciliation: all 80 discovered-versus-cataloged rows are equal.
- Traceability summary: 12,712 of 12,712 feature rows have every required link.

The schema-detail reconciliation is also exact: all 141 HTTP rows retain
request, response, and explicit unresolved detail; all 273 Desktop IPC rows
retain request/event, response/callback, and unresolved detail; and all 299
preload rows retain a source signature and unresolved detail.

`parity-matrix.md` and `traceability.md` each contain exactly one row for every
master feature ID: 12,712 rows, no missing ID, extra ID, duplicate ID, or blank
data cell. All 70 local Markdown links in non-audit artifacts resolve.

## Requirement, design, task, and validation references

The pack has 44 requirements, 264 acceptance criteria, 40 design decisions,
420 task leaves, and 58 validation scenarios. Their sets exactly equal
`catalogs/native-spec-mapping.json`. Every master feature reference resolves;
every criterion, design decision, task, and validation is reverse-referenced;
and every mapping-JSON feature/task/design/validation reference resolves. The
design criterion table has one row for each of the 264 criteria, with no
duplicate or orphan.

The 420 task leaves span waves 1 through 52. Every leaf has Outcome, Wave,
Dependencies, Reads, Writes, Requirements, Design, Validation, and Done when.
Every dependency names a preceding task in a strictly earlier wave. Prefix-aware
directory/file comparison finds no overlapping expected writes in a shared
wave. Every read path either exists now or overlaps output from a preceding
task. In particular, `comfy-parity-three-d-latent-content` reads the generated
latent-format directory; the obsolete nonexistent
`crates/comfy_model/src/latent.rs` path is absent. No task writes under
`projects/comfy`, no Rust filename contains a hyphen, and the obsolete
`comfy-cli-flags.csv` path is absent. Task validation commands use
`./script/clippy`; the only literal `cargo clippy` occurrence is the opening
prohibition.

Exactly two serialized tasks own Cargo manifests or `Cargo.lock`:
`comfy-parity-native-crate-foundation` creates the workspace, local adapter
stubs, non-vendor dependency lists, and initial lock; later,
`comfy-parity-vendor-dependency-lock` writes all eight adapter dependency lists
and regenerates the lock once. No later task writes a manifest or lockfile. The
eight accelerator ABI-foundation tasks all depend directly on the vendor-lock
task, occupy disjoint adapter/package/ABI paths in wave 32, forbid dependency
changes, and use `--locked` validation. Their kernel and hardware-certification
leaves also use `--locked`; no backend task can race or implicitly mutate the
lock.

## Concrete native-contract artifacts

`catalogs/native-diffusion-fixture.json` parses and has one authoritative
`sd15-tiny-v1` contract. Its ten referenced model, latent, sampler, scheduler,
and node feature IDs all exist and name the intended SD15, Euler, normal, and
six-node contracts. Its positive and negative token RLE each expand to 77 IDs;
its 19 checkpoint paths are unique; its CPU/f32, 32x32, batch-one, four-step,
seed, CLIP, UNet, VAE, latent, detector, key-prefix, weight-generation, source
fingerprint, and test-support-only reduced-artifact rules are nonblank. The
diffusion-foundation task writes the fixture directory and requires every named
checkpoint; the next E2E task consumes the same JSON and directory.

Design D35 now contains a concrete Rust source trait and
`sim:comfy-plugin@1.0.0` WIT world. The WIT contract covers explicit port
direction/type/cardinality/presence, indexed singular/list input transfer,
push-plus-finish scalar/tensor/artifact/model outputs, absent optional versus
empty list, ownership revocation, cancellation, and bounded filesystem,
network/provider, secret, clock, randomness, model, transactional-output,
sanitized-log, declarative-UI, and route capability interfaces. The plugin task
and `VAL-PLUGIN-001` enumerate the corresponding allow/deny, quota, malformed,
trap, hang, cancellation, rollback, and legacy-mapping cases.

The target design and task graph also separate eight accelerator ABI/package
foundations from kernel breadth. Each foundation names its adapter crate,
dependency decision, ABI floor, targets, required libraries, discovery order,
symbol/layout contract, SDK/header digest, unsafe boundary, license/signing
rule, unavailable stub, package outputs, and validation evidence before its
kernel task can start.

## Native-only synchronization

`master-reconciliation.json` fixes all four production compatibility switches
to `false`: Python Comfy process, external Comfy connection, Python extension
execution, and JavaScript extension execution. It names the replacement as a
versioned Rust source trait plus WASM Component Model WIT with explicit ports
and deterministic legacy mappings.

Prescriptive target and evidence text was searched for the superseded managed,
bundled, external-server, browser-host, and hybrid recommendations. Remaining
mentions are source behavior, negative migration requirements, development-only
oracle steps, or explicit external website/provider navigation; none authorizes
a production ComfyUI/Python dependency. Direct source rows such as
`COMFY-DESKTOP-005` and `COMFY-DESKTOP-074` are `conflicting` and require
inactive migration/native lifecycle behavior. The extension family separates
ten incompatible Python/JavaScript contracts as `conflicting` while retaining
natively implementable path, metadata, schema, progress, cache, replacement,
and offline contracts as native missing/deferred work.

## Residual limitations, not consistency failures

Base ComfyUI and base Frontend catalogs are checksum-locked source snapshots;
their original extractors are not checked in. The canonical pipeline states
this limitation and does not claim to have rerun those absent extractors.
Future source-baseline refreshes therefore require an explicit snapshot refresh
and identity-preservation review. Hardware-, account-, provider-, platform-,
and dependency-gated runtime observations likewise remain pending. These
limitations are explicit in the pack and do not create an unexplained blank,
false evidence promotion, or broken artifact reference in this snapshot.

## Handoff

The frozen pack passes artifact-consistency and strict-spec validation with no
material blocker found. This PASS is limited to the specification artifacts;
implementation-readiness judgment and runtime conformance remain separate
audits and delivery gates.
