# Comfy-Desktop parity evidence

## Audit status

This report records the static and existing-test evidence gathered for the vendored Comfy-Desktop source. It deliberately preserves runtime, cloud, account, packaging-host, and target-Zed uncertainties. The core machine-readable feature ledger contains 206 independently testable capabilities. A second renderer-surface ledger adds 43 source-specific contracts for production Vue files that previously appeared only through broad parent mappings; it contains 41 functional surfaces and two explicit presentational/infrastructure dispositions. Every row has a stable identifier, source evidence, availability, evidence level, observable behavior, Zed status, acceptance statement, and validation approach.

The generated master catalog reconciles each Desktop row against the native-only Zed target. Existing-test evidence means that a source test expressly covers the behavior; it does not claim that the test ran in this environment.

## Source baseline

| Property | Evidence-backed value |
|---|---|
| Source root | `projects/comfy/Comfy-Desktop` |
| Git identity | No nested `.git` metadata is present, and the enclosing snapshot is not a Git work tree; no SHA is asserted. |
| Package | `comfyui-desktop-2` |
| Package version | `1.0.28` |
| Electron | `40.4.1` |
| Required runtimes | Node `>=22`; pnpm `>=10`; package manager `pnpm@10.28.1` |
| Source-tree file count | 735 files, excluding `node_modules`, `out`, and `dist` |
| Deterministic tree fingerprint | SHA-256 `2442854931f3a5a80e68aa55eab21a26dcefe868b4e875251a5b4d811668e448` |
| Fingerprint recipe | Sort every included relative file path bytewise, hash each file with SHA-256, then SHA-256 hash the resulting ordered digest stream. |
| Declared targets | Windows NSIS, macOS DMG/ZIP, Linux AppImage/DEB |

The product-level `AGENTS.md` and repository rules were read before discovery. The pass reconciled entry points, source-plugin registration, typed IPC and preload interfaces, handler registration, renderer listeners, menus, window events, settings schemas, persisted files, feature flags, locales, unit/integration/E2E tests, packaging scripts, and platform branches. README material was not used as sole behavioral evidence.

## Evidence and availability model

- `test-backed`: an existing test explicitly demonstrates the named behavior. Tests were inspected but not executed here.
- `code-inferred`: executable production code supports the behavior, but no directly matching test or safe runtime observation was found.
- `observed`: direct runtime confirmation. No feature receives this label in this pass.
- `documented-only` and `unverified`: retained by the shared methodology; no feature row was reduced to either label because the inspected executable source supported every inventoried row.

Availability uses the requested values: `active`, `conditional`, `platform-specific`, `experimental`, `developer-only`, `cloud/paid`, `deprecated/dead`, `infrastructure-only`, and `uncertain`.

## Inventory counts

### Features by domain

| Domain | Rows |
|---|---:|
| Source and installation | 35 |
| Onboarding and migration | 15 |
| Launch, process, and lifecycle | 25 |
| Window host and navigation | 15 |
| Terminal, logs, crash, and diagnostics | 16 |
| Updates, snapshots, and downloads | 40 |
| Settings, persistence, cloud, security, and UI | 45 |
| Platform, packaging, and recovery | 15 |
| **Total** | **206** |

### Features by availability

| Availability | Rows |
|---|---:|
| Active | 148 |
| Conditional | 17 |
| Platform-specific | 23 |
| Experimental | 2 |
| Developer-only | 3 |
| Cloud/paid | 8 |
| Deprecated/dead | 3 |
| Infrastructure-only | 1 |
| Uncertain | 1 |

### Features by evidence

| Evidence | Rows | Share |
|---|---:|---:|
| Test-backed | 187 | 90.8% |
| Code-inferred | 19 | 9.2% |
| Observed | 0 | 0.0% |

The 206 source rows retain their source evidence here; `features.csv`, `parity-matrix.md`, and `traceability.md` carry the reconciled native-only Zed status, gap, acceptance, task, and validation mappings.

### Renderer surface gap closure

[`catalogs/desktop-renderer-surfaces.csv`](catalogs/desktop-renderer-surfaces.csv) reconciles exactly 43 production `.vue` files that were present in `desktop-source-coverage.csv` but absent as direct source evidence from the other Desktop feature/derived catalogs. The deterministic generator [`generate_desktop_renderer_surfaces.py`](generate_desktop_renderer_surfaces.py) reproduces that candidate set while excluding its own output, derives stable path-hash IDs, extracts exact prop/emit/handler declarations, verifies focused same-component tests, and writes the same IDs back to source coverage.

