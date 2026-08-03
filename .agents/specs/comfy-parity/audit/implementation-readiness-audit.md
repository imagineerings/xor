# Independent native implementation-readiness audit

## Result

**PASS — no material implementation-readiness blocker was found in the final frozen snapshot.**

This verdict supersedes the earlier FAIL report. The repaired pack gives each material native subsystem an evidence-bearing contract, an owning Rust/GPUI component, dependency-ordered write ownership, observable acceptance criteria, and runnable validation. A PASS means the plan can enter task execution; it does not claim that planned code or runtime parity has already been delivered.

## 2026-07-21 Task 315 implementation audit

Task 315 is implemented and validated, not merely ready. The prior completion
claim was reopened because it allowed a cloneable retry controller and a
caller-forgeable raw-byte worker authorization bridge. The repair introduces a
non-cloneable, one-use planned-workspace token; freezes planner and retry state
after issuance; and consumes the token only in the paired `WorkerSession`
adapter before `CpuWorkspaceAuthority` performs the sole ceiling check and
`CpuBackend` performs the sole bind.

Independent whole-repository scans found no competing definition or production
call path. Ordinary and locked package suites, the feature-enabled GPUI suite,
ownership twice, native image twice, native diffusion twice, all named
validators, exact scoped clippy, repository-wide clippy, and formatting all
exit successfully. The implementation audit therefore finds no remaining
workspace-ownership or historical-evidence gap; later parity tasks and
hardware-specific certification remain separate executable gates.

## Strict validator

The required command was run exactly against the final frozen snapshot:

```sh
python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/comfy-parity --require-complete
```

Exact output:

```text
Validated spec pack: .agents/specs/comfy-parity
```

Exit status: `0`.

## Structural and task-graph audit

- Requirements/design/validation records: **44 requirements**, **264 acceptance criteria**, **40 design decisions**, and **58 validation scenarios**.
- Feature ledger: **12,712 rows**; no row has an empty requirement, design, task, or validation mapping.
- Tasks: **420** across **52 waves**, with **420 unique IDs**.
- Every task has Outcome, Wave, Dependencies, Reads, Writes, Requirements, Design, Validation, and Done when.
- Every dependency resolves and points to an earlier wave; no cycle or wave inversion was found.
- Same-wave write checks found **0** exact or directory/descendant conflicts.
- Missing-path analysis found **0** unresolved Reads after transitive producers were considered.
- No placeholder validation text, no `cargo clippy`, and no ellipsis in assigned feature lists was found. The catalog-only regeneration leaf is the sole task without `./script/clippy`.
- Exact node-family ownership is available in `native-spec-mapping.json` and repeated in each node leaf's done condition.

## Native compute and runtime contracts

- D25 defines checked descriptors, immutable shared storage, unique write leases/copy-on-write, backend-neutral storage handles, fences, streams, cancellation, and typed errors without exposing a third-party tensor framework.
- The 600-row tensor discovery ledger is not misrepresented as an overload table. Task 7 must resolve 518 callable candidates, classify 82 type/namespace/value contracts, and resolve or block all 104 receiver-unverified rows before downstream compute work. Its generated implementation ledger owns signatures, promotion, shape/layout, aliasing, numerics, autograd, RNG, fixtures, and tolerances.
- Autograd, RNG, memory reservation/eviction/OOM, 94 model-family descriptors, safe formats, patch graphs, 44 samplers, 9 schedulers, 33 latent formats, conditioning, and per-node execution are split into testable native leaves with serialized closure tasks.
- The private Rust worker has concrete framed IPC, version/capability handshake, identity and sequence validation, size limits, heartbeat, output prepare/commit, cancellation, graceful shutdown, process-tree orphan control, crash recovery, and visible GPUI error ownership.
- Caching includes implementation, demanded dependencies, artifacts/patches, backend/dtype, plugin API/digest, RNG phase, configuration, and compatibility identity. Effects are prepare/commit transactions and late cancelled results cannot become success.

## Rust/WASM plugin contract

The prior ABI blockers are resolved:

- First-party plugins have explicit `RustComfyPlugin` and `RustNodeInstance` source traits; no stable Rust dylib ABI is promised.
- `sim:comfy-plugin@1.0.0` uses a host-owned invocation and explicit port IDs. `input-info` distinguishes absent optional input, present empty list, singular values, and non-empty lists.
- Indexed reads/takes and push-plus-finish writes cover scalar, tensor, artifact, and model values for required, optional, singular, and list cardinalities. Transfer, bounds, finish validation, use-after-take, cancellation, and terminal revocation are specified.
- The canonical `namespace:name@major` type-ID registry fixes value family, wire schema, aliases, publisher namespace ownership, additive minor negotiation, major-version representation changes, and unknown-type placeholders.
- Versioned bounded capabilities cover typed-root filesystem reads, network/provider calls, secrets, clocks, randomness, model handles, transactional outputs, sanitized logs, declarative UI state, and routes. Grants, bounds, quotas, redaction, idempotency, cancellation, rollback, and no-late-effect rules are explicit.
- Task 11 and `VAL-PLUGIN-001` compile Rust and WIT fixtures and exercise port fields, list/optional behavior, every capability's allowed and denied path, ownership, quotas, traps, hangs, cancellation, mappings, and workflow preservation.

