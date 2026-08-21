# Requirements: Port Zed v1.11.3 changes into Zed

## Problem

Zed identifies as version 1.10.2 and retains extensive Zed-specific behavior,
but the applicable correctness, security, performance, platform, editor, Git,
agent, dependency, and compatibility changes from Zed's v1.11.3 release line
have not been reconciled against the current authoritative filesystem. The two
upstream tags are sibling release branches, the local snapshot has no Git
metadata, and most overlapping production paths have local divergence. The port
therefore requires explicit commit/file accounting and Zed-aware adaptation
rather than cherry-picking or wholesale file replacement.

## Scope

- In scope: every right-exclusive commit in `v1.10.2..v1.11.3`, all 161
  independently reviewable decisions, every endpoint
  changed path, every net-unchanged right-range path, all applicable production
  and test behavior, dependencies, protocols, settings, actions, assets,
  documentation, CI/build/tooling, generated outputs, and version metadata.
- In scope: preservation of Zed branding, `zed`/`zed_actions` shell paths,
  credentials, hosted endpoints, multi-workspace/sidebar behavior, Comfy
  integration, persisted state, and all pre-existing filesystem changes.
- Out of scope: Zed organization community boards/ranking automation and Zed
  merchandise actions; commits, pushes, pull requests, releases, deployments,
  external tracker changes, or any remote Zed repository.

## Constraints

- The local filesystem is the sole Zed authority. Missing Git metadata SHALL
  remain explicit and SHALL NOT be reconstructed or guessed.
- Source implementation SHALL begin only after the complete pack passes
  `validate_spec.py --require-complete`.
- No accepted port may be represented by a stub, placeholder, skipped test,
  compile-only facade, dead code, or an unverified completion claim.
- GPUI tests SHALL use GPUI executor timers and deterministic reproduction
  practices described by the repository's `gpui-test` skill.

## Requirements

### Requirement 1: Runtime, dependency, GPUI, and platform parity

#### Acceptance criteria

1. WHEN an applicable upstream dependency, runtime, filesystem, network, task-scheduler, rendering, or GPUI change is approved THEN Zed SHALL carry its endpoint behavior and compatible dependency state without discarding local runtime integrations.
2. WHERE an approved change has macOS, Windows, Linux, Wayland, X11, headless, or WGPU branches THE port SHALL retain the upstream platform semantics, compile every locally available branch, and record any branch that requires external platform CI or hardware.

### Requirement 2: Editor, language, search, terminal, and interaction parity

#### Acceptance criteria

1. WHEN an approved editor, language, grammar, Vim, Markdown, picker, search, terminal, command-palette, theme, or interaction change is exercised THEN Zed SHALL expose the upstream v1.11.3 behavior through its existing UI and action architecture.
2. WHEN the upstream change includes a regression, GPUI, grammar, fixture, or settings test THEN Zed SHALL port or adapt the test and preserve existing action names, settings compatibility, and accessibility behavior.

### Requirement 3: Git, project, worktree, protocol, and workspace parity

#### Acceptance criteria

1. WHEN a project is a normal, bare, or linked worktree THEN Zed SHALL preserve upstream v1.11.3 repository identity and protocol semantics, including compatible generated protocol output.
2. WHEN users inspect, stage, unstage, partially stage, search, diff, or navigate Git/project content THEN Zed SHALL provide the approved upstream behavior and regression fixes without corrupting index, buffer, or diff state.
3. WHEN upstream workspace, pane, project-panel, or Git UI changes are ported THEN Zed SHALL preserve its multi-workspace model, thread sidebar, persistence, panel registration, and Comfy panel integration.

### Requirement 4: Agent, provider, context-server, account, and collaboration parity

#### Acceptance criteria

1. WHEN approved agent, ACP, model-provider, Bedrock, OpenAI, OpenCode, context-server, or tool-schema behavior is used THEN Zed SHALL match upstream v1.11.3 request, validation, UI, and error behavior.
2. WHEN approved account, OAuth, cloud API, LiveKit, collaboration, or protocol behavior is used THEN Zed SHALL carry the correctness/compatibility change through retained Zed services and endpoints.
3. WHEN these changes overlap local agent UI, credentials, subscription logic, or thread navigation THEN the port SHALL preserve Zed credentials, hosted URLs, sidebar semantics, stateless behavior, and local provider customizations.

### Requirement 5: Product shell, settings, assets, documentation, and delivery integrity

#### Acceptance criteria

1. WHEN approved actions, menus, open-listener behavior, settings, keymaps, icons, updater UI, documentation, or app-shell behavior is ported THEN Zed SHALL expose it with Zed names, URLs, branding, release channels, and product boundaries.
2. WHEN approved Cargo, generated, licensing, CI, workflow-generator, or repository-tooling changes are included THEN paired sources/outputs SHALL remain synchronized and no Zed-only external automation SHALL be activated.
3. WHEN the port is complete THEN the Zed package and lockfile SHALL identify version 1.11.3, excluded Zed community/merch behavior SHALL remain absent, and patch-equivalent/reverted upstream entries SHALL not be duplicated.

### Requirement 6: Complete reconciliation, preservation, and evidence

#### Acceptance criteria

1. THE specification SHALL account for all 160 right-exclusive commits, classify exactly once all 161 independently reviewable changes, and reconcile all 425 endpoint changed paths and every net-unchanged right-range path.
2. EVERY approved behavior SHALL trace forward and backward through a design decision, executable task, validation identifier, and concrete completion evidence.
3. BEFORE completion THE port SHALL pass focused tests, relevant integration/GPUI/persistence/platform checks, formatting, `./script/clippy`, and the complete specification validator.
4. THE implementation SHALL modify only declared port/spec paths, preserve unrelated current filesystem content, and report any validation that cannot execute on the current macOS arm64 host without converting it into a false pass.
