# Sim native-runtime architecture evidence

## Outcome

The target has strong generic GPUI, workspace, persistence, task, Wasmtime, rendering, media, process, settings, and testing primitives plus validated native Comfy foundations for schemas, settings, trust, tensors/CPU execution, the Rust worker, safe formats, Rust/WASM plugins, registries, execution reducers, workflow/media adapters, file services, the GPUI graph shell, and a profile-scoped execution presentation service with a production-registered Execution dock panel. These foundations are partial until their later breadth and release tasks pass; planned work is never counted as current support.

Production must be a Sim-owned Rust control plane plus a Sim-owned Rust compute worker per selected device group. ComfyUI is a development-only conformance oracle. Production may not launch, manage, bundle, connect to, or depend on ComfyUI/Python, and may not execute Python or JavaScript compatibility extensions.

## Inspected target areas

- `crates/sim/src/main.rs`, `crates/sim/src/sim.rs`, generated canonical menus, and visual-test infrastructure.
- `crates/workspace`, `crates/gpui`, `crates/ui`, `crates/sim_actions`, `assets/keymaps`, `crates/settings`, `crates/settings_content`, `crates/settings_ui`, and `crates/db`.
- `crates/git_ui/src/git_graph.rs`, GPUI interaction tests, `crates/http_client`, `crates/remote`, `crates/util/src/process.rs`, `crates/terminal`, and `crates/fs`.
- `crates/image_viewer`, `crates/audio`, `crates/media`, `crates/auto_update`, `crates/extension_host`, `crates/sandbox`, `crates/system_specs`, `crates/paths`, and credentials providers.
- Root `Cargo.toml`, `crates/llama_cpp`, and `crates/gpui_wgpu` for native compute/runtime evidence.

The machine-readable companion is `catalogs/sim-architecture.csv`.

## Current support and constraints

| Capability | Status | Evidence-backed conclusion |
| --- | --- | --- |
| Native execution, tensors, autograd, RNG | partial | The native tensor facade, deterministic CPU contract, autograd/RNG foundations, prompt compiler, DAG/cache/queue/history reducers, and private execution events are implemented; the generated operator/device/model breadth remains. |
| Native models/formats/samplers/schedulers | partial | Safe bounded model formats and descriptor registries are implemented; full family, quantization, attention, sampler, scheduler, latent, and diffusion execution remains. |
| Native worker/device/memory planner | partial | Versioned private Rust IPC, worker supervision, cancellation, output transactions, recovery, and the CPU backend are implemented; vendor devices and the full memory planner remain. |
| Workflow/graph/UI | partial | Lossless workflow/prompt/media adapters, file authority, a registered serializable GPUI graph item, typed ports/widgets/links, commands, persistence, generated scoped keymaps and native menus, and the profile-scoped Execution dock panel with queue/history/output/error projections are implemented; later panels/editors/shell breadth remains. |
| Native HTTP/WebSocket/CLI host | missing | Generic clients exist; no Rust Comfy handlers/event projection or `sim comfy` contract exists. |
| Rust/WASM plugins | partial | The dedicated versioned Rust/WIT SDK and bounded Component Model host implement explicit typed ports, handles, grants, limits, cancellation, and deterministic legacy mapping; frontend/Python legacy breadth remains. |
| Media/output compatibility | partial | Bounded shared metadata carriers, native asset namespaces, indexing, and transactional outputs are implemented; the full image/audio/video/3D codec and editor matrix remains. |
| Accessibility | partial | Production uses `Application::with_platform`; the native graph has an application role, a scoped `ComfyGraph` key context, semantic entity/control labels, focus, and live announcements. VAL-GPUI-012/013 own this bootstrap; later surface tasks retain the whole-application audit. |
| Python/JavaScript extension behavior | conflicting | Source execution contracts are intentionally replaced by Rust/WASM and lossless placeholders; they are not delegated to Python or a browser. |
| Cloud/paid providers | missing/uncertain | Existing clients are not verified Comfy provider contracts; they remain disabled without approved APIs, grants, credentials, and tests. |

## Task 18 exact execution ownership ledger

This ledger is authoritative for Task 18. `regenerate_native_sim_evidence.py` rejects any mismatch between these 119 source feature IDs and `crates/comfy_ui/src/execution_catalog.rs`; it also rejects command, menu, component-catalog, and VAL-GPUI-005 component-set drift. `partial` records a concrete native or consumed foundation implementation without claiming later closure. `deferred` retains the exact later executable owner without misclassifying an intentionally later-owned row as an unaccounted gap.

