# Source inventory

## Inventory boundary

The normative feature ledger is [`catalogs/features.csv`](catalogs/features.csv). It contains **13,295** stable, non-reused feature contracts derived from ComfyUI, ComfyUI-Frontend, Comfy-Desktop, comfy-cli, docs, embedded-docs, Zed target evidence, and cross-product compatibility surfaces. Separate registry, localization, telemetry, test, generated-documentation, and source-file ledgers remain authoritative for count reconciliation even where an individual row is metadata or coverage support rather than a distinct user workflow.

No source application, account, paid service, dependency set, or remote state was modified. Runtime evidence includes the safe ComfyUI parser/feature-flag probes, docs link and Bun tests, and embedded-docs link check recorded in [baseline.md](baseline.md). The comfy-cli runtime probe failed before command construction because Python 3.9.6 is below the declared 3.10 minimum and `questionary` is unavailable. Existing tests support `test-backed` classifications only where focused; a test-backed row is not represented as locally passing unless the baseline records a successful run.

## Feature counts by product

| Product | Features |
| --- | ---: |
| ComfyUI | 4,173 |
| ComfyUI-Frontend | 3,628 |
| Comfy documentation | 1,599 |
| Comfy CLI | 1,348 |
| Comfy-Desktop | 1,268 |
| Comfy embedded documentation | 855 |
| ComfyUI-Frontend website | 282 |
| Cross-product | 94 |
| ComfyUI-Frontend desktop-ui | 48 |
| **Total** | **13,295** |

## Feature counts by domain

| Domain | Features |
| --- | ---: |
| schema | 1,033 |
| built-in-nodes | 896 |
| node documentation | 855 |
| node | 801 |
| graph | 750 |
| tensor operation | 600 |
| conditioning-model-patch contract | 583 |
| CQL policy | 419 |
| asset | 390 |
| parameter | 370 |
| workflow | 330 |
| desktop-preload | 299 |
| cloud | 293 |
| ui | 287 |
| desktop-ipc | 273 |
| setting | 262 |
| external-service | 217 |
| model-device-format | 211 |
| configuration | 208 |
| website-cloud | 176 |
| menu | 173 |
| application-ui | 161 |
| http | 141 |
| desktop-telemetry | 139 |
| tutorials | 139 |
| extension | 131 |
| graph-editor | 130 |
| command | 123 |
| queue | 119 |
| tooling | 108 |
| module | 104 |
| asset-viewer-editor | 103 |
| error | 99 |
| graph-editing-widgets | 93 |
| workflow-template | 89 |
| assets-specialized-media | 75 |
| desktop-configuration | 74 |
| format | 74 |
| workflow-experience | 68 |
| model-support-data | 65 |
| redirect | 65 |
| cloud-account-workspace | 64 |
| exec | 60 |
| application-ui-state | 58 |
| extension documentation | 56 |
| snippets | 56 |
| random number generation | 54 |
| partner API | 52 |
| frontend-extension-manager | 49 |
| desktop-menu | 45 |
| settings-persistence-cloud-security-and-ui | 45 |
| desktop-window | 44 |
| desktop-renderer-surface | 43 |
| cloud OpenAPI | 42 |
| updates-snapshots-and-downloads | 40 |
| autograd | 36 |
| custom-nodes | 36 |
| desktop-persistence | 36 |
| telemetry | 36 |
| environment | 35 |
| source-and-installation | 35 |
| desktop-setting | 31 |
| registry | 31 |
| settings | 30 |
| ext | 28 |
| a11y | 26 |
| desktop-shell | 26 |
| websocket | 26 |
| workflow-lifecycle-sharing | 26 |
| cloud-billing-workspace | 25 |
| launch-process-and-lifecycle | 25 |
| lifecycle | 24 |
| persistence | 24 |
| embedded metadata | 23 |
| desktop-theme-style | 21 |
| development | 21 |
| filesystem-layout-configuration | 20 |
| lifecycle documentation | 20 |
| desktop-input | 19 |
| interface | 19 |
| queue-execution-ui | 19 |
| sec | 19 |
| workflow-template-support | 17 |
| documented claim | 16 |
| installation | 16 |
| terminal-logs-crash-and-diagnostics | 16 |
| onboarding-and-migration | 15 |
| platform-packaging-and-recovery | 15 |
| window-host-and-navigation | 15 |
| .github | 14 |
| queue-execution-state | 14 |
| desktop-native-ui | 12 |
| event | 12 |
| support | 12 |
| WebSocket | 12 |
| desktop-diagnostics | 10 |
| diagnostics-errors | 10 |
| website-interaction | 9 |
| desktop-installation | 8 |
| REST | 8 |
| authentication-secrets | 7 |
| http-client | 7 |
| modes | 7 |
| settings-keybindings | 7 |
| desktop-installation-source | 6 |
| manager | 6 |
| agent-tools | 5 |
| Python extensions | 5 |
| specs | 4 |
| web extensions | 4 |
| account | 3 |
| comfy-cli | 3 |
| Desktop bridge | 3 |
| desktop-feature-flag | 3 |
| desktop-platform | 3 |
| troubleshooting | 3 |
| cloud-account-state | 2 |
| community | 2 |
| Desktop IPC | 2 |
| execution inputs | 2 |
| get_started | 2 |
| identifiers | 2 |
| models | 2 |
| outputs | 2 |
| package-test-configuration | 2 |
| workflow import | 2 |
| api-reference | 1 |
| architecture | 1 |
| baseline | 1 |
| changelog | 1 |
| database-migration-support | 1 |
| deprecation | 1 |
| desktop-update | 1 |
| feature flags | 1 |
| formats | 1 |
| graph sockets | 1 |
| index.mdx | 1 |
| migrations | 1 |
| prompt protocol | 1 |
| route | 1 |
| security | 1 |
| target status | 1 |
| widgets | 1 |
| **Total** | **13,295** |

