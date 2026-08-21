# Validation plan: Zed v1.11.3 upstream port

## Evidence policy

Planned commands are not completion evidence. Each task records the exact command,
exit status, and relevant result after it runs. A skipped, filtered-out, or
platform-unavailable test is not a pass. Current-host evidence is macOS arm64
(Darwin 25.5.0) with Rust 1.95.0; Windows/Linux-only behavior requires available
cross-target/static checks and an explicit external-CI status.

## Validation matrix

### VAL-INVENTORY: Commit and path closure

- Verify endpoint/tag SHAs, merge base, 10/160 symmetric topology, 160 matrix
  rows, 425 endpoint rows, and the net-unchanged ledger using Git plumbing.
- Verify `git cherry v1.10.2 v1.11.3` equivalents and the exact U036/U038
  revert pair.
- Expected signal: no unexplained commit, endpoint path, rename side, or
  net-unchanged touched path.

### VAL-RUNTIME: Runtime and dependency behavior

```sh
cargo test -p gpui -p fs -p reqwest_client -p node_runtime
```

- Run narrower upstream regression filters for streamed requests, scheduler
  completion, filesystem trash/watch behavior, SVG limits, and renderer teardown.
- Validate `Cargo.toml`/`Cargo.lock` dependency sources and versions.
- Expected signal: focused tests pass with no duplicate equivalent/reverted code.

### VAL-PLATFORM: Platform branch coverage

- macOS: run native GPUI/macOS tests and manually exercise window movement,
  update-temp cleanup, font rendering, and display-link/window controls where a
  deterministic test is unavailable.
- Windows: compile/static-check touched `gpui_windows`, WSL sandbox, OAuth scope,
  trash, and task dispatcher branches when the target/toolchain is available.
- Linux: compile/static-check Wayland, X11, headless-window, IME, drag overlay,
  Hyprland focus, and dispatcher branches when dependencies are available.
- Expected signal: each touched platform branch has a pass or a precisely named
  unavailable external-CI requirement; no generic “platform not tested” claim.

### VAL-EDITOR: Editor/language/search/terminal behavior

```sh
cargo test -p editor -p language -p search -p terminal -p terminal_view -p markdown
```

- Port upstream inline/dedicated tests for bracket colors, formatting ranges,
  completions, selection/rename, text finder, Markdown links, terminal link/IME,
  grammar runnables/outlines, settings, and keymaps.
- For scheduler-sensitive GPUI failures, reproduce with `SEED`; use
  `PENDING_TRACES=1` for parking failures and GPUI executor timers for delays.
- Expected signal: all focused tests pass without retries or skips added to mask
  failures.

### VAL-GIT: Git/project/worktree/workspace behavior

```sh
cargo test -p git -p git_ui -p project -p worktree -p workspace
```

- Validate protobuf source/generated pairing and default compatibility.
- Exercise bare/linked worktree grouping, partial staging/unstaging, type-changed
  files, diff bases, tags, graph columns, search, panel deletion/middle-click,
  LSP rename/watch behavior, and reopen/discard regression cases.
- Run Zed multi-workspace/sidebar/persistence and Comfy panel regression tests.
- Expected signal: repository/index/buffer state is correct and Zed workspace
  architecture remains intact.

### VAL-AGENT: Agent/provider/service behavior

```sh
cargo test -p agent -p agent_ui -p acp_thread -p context_server -p language_model_core -p language_models
```

- Run MCP schema `$ref`/`$defs`, ACP boolean/elicitation/default-agent,
  provider-auth, Bedrock routing/cache/Mantle, OpenCode, OAuth retry/scope,
  LiveKit token, cloud/user IDs, and agent hyperlink/search tests.
- Use fake/local services; no paid calls, credentials, or external mutations.
- Exercise Zed credential provider, stateless mode, URLs, subscriptions, and
  thread-sidebar integration.
- Expected signal: errors propagate to UI, schemas remain compatible, and Zed
  service/product seams remain Zed-native.

### VAL-SHELL: App shell, schemas, docs, tooling, and exclusions

```sh
cargo check -p zed
```

```sh
cd docs && npx prettier --check src/
```

- Run keymap/action checks, settings schema generation/checks, workflow generator
  checks, asset validation, license/tooling tests, and generated-pair diffs.
- Verify `crates/zed/Cargo.toml` and `Cargo.lock` report 1.11.3 and release channel
  remains Zed stable.
- Negative checks: no Zed merch action, Zed Guild/community board workflows, or
  Zed issue-ranking dependency is introduced.
- Expected signal: Zed app integration and docs are coherent with no Zed product
  boundary leakage.

### VAL-REPOSITORY: Final local repository gates

```sh
cargo fmt --all -- --check
```

```sh
./script/clippy
```

```sh
python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/zed-v1.11.3-port --require-complete
```

- Run the smallest relevant broader test suites exposed by touched crates.
- Inspect the final changed-path manifest against task `_writes` and the initial
  preservation evidence.
- Expected signal: zero formatting/clippy/spec errors and no unexplained writes.

### VAL-AUDIT: Independent completeness audit

- A fresh subagent receives the verified upstream object store, local tree, spec
  pack, and validation evidence but not the lead's conclusions.
- It independently checks commit/path counts, decision exclusivity, approved
  implementation, exclusions, forward/reverse traceability, task completion,
  validation evidence, platform claims, and preservation.
- Expected signal: no unexplained upstream change, missing task, false completion,
  unvalidated accepted port, or undocumented blocker.

## Evidence log

