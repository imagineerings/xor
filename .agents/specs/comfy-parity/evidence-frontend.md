# ComfyUI-Frontend evidence inventory

## Baseline

| Item | Value |
|---|---|
| Source root | `projects/comfy/ComfyUI-Frontend` |
| Package | `@comfyorg/comfyui-frontend` |
| Package version | `1.48.2` |
| Desktop UI package | `@comfyorg/desktop-ui` `0.0.6` |
| Website package | `@comfyorg/website` `0.0.1` |
| Git metadata | Unavailable at the nested source root; no SHA is claimed |
| Source fingerprint | `aeb208b759effdacf2ea3b1929f0a3e583201f0b7b3cb006f36f1007364b8ca3` |
| Fingerprint procedure | From the source root: `find . -type f -not -path './node_modules/*' -print0`, byte-sort paths, SHA-256 each file, then SHA-256 the ordered digest list |
| Installed runtime | `node_modules` absent |
| Runtime evidence produced in this audit | None |

The fingerprint includes hidden files and all source-tree files present at discovery time. `node_modules` was absent, so the exclusion did not omit installed content. The source tree is embedded in a larger workspace without nested Git metadata; package versions and the deterministic fingerprint are therefore the evidence-backed baseline.

## Evidence policy

- `observed` means directly exercised at runtime in this audit. There are no frontend rows at this level.
- `test-backed` means an existing Playwright, Vitest, component, property, or protocol test explicitly asserts the behavior. The test was inspected but not rerun.
- `code-inferred` means executable production code or a runtime registry defines the behavior without dynamic confirmation here.
- `documented-only` is reserved for claims found only in prose. Documentation files are accounted for in the source ledger, but no feature row relies solely on them.
- `unverified` is used when neither executable code nor a relevant test establishes the behavior. Open-world custom-extension behavior and runtime-only cloud outcomes remain explicit uncertainties rather than invented rows.

Availability uses the requested values: `active`, `conditional`, `platform-specific`, `experimental`, `developer-only`, `cloud/paid`, `deprecated/dead`, `infrastructure-only`, and `uncertain`.

## Method and constraints

The inventory reconciles bootstrap and routing, runtime registries, command and keybinding definitions, menu composition, settings schemas and definitions, WebSocket and HTTP contracts, workflow schemas and migrations, browser persistence, extension interfaces, feature flags, telemetry constants, localization keys, production source files, browser tests, component/unit tests, fixtures, snapshots, stories, documentation, build support, the desktop-ui package, and website routes.

The following constraints apply:

- No Comfy source was modified.
- No dependency was installed.
- No real account, credential, paid service, cloud mutation, or externally mutating request was used.
- `pnpm exec vitest --version` did not resolve with the absent install and was interrupted. No test command is reported as passed or failed.
- The two rows marked `uncertain` are pre-existing skipped/fixme Playwright declarations: the template thumbnail media requirement and publish-dialog tag suggestion interaction.
- The product-local frontend ledger originally recorded source-audit-time `uncertain` placeholders. The master generator now synchronizes those target-only columns to the native architecture decisions while preserving source behavior and evidence.

## Inventory totals

`catalogs/frontend-features.csv` contains the original 2,560-row feature ledger. Forty-nine rows are explicitly classified `coverage-anchor`; they exist only so every source file has a stable feature mapping. The remaining 2,511 rows are independently testable behaviors or discrete route, command, setting, keybinding, menu, protocol, format, migration, persistence, extension, feature-flag, or telemetry contracts.

Static orphan closure initially added 241 independently testable rows from the authoritative subcatalogs to the generated master ledger: 173 menu/item/infrastructure contracts, 24 browser-storage contracts, seven HTTP/static-template contracts, four previously omitted telemetry events, 32 independently identified UI-button telemetry side effects, and the omitted Desktop UI root route. The source-specific component and functional-module passes add another 1,157 rows, for a reconciled Frontend contribution of 3,958 rows: 2,356 test-backed, 1,599 code-inferred, and three documented-only disabled website navigation entries. `add_frontend_supplemental` generates the first 241 rows; the component and functional-module generators produce the other 1,157 without rewriting or duplicating existing feature IDs.

### Original frontend-features.csv by stable-ID domain

| Prefix | Rows |
|---|---:|
| `COMFY-GRAPH` | 750 |
| `COMFY-ASSET` | 390 |
| `COMFY-WORKFLOW` | 321 |
| `COMFY-CLOUD` | 291 |
| `COMFY-UI` | 287 |
| `COMFY-SETTING` | 262 |
| `COMFY-QUEUE` | 119 |
| `COMFY-FRONTEXT-EXT` | 114 |
| `COMFY-A11Y` | 26 |
| Total | 2,560 |

### Original frontend-features.csv by product surface