## Feature counts by source classification

| Classification | Features |
| --- | ---: |
| test-backed behavior | 1,677 |
| executable Python typed schema | 952 |
| built-in node reference | 896 |
| localized embedded node documentation | 855 |
| functional Vue component surface | 691 |
| built-in node | 565 |
| clip_text_encoder_architecture | 398 |
| CLI option or argument | 370 |
| frontend functional module | 339 |
| node policy | 322 |
| English product page | 307 |
| individual desktop-preload contract | 299 |
| individual desktop-ipc contract | 273 |
| callable-operation; elementwise-or-runtime-operation | 261 |
| API node | 224 |
| hosted API-node outbound proxy endpoint | 217 |
| independently-testable-capability | 206 |
| setting | 152 |
| infrastructure-only | 146 |
| HTTP client contract | 142 |
| reachable CLI leaf | 123 |
| command | 118 |
| documentation tooling contract | 108 |
| analytics event contract | 105 |
| production module | 104 |
| CLI flag | 101 |
| stable CLI error | 99 |
| registered model architecture detector/loader | 94 |
| bundled workflow template | 89 |
| node pack | 87 |
| menu action | 86 |
| individual desktop-configuration contract | 74 |
| runtime HTTP route | 68 |
| persisted browser state | 66 |
| bundled model configuration, tokenizer, or search-path artifact | 65 |
| documentation redirect | 65 |
| namespace-or-value-reference; value-or-constant-contract | 64 |
| website/marketing route | 59 |
| OpenAPI component schema | 58 |
| legacy extension documentation contract | 56 |
| reusable English snippet | 56 |
| RNG phase contract | 54 |
| partner endpoint mapping | 52 |
| telemetry contract | 52 |
| coverage-anchor | 49 |
| runtime /api compatibility alias | 47 |
| individual desktop-menu contract | 45 |
| individual desktop-window contract | 44 |
| registered sampling algorithm | 44 |
| callable-operation; neural-network-module | 43 |
| feature flag / remote configuration | 43 |
| cloud API operation | 42 |
| context/menu action | 41 |
| functional | 41 |
| autograd construct | 36 |
| individual desktop-persistence contract | 36 |
| telemetry event | 36 |
| documentation configuration/format | 35 |
| CLI persisted or interchange format | 34 |
| keybinding | 34 |
| latent tensor scaling/channel contract | 33 |
| core frontend extension | 32 |
| callable-operation; external-tensor-kernel | 31 |
| clip_architecture | 31 |
| individual desktop-setting contract | 31 |
| vae_architecture | 31 |
| vae_selection_branch | 30 |
| callable-operation; shape-layout-transform | 29 |
| tracked-step telemetry contract | 29 |
| frontend extension API | 27 |
| menu navigation-action | 27 |
| individual desktop-shell contract | 26 |
| model search path category | 26 |
| callable-operation; reduction | 25 |
| CLI lifecycle state | 24 |
| WebSocket or frontend event contract | 24 |
| CLI JSON schema | 23 |
| application route | 22 |
| JSON WebSocket contract | 22 |
| individual desktop-theme-style contract | 21 |
| OpenAPI-only operation | 21 |
| bundled filesystem layout or configuration artifact | 20 |
| documented lifecycle contract | 20 |
| input | 20 |
| callable-operation; comfy-operator-indirection | 19 |
| individual desktop-input contract | 19 |
| bundled template support asset | 17 |
| CLI extension contract | 17 |
| menu destructive-action | 17 |
| CLI documentation claim | 16 |
| persisted format / migration contract | 16 |
| weight_adapter_runtime | 16 |
| type-reference; type-contract | 15 |
| callable-operation; linear-algebra | 14 |
| environment variable | 14 |
| patch_mapping | 14 |
| callable-operation; indexing-masking | 13 |
| callable-operation; random-number-generation | 13 |
| numeric/weight format | 13 |
| active | 12 |
| callable-operation; neural-network-functional | 12 |
| callable-operation; spatial-functional-kernel | 12 |
| CLI event contract | 12 |
| CMS staging localization | 12 |
| schema-bearing but unregistered node | 12 |
| vae_tiling | 12 |
| callable-operation; storage-dtype-device | 11 |
| menu radio-action | 11 |
| menu submenu | 11 |
| callable-operation; activation-normalization-functional | 10 |
| callable-operation; tensor-creation | 10 |
| label | 10 |
| media carrier | 10 |
| menu toggle-action | 10 |
| guidance_hook | 9 |
| hardware backend | 9 |
| model_execution | 9 |
| registered sigma scheduler | 9 |
| validation | 9 |
| controlnet | 8 |
| migration | 8 |
| cache | 7 |
| HTTP/static-resource client contract | 7 |
| individual desktop-installation-source contract | 6 |
| node extension API | 6 |
| patch_family_equation | 6 |
| secret input | 6 |
| child output | 5 |
| conditional | 5 |
| conditioning_value | 5 |
| database table | 5 |
| memory mode | 5 |
| menu checkbox-action | 5 |
| patch_payload | 5 |
| reclassified-external-operation; elementwise-or-runtime-operation | 5 |
| runtime static route | 5 |
| server feature flag | 5 |
| attention backend | 4 |
| callable-operation; accelerated-attention-kernel | 4 |
| callable-operation; spectral-transform | 4 |
| configuration file | 4 |
| coverage reconciliation | 4 |
| database migration | 4 |
| producer-consumer mismatch | 4 |
| tensor contract | 4 |
| audio file | 3 |
| binary WebSocket contract | 3 |
| deprecated/dead | 3 |
| frontend extension bridge | 3 |
| graph execution | 3 |
| guidance | 3 |
| image file | 3 |
| individual desktop-feature-flag contract | 3 |
| individual desktop-platform contract | 3 |
| list semantics | 3 |
| menu disabled-item | 3 |
| menu dynamic-action | 3 |
| namespace-or-value-reference; namespace-contract | 3 |
| origin boundary | 3 |
| patch_semantics | 3 |
| queue/cancellation | 3 |
| API-node runtime | 2 |
| CLI-settable feature flag | 2 |
| CMS staging English | 2 |
| cross-product identifier contract | 2 |
| deployment mode contract | 2 |
| extension ABI | 2 |
| extension policy | 2 |
| filesystem boundary | 2 |
| frontend extension ABI | 2 |
| history | 2 |
| jobs API | 2 |
| memory control | 2 |
| model file | 2 |
| node schema JSON | 2 |
| package or test configuration coverage anchor | 2 |
| persisted workflow schema | 2 |
| preload ABI | 2 |
| presentational/infrastructure-only | 2 |
| preview/protocol | 2 |
| protocol alias contract | 2 |
| queue | 2 |
| queue/protocol | 2 |
| runtime | 2 |
| runtime extension | 2 |
| runtime/extension | 2 |
| telemetry volume-guard infrastructure | 2 |
| tensor container | 2 |
| trusted code extension | 2 |
| video file | 2 |
| weight_adapter_registry | 2 |
| workflow extension | 2 |
| 3D contract | 1 |
| API description | 1 |
| API-node extension | 1 |
| API-node policy | 1 |
| assets/concurrency | 1 |
| assets/persistence | 1 |
| authentication | 1 |
| authorization | 1 |
| authorization/assets | 1 |
| authorization/filesystem | 1 |
| availability contract | 1 |
| backend write-only carrier | 1 |
| backward compatibility contract | 1 |
| binary wire contract | 1 |
| cache/extension | 1 |
| cache/graph | 1 |
| cache/protocol | 1 |
| cancellation | 1 |
| capability schema | 1 |
| child input/output | 1 |
| cloud authentication contract | 1 |
| compatibility | 1 |
| compatibility extension | 1 |
| compile-time mode contract | 1 |
| conditioning contract | 1 |
| consumer-provider uncertainty | 1 |
| content boundary | 1 |
| content/extension boundary | 1 |
| cross-process protocol catalog | 1 |
| cross-product filesystem contract | 1 |
| cross-product graph contract | 1 |
| cross-product output schema | 1 |
| cross-product prompt contract | 1 |
| cross-product serialization contract | 1 |
| cross-product state contract | 1 |
| cross-product version contract | 1 |
| data carrier | 1 |
| database configuration | 1 |
| database migration support template | 1 |
| deployment and entitlement contract | 1 |
| developer boundary | 1 |
| dynamic protocol contract | 1 |
| dynamic wire contract | 1 |
| error surface | 1 |
| execution graph JSON | 1 |
| execution graph schema | 1 |
| explicit non-carrier | 1 |
| extension distribution contract | 1 |
| extension hook | 1 |
| extension lifecycle contract | 1 |
| extension manifest | 1 |
| extension protocol contract | 1 |
| extension recovery contract | 1 |
| extension security | 1 |
| file locator JSON | 1 |
| filesystem/recovery | 1 |
| flow control | 1 |
| history JSON | 1 |
| history/telemetry | 1 |
| import discriminator | 1 |
| import recovery contract | 1 |
| import state-transition contract | 1 |
| input/mutated/restored | 1 |
| input/output | 1 |
| internal event WebSocket contract | 1 |
| job JSON | 1 |
| latent file | 1 |
| legacy JSON compatibility | 1 |
| legacy text migration | 1 |
| localization extension | 1 |
| lossy compatibility contract | 1 |
| media carrier variant | 1 |
| media carrier with accept-list conflict | 1 |
| media contract | 1 |
| media import carrier | 1 |
| menu developer-action | 1 |
| menu dynamic-submenu | 1 |
| menu infrastructure-submenu | 1 |
| migration and recovery contract | 1 |
| migration corpus | 1 |
| mode registry contract | 1 |
| model file security | 1 |
| model/custom-node discovery | 1 |
| multi-user protocol contract | 1 |
| native IPC event contract | 1 |
| negotiated binary wire contract | 1 |
| network boundary | 1 |
| node schema compatibility | 1 |
| output policy | 1 |
| path compatibility contract | 1 |
| persisted settings file | 1 |
| persisted users file | 1 |
| persistence/recovery | 1 |
| platform-specific deployment contract | 1 |
| preview | 1 |
| protocol | 1 |
| protocol namespace contract | 1 |
| protocol/output schema | 1 |
| Python extension API | 1 |
| queue tuple | 1 |
| queue/concurrency | 1 |
| queue/security | 1 |
| reclassified-external-operation; indexing-masking | 1 |
| reclassified-external-operation; linear-algebra | 1 |
| recovery contract | 1 |
| registry conflict contract | 1 |
| required native parity decision | 1 |
| resource boundary | 1 |
| REST request contract | 1 |
| route | 1 |
| secret handling | 1 |
| secret-like input | 1 |
| security compatibility fact | 1 |
| serialization transform | 1 |
| server extension API | 1 |
| server recovery | 1 |
| session boundary | 1 |
| settings file | 1 |
| static compatibility contract | 1 |
| target evidence contract | 1 |
| telemetry identity infrastructure | 1 |
| telemetry infrastructure prefix; not emitted as an event | 1 |
| transport security | 1 |
| users file | 1 |
| vae_state_dict_conversion | 1 |
| validation compatibility contract | 1 |
| validation/extension | 1 |
| vector file | 1 |
| widget execution contract | 1 |
| widget persistence contract | 1 |
| widget state-transition format | 1 |
| wire connection contract | 1 |
| wire event contract | 1 |
| wire negotiation contract | 1 |
| **Total** | **13,295** |

