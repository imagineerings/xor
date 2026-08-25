# Design: Agentic compile-time feature boundary

## Overview

`crates/zed` owns `agentic` because it is the smallest crate that selects the complete desktop product. The feature is enabled by default to preserve the current application. It activates optional agent-only dependencies and forwards `agentic` into shared participating crates. Those crates expose no default `agentic` feature, preventing a transitive default from re-enabling the subsystem in `zed --no-default-features`.

The implementation gates complete modules where agent code already has a module boundary and gates narrow startup, menu, toolbar, URL-dispatch, and persistence registration blocks where agent and editor behavior share a file. Runtime `disable_ai` remains an enabled-build product preference; it is not used to implement the compile-time boundary.

## Existing context

- `crates/zed/Cargo.toml` is the desktop package and originally had `default = []`, while its agent, ACP, agent settings/skills/UI, provider-bundle, prompt, and web-search dependencies and startup calls were unconditional.
- `crates/zed/src/main.rs` initializes ACP tools, language models, agent registries, the agent UI, prompt loading, and AGENTS.md background watching.
- `crates/zed/src/zed.rs` owns panel restoration, workspace action registration, menu/keymap filtering, agent toolbars, and agent deep-link dispatch support.
- `crates/workspace/src/multi_workspace.rs` owns both feature-neutral project grouping and the agent threads sidebar. The disabled product keeps grouping enabled while omitting sidebar registration and actions.
- Shared UI crates such as `settings_ui`, `title_bar`, `git_ui`, `diagnostics`, `auto_update_ui`, `editor`, `terminal_view`, `onboarding`, and `workspace` contain agent-only modules or small agent integrations alongside normal editor behavior.
- `language_model` and the feature-neutral portion of `ai_onboarding` remain in the disabled graph because edit prediction uses their registry/onboarding abstractions. The `language_models` provider bundle and agent-specific `ai_onboarding` modules remain agentic.
- Cargo features are additive. Therefore every participating crate must keep `agentic` out of its defaults, and the application must explicitly forward it.
- Workspace persistence already treats unavailable serialized item/panel types as recoverable restoration failures. The disabled build must preserve that behavior and add focused regression coverage for agent references rather than registering placeholder agent types.

## Design decisions

### D-FEATURE-OWNER: `zed` owns product selection

<!-- impl: crates/zed/Cargo.toml#agentic -->

- Responsibility: Define `default = ["agentic"]`, activate every optional agent dependency, and forward `agentic` to participating shared crates.
- Integration: Existing opt-in features such as `comfy`, `rust-tools`, and `multiplayer-tools` remain orthogonal. Any feature that requires agent code includes `agentic` explicitly.
- Rationale: A virtual workspace cannot own package features, while lower-level crates cannot select the application product.

### D-PARTICIPANTS: Shared crates expose non-default forwarded features

<!-- impl: crates/agent_ui/Cargo.toml#dependencies -->

- Responsibility: Each participating crate declares `agentic = [...]`, makes agent-only dependencies optional, and gates its agent module or registration boundary.
- Integration: Pure agent crates remain normal crates but are optional in the `zed` graph. Shared crates compile their normal modules without `agentic`. When a participating crate is reached through a pure agent crate rather than directly from `zed`, that dependency edge forwards `agentic` explicitly (for example `agent_ui` to `ai_onboarding`).
- Rationale: This prevents Cargo feature unification from leaking the subsystem through an unconditional shared dependency while avoiding parallel crate implementations.

### D-STARTUP: Gate registration and initialization boundaries

<!-- impl: crates/zed/src/main.rs#main -->
<!-- impl: crates/zed/src/zed.rs#initialize_panels -->

- Responsibility: Compile agent startup, panel construction, actions, menus, toolbars, services, registries, background tasks, permissions, provider/web tooling, and agent-specific network listeners only with `agentic`.
- Integration: Use `#[cfg(feature = "agentic")]` on cohesive functions, modules, imports, match arms, and startup blocks. Non-agentic initialization ordering is otherwise unchanged. Feature-neutral multi-workspace project grouping remains enabled in the disabled product; only its agent sidebar adapter and action registrations are gated.
- Rationale: Registration points are the observable boundary and give stronger absence guarantees than hiding rendered controls.

### D-PERSISTENCE: Preserve and explicitly reject unavailable references

<!-- impl: crates/workspace/src/workspace.rs#SerializableItemRegistry::deserialize -->
<!-- impl: crates/zed/src/zed/open_listener.rs#OpenRequest::parse -->

- Responsibility: Allow generic settings/workspace parsing to retain unknown agent keys and recover from unavailable agent item/panel deserializers; reject agent deep links and commands explicitly in the disabled application.
- Integration: No placeholder agent panel or runtime shim is registered. Disabled action/keymap and URL tests combine with existing workspace session-restoration tests to exercise the compatibility paths without an agent registration.
- Rationale: A placeholder would silently change meaning and retain agent action surface; destructive migration would prevent switching back to an agentic build.

