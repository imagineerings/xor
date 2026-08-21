# Cross-product compatibility evidence

## Scope and baseline

This pass reconciles compatibility contracts that cross the Python engine,
browser frontend, Desktop host, and eventual Zed boundary. It is intentionally
separate from the product inventories: a backend writer and frontend reader can
each be correctly inventoried while still disagreeing at their shared boundary.

| Product | Baseline | Evidence |
| --- | --- | --- |
| ComfyUI | 0.27.1; fingerprint `21de8fece20d8d5bfa94daaa52d6ccfe2db6726ca0803ca3b383ad164cbd1d5f` | `baseline.md`; `projects/comfy/ComfyUI/pyproject.toml`; `comfyui_version.py` |
| ComfyUI Frontend | 1.48.2; fingerprint `aeb208b759effdacf2ea3b1929f0a3e583201f0b7b3cb006f36f1007364b8ca3` | `baseline.md`; `projects/comfy/ComfyUI-Frontend/package.json` |
| Comfy Desktop | 1.0.28; fingerprint `2442854931f3a5a80e68aa55eab21a26dcefe868b4e875251a5b4d811668e448` | `baseline.md`; `projects/comfy/Comfy-Desktop/package.json` |
| Zed | 1.10.2; fingerprint `99ceb40a1cc3359cde6e0865fe1b6138a06317d5fbd892f1595de10a96b07e9a` | `baseline.md`; `crates/zed/Cargo.toml` |

Git metadata is absent at the nested source roots, so this pass does not claim
commit SHAs. The package versions and deterministic source-tree fingerprints
from `baseline.md` are the source identities.

No dependencies were installed and no account, credential, paid service,
remote mutation, or source application write was used. The installed Python is
below ComfyUI's requirement and the JavaScript dependencies are absent. As a
result, runtime evidence produced by this cross pass is 0%; `test-backed` means
the source test was inspected, not rerun here.

## Artifacts and row policy

- `catalogs/cross-formats.csv` assigns `COMFY-FORMAT-041` through
  `COMFY-FORMAT-074`. `COMFY-FORMAT-001` through `040` were already assigned by
  the backend inventory, so this range avoids collisions.
- `catalogs/cross-compatibility.csv` assigns `COMFY-COMPAT-001` through
  `COMFY-COMPAT-060`.
- Every row maps to existing requirement criteria, design decisions, task IDs,
  and planned validation IDs. An empty `test_evidence` cell means that no
  focused source test was found; the row is consequently `code-inferred`, not
  test-backed.
- Cross rows do not replace backend/frontend/Desktop feature IDs. They bind
  those product-local rows into one independently testable compatibility
  contract.

## Counts

### Cross-format catalog

`catalogs/cross-formats.csv` has 34 rows.

| Dimension | Value | Rows |
| --- | --- | ---: |
| Domain | workflow | 9 |
| Domain | embedded metadata | 23 |
| Domain | outputs | 1 |
| Domain | models | 1 |
| Evidence | test-backed | 29 |
| Evidence | code-inferred | 5 |
| Availability | active | 27 |
| Availability | conditional | 6 |
| Availability | deprecated/dead | 1 |
| Zed status | missing | 31 |
| Zed status | conflicting | 3 |

The 34 rows comprise nine workflow/serialization/state transforms
(`041`-`049`), nineteen carrier or explicit-boundary formats (`050`-`068`),
and six cross-cutting import/output/path policies (`069`-`074`). All 34 involve
the frontend; 24 also have a backend producer or server-side contract.

### Cross-compatibility catalog

`catalogs/cross-compatibility.csv` has 60 rows.

| Domain | Rows |
| --- | ---: |
| WebSocket | 12 |
| REST | 8 |
| Modes | 7 |
| Python extensions | 5 |
| Web extensions | 4 |
| Desktop bridge | 3 |
| Desktop IPC | 2 |
| Identifiers | 2 |
| Execution inputs | 2 |
| Workflow import | 2 |
| Baseline, graph sockets, widgets, prompt, outputs, models, flags, security, migrations, deprecation, formats, architecture, and target status | 13 |
| Total | 60 |