## Feature counts by evidence level

| Evidence level | Features |
| --- | ---: |
| code-inferred | 7,042 |
| test-backed | 3,215 |
| documented-only | 2,352 |
| source-fingerprinted | 583 |
| observed | 103 |
| **Total** | **13,295** |

Direct runtime validation covers **103/13,242 (0.78%)** independently testable master rows. This percentage counts only `observed` rows, not inspected tests, and excludes explicit coverage anchors/reconciliation rows.

## Feature counts by availability

| Availability | Features |
| --- | ---: |
| active | 9,284 |
| cloud/paid | 1,627 |
| conditional | 773 |
| pinned source contract | 583 |
| infrastructure-only | 309 |
| experimental | 166 |
| developer-only | 164 |
| platform-specific | 142 |
| deprecated/dead | 114 |
| uncertain | 82 |
| cloud/paid; experimental | 48 |
| conditional;cloud/paid | 2 |
| conditional;deprecated/dead | 1 |
| **Total** | **13,295** |

## Current Zed status

| Status | Features |
| --- | ---: |
| missing | 9,904 |
| deferred | 2,274 |
| conflicting | 788 |
| partial | 189 |
| uncertain | 87 |
| equivalent | 53 |
| **Total** | **13,295** |

Generic workspace, GPUI, settings, persistence, subprocess, action, focus, Wasmtime, wgpu/Metal, media, and visual-test primitives alone are design inputs, not Comfy behavior. Native Comfy foundations now have task-level implementation and validation evidence, but master feature rows remain `missing`, `conflicting`, `deferred`, or narrowly `partial` until their exact per-feature behavior and final closure artifacts pass; planned code is never promoted. Python/JavaScript extension execution and Python/server lifecycle rows are `conflicting` with the production-native boundary and map to Rust/WASM or native lifecycle migrations. The accessibility bootstrap, native graph semantics, and implemented graph keybinding rows are `partial`: production now enables GPUI accessibility without an environment gate. Exact later-owned accessibility rows retain their prior missing or conflicting status until their surface tasks and whole-application audits pass. Cross-product disagreements remain `conflicting`. `deferred` rows are still source-traced and preserve compatibility or an explicit service/product decision.

