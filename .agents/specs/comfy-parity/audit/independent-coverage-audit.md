# Independent native-parity coverage audit

## Result

**PASS — the frozen native Rust/GPUI specification satisfies the coverage, reconciliation, traceability, regeneration, and implementation-readiness gates audited here. No blocking defect remains.**

This is a specification result, not runtime-parity evidence. Current Sim has no
cataloged equivalent or partial Comfy implementation: the ledger records 9,667
`missing`, 835 `conflicting`, 2,123 `deferred`, and 87 `uncertain` rows. Source
runtime observation covers 103 of 12,659 independently testable rows (0.8137%).

## 2026-07-21 Task 315 implementation coverage addendum

The workspace-ownership requirement slice now has independently rerun runtime
coverage. Repository-wide scans and executable ownership tests prove one
non-cloneable CPU scratch authority, one planner-owned reduced-workspace retry
state, one non-cloneable one-use planned-authorization token, one consuming
worker adapter, and one backend binding/checking path. Tests reject duplicate
token issuance, retry or plan mutation after issuance, raw-byte authorization,
cross-backend use, over-ceiling requests, cancellation publication, OOM partial
state, and unreleased accounting.

Forward and reverse mappings for Task 315 resolve to D18, D28, D29, D32, D39,
D41 and nineteen validation scenarios. Every declared ordinary and locked
package suite, both native E2Es twice, all named validators, ownership twice,
formatting, scoped clippy, and full repository clippy passed. Current primary
artifact digests are `VAL-MEMORY-001`
`c3ff410dcf74326d45aa5cc19187dd6dc6926355bc7b5c274f9db97fdb0f47b3`,
`VAL-NATIVE-E2E-001`
`f778f093c7e36fd03572b0b747e3b6c2bcdaab403abd5107daae6d40eaba9097`,
and `VAL-NATIVE-E2E-002`
`fca775c4c07d7e6876ac32dd0c76ff147cd63cb42fdada9e4b46d3cf3c296f80`.

## Baseline reproduction

The fingerprint recipes in `baseline.md` were independently rerun against the
frozen source trees. No nested Git metadata exists, so the pack correctly uses
declared versions and deterministic fingerprints rather than invented SHAs.

| Snapshot | Files | Reproduced SHA-256 |
| --- | ---: | --- |
| ComfyUI 0.27.1 | 949 | `21de8fece20d8d5bfa94daaa52d6ccfe2db6726ca0803ca3b383ad164cbd1d5f` |
| ComfyUI-Frontend 1.48.2 | 4,697 | `aeb208b759effdacf2ea3b1929f0a3e583201f0b7b3cb006f36f1007364b8ca3` |
| Comfy-Desktop 1.0.28 | 735 | `2442854931f3a5a80e68aa55eab21a26dcefe868b4e875251a5b4d811668e448` |
| comfy-cli 0.0.0 CI placeholder | 312 | `09d0b5f262bce3105f83777a310f1e391c4624f95142da5e3230626b68a276e6` |
| Comfy docs, no declared project version | 5,800 | `1f4c9c460b8f5b35e30eb4d2d64bc201a958f247ab21af6c68743cce28c33931` |
| embedded-docs 0.5.7, with ComfyUI pinning 0.5.6 | 10,298 | `5aebf925cf36fe7b8df3c89466ad96ffa42110542a392ec6156b88fc807ec956` |
| Sim 1.10.2 fingerprint manifest | 3,310 | `99ceb40a1cc3359cde6e0865fe1b6138a06317d5fbd892f1595de10a96b07e9a` |

Each of the six source-file ledgers exactly matches its filesystem: 949, 4,697,
735, 312, 5,800, and 10,298 rows respectively, with zero missing paths, extra
paths, duplicates, or available per-file hash mismatches. The 33-input
checksum-locked source-snapshot manifest also equals its independently
recomputed value.

## Master ledger and evidence integrity

- `catalogs/features.csv` has 12,712 rows, 12,712 unique stable IDs, 39 columns,
  no blank cell, and no invalid evidence, availability, confidence, or target
  status value.