### Queue feature dispositions

| Feature | Source contract | Disposition | Current owner | Closure owner | Target status |
| --- | --- | --- | --- | --- | --- |
| `COMFY-QUEUE-001` | Interrupt, single-job cancel, bulk cancel, pending clear, and history clear | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-002` | Execution lifecycle, node progress, previews, outputs, errors, and notifications | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-003` | Queue, job history, polling, reconnect, retry, stale-job, and overlay state | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-api-host` | `partial` |
| `COMFY-QUEUE-004` | Prompt serialization, batch queueing, queue-front, and partial execution | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-005` | Frontend HTTP contract GET /prompt | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-006` | Frontend HTTP contract POST /prompt | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-007` | Frontend HTTP contract GET /queue | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-008` | WebSocket/event message status | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-009` | WebSocket/event message progress | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-010` | WebSocket/event message progress_state | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-011` | WebSocket/event message progress_text | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-012` | WebSocket/event message executing | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-013` | WebSocket/event message executed | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-014` | WebSocket/event message execution_start | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-015` | WebSocket/event message execution_success | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-016` | WebSocket/event message execution_error | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-017` | WebSocket/event message execution_interrupted | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-018` | WebSocket/event message execution_cached | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-019` | WebSocket/event message b_preview | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-020` | WebSocket/event message b_preview_with_metadata | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-021` | WebSocket/event message promptQueueing | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-022` | WebSocket/event message promptQueued | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-023` | WebSocket/event message reconnecting | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-024` | WebSocket/event message reconnected | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-025` | Setting Comfy.Execution.PreviewMethod: Live preview method | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-026` | Setting Comfy.PromptFilename: Prompt for filename when saving workflow | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-027` | Setting Comfy.Queue.History.Expanded: Comfy.Queue.History.Expanded | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-028` | Setting Comfy.Queue.MaxHistoryItems: Queue history size | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-029` | Setting Comfy.Queue.QPOV2: Docked job history/queue panel | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-030` | Setting Comfy.Queue.ShowRunProgressBar: Comfy.Queue.ShowRunProgressBar | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-031` | Setting Comfy.QueueButton.BatchCountLimit: Batch count limit | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-032` | Setting Comfy.Toast.DisableReconnectingToast: Comfy.Toast.DisableReconnectingToast | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-033` | Command Comfy.Interrupt: Interrupt | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-034` | Command Comfy.Memory.UnloadModelsAndExecutionCache: Unload Models and Execution Cache | `later_owned` | `none` | `comfy-parity-native-memory-planner` | `deferred` |
| `COMFY-QUEUE-035` | Command Comfy.Queue.ToggleOverlay: Toggle Job History | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-036` | Command Comfy.QueuePrompt: Queue Prompt | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-037` | Command Comfy.QueuePromptFront: Queue Prompt (Front) | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-038` | Command Comfy.QueueSelectedOutputNodes: Queue Selected Output Nodes | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-039` | Command Comfy.ToggleQPOV2: Toggle Queue Panel V2 | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-040` | workflow actions: Export API prompt JSON | `later_owned` | `none` | `comfy-parity-workflow-experience` | `deferred` |
| `COMFY-QUEUE-041` | actionbar.spec: Does not auto-queue multiple changes at a time | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-042` | appModeValidationWarning.spec: keeps the app mode run button enabled when the warning is visible | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-workflow-experience` | `partial` |
| `COMFY-QUEUE-043` | appModeWidgetValues.spec: Widget values are sent correctly in prompt POST | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-workflow-experience` | `partial` |
| `COMFY-QUEUE-044` | bottomPanelLogs.spec: resyncs the terminal when the WebSocket reconnects | `later_owned` | `none` | `comfy-parity-process-diagnostics` | `deferred` |
| `COMFY-QUEUE-045` | bottomPanelLogs.spec: resumes WebSocket log streaming after the reconnect | `later_owned` | `none` | `comfy-parity-process-diagnostics` | `deferred` |
| `COMFY-QUEUE-046` | defaultKeybindings.spec: 'Ctrl+Enter' queues prompt | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-047` | defaultKeybindings.spec: 'Ctrl+Shift+Enter' queues prompt to front | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-048` | defaultKeybindings.spec: 'Ctrl+Alt+Enter' interrupts execution | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-049` | publishDialog.spec: shows profile creation prompt when user has no profile | `later_owned` | `none` | `comfy-parity-workflow-experience` | `deferred` |
| `COMFY-QUEUE-050` | publishDialog.spec: shows save prompt for temporary workflow | `later_owned` | `none` | `comfy-parity-workflow-experience` | `deferred` |
| `COMFY-QUEUE-051` | queueClearHistory.spec: Dialog opens from queue panel history actions menu | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-052` | queueClearHistory.spec: Cancel button closes dialog without clearing history | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-053` | queueClearHistory.spec: Close (X) button closes dialog without clearing history | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-054` | queueClearHistory.spec: Confirm clears queue history and closes dialog | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-055` | queueClearHistory.spec: Dialog state resets after close and reopen | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-056` | errorDialog.spec: Should display an error dialog when prompt execution fails | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-057` | errorOverlay.spec: Error overlay appears on execution error | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-058` | execution.spec: Report error on unconnected slot | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-059` | execution.spec: Execute to selected output nodes | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-060` | execution.spec: preserves validation errors when another active root starts execution | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-061` | interaction.spec: Can enter prompt | `foundation` | `comfy-parity-native-graph` | `comfy-parity-native-graph` | `partial` |
| `COMFY-QUEUE-062` | interaction.spec: Can close prompt dialog with canvas click (number widget) | `foundation` | `comfy-parity-native-graph` | `comfy-parity-native-graph` | `partial` |
| `COMFY-QUEUE-063` | interaction.spec: Can close prompt dialog with canvas click (text widget) | `foundation` | `comfy-parity-native-graph` | `comfy-parity-native-graph` | `partial` |
| `COMFY-QUEUE-064` | jobHistoryActions.spec: Docked job history action is visible with text | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-065` | jobHistoryActions.spec: Show run progress bar action is visible | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-066` | jobHistoryActions.spec: Clicking docked job history closes popover | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-067` | jobHistoryActions.spec: Clicking show run progress bar toggles setting | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-068` | linearMode.spec: Run button visible in linear mode | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-069` | metadataWorkflowImport.spec: loads Python JSON prompt with NaN/Infinity from ${fileName} (${parser}) | `foundation` | `comfy-parity-workflow-formats` | `comfy-parity-workflow-formats` | `partial` |
| `COMFY-QUEUE-070` | minimap.spec: Dragging on minimap pans the main canvas progressively | `foundation` | `comfy-parity-native-graph` | `comfy-parity-native-graph` | `partial` |
| `COMFY-QUEUE-071` | nodeSearchBoxV2Extended.spec: Search narrows results progressively | `later_owned` | `none` | `comfy-parity-assets-editors-viewers` | `deferred` |
| `COMFY-QUEUE-072` | outputHistory.spec: Skeleton appears on execution start | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-073` | outputHistory.spec: Multiple outputs from single execution | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-074` | outputHistory.spec: Cancel button sends interrupt during execution | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-075` | outputHistory.spec: Full execution lifecycle cleans up in-progress items | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-076` | outputHistory.spec: Auto-selection follows latest in-progress item | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-077` | outputHistory.spec: Clicking item breaks auto-follow during execution | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-078` | outputHistory.spec: In-progress items are outside the scrollable area | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-079` | outputHistory.spec: Execution error cleans up in-progress items | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-080` | outputHistory.spec: Progress bars update for both node and overall progress | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-081` | performance.spec: workflow execution | `later_owned` | `none` | `comfy-parity-performance` | `deferred` |
| `COMFY-QUEUE-082` | previewAsText.spec: does not include preview widget values in the API prompt | `foundation` | `comfy-parity-workflow-formats` | `comfy-parity-workflow-formats` | `partial` |
| `COMFY-QUEUE-083` | errorsTab.spec: Should keep execution errors matching the search query | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-084` | errorsTabExecution.spec: Should show Find on GitHub and Copy buttons in error card | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-085` | errorsTabExecution.spec: Should show runtime error log in the execution error group | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-086` | queueOverlay.spec: Toggle button opens expanded queue overlay | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-087` | queueOverlay.spec: Overlay shows filter tabs (All, Completed) | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-088` | queueOverlay.spec: Overlay shows Failed tab when failed jobs exist | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-089` | queueOverlay.spec: Completed filter shows only completed jobs | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-090` | queueOverlay.spec: Toggling overlay again closes it | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-091` | queueOverlay.spec: Job details popover stays inside the viewport for bottom rows | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-092` | queueSettings.spec: limit query parameter on /api/jobs reflects the setting | `later_owned` | `none` | `comfy-parity-native-api-host` | `deferred` |
| `COMFY-QUEUE-093` | queueSettings.spec: queue panel caps history items to the configured number | `later_owned` | `none` | `comfy-parity-settings-localization-ui` | `deferred` |
| `COMFY-QUEUE-094` | queueButtonModes.spec: Run button is visible in topbar | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-095` | queueButtonModes.spec: Queue mode trigger menu is visible | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-096` | queueButtonModes.spec: Clicking queue mode trigger opens mode menu | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-097` | queueButtonModes.spec: Queue mode menu shows available modes | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-098` | queueButtonModes.spec: Selecting a non-default mode updates the Run button label | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-099` | queueButtonModes.spec: Run button sends prompt when clicked | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-100` | queueNotificationBanners.spec: promptQueueing event shows a queueing banner | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-101` | queueNotificationBanners.spec: promptQueued upgrades a pending banner to queued | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-102` | queueNotificationBanners.spec: promptQueued with batch count > 1 shows plural text | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-103` | queueNotificationBanners.spec: promptQueued with mismatched requestId enqueues a separate queued banner | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-104` | queueNotificationBanners.spec: Banner auto-dismisses after timeout | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-105` | queueNotificationBanners.spec: Second notification shows after first auto-dismisses | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-106` | queueNotificationBanners.spec: promptQueued without prior queueing shows queued banner directly | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-107` | selectionToolboxRename.spec: Rename shows prompt dialog for group | `foundation` | `comfy-parity-native-graph` | `comfy-parity-native-graph` | `partial` |
| `COMFY-QUEUE-108` | jobHistory.spec: opens from the queue overlay docked history action | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-109` | jobHistory.spec: clears pending queue jobs and leaves running/history jobs | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-110` | jobHistory.spec: disables clear queue | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-111` | subgraphNavigation.spec: Stale progress is cleared on subgraph node after navigating back | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-112` | subgraphNavigation.spec: Stale progress is cleared when switching workflows while inside subgraph | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-113` | subgraphNested.spec: Loads and queues without nested promotion resolution failures | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-execution-e2e` | `partial` |
| `COMFY-QUEUE-114` | workflowTabStatus.spec: drops the indicator on user interrupt rather than showing an error | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-workflow-experience` | `partial` |
| `COMFY-QUEUE-115` | error.spec: should display error state when node causes execution error | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-116` | error.spec: parent subgraph node shows error ring when an interior node fails execution | `native` | `comfy-parity-execution-ui` | `comfy-parity-execution-ui` | `partial` |
| `COMFY-QUEUE-117` | workflowDeleteSettings.spec: on (default): right-click → Delete prompts the confirm dialog | `later_owned` | `none` | `comfy-parity-workflow-experience` | `deferred` |
| `COMFY-QUEUE-118` | workflowSettings.spec: toggling sort preserves node set in both workflow JSON and API prompt | `foundation` | `comfy-parity-workflow-formats` | `comfy-parity-workflow-formats` | `partial` |
| `COMFY-QUEUE-119` | wsReconnectStaleJob.spec: preserves active job when the queue endpoint fails on reconnect | `shared_closure` | `comfy-parity-execution-ui` | `comfy-parity-native-api-host` | `partial` |