| Product surface | Rows |
|---|---:|
| Main ComfyUI frontend | 2,490 |
| Public website and Cloud marketing/payment routes | 59 |
| Desktop installer/maintenance UI | 11 |

### Original frontend-features.csv by evidence and availability

| Evidence level | Rows |
|---|---:|
| Test-backed | 2,017 |
| Code-inferred | 543 |
| Observed | 0 |

| Availability | Rows |
|---|---:|
| Active | 2,128 |
| Cloud/paid | 138 |
| Conditional | 98 |
| Infrastructure-only | 86 |
| Deprecated/dead | 33 |
| Platform-specific | 31 |
| Experimental | 26 |
| Developer-only | 18 |
| Uncertain | 2 |

### Original frontend-features.csv by row classification

| Classification | Rows |
|---|---:|
| Playwright-declared behavior | 1,677 |
| Setting | 152 |
| HTTP client contract | 142 |
| Command | 118 |
| Website/marketing route | 59 |
| Telemetry contract | 52 |
| Coverage anchor | 49 |
| Feature flag or remote-config key | 43 |
| Persisted browser state | 42 |
| Context/menu action | 41 |
| Keybinding | 34 |
| Core frontend extension | 32 |
| Frontend extension API member | 27 |
| WebSocket or local event contract | 24 |
| Application route | 22 |
| Menubar action | 22 |
| Persisted/import format | 16 |
| Migration | 8 |

## Bootstrap, distribution, and navigation evidence

`src/main.ts` performs these ordered operations:

1. Refresh anonymous `/api/features` remote configuration before Firebase or Vue initialization. The anonymous fetch aborts after five seconds and clears runtime config on failure.
2. Initialize cloud telemetry only for the Cloud distribution; initialize the host telemetry bridge when `window.__comfyDesktop2.Telemetry` exists.
3. Initialize Firebase, Sentry, PrimeVue, Pinia, vue-i18n, VueFire auth, toast service, global assertions, proxy-widget migration, and preview-node promotion.
4. Start store bootstrap, then mount the app.

`src/stores/bootstrapStore.ts` waits for Cloud auth initialization and authentication, initializes the server user, waits until user selection/login is resolved, fetches custom-node localization, loads server settings, and loads workflow metadata. `src/views/GraphView.vue` then registers core commands, menus, keybindings, sidebar tabs, bottom-panel tabs, queue polling, execution event bindings, locale changes, palette changes, server config, model folders, node frequencies, version checks, and reconnect refresh.

`src/router.ts` selects hash history for `file:` URLs, root history for Electron, the Vite base for Cloud, and the current path for reverse-proxy localhost deployments. It preserves query namespaces for templates, shares, invitations, workspace creation, OAuth, pricing, and desktop login. The desktop login code is stripped after capture. Cloud auth waits up to 16 seconds, allows only named public routes while logged out, preserves share attribution, redirects protected web routes to login, invokes the desktop sign-in dialog in desktop mode, and routes survey-required users through onboarding.

The route catalog contains 82 rows:

- 11 main-application routes: graph, user selection, login, signup, password recovery, survey, OAuth consent, user check, support failure, auth timeout, and subscribe redirect.
- 12 desktop-ui child routes covering the empty-child `/` WelcomeView, its `/welcome` alias, installation, Git download, desktop/server start and update, manual configuration, metrics consent, maintenance, unsupported hardware, and dialog popouts.
- 59 Astro website routes, including `/zh-CN` variants, Cloud pricing/enterprise/supported nodes, payment results, models, demos, downloads, privacy, terms, affiliates, customers, and marketing pages.

The website and payment rows are not silently excluded. Their native Zed
surface, approved provider mapping, documentation/navigation treatment, or
deliberate deferral remains a required decision; browser handoff does not
execute Comfy extensions.

## Shell and interaction surfaces

`src/views/GraphView.vue`, `src/components`, `src/renderer`, and the Playwright suites establish these surfaces:

- Graph canvas and optional App Mode/linear view.
- Topbar workflow tabs, overflow actions, current-user controls, and host action buttons.
- Floating or docked actionbar, queue/run controls, batch count, run-on-change, queue-front, and progress display.
- Left/right configurable sidebar with workflows, node library, model library, assets, apps, and conditional job history.
- Right-side panel with workflow overview, global settings, node parameters, node settings, node information, and errors.
- Bottom panels for essentials shortcuts, view controls, logs, and desktop command terminal; extensions can add tabs.
- Application menu, workflow actions, canvas menu, node menu, group menu, image menu, selection toolbox menu, job menu, asset menus, help menu, and Manager menus.
- Global dialogs, confirmation/prompt dialogs, error dialogs, settings, keybinding editing, sign-in, subscription, credits, sharing, publishing, templates, Manager, mask editor, 3D viewer, crop, model import, asset export, and queue-history clearing.
- Global and scoped toasts, reconnect banners, queue banners, release notifications, reroute migration warnings, invitation acceptance, Manager progress, Cloud promotion, error overlay, and right-panel error groups.

