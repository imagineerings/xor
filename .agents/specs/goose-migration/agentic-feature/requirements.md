# Requirements: Agentic compile-time feature boundary

## Problem

Zed currently compiles and initializes its agent subsystem unconditionally. The Goose migration portfolio adds more agent UI, services, tools, providers, permissions, persistence, background work, and network integrations, so a runtime setting cannot provide a reliable product boundary or prevent agent-only dependencies from entering a build. Maintainers need one application-owned Cargo feature that preserves the current product by default and produces a normal, non-agentic Zed binary when disabled.

## Scope

### In scope

- The desktop `zed` application feature and explicit forwarding into every shared crate that contains an agentic integration boundary.
- Compile-time exclusion of the existing Zed agent subsystem and every Goose migration production deliverable reachable from Zed.
- Optional Cargo dependencies, whole-module or registration-boundary gating, safe handling of persisted agentic references, developer commands, and two-configuration validation.
- A cross-pack rule classifying every existing and future Goose migration production write as agentic or demonstrably feature-neutral.

### Out of scope

- Runtime replacement implementations for agent behavior.
- Removing workspace members that are not reachable from the `zed` application dependency graph.
- Changing non-agentic editor behavior, edit prediction behavior that does not depend on the agent subsystem, or independently built external services beyond preventing their registration in a non-agentic application.

## Glossary

- **Agentic subsystem**: Agent conversations, ACP integrations, agent UI and commands, agent settings and skills, agent-specific language-model/provider initialization, tool and permission infrastructure, and Goose migration deliverables.
- **Participating crate**: A shared crate compiled into `zed` that contains an agentic module or registration boundary and therefore receives an explicitly forwarded `agentic` feature.
- **Feature-neutral write**: A production change whose compiled behavior is useful without the agent subsystem and neither registers nor initializes agentic behavior. Its agentic caller or adapter remains gated.

## Requirements

### Requirement 1: Application-owned feature topology

**User story:** As a build maintainer, I want one feature selected at the application boundary, so that Cargo resolves a coherent agentic or non-agentic product.

#### Acceptance criteria

1. **1.1** THE `zed` application crate SHALL own a Cargo feature named `agentic` and explicitly forward it to every participating crate.
2. **1.2** THE default `zed` feature set SHALL include `agentic` to preserve current behavior.
3. **1.3** WHEN `zed` is built with `--no-default-features`, THEN THE resolved application graph SHALL not enable `agentic` in any participating crate through default features or feature unification.
4. **1.4** WHEN `zed` is built with `--no-default-features --features agentic`, THEN THE resulting feature selection and product behavior SHALL be equivalent to selecting the default agentic boundary, excluding unrelated opt-in features.

### Requirement 2: Compile-time exclusion

**User story:** As a distributor, I want a non-agentic Zed binary, so that excluded functionality and dependencies are not shipped or initialized.

#### Acceptance criteria

1. **2.1** WHILE `agentic` is disabled, THE application SHALL compile without agent UI, agent actions and command registrations, agent menus, agent services and registries, agent background work, agent tools, agent permissions, or agent-specific network initialization.
2. **2.2** WHILE `agentic` is disabled, THE application dependency graph SHALL exclude agent-only crates and third-party dependencies that are reachable solely through them.
3. **2.3** WHERE a shared crate contains agentic and non-agentic behavior, THE crate SHALL gate a whole agentic module or its narrow registration boundary instead of using a runtime boolean.
4. **2.4** WHERE `agentic` is enabled, THE application SHALL retain the current agent subsystem behavior and registrations.

### Requirement 3: Normal non-agentic application behavior

**User story:** As a user of a non-agentic build, I want the editor to remain a complete editor, so that removing the agent does not degrade unrelated workflows.

#### Acceptance criteria

1. **3.1** WHEN `zed` is built and tested without default features, THEN all compiled non-agentic application code and tests SHALL succeed.
2. **3.2** WHEN the non-agentic binary launches, THEN normal project, editor, terminal, collaboration, settings, update, and extension functionality SHALL initialize without requiring agent globals or services.
3. **3.3** THE implementation SHALL not introduce parallel non-agentic implementations of agent services or runtime feature booleans that substitute for compile-time exclusion.
4. **3.4** WHILE `agentic` is disabled, THE application SHALL retain feature-neutral multi-workspace project grouping, restoration, navigation, and project-window actions without registering the agent threads sidebar.

### Requirement 4: Safe persisted references

**User story:** As a user switching between builds, I want unavailable agentic state handled explicitly, so that my persisted data is neither corrupted nor reinterpreted.

#### Acceptance criteria

1. **4.1** IF persisted workspace state references an agent-only panel, item, command, or workflow while `agentic` is disabled, THEN THE application SHALL reject or skip that unavailable reference through an explicit, tested compatibility path without panicking.
2. **4.2** IF settings or keybindings contain agentic keys or action names while `agentic` is disabled, THEN THE application SHALL preserve their meaning for a future agentic build while preventing registration or execution in the current build.
3. **4.3** IF an external request targets an agentic-only URL or command while `agentic` is disabled, THEN THE application SHALL report that the capability is unavailable and SHALL not silently route it to another behavior.

### Requirement 5: Goose migration coverage contract

**User story:** As a migration implementer, I want every task to respect the product boundary, so that later work cannot accidentally leak into non-agentic builds.

#### Acceptance criteria

1. **5.1** THE Goose migration specification SHALL classify every existing production write target as agentic or feature-neutral and SHALL name the compile-time boundary that contains agentic behavior.
2. **5.2** WHEN a future Goose migration task adds or changes a production write, THEN its task metadata and validation SHALL demonstrate either `agentic` gating or feature-neutrality with a gated consumer or adapter.
3. **5.3** IF a future migration deliverable adds a dependency, registration, background task, permission, tool, command, menu, or network initializer, THEN THE task SHALL include disabled-build validation proving its absence.

### Requirement 6: Reproducible validation and documentation

**User story:** As a contributor, I want exact commands for both products, so that I can reproduce the supported configurations and detect feature leaks.

#### Acceptance criteria

1. **6.1** THE specification and repository-facing documentation SHALL give exact build, test, and run commands for the default, explicit-agentic, and non-agentic configurations.
2. **6.2** THE validation SHALL include default build and tests, explicit `--features agentic` build and tests, and `--no-default-features` build and tests.
3. **6.3** THE validation SHALL inspect Cargo metadata or the resolved dependency tree to prove agent-only optional dependencies and agentic registrations are absent from the disabled application.
4. **6.4** THE validation SHALL check workspace feature unification so no participating crate enables `agentic` in the disabled application graph.

## Constraints

- Prefer the smallest existing module and registration boundaries; do not create a second application or agent implementation.
- Preserve unrelated work in the repository and avoid changes outside the desktop application dependency graph and Goose migration specifications.
- Use `./script/clippy` for Rust lint validation when linting is required.