## Registry-to-inventory reconciliation

| Registry or manifest | Discovered | Cataloged | Reconciliation |
| --- | ---: | ---: | --- |
| ComfyUI registered nodes | 789 | 789 | No unresolved registrations; inactive schema-bearing classes are counted separately. |
| ComfyUI inactive schema-bearing nodes | 12 | 12 | Not active runtime registry entries. |
| ComfyUI runtime-effective HTTP paths | 120 | 120 | Master route catalog has 141 rows after 47 compatibility aliases and 21 OpenAPI-only operations are represented. |
| ComfyUI HTTP catalog rows | 141 | 141 | Runtime, alias, static, and OpenAPI-only contracts use distinct rows. |
| ComfyUI HTTP rows with route-specific request/response detail | 141 | 141 | Each row retains a static handler/OpenAPI excerpt and states runtime-only uncertainty explicitly. |
| ComfyUI WebSocket event contracts | 26 | 26 | JSON and binary/client/server directions are represented. |
| ComfyUI config/CLI/environment rows | 153 | 153 | Observed help/flag probes are marked per row. |
| ComfyUI model/format/hardware rows | 211 | 211 | Conditional families remain visible. |
| ComfyUI tensor operation/reference contracts | 600 | 600 | Callable operations, type references, namespace/value references, and receiver-unverified candidates remain distinct; static calls do not imply semantic closure. |
| ComfyUI autograd contracts | 36 | 36 | Custom Functions, gradient modes/state, checkpointing, mixed precision, optimizers/scalers, and reverse-mode execution are explicit native obligations. |
| ComfyUI phase-scoped RNG contracts | 54 | 54 | Named phases retain seed/generator/device sites, retry/cancellation policy, and no-global-RNG native decisions. |
| ComfyUI Python files scanned for tensor runtime | 683 | 683 | Every scanned file maps to the canonical 949-row backend source ledger; one Python 3.10 match/case file uses syntax-only normalization. |
| ComfyUI persisted formats/migrations | 40 | 40 | Prompt, queue/history, database, users, media, model/tensor, and migration contracts are represented. |
| ComfyUI schema rows | 1,010 | 1,010 | Executable and OpenAPI component schemas are retained. |
| ComfyUI hosted external endpoints | 217 | 217 | No live provider was called. |
| Frontend commands | 118 | 118 | Literal and dynamically registered command IDs reconciled. |
| Frontend default keybindings | 34 | 34 | Each binding resolves to a command. |
| Frontend menus | 236 | 236 | Command-backed and local actions are distinct. |
| Frontend settings | 152 | 152 | 149 literal definitions plus three explicit schema-only uncertain entries. |
| Frontend routes | 82 | 82 | Main, Desktop UI, and website routes. |
| Frontend WebSocket/local events | 24 | 24 | Backend-received and local events. |
| Frontend HTTP client contracts | 149 | 149 | Literal plus reconciled dynamic calls. |
| Frontend feature/config flags | 43 | 43 | Client hello, server flags, and remote config. |
| Frontend telemetry rows | 88 | 88 | Events and literal button identifiers. |
| Frontend persisted formats/migrations | 24 | 24 | Formats and explicit migrations. |
| Frontend persisted state keys | 66 | 66 | Literal and dynamic patterns. |
| Frontend extension contracts | 59 | 59 | Interface members and core modules. |
| Frontend broad-anchor-only production/cloud/platform Vue files | 804 | 804 | Every audit-predicate match has one stable source-specific component contract. |
| Frontend explicitly required already-referenced Vue surfaces | 1 | 1 | AssetsSidebarTab is retained as a functional component contract even though another authoritative catalog already cited its source path. |
| Frontend functional Vue component surface contracts | 691 | 691 | Each row records concrete props/models/emits/events/handlers, visible states, failure, accessibility, persistence, interfaces, and validation. |
| Frontend presentational Vue infrastructure dispositions | 114 | 114 | Each row has a source-specific render-only reason and remains traceable through its consuming surface. |
| Frontend functional-module predicate candidates | 352 | 352 | Every normalized broad-anchor-only service/store/composable candidate has one stable source-specific contract. |
| Frontend functional module capabilities | 339 | 339 | Each row records exports, transitions, async lifetime, errors, persistence, side effects, source digest, and validation. |
| Frontend functional module infrastructure dispositions | 13 | 13 | Each pure plumbing/re-export/helper row has a source-specific non-capability reason and consuming anchor. |
| Cross-product persisted formats and media carriers | 34 | 34 | Workflow, prompt, metadata, model, output, legacy, and migration carriers are reconciled across producers/consumers. |
| Cross-product compatibility contracts | 60 | 60 | REST, WebSocket, IPC, extension, mode, identifier, state, and source-conflict contracts are represented. |
| Desktop IPC channels | 273 | 273 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop preload API members | 299 | 299 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop menu actions | 45 | 45 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop shell actions | 26 | 26 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop window/application/WebContents/updater events | 44 | 44 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop settings | 31 | 31 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop persisted formats/stores | 36 | 36 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop telemetry/event literals | 139 | 139 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop feature flags | 3 | 3 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop CLI/environment entries | 74 | 74 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop renderer CSS custom properties | 21 | 21 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop keybindings/gestures | 19 | 19 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop installation source modes | 6 | 6 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop platform matrix rows | 3 | 3 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop renderer surface contracts | 43 | 43 | Every discovered row has a stable master feature ID and retains its parent Desktop feature ID. |
| Desktop renderer rows with source-specific contracts and source-file mappings | 43 | 43 | Every formerly broad-anchor-only production Vue file has one stable functional or explicit presentational/infrastructure contract and the same ID in desktop-source-coverage.csv. |
| Desktop menu action rows with honest per-item evidence | 45 | 45 | No derived menu item claims test-backed evidence without a focused test for its exact action and condition. |
| Desktop discarded legacy settings | 4 | 4 | Load-time removal and rewrite behavior is retained instead of omitting obsolete keys. |
| Desktop telemetry/event rows with source, payload, consent, redaction, and rate detail | 139 | 139 | Every production comfy.desktop.* and app:* literal has a stable ID and explicit provider/infrastructure disposition. |
| Desktop IPC rows with request/event and response/callback detail | 273 | 273 | Exact static handler/call excerpts are retained; runtime structured-clone and error variants remain explicit contract-test work. |
| Desktop preload members with exact source signatures | 299 | 299 | Every bridge member retains the cited TypeScript declaration and unresolved runtime boundaries. |
| comfy-cli reachable command leaves | 123 | 123 | Hidden aliases and the shadowed/dead command collision remain explicitly classified. |
| comfy-cli option/argument bindings | 370 | 370 | Source declarations, alias repetitions, command scope, resolved types, nullability, value arity/cardinality, repeatability, enum choices, explicit parser constraints, paired boolean forms, defaults, envvars, hidden state, extraction evidence, and globals are retained. |
| comfy-cli JSON schemas | 23 | 23 | Envelope/stream mappings and the orphan comfy version mapping are reconciled separately. |
| comfy-cli stable error codes | 99 | 99 | Every error code retains meaning, hint, evidence, and native target decision. |
| comfy-cli event union | 12 | 12 | The converted, prompt_preview, settled, and state schema/emitter mismatches remain explicit. |
| comfy-cli production environment variables | 35 | 35 | Test/CI-only variables remain in source coverage and are not promoted to production controls. |
| comfy-cli configuration keys | 20 | 20 | Every key has a native mapping or architecture-conflicting migration decision. |
| comfy-cli persisted/interchange formats | 34 | 34 | The comfy-lock.yaml prose versus comfy.lock.yaml executable spelling conflict is retained. |
| comfy-cli lifecycle contracts | 24 | 24 | Python child lifecycle rows remain architecture conflicts; observable stages map to native operations. |
| comfy-cli extension contracts | 17 | 17 | Python/frontend override execution is prohibited; legacy identities map to Rust/WASM ports/placeholders. |
| comfy-cli CQL policy rows | 419 | 419 | Pack labels, node policies, versions, Git refs, and cloud-disabling labels reconcile. |
| comfy-cli partner allowlist endpoints | 52 | 52 | 52 aliases reconcile to the allowlist; excluded/proxy OpenAPI totals remain in the source ledger. |
| comfy-cli capability records | 1,244 | 1,244 | All behavioral capability rows are promoted to the master feature ledger; tests/source support remain separate closure ledgers. |
| comfy-cli module/service contracts | 104 | 104 | Every production module row is promoted to a master module/service contract so source-file closure requires a master feature ID. |
| docs authoritative/content records | 1,273 | 1,273 | English source, snippets, staging, roles, corroboration, and documented-only treatment are retained. |
| docs built-in node page reconciliation | 896 | 896 | Registry and embedded-doc exact/case/normalized/unverified deltas remain explicit. |
| embedded-docs node records | 855 | 855 | All locales, assets, AI-generated markers, fingerprints, sync, and registry matches reconcile. |
| docs Cloud OpenAPI operations | 42 | 42 | Route-shape corroboration does not promote documented cloud semantics to executable evidence. |
| docs redirects | 65 | 65 | Path and case behavior is retained as documentation-site configuration. |
| docs tooling contracts | 108 | 108 | Package scripts, CI, checks, and developer tools remain code-inferred developer/infrastructure behavior. |
| docs configuration/format records | 35 | 35 | Schemas, locks, navigation, and registries remain evidence records rather than product behavior unless corroborated. |
| docs extension contracts | 56 | 56 | All legacy behaviors map to explicit Rust/WASM ports or prohibited execution. |
| docs lifecycle contracts | 20 | 20 | Embedded version skew and documentation-only lifecycle claims remain visible. |