### Execution command dispositions

| Command | Feature | Owner | Native action now | Target status |
| --- | --- | --- | --- | --- |
| `Comfy.ClearPendingTasks` | `COMFY-UI-067` | `comfy-parity-execution-ui` | `yes` | `partial` |
| `Comfy.Interrupt` | `COMFY-QUEUE-033` | `comfy-parity-execution-ui` | `yes` | `partial` |
| `Comfy.Memory.UnloadModels` | `COMFY-UI-075` | `comfy-parity-native-memory-planner` | `no` | `deferred` |
| `Comfy.Memory.UnloadModelsAndExecutionCache` | `COMFY-QUEUE-034` | `comfy-parity-native-memory-planner` | `no` | `deferred` |
| `Comfy.QueuePrompt` | `COMFY-QUEUE-036` | `comfy-parity-execution-ui` | `yes` | `partial` |
| `Comfy.QueuePromptFront` | `COMFY-QUEUE-037` | `comfy-parity-execution-ui` | `yes` | `partial` |
| `Comfy.QueueSelectedOutputNodes` | `COMFY-QUEUE-038` | `comfy-parity-execution-ui` | `yes` | `partial` |
| `Comfy.Queue.ToggleOverlay` | `COMFY-QUEUE-035` | `comfy-parity-execution-ui` | `yes` | `partial` |
| `Comfy.ToggleQPOV2` | `COMFY-QUEUE-039` | `comfy-parity-execution-ui` | `yes` | `partial` |

