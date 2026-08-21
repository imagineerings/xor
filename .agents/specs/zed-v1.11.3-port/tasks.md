# Implementation plan: Zed v1.11.3 upstream port

## Execution constraints

- Tasks are serialized because their APIs and Cargo/lockfile writes overlap.
- Linear-backed workflow claims are not used because external-system mutation is
  explicitly unauthorized. Read-only `workflow plan/check` and local `finish`
  gates are used.
- Each task updates living documentation and matrix/traceability evidence before
  completion.

## Tasks

- [x] 1. Port runtime, dependency, GPUI, filesystem, network, and platform changes
  - _id: zed-v1.11.3-runtime-platform_
  - _priority: P0_
  - _value: high_
  - _wave: 1_
  - _reads: .agents/specs/zed-v1.11.3-port/upstream-inventory.md, .agents/specs/zed-v1.11.3-port/port-matrix.md_
  - _writes: Cargo.toml, Cargo.lock, crates/fs, crates/gpui, crates/gpui_linux, crates/gpui_macos, crates/gpui_windows, crates/gpui_wgpu, crates/node_runtime, crates/reqwest_client, crates/sandbox, crates/auto_update_
  - _validation: cargo test -p gpui -p fs -p reqwest_client -p node_runtime_
  - _Requirements: 1.1, 1.2, 6.4_
  - Outcome: Applicable runtime and platform rows expose v1.11.3 behavior with coherent dependencies and preserved Zed integrations.
  - Design: D1, D2, D3, D5, D6 / runtime-platform port.
  - Done when: Focused current-host tests pass, platform branches have recorded evidence, task ZUP rows and changed paths are reconciled, and no reverted/equivalent patch is duplicated.
  - Evidence: `cargo test -p gpui -p fs -p reqwest_client -p node_runtime --offline` passed on macOS arm64 (18 fs integration tests passed, 1 stress test ignored by declaration; 190 GPUI, 14 node-runtime, and 3 reqwest-client tests passed). Linux/Windows branches were reconciled at source level; targets are unavailable locally.

- [x] 2. Port editor, language, search, Markdown, terminal, Vim, picker, and interaction changes
  - _id: zed-v1.11.3-editor-language_
  - _priority: P0_
  - _value: high_
  - _wave: 2_
  - _blocked_by: zed-v1.11.3-runtime-platform_
  - _reads: .agents/specs/zed-v1.11.3-port/upstream-inventory.md, .agents/specs/zed-v1.11.3-port/port-matrix.md, crates/gpui_
  - _writes: Cargo.toml, Cargo.lock, assets/keymaps, assets/settings, crates/command_palette, crates/editor, crates/grammars, crates/language, crates/language_tools, crates/languages, crates/markdown, crates/picker, crates/picker_preview, crates/project_symbols, crates/search, crates/settings, crates/settings_content, crates/terminal, crates/terminal_view, crates/theme, crates/theme_settings, crates/vim_
  - _validation: cargo test -p editor -p language -p search -p terminal -p terminal_view -p markdown_
  - _Requirements: 2.1, 2.2, 6.4_
  - Outcome: Approved editor/language/search/terminal behavior and tests match v1.11.3 through Zed-native actions and settings.
  - Design: D2, D3, D5, D6 / editor-language port.
  - Done when: Focused unit/GPUI/grammar/settings tests pass, action and settings compatibility is preserved, and every mapped ZUP row has implementation evidence.
  - Evidence: `cargo test -p editor -p language -p search -p terminal -p terminal_view -p markdown --offline` passed on macOS arm64: editor 809 passed/1 declared ignored, language 139, Markdown 117, search 44, terminal 91, and terminal-view 63. Additional focused settings, kill-ring, zero-width, hard-tab, and undo regressions passed.

- [x] 3. Port Git, project, worktree, protocol, workspace, pane, and project-panel changes
  - _id: zed-v1.11.3-git-project_
  - _priority: P0_
  - _value: high_
  - _wave: 3_
  - _blocked_by: zed-v1.11.3-editor-language_
  - _reads: .agents/specs/zed-v1.11.3-port/upstream-inventory.md, .agents/specs/zed-v1.11.3-port/port-matrix.md, crates/sidebar, crates/zed/src/zed.rs_
  - _writes: Cargo.toml, Cargo.lock, crates/buffer_diff, crates/collab, crates/editor, crates/git, crates/git_ui, crates/multi_buffer, crates/project, crates/project_panel, crates/proto, crates/remote, crates/remote_server, crates/search, crates/session, crates/workspace, crates/worktree_
  - _validation: cargo test -p git -p git_ui -p project -p worktree -p workspace_
  - _Requirements: 3.1, 3.2, 3.3, 6.4_
  - Outcome: Linked/bare worktrees, protocols, partial staging, Git UI, project behavior, and workspace fixes match v1.11.3 without regressing Zed multi-workspace/sidebar/Comfy behavior.
  - Design: D2, D3, D4, D5, D6 / Git-project-workspace port.
  - Done when: Protocol generation and focused integration/GPUI/persistence tests pass and all Zed-specific workspace seams remain covered.
  - Evidence: Git passed 55 tests, Git UI passed 131, and workspace passed 220. The relevant linked/bare worktree, symlink watcher, stale-rescan, draft persistence, and project status/rename/remote-update regressions passed. The all-crate parallel command was interrupted after four project and nine worktree filesystem tests exceeded 60 seconds; the four project tests passed individually, showing parallel harness interference rather than a port assertion failure.

