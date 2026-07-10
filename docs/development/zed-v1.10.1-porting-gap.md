# Zed v1.10.1 Porting Gap

Generated on 2026-07-10 from the upstream compare:
https://github.com/zed-industries/zed/compare/v1.7.2...v1.10.1

Local baseline reviewed:

- Repo: `simtropolis/sim`
- Branch: `codex/zed-v1.10-batch-143`
- Head: `1d461394ee fix(gpui_linux): set initial Wayland app ID`
- Local `README.md` says Sim currently tracks upstream Zed `v1.10.0`

## Method

I fetched upstream `zed-industries/zed` tag `v1.10.1` locally and compared
`v1.7.2..v1.10.1` against this branch with:

```sh
git cherry -v HEAD v1.10.1 v1.7.2
```

That range contains 359 upstream commits. Git patch-id found:

- 38 upstream patches already present exactly.
- 321 upstream patches not present as exact patch-id matches.

This is intentionally conservative. Many Sim ports were rewritten, renamed,
split, or adapted for Sim branding and architecture, so an upstream commit can
show as `+` even when the behavior was manually ported. Treat this document as
a porting triage list, not a perfect cherry-pick ledger.

## Resolved Sandbox Scope

The remaining upstream sandbox items are explicit product or architecture
deferrals; there is no unreviewed security port left in this release range.

### Agent Sandbox And Terminal Safety

This was the largest security cluster. Its applicable security and reliability
fixes are now reviewed as a set; the entries below explain deliberate product
and architecture divergences.