### Job and run menu dispositions

| Feature | Menu action | Owner | Target status |
| --- | --- | --- | --- |
| `COMFY-MENU-001` | Inspect asset | `comfy-parity-assets-editors-viewers` | `deferred` |
| `COMFY-MENU-002` | Add to current workflow | `comfy-parity-assets-editors-viewers` | `deferred` |
| `COMFY-MENU-003` | Download | `comfy-parity-assets-editors-viewers` | `deferred` |
| `COMFY-MENU-004` | Open workflow | `comfy-parity-workflow-experience` | `deferred` |
| `COMFY-MENU-005` | Export workflow | `comfy-parity-workflow-experience` | `deferred` |
| `COMFY-MENU-006` | Copy job ID | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-007` | Delete asset | `comfy-parity-assets-editors-viewers` | `deferred` |
| `COMFY-MENU-008` | Copy error message | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-009` | Report error | `comfy-parity-process-diagnostics` | `deferred` |
| `COMFY-MENU-010` | Remove job | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-011` | Cancel job | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-057` | Docked job history | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-058` | Show run progress bar | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-059` | Clear history | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-060` | Run | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-061` | Run on change | `comfy-parity-execution-ui` | `partial` |
| `COMFY-MENU-062` | Run instant | `comfy-parity-execution-ui` | `partial` |