The menu ledger has 236 rows: the original 22 command-backed menubar placements and 41 workflow/node/selection/group/image actions, plus 173 statically reconciled job, asset, workspace/member, keybinding, App Builder, help, queue, filter/sort, website navigation, LiteGraph, custom-extension, data-driven consumer, and infrastructure contracts. Each actionable item records its surface, path, condition, action/target, definition, consumer, availability, and evidence. Non-action labels, generic renderers, extension hooks, disabled website TODO items, and the absence of a Desktop UI app-menu registry are classified explicitly instead of being counted as user actions.

## Graph editor behavior

The graph ledger is grounded in `src/lib/litegraph`, `src/renderer`, `src/core/graph`, `src/composables/graph`, `src/scripts/app.ts`, and 750 stable `COMFY-GRAPH` rows. Test-backed rows cover:

- Node creation from search, sidebar drag, clipboard, templates, media drop, custom frontend-only registrations, and model assets.
- Search ranking, category/source/input/output filters, bookmarks, essentials, frequency, keyboard selection, ghost placement, cursor following, and legacy search modes.
- Typed links, compatibility highlighting, drag-to-connect, rewiring, input conversion, link-release search/context behavior, auto-pan, collapsed-node links, hidden links, marker shapes, and reroute networks.
- Selection by click, modifiers, rectangle, group-child selection, select-all, live selection, focus, z-order, multi-node bounding box, and selection toolbox positioning.
- Move, snap-to-grid, keyboard nudging, resize, adjust-size, title edit, rename, duplicate, copy, paste, paste-with-connect, deletion, bypass-link repair, and clipboard-image priority.
- Node collapse, mute, bypass, pin, colors, shapes, opacity, badges, error state, advanced widgets, help, previews, tooltips, and lifecycle badges.
- Groups: creation, copy/paste, child selection, resize-to-contents, padding, colors, shapes, title editing, locking, and propagating always/never/bypass mode.
- Subgraphs: creation, nesting, navigation, breadcrumbs, URL hash validation, UUID identity, draft positions, slots, promoted inputs/widgets, duplicate independence, seed behavior, unpacking, publishing, library search, serialization, and muted/bypassed ancestor behavior.
- Native and legacy reroutes, floating endpoints, branching, native/legacy coexistence guard, spline offsets, and migration warnings.
- Undo/redo and graph-load suppression through `ChangeTracker`, including mask-editor-local history precedence.
- Pan, zoom, fit-to-selection, reset view, lock/unlock, navigation modes, mouse-wheel behavior, minimap interaction, viewport restore, renderer toggling, low-detail thresholds, and performance settings.

The app can render legacy LiteGraph nodes or the conditional Vue Nodes renderer. `Comfy.VueNodes.Enabled`, renderer metadata (`LG`, `Vue`, `Vue-corrected`), layout normalization, dedicated widget/layout/output stores, and extension callback compatibility are separately cataloged.

## Workflow formats, loading, validation, and persistence

`src/platform/workflow/validation/schemas/workflowSchema.ts` accepts:

- Schema `0.4`: numeric/string node IDs, array links, optional object floating links, groups, viewport/config extras, model descriptors, recursive subgraph definitions, and unknown passthrough fields.
- Schema `1`: object links, explicit graph state, reroutes, recursive subgraphs, UUID identities, revisions, exposed subgraph I/O and widgets, and passthrough fields.
- API prompt JSON: node-ID records with `class_type`, `inputs`, and `_meta.title`.

Version `1` selects schema 1. Any other numeric version is checked with the 0.4 schema. A missing/non-numeric version rejects. When workflow validation is enabled, schema and link validation attempt repairs and surface warnings, but a validation failure does not block loading the original graph.

`ComfyApp.handleFile` uses this precedence:

1. Native workflow metadata.
2. API prompt metadata.
3. A1111 `parameters` text.
4. If no workflow metadata exists and the file is image/audio/video, create the corresponding loader node and upload/paste the media.
5. Otherwise surface a file-load error.

The format ledger covers JSON 0.4, JSON 1, API prompt, PNG, AVIF, WebP, MP3, Ogg/Opus, FLAC, WebM, MP4/MOV/M4V, SVG, GLB, latent/safetensors, A1111 parameters, and node-template clipboard data. Existing fixtures cover normal metadata and non-finite JSON variants.

Load behavior also covers:

- Default-graph fallback for absent or non-object data.
- Missing node detection across root and nested subgraphs, with inactive nodes/ancestor paths excluded and unknown type names sanitized.
- Node replacement metadata and missing custom-node grouping by registry/repository ID.
- Missing model and media scans, async verification abort when switching workflows, warnings, downloads/import, and refresh after external media/model changes.
- Legacy KSampler widget normalization and renderer layout scaling.
- Viewport restore with fit fallback when no node intersects the restored visible area; templates always fit.
- Deferred warning display for shared/template flows.

Workflow persistence uses a two-layer V2 design:

- Workspace-scoped localStorage index `Comfy.Workflow.DraftIndex.v2:{workspaceId}`.
- Per-draft localStorage payload `Comfy.Workflow.Draft.v2:{workspaceId}:{hash(path)}`.
- Per-browser-tab sessionStorage pointers keyed by API client ID.
- localStorage copies of last active/open paths for browser restart restoration.
- Maximum 32 drafts and a 512 ms persistence debounce.

The V1-to-V2 migration preserves LRU ordering, skips failed payload writes, migrates tab pointers when a client ID exists, leaves V1 data in place for rollback until 2026-07-15, and creates an empty V2 index when no V1 data exists. Autosave supports `off` or `after delay`, coalesces changes during an active save, saves only modified persisted workflows, and logs failures without losing the scheduled follow-up state.

The browser-state ledger now contains 66 rows. The 24 closure rows cover reroute/type and floating-actionbar preferences, mask-brush settings, the raw `comfy_api_key`, website banner dismissals, current workspace/token/expiry, credit and subscription checkout markers, attribution identifiers, survey/feature-usage state, Manager conflict/UI state, OAuth correlation, one-shot pricing resumption, namespaced preserved queries, and all builder/sidebar/bottom-panel splitter keys. `comfy_api_key` and `Comfy.Workspace.Token` are explicitly secret-bearing source behaviors: parity does not justify reproducing raw browser-secret persistence in Zed. The native design must import them only through an explicit compatibility flow into the platform secret provider, persist opaque references, redact diagnostics, and preserve source clear/expiry/error behavior.

## Queue, execution, progress, errors, and recovery

`src/scripts/app.ts`, `src/scripts/api.ts`, `src/stores/executionStore.ts`, `src/stores/queueStore.ts`, and queue/browser tests establish:

- Prompt serialization includes API prompt, workflow metadata, client ID, optional partial execution targets, optional queue position/front, optional preview method, Comfy account auth token, API key, and usage-source marker.
- Batch queueing runs before/after widget callbacks and promoted-widget controls per generation so seed/control values can advance deterministically.
- Concurrent queue requests are stored and drained by one processor; the current implementation uses a stack (`push`/`pop`) for requests arriving while processing.
- Partial execution queues selected output targets and distinguishes callbacks from a whole-workflow run.
- Queue/history retrieval supports current running/pending jobs, paginated history, details, outputs, retries, clear/delete, job filtering, maximum history, overlay/sidebar views, and legacy/v2 panel modes.
- Single-job and bulk cancel APIs are explicit runtime-conditional contracts; interrupt accepts the active prompt ID as a hint; pending and history lists can be cleared.
- Status and execution-success messages refresh queue state and conditionally refresh assets.
- Reconnect shows notification state, refreshes queue/history, rejects stale job updates, and falls back to one-second `/prompt` polling when the initial WebSocket cannot open.
- Node progress, aggregate progress state, progress text, executing IDs, cached nodes, previews, output merges, success, interruption, and error are independently cataloged.
- Account preconditions open sign-in/subscription/credit dialogs and are excluded from the execution error count. Missing node type errors trigger a whole-graph rescan. HTTP 403 produces an access-restricted dialog. Node/prompt errors populate the right-panel/overlay path when enabled.

The WebSocket/event ledger contains 24 rows: 18 backend contracts and six frontend local events. Binary type 1 is legacy JPEG/PNG preview, type 3 is progress text with optional prompt metadata, and type 4 is metadata-prefixed preview that also emits the legacy preview event. Custom message types are dispatched only when a listener registered. Listener exceptions and rejected promises are isolated so one extension listener cannot stop others.

## Assets, editors, and viewers

The 390 `COMFY-ASSET` rows cover:

- Input/output/model assets with list/grid display, pagination, deduplication, filtering, sorting, type detection, selection, output stacks, previews, metadata, upload, rename, tag update, deletion, download, export, and progress/error state.
- Asset API and legacy file/model APIs, Cloud-owned/public assets, model-folder discovery, missing-model download/import, model metadata, model-to-node mappings, drag-to-canvas, and model selector surfaces.
- Missing media grouping, size/status display, resolver behavior, shared-workflow missing-media handling, and clearing loader widgets after deletion.
- Mask editor brush layers, dominant-axis adjustment, brush size, color picker, rotate, mirror, bucket, undo/redo, load/save, cancellation, and external state.
- Painter, image crop, image compare, bounding-box widgets, webcam capture, microphone recording, waveform audio player, HDR viewer, GLSL preview, image/video/text previews, and result gallery.
- 3D beta viewer for GLB/FBX/OBJ/STL/PLY, camera modes, lights, grid, background, HDRI, animation, gizmos, LOD, point-cloud engines, serialization/cache, recording, and mesh export.