| Renderer classification/evidence | Rows |
|---|---:|
| Functional | 41 |
| Presentational/infrastructure-only | 2 |
| Test-backed by a focused same-component test | 10 |
| Code-inferred | 33 |
| Active | 28 |
| Conditional | 12 |
| Cloud/paid | 1 |
| Infrastructure-only availability | 2 |

The functional rows include the system-modal host, title-popup menu/root, model-directory list, app updates, instance and snapshot pickers, context/dialog hosts, import/restore/diff/preview/inspector surfaces, cloud-choice modal, settings root, argument/channel/environment/path fields, action menus, status facts, and shared/per-instance storage. The micro-section layout wrapper and decorative Comfy C logo are the only infrastructure rows; their Zed status is `deferred` because they are validated through consuming GPUI surfaces rather than requiring standalone workflows.

The ledger retains source accessibility limitations instead of normalizing them away. In particular, `MenuView` has `tabindex=-1` without local keyboard activation, `ContextMenu` has buttons and Escape but no menu role or roving arrow focus, and snapshot diff/file-preview/inspector disclosure headings are click-only `div` elements without button/expanded keyboard semantics. Zed acceptance requires a keyboard-accessible path while preserving the observable state and action result.

## Registry reconciliation

| Registry or surface | Source count | Catalog count | Reconciliation |
|---|---:|---:|---|
| Source plugins | 6 | 6 | `standalone`, `portable`, `git`, `cloud`, `remote`, and `desktop` each have a row. |
| Current settings/schema keys | 27 | 27 | Every `KnownSettings`/`SETTINGS_SCHEMA` key has a default/fallback, persistence, availability, and migration note. |
| Discarded legacy setting keys | 4 | 4 | `primaryInstallId`, `pinnedInstallIds`, `maxCachedFiles`, and `closeDirectlyOnLastWindow` are explicit deprecated/dead load-time removal contracts; the first three have focused settings tests, while the fourth is code-inferred from the same executable removal loop. |
| Literal `ipcMain` registration sites | 180 | 180 represented | 114 `handle` plus 66 `on` sites; 178 distinct literal channels because two tooltip lifecycle channels intentionally have multiple listeners. |
| Computed picker-settings IPC registration sites | 26 | 26 represented | All `PICKER_SETTINGS_CHANNELS` members are resolved to their literal value and their computed registration/use. |
| All registered IPC channels | 204 distinct across 206 sites | 204 registered rows | 139 request/response plus 65 one-way registered channels. |
| Desktop IPC contract union | 273 | 273 | 139 request/response, 70 renderer-to-main one-way, and 64 main-to-renderer events. Five sender messages use WebContents `ipc-message` routing instead of global `ipcMain` registration. |
| Preload API members | 299 | 299 | Nine exposed bridge surfaces are enumerated member by member. |
| Menu/context actions | 45 | 45 | Title popup, context menu, native platform menus, dev roles, and disabled tray actions are represented individually. Each derived item/condition is `code-inferred` because no focused test was attached to every exact row. |
| Title-bar/chooser shell actions | 26 | 26 | Eight title-bar actions plus chooser search/create/pick/error and every conditional install-card action/open gesture are represented individually. |
| Window/application/WebContents/updater events | 44 | 44 | App, host, hosted Comfy, panel/title bar, checkout/auth child, embedded-popup, and updater events include effects and cleanup/recovery. The ledger explicitly includes `will-navigate`, `did-navigate-in-page`, `blur`, Firebase `console-message`, `update-available`, `update-downloaded`, and `download-progress`. |
| Keyboard/mouse/drop gestures | 19 | 19 | Hosted-view, terminal, modal/menu/card, snapshot drop, reorder, and dev shortcuts are represented. |
| Persisted formats/stores | 36 | 36 | Atomic JSON, YAML, markers, locks, snapshots, logs, browser partitions, IndexedDB, and session storage are represented, including the identity/first-launch/cloud-entry guards and per-install model-path YAML. |
| Production telemetry/event literals | 139 | 139 | The ledger contains 137 `comfy.desktop.*` literals plus `app:relaunch` and `app:user_logged_in`, with exact call/payload evidence, consent, redaction, rate limiting, provider routing, and tracked-step expansion. |
| Feature flags discovered in production | 3 | 3 | Two PostHog flags plus one desktop-managed Comfy CLI flag. |
| CLI flags and environment variables | 74 | 74 | 39 command-line flags and 35 environment variables are distinct from presentation variables. |
| Renderer/host CSS custom properties | 21 | 21 | Renderer variables and two CSS variables injected by the main host are explicitly classified rather than counted as CLI flags. |
| Production Vue renderer surfaces absent from prior direct catalog evidence | 43 | 43 | 41 functional contracts and two explicit presentational/infrastructure dispositions; all have exact props/emits/handlers and matching stable IDs in source coverage. |
| Localized scalar paths per locale | 1,176 English; 1,176 Chinese | 1,176 rows present in both | Key sets match. Array entries under `firstUse.whyCloud.benefits` are represented by indexed scalar paths. |
| Source tree | 735 files | 735 rows | Every vendored file is mapped to features or explicitly classified. |
| Test files | 232 | 232 | 191 `src` test files and 41 E2E spec files; the extracted ledger contains 3,422 suite-or-case declarations. |