### Execution component dispositions

| Feature | Source surface | Owner | Target status |
| --- | --- | --- | --- |
| `COMFY-FRONTEND-SURFACE-922B12C3CA3D` | ErrorOverlay surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-F7223A6667BB` | ExecuteButton surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-19BAB3FC51C6` | QueueInlineProgress surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-BA68BC33A2AB` | QueueInlineProgressSummary surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-67797BF57062` | QueueNotificationBanner surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-63A4ABE54AC4` | QueueNotificationBannerHost surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-F494BDB6FD2E` | QueueOverlayActive presentation disposition | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-052D51C10184` | QueueOverlayExpanded surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-0C01631C3DFA` | QueueOverlayHeader surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-BE5BE58D2FDE` | QueueProgressOverlay surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-26F40752861E` | QueueClearHistoryDialog surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-6085B98C498A` | JobContextMenu surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-F6FF6DAE75BF` | JobDetailsHoverPopover surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-F3428874E71D` | JobDetailsPopover surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-9F0D36286AB9` | JobFilterActions surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-FB9FC24AF7FA` | JobFilterTabs surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-BC42336531A9` | JobFiltersBar surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-A14F4CA91E43` | ErrorCardSection surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-E721F4A4F9B9` | ErrorGroupList surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-97D04E89D68E` | ErrorNodeCard surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-F69CDE266EDA` | TabErrors surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-92B3D7C9D258` | ProgressToastItem surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-228A24CC9226` | LinearProgressBar surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-1F516FA6CD5A` | OutputHistoryActiveQueueItem surface contract | `comfy-parity-execution-ui` | `partial` |
| `COMFY-FRONTEND-SURFACE-6F5EE356A779` | ManagerProgressToast surface contract | `comfy-parity-frontend-extension-compatibility` | `deferred` |

## Recommended production architecture

```text
GPUI workspace item / dock panels / settings
                |
                v
ComfyRuntime application service
  workflow + queue/history + cache + journals + native API projection
                |
       private versioned Rust IPC
                |
Sim-owned Rust compute worker per device group
  tensor/autograd/RNG + native backends + memory planner
  ArtifactIndex/ModelStore + model families/patches
  native DAG executor + nodes + samplers/schedulers
  bounded Rust/WASM plugin host + native media/output transactions