## Commands, keybindings, focus, and accessibility

The command registry has 118 localized command IDs. Core commands use `commandStore.execute`, which awaits async handlers, supplies optional metadata/error handlers, and throws for unknown IDs. Extensions can register commands with source attribution. Dynamic sidebar and bottom-panel commands are reconciled to their generating stores.

The default keybinding registry has 34 rows. It includes queue/run-front/interrupt, node-definition refresh, sidebar tabs, App Mode, save/open/group/settings, zoom/fit, pin/collapse/bypass/mute, logs/shortcuts panels, convert-to-subgraph, minimap, canvas lock/unlock, subgraph exit, select-all, paste-with-connect, and Delete/Backspace. Target element scopes are recorded.

The keybinding service and tests establish:

- Editable controls retain browser text-editing chords.
- Browser-reserved chords are rejected by the editor.
- Dialog and Escape handling take precedence over canvas/global dispatch.
- Canvas-scoped bindings require the configured target/focus context.
- User-added/unset bindings and the current preset persist as settings.
- Keybinding editing detects conflicts and unsaved changes.
- Canvas mode selector, menus, popovers, dialogs, tabs, form controls, and node controls expose ARIA labels/roles/states and focus restoration where tested.
- `Comfy.Appearance.DisableAnimations` defaults from `prefers-reduced-motion` and disables most CSS animation/transition work.

The source scan found 1,456 production references to ARIA, roles, tabindex, focus, keyboard, accessibility, or Escape behavior. The 26 `COMFY-A11Y` rows include independently asserted focus and ARIA interactions; source files without a discrete browser assertion map to the accessibility coverage anchors and remain listed in the source ledger.

## Settings, palettes, localization, and flags

The settings schema has 152 production IDs after removing three entries explicitly labeled for tests. Of these, 149 have a literal registration/definition in production source. Three compatibility keys are schema-only and remain explicit uncertainties:

- `Comfy.RerouteBeta`
- `LiteGraph.Pointer.TrackpadGestures`
- `VHS.AdvancedPreviews`

There are 112 English setting labels. Forty schema settings intentionally have no English settings-panel label because they are hidden state, persistence metadata, extension/renderer compatibility, or programmatic controls. `frontend-settings.csv` records type, UI type, default expression, version, availability, source, and test for every schema ID.

Settings behavior includes:

- Load values before registration with three retries and exponential delay capped at eight seconds.
- Deep-clone reads/writes to prevent external mutation.
- Install-version-specific defaults and deprecated-value migration.
- No-op when an effective value does not change.
- Batched and single persistence through `/api/settings`.
- `onChange`, legacy change event, and telemetry after local application.
- Duplicate registration warns and keeps the first registration.
- Hidden, deprecated, experimental, distribution-specific, and renderer-specific visibility.

Thirteen locales are present: Arabic, English, Spanish, Persian, French, Hebrew, Japanese, Korean, Brazilian Portuguese, Russian, Turkish, Simplified Chinese, and Traditional Chinese. Each has `main`, `commands`, `settings`, and `nodeDefs` namespaces. The compact localization ledger has 12,586 unique flattened keys and records locale coverage, missing locales, extra non-English keys, and values identical to English.

The flag catalog contains 43 unique keys from three sources:

- Client WebSocket hello capabilities.
- Known server feature flags.
- Remote runtime configuration.

Flag precedence is dev override, remote configuration where applicable, server feature, then consumer default. Team workspace and consolidated billing flags are Cloud-only auth-gated flags that use cached values during the anonymous-to-authenticated window. Nightly/dev builds force selected experimental features. The catalog also records Firebase, telemetry-provider, Turnstile, upload, sharing, ComfyHub, model, asset, secrets, onboarding, subscription, Sentry, health-alert, and endpoint-base configuration keys.

## Frontend extension and custom-node compatibility

`ComfyExtension` exposes 27 discrete members in the catalog:

- Contributions: commands, keybindings, menu commands, settings, bottom-panel tabs, about badges, topbar badges, and actionbar buttons.
- Lifecycle: `init`, `setup`, node-definition augmentation, custom widgets, custom node registration, before/after graph configuration, loaded-node and created-node callbacks.
- Interaction: selection-toolbox commands and canvas/node context menu items.
- Experimental auth callbacks: user resolved, token refreshed, and logout.
- Output update callback.