- **VAL-RUNTIME / VAL-PLATFORM (macOS arm64):** `cargo test -p gpui -p fs -p reqwest_client -p node_runtime --offline` passed: 190 GPUI tests, 18 filesystem integration tests (plus one declared ignored stress test), 14 node-runtime tests, and 3 reqwest-client tests. The streamed-request pending-reader regression passed. Linux and Windows changes were reconciled against their source branches; no local cross-target toolchains are installed.
- **VAL-EDITOR:** `cargo test -p editor -p language -p search -p terminal -p terminal_view -p markdown --offline` passed: editor 809 passed/1 declared ignored, language 139, Markdown 117, search 44, terminal 91, and terminal-view 63. Focused settings-content (32 passed/1 declared ignored plus 2 doctests), editor undo, kill-ring, zero-width selection, and language hard-tab regressions also passed.
- **VAL-GIT:** Git passed 55 tests, Git UI passed 131, and `CARGO_INCREMENTAL=0 cargo test -p workspace --offline` passed 220. Port-relevant bare/linked worktree, symlink watcher, root-rescan, draft persistence, repository status, file status, work-directory rename, and remote-update tests passed. The combined five-crate command was interrupted after four project tests exceeded 60 seconds in parallel; each passed when filtered. A later worktree aggregate similarly left nine filesystem/scheduler tests parked after all port-relevant worktree tests had passed. These are recorded as aggregate scheduler/harness limitations, not blanket suite passes.
- **VAL-AGENT:** `acp_thread` passed 121 tests and `agent` passed 678 with 11 declared ignored. `agent_ui` passed 388 tests before a broad fuzzy-match expectation failed against the current matcher; after using provider-specific queries, its focused regression passed. The clean combined `CARGO_INCREMENTAL=0 cargo test -p context_server -p language_model_core -p language_models --offline` rerun passed context-server 96, language-model core 37, and language-model providers 74 tests, with zero failures. Focused provider configuration/reset, fake-provider, schema-reference, OpenCode protocol, Bedrock, and MCP post-initialize OAuth transition tests also passed.
- **VAL-SHELL:** `CARGO_NET_OFFLINE=true cargo xtask workflows` regenerated generator-owned YAML, and `CARGO_NET_OFFLINE=true cargo xtask check-workflows` passed after explicit least-privilege permissions were added to hand-authored workflows. Focused checks for `auto_update`, `ui`, language models, settings content, editor, Git UI, project, Copilot UI, and xtask passed. `crates/zed/Cargo.toml` and `Cargo.lock` both contain Zed version 1.11.3. The docs tree is an mdBook with no `package.json` or lockfile, so the planned `npx prettier --check src/` command has no locally defined toolchain.
- **VAL-REPOSITORY:** `cargo fmt --all -- --check` passed. `CARGO_NET_OFFLINE=true ./script/clippy -p ui -p language_model -p language_models -p git -p settings_content -p auto_update -p gpui -p fs -p reqwest_client -p node_runtime -p xtask` passed with warnings denied. The exact repository-wide `CARGO_NET_OFFLINE=true ./script/clippy` reached only `webrtc-sys`; its build script attempted `https://github.com/zed-industries/livekit-rust-sdks/releases/download/webrtc-0001d84-4/webrtc-mac-arm64-release.zip`. The user's network scope forbids that non-upstream evidence/binary fetch, so the full gate cannot complete. `cargo check -p zed --offline` has the same boundary; no Rust source diagnostic preceded it.
- **Environment limits:** repeated broad, non-incremental test linking grew `target/debug/deps` to roughly 95 GiB and exhausted the filesystem. Only reproducible `target/debug/incremental` compiler cache was removed, with user approval; source and user artifacts were preserved. The supplied Zed tree contains no `.git`, so a post-change Git status/diff and local revision proof are impossible.
- **VAL-AUDIT:** The final independent read-only audit reconciled all 160 commits, 161 decisions, 425 endpoint paths, classifications, accepted production implementations, Zed-specific seams, and tests/adaptations. It found no unexplained implementation omission, conflict marker, duplicate regression, parse error, or stub. Audit follow-ups verified that all forward rows have classification-specific current status, all Zed shell paths resolve, and every matrix row records row-specific current behavior/resolution/local paths/validation. Overall completion remains contingent on the repository-wide and cross-platform gates described above.

## Explicit pre-implementation blockers

The following decisions are deferred, not approved executable ports under the
current authority:

- `ZUP-015`: crates.io WGPU 29.0.4 is absent from the local cache and registry
  network access is outside the permitted upstream-evidence scope.
- `ZUP-061`: direct wasm-encoder/wasmparser 0.252 sources are absent locally and
  registry access is not authorized.
- `ZUP-067`: the pinned nightly, rustc-dev/LLVM components, cargo-dylint,
  dylint-link, Dylint crates, and pinned Clippy source are absent and cannot be
  installed under the network restriction.
- `ZUP-091-LICENSE` and `ZUP-097`: license files/symlinks depend exclusively on
  the deferred ZUP-067 lint workspace.
- `ZUP-094`: the local tree does not contain the authoritative Zed cloud server
  schema/rollout required to replace `AuthenticatedUser.id`; remote Zed
  inspection is expressly prohibited.
- `ZUP-099`: the security-critical tree-sitter Markdown fork/revision is absent
  from the local cache and fetching it is outside current network authority.
- `ZUP-159`: Zed intentionally points at `simtropolis/trash-rs`; the requested
  revision cannot be verified in that fork without prohibited remote Zed access,
  and changing provenance to Zed's fork is not authorized.

These entries remain in inventory, traceability, negative/blocker validation,
and the final independent audit. They are never counted as implemented or
passed. All other accepted entries continue through implementation.