## Deterministic native vertical slices

The first slice is a native five-node image workflow with exact early-registry membership, CPU tensors, native PNG decode/encode, scale/invert kernels, Rust worker events, GPUI output, cache hit/invalidation, output transactions, cancellation, worker kill/restart, keyboard inspection, and isolated no-network/no-Python/no-source-tree checks.

The second slice is no longer an unspecified “tiny diffusion” choice. `catalogs/native-diffusion-fixture.json` pins:

- SD15 `COMFY-MODEL-0117`, SD15 latent `COMFY-MODEL-0045`, Euler `COMFY-MODEL-0179`, and normal scheduler `COMFY-MODEL-0209`;
- exact six node feature IDs, CPU f32, 32×32, batch 1, four steps, CFG 7, denoise 1, and seed;
- verified source config/tokenizer digests and exact token IDs;
- reduced CLIP, epsilon UNet, AutoencoderKL topology, key prefixes/sentinels, deterministic weight generation, latent constants, and every required detector/token/conditioning/sigma/RNG/noise/denoiser/latent/VAE/PNG/event checkpoint.

The reduced artifact is admitted only by test support with a production-detector transcript; it cannot become a user checkpoint format or replace real SD15 detector tests. Tasks 29 and 30 own the fixture materialization and native Rust worker/GPUI E2E without family, tokenizer, sampler, scheduler, or latent substitution.

## Backend dependency, ABI, and packaging ownership

- Task 2 alone creates the base crates, path dependencies, forwarding feature stubs, initial `Cargo.lock`, worker binary target, and dependencies needed by the pre-accelerator waves.
- One later serialized `comfy-parity-vendor-dependency-lock` task owns the only subsequent manifest/lock mutation. It pins every adapter dependency and writes `native-backend-dependencies.json` before parallel ABI work.
- Later worker/CLI, ABI, kernel, and certification leaves are not authorized to mutate dependency sections or `Cargo.lock`; their validation uses `--locked`.
- D27 fixes a per-backend binding/load strategy, ABI/SDK floor, supported targets, library discovery order, package payload/license/signature policy, and typed unavailable behavior for CUDA, ROCm, Metal, DirectML, XPU, NPU, MLU, and CoreX.
- Each accelerator has a serialized ABI-foundation leaf owning its disjoint adapter source/build/ABI/package files and a generated symbol/signature/layout ledger before kernel implementation. Hardware certification cannot promote an unavailable or legally unverified backend.

## Sim/GPUI repository fit

- Root workspace and `crates/sim/Cargo.toml` registration, descriptive crate roots, the separate worker binary, Sim CLI entry point, `sim.rs`, application menus, and later Desktop integration module all have explicit owners.
- The graph is a registered serializable workspace item. Queue/history/execution, assets, settings, operations, logs, viewers/editors, and lifecycle surfaces map to GPUI entities, dock panels, modals/popovers, application services, background tasks, or managed child processes.
- Keymap assets are explicitly loaded through Sim initialization; action/menu registration and the production accessibility constructor path are tested.
- Settings/default content and the central Settings Editor are updated through their existing crates. Runtime/profile/workflow/attempt/cache/journal persistence uses registered settings and DB domains with migrations and restart/crash tests.
- Expensive parsing, indexing, hashing, layout, codec, model, and execution work stays off the GPUI foreground thread. Task handles/cancellation lifetimes and visible error propagation are explicit.
- Numeric performance, responsiveness, cancellation, worker readiness, preview latency, and resource-convergence budgets are measurable and release-gated.

## Native-only boundary and retained gates

Searches of the planning artifacts found no production Python/ComfyUI process, external-Comfy proxy, JavaScript/Node extension host, or browser execution fallback. Public HTTP/WebSocket and `sim comfy` are native projections over the same Rust services used by GPUI. Source applications remain development-only conformance oracles and release tests consume recorded fixtures without source trees or network access.

The following are honest implementation/release gates rather than planning blockers:

- receiver-unverified tensor rows must resolve in Task 7 before compute breadth;
- vendor SDK/header/license verification and hardware certification may leave a conditional accelerator unavailable;
- account-, paid-, cloud-, and platform-specific runtime observation remains conditional on authorized environments;
- per-node, model, operator, platform, accessibility, security, recovery, and performance closure must pass before an equivalence decision is promoted.

## Handoff decision

The specification is ready to enter execution in wave order. The recommended first delivered slice remains the native CPU image workflow, followed by the pinned SD15 diffusion fixture. Accelerator availability, provider integrations, and parity labels must remain at their recorded conditional or uncertain status until their implementation and certification gates produce evidence.