| Dimension | Value | Rows |
| --- | --- | ---: |
| Evidence | test-backed | 33 |
| Evidence | code-inferred | 27 |
| Zed status | missing | 49 |
| Zed status | conflicting | 5 |
| Zed status | uncertain | 6 |
| Availability | active | 35 |
| Availability | conditional (exact label) | 13 |
| Availability | conditional plus cloud/paid | 2 |
| Availability | conditional plus deprecated/dead | 1 |
| Availability | cloud/paid | 2 |
| Availability | developer-only | 1 |
| Availability | infrastructure-only | 4 |
| Availability | platform-specific | 1 |
| Availability | deprecated/dead | 1 |

Product inclusion is non-exclusive: 51 compatibility rows involve the
frontend, 45 involve ComfyUI, 18 involve Desktop, and one explicitly records
the Zed baseline.

### Combined cross pass

- 94 stable cross-product contracts.
- 62 test-backed and 32 code-inferred; no documented-only or unverified rows.
- 80 missing, 8 conflicting, and 6 uncertain in current Zed evidence.
- 0 equivalent, 0 partial, and 0 deferred cross rows.
- 62/94 (66.0%) have focused source-test evidence.
- 0/94 (0.0%) were dynamically confirmed in this environment.
- All 94 IDs are unique and all rows contain requirement, design, task, and
  validation mappings.

These numbers describe the cross catalogs only. They must not be added to
product feature counts as if they were newly discovered UI/backend/Desktop
features; they are compatibility edges between those features.

## Workflow and prompt contracts

### Workflow JSON 0.4

`zComfyWorkflow` accepts integer or string node IDs, `last_node_id`,
`last_link_id`, node arrays, and six-item array links:

`[link id, origin node id, origin slot, target node id, target slot, type]`.

Optional groups, config, extra, models, floating links, and subgraph definitions
are passthrough objects. Vectors may arrive as tuples or objects with numeric
`0` and `1` properties. Slot indexes may be integers or integer strings and
are normalized to numbers. Node `widgets_values` may be an array or a record.

The Python engine does not execute the UI workflow. The frontend separately
generates an API prompt and places the UI workflow under
`extra_data.extra_pnginfo.workflow`, which output nodes may embed. This is why
Zed needs a lossless UI document and a distinct prompt transform.

### Workflow JSON schema 1

Version 1 replaces the 0.4 last-ID fields with a `state` object and array links
with passthrough link objects. It adds first-class reroutes and recursive
subgraph definitions. Graph and subgraph identity uses UUIDs plus revisions;
subgraph I/O slots also have stable UUIDs. Instances reference their subgraph
definition UUID as node type, while execution identity remains an instance
path.

Only the exact numeric value `1` selects this schema. Every other numeric
version is validated as 0.4. A missing or nonnumeric version rejects. This is a
compatibility fallback, not evidence that arbitrary future versions satisfy
0.4.

### Validation is normally fail-open

`Comfy.Validation.Workflows` defaults to false. When enabled, Zod schema
validation and link repair run, but `loadGraphData` uses the original graph if
validation fails. A Zed parser should preserve that recovery outcome while
using bounded, memory-safe parsing and retaining original bytes. A strict-only
importer would reject source-loadable artifacts; an unbounded fail-open parser
would create an avoidable security regression.

### API prompt

The API graph is an object keyed by node ID. Each node has `class_type`, an
`inputs` record, and `_meta.title` in frontend output. Connection inputs are
two-item arrays; literal arrays are wrapped in `{ "__value__": ... }`, and
curves additionally carry `"__type__": "CURVE"`.

`graphToPrompt` applies virtual nodes, serializes the UI workflow, strips
localized slot names, compresses widget input slots, records frontend version,
resolves subgraphs, omits virtual/muted/bypassed nodes, and removes inputs whose
upstream node was removed. Missing node classes make `loadApiJson` visibly
warn, but the source importer skips them and therefore cannot losslessly
round-trip that API prompt. Zed should retain the original prompt bytes even if
its editable reconstruction is partial.

### Node, socket, widget, and seed compatibility

- Runtime node IDs are branded strings. Canonical integer strings serialize
  back to JSON integers; noncanonical strings remain strings. Empty IDs reject.
- Nested execution IDs are colon-separated node paths. A local node ID cannot
  itself contain a colon. Locator IDs use a subgraph UUID plus local node ID and
  are distinct from instance-dependent execution IDs.