- Product totals independently reconcile: ComfyUI 3,590; Frontend 3,628;
  Frontend desktop UI 48; Frontend website 282; Desktop 1,268; CLI 1,348;
  docs 1,599; embedded docs 855; cross-product 94.
- Evidence totals reconcile: 7,042 `code-inferred`, 3,215 `test-backed`, 2,352
  `documented-only`, and 103 `observed`.
- The 142 domain counters, 306 classification counters, 12 availability
  counters, and all product/evidence/status counters exactly equal
  `catalogs/master-reconciliation.json` and each sums to 12,712.
- All 1,273 docs page records, 42 docs Cloud OpenAPI records, and 855 embedded
  node-document records remain `documented-only`; documentation was not
  promoted to executable evidence.
- All 85 CSV catalogs parse with uniform row width (68,860 data rows), all 11
  JSON catalogs parse, and all 70 relative links in the top-level spec artifacts
  resolve.
- `parity-matrix.md` and `traceability.md` each contain every feature ID exactly
  once, with no missing, extra, or duplicate row.
- Target statuses agree across all 6,064 populated status fields in subordinate
  catalogs. The stale Desktop phrase deferring target evidence to a future lead
  architecture audit is absent from both `desktop-features.csv` and its
  generator.

## Registry and source reconciliation

All 80 discovered-versus-cataloged rows in
`catalogs/master-reconciliation.json` are equal. Principal totals include:

- ComfyUI: 789 registered nodes, 12 inactive schema-bearing nodes, 120 effective
  HTTP paths represented by 141 route rows, 26 WebSocket contracts, 153
  configuration rows, 211 model/format/hardware rows, 600 tensor-operation
  contracts, 36 autograd contracts, 54 RNG contracts, 40 persisted/media
  formats, 1,010 schemas, and 217 hosted external endpoint contracts.
- Frontend: 118 commands, 34 default keybindings, 236 menus, 152 settings, 82
  routes, 24 WebSocket/local events, 149 HTTP client contracts, 43 flags, 88
  telemetry rows, 24 format/migration records, 66 persisted-state records, 59
  extension contracts, 805 component surfaces, and 352 functional-module
  candidates.
- Desktop: 273 IPC channels, 299 preload members, 45 menu actions, 26 shell
  actions, 44 window/lifecycle events, 31 settings, 36 persisted stores, 139
  telemetry rows, 19 keybindings/gestures, 6 installation source modes, 3
  platform rows, and 43 renderer surfaces.
- comfy-cli: 123 command leaves, 370 parameters, 23 schemas, 99 errors, 12
  events, 35 environment variables, 20 configuration keys, 34 formats, 24
  lifecycle contracts, 17 extension contracts, 419 CQL policy rows, 52 partner
  endpoints, 104 production modules, 1,244 capability records, and 2,295 test
  functions in the separate test ledger.
- Documentation: 1,273 content records, 896 built-in node-page reconciliation
  rows, 855 embedded node records, 42 Cloud OpenAPI operations, 65 redirects,
  108 tooling contracts, 35 configuration/format rows, 56 extension contracts,
  and 20 lifecycle contracts.

## Forward and reverse traceability

- All 12,712 features have nonblank requirement, design, task, and validation
  links. This includes all 10,060 active or conditional rows.
- The pack defines 264 acceptance criteria, 40 design decisions, 420 executable
  tasks, and 58 validation scenarios. Every canonical record is referenced by
  both feature evidence and at least one task; no unknown or reverse-orphaned
  record exists.
- The design criterion table contains all 264 criteria exactly once.
- The 420 tasks span 52 dependency waves. Task numbering and IDs are unique;
  every required leaf field is present; dependencies resolve; the graph is
  acyclic and strictly earlier-wave; all reads exist now or are produced by a
  transitive predecessor; and same-wave writes have zero file or
  directory/descendant conflicts.