The frontend localization ledger contains 12,586 rows; the Desktop localization ledger contains 1,176 matched scalar paths. Localization rows are count-reconciled data contracts rather than one feature per translated scalar. The master ledger maps their consuming settings, commands, menus, routes, notifications, errors, and surfaces.

## Source-file coverage

| Product | Files | Explicit classifications |
| --- | ---: | --- |
| ComfyUI | 949 | documented-only=14; generated=3; infrastructure-only=358; production configuration/data=45; production data placeholder=37; production data/template=106; production source=288; test-only=98 |
| ComfyUI-Frontend | 4,697 | cloud/paid=328; documented-only=150; generated=2; infrastructure-only=416; platform-specific=54; production=1800; test-only=1947 |
| Comfy-Desktop | 735 | asset=23; generated-or-declaration=5; infrastructure-only=173; production=292; test-only=242 |
| comfy-cli | 312 | asset=2; documentation=7; infrastructure-only=25; production=137; test-only/support=141 |
| docs | 5,800 | CI workflow=9; CMS staging content=14; English built-in-node documentation=896; English product documentation=307; English reusable snippet=56; configuration/schema/lock/registry=20; executable automation/tooling=45; governance/tool documentation=16; localized generated content=3723; media asset=708; repository/site infrastructure=6 |
| embedded-docs | 10,298 | CI workflow=4; English node documentation=855; executable/package tooling=4; governance/tool documentation=1; localized node documentation=9405; node ancillary asset=1; node media asset=23; package configuration=2; repository/package infrastructure=3 |

