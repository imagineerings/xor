# Design: Cargo execution presets and actions

## Current implementation baseline

`crates/cargo_ui/src/cargo_preset.rs` parses bounded versioned settings into `CargoPreset`, migrates version 1 to version 2, merges trusted project over user content, compiles pure `TaskTemplate` values, produces Cargo-locator `DebugScenario` values, and persists safe workspace selection. `cargo_preset_editor.rs` owns the ephemeral modal, validation, redacted preview and explicit save scopes. `cargo_actions.rs` owns the applicability table, Clean confirmation boundary and dispatch adapter. `CargoPanel` owns selection/UI; `Workspace` remains the task/debug entry point.

<!-- impl: crates/cargo_ui/src/cargo_preset.rs#CargoPreset -->
<!-- impl: crates/cargo_ui/src/cargo_preset_editor.rs#CargoPresetEditor -->
<!-- impl: crates/cargo_ui/src/cargo_actions.rs#plan_cargo_action_with_confirmation -->
<!-- impl: crates/workspace/src/tasks.rs#Workspace::schedule_task_reference_with_completion -->

## Design decisions

### D1: Keep Cargo configuration separate from Cargo metadata

Presets reference stable Cargo selections but live in `cargo_ui`/settings, not `CargoWorkspaceStore`. Metadata describes observed project structure; presets describe user intent. Every invocation re-resolves the intent against the latest visible snapshot.

### D2: Compile into Tasks and DAP only

The pure compiler produces structured command/args/env/cwd and task presentation. Dispatch calls existing `Workspace::schedule_task` or `Workspace::start_debug_session`. A pre-launch reference, when added, is resolved by Tasks and chained through its lifecycle; Cargo code does not spawn it directly.

### D3: Extend the schema additively and safely

Schema version 2 adds optional `toolchain` and `pre_launch_task` fields with bounded strings and migration from version 1. Toolchain selection becomes a structured leading `+toolchain` Cargo argument accepted by the authoritative host and never invokes installation. Unknown versions and invalid entries yield isolated diagnostics.

### D4: Make authoring ephemeral first and persistence explicit

The Cargo-panel modal owns a JSON draft separate from active settings. It validates against current Cargo identities and renders redacted program plus argv tokens and environment key names. Run/Debug can use the draft without saving. Save for User updates the existing user settings workflow; Save for Project uses the authoritative project buffer workflow, requires trust and one explicit confirmation, and strips environment values. Workspace DB stores only ID/selection.

### D5: Bound additional actions by safety and existing infrastructure

Doc, Clippy and Fmt compile to ordinary explicit Tasks. Clean is destructive: the pure planner rejects it by default and the panel requires a second invocation bound to the same selected node before scheduling. Tree is read-only and always adds structured `--locked --offline` arguments before any trailing program delimiter. Update is rejected. Action availability remains table-driven and accessible.

### D6: Preserve remote, trust, privacy and compatibility

All tasks/DAP execute where the project lives. Guests obey existing execution policy; disconnected/mismatch states disable actions. Environment values never enter preview, panel persistence or protocol. User-authored tasks/debug scenarios remain independent and unmodified.

## Cross-pack dependencies

- `cargo-dashboard` supplies current visible selection/configuration facts.
- `rust-tools-platform` supplies compile-time registration and host parity.
- `structured-execution` is used by test/coverage consumers, not by ordinary Cargo action output.
- `rust-test-explorer` may reuse the pure preset compiler but owns test discovery/results.

## Test seams and migration

Pure preset parsing/compilation uses adversarial argv/env fixtures. A fake dispatcher asserts Task/DAP routing without processes. Settings tests cover version migration and trust. Compatibility fixtures load representative global/project task and debug definitions, dispatch both user and Cargo actions, and compare definitions/history before and after.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3 | D1, D3, D4 | Existing preset/settings/persistence tests |
| 1.4, 1.5, 1.6, 1.7 | D2, D5 | Existing compiler/action/dispatcher tests |
| 1.8, 1.9, 1.10 | D1, D2, D6 | Existing trust/dispatch/docs and source audit |
| 2.1, 2.2 | D2, D3 | New schema/compiler/lifecycle tests |
| 2.3, 2.4 | D4, D6 | New GPUI editor/redaction/settings tests |
| 2.5, 2.6 | D5 | New action table/task plan tests and rejection audit |
| 2.7 | D6 | Dedicated tasks/debug compatibility suite |
| 3.1 | D3, D6 | Invalid schema/reference isolation tests |
| 3.2 | D4, D6 | Redaction/persistence/protocol assertions |
| 3.3 | D1, D6 | Stale selection re-resolution tests |

## Remaining delta

None. All execution stays within the existing Task/DAP/settings/project-buffer lifecycles; no process runner, DAP transport or settings subsystem was added.