Extensions load from `/api/extensions`. Core modules load first. Custom extension modules import concurrently. Failed imports are logged without stopping other modules. Enabled extension async hooks run concurrently through `Promise.all`; synchronous throws and async rejections are isolated. Disabled names persist in `Comfy.Extension.Disabled` and require reload. Four known obsolete/conflicting extensions are always disabled: `pysssss.Locking`, `pysssss.SnapToGrid`, `pysssss.FaviconStatus`, and `KJNodes.browserstatus`.

Thirty-two core extension modules are cataloged, including clipspace, context filtering, bounding boxes, widgets, prompts, attention editing, Electron adaptation, groups, image compare/crop, lazy 3D, mask editor, templates, notes, painter, preview, reroute, text/image/mesh output, touch, uploads, recording, webcam, and widget inputs, plus Cloud/nightly-gated modules.

The compatibility surface remains open-world:

- `ComfyExtension` has an index signature.
- Custom nodes can register frontend-only LiteGraph node types.
- Custom WebSocket message types dispatch when listeners register.
- Node execution outputs permit arbitrary passthrough keys.
- LiteGraph node callbacks, widget arrays, node serialization, graph versioning, and legacy context menus are ecosystem contracts.

Production Zed uses an explicit compatibility boundary: versioned Rust/WASM contributions with explicit ports, deterministic legacy identifier mappings, preserved unknown payloads, and visible unsupported placeholders. Native GPUI cannot execute unchanged JavaScript, DOM, or LiteGraph hooks, so no real or managed browser/frontend execution path is permitted; the source contracts remain development-oracle evidence and migration inputs.

## Cloud, paid, telemetry, surveys, and workspace surfaces

Cloud/paid rows are retained for authentication, OAuth, session-cookie minting, unified-token remint/retry, signup Turnstile, email/password/social auth, user checks, onboarding survey, waitlist/support, subscriptions, tiers, checkout, credits, top-up, cancellation, resubscribe, account preconditions, personal/team workspaces, invitations, roles, workspace tokens, billing operations, workflow sharing, asset consent, ComfyHub publish/profile, user secrets, server health alerts, telemetry, feedback, and surveys.

Telemetry is compiled/gated by distribution and dispatched through an error-isolated registry. The catalog contains 56 event contracts and 32 distinct literal UI-button identifiers, for 88 unique stable IDs. Duplicate emission sites for `queue_run_multiple_batches_submitted` and `error_dialog_closed` are consolidated into their respective identifier rows instead of reusing the umbrella `COMFY-CLOUD-059`. In addition to the typed application events, the catalog retains Desktop UI `install_stepper_change`, Desktop-hosted graph `execution`, website `$pageview`, and website `website:download_button_clicked`. The event set covers auth, subscription, checkout, credits, workspace invitations, surveys, email verification, templates, workflow import/open/save/create/share, App Mode, visibility/tab/shell layout, node search/add, settings, help, execution, install progress, downloads, generic UI clicks, and page views. Product behavior must not depend on telemetry success.

No cloud/paid row was dynamically exercised. Zed must explicitly choose retained, native-provider-mapped, documentation/navigation-only, or deferred behavior for each row.

## HTTP and protocol client usage

`frontend-http-usage.csv` has 149 call-contract rows. It includes literal `fetchApi`, `apiURL`, `internalURL`, and Axios calls plus manually reconciled dynamic assets, tasks, Manager, secrets, sharing, and ComfyHub paths. The closure rows retain same-origin credentialed OAuth consent GET/POST plus static template index, logo index, validated logo URL, preview-media URL, and workflow-JSON consumers. `URL` in the method column means the source constructs a URL that another client or media element consumes; it is not an inferred HTTP verb.

Core contracts include extensions, templates, embeddings, object info, prompt submission/status, assets from workflow, models and metadata, queue/history/jobs, system stats, interrupt/free, users, settings, userdata, global subgraphs, logs, folders, i18n, uploads, views, features, node replacements, tasks, secrets, workspaces/invites/members/billing, Manager v2 queue and custom-node endpoints, sharing/import, and ComfyHub.

Dynamic path constants and open-world custom routes cannot be proven by a literal scan. They are retained as templated rows (`{param}`) and must be reconciled with the backend and desktop inventories.

## Source-specific component and functional-module contracts

[`catalogs/frontend-component-surfaces.csv`](catalogs/frontend-component-surfaces.csv) closes every normalized production, cloud, or Desktop-UI Vue file that previously had only a broad coverage anchor. It contains 805 stable rows: the exact 804-file audit predicate plus one required `AssetsSidebarTab` override that was already cited elsewhere. The ledger classifies 691 independently functional surfaces and 114 source-specific render/presentation infrastructure dispositions. Every row records exact props, models, emits, template events, handlers, state guards, visible success, failure/recovery, accessibility, persistence/interfaces, availability, and validation. All 805 are `code-inferred`; adjacent tests remain cited but are not treated as proof of every extracted component behavior.

