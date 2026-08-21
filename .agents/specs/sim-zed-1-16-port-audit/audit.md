# Sim port audit: Zed v1.10.2-era tree to v1.16.1

## Comparison anchors and history

- Old base: `adc60ccf12e199b8828bad3abb2591e147034734` (`v1.10.2`)
- Old Sim tip: `d41ad2b582bceb6b1b49eb68f877ebed7d68eeb2` (`sim-dev` at audit start)
- New base: `eb8e1c8b5502b7007465fbbc465f4a736fa39210` (`v1.16.1`)
- Rebased tip before repairs: `5ab1c4de4a35e61476ff3cb88a5bcf7d9354d35e`

The old base and old Sim tip have no merge base. The new port is represented by
one squashed `Import Sim changes` commit. A literal `git range-diff` therefore
cannot establish commit equivalence: it compares an independent root history to
a single rewritten commit and skips trustworthy rename/patch correspondence.
The durable comparison is the four-tree endpoint ledger in `port-ledger.csv`.

The ledger classifies all 4,059 relevant paths: 3,622 preserved exactly, 286
ported with adaptation, 49 deleted by both deltas, 100 missing/unresolved against
the old endpoint delta, and 2 rebased-only paths. The 100 unresolved entries are
an explicit review queue, not a claim that their old v1.10-era edits should be
blindly reintroduced over v1.16.1.

## Why `cargo run` failed

The initial command failed before compilation because
`agent_ui`'s `multiplayer-tools` feature referenced `git_ui/multiplayer-tools`
without declaring `git_ui`. Removing that first mask exposed a stack of rebase
defects:

1. duplicate and malformed dependency declarations, including a duplicate
   `zed_credentials_provider` entry;
2. an interleaved/corrupted imported lockfile and unapproved Sim fork
   substitutions for existing Zed dependencies;
3. duplicated or interleaved Rust conflict resolutions, dropped protocol variants,
   stale v1.10 APIs, and old modules retained after v1.16 architectural moves;
4. a macOS host limitation: Xcode is selected, but `xcrun metal` reports that the
   optional Metal Toolchain is not installed;
5. an immediate runtime panic caused by the old Sim tree's incomplete rename of
   `zed-keybind-context` assets to `sim-keybind-context` while all registrations
   and the config identity still used `zed-keybind-context`.

The code now builds and starts with upstream's existing runtime shader path:

```sh
ZED_STATELESS=1 cargo run -p zed --features runtime-shaders,rust-tools
```

The app remained active after initialization with no panic and was then stopped
with Ctrl-C. Plain `cargo run` still requires installing Apple's Metal command-line
toolchain (`xcodebuild -downloadComponent MetalToolchain`); upstream's default
shader-build behavior was intentionally not changed.

## Dependency reconciliation

Every dependency declaration that existed in v1.16.1 is byte-semantically equal
to its upstream declaration, including source URL, revision, package name,
version, features, target table, and platform configuration. The audit reports:

- 5,717 exact upstream declarations;
- zero drifted or missing upstream declarations;
- 79 reviewed Sim additions in existing manifests and 27 new Sim manifests;
- zero unapproved Sim fork declarations or lock records;
- 1,608 preserved upstream external lock records.

The lockfile was regenerated after manifest reconciliation. It replaces only
three registry versions required by retained Sim additions: `cap-primitives`
3.4.4 to 3.4.6, `cap-std` 3.4.4 to 3.4.6, and `flate2` 1.1.8 to 1.1.9. It adds
28 external records for Sim-only Nostr/crypto and related functionality. No
fork exception was selected or requested.

## Original Sim behavior mapping

