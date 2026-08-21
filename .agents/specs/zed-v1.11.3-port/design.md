# Design: Zed v1.11.3 upstream port

## Overview

The port uses the verified upstream object database as immutable evidence and
the current Zed filesystem as the only downstream authority. It reconciles
commit history and endpoint trees separately because the release tags are
sibling branches. Applicable endpoint behavior is delivered in ordered
subsystem increments. File replacement is allowed only for a current blob that
exactly matches upstream v1.10.2; all other overlaps use a three-way,
symbol-aware Zed adaptation.

## Decisions

### D1: Separate patch identity from endpoint behavior

- Choice: inventory all 160 right-exclusive commits, use patch-id equivalence to
  identify changes already on v1.10.2, and use the 425-path endpoint diff for
  final file/behavior closure.
- Rationale: the tags share merge base
  `3648fe6f19644fa4fcbd4f1db3e5888efb8269c1` but neither requested tag is the
  other's ancestor.
- Consequence: eight equivalent commits and the complete revert pair remain
  visible without being reapplied.

### D2: Content-addressed preservation and three-way adaptation

- Choice: hash each affected current path as a Git blob. Copy an upstream target
  unchanged only when current content equals its upstream v1.10.2 blob. For a
  diverged path, merge `current Zed` with `upstream v1.10.2 -> v1.11.3` at symbol
  granularity and resolve conflicts manually.
- Rationale: local Git status/history is unavailable, so the whole current
  filesystem must be treated as authoritative pre-existing work.
- Consequence: no bulk checkout, blind cherry-pick, or remote Zed comparison is
  used. Before/after content manifests provide rollback/audit evidence.

### D3: Deliver subsystem increments in dependency order

- Choice: execute runtime/platform, editor/language, Git/project, agent/services,
  and delivery-shell increments sequentially, followed by an independent audit.
- Rationale: shared manifests, GPUI APIs, editor/diff primitives, provider APIs,
  and the app shell create real dependencies and overlapping write sets.
- Consequence: task waves are intentionally serialized even where read-only
  review was parallel.

### D4: Translate Zed shell boundaries into Zed boundaries

- Choice: adapt `crates/zed/**` to `crates/zed/**`, `zed_actions` to
  `zed_actions`, `zed_urls.rs` to `zed_urls.rs`, and any remaining symbol/action
  names idiomatically. Preserve Zed URLs, bundle IDs, release channels,
  credentials, stateless mode, sidebar, and Comfy registration.
- Rationale: these are intentional downstream architectural/product seams.
- Consequence: applicable underlying behavior is ported; Zed merchandise and
  organization automation remain excluded.

### D5: Preserve compatibility-bearing data and generated pairs

- Choice: update Cargo manifests with the lockfile; `.proto` sources with
  generated Rust; settings/defaults with settings content and all-settings docs;
  workflow YAML with xtask generators; keymaps with action registries; and
  licenses with introduced tooling crates.
- Rationale: partial updates create wire, persistence, schema, or build drift.
- Consequence: no database migration is needed in this range, but protocol
  fields, settings defaults, actions, and generated outputs receive explicit
  validation.

### D6: Treat upstream tests as behavior evidence

- Choice: port dedicated upstream tests and retain hunk-level test evidence for
  commits that modify inline tests. Add Zed regression coverage when adaptation
  crosses a local seam.
- Rationale: compile success alone cannot establish the ported behavior.
- Consequence: GPUI tests use deterministic seeds/executors; platform-only
  behavior not executable on macOS is recorded for external CI rather than
  marked passed.

### D7: Exclusions require negative verification

- Choice: verify excluded Guild/community/ranking automation and the Zed merch
  action remain absent, while proving no accepted dependency requires them.
- Rationale: explicit non-application is part of completeness.
- Consequence: exclusions are covered by the delivery-shell task and final
  audit, not silently omitted.

### D8: Keep specifications living through implementation

- Choice: update requirements, decisions, task manifests, matrix decisions,
  traceability, and validation evidence whenever implementation discovery
  changes actual behavior or write scope.
- Rationale: the current tree is heavily diverged and implementation can expose
  assumptions that static comparison cannot.
- Consequence: re-run the complete spec validator after every material replan
  and at completion.

### Implementation discovery: ACP configuration is no longer beta-gated

- The v1.11.3 provider/configuration API makes boolean ACP options available
  unconditionally in the retained Zed agent UI. Tests now exercise boolean
  cycling directly instead of installing the obsolete `AcpBetaFeatureFlag`.