The preload member split is: 158 `window.api`, 11 `window.__comfyDesktop2`, 8 terminal, 4 logs, 1 telemetry, 33 title-bar, 76 title-popup, 4 system-modal, and 4 title-tooltip/coachmark members.

[`catalogs/desktop-ipc.csv`](catalogs/desktop-ipc.csv) and [`catalogs/desktop-preload-apis.csv`](catalogs/desktop-preload-apis.csv) retain route-specific TypeScript handler/call/declaration excerpts, separated request-or-event and response-or-callback schema evidence, confidence, and an explicit unresolved-schema statement. These fields replace the former generic “typed by `ipc.ts`” placeholder. Static types do not prove structured-clone runtime values, serialized errors, callback ordering, unsubscribe cleanup, or version skew; `VAL-DESKTOP-001` must capture those variants.

The 735 source rows classify as 292 production, 180 infrastructure-only, 16 assets, 242 test-only/support, and 5 generated/declaration files. Test support/config files explain why the source classification has 242 test-only rows while the executable suite ledger has 232 files.

## Product behavior findings

### Installation sources and compatibility

The runtime registry contains six source plugins with distinct compatibility contracts:

- `standalone` manages an isolated local Python environment, ComfyUI checkout, Manager configuration, model/media paths, release selection, templates, updates, snapshots, and child lifecycle on Windows, macOS, and Linux.
- `portable` recognizes a Windows portable layout and embedded Python. It is Windows-only and hidden in packaged builds, so it is a developer/adoption path rather than a normal production picker tile.
- `git` launches a tracked source checkout and optional venv. It is hidden and skips the managed install workflow.
- `remote` persists a validated external ComfyUI URL and does not own a local inference process.
- `cloud` is seeded on every boot, uses the shared browser partition, and is subject to cloud-capacity and user-tier gates.
- `desktop` is a hidden Windows/macOS legacy-v1 adapter used for detection, adoption, and migration.

The source-defined workflow covers dependent wizard fields, GPU discovery, hardware/driver validation, stable tags, variant selection, express/manual install, install-path and free-space validation, probe/track, nested roots, copy, delete, rename, reorder, shared versus isolated models/input/output, Comfy CLI arguments, environment variables, model paths, and cancellable size scans. The catalogs retain cloud, hidden, legacy, OEM, mirror, and starter-template behavior rather than assuming local standalone is the only target.

### Onboarding, adoption, and migration

Startup derives a cohort from durable installations and `firstUseCompleted`. A new user sees cloud-versus-local choice, terms, optional templates, progress, and a title-bar coachmark. `desktop-first-use-fork-default` experimentally changes the default branch. Returning local users can bypass the fork; legacy Desktop detection gates a migrate-versus-install choice. Mid-flow cancellation does not mark first use done, so restart replays the flow.

Legacy adoption uses an acknowledgement/response IPC handshake before destructive work. Standalone layout migration, local snapshot migration, OEM workflow import, and interrupted-operation markers are separately inventoried. Snapshot-based installation validates the versioned file before selecting name, release, and variant.

### Process and session lifecycle

The externally visible core state machine is:

`stopped → launching → running → stopping → stopped`

