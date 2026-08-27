# Design: Rust tools platform boundary

## Current implementation baseline

`zed/rust-tools = [dep:cargo_ui, tasks_ui/rust-test-actions]`; `project` gates `cargo-workspace`, `structured-execution`, `rust-tests`, and `rust-coverage`; `remote_server/rust-tools` selects the Rust test and coverage providers. Zed imports/initializes menus/panels only under cfg. HeadlessProject constructs/registers the feature-gated stores only under its feature. `script/check-rust-tools-feature-boundary`, CI checks, bundle dry runs and enabled/disabled tests enforce the boundary. Protobufs remain inert and compiled.

## Design decisions

### D1: Treat the project host as sole command authority

Local and remote/headless projects instantiate the same store contracts around their existing environment/trust services. Multiplayer peers receive filtered bounded projections. WSL/dev-container work only through existing project representations. There is no provider-local filesystem or process fallback.

### D2: Apply trust and privacy before command construction

Trust gates every probe/discovery/action. Revocation changes generation and cancels owned tasks. Protocol conversion strips host paths/raw output/env. Logs and diagnostics use existing bounding; persisted state contains only non-secret IDs/filter/selection.

### D3: Keep the compile-time boundary narrow and additive

`cargo_ui` is optional to Zed. Project Cargo/test dependencies are optional features. Generic `language_tools` remains Cargo-free. Desktop/settings/menu/headless request registrations share feature selection. Protobuf removal is disproportionate and provides no runtime benefit, so inert messages remain.

### D4: Do not conflate rust-tools with all Rust language support

`languages::init`, Rust grammars, rust-analyzer and pre-existing task discovery stay unconditional under current architecture. A future broader distribution feature requires its own dependency audit/spec. Naming remains `rust-tools`, `cargo_workspace`, `cargo_ui`, `language_tools`; no `metal_cargo`.

### D5: Validate release parity and mismatch explicitly

The boundary script parses manifests/source and locked normal graphs, checks core and enabled bundle plans, and CI checks both Zed and remote variants. Capability/version mismatch is a terminal UI state for that feature, not a retry/fallback signal.

### D6: Graduate only with integrated hermetic evidence

The remaining comprehensive fixture composes owner-pack fixtures. Dashboard and structured packs establish reviewed budgets; this pack consumes them in release gates. Rust-test physical cells prove production routing. Manual accessibility scripts cover focus order, role/name/state announcement, expansion, selection, status changes and action-disabled reasons.

## Persistence, failures and compatibility

Existing workspace/settings migrations remain additive. Disabled builds instantiate no Rust workspace persistence/settings registration. Disconnect/trust/mismatch preserves only safe stale state. Older/incompatible hosts leave unrelated editor features usable. Inert protocol types do not imply capability.

## Cross-pack dependencies

- `cargo-dashboard/2.1`, `2.2`, and `2.3` provide the comprehensive fixture and dashboard budgets.
- `structured-execution/2.1` and `2.2` provide generic result budgets.
- `rust-test-explorer/2.1` and `2.2` provide the physical discovery/action matrix and release integration.
- `cargo-execution/3.1` provides dedicated configuration coexistence evidence.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3, 1.4 | D1 | Existing local/headless/peer tests; new physical matrix |
| 1.5, 1.7, 1.8 | D1, D5 | Existing mismatch/generation/registration tests |
| 1.6 | D2 | Existing proto round-trip/privacy assertions |
| 2.1, 2.2, 2.3 | D2 | Existing trust/offline/no-installer tests |
| 2.4, 2.7, 2.8 | D2 | Existing redaction/malformed/bounds tests |
| 2.5, 2.6, 2.9 | D1, D2 | Existing debounce/stale/optional-provider tests |
| 3.1, 3.2, 3.3 | D3, D5 | Boundary script, enabled/disabled builds/tests |
| 3.4 | D3 | Dependency/source audit |
| 3.5, 3.6 | D3, D5 | Remote build/handler tests and proto build |
| 3.7 | D4 | Boundary source assertion |
| 3.8 | D5 | CI and bundle dry-run checks |
| 4.1, 4.2, 4.3, 4.4 | D6 | New integrated fixture/budgets/physical/accessibility gates |
| 4.5 | D5, D6 | Existing and retained hermetic CI combinations |

## Remaining delta

The integrated fixture, deterministic budget gates, hermetic local matrix and manual certification documents are implemented. D6 still requires dated production SSH/WSL/development-container/multiplayer results plus physical VoiceOver and NVDA results. D1–D5 are verified baselines and must not be rewritten as a new platform layer.
