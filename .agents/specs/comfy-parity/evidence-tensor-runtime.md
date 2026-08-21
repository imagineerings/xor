# Native tensor-runtime evidence

## Scope and baseline

This evidence pack statically inventories the tensor/operator, autograd, and random-number surfaces that the pinned ComfyUI source uses and turns each surface into an explicit native Rust conformance obligation. Production Zed may use ComfyUI only as a development-time oracle. None of these rows authorizes a Python runtime, a PyTorch process, JavaScript execution, or an external ComfyUI dependency in production.

The source baseline is ComfyUI `0.27.1` with `949` regular files and all-file fingerprint `21de8fece20d8d5bfa94daaa52d6ccfe2db6726ca0803ca3b383ad164cbd1d5f`. The generator scanned `683` Python files and reconciled every one against [`catalogs/backend-source-coverage.csv`](catalogs/backend-source-coverage.csv); it does not duplicate the canonical 949-row source closure. `683` files parsed directly with the host AST. `0` file used syntax-only normalization of Python 3.10 `match`/`case` headers so Python 3.9 could preserve and inspect the original call expressions and line numbers. The canonical source catalog's `infrastructure-only` label can mean an internal implementation-support module with no independently named feature row. Calls in executable product paths are therefore counted as production execution evidence while the original source classification remains preserved on each row; only `.ci`, `.github`, and `script_examples` Python is placed in the support tier.

## Evidence method

The generator resolves imports, aliases, direct PyTorch/ecosystem calls, decorators, types, namespaces, constants, and a bounded Tensor-method vocabulary. A direct imported call is high-confidence static evidence. A method whose receiver flows from an annotated Tensor or a resolved tensor-producing call is medium-confidence. A same-named method whose receiver cannot be proven is retained as a low-confidence candidate. This prevents both silent omission and false claims of certainty.

Existing tests are linked when they directly call the same symbol. Such a link raises the row to `test-backed`, but does not prove that a test covers every production shape, dtype, device, numeric, gradient, cancellation, or error variant. No tensor/model runtime was loaded and no row is classified `observed`.

## Tensor/operator reconciliation

[`catalogs/backend-tensor-operations.csv`](catalogs/backend-tensor-operations.csv) contains `600` symbol rows. It separates `511` callable operations from `15` type rows and `67` namespace/value rows. Its recorded call sites reconcile to `15717` production, `262` test, and `1` support calls. There are `94` low-confidence receiver-unverified rows; their exact candidate sites remain visible and require type/call-graph confirmation before implementation closure.

| Semantic group | Rows |
|---|---:|
| accelerated-attention-kernel | 4 |
| activation-normalization-functional | 10 |
| comfy-operator-indirection | 19 |
| elementwise-or-runtime-operation | 266 |
| external-tensor-kernel | 31 |
| indexing-masking | 14 |
| linear-algebra | 15 |
| namespace-contract | 3 |
| neural-network-functional | 12 |
| neural-network-module | 43 |
| random-number-generation | 13 |
| reduction | 25 |
| shape-layout-transform | 29 |
| spatial-functional-kernel | 12 |
| spectral-transform | 4 |
| storage-dtype-device | 11 |
| tensor-creation | 10 |
| type-contract | 15 |
| value-or-constant-contract | 64 |

Every row carries native shape, dtype, layout, device, numerics, VJP/JVP, and cancellation requirements. The implementation boundary is a Zed-owned `comfy_tensor` facade. A selected compute crate may sit behind that facade, but its types, handles, serialization, and backend assumptions cannot become workflow or Rust/WASM plugin ABI.

## Autograd reconciliation

[`catalogs/backend-autograd.csv`](catalogs/backend-autograd.csv) contains `36` rows, including `7` explicit `torch.autograd.Function` subclasses. The catalog records forward/reverse method signatures, same-file `.apply` sites, gradient modes, graph detachment, gradient state, hook/retention behavior, and receiver uncertainty. Uses reconcile to `382` production, `1` test, and `0` support sites.

| Autograd construct | Rows |
|---|---:|
| activation-checkpointing | 1 |
| custom-autograd-function | 7 |
| custom-function-context | 4 |
| gradient-mode | 3 |
| gradient-state | 3 |
| graph-detachment-or-storage-alias | 2 |
| mixed-precision-autograd | 3 |
| optimizer-or-gradient-scaler | 10 |
| reverse-mode-execution | 3 |

Native parity requires graph ownership, saved-tensor lifetimes, broadcasting reduction in VJPs, in-place version checks, None gradients, hook order, repeated backward, finite-difference checks, worker cancellation, and recovery without partial gradient publication. Forward-only implementations are acceptable only after reachability evidence proves a row cannot participate in a cataloged autograd path.

## Phase-scoped RNG reconciliation

[`catalogs/backend-rng.csv`](catalogs/backend-rng.csv) contains `54` rows keyed by mechanism, resolution, and semantic phase. Calls reconcile to `134` production, `46` test, and `0` support sites.

| RNG phase | Rows |
|---|---:|
| context-window-selection | 3 |
| model-internal-stochasticity | 16 |
| node-level-noise | 3 |
| runtime-utility | 8 |
| sampling-noise-and-solver | 8 |
| stochastic-quantization | 4 |
| temporary-output-naming | 1 |
| test-fixture | 5 |
| training-and-data-order | 6 |

Zed must not use a process-global RNG as an implicit compatibility mechanism. Each row requires a versioned phase identity derived from workflow seed, node identity, execution ordinal, phase, sample or batch index, and declared retry policy. Cancellation, validation failure, OOM retry, and worker recovery may not commit partial RNG advancement. CPU-seeded transfer and native-device generation remain distinct contracts because they can produce different observable sequences.

## Boundaries and limitations

- The inventory is static `code-inferred` or direct-symbol `test-backed` evidence. It does not demonstrate runtime branch reachability, dynamic monkey-patching, actual accelerator kernels, overload selection, or numerical equivalence.
- Dynamic operator selection and calls through arbitrary variables cannot be resolved in general. Receiver-unverified Tensor-method candidates are explicit rows with low confidence.
- NumPy arithmetic, SciPy, PIL, OpenCV, media codecs, and model-container parsing are handled by other parity domains. Python and NumPy random calls are included here because phase RNG affects deterministic behavior.
- External accelerated attention, torchvision, torchaudio, kornia, einops, torchsde, and device-extension calls are obligations to reproduce or reject with source-compatible errors; their presence does not authorize those Python packages in production.
- Direct test-symbol matches are not semantic coverage. Native conformance fixtures must exercise success, boundaries, invalid shape/dtype/device, special values, cancellation, retry, worker crash, persistence, and backend variance.

## Generated artifacts

| Artifact | Rows | SHA-256 |
|---|---:|---|
| [`catalogs/backend-tensor-operations.csv`](catalogs/backend-tensor-operations.csv) | 600 | `7f2f90249fe6d4413aaade485d6197359818cc0c2feb47df73c56d25283f11dc` |
| [`catalogs/backend-autograd.csv`](catalogs/backend-autograd.csv) | 36 | `d51ff8465e2a161bef2093bbdb37f7547a6d6157d0fa1c4d6f0a30b8fd682670` |
| [`catalogs/backend-rng.csv`](catalogs/backend-rng.csv) | 54 | `d207ea66d8949eb73067828da6f2ed160ab8bdf641b4cf6ed1789faa0f65d06b` |
| [`catalogs/backend-tensor-runtime-reconciliation.json`](catalogs/backend-tensor-runtime-reconciliation.json) | reconciliation | generated deterministically |