Launch failure/cancellation returns to stopped state after process, reservation, and task cleanup; an unexpected running-process exit additionally retains crash detail for late-opening windows. Broadcasts cover launching, started, failed, stopping, stopped, progress, prompt, and crash state. Getter IPC hydrates windows that missed an earlier broadcast.

Managed launch uses the selected Python environment and typed Comfy arguments, merges source-specific environment paths, redacts sensitive command arguments, reserves the port, rechecks conflicts for time-of-check/time-of-use races, starts the process tree, waits for TCP readiness, and then navigates the hosted view. Key boundaries are a 300,000 ms boot timeout, three port retries, and five Manager reboot-marker retries. Conflict UI supports cancel, terminate the identified owner, or select the next port. Remote/cloud entries retain the same visible session model while omitting local process ownership.

Cancellation propagates through abort signals to downloads, subprocess waits, copy/update operations, and launch. Stop kills the child tree and releases port state. Quit and window-close paths consult renderer-owned overlays and configured confirmation before terminating sessions. OS shutdown suppresses update-on-quit to avoid deadlock/corruption.

### Windows, panels, and browser state

The shell supports several top-level windows, in-place chooser-to-install attachment, focus-instead-of-duplicate, explicit duplicate cloud windows where allowed, return-to-dashboard detachment, per-install bounds, last-session restore, startup-hidden reveal handshake, and source-requested unique browser partitions. Installation windows use `persist:shared` unless the record requests a unique `persist:<installation-id>` partition.

Each host owns a custom title bar, body/chooser panel, optional Comfy view, reused title popup, full-host system modal, tooltip/coachmark, checkout child/backdrop, and optional terminal/log popouts. Panel state covers chooser, new install, lifecycle, settings/directories/downloads/global settings, console, and hosted Comfy. Navigation, focus, and close consultation are explicit events rather than direct renderer window destruction.

Renderer failure and navigation failure are distinct: `render-process-gone` reports/reloads the renderer, while `did-fail-load` retries after two seconds. Retained session/crash getters prevent a refreshed/new window from losing state.

### Updates and rollback

App update checks run at startup and every ten minutes; the legacy `autoUpdate` setting no longer gates checks. Manual check, available, downloading, progress, downloaded/ready, install, and failed user-action states feed title-bar and settings UI. Windows can stage a downloaded version for the next startup, with a five-second bounded check, five-second minimum splash, and a version loop breaker. macOS preserves single-instance relaunch behavior. DEB/system-managed Linux packages report self-update unavailable; AppImage capability is distinguished.

Comfy release metadata uses schema-version-1 cache data, one-hour startup freshness, and a fifteen-minute periodic refresh. Managed Comfy updates create a pre-update snapshot and treat checkout plus dependency synchronization transactionally; dependency failure triggers rollback. Custom-node updates, release channel selection, interrupted-operation recovery, and post-operation state are distinct rows.

### Snapshots

Snapshot schema version 1 records triggers `boot`, `restart`, `manual`, `pre-update`, `post-update`, and `post-restore`. Users can list, inspect, diff against previous/current, create, delete, restore, export one, export all, preview import, inspect conflicts, confirm import, migrate a local/legacy install, or seed a new install. Export uses a `comfyui-desktop-2-snapshot` version-1 envelope. Restore is destructive-confirmed, stops active Comfy, applies repository/package/custom-node state, surfaces partial failure, and produces a post-restore record.

### Downloads

Hosted Comfy can request model and asset downloads through the restricted bridge. Model extensions are `.safetensors`, `.sft`, `.ckpt`, `.pth`, and `.pt`; destinations must resolve within configured model aliases/roots. Asset paths are sanitized and contained within allowed media roots. Filename collision handling, temporary sibling writes, sidecar resume metadata, and atomic rename prevent partial content from replacing a valid file.

The visible download state machine is:

`pending → downloading ↔ paused → completed | error | cancelled`

Each entry can expose bytes, percentage, speed, ETA, show-in-folder, pause, resume, cancel, retry, dismiss, and clear-finished actions. The title tray retains ten recent entries; a larger view remains open independently of popup focus. Completed images may expose a bounded 64-pixel thumbnail. Native taskbar/dock progress aggregates active transfers. Template downloads use a pool of three, two retries, and 1.05 disk-headroom factor; users can stop waiting while the work continues in the tray. Download attribution tokens never enter renderer broadcasts or logs.

### Settings and persisted state