Every source file in all six source repositories has a ledger row and either one or more feature/record mappings or an explicit production, infrastructure, generated, translated mirror, test-only/support, asset, documentation, staging, deprecated/dead, or placeholder classification with a reason. Infrastructure, translations, and test-support files are not promoted into fictional executable behavior. Zed target evidence is separately mapped in `catalogs/zed-architecture.csv` and [evidence-zed.md](evidence-zed.md).

## Tests, fixtures, stories, and snapshots

| Test ledger | Rows | Runtime rerun in this audit |
| --- | ---: | --- |
| ComfyUI test functions | 970 | No; dependency/runtime constraints are recorded in baseline.md. |
| Frontend Playwright declared cases | 1,677 | No; dependency/runtime constraints are recorded in baseline.md. |
| Comfy-Desktop test files | 232 | No; dependency/runtime constraints are recorded in baseline.md. |
| Comfy-Desktop declared suites/cases | 3,422 | No; dependency/runtime constraints are recorded in baseline.md. |
| comfy-cli test functions | 2,295 | No; dependency/runtime constraints are recorded in baseline.md. |
| docs executable Bun tests | 8 | Yes; 8/8 passed. |
| embedded-docs local link-check suite | 1 | Yes; the link checker passed. |

Frontend source reconciliation additionally records 1,013 unit/component test files and 77 Storybook files. Desktop reconciliation records 3,422 suite-or-case declarations across its 232 test files. comfy-cli records 2,295 test functions, 316 classes, and 129 fixtures but none ran locally. The docs audit ran 8/8 Bun tests and checked 4,988 documentation files with a passing validator; embedded-docs passed its local link checker. These totals characterize evidence reach and are not generalized beyond the recorded runs.