| Behavior family | Original Sim evidence | Port implementation and disposition | Validation / residual risk |
| --- | --- | --- | --- |
| Comfy native graph, tensor/model/sampler/runtime, media codecs, plugins, workers, and UI | New `comfy_*` crates, specifications, operation contracts, fixtures, model families, codecs, and backend adapters | Sim-only crates and features are retained; their files dominate the 3,622 exact entries. Existing Zed dependencies were not redirected to Sim forks. | Manifest resolution succeeds, but `--features comfy` stops in `comfy_tensor` before Rust compilation because all 511 declared evidence hashes disagree with their fixtures. The validators, declarations, and fixtures are byte-identical to `sim-dev`, so this is a pre-existing Sim defect. Comfy workflows and accelerator backends remain unverified. |
| Rust Cargo model and structured test execution | Cargo workspace discovery, toolchain/profile model, structured protocol messages, Rust test provider, task handles, and test explorer | Old `RelPath::unix` calls were ported to v1.16.1 `from_unix_str`; dropped protocol oneof variants were restored at unused tags 478-490; workspace scheduling now preserves both v1.16 completion callbacks and Sim structured handles. | 15 Cargo workspace tests and 4 task scheduling tests pass; `rust-tools` runnable check passes. Tags differ from old Sim's 462-474 because those collide with v1.16.1, so old Sim wire peers are not compatible without negotiation. |
| Collaboration, Nostr credentials, channels, sidebar, and shared workspace surfaces | `collaboration_domain`, `nostr_compat`, collab/channel/sidebar changes, Nostr credential features | New Sim crates/dependencies are retained without replacing existing Zed dependencies; v1.16 workspace ownership APIs are preserved in sidebar and multi-workspace adaptations. | Compiles in the runnable target. Multi-peer sessions, Nostr login/key storage, channel membership, and remote compatibility were not exercised. |
| Agent UI and external-agent workflows | Agent panel/thread/message/editor/provider changes and imported agent specifications | Duplicate stale handlers/subscribers were removed; provider settings were adapted to v1.16's settings-view architecture; current Zed behavior remains around the Sim additions. | All 428 `agent_ui` library tests pass. No live provider credentials, MCP server, or long-running agent session was exercised. |
| GPUI accessibility and list/context-menu semantics | disabled/expanded/radio accessibility additions | Sim properties were mapped to v1.16.1 `AriaProperties` and AccessKit set/clear APIs; context-menu tuple shape and list-item backing state were updated. | Focused GPUI accessibility and UI radio tests pass. Full VoiceOver interaction was not performed. |
| Git review, branch/unstaged diff, graph, and worktree services | Git UI and diff/worktree changes | Stale duplicated v1.10 modules/functions were removed where v1.16 moved ownership into `git_ui_core`; surviving Sim hooks were adapted to current signatures. | Runnable and `agent_ui` diff tests compile/pass. Real repository staging, conflict resolution, branch switching, and collaborative review remain manual. |
| Terminal task behavior | Smarter terminal shrink-to-used behavior and structured task spawning | Sim's shrink behavior remains; listener calls use the v1.16 listener type. Task spawning merges structured lifecycle/error reporting with upstream save/completion behavior. | Four scheduling/save tests pass. Interactive cancellation and remote structured execution remain unverified. |
| Language server settings | Old Sim global-disable propagation and per-language lists | The stale global-to-language rewrite was not reintroduced. v1.16.1 typed `ConfiguredLanguageServer` lists remain authoritative and per-language lists stay pure. | Three merge tests pass. This is an intentional semantic supersession, not literal patch preservation. |
| Branding, URL scheme, menus, providers, and keymaps | Sim branding variants, `register_sim_scheme`, menu/keymap/provider edits | The initial approach renamed existing Zed icon/vector enums and assets together; this was corrected. All existing asset paths, blobs, enum identities, references, default keymaps/settings, and theme metadata now match v1.16.1. Sim branding will be introduced later as explicit additions. The broken keybind grammar rename was likewise reverted to upstream identity. | All 463 upstream assets match their v1.16.1 blob IDs. Zed base-keymap and exhaustive icon/vector inventory tests pass, and stateless startup emits no asset-loading errors. Detailed menu/URL invocation and complete visual review were not manually exercised. |
| v1.10-era edits in the 100 unresolved ledger paths | Old endpoint changes across editor, project, Git, language, platform, docs, and tests | Left on the explicit unresolved list where the v1.16 endpoint is unchanged from upstream and the old edit has no demonstrated current Sim requirement. Several old Git modules are superseded by `git_ui_core`; other paths need feature-owner review. | Not claimed as parity. Each path and all four blob IDs are recorded in `port-ledger.csv` for follow-up. Cross-platform Linux/Windows behavior is unverified. |

## Corrections made

- Restored exact v1.16.1 dependency declarations for `scap`, `xim`, `font-kit`,
  `reqwest`, WGPU, tree-sitter, async/runtime, and platform dependencies; audited
  every manifest for equivalent substitutions.
- Regenerated `Cargo.lock` from the reconciled manifests.
- Repaired missing feature dependencies and duplicate manifest entries.
- Removed duplicated/interleaved Rust hunks across GPUI, Git UI, agent UI,
  context-server transport, providers, settings, terminal, UI, workspace, and Zed.