- Language-model selector tests use provider/model-specific queries because the
  current fuzzy matcher legitimately returns additional models for broad
  fragments such as `mini` and `ol`; this preserves behavior while making the
  regression deterministic.

## Components and flow

| Component | Input | Output / responsibility |
| --- | --- | --- |
| Upstream evidence store | Verified Zed tags | Commit patches, trees, symbols, tests, patch identity |
| Inventory and port matrix | Upstream evidence + local content hashes | Stable ZUP decisions and file closure |
| Runtime/platform port | Dependency/runtime/GPUI/platform rows | Foundation APIs and platform fixes |
| Editor/language port | Editor/language/search/terminal rows | User behavior and focused tests |
| Git/project port | Worktree/protocol/Git/workspace rows | Repository semantics and Zed workspace adaptation |
| Agent/services port | Agent/provider/context/collab rows | Retained service behavior with Zed credentials/endpoints |
| Delivery-shell port | App/settings/assets/docs/tooling/version rows | Zed-branded integration and paired generated output |
| Completeness audit | All artifacts and implementation evidence | Forward/reverse closure and final gates |

## Failure and recovery

- If a current path changes after discovery, recompute its content relation and
  replan that path before editing it.
- If a three-way merge conflicts, leave the authoritative local behavior intact
  until the upstream invariant and local seam are both understood; do not choose
  an entire side wholesale.
- If an accepted change cannot compile because a prerequisite is missing, stop
  dependent tasks, update dependencies/design/task manifests, and revalidate.
- If a platform test cannot run on macOS arm64, run static/current-host checks,
  record the exact missing platform, and keep completion contingent on available
  cross-platform evidence rather than adding a skip.
- If broad validation exposes unrelated pre-existing failure, reproduce it
  against the pre-port preservation evidence where possible and distinguish it
  from port regressions without discarding local changes.

## Platform and compatibility strategy

- Current local execution host: Darwin 25.5.0, arm64, Rust 1.95.0.
- macOS paths receive native tests/manual checks where available.
- Windows, Linux, Wayland, X11, and headless branches receive compile/static
  checks available from the workspace and explicit external-CI coverage notes.
- Protocol additions must keep older/default field behavior compatible.
- Settings/keymaps/actions must preserve deprecated aliases and unknown persisted
  values where the current implementation supports them.

## Traceability

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D2, D3, D5 / runtime port | Build, unit, integration | `VAL-RUNTIME` and lockfile checks pass |
| 1.2 | D3, D6 / platform branches | Platform, static, manual | `VAL-PLATFORM` records every touched OS branch |
| 2.1 | D2, D3, D6 / editor port | Unit, GPUI, grammar | `VAL-EDITOR` passes focused behavior suites |
| 2.2 | D5, D6 / settings and tests | Regression, static | Upstream/adapted tests and schema checks pass |
| 3.1 | D3, D5 / worktree and proto | Integration, compatibility | `VAL-GIT` covers linked/bare worktrees and proto output |
| 3.2 | D3, D6 / Git and project UI | Unit, GPUI, integration | staging/diff/search/navigation tests pass |
| 3.3 | D2, D4 / Zed workspace seams | GPUI, persistence, regression | multi-workspace/sidebar/Comfy checks pass |
| 4.1 | D3, D5, D6 / agent providers | Unit, schema, integration | `VAL-AGENT` provider and tool-schema tests pass |
| 4.2 | D4, D5 / retained services | Integration, protocol | OAuth/cloud/LiveKit/context tests pass with fakes |
| 4.3 | D2, D4 / Zed service seams | Regression, manual | credentials/sidebar/endpoints remain Zed-native |
| 5.1 | D4, D5 / product shell | GPUI, schema, docs | `VAL-SHELL` action/keymap/settings/docs checks pass |
| 5.2 | D5, D7 / delivery tooling | Static, generated-pair | workflow/tooling/license outputs are synchronized |
| 5.3 | D1, D4, D7 / release state | Static, negative | version is 1.11.3; excluded/equivalent rows verified |
| 6.1 | D1 / ledgers | Static completeness | 160/425/net-unchanged counts reconcile |
| 6.2 | D8 / living traceability | Static forward/reverse | `traceability.md` has no orphan criterion/ZUP/task/check |
| 6.3 | D6, D8 / validation gates | Repository validation | format, tests, clippy, spec validator pass |
| 6.4 | D2, D8 / preservation | Content audit | no unexplained writes outside declared scope |