## Orphan and uncertainty reconciliation

| Orphan search | Result |
| --- | --- |
| Frontend default binding commands without english localization | 0 retained: none |
| Frontend literal command ids without english localization | 0 retained: none |
| Frontend literal setting definitions without schema | 0 retained: none |
| Frontend localized commands without literal definition | 0 retained: none |
| Frontend localized settings without schema | 0 retained: none |
| Frontend schema settings without english localization | 40 retained: Comfy.AppBuilder.VueNodeSwitchDismissed; Comfy.Assets.UseAssetAPI; Comfy.ColorPalette; Comfy.CustomColorPalettes; Comfy.Desktop.CloudNotificationShown; Comfy.Extension.Disabled; Comfy.InstalledVersion; Comfy.Keybinding.CurrentPreset; Comfy.Keybinding.NewBindings; Comfy.Keybinding.UnsetBindings; Comfy.Memory.AllowManualUnload; Comfy.Minimap.NodeColors; Comfy.Minimap.RenderBypassState; Comfy.Minimap.RenderErrorState; Comfy.Minimap.ShowGroups; Comfy.Minimap.ShowLinks; Comfy.Minimap.Visible; Comfy.NodeLibrary.Bookmarks; Comfy.NodeLibrary.Bookmarks.V2; Comfy.NodeLibrary.BookmarksCustomization; Comfy.Queue.History.Expanded; Comfy.Queue.ShowRunProgressBar; Comfy.Release.Status; Comfy.Release.Timestamp; Comfy.Release.Version; Comfy.RerouteBeta; Comfy.RightSidePanel.IsOpen; Comfy.Server.LaunchArgs; Comfy.Server.ServerConfigValues; Comfy.Templates.SelectedModels; Comfy.Templates.SelectedRunsOn; Comfy.Templates.SelectedUseCases; Comfy.Templates.SortBy; Comfy.Toast.DisableReconnectingToast; Comfy.TutorialCompleted; Comfy.VersionCompatibility.DisableWarnings; Comfy.WorkflowActions.SeenItems; LiteGraph.Canvas.LowQualityRenderingZoomThreshold; LiteGraph.Pointer.TrackpadGestures; VHS.AdvancedPreviews |
| Frontend schema settings without literal definition | 3 retained: Comfy.RerouteBeta; LiteGraph.Pointer.TrackpadGestures; VHS.AdvancedPreviews |
| comfy-cli comfy models (legacy hidden function) | Shadowed by the visible `models` Typer group; retain as deprecated/dead source evidence. |
| comfy-cli comfy query | Only HELP_EXAMPLES/run-cli text mention it; no command registration exists. |
| comfy-cli comfy version | Advertised by COMMAND_SCHEMAS but no command registration exists; global --version is the executable surface. |
| docs English navigation exact missing | 12 path-case/content references retained in docs-reconciliation.json. |
| docs translation validation | 51 reported truncation/translation issues retained; generated reports were removed and the source fingerprint restored. |

