# Requirements: Generic LSP call hierarchy

## Purpose and status

This Next pack owns standard LSP call hierarchy for all languages. Rust receives it from rust-analyzer; this is not a Rust semantic engine and is not gated by `rust-tools`. Repository audit found no `CallHierarchy` implementation.

Canonical IDs are `call-hierarchy/<criterion>`.

### Requirement 1: Provide bounded standard LSP call hierarchy [Not started / Next]

#### Acceptance criteria

1. **1.1** WHEN the active language server advertises standard call-hierarchy support and the cursor resolves, THE editor SHALL offer Show Call Hierarchy and request `prepareCallHierarchy` through the existing LSP store.
2. **1.2** THE view SHALL show callers or callees as a lazy finite tree using standard incoming/outgoing LSP requests and SHALL NOT build a local call graph or Rust-specific index.
3. **1.3** WHEN a row is activated, THE workspace SHALL navigate to its visible location/range using existing project navigation and SHALL label unavailable/outside-visible locations safely.
4. **1.4** THE view SHALL provide keyboard traversal, focus, selection, expansion, scrolling, accessible role/name/state labels, refresh and direction switching consistent with existing Zed tree surfaces.
5. **1.5** Requests SHALL be cancellable, generation-bound and depth/node/page bounded; cycles SHALL render as non-expandable references rather than recursive loops.
6. **1.6** FOR remote/multiplayer projects, requests SHALL execute through the authoritative language-server/project transport and return only peer-visible locations; no client-local language server fallback is allowed.
7. **1.7** Unsupported, empty, partial, stale, disconnected, cancelled and LSP-error states SHALL be distinct and SHALL preserve unrelated editor behavior.
8. **1.8** Tests SHALL use fake language servers for Rust and at least one non-Rust language, cycles, malformed responses, remote routing, cancellation, accessibility and a bounded large hierarchy.

## Non-goals

No Rust call-graph database, whole-workspace indexing, richer usages replacement, Cargo dependency graph, public provider API, or rust-tools dependency.

## Open questions

1. **Initial direction and surface.** Recommended default: a reusable `language_tools` tree view opened from the editor with Incoming Calls selected and lazy direction switching. UI task 2.1 depends on the final presentation choice.
