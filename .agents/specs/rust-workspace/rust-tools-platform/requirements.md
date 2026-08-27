# Requirements: Rust tools platform boundary

## Purpose and status

This pack owns cross-capability host authority, trust/privacy policy, compile-time `rust-tools` selection, local/remote/multiplayer compatibility, release/CI parity and final certification. Capability-specific models and UIs remain in their owning packs.

Canonical IDs are `rust-tools-platform/<criterion>`.

### Requirement 1: Preserve authoritative-host behavior [Verified baseline]

#### Acceptance criteria

1. **1.1** FOR local projects, Cargo model/configuration probes, Rust test discovery and user-invoked actions SHALL use existing local project environments and worktree/trust boundaries.
2. **1.2** FOR remote-server/SSH projects, probes and discovery SHALL execute on the authoritative host and actions SHALL use existing remote Tasks/DAP; clients SHALL NOT use mirrored local paths.
3. **1.3** WHERE WSL or dev-container projects use existing Zed remote/project-environment mechanisms, Rust workspace tools SHALL use those mechanisms without a provider-specific filesystem runner; unsupported modes SHALL be explicit.
4. **1.4** FOR multiplayer guests, hosts SHALL filter models/results to peer-visible worktrees and SHALL enforce existing guest Tasks/DAP permission policy.
5. **1.5** WHEN client/host capabilities or protocol versions differ, THE UI SHALL show a stable mismatch state, avoid retry loops, preserve unrelated editor functions and never downgrade to local execution.
6. **1.6** THE wire boundary SHALL contain stable IDs, bounded statuses/configuration/results and visible `ProjectPath` values only, never absolute host paths, environment values, raw metadata, terminal streams or unbounded diagnostics.
7. **1.7** WHEN connections or host generations change, stores SHALL cancel/invalidate in-flight work, retain privacy-safe stale data where appropriate and accept only current peer/project generations.
8. **1.8** Desktop and headless hosts SHALL register requests only for compiled capabilities.

### Requirement 2: Preserve trust, privacy and lifecycle policy [Verified baseline]

#### Acceptance criteria

1. **2.1** WHILE a worktree is untrusted, THE system SHALL run no Cargo metadata/configuration probe, Rust discovery command, task or debug launch for it and SHALL explain how to trust it.
2. **2.2** WHEN trust is revoked, THE system SHALL cancel owned work, invalidate executable plans, retain only safe stale data and reject former-generation results.
3. **2.3** Opening or refreshing Cargo/Tests SHALL never fetch dependencies, install toolchains/runners or initiate registry/network activity.
4. **2.4** Models, protocols, UI, telemetry and logs SHALL omit secret/environment values; summaries SHALL show keys only and errors SHALL be bounded/sanitized.
5. **2.5** Relevant file/config/provider changes SHALL debounce minimum invalidation and cancel/supersede obsolete work.
6. **2.6** Failed refreshes after good data SHALL expose stale safe data plus error; first failures SHALL be explicit non-stale error/empty states.
7. **2.7** Malformed metadata/settings/protocol/output/path/enum input SHALL fail fallibly, isolate the affected root/provider/run and preserve unrelated valid data.
8. **2.8** Commands, outputs, diagnostics, nodes, retained runs and payloads SHALL have enforced limits with observable partial/truncated status.
9. **2.9** Cargo panel/tasks SHALL remain usable when structured results or Rust test discovery are unavailable.

### Requirement 3: Preserve the rust-tools build boundary [Verified baseline]

#### Acceptance criteria

1. **3.1** WHERE `zed/rust-tools` is enabled, THE build SHALL initialize Cargo UI/presets/actions, generic Tests UI, Rust provider and their settings/menu registrations.
2. **3.2** WHERE it is disabled, THE build SHALL register none of those elements/stores/handlers and SHALL execute no Cargo workspace/configuration or Rust discovery command on their behalf.
3. **3.3** Disabled normal dependency graphs SHALL exclude `cargo_ui`, `cargo_metadata`, and dependencies introduced solely for these tools.
4. **3.4** `language_tools` and generic structured execution SHALL have no Cargo model/UI or Rust provider dependency.
5. **3.5** `remote_server/rust-tools` SHALL include/exclude matching host stores and handlers in parity with desktop selection.
6. **3.6** Inert protobuf definitions MAY remain compiled in disabled builds, but no associated store or handler SHALL be instantiated.
7. **3.7** Existing Rust language initialization, grammars, rust-analyzer and prior task-target discovery SHALL remain outside this boundary unless separately specified.
8. **3.8** CI/release validation SHALL cover enabled/disabled desktop and remote builds, dependency leakage, forwarding, bundle plans and mismatch behavior.

### Requirement 4: Close platform certification gaps [Required change]

#### Acceptance criteria

1. **4.1** A comprehensive hermetic Rust workspace fixture SHALL exercise the integrated Cargo dashboard, preset planning, structured results and Rust provider without host tools/network and SHALL remain separate from Zed's real workspace.
2. **4.2** Release validation SHALL enforce accepted dashboard and structured-result time/memory budgets at 1,000 packages and 10,000 tests/rows, including foreground-thread separation.
3. **4.3** A maintained physical matrix SHALL record local, actual SSH/headless, available WSL/dev-container, multiplayer and supported OS outcomes, including capability mismatch and disconnect/reconnect.
4. **4.4** Cargo and Tests panels SHALL receive manual screen-reader certification on supported desktop accessibility stacks in addition to automated roles/labels/keyboard tests, with issues recorded rather than implied away.
5. **4.5** CI SHALL retain enabled/disabled feature combinations and hermetic fake-runner tests so routine checks require no developer Cargo/rustc/rustup/registry credentials or network.

## Non-goals

No all-Rust optionality, public plugin API, universal build model, terminal parsing, automatic install/fetch, new remote transport, or provider-specific WSL/dev-container path. Apple `metal` and branded `metal_rust` remain unrelated to `rust-tools`.

## Open questions

None. Unsupported physical cells are recorded explicitly; they do not block supported cells or justify a local fallback.