### D-MIGRATION-CONTRACT: Cross-pack write classification

<!-- impl: .agents/specs/goose-migration/feature-boundary.md#Future-task-rule -->

- Responsibility: `feature-boundary.md` is the canonical cross-pack policy and classifies each migration pack's production write families. Every pack's task plan links to it.
- Integration: Agent-only paths are compiled only through `agentic`; shared/feature-neutral paths may compile normally but their agent adapters, registration, and consumers are gated. External services and SDK artifacts are not linked into the desktop graph and cannot be launched or registered by a disabled application.
- Rationale: Central classification avoids duplicating a volatile path table in 18 plans while making future task review mechanical.

### D-VALIDATION: Resolve and inspect both Cargo products

<!-- impl: script/check-agentic-feature -->

- Responsibility: Validate compilation/tests and inspect `cargo tree -e features`/`cargo metadata` for both products. Add a source-level registration boundary test where resolved dependencies alone cannot prove absence.
- Integration: Commands target `-p zed`; workspace feature-unification inspection verifies that no `agentic` feature appears in the disabled resolved graph.
- Rationale: Package-level checks are reproducible and do not require compiling unrelated workspace member binaries that intentionally own agent code.

## Exact developer commands

Default agentic product:

```bash
cargo build -p zed
cargo test -p zed
cargo run -p zed
```

Explicit agentic product without unrelated defaults:

```bash
cargo build -p zed --no-default-features --features agentic
cargo test -p zed --no-default-features --features agentic
cargo run -p zed --no-default-features --features agentic
```

Non-agentic product:

```bash
cargo build -p zed --no-default-features
cargo test -p zed --no-default-features
cargo run -p zed --no-default-features
```

Resolved-graph checks:

```bash
cargo tree -p zed --no-default-features -e features
cargo tree -p zed --no-default-features --features agentic -e features
cargo metadata --format-version 1 --no-deps
```

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-FEATURE-OWNER, D-PARTICIPANTS | Manifest audit and resolved feature-tree test |
| 1.2 | D-FEATURE-OWNER | Default metadata and default build |
| 1.3 | D-PARTICIPANTS, D-VALIDATION | Disabled `cargo tree -e features` contains no participating `agentic` feature |
| 1.4 | D-FEATURE-OWNER, D-VALIDATION | Explicit-agentic build/test and graph comparison |
| 2.1 | D-STARTUP | Disabled compile plus registration-boundary tests |
| 2.2 | D-PARTICIPANTS, D-VALIDATION | Disabled dependency-tree denylist check |
| 2.3 | D-PARTICIPANTS, D-STARTUP | Code review and compile checks for shared crates |
| 2.4 | D-STARTUP | Default and explicit-agentic tests |
| 3.1 | D-VALIDATION | Disabled build/test |
| 3.2 | D-STARTUP | Disabled launch smoke test |
| 3.3 | D-STARTUP | Source audit rejects runtime feature switches and duplicate implementations |
| 3.4 | D-STARTUP, D-PERSISTENCE | Disabled full suite exercises multi-workspace grouping, restoration, navigation, and project-window actions without sidebar registration |
| 4.1 | D-PERSISTENCE | Disabled restoration regression test |
| 4.2 | D-PERSISTENCE | Disabled settings/keybinding compatibility test |
| 4.3 | D-PERSISTENCE | Disabled agent URL/command error-path test |
| 5.1 | D-MIGRATION-CONTRACT | Cross-pack classification audit |
| 5.2 | D-MIGRATION-CONTRACT | Canonical task validator plus manual write audit |
| 5.3 | D-MIGRATION-CONTRACT, D-VALIDATION | Future-task checklist and disabled absence validation |
| 6.1 | D-VALIDATION | Exact commands in this design and validation record |
| 6.2 | D-VALIDATION | Recorded command results for all three selections |
| 6.3 | D-VALIDATION | Dependency denylist and registration source test |
| 6.4 | D-PARTICIPANTS, D-VALIDATION | Workspace feature-unification check |

## Testing strategy

- Run manifest/source boundary tests first so feature leaks fail quickly.
- Compile and test `zed` with defaults, explicit `agentic`, and no default features.
- Inspect the disabled resolved tree for agent, agent UI/settings/skills/servers, ACP agent tooling, prompt store, and agent-only provider/web dependencies.
- Exercise generic workspace/session restoration, settings/keymap compatibility, action absence, and agent URL rejection in a disabled build.
- Launch the disabled binary as a bounded smoke check where the environment supports a GUI; otherwise record the environment limitation separately from compile/test success.