```

GPUI never talks to the public compatibility host internally. The worker isolates GPU faults and large model-memory lifetimes but remains part of Sim. Live tensors/device pointers do not cross IPC. Public HTTP/WebSocket and headless CLI project the same native services and never forward to ComfyUI.

## Compute and model implications

Comfy source evidence uses autograd for training nodes, custom operations, and gradient-dependent samplers, so inference-only tensor support is insufficient. A Sim-owned tensor facade must define shape, broadcasting, dtype promotion/accumulation, strides/layout, view/copy, empty/scalar, NaN/infinity/rounding, device/fallback, determinism, VJP, RNG, cancellation, and structured errors. A native Rust backend ecosystem may sit behind that facade, but its types cannot become workflow/plugin compatibility APIs.

The reference CPU backend anchors semantics. CUDA, ROCm, Metal, DirectML, XPU, NPU, MLU, and CoreX adapters require actual operation/dtype/layout/memory certification. WGPU is not described as MPS or DirectML merely because it uses Metal or D3D12. The GPUI rendering device has no current inference scheduling, long-kernel cancellation, memory isolation, or device-loss contract, so initial compute devices are worker-owned and separate.

Model loading requires bounded safetensors and GGUF readers plus a restricted weights-only PyTorch archive/pickle reader that never executes reducers. Every one of the 94 family rows needs a detector, descriptor, tiny fixture, mapping, forward checkpoints, dtype/device matrix, and exact failure/cancellation/OOM cases. LoRA/LoHa/LoKr/OFT, ControlNet, VAE, CLIP, merges, quantization, and patches form ordered copy-on-write graphs included in cache identity.

## GPUI ownership

- `ComfyRuntime` is an application service/global, not an expansion of every `workspace::AppState` constructor.
- Each workflow is a serializable workspace item bound to a stable native `ProfileId`. Node library, queue/history, assets/models, operations, logs, and diagnostics are dock panels. Sustained editors are workspace items; bounded choices use modals/popovers.
- Foreground entity work remains short and fallible. Expensive parsing, indexing, hashing, model/tensor/media work runs in background tasks or the Rust worker. Stored task handles/cancellation tokens match ownership; failures reach visible entities and durable attempt/operation states.
- New persistence domains register through existing migration patterns. Critical workflow/attempt/output/journal writes are awaited and surfaced. Settings integrate through the central settings schema/page/defaults, not a private unregistered file.
- Graph rendering needs an accessible semantic companion with stable node/port/widget relationships, actions, selection, and focus. Production cannot retain the inaccessible default.

## Rust/WASM plugin boundary

Curated plugins use a versioned Rust source trait and are linked/signed components; no stable Rust dylib ABI is promised. Third-party Rust authors compile a versioned WIT Component Model. Manifests declare plugin/API versions, digest/signature/provenance, node versions, explicit port IDs/types/cardinality/default/lazy/serialization, legacy Python/JS identifiers, grants, effects, cache/determinism, and declarative UI.

WASM stores use bounded memory/table/instances/channels/output, fuel/epoch/deadlines, capability revocation, opaque invocation-scoped tensor/model/asset handles, and deterministic legacy resolution. No raw GPU pointers, Python modules, JavaScript/DOM/LiteGraph hooks, arbitrary web directories, Node host, or browser fallback executes.

## Validation consequences

- Add operator/autograd/RNG catalogs and exact CPU/backend matrices.
- Compare all 44 sampler trajectories, all 9 sigma schedules, all 33 latent formats, and every model-family checkpoint, not only final images.
- Implement every one of 565 local and 224 API-node rows natively or through a native provider, with exact per-node schema and behavior evidence.
- Run the first native slice `LoadImage -> ImageScale -> ImageInvert -> PreviewImage -> SaveImage`, including cache, cancellation, worker kill/recovery, metadata/output transaction, GPUI inspection, and no-network/no-Python/no-source-tree gates.
- Run the shape-reduced diffusion slice through checkpoint loading, CLIP, latent, KSampler, VAE, and SaveImage with intermediate checkpoints, OOM, cancellation, and recovery.
- Inspect Cargo reverse dependencies, package manifests/binaries/settings/menus/CLI, and runtime network/process traces to prove no production Comfy/Python/JS/browser/external fallback.
- Use GPUI executor timers in GPUI tests and `./script/clippy` for repository lint validation.

## Open uncertainties

- Native backend ecosystem selection remains an implementation ADR after prototype conformance and licensing/distribution measurement; the Sim tensor facade and fixtures prevent vendor lock-in.
- Device rows cannot be promoted without actual hardware/driver certification. Unavailable labs remain conditional, not guessed.
- Native codec libraries, vendor SDKs, model licenses, package size, signing/notarization, and unsafe FFI require platform/security review.
- Cloud/paid service semantics remain unverified without approved contracts and non-mutating test accounts.
- This generator does not execute runtime tests; completed task evidence and VAL-GPUI-012/013 artifacts record executable validation, while later uncompleted rows retain no passing claim.