- `e52cc1abe9` Better sandboxing UI (#59437). The full 902-line upstream
  redesign depends on action and terminal-card infrastructure Sim does not
  have; treat it as a follow-up UI design project, not a mechanical port.
- `dfd44a45dd` Add Windows terminal sandboxing via WSL (#58971). Core WSL
  Bubblewrap wrapping, path translation, environment filtering, and timeout
  handling are present. When the WSL sandbox environment is unavailable, Sim
  now requires an explicit retry, deny, or unsandboxed one-time/thread/persistent
  decision; it never falls back silently.
- `037f32aef0` Refine sandbox off-switch and WSL environment blocklist (#59520).
  The expanded WSL environment blocklist is present. Sim intentionally retains
  its explicit `disabled` setting instead of adopting upstream's off-switch
  semantics.
- `67407035a0` Fix WSL downloaded binary path (#60210 / #60283). Defer until
  Sim adopts upstream's WSL helper-download mechanism; the current WSL port
  requires a distro-provided `bwrap` and has no downloaded helper path.

Completed on this branch:

- `eea5f57dc5` Allow sandboxed Git writes from worktrees (#57981). Adapted
  for macOS: project `.git` content is denied by Seatbelt until a distinct
  per-command/thread/persistent Git-metadata approval is granted. Windows/WSL
  overlays existing Git metadata paths read-only until that same approval is
  granted. Bubblewrap cannot overlay a missing `.git`; a newly initialized
  repository becomes protected on its next sandboxed command.
- `3d1b26d683` Protect Git metadata in Windows sandboxes (#59799). The WSL
  wrapper now resolves optional Git paths and overlays existing protected
  metadata read-only after the worktree's writable bind.
- `5d06fffd98` Harden Git sandbox metadata grants (#59785). Git metadata
  grants now validate gitfiles, linked-worktree `commondir` references, and
  symlinks before exposing an external repository path as writable; unsafe or
  unverifiable paths stay protected.
- `c49a29f461` Linux domain filtering and sandbox cleanup (#59790). Linux
  terminals now run under Bubblewrap, and host-restricted network access is
  bridged over a Unix socket to the same allowlisting proxy used on macOS. The
  restricted-network socket is passed to Bubblewrap rather than its working
  directory argument; Linux cross-compilation remains blocked locally by a
  missing `x86_64-linux-gnu-gcc` toolchain.
- `4a99aa870e` More sandboxing changes (#60111). The security core is adapted:
  macOS captures Seatbelt paths before policy generation, and Linux captures
  writable bind inodes, passes them over `SCM_RIGHTS`, then verifies each mount
  in the sandbox before launching the command. Upstream's accompanying
  untrusted-workspace UI remains covered by the separately deferred UI project.
- `3648fe6f19` Sandboxing polish (#60173). The WSL unavailable-sandbox fallback
  is present. Sim intentionally retains its verified, user-approved
  `allow_git_access` grant instead of upstream's removal of the granular Git
  permission: it preserves a narrower permission boundary without weakening the
  protected-path enforcement.

- `bf746a7a26` Replace `allow_network` with `NetworkAccess` (#59218).
  Sim's macOS Seatbelt policy now distinguishes blocked, proxy-localhost, and
  unrestricted network access.
- `8117571de3` Model sandbox network escalation as a host allowlist (#59219).
  Commands now request validated host patterns, durable grants preserve those
  patterns, and legacy boolean grants retain their unrestricted semantics.
- `3c7ca5ec5d` Enforce the network allowlist via an in-process proxy (#59220).
  Local macOS terminals are confined to a loopback proxy that enforces the
  approved allowlist; nonlocal requests are widened to explicit unrestricted
  access before approval, while unsupported local restricted requests fail
  closed.
- `2c0a044237` Make unrestricted sandbox network access explicit (#59385).
  Restricted hosts, no network, and all-host access are separate terminal
  policies; only the all-host policy can bypass the enforcing proxy.
- `a3e15cefac` Show command in sandbox permission prompts (#59362). The
  authorization metadata and detail panel now include the terminal command.
- `54cd092189` Add agent sandbox permissions settings page (#59448). Sim now
  provides a dedicated Agent Settings subpage for reviewing and revoking
  network, filesystem, and unsandboxed terminal grants.
- `dd7e50e66b` Add global setting to enable/disable agent sandbox (#59497).
  The settings-page switch is backed by a persisted runtime gate, while
  preserving the granular grants for a later re-enable.
- `9622ae92e1` Improve agent 401/403 error reporting (#59119). Agent errors
  now distinguish missing credentials from authentication failure, retain the
  provider permission message, and let account/subscription providers supply
  accurate recovery copy.
- `56562c28b0` Make agent settings searchable (#60151). Present through Sim's
  adapted `883e6bda3d` port, including aliases for skills, sandboxing, and
  tool permissions.
- `32b4ecbc76` Split edit-prediction interpolation failures from empty
  interpolations (#60499). Sim now emits `interpolate_failed` only when buffer
  changes make interpolation impossible, preserving `interpolated_empty` when
  the prediction has simply been consumed.
- `832ab56db8` Expand bare `$VAR` in Dockerfiles for dev containers (#59280).
  Sim's `ce26e088b9` expands both `$VAR` and `${VAR}` without partially
  consuming longer variable names.
- `770d5a8aa3` Support slashes in selected model IDs (#59523). Sim's
  `2953c0b39e` splits only at the provider/model separator and preserves
  slashes in the model ID.
- `91ff38aee2` Fix Python docstring formatting in hover popovers (#59480).
  Sim's `25ef3a287a` preserves hover Markdown soft breaks, including the
  docstring-style continuation layout.
- `e017293aed` Fix command aliases JSON schema (#57812). Sim's `198719aa54`
  uses a separate arbitrary-string alias target schema while retaining strict
  registered-action validation for keymaps.
- `8036a3c74b` Treat non-breaking glue characters as non-wrapping (#59775).
  The line wrapper now preserves narrow and regular no-break spaces plus the
  non-breaking hyphen as part of their surrounding word.
- `03a8544040` Middle-truncate long picker filenames (#59072). Sim's
  `29d1201150` supplies middle truncation across picker filename surfaces.
- `dde7c1c07f` Add a command to reset pane sizes (#59046). Present in Sim's
  `96994d190a` workspace action.
- `49eb6b2de7` Fix Mermaid long-label overlap (#59140). Sim's `7c067aaecf`
  applies the corresponding diagram wrapping fix.
- `67427ffbc6` Edit a queued message with Up in an empty editor (#58807).
  Sim's `3314dfc9c1` restores the queued message through the editor shortcut.
- `9053e7dd1c` Fix sidebar scrolling when hovering a header (#59054). The
  current sidebar header uses `block_mouse_except_scroll`, preserving clicks
  while allowing wheel events to reach the underlying list.
- `83aa943705` Fix workspace error popup overflow (#59185). The notification
  layout already keeps header actions fixed while the message column shrinks
  within its available width.
- `71fe7ec366` Insert dropped file links at the cursor in the agent panel
  (#55127). Sim's `f49ad73122` inserts project mentions at the selection and
  covers ordered multi-file drops.
- `40211567b8` Make grep results clickable in the agent panel (#59230).
  Each grep result now emits a file resource link and source location alongside
  the existing model-facing Markdown output.
- `45afbac0a5` Fix project grouping for Git repository subdirectories (#57998).
  Worktree identity now retains separately opened sibling subdirectories while
  the title bar disambiguates multiple visible folders from one repository.
- `e0f77d16fb` Update bundled JSON schemas (#58948). Applied as a mechanical
  refresh of the packaged npm and TypeScript schemas and their updater metadata.
- `adedf355d5` Use version-control colors in diff hunk line numbers (#59973).
  Added and deleted line numbers now inherit their diff colors without
  overriding breakpoint or active-line styling.
- `56816c0333` Reduce project-panel entry rendering clones (#56993). Applied
  as a mechanical rendering-path optimization.
- `eae0b583c0` Fix Markdown examples on Linux (#58406). Present through Sim's
  earlier `ec0a79b346` upstream-fix batch, which enables the Wayland and X11
  test backends.
- `18c98b0211` Fix the `BufferChunks::next` bitmap mask (#57544). The corrected
  relative-width mask and formatted-chunk regression test are already present.
- `fe22af1e6e` Render agent Markdown soft breaks as line breaks (#57376). The
  current Markdown renderer already uses hard breaks for the agent style while
  retaining soft-break behavior for other styles.
- `e990f31ad7` Include the active selection head in completion queries (#57405).
  The current completion flow uses the selection head except for snippet-choice
  menus, which correctly use the range start.
- `90587dd639` Keep escaped inline-code pipes inside Markdown table cells
  (#57744). Parser substitution and renderer/link regressions are already
  present.
- `053ea47e5a` Avoid treating scheme-prefixed prose as Markdown URLs on paste
  (#59071). The current clipboard flow uses a standalone-link detector.
- `7fd5ea4bf3` Prevent list scroll events from being reverted by pending
  remeasure scroll state (#59002). The current GPUI list implementation has
  the pending-scroll rebase logic and its regression coverage.
- `f56e782661` Fix Git history loading on empty repositories (#58649). The
  Git panel now differentiates loading, empty, and failed-history states.
- `992f395c3d` Fix columnar selections on multi-byte rows (#57097). The
  selection implementation anchors the rectangle in visual x coordinates and
  handles mouse drags past end-of-line.
- `e0878c4989` Highlight readonly Python semantic tokens as constants (#59811).
  The Python grammar rule is already present.
- `6d0e7ff18c` Fix block invalidation anchored past soft wraps (#59018). The
  block map invalidates from each buffer row's first wrap point.

### Process, Terminal, And Startup Reliability

Completed on this branch:

- `e1bfcf85db`, `60ed56b372`, and `d4cc8d2409`: macOS process spawning,
  descriptor ownership, and child reaping are present in Sim's adapted
  `util::command::darwin` implementation and its regression tests.
- `c578f4d12b` Fix shell hang on shell syntax errors (#59270). Present in
  `ShellBuilder`: POSIX and Fish redirect stdin before evaluating the command.
- `c642b422de` Use Windows job objects to reap spawned process trees (#58885).
  Present through Sim's `util::process` job-object wrapper.
- `7854e4535d` Fix procfs file descriptor leak in PTY process tracking
  (#58683). Present in `terminal::pty_info`, including the bounded process-map
  regression test.
- `a923597341` Fix agent terminal in headless eval sandbox (#59969). Present
  in Sim's headless-terminal support.
- `253606e8e0` Spawn crash handler on a background thread (#58881). Applied
  without adaptation; all Sim callers already poll the returned future on a
  background executor.
- `620ceaaaca` Flush thread content to the database on app quit (#58962).
  Non-empty agent threads are synchronously re-saved during shutdown so an
  in-flight background save cannot leave sidebar metadata without content.
  Its `8ad9d18b93` follow-up also preserves draft prompts and saves sessions
  concurrently within the shutdown timeout.
- `9cb139fc38` Delete subagent threads when deleting their parent (#60071).
  Thread deletion now removes the full descendant tree and its per-thread
  sandboxed terminal temporary directories.

Not applicable to Sim's current product surface:

- `a2fee92e30` Fix `terminal_init` race with PTY startup (#59613). Sim does
  not expose or invoke Zed's `terminal_init_command`, so there is no local
  command-injection race to port.

### Filesystem, Git, Remote, And Worktree Correctness

- `29622911de` Prevent archival of manually-created worktrees (#58275). This
  is a 16-file protocol, Git UI, and agent-archive change. Sim lacks the
  upstream worktree-creation tracking and `zed.proto` surface it relies on,
  so port it only as a cohesive feature rather than a partial archival patch.
- `64b8491fc6` Fix picker layout persistence with preview setting changes
  (#59827). Not applicable until Sim adopts picker preview layouts and their
  key-value persistence layer; neither exists in `crates/picker` today.
- `7b128f9263` Use remote host path style when validating trust scope (#60139).
  Not applicable to Sim's current trust modal: it does not accept an editable
  trust-scope path, and instead trusts the displayed worktree parent paths
  directly. Revisit only if Sim adds manual trust-scope entry.

Completed on this branch through adapted local ports:

- `2252cad9b9`, `2408640e5f`, `36a3a2a784`, and `af7bdd5fef`: Git watch
  preservation, quiet metadata rescans, remote changed paths, and worktree
  removal by path. The quiet-rescan behavior is supplied by Sim's local
  `1182d49472` port rather than the exact upstream patch.
- `cafbf4b5df` Improve `didChangeWatchedFiles` handler performance (#59078).
  Dynamic LSP watcher registrations now update per-registration glob sets
  incrementally and compile them only when matched; unregistering one
  registration removes only its own globs.
- `26a355b11d` Show notebook kernel launch errors on the cell (#59137).
  Sim's `01256cba78` displays a cell error rather than leaving the execution
  indicator active when the kernel cannot launch.
- `a9e469198a` Add notebook cell stop button (#57093). Sim's `e2ee985f94`
  supplies the equivalent control.
- `362035d52a` Fix opening folders whose name ends in a position-like suffix
  (#59384). Ported earlier as Sim's `ec0a79b346`, including directory and
  platform-specific colon-suffix regression tests.
- `f4c621b78d` Cancel in-flight open-path tasks when dialog is dismissed
  (#59423). The delegate cancels its fuzzy-match task before closing.
- `e4f6742a99` Use fast repository access checks in the Git panel (#59514).
  Sim uses `git rev-parse` through the repository's dedicated access check.
- `cd7f1a0fb1` Preserve Git repositories during watcher rescans (#59976).
  Recursive refreshes retain repository identities and only reap a repository
  when its `.git` entry is confirmed absent.
- `f8e1ab7f3c` Coalesce queued rescans after watcher overflow (#60098). Sim's
  filesystem watcher preserves ordinary events while coalescing ancestor and
  descendant rescan notifications.
- `c35650a884` Support checking out remote `HEAD` refs (#57648). Sim resolves
  symbolic remote HEAD references to their tracking branches before checkout.

### Editor, Language, Markdown, And Keymap Fixes


Completed on this branch:

- `b6c7496aea` Avoid eager `BufferSnapshot` clones (#59190). Sim already
  returns borrowed snapshots from `range_to_buffer_ranges`; the remaining
  repeated-bound and default-theme allocations are now aligned with upstream.
- `96285fc140` Align split buffer headers with scrollbar layout (#53782).
  Sim already tracks each split editor's scrollbar margins and clips headers
  around horizontal and vertical scrollbars.
- `489c880a58` Reuse display-map cursors while converting word diffs (#58658).
  Word-diff painting now processes visible ranges monotonically and resets
  safely for overlapping ranges.
- `1722fe63bc` Speed up multi-cursor editing (#58510). The editor runtime
  fast paths and selection-equivalence tests are ported; Sim retains its own
  divergent benchmark harness.
- `2a93ca53fd` Handle dynamic semantic token registration (#60015). Sim
  advertises support, updates server capabilities on registration or removal,
  and refreshes semantic tokens for already-open documents.
- `0deb6c0dea` Resolve code lens actions in remote workflows (#59999). Remote
  code lenses use the code-action resolution RPC and update their cached lens
  entry once the host resolves it.
- `ae0f4462ae` Improve Helix keymap (#59638) and `dfb70de652` add `z c`
  center-cursor bindings (#58660). Both keymap additions and the required
  Helix next/previous operator context are already present.
- `b30ef390e1` Use visible cursor position for rename (#55542). Rename shifts
  forward visual or Helix selections back onto the rendered cursor position,
  with regression coverage in the Helix test suite.
- `270a0671e8` Fix keymap editor edits/deletes for deprecated action aliases
  (#60300 / #60362). Sim canonicalizes aliases during keymap updates and
  covers both edit and delete flows.
- `c7ad65e468` Optimize Markdown search highlight painting (#59473). Search
  ranges are ordered once, rendered lines are traversed once, and wrapped-line
  layouts are reused while painting highlights.
- `84b753cb51` Refresh active debug line highlight on theme change (#59274).
  Applied directly across the editor, debugger, and related theme consumers.
- `6e129aa5df` Show file icons in editor breadcrumbs when the tab bar is hidden
  (#56267).
- `50b4a1c17e` Fix Vim's out-of-scope `InsertLineAbove` indentation (#55459).
- `45e84381e2` Bound edit-prediction diagnostic message context (#59644).
- `ced0d857fd` Highlight Go predeclared types and built-in functions (#59780).
- `438070b1cf` Fix Debug Test for Go subtests (#53680). Nested subtest regexes
  are now escaped for task expansion and unescaped for Delve's launch request.
- `914cc66103` Restore unnamed bookmarks for the existing toggle action
  (#60185), while keeping label prompts behind a dedicated action.
- `9ac117693b` Preserve Windows title-bar button hit testing for non-movable
  windows (#59816).
- `e45e42af6e` Use the agent thread title for waiting notifications (#59377).
- `50b2f63cfe` Avoid sidebar flicker when reselecting the active draft thread
  (#59342).
- `7b73d5ccc3` Keep the Copilot sign-in verification window above its parent
  window (#59657).
- `14f9b9d077` Keep the agent message editor's horizontal padding aligned
  with its content column (#59735).

### v1.10.1 Tail Commits

These are the commits after the v1.10.x stabilization markers that should be
checked explicitly before calling the port complete:

- `67407035a0` Fix WSL downloaded sandbox binary path; deferred pending the
  helper-download mechanism described in the sandbox section.
- `270a0671e8` Fix keymap editor deprecated action alias editing/deleting.
  Already adapted locally, including alias-aware persistence and regression
  coverage for deleting the alias entry.
- `f3f6c6b80a` Restore workspace by default when `cli_default_open_behavior = "new_window"` and no path is passed. Applied: an empty CLI request now restores the previous workspace for the configured new-window behavior.
- `3add0bc55a` Add GPT 5.6 model entries. Intentionally deferred under the
  product-fit decision recorded below.
- `831248bf21` Version bump to `1.10.1`; likely not directly relevant except for Sim's own version metadata.

## Likely Already Ported Or Partially Ported

These did not all match patch-id exactly, but local history contains close
ports. Verify behavior before re-porting:

- Gutter timestamp width: local `Expand git blame gutter timestamp width`.
- Select-inside brackets: local `Add select-inside-brackets action`.
- Branch/worktree picker copy: local `Clarify branch and worktree picker placeholders`.
- Dev container dismissal: local `fix(dev_container): persist dismissal across worktrees`.
- LiveKit refresh logging: local `Log LiveKit connection refresh outcomes`.
- EditorConfig/Prettier: local `Honor editorconfig in Prettier config resolution`.
- Noop macOS text fallback warning: local `Warn on noop macOS text system fallback`.
- Git status optimizations: local commits around stash/default branch/pushed commit checks.
- Remote updated paths: local `worktree: Include paths in remote entry updates`.
- Memory logging: local `feat(sim): log periodic memory usage`.
- Filesystem dispatch/rescan work: local `fix(fs): dispatch watcher events from reader thread` and `fix(fs): coalesce queued watcher rescans`.
- MCP settings and elicitation UI: local `agent_ui: Open MCP settings from panel` and `acp: Add elicitation UI support`.

## Lower Priority / Product Decision Needed

The remaining exact-missing set also contains many items that may not be worth
porting directly:

- Upstream release, CI, docs, community automation, duplicate-bot, and zed.dev
  deployment changes.
- Zed-specific hosted settings UI changes if Sim's agent/settings surfaces have
  diverged.
- Branding/version-only commits such as `Bump Zed to v1.8.0`, `v1.10.x preview`,
  `v1.10.x stable`, and `Bump to 1.10.1`.
- Upstream collaboration/client username changes if Sim's collab protocol has
  already diverged through channel enhancements.

## Exact-Missing P2 Product-Fit Decisions

These are exact-missing P2 upstream commits where the right answer is product
fit, not automatic upstream parity.

### Deferred Product Backlog

The following remain exact-missing but are intentionally deferred. They are
preference-heavy UI additions, product-direction choices, or upstream workflow
polish rather than requirements for a sound Sim experience:

- `cb7721602b`, `62b4fd26cf`, `b2143f449b`, `5e32405669`, `c3c38c5c09`,
  `f9a4bfd826`, `35e2ef8af5`, `46ff888db8`, `db30c67ed2`, and `a8ffae4c00`.
- `fd5d42dd55`, `0a2f8b5b5c`, `ccf4058b7a`, `3df8983deb`, `076fd14c88`,
  `e25e52be87`, `514b14ed49`, and `d8228b4280`.
- `a1a881b0f7`, `7e4caf003c`, `4eb039b451`, `42a2eff274`, `6c923cc117`,
  `02aabb9cef`, `776585038e`, `26ee53e5aa`, `8186af99a3`, `914cc66103`,
  and `e4e2c9d3d5`.
- `2df74932bc` adds rich embedded ACP-resource previews. Sim already preserves
  embedded text resources, but the upstream renderer is a broad, non-clean
  port across Sim's diverged ACP UI, so it is deferred for dedicated ownership.
- `10628c3d2c` and `cf93437d6a` add in-thread search and its Escape behavior.
  They depend on upstream's separate `ThreadSearchBar` architecture, which
  Sim does not currently have; defer them as one product-owned search feature.
- `2c346f60a7` changes project-diff identity and ordering across Sim's diverged
  diff view; defer it for a dedicated project-diff refactor.
- `39bba3c7ec` replaces CLI-install failures with notifications, but Sim's
  installer and macOS documentation have diverged; defer it for installer UX
  ownership rather than applying the outdated flow.

### Verified Adaptations

The following exact-missing commits have now been checked against the current
source and are already implemented under Sim's adapted UI and worktree
architecture:

- `c78bd36fd8`: regeneration preserves earlier subagent edits by treating
  earlier subagent tool calls as potentially edited, then keeping the linked
  action-log edits before rewinding.
- `c0f4059806`: directory renames select the full dotted directory name while
  file renames keep their final extension unselected.
- `3e8aae49b5`: `git_ui::worktree_service` provides foreground user worktree
  creation with rollback and workspace setup.
- `f2fbe40b5d` and `906b7e6612`: enclosing-bracket selection actions and
  their selection-layer implementation are present with editor coverage.
- `f2006b20ac`: inline blame supports the status-bar location.
- `c57b348ad1`: the panel's `menu::Cancel` handler dismisses agent and
  terminal notifications before propagating Escape.
- `031ccfd736`: agent-panel terminals explicitly disable workspace actions.
- `eef824cce5`: Sim's current agent settings navigation already exposes the
  MCP settings surface instead of retaining the upstream modal flow.
- `fa66442a01`: ACP elicitation cards, form state, responses, and tests are
  present in the conversation and thread views.

`02b62a3d1f` and `15c31d4147` remain provider-settings product decisions:
the upstream forms assume Zed's large provider-page architecture, while Sim
currently has a differently scoped settings surface.

### Defer

These may fit Sim, but they touch areas where Sim has likely diverged or where
the user value depends on product direction:

- `20a3f7705f` Cloud listings support `supports_disabling_thinking`.
- `d7ac5e6cf4` Preserve waiting tool-call status on ACP updates.
- `8432a26a9d` Fix thinking toggle button.
- `638b33ca2b` Placeholder title in agent empty-state toolbar.
- `fef979dec4` Anthropic-compatible provider support in settings.
- `c486f6f529` Agent toolbar title display iteration.
- `df9c9f055e` Search archived agent threads by project name.
- `c7987fabf7` Truncate long model names in config option selector.
- `f39cf25c0b` Hide agent servers from extension chips.
- `f16a46967b` Do not return diff after successful agent edits.
- `33a54ce423` Provider-side compaction in language model clients.
- `f208b2f108` Store rejected edit-prediction patches.
- `6f31ec3328` Qwen prompt format.
- `6076ce2738` Add initial Markdown element benchmark.
- `790b73e2fb` Detect SCP remotes with non-standard SSH usernames.
- `195760c617` Thread search design improvements.
- `8ba35e5eac` Update ACP client protocol to 0.15.0.
- `2df089ebe1` Advertise model picker support to Cursor.
- `e8e479c3c` Fix env var name input for local MCPs.
- `61ce210ec2` Make remote MCP server setup more discoverable.
- `4eb039b451` Ollama partial model-load failure handling.
- `438070b1cf` Fix Debug Test for Go subtests.
- `0d3badc857` Fix profile configuration modal from picker.
- `7187d65774` Set `support_thinking` in OpenAI-compatible Responses API.
- `f844c93c94` OpenRouter prompt caching for Anthropic models.
- `daf4656c87` Route Skill permissions to their own settings page.
- `aa7dc30ec3` Show favorite model button for unselected models.
- `10f501d700` Remote benchmark orchestration for eval CLI.
- `0346cc77ee` Solo diff Git action and related improvements.
- `784b14e207` Hashed edit-prediction region format.
- `17c0ebb0f7` Batch settled edit-prediction telemetry.
- `593ca12e3f` Add Max reasoning effort.
- `9de0590a81` Update ACP Rust SDK to 1.0.0.
- `5989a369d3` Send V4 prediction requests.
- `1fd93cbd34` Unship shared threads.
- `3eb9bf2d21` Move MCP server timeout setting to MCP subpage.
- `4b119fc547` Add llama.cpp provider.
- `b04ae5ed81` Update ACP SDK to 1.0.1.
- `3bb922014e` Rename llama.cpp provider ID.
- `29faf94a0f` Fix V4 cursor marker leak.
- `6a1ced4a8e` Use compaction boundaries for summaries and titles.
- `5329bd81d2` Make "new from summary" less prominent.
- `45015f89d7` Support boolean ACP config options.
- `e783b2f063` Defer ACP status updates until turn completion.
- `40d20036af` Remove AI settings feature flag and adjust design.
- `3add0bc55a` Add GPT 5.6 model entries. Defer: Sim's independently
  maintained catalog already has later model families, so upstream's Sol/Terra/Luna
  defaults should be adopted only with a deliberate provider-catalog update.

### Skip

These are Zed-specific release, CI, community, branding, or reverted changes
and should not be ported unless Sim intentionally adopts the same workflow:

- `5976ffb4b6`, `4cab63fb59`, `3b2acfe0a`, `8d56867088`, `111c4082fd`,
  `831248bf21`: Zed version/release bump markers.
- `53f1ae01a4`, `cfc5f26970`, `838fea165c`, `51fac82ddd`,
  `3601a7c8c2`, `153f709a18`, `01c316ae31`, `f99df1a155`,
  `3c312b596e`: upstream docs-only or Zed-specific documentation changes.
- `511d197477`, `1d217ee39d`, `c32c037c6b`, `a851320e6d`,
  `e5966915e4`, `6b4e27a4d5`, `555ed0495f`, `a225d51024`,
  `1e7f1a11f9`, `968379f5a1`, `8372eb1b13`: upstream CI, release,
  duplicate-bot, PR-board, or community automation changes.
- `c896cc99a9`, `69b602c797`: upstream community/VIP metadata.
- `4d94097df0`: reverted Finder open-behavior change.
- `e4dbdaa622`: cherry-pick-bot config removal.
- `0a7c84bc37`, `380a5c79b6`, `53e4d34a71`: upstream collab `username`
  protocol churn; handle only if Sim wants protocol parity with Zed hosted
  collab.
- `d7b9b28deb`: JavaScript docs for format-on-save default; skip unless Sim
  also ports the default behavior and documentation set.

## Completion Status

The applicable security, reliability, editor, and tail-release ports in this
comparison are implemented or verified as adapted local behavior. Exact-missing
P2 changes are deliberately classified in the product backlog above; release,
CI, community, and branding changes are intentionally skipped. Future work on
the sandbox UI, WSL helper download, model catalog, ACP previews, search, and
project diff should be planned as Sim-owned product work rather than upstream
parity work.