There are 27 current setting keys. Required path/cache defaults are materialized; optional keys use consumer-defined fallback. `settings.json` and `installations.json` use temporary/backup atomic strategies and load-time recovery. Removed settings `primaryInstallId`, `pinnedInstallIds`, `maxCachedFiles`, and `closeDirectlyOnLastWindow` are discarded; stale `onAppClose: tray` becomes `quit` because tray creation is disabled. Installation migration converts legacy `useSharedPaths` into separate model and media sharing fields.

Other compatibility-bearing persistence includes window/session JSON, Windows data-location marker, release/fetch/experiment/tier caches, device/identity and download-attribution markers, `identity-migration-completed`, `first-launch-completed`, `cloud-entered-completed`, shared/per-install model-path YAML, process-owned port locks, managed-install and operation markers, snapshot files/manifest, standalone manifest/environment marker, partial download metadata, rotating logs, OEM manifest, update timestamp localStorage, browser partitions, Firebase IndexedDB, and post-sign-in sessionStorage. Exact filenames and lifecycle semantics are in `desktop-persistence.csv`; the port-lock implementation is in `src/main/lib/process.ts`.

### Authentication, cloud, telemetry, and security

Popup creation is allowlisted to Comfy checkout, production/dev Firebase auth, Google accounts, and GitHub OAuth prefixes; other HTTP(S) links are externalized. Firebase interception requires exact HTTPS handler hosts/path and only Google/GitHub providers. Its callback server binds `127.0.0.1` on fixed port 9876, caps the body at 64 KiB, sends no-store responses, and injects validated auth state into the shared partition. Checkout return detection uses exact/suffix-safe Comfy host checks and the child closes on Escape, explicit close, or a validated return navigation.

Hosted/panel/title/popup web content uses `nodeIntegration: false` and `contextIsolation: true`. Some trusted local preload views disable Chromium sandboxing because Rollup emits shared preload chunks; this is a stated source limitation, not a parity recommendation. Two constant `data:` overlays (checkout backdrop/close control) enable Node integration and non-isolated IPC, but main filters the sender WebContents identity and no remote content is loaded into those overlays. Zed should preserve the narrow capability boundary without reproducing Electron-specific sandbox concessions.

The context bridges do not expose raw Node or `ipcRenderer`. Main revalidates privileged IDs, actions, URLs, paths, popup messages, and file containment. The title-popup settings shim uses Electron's private `ipcMain._invokeHandlers` map; this is source-compatible with the pinned Electron but explicitly fragile and should become ordinary typed service calls in Zed.

Cloud capacity has `normal`, `degraded`, and `disabled` values; paid tier relaxes disabled to degraded confirmation while free tier is blocked. The 139-row telemetry/event ledger records every production literal, including 19 tracked-step bases that derive `.start`, `.end`, and `.error` wire names, ten separately literal derived failure names, 23 Datadog-mirrored names, and four infrastructure-only rows. The still-emitted `comfy.desktop.session.installation_started` compatibility shadow is explicitly `deprecated/dead`; no production literal qualified as developer-only or uncertain, and disabled tray behavior remains classified in the menu ledger. Telemetry uses tri-state consent, token/path/PII scrubbing, a per-event cap of 60/minute, and a process cap of 5,000 events; only the consent-decision event crosses the denied/undecided gate. The catalogs retain cloud/paid behavior, but this pass did not authenticate, bill, mutate a cloud account, or emit to either provider.

### Input, accessibility, and menus

Hosted Comfy intercepts Cmd/Ctrl+W for managed close, F5 and Cmd/Ctrl+R for reload, Cmd/Ctrl plus/minus for 0.5-step zoom, and Cmd/Ctrl+0 for reset. Terminal copy/paste/SIGINT differs by macOS, Windows, and Linux. Escape cancels the topmost modal/popup where permitted; menu/list/tab/card surfaces use arrows plus Enter/Space and maintain focus boundaries. Snapshot JSON accepts a single valid file by chooser or drop; invalid/multiple drops surface validation. Installation reorder and title-pill activation support pointer and keyboard paths. Status pills/banners expose text/icon state rather than color alone.

The action ledger contains title popup, link/image/edit context actions, macOS app/edit/window roles, non-mac fullscreen, development reload/devtools roles, and dead tray actions. Electron's built-in macOS `editMenu` expansion is recorded action-by-action but remains runtime-unobserved in this snapshot.