The 40 schema settings without English labels and the three schema settings without literal definitions are retained as hidden, compatibility, extension, or uncertain state rather than omitted. Backend dynamic custom nodes and API extensions not present in the snapshot remain open-world contracts. comfy-cli retains its shadowed `models` function, orphan `comfy version` schema mapping, prose-only `comfy query`, event-union drift, filename spelling conflict, and documentation-only Keyframe Relay claim. Docs retains navigation/path-case/localization deltas, three uncorroborated Cloud OpenAPI operations, provider-unverified node pages, and embedded-docs 0.5.7 versus ComfyUI pin 0.5.6. Cloud behavior, platform-native branches, hardware inference, installed plugins, and paid provider outcomes remain explicit runtime uncertainties.

## Master-catalog provenance

| Source catalog | Features |
| --- | ---: |
| frontend-features.csv | 2,560 |
| docs-pages.csv | 1,273 |
| backend-schemas.csv | 1,010 |
| embedded-docs-nodes.csv | 855 |
| frontend-component-surfaces.csv | 805 |
| backend-nodes.csv | 789 |
| backend-tensor-operations.csv | 600 |
| backend-conditioning-contracts.csv | 583 |
| comfy-cli-cql-policy.csv | 419 |
| comfy-cli-parameters.csv | 370 |
| frontend-functional-modules.csv | 352 |
| desktop-preload-apis.csv | 299 |
| desktop-ipc.csv | 273 |
| backend-external-services.csv | 217 |
| backend-models.csv | 211 |
| desktop-features.csv | 206 |
| backend-source-coverage.csv | 194 |
| frontend-menus.csv | 173 |
| backend-config.csv | 153 |
| backend-http-routes.csv | 141 |
| desktop-telemetry.csv | 139 |
| comfy-cli-commands.csv | 123 |
| docs-tooling.csv | 108 |
| backend-features.csv | 107 |
| comfy-cli-modules.csv | 104 |
| comfy-cli-errors.csv | 99 |
| desktop-cli-environment.csv | 95 |
| docs-redirects.csv | 65 |
| cross-compatibility.csv | 60 |
| docs-extension-contracts.csv | 56 |
| backend-rng.csv | 54 |
| comfy-cli-partner-openapi.csv | 52 |
| desktop-menu-actions.csv | 45 |
| desktop-window-events.csv | 44 |
| desktop-renderer-surfaces.csv | 43 |
| docs-openapi-cloud.csv | 42 |
| backend-formats.csv | 40 |
| backend-autograd.csv | 36 |
| desktop-persistence.csv | 36 |
| frontend-telemetry.csv | 36 |
| comfy-cli-environment.csv | 35 |
| docs-config-formats.csv | 35 |
| comfy-cli-formats.csv | 34 |
| cross-formats.csv | 34 |
| desktop-settings.csv | 31 |
| backend-websocket-events.csv | 26 |
| desktop-shell-actions.csv | 26 |
| comfy-cli-lifecycle.csv | 24 |
| frontend-persisted-state.csv | 24 |
| comfy-cli-schemas.csv | 23 |
| comfy-cli-config.csv | 20 |
| docs-lifecycle-contracts.csv | 20 |
| desktop-keybindings-gestures.csv | 19 |
| comfy-cli-extensions.csv | 17 |
| comfy-cli-documentation.csv | 16 |
| backend-inactive-nodes.csv | 12 |
| comfy-cli-events.csv | 12 |
| frontend-http-usage.csv | 7 |
| desktop-source-plugins.csv | 6 |
| desktop-feature-flags.csv | 3 |
| desktop-platform-matrix.csv | 3 |
| frontend-routes.csv | 1 |
| **Total** | **13,295** |

The individual source ledgers contain richer registry-specific columns. `catalogs/features.csv` normalizes those rows into actor, trigger, conditions, observable state, failure/recovery, persistence, protocol/side effects, platform variants, target gap, acceptance, and trace fields without replacing the source ledgers.