- Restored dropped Sim protocol messages without colliding with v1.16.1 tags.
- Ported old path, accessibility, workspace, task, settings, provider, and Git APIs
  to the v1.16.1 architecture.
- Exposed the existing upstream runtime shader path as a non-default Zed feature
  for hosts without Apple's optional Metal toolchain.
- Restored the upstream `zed-keybind-context` asset directory, fixing the startup
  panic caused by the incomplete old Sim rename.
- Corrected the superseded asset-renaming approach by restoring all existing
  icon/vector filenames, blobs, enums, call sites, keymaps, settings, and theme
  metadata to exact v1.16.1 identity. Sim branding remains future additive work.
- Retained exhaustive missing/dangling vector tests while making them validate
  `ZedLogo` and `ZedXCopilot` and their upstream paths.
- Retained only `assets/keymaps/default-comfy.json` and
  `assets/settings/default-comfy.json` as genuinely new Sim asset files.
- Fixed the imported `script/clippy` argument filter so its empty-argument path
  works with macOS Bash under `set -u`.
- Removed a duplicated, unused SVG size-limit constant left inside
  `SvgRenderer::render_pixmap`; the active cap remains in `rasterize_tree`, as in
  v1.16.1.

## Validation results

| Command / check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |
| `bash -n script/clippy` | Passed |
| `CARGO_INCREMENTAL=0 ./script/clippy --no-all-features -p zed --features runtime-shaders,rust-tools` | Passed in release mode for all targets; required the existing upstream WebRTC artifact download |
| `cargo metadata --no-deps --format-version 1 --locked` | Passed |
| `generate_audit.py --check` | Passed; 4,059 paths classified |
| `generate_dependency_audit.py --check` | Passed; zero unresolved dependency findings |
| `cargo check -p zed --features runtime-shaders` | Passed |
| `cargo check -p zed --features runtime-shaders,rust-tools` | Passed |
| `cargo check -p zed --features runtime-shaders,rust-tools,comfy` | Failed in byte-identical old Sim evidence validation: 511/511 stale fixture hashes |
| GPUI accessibility test | Passed, 1/1 |
| UI context-menu radio accessibility test | Passed, 1/1 |
| settings-content language-server merge tests | Passed, 3/3 |
| project Cargo workspace tests with `rust-tests` | Passed, 15/15 |
| workspace resolved-task scheduling tests | Passed, 4/4 |
| Zed base-keymap test with `runtime-shaders,rust-tools` | Passed, 1/1 |
| `cargo test -p icons` | Passed, 2/2 exhaustive asset inventory tests |
| `cargo test -p ui components::image::tests` | Passed, 2/2 exhaustive vector inventory tests |
| Full v1.16.1 asset path/blob audit | Passed, 463/463 exact; zero missing, renamed, or content-drifted upstream assets |
| New asset inventory | Exactly two additions: the Comfy default keymap and settings files |
| `cargo check -p zed --features runtime-shaders,rust-tools` | Passed after permitting the existing upstream WebRTC artifact download |
| `agent_ui` library suite | Passed, 428/428; the combined command then ran a separate headless `gpui_macos` pasteboard test which aborted on a null AppKit pasteboard |
| `ZED_STATELESS=1 cargo run -p zed --features runtime-shaders,rust-tools` | Built, passed initialization without asset-loading errors, remained active for observation, then intentionally interrupted |

## Remaining risk

1. Comfy cannot currently compile with its feature enabled until its evidence
   declarations and fixtures are reconciled by the Comfy feature owner; rewriting
   all hashes without validating fixture authority would be unsafe.
2. The 100 `missing_unresolved` endpoint paths require feature-owner decisions;
   they are not silently declared dropped or preserved.
3. Protocol tag relocation avoids v1.16 collisions but needs compatibility/version
   handling for communication with old Sim binaries.
4. Collaboration/Nostr, Comfy UI/media generation, real Git operations, remote
   structured tests, URL handling, visual branding, VoiceOver, and Linux/Windows
   platform behavior were not manually exercised.
5. Plain macOS shader builds still need Apple's optional Metal Toolchain. The
   runtime-shader launch is validated, but upstream defaults remain unchanged.
6. A standalone headless `gpui_macos` pasteboard test aborts outside a normal
   AppKit application context; the `agent_ui` tests themselves all passed.