- Empty, `*`, and zero socket types are wildcards. Exact matches are
  case-insensitive; EVENT output connects to ACTION input; comma-separated
  unions match when any pair matches. Legacy array/numeric types remain
  preserved by the workflow schema even though LiteGraph's primary matcher is
  string-based.
- `widget.serialize` controls workflow persistence.
  `widget.options.serialize` independently controls API-prompt inclusion.
  `serializeValue` may be asynchronous.
- Seed/controlled values persist modes `fixed`, `increment`, `decrement`,
  `randomize`, and combo-only `increment-wrap`, while only the current value is
  sent to Python. Partial execution suppresses advancing the control. Numeric
  control clamps to declared bounds and ±1125899906842624.

## Embedded metadata and file-format reconciliation

| ID | Carrier | Backend write | Frontend read | Important bound/fallback |
| --- | --- | --- | --- | --- |
| `050` | PNG | SaveImage text keys | tEXt, comf, iTXt; zlib iTXt | bad signature `{}`; FileReader error rejects |
| `051` | APNG | after-IDAT latin-1 comf chunks | generic PNG comf parser | strict latin-1 can fail |
| `052` | WebP | animated WebP EXIF ASCII tags | optional `Exif\0\0`, title/lowercase keys | RIFF odd padding honored |
| `053` | AVIF | no writer found | AVIF Exif item/TIFF ASCII | invalid boxes/offsets resolve `{}` |
| `054` | SVG | metadata CDATA after `<svg>` | case-insensitive metadata/CDATA regex | hostile active content boundary |
| `055` | FLAC | PyAV container metadata | Vorbis comment block | reads whole file; malformed bounds need hardening |
| `056` | MP3 | PyAV container metadata | 4096-byte page scan and NUL regex | invalid signature logs but scan continues |
| `057` | Ogg/Opus | backend emits `.opus` | OggS/comments when MIME is `audio/ogg` | accept list omits `.opus`/`audio/opus` |
| `058` | WebM | SaveWEBM container tags | first 2 MiB EBML SimpleTag scan | later metadata is invisible |
| `059` | MP4 | metadata tags with `use_metadata_tags` | first 64 MiB keys/ilst | atoms after cap are invisible |
| `060` | MOV | same ISOBMFF writer family | same keys/ilst parser | extension fallback is case-sensitive |
| `061` | M4V | same ISOBMFF writer family | same keys/ilst parser | codec/container availability varies |
| `062` | GLB | `asset.extras` JSON strings for generated mesh | first 1 MiB, first JSON chunk | File3D pass-through does not inject metadata |
| `063` | `.latent` | safetensors `__metadata__` | first 4 MiB header | malformed/oversized header is undefined |
| `064` | `.safetensors` | model/merge writers may embed keys | same first-4-MiB header parser | model files are source-supported workflow carriers |
| `065` | JSON | direct artifact | tolerant text parser/discriminator | invalid/non-string result is undefined |
| `066` | A1111 parameters | legacy external producer | final fallback graph converter | partial/unknown options log |
| `067` | OpenEXR | SaveImageAdvanced string attributes | no frontend parser/accept entry | producer/consumer conflict |
| `068` | PLY | 3D asset only | viewer parser only | explicitly not a workflow carrier |

Two producer/consumer mismatches are especially actionable:

1. The backend emits `.opus`, but the frontend file accept list includes only
   `.ogg`/`audio/ogg`. The inspected `.opus` fixture works because its File MIME
   is `audio/ogg`; OS/browser MIME assignment can prevent entry to the parser.
2. SaveImageAdvanced embeds prompt/workflow in OpenEXR, while the frontend has
   no EXR metadata parser and does not advertise EXR as a workflow format.

Metadata suppression is also per writer, not global in practice.
`--disable-metadata` governs core PNG/APNG/WebP, audio, SaveVideo, GLB,
latent/model, PNG-advanced, and EXR-advanced paths. In this snapshot SaveWEBM
and SaveSVG do not consult it. File3D pass-through preserves existing container
metadata rather than injecting current prompt/workflow. A single blanket Zed
preference would not reproduce these contracts.

All inspected metadata parsers accept Python's bare `NaN`, `Infinity`, and
`-Infinity` through `parseJsonWithNonFinite`, coercing them to JSON null. This
must be a token-aware transform; global text replacement would corrupt strings.