### Platform and packaging differences

- Windows uses an assisted NSIS installer, normally per-user, with optional directory/desktop shortcut and `/ALLUSERS`/`/CURRENTUSER`; it bootstraps the x64 payload and VC++ redistributable with UAC retry/ignore/abort. Data location depends on system versus non-system installation drive and persists in `data-location.json`. VC runtime and NTSTATUS diagnostics are Windows-specific.
- macOS ships DMG and ZIP with an arm64 bootstrap and notarization flow. Native app/edit/window menus, dock activation, traffic lights, fullscreen relayout, and updater relaunch differ from other platforms.
- Linux ships x64 AppImage and DEB. XDG config/data/cache/state migration is explicit. DEB hooks install/remove an AppArmor profile needed by affected Ubuntu Chromium user-namespace policy. DEB updates are system managed; AppImage self-update is separately detected.

Packaging configuration contains differing macOS category values across builder/provider configuration. Preserve this as a packaging decision to resolve, not a behavioral fact to normalize silently.

## Runtime and test constraints

No `node` executable or installed `node_modules` tree is present. A fallback `pnpm` launcher exists, but using it would require installing dependencies, which the mission forbids. Electron was therefore not launched, and source unit/integration/E2E suites were not run. No real account, paid service, credential, update server, installer, external mutation, or production process was used.

Consequences:

- Runtime-validation rate for the 206 feature rows is 0.0%.
- Runtime-validation rate for the 43 renderer-surface rows is also 0.0%; the ten `test-backed` rows reflect inspected tests, not a local test run.
- The 187 test-backed rows reflect inspected tests, not a green local run.
- Window-manager, native menu, updater-provider, GPU/driver, packaged preload, installer/UAC, AppArmor, OAuth browser, cloud capacity, and actual process-tree behavior need platform runtime validation.
- Network-unavailable behavior is code/test backed, but live latency, provider response variation, and TLS/proxy behavior remain unobserved.
- The runtime Comfy feature registry is version-dependent. Desktop currently injects `show_signin_button=true` only when the launched Comfy reports the flag; arbitrary user-authored recognized `--feature-flag` arguments are preserved.

## Explicit deprecated, hidden, and infrastructure classifications

- Legacy Desktop source: migration-only, hidden, `deprecated/dead` as a normal launch target.
- Tray `Show App`/`Quit`: implementation retained, but tray creation is disabled; `onAppClose: tray` is sanitized away.
- Portable source: Windows-only and hidden in packaged builds.
- Git source: hidden/developer-oriented.
- Developer reload/devtools/update-cycle and E2E override hooks: gated from production.
- `draggableList.ts`: tested pointer-drag helper with no production importer; classified infrastructure-only rather than an active interaction.
- Stale legacy settings/component paths and generated/declaration/assets are retained in source coverage with an explicit classification.

## Recommended Zed placement from desktop evidence

Desktop's observable lifecycle maps to native Zed components rather than its
Python/Electron implementation:

- `RuntimeSupervisor` owns Zed Rust workers, device groups, private IPC,
  readiness, cancellation fences, logs, crash recovery, and bounded restart.
  Legacy Desktop/Comfy installations are read-only migration sources and are
  never launched, updated, deleted, or connected as execution engines.
- Native profile, backend, model, plugin, compatibility-registry, codec,
  update, download, snapshot, diagnostics, provider, and cloud-capacity state
  use separate GPUI entities/services with explicit foreground boundaries.
  Filesystem, hashing, parsing, network provider, download, model, and worker
  work runs in owned background tasks or the Rust worker.
- Workflow/editor views are workspace items; node library, queue/history,
  assets/models, operations, worker status/logs, and diagnostics are dock
  panels; bounded profile, mapping, permission, update, auth, and destructive
  choices are modals/popovers.
- Typed versioned persistence imports only approved workflows, models, outputs,
  settings, snapshots, and plugin mappings. Electron IPC/private-handler and
  Python/pip/Git internals become native Rust service/action mappings,
  inactive legacy records, or explicit defers.
- Every failure reaches durable visible state. Cancellation aborts eligible
  tasks, waits on non-preemptible device fences, terminates only verified
  Zed-owned workers, cleans proven temporary state, and emits one terminal
  attempt/operation state.