Three active/conditional target contracts intentionally remain `uncertain`, but
none is unmapped: `COMFY-COMPAT-015` (open-world cloud/manager/extension REST
routes), `COMFY-COMPAT-025` (legacy/proposed WebSocket progress framing), and
`COMFY-COMPAT-029` (frontend-only notification/asset-transfer events). Their
producer-side behavior is unavailable, so the pack correctly requires evidence
before claiming parity.

## Native-only architecture and first slice

The architecture consistently assigns production execution to Sim-owned Rust
crates, a private Sim-owned Rust worker, GPUI entities/services, native
tensor/autograd/RNG/model/sampler/scheduler/media implementations, and an
optional native HTTP/WebSocket compatibility host that never forwards to
ComfyUI. Release gates prohibit Python/Comfy process paths, external Comfy
connections, JavaScript extension execution, and source-tree dependencies.
Rust/WASM plugins use versioned explicit ports, grants, resource bounds, and
deterministic legacy identifier mappings; unsupported imperative extensions
remain visible lossless placeholders.

The crate foundation predeclares eight local accelerator adapter crates as
typed-unavailable stubs. Eight separate backend-foundation leaves then own the
CUDA, ROCm, Metal, DirectML, XPU, NPU, MLU, and CoreX Cargo/build surfaces,
checked ABI declarations, symbol/layout manifests, loaders, licenses, notices,
package/signing metadata, and unavailable errors before their corresponding
kernel leaves can advertise support. Each kernel leaf depends directly on its
one foundation leaf; all eight pairs have disjoint writes and resolved reads.

Dependency and lock ownership is serialized. Task 2 creates the workspace and
initial unavailable adapter manifests; Task 31,
`comfy-parity-vendor-dependency-lock`, is the only later task that writes the
root `Cargo.toml`, `Cargo.lock`, or any adapter `Cargo.toml`. It pins all eight
dependency sets together, regenerates the lock once, and produces the planned
`native-backend-dependencies.json` ledger. Every backend-foundation leaf in
wave 32 depends on Task 31 and reads that ledger. A scan of all later waves
found zero authorized Cargo manifest or lockfile writer.

The first end-to-end slice is task 19, `comfy-parity-native-execution-e2e` in
wave 19: native LoadImage, ImageScale, ImageInvert, PreviewImage, and SaveImage,
including Rust PNG/tensor execution, worker events, GPUI output, caching,
cancellation, output transactions, and crash recovery. The deterministic native
diffusion slice follows as task 30.

`catalogs/native-diffusion-fixture.json` pins the second slice to SD15, the SD15
latent format, Euler, the normal scheduler, and the exact six node feature IDs.
All 10 referenced feature IDs exist. Its 19 required checkpoint names are
unique, its 64-bit seed is fixed, and the declared CLIP config, tokenizer vocab,
and tokenizer merges SHA-256 values exactly match the frozen ComfyUI source
files. The reduced executable artifact is test-support-only; production family
detection still requires the source-shape detector projection.

## Regeneration and strict validation

An exact copy of the frozen pack was regenerated outside the workspace using
the repository interpreter:

```sh
/usr/bin/python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice
```

It exited 0 after both passes and ended with:

```text
Comfy parity regeneration pipeline completed with snapshot-input closure.
```

The required command was then run exactly in the repository:

```sh
python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/comfy-parity --require-complete
```

It exited 0 with exact output:

```text
Validated spec pack: .agents/specs/comfy-parity
```

## Explicit uncertainty and exclusions

No production capability is silently excluded. Runtime validation did not use
real accounts, credentials, paid services, model downloads, external mutation,
unavailable accelerators, Windows/Linux hosts, or dynamic extensions absent
from the snapshots. Missing dependencies also prevented most source suites from
being rerun. Those limitations remain explicit as code/test/documentation
evidence, deferred decisions, or uncertainty. They do not block specification
execution, but they do block any claim that native runtime parity already
exists.

The audit modified only this report. It did not change source applications,
Sim implementation code, dependencies, Git state, or external systems.
