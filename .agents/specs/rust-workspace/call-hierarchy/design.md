# Design: Generic LSP call hierarchy

## Current implementation baseline

No call-hierarchy symbols were found. Existing seams are `project::lsp_store` reference/definition request routing, editor actions/navigation, and generic trees in `language_tools`. Standard LSP types should come from the existing `lsp` dependency.

## Design decisions

### D1: Add standard LSP requests beside definitions/references

Introduce prepare/incoming/outgoing request routing in `LspStore`, including local and remote serialization through existing LSP command infrastructure. Convert locations to visible project paths/ranges and preserve server opaque item data only within bounded request state.

### D2: Build a lazy generic hierarchy projection

`language_tools` owns a call-hierarchy view over opaque node IDs and standard presentation fields. Expanding a node requests one direction page. An ancestry set detects cycles. Default limits are reviewed, finite and observable when truncated.

### D3: Keep editor integration and navigation conventional

The editor action resolves the current buffer position, opens/focuses the view, and surfaces unsupported/empty states. Row activation uses existing Workspace project navigation. No Rust/Cargo dependency is introduced.

### D4: Preserve remote, cancellation and failure boundaries

Every request carries buffer/server/project generation and is cancelled on new cursor root, direction change, collapse or disconnect. Remote hosts execute their language server request; peers receive only visible locations. Malformed/late responses are isolated.

### D5: Test through fake language servers

Fake Rust and non-Rust servers provide prepare/incoming/outgoing responses, cycles, paging, delays and malformed locations. GPUI tests use executor timers and verify accessibility and bounded rendering.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2 | D1, D2 | Fake LSP request/tree tests |
| 1.3, 1.4 | D2, D3 | Navigation and GPUI accessibility tests |
| 1.5 | D2, D4 | Cycle/limit/cancellation tests |
| 1.6 | D1, D4 | Remote fake-host visibility tests |
| 1.7 | D3, D4 | State/error tests |
| 1.8 | D5 | Cross-language deterministic suite |

## Cross-pack dependencies

None. This generic feature should eventually move to a general language-navigation catalog; it is recorded here only because the Rust enhancement summary proposed it.