Import priority is observable: process templates if present, prefer UI
workflow and return on success, then try API prompt, then A1111 parameters.
Import never queues execution. With no recognized metadata, an image/audio/video
creates its matching load node and uploads; an unsupported type shows a
file-load alert.

## REST reconciliation

The backend route catalog has 141 rows and the frontend HTTP/URL-usage catalog
has 149 rows. They are not intended to join one-to-one. The reconciliation
classes are:

1. Core `PromptServer.routes` handlers are retained unprefixed and duplicated
   under `/api`. Current frontend `fetchApi` adds `/api`; legacy automation may
   still use the unprefixed route.
2. Jobs are already declared at `/api/jobs` within the duplicated registry, so
   `/api/api/jobs` aliases are also materialized. Tests and frontend use
   `/api/jobs`.
3. Asset routes register directly under `/api/assets` and have no unprefixed
   alias.
4. `/internal/*` is an unprefixed subapplication. `internalURL` deliberately
   does not add `/api`.
5. `/extensions`, `/templates`, `/docs`, and the frontend root are static or
   direct handlers. `fileURL` leaves them unprefixed. Custom workflow template
   files are separately mounted at `/api/workflow_templates/{module}`.
6. Cloud/auth/billing/secrets/workspace/hub, manager-v2, asset transfer, and
   custom-node routes can be provider- or extension-owned. Their absence in
   local core is a capability state, not evidence that the frontend call is
   dead.

Every `fetchApi` request adds `Comfy-User`. Cloud builds may also wait up to ten
seconds for authentication, add provider credentials, and use a unified 401
remint/retry flow. Zed should keep identity, retry safety, and route capability
scoped to a profile.

## WebSocket reconciliation

The backend catalog has 26 rows. The frontend has 24 rows: 18 wire or
bidirectional events and six frontend-local events. Backend binary type 2 is an
internal unencoded image tuple that is converted to wire type 1; it is not a
client frame.

### Connection and feature hello

Frontend connects to unprefixed `/ws`, reusing `clientId` from `window.name`
when available and adding a cloud token when applicable. Server replaces an old
socket for a reused SID or creates a UUID-hex SID, then sends status and SID.
The first client text frame advertises client feature flags. Only a first-frame
`feature_flags` message negotiates; later occurrences are ignored for that
purpose. Server replies with its manifest.

Core server flags in this snapshot are:

- `supports_preview_metadata=true`
- `max_upload_size` in bytes
- `extension.manager.supports_v4=true`
- `node_replacements=true`
- `assets` from CLI

Registered CLI-provided flags include `show_signin_button` and
`enable_telemetry`; typed invalid values are warned and dropped, and CLI values
cannot overwrite core keys.

### Framing

- Type 1: big-endian event, image type, JPEG/PNG bytes.
- Type 3: bundled server emits node-ID length, node ID, and text. Frontend also
  contains a test-backed decoder for a proposed prompt-ID-prefixed form, but it
  enables that decoder only from a server flag. The bundled server neither
  advertises `supports_progress_text_metadata` nor emits that form. It remains
  an explicit uncertainty.
- Type 4: big-endian event, metadata JSON length, JSON, image bytes. Negotiated
  clients associate nested node/prompt identity, and the frontend additionally
  dispatches its legacy `b_preview` local event.

Normal `executing` events include display/prompt fields, but reconnect replay
sends only `node`. The frontend's declared Zod schema requires more fields,
while its runtime handler intentionally falls back to `display_node || node`.
A strict generated decoder would reject an evidence-backed source variant;
Zed's schema must model the union.

Backend emits nine `assets.seed.*` lifecycle events with no bundled typed
frontend consumer. Conversely, frontend declares `notification`,
`asset_download`, and `asset_export`, for which no local core producer exists.
Those are producer/consumer deltas, not silently discarded rows.

Unknown JSON event names are delivered only when a custom listener registered
that exact type; otherwise the frontend reports each type once. Listener sync
throws and rejected promises are isolated so later frames and other listeners
continue.

## Python and web extension compatibility

### Python V1