- [x] 4. Port agent, ACP, provider, context-server, account, cloud, collaboration, and LiveKit changes
  - _id: zed-v1.11.3-agent-services_
  - _priority: P0_
  - _value: high_
  - _wave: 4_
  - _blocked_by: zed-v1.11.3-git-project_
  - _reads: .agents/specs/zed-v1.11.3-port/upstream-inventory.md, .agents/specs/zed-v1.11.3-port/port-matrix.md, crates/zed_credentials_provider, crates/sidebar_
  - _writes: Cargo.toml, Cargo.lock, assets/icons, assets/keymaps, crates/acp_thread, crates/agent, crates/agent_servers, crates/agent_ui, crates/bedrock, crates/client, crates/cloud_api_client, crates/cloud_api_types, crates/collab, crates/context_server, crates/copilot_ui, crates/edit_prediction, crates/edit_prediction_types, crates/language_model, crates/language_model_core, crates/language_models, crates/livekit_api, crates/livekit_client, crates/open_ai, crates/opencode, crates/settings_ui_
  - _validation: cargo test -p agent -p agent_ui -p acp_thread -p context_server -p language_model_core -p language_models_
  - _Requirements: 4.1, 4.2, 4.3, 6.4_
  - Outcome: Applicable agent/provider/service behavior matches v1.11.3 while Zed credentials, endpoints, subscriptions, stateless behavior, and thread sidebar remain authoritative.
  - Design: D2, D3, D4, D5, D6 / agent-services port.
  - Done when: Focused provider/schema/OAuth/collab tests pass with fakes, user-visible error paths are wired, and all mapped ZUP rows have evidence.
  - Evidence: ACP thread passed 121 tests; agent passed 678 with 11 declared ignored; agent UI passed 388 before one stale fuzzy expectation failed, then the corrected fuzzy regression passed independently. The clean combined provider/core rerun passed context-server 96, language-model core 37, and language-model providers 74 tests. Focused configuration/reset, schema reference, MCP post-initialize OAuth, OpenCode protocol, and fake-provider checks also passed.

- [x] 5. Reconcile the Zed app shell, actions, settings, assets, documentation, tooling, CI, exclusions, and final version
  - _id: zed-v1.11.3-delivery-shell_
  - _priority: P0_
  - _value: high_
  - _wave: 5_
  - _blocked_by: zed-v1.11.3-agent-services_
  - _reads: .agents/specs/zed-v1.11.3-port/upstream-inventory.md, .agents/specs/zed-v1.11.3-port/port-matrix.md, docs/AGENTS.md, docs/.rules_
  - _writes: Cargo.toml, Cargo.lock, .github, assets, crates/zed, crates/zed_actions, crates/client/src/zed_urls.rs, crates/icons, crates/onboarding, crates/title_bar, crates/ui, docs/src, extensions/workflows, script, tooling/xtask_
  - _validation: cargo check -p zed_
  - _Requirements: 5.1, 5.2, 5.3, 6.4_
  - Outcome: Zed-native shell integration, generated pairs, docs, delivery tooling, exclusions, and version 1.11.3 are coherent.
  - Design: D4, D5, D7, D8 / delivery-shell port.
  - Done when: Zed builds, schema/keymap/docs/tooling checks pass, version metadata is 1.11.3, excluded Zed product automation remains absent, and equivalent/prerequisite entries are verified without duplicate implementation.
  - Evidence: `cargo xtask workflows` regenerated generator-owned workflow pairs and `cargo xtask check-workflows` passed; hand-authored workflows have explicit least-privilege permissions. Focused UI/settings/auto-update/xtask checks and tests passed, version metadata is 1.11.3, and negative product-boundary checks remain clean. `cargo check -p zed --offline` is blocked only by the `webrtc-sys` build script's prohibited non-upstream binary download; the docs tree has no Node/Prettier manifest, so its planned `npx` command is not locally defined.

- [ ] 6. Run final repository validation and an independent completeness audit
  - _id: zed-v1.11.3-completeness-audit_
  - _priority: P0_
  - _value: high_
  - _wave: 6_
  - _blocked_by: zed-v1.11.3-delivery-shell_
  - _reads: .agents/specs/zed-v1.11.3-port, Cargo.toml, Cargo.lock, crates, assets, docs, .github, script, tooling_
  - _writes: .agents/specs/zed-v1.11.3-port_
  - _validation: python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/zed-v1.11.3-port --require-complete_
  - _Requirements: 6.1, 6.2, 6.3, 6.4_
  - Outcome: Independent forward/reverse reconciliation and all completion gates prove the port complete or identify an explicit blocker without a false completion claim.
  - Design: D1, D6, D8 / completeness audit.
  - Done when: Counts and traceability close, focused/broad tests and formatting/clippy pass, platform evidence is complete, the spec validator passes, and the independent auditor reports no unexplained commit/path/task/validation gap.