[`catalogs/frontend-functional-modules.csv`](catalogs/frontend-functional-modules.csv) closes 352 broad-anchor-only production service, store, and composable TypeScript modules after excluding tests, types, configs, and pure build support. It contains 339 functional state/service contracts and 13 explicit source-specific infrastructure dispositions. Availability is 295 active, 41 cloud/paid, 3 Desktop-UI platform-specific, and 13 infrastructure-only; evidence is 232 focused same-module `test-backed` rows and 120 `code-inferred` rows. The ledger records exports, actions, guards, reactive/async lifetime, error/cancel/retry behavior, persistence, I/O effects, source digest, and an independent fixture command for each module.

The master generator idempotently appends each component/module ID to its exact `frontend-source-files.csv` row while preserving other feature IDs. These two ledgers prevent functional pages, panels, viewers, editors, queue/assets/workflow/auth/billing/workspace services, graph/widget logic, and Manager stores from being hidden under a product-wide anchor.

Together with the 241 registry/persistence/menu/telemetry/route/HTTP supplements, the Frontend contributes 3,958 master feature rows: 2,560 primary rows plus 1,398 source-specific supplements.

## Source-file and test coverage

`frontend-source-files.csv` accounts for 4,697 files, including hidden files that `rg --files` omits by default:

| Classification | Files |
|---|---:|
| Production source/style/localization | 1,800 |
| Test-only tests, fixtures, snapshots, and stories | 1,947 |
| Infrastructure/static assets/build support | 416 |
| Public website and Cloud production source | 328 |
| Documentation/guidance | 150 |
| Desktop-ui platform source | 54 |
| Generated contract source | 2 |
| Total | 4,697 |

The classification reconciliation treats 789 non-document `browser_tests/**` files as test-only, including JSON workflows and binary media fixtures; its six Markdown guidance files remain documented-only. Storybook setup, website E2E viewport data, and lint, typecheck, Playwright, Vitest, Knip, Vite, Astro, and package configuration are infrastructure/test support rather than production capability evidence. Feature mappings remain attached so fixtures still support their behavioral contracts without inflating production counts.

By source product, 4,111 files belong to the main frontend tree and repository support, 499 to the website, and 87 to desktop-ui. Each row has a classification, reason, stable feature mapping, extension, and byte count.

Test discovery found:

- 245 Playwright spec files.
- 1,677 literal declared Playwright test cases mapped one-to-one to stable IDs.
- 1,013 additional unit/component/spec files.
- 77 Storybook files.

Three Playwright files use generated/indirect test declarations not captured as literal `test("...")` rows, but the files remain mapped in the source ledger:

- `browser_tests/tests/assetDeleteClearsLoadImage.spec.ts`
- `browser_tests/tests/propertiesPanel/errorsTabMissingMediaRuntime.spec.ts`
- `browser_tests/tests/subgraph/subgraphSeed.spec.ts`

## Registry reconciliation

The machine-readable reconciliation is `catalogs/frontend-reconciliation.csv`. After the dynamic command refresh, the expected results are:

| Registry | Discovered | Mapped | Delta |
|---|---:|---:|---:|
| Localized commands | 118 | 118 | 0 |
| Default keybindings | 34 | 34 | 0 |
| Menubar/context actions and infrastructure | 236 | 236 | 0 |
| Production settings schema IDs | 152 | 149 literal definitions | 3 explicit compatibility uncertainties |
| Localized settings | 112 | 112 schema matches | 0 orphan labels |
| Routes | 82 | 82 | 0 |
| WebSocket/local event contracts | 24 | 24 | 0 |
| HTTP call contracts | 149 | 149 | 0 |
| Feature/config flags | 43 | 43 | 0 |
| Telemetry events and literal button IDs | 88 | 88 | 0 |
| Formats and migrations | 24 | 24 | 0 |
| Browser persistence rows | 66 | 66 | 0 |
| Extension API/core module rows | 59 | 59 | 0 |
| Broad-anchor-only production/cloud/platform Vue files | 804 | 804 source-specific rows | 0 |
| Required already-referenced Vue override | 1 | 1 source-specific row | 0 |
| Functional Vue component surfaces | 691 | 691 | 0 |
| Presentational Vue infrastructure dispositions | 114 | 114 | 0 |
| Functional-module predicate candidates | 352 | 352 source-specific rows | 0 |
| Functional module capabilities | 339 | 339 | 0 |
| Functional module infrastructure dispositions | 13 | 13 | 0 |
| Playwright files with literal tests | 245 | 242 | 3 indirect/generated declaration files |
| Playwright literal cases | 1,677 | 1,677 | 0 |
| Source files | 4,697 | 4,697 | 0 |