Legacy modules export `NODE_CLASS_MAPPINGS` and may export display-name and web
directory mappings. Node classes expose the established input/output, list,
lazy, validator, change detection, output-node, and expansion conventions.
Built-in identifiers are protected by the ignore set captured before external
loading. That ignore set is not extended after each custom module, so a later
custom module can overwrite an earlier custom identifier; the outcome depends
on traversal order.

### Python V3

`comfy_entrypoint` may be sync or async and must return a `ComfyExtension`.
`on_load` runs, `get_node_list` must return a list, and each `io.ComfyNode`
schema is finalized into the shared registries. Wrong types and exceptions warn
and skip the module without aborting unrelated modules.

### Discovery and routes

Prestartup scripts run before module import. `.disabled`, `__pycache__`,
disable-all/whitelist, and manager policy gates apply. Failed imports and import
times are logged. Custom aiohttp RouteDefs registered through the shared route
table inherit dual-prefix behavior; routes/static mounts added directly to the
application keep only their declared paths.

### Frontend web modules

`tool.comfy.web` in pyproject or legacy `WEB_DIRECTORY` registers a static
directory. `/extensions` enumerates JavaScript recursively. Frontend loads core
extensions first, then advertised third-party modules in parallel. One import
failure does not stop the rest.

Extension names are required and unique. Disabled names persist even while a
pack is absent, and four superseded legacy extensions are always disabled.
The hook interface is open-ended and currently includes commands, keybindings,
menus, settings, panels, badges, buttons, widgets, node definitions/classes,
context menus, graph lifecycle, authentication, and output callbacks.

Callback exceptions are isolated, but an async hook has no timeout and can
hold an aggregate invocation indefinitely. More importantly, source Python
extensions run in the engine process and frontend modules run with page-origin
DOM/network/LiteGraph authority. Native Zed should preserve that ambient power
only in an explicitly trusted compatibility mode; untrusted extensions need a
sandbox or a lossless missing-extension placeholder.

## Desktop bridge and mode compatibility

The Desktop catalogs contain 273 IPC channel rows and 299 preload members:

| Surface | Members |
| --- | ---: |
| `window.api` | 158 |
| `window.__comfyDesktop2` plus Terminal, Logs, Telemetry | 24 |
| `window.__comfyTitlePopup` | 76 |
| `window.__comfyTitleBar` | 33 |
| `window.__comfySystemModal` | 4 |
| `window.__comfyTitleTooltip` | 4 |
| Total | 299 |

The official hosted frontend directly consumes the 24-member
`__comfyDesktop2` family for remote detection, model/asset downloads,
pause/resume/cancel/progress, theme reporting, terminal, logs, and telemetry.
Desktop's panel/chooser renderer consumes `window.api`; title surfaces consume
their narrower bridges.

There is also a bridge-generation mismatch. A frontend compiled with
`distribution=desktop` loads a legacy `electronAPI` adapter. Current
Comfy-Desktop's `comfyPreload` exposes `__comfyDesktop2`, not `electronAPI`.
Hosted behavior therefore depends on selecting the matching frontend build and
capability path. This pass found no current Desktop preload producer for the
legacy `electronAPI` global.

Desktop source plugins define six stable modes:

| Source ID | Category | Compatibility boundary |
| --- | --- | --- |
| `standalone` | local | managed bundle/environment and local process |
| `portable` | local | archive/embedded Python layout and portable updater |
| `git` | local | source checkout and its venv |
| `desktop` | local | legacy Desktop layout, adoption, and migration |
| `remote` | remote | URL-only; no managed process |
| `cloud` | cloud | authenticated/entitled URL provider |

Unknown source fields remain in installation records, and an unknown source
fails actions visibly instead of being rewritten. Zed profiles should preserve
the same source identity and isolate workflows, cookies/auth, queue/history,
downloads, paths, and windows per selected instance.

## Migrations, deprecated behavior, and feature flags

The cross rows bind these migration families to preservation and validation:

- workflow 0.4 versus schema 1 dispatch;
- legacy reroute nodes to native reroutes, with warning and no mixed-network
  migration;
- proxy-widget quarantine/deferred migration;
- node-definition V1 to normalized V2;
- renderer layout-scale normalization;
- KSampler `sample_` prefix and boolean control migration;
- workflow draft V1 to V2, with V1 retention coded through 2026-07-15;
- legacy Desktop adoption/migration to standalone;
- deprecated audio nodes, A1111 import, legacy routes, extension names, and
  bridge generations.