The first native slice is the deterministic image workflow in the Rust worker,
including GPUI output, cache, cancellation, worker kill/recovery, restart, and
native-only package gates. Desktop installation/update/recovery surfaces follow
after that runtime exists; they operate on native backends, models, plugins,
codecs, registries, and Zed workers, never a Python test server.

## Open decisions and uncertainty queue

1. The native graph editor is the production surface. Unsupported imperative
   web hooks require Rust/WASM mappings or placeholders; no embedded renderer
   or browser fallback executes them.
2. How cloud authentication, checkout, billing, and provider telemetry are licensed/configured for Zed; no real account was used.
3. Which updater/distribution mechanism replaces Electron Builder/NSIS/DMG/AppImage/DEB while preserving staged update, system-managed, rollback, and recovery behavior.
4. Which hidden Git/portable/legacy source data is safe and useful for one-way
   import into native profiles versus retained as inactive compatibility-only
   evidence.
5. Which current Electron persisted files require direct import versus a one-way migration into Zed schemas.
6. Runtime confirmation of native menu expansion, focus order, screen-reader announcements, multi-display restore, duplicate-window rules, and close/OS-shutdown races on all three platforms.
7. Version negotiation and migration policy for private/conditional Electron
   contracts, legacy feature flags/Manager markers/custom-node updates, native
   worker IPC, Rust/WASM APIs, compatibility registries, and cloud providers.

## Generated artifacts

- [`catalogs/desktop-features.csv`](catalogs/desktop-features.csv) — stable feature ledger.
- [`catalogs/desktop-ipc.csv`](catalogs/desktop-ipc.csv) — individual Electron IPC channels and use sites.
- [`catalogs/desktop-preload-apis.csv`](catalogs/desktop-preload-apis.csv) — individual preload bridge members.
- [`catalogs/desktop-settings.csv`](catalogs/desktop-settings.csv) — current and discarded legacy setting keys, defaults/fallbacks, tests, and persistence/removal behavior.
- [`catalogs/desktop-source-plugins.csv`](catalogs/desktop-source-plugins.csv) — six source plugins and visibility/platform rules.
- [`catalogs/desktop-feature-flags.csv`](catalogs/desktop-feature-flags.csv) — discovered production flags.
- [`catalogs/desktop-cli-environment.csv`](catalogs/desktop-cli-environment.csv) — CLI flags, environment variables, and separately classified CSS custom properties referenced by production source.
- [`catalogs/desktop-persistence.csv`](catalogs/desktop-persistence.csv) — files, stores, formats, migrations, and recovery.
- [`catalogs/desktop-menu-actions.csv`](catalogs/desktop-menu-actions.csv) — native/custom/context/dead tray actions.
- [`catalogs/desktop-shell-actions.csv`](catalogs/desktop-shell-actions.csv) — title-bar, chooser, and installation-card actions.
- [`catalogs/desktop-window-events.csv`](catalogs/desktop-window-events.csv) — app/window/WebContents/popup/updater event contracts.
- [`catalogs/desktop-telemetry.csv`](catalogs/desktop-telemetry.csv) — production telemetry/event literals with exact payload and privacy/volume/provider contracts.
- [`catalogs/desktop-summary.json`](catalogs/desktop-summary.json) and [`catalogs/desktop-reconciliation.csv`](catalogs/desktop-reconciliation.csv) — machine-readable Desktop counts and registry closure.
- [`catalogs/desktop-keybindings-gestures.csv`](catalogs/desktop-keybindings-gestures.csv) — keyboard, pointer, and drop contracts.
- [`catalogs/desktop-platform-matrix.csv`](catalogs/desktop-platform-matrix.csv) — Windows/macOS/Linux packaging and behavior.
- [`catalogs/desktop-localization.csv`](catalogs/desktop-localization.csv) — 1,176 scalar localization paths and two-locale presence.
- [`catalogs/desktop-source-coverage.csv`](catalogs/desktop-source-coverage.csv) — all 735 source-tree files mapped or classified.
- [`catalogs/desktop-tests.csv`](catalogs/desktop-tests.csv) — 232 test files, extracted suite/case titles, feature mapping, and run status.
- [`catalogs/generate-desktop-catalogs.py`](catalogs/generate-desktop-catalogs.py) — deterministic catalog generator.
- [`catalogs/generate-desktop-telemetry.py`](catalogs/generate-desktop-telemetry.py) — deterministic production telemetry/event literal generator.