The three schema-only settings, three indirect Playwright files, open-world extension keys, dynamic custom routes, absent runtime install, and all Zed-status values are preserved as uncertainties. Extension interface members 037–063 cite `extensionAPI.spec.ts` only for the six members it directly exercises; the other 21 members are code-inferred and no longer cite the stale nonexistent extension-service test path. They are not silently counted as verified parity.

## Zed migration implications for the lead design

The frontend evidence maps to a fully native production strategy:

- Use idiomatic GPUI entities, actions, focus contexts, menus, dock items,
  workspace items, modals/popovers, background tasks, and versioned persisted
  models for every native shell and editing surface.
- Bind graph nodes and widgets to compiled built-in descriptors and versioned
  Rust/WASM manifests. Native runtime commands own prompt, queue, history,
  progress, models, assets, settings, and execution; the public HTTP/WebSocket
  host is an automation projection rather than the UI's internal transport.
- Classify every frontend extension hook as declarative Rust/WASM contribution,
  deterministic legacy mapping, preserved placeholder, documented-only claim,
  or explicit defer. Python, JavaScript, DOM, LiteGraph imperative hooks,
  arbitrary web directories, embedded WebViews, and external-browser handoff
  do not execute compatibility code.
- Implement tensor/model/device/sampler/node behavior in the Zed-owned Rust
  worker and validate source protocols/object-info only as conformance outputs.
- Treat Desktop lifecycle and public website/cloud surfaces as separately
  mapped native/platform/provider capabilities, not assumptions about graph
  architecture.

The first frontend vertical slice opens and edits the native image graph
`LoadImage -> ImageScale -> ImageInvert -> PreviewImage -> SaveImage`, runs it
through CPU tensors in the Rust worker, shows queue/progress/preview/output,
cancels, recovers a killed worker, saves/reopens, and passes keyboard,
accessibility, visual, persistence, and native-only release checks. The second
slice adds the shape-reduced native diffusion workflow.

## Catalog index

| Artifact | Purpose |
|---|---|
| `catalogs/frontend-features.csv` | Canonical per-feature ledger with evidence, behavior, failure/recovery, Zed status/gap, acceptance, and validation fields |
| `catalogs/frontend-component-surfaces.csv` | Source-specific Vue component contracts or explicit presentation/infrastructure dispositions |
| `catalogs/frontend-functional-modules.csv` | Source-specific service/store/composable state and side-effect contracts or infrastructure dispositions |
| `catalogs/frontend-test-cases.csv` | One row per literal Playwright test declaration |
| `catalogs/frontend-commands.csv` | Command IDs, labels, source, availability, and tests |
| `catalogs/frontend-keybindings.csv` | Default chords, commands, and target scopes |
| `catalogs/frontend-menus.csv` | Menubar and context-menu action surfaces |
| `catalogs/frontend-settings.csv` | Production settings schema, UI type, defaults, versions, availability, source, and test evidence |
| `catalogs/frontend-feature-flags.csv` | Client, server, and remote configuration keys |
| `catalogs/frontend-routes.csv` | Main app, desktop-ui, and website routes |
| `catalogs/frontend-websocket.csv` | Typed backend, binary, and frontend-local events |
| `catalogs/frontend-http-usage.csv` | Frontend HTTP/URL contracts and templated dynamic paths |
| `catalogs/frontend-formats-migrations.csv` | Workflow/media formats and migrations |
| `catalogs/frontend-persisted-state.csv` | Browser storage keys and dynamic patterns |
| `catalogs/frontend-extensions.csv` | ComfyExtension members and core extension modules |
| `catalogs/frontend-telemetry.csv` | Telemetry event constants and literal UI button IDs |
| `catalogs/frontend-localization.csv` | Compact per-key locale coverage and orphan analysis |
| `catalogs/frontend-source-files.csv` | Every source-tree file with classification and feature mapping |
| `catalogs/frontend-reconciliation.csv` | Registry-to-ledger counts and deltas |
| `catalogs/frontend-summary.json` | Baseline, counts, orphan sets, and runtime constraint summary |

## Remaining verification work

- Install the exact lockfile dependency graph in an authorized disposable environment and run targeted unit, browser, website, and desktop-ui suites.
- Launch a deterministic local ComfyUI backend and exercise workflow import/export, queueing, previews, invalid prompts, interruption, cancel, reconnect, stale messages, missing assets/models/nodes, external media change, browser restart, and storage corruption.
- Exercise Cloud routes only with approved test accounts and non-billable fixtures.
- Resolve the three schema-only settings against runtime server values and extension registrations.
- Reconcile templated frontend HTTP rows against the backend route catalog and desktop IPC/host contracts.
- Re-run the master generator after any frontend-ledger change and require its synchronized target status, evidence, decision, gap, and acceptance columns to match the master row.