Availability/deprecation must be first-class data. A feature that is disabled,
cloud-paid, platform-only, experimental, developer-only, or deprecated remains
addressable and traceable. Unsupported execution can be mapped only to an
approved native provider/plugin boundary or deferred,
but its serialized data must not disappear.

## Known mismatches and uncertainties

| Contract | State | Consequence |
| --- | --- | --- |
| `.opus` writer versus frontend accept list | conflicting | browser/OS MIME can prevent import |
| EXR metadata writer versus no frontend reader | conflicting | backend output cannot reopen as workflow in bundled frontend |
| SaveWEBM/SaveSVG versus `--disable-metadata` | conflicting | global preference is not consistently honored |
| `executing` reconnect payload versus declared schema | conflicting | strict decoder would reject source-valid replay |
| Desktop `__comfyDesktop2` versus legacy `electronAPI` | conflicting | frontend build and host bridge must match |
| New progress-text framing | uncertain | decoder/test exists, producing backend/flag does not |
| Cloud/paid route and event providers | uncertain | client contracts exist without local server evidence |
| Custom/custom Python ID collision order | uncertain | traversal-dependent and not dynamically observed |
| GLB drop-to-load | uncertain runtime | parser test exists; browser case is commented out |
| Dynamic custom routes/events/hooks | open world | absent extensions cannot be enumerated |

## Recommended architecture and first slice

The compatibility evidence now maps to the native boundary in `design.md` D1:

1. Implement prompt validation, execution, tensors, autograd, RNG, models,
   devices, memory, samplers, schedulers, nodes, media, cache, cancellation, and
   recovery in Zed-owned Rust crates and workers.
2. Keep lossless workflow, protocol, identifier, metadata, migration, and
   attempt types separate from GPUI entities and from private worker handles.
3. Build native GPUI graph/workspace surfaces from compiled built-in descriptors
   and versioned Rust/WASM manifests. Source object-info is a conformance output,
   not a production authority.
4. Replace Python and JavaScript extension execution with explicit Rust/WASM
   ports, deterministic legacy mappings, capability grants, resource limits,
   and lossless unresolved placeholders. No browser handoff executes hooks.
5. Expose REST/WebSocket and CLI compatibility as native Rust projections of
   the same runtime. Keep cloud/auth/paid behavior behind approved providers and
   fail closed when absent.

The first end-to-end slice is `LoadImage -> ImageScale -> ImageInvert ->
PreviewImage -> SaveImage` on native CPU tensors in the Rust worker. It covers
progress/output events, transactional metadata, cache hit/invalidation,
deterministic cancellation, worker termination/recovery, GPUI inspection, and
no-network/no-Python/no-source-tree release gates. The second slice uses a
shape-reduced checkpoint through CLIP, latent construction, KSampler, VAE, and
SaveImage with intermediate conformance checkpoints.

## Traceability recommendations

The prior hand-written hybrid mapping is superseded. The normative per-row
native mapping is generated in `catalogs/features.csv`, `parity-matrix.md`, and
`traceability.md`. Cross-format rows map to native workflow/media adapters;
protocol rows map to the native compatibility host; extension rows map to
Rust/WASM or placeholders; Desktop mode rows map to native profiles, artifacts,
workers, providers, and migrations; every active node/model/sampler/device row
maps to an exact native implementation task and validation.

## Coverage gate result for this pass

- Format IDs are unique and contiguous from 041 through 074.
- Compatibility IDs are unique and contiguous from 001 through 060.
- All 94 rows cite executable source and carry evidence, availability,
  confidence, Zed status/gap, requirement, design, task, validation, and open
  question fields.
- Backend 141-route and 26-event totals, frontend 149-HTTP-use and 24-event
  totals, Desktop 273-channel and 299-preload-member totals, and six Desktop
  source IDs are reconciled above.
- Active uncertainty remains explicit for dynamic extensions, cloud providers,
  the proposed progress-text frame, traversal-dependent conflicts, and
  unavailable runtime/platform environments.

The master generator merges these rows into the global parity matrix and
traceability ledger without duplicating them as product-local feature counts;
regeneration and strict validation enforce that mapping.
