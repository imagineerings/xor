#!/usr/bin/env python3

from __future__ import annotations

import csv
import hashlib
import json
import re
from pathlib import Path


SPEC_ROOT = Path(__file__).resolve().parent
REPO_ROOT = SPEC_ROOT.parents[2]
CATALOGS = SPEC_ROOT / "catalogs"
DESKTOP_PREFIX = "projects/comfy/Comfy-Desktop/"
OUTPUT_NAME = "desktop-renderer-surfaces.csv"


def contract(
    surface: str,
    parents: str,
    actor: str,
    trigger: str,
    preconditions: str,
    inputs: str,
    success: str,
    interaction: str,
    state: str,
    failure: str,
    persistence: str,
    interfaces: str,
    reason: str,
    *,
    availability: str = "active",
    classification: str = "functional",
) -> dict[str, str]:
    return {
        "component_surface": surface,
        "parent_feature_ids": parents,
        "classification": classification,
        "availability": availability,
        "actor": actor,
        "trigger": trigger,
        "preconditions": preconditions,
        "inputs_defaults": inputs,
        "observable_success": success,
        "interaction_accessibility": interaction,
        "state_concurrency": state,
        "failure_recovery": failure,
        "persistence_serialization": persistence,
        "interfaces_side_effects": interfaces,
        "disposition_reason": reason,
    }


SURFACES = {
    "src/renderer/src/comfySystemModal/SystemModalApp.vue": contract(
        "System confirmation modal bridge host",
        "COMFY-DESKTOP-174;COMFY-DESKTOP-176",
        "Desktop main process and the user resolving a privileged confirmation.",
        "The context-isolated system-modal bridge delivers a modal specification, then the user activates confirm, cancel, or the optional secondary action.",
        "The reusable system-modal WebContents is mounted and the delivered specification has a stable modal id.",
        "SystemModalSpec carries id, title, message, optional detail groups, labels, action tones, and a retained-but-ignored theme field.",
        "The latest specification renders through BaseAlert; ready is sent after listener registration, notifyRendered is sent after the latest Vue flush, and action sends the exact modalId with confirm, cancel, or secondary.",
        "BaseAlert owns dialog semantics and dismissal; detail groups render as labeled lists, and every resolution has a visible labeled control.",
        "A monotonically increasing renderSeq suppresses stale render acknowledgements; the single bridge listener is unsubscribed on unmount.",
        "A missing bridge or missing current specification is a no-op; a superseded render tick cannot acknowledge the newer modal, and user cancellation remains a distinct result.",
        "Modal state is transient to the reusable WebContents; no modal choice is durably stored by this component.",
        "window.__comfySystemModal.ready/onModal/notifyRendered/action; privileged work remains in the main-process handler selected by modalId.",
        "This is an independently observable confirmation handshake, not merely modal chrome.",
    ),
    "src/renderer/src/comfyTitlePopup/MenuView.vue": contract(
        "Title-popup menu item renderer and activation surface",
        "COMFY-DESKTOP-178;COMFY-DESKTOP-181",
        "A title-bar popup user.",
        "The user clicks a non-separator menu row that has an id.",
        "The popup supplies ordered MenuItem records with optional localization key, fallback label, checked state, separator kind, and id.",
        "Missing labelKey falls back to label; separators and checked indicators are presentation inputs and only id-bearing ordinary items activate.",
        "Rows render localized labels, checkmarks, and role=menuitem; selecting a valid row emits its exact id once.",
        "Separators use role=separator. Menu items have tabindex=-1 and this component declares no arrow, Enter, or Space handler, so keyboard movement/activation must be supplied by the host or treated as an explicit source limitation.",
        "The component is stateless; activation is synchronous and ordered by the input item array.",
        "Separator clicks and rows without an id emit nothing; an unknown localization key uses the supplied English fallback.",
        "No persistence; the activation target owns any durable change.",
        "Vue activate(id) emit consumed by TitlePopupApp and then window.__comfyTitlePopup.activate(id).",
        "The row-level guard and localization behavior are independently testable menu contracts.",
    ),
    "src/renderer/src/comfyTitlePopup/PickerInlineProgress.vue": contract(
        "Instance-picker operation progress and terminal-action surface",
        "COMFY-DESKTOP-058;COMFY-DESKTOP-059;COMFY-DESKTOP-136;COMFY-DESKTOP-185",
        "A user monitoring an installation or instance-picker operation.",
        "The operation snapshot changes or the user chooses cancel, open, retry, or dismiss.",
        "OperationStatus supplies done, ok, error, status, percent, cancellable, and operation-kind data; installationName identifies the target.",
        "Negative percent is indeterminate; determinate percent is clamped to 0..100; MSG_CANCELLED is a distinct cancelled terminal state.",
        "Inflight, success, error, and cancelled states are mutually exclusive and expose only their valid actions; success names the installation and errors render OperationErrorDetail.",
        "Actions are native buttons. Visible status and percentage accompany the animated ring; reduced-motion behavior is not declared locally and remains a validation target.",
        "The surface derives state solely from the latest operation prop and emits one action request; the owner controls cancellation/retry ordering.",
        "Unknown/negative progress stays indeterminate, cancelled work never appears as a generic error, and non-cancellable work exposes no cancel button.",
        "No persistence; operation progress and terminal state are supplied by the owning lifecycle service.",
        "Vue open/cancel/retry/dismiss emits; OperationErrorDetail and progressStatusLabel helpers; no direct privileged call.",
        "The four-state progress contract and action gating affect recovery behavior.",
        availability="conditional",
    ),
    "src/renderer/src/comfyTitlePopup/TitlePopupApp.vue": contract(
        "Reusable title-popup root and view switcher",
        "COMFY-DESKTOP-086;COMFY-DESKTOP-140;COMFY-DESKTOP-149;COMFY-DESKTOP-174;COMFY-DESKTOP-175",
        "Desktop main process and a title-bar popup user.",
        "Main sends menu, downloads, downloads-full, instance-picker, or global-settings configuration and subsequent live snapshots.",
        "A context-isolated title-popup bridge, PopupConfig theme/kind payload, and optional live download/picker/settings snapshots are available.",
        "Initial kind is menu, default theme is dark, collections are empty, and global-settings/app-update fields have explicit safe defaults until the first host snapshot.",
        "Exactly one child surface renders for the latest kind, locale and light/dark state synchronize, natural download/picker height is requested before reveal, and only the newest config sends notifyRendered.",
        "Escape closes the popup unless its own useModal dialog is visible; ModalDialog and DialogHost remain mounted for child flows. The root is intentionally not re-keyed, avoiding reopen flicker.",
        "renderSeq rejects stale paint acknowledgements; reused-WebContents local state survives reopen, live subscriptions update root snapshots, and every listener is removed on unmount.",
        "A pending modal is dismissed on applicable kind switches/reopens, missing measurement elements defer resize, and a missing bridge leaves a non-privileged inert shell.",
        "Popup view state is transient; persisted settings/download/installation state is owned by the host and arrives as snapshots.",
        "window.__comfyTitlePopup ready/onConfig/onDownloadsChanged/onInstancePickerSnapshot/onGlobalSettingsSnapshot/onWillShow/onDismissModals/requestSize/notifyRendered/activate/close.",
        "The root owns ordering, reuse, resize, and stale-update behavior across all title-popup surfaces.",
    ),
    "src/renderer/src/comfyTitlePopup/globalSettings/GitHubLinkCard.vue": contract(
        "Global-settings GitHub link and star-count card",
        "COMFY-DESKTOP-106;COMFY-DESKTOP-149",
        "A global-settings user.",
        "The user activates the card after or while star-count data is loading.",
        "url and stars are required; loading defaults false and label defaults to Comfy Desktop.",
        "A null star count hides the count, loading shows a skeleton, and a numeric count uses locale-aware compact notation with one fractional digit.",
        "Activation emits the exact URL once while the visible card shows its label, external-link affordance, and optional star count.",
        "The card is a native button with focus-visible styling; decorative icons and loading skeleton are aria-hidden, and the numeric star count has an unabridged aria-label.",
        "Rendering follows the latest props; the component does not initiate or deduplicate the star request.",
        "Missing star data leaves the link usable; external-open failure is owned by the consumer and must not mutate the card into a false success state.",
        "No persistence; star data is a host-provided snapshot.",
        "Vue open(url) emit; the consumer owns validated external URL opening.",
        "The link remains functional independently of optional network-derived star metadata.",
    ),
    "src/renderer/src/comfyTitlePopup/globalSettings/GlobalSettingsMicroSection.vue": contract(
        "Global-settings labeled micro-section layout primitive",
        "COMFY-DESKTOP-149",
        "A global-settings surface composing labeled content.",
        "The parent renders the section with title, optional tooltip, and slotted controls.",
        "The parent supplies the semantic title and owns every interactive slotted child.",
        "title is required; tooltip is optional; default slot supplies all functional content.",
        "The component emits no action and performs no service call; it groups the slot below a heading and optional InfoTooltip.",
        "A semantic section and h3 expose grouping; keyboard and focus behavior belong entirely to the slotted controls and InfoTooltip.",
        "Stateless and synchronous.",
        "An absent tooltip omits only the tooltip; missing functional slot content yields an empty labeled group without side effects.",
        "No persistence.",
        "Vue slot composition and InfoTooltip only.",
        "Explicit infrastructure-only classification: this wrapper contributes semantic grouping and layout, not an independent user workflow.",
        availability="infrastructure-only",
        classification="presentational/infrastructure-only",
    ),
    "src/renderer/src/comfyTitlePopup/globalSettings/GlobalStorageSections.vue": contract(
        "Immediate-write global model/input/output storage controls",
        "COMFY-DESKTOP-147;COMFY-DESKTOP-149;COMFY-DESKTOP-152;COMFY-DESKTOP-186;COMFY-DESKTOP-187",
        "A global-settings user managing shared storage roots.",
        "The user opens, browses, adds, changes, promotes, or removes a shared model/input/output directory.",
        "GlobalStorageSnapshot supplies directory fields, ordered model roots, and system default; a title-popup global-settings bridge must be present for privileged effects.",
        "All model roots render as shared; writes are immediate with no save step; unchanged or cancelled chooser results do nothing.",
        "Opening reveals the selected path; accepted chooser results update the exact field/list; add appends, change replaces, make-primary moves to index zero, and confirmed remove changes configuration without deleting files.",
        "Controls inherit ModelsDirList and StorageDirRow labels/menu behavior; destructive removal requires a localized confirmation.",
        "The parent snapshot remains authoritative; touched emits before each accepted mutation and async writes preserve user action order.",
        "Invalid indices, absent fields, cancelled chooser, unchanged paths, or denied removal are no-ops; bridge rejection remains visible through the owning modal/settings error channel.",
        "Successful bridge updates persist shared storage settings immediately; removal never removes filesystem contents.",
        "window.__comfyTitlePopup.globalSettingsBrowseFolder/globalSettingsOpenPath/globalSettingsUpdateField/globalSettingsSetModelsDirs plus touched emit.",
        "This component owns concrete storage mutation and destructive-confirmation contracts shared by the popup experience.",
    ),
    "src/renderer/src/comfyTitlePopup/globalSettings/ModelsDirList.vue": contract(
        "Ordered model-directory list with per-row action menus",
        "COMFY-DESKTOP-152;COMFY-DESKTOP-181;COMFY-DESKTOP-186;COMFY-DESKTOP-187",
        "A settings user managing model search roots.",
        "The user opens a directory, browses a replacement, views extra-path details, adds a root, or uses the row menu to promote/remove it.",
        "Ordered dirs and systemDefault are required; row flags identify primary, shared, locked, or extra paths.",
        "Locked/extra rows suppress mutation actions; only non-primary removable rows can be removed and only eligible non-primary rows can be promoted.",
        "Rows show primary/shared/local state, emit the exact source index for open/change/details/remove/make-primary, and emit add for the footer control.",
        "Buttons have accessible labels; the action menu exposes aria-haspopup/expanded and role=menu/menuitem, handles Escape, Up, Down with wraparound, and restores focus to its toggle on close when requested.",
        "Only one row menu is open; nextTick focuses its first item and outside row clicks close it without applying an action.",
        "Out-of-range or ineligible actions do not emit; locked/default constraints remain visible through omitted actions rather than failing after activation.",
        "No direct persistence; ordered-list mutations are emitted to the owning storage service.",
        "Vue change/remove/make-primary/open/add/details emits; DOM focus ownership for per-row menus.",
        "The list defines independently testable ordering, eligibility, menu, and focus contracts.",
    ),
    "src/renderer/src/comfyTitlePopup/globalSettings/UpdatesSection.vue": contract(
        "Desktop app-update status, progress, actions, and preferences",
        "COMFY-DESKTOP-107;COMFY-DESKTOP-109;COMFY-DESKTOP-110;COMFY-DESKTOP-113;COMFY-DESKTOP-114;COMFY-DESKTOP-155;COMFY-DESKTOP-185",
        "A global-settings user checking or installing a Desktop update.",
        "The update snapshot changes or the user chooses check-for-update, update-now, a preference field update, or path open.",
        "AppUpdateState, optional progress, download/check flags, capabilities, installed version, platform, last-check time, and preference sections are supplied by the host.",
        "Version labels normalize a leading v; actions are disabled while checking/downloading; systemManaged and canSelfUpdate choose explanatory/action variants.",
        "The surface distinguishes idle, available, downloading, downloaded, current, failure, and system-managed states; shows percent/detail where available; and emits only the applicable update/check/settings action.",
        "Native buttons expose disabled state and focus-visible styling; status is textual rather than color-only, and preference controls inherit SettingsSectionList semantics.",
        "It is a pure projection of the latest update snapshot; repeated clicks are blocked while check/download is active.",
        "Malformed dates/version labels fall back to safe text; unavailable self-update explains the platform path, and an error state remains actionable rather than reporting current.",
        "Preference updates may persist through the owner; update state/progress is transient and host-owned.",
        "Vue check-for-update/update-now/update-field/open-path emits; updater and filesystem effects occur behind the title-popup bridge owner.",
        "Update state, platform capability, progress, and action gating are observable renderer behavior.",
    ),
    "src/renderer/src/comfyTitlePopup/instancePicker/InstanceRow.vue": contract(
        "Keyboard-selectable instance-picker option row",
        "COMFY-DESKTOP-086;COMFY-DESKTOP-165;COMFY-DESKTOP-181",
        "A user choosing a local, remote, legacy, or cloud instance.",
        "The row is clicked or receives Enter/Space while focused.",
        "Installation is required; active/current/running/operating/update/recency/capacity props select badges and state.",
        "capacityStatus defaults to normal and absent recency/status metadata omits only those badges.",
        "The row emits the exact installation once and shows precedence-ordered operating, update, running, capacity, current, and recency state.",
        "role=option, aria-selected, tabindex=0, Enter, and Space provide keyboard parity. Capacity state is expressed with text pills, not color alone.",
        "The component is stateless. Even a cloud-disabled row emits select so the parent can explain the capacity block rather than silently ignoring the user.",
        "Missing optional metadata degrades to the base row; cloud disabled/degraded state is not mislabeled as running or launchable by this component.",
        "No persistence; selection and launch decisions belong to the picker state/service.",
        "Vue select(installation) emit; installTypeMeta projection only.",
        "Selection, status precedence, and disabled-cloud explanation are independently observable.",
    ),
    "src/renderer/src/comfyTitlePopup/instancePicker/PickerSnapshotsList.vue": contract(
        "Instance-picker snapshot list, disclosure, save, restore, and delete",
        "COMFY-DESKTOP-121;COMFY-DESKTOP-123;COMFY-DESKTOP-125;COMFY-DESKTOP-126;COMFY-DESKTOP-181",
        "A user managing snapshots for the selected instance.",
        "The user saves, expands a row, restores, or deletes a snapshot.",
        "SnapshotListData supplies ordered summaries and context; the popup has browser confirm but no useModal primitive.",
        "No snapshots renders an explicit empty state; summary text is derived from available ComfyUI/node/package deltas.",
        "Save emits immediately; expansion reveals snapshot detail; restore/delete emit the exact filename only after their localized destructive confirmation is accepted.",
        "Actions are buttons, but source disclosure markup must be checked for complete keyboard/expanded semantics during parity validation; window.confirm owns focus for destructive choices.",
        "Expanded filename is local transient state; an incoming list that lacks it naturally removes the expanded content.",
        "Cancelled confirmation emits nothing; empty lists remain usable for save; an invalid/missing snapshot cannot be restored or deleted through a rendered row.",
        "No direct persistence; save/restore/delete requests are emitted to the snapshot service.",
        "window.confirm and Vue save/restore(filename)/delete(filename) emits.",
        "The popup-specific confirmation and exact filename routing are not represented by the coarse snapshot rows alone.",
        availability="conditional",
    ),
    "src/renderer/src/components/ContextMenu.vue": contract(
        "Viewport-clamped renderer context menu",
        "COMFY-DESKTOP-179;COMFY-DESKTOP-181",
        "A renderer user invoking an item-specific context menu.",
        "open becomes true at pointer coordinates, the user chooses an enabled item, clicks outside, or presses Escape.",
        "open, x, y, and ordered ContextMenuItem inputs are required; separators, disabled state, danger style, title, label, and id come from each item.",
        "The menu is teleported to body and clamped within four pixels of the current viewport after layout.",
        "An enabled click emits select(id) then close; outside mousedown and Escape emit close; disabled clicks emit neither selection nor close.",
        "Items are native buttons with aria-disabled/title. The source declares no menu role or arrow-key roving focus, so Tab/button semantics and Escape are the only local keyboard behavior and the gap must stay explicit.",
        "Capture listeners exist only while open and are removed on close/unmount; adjusted coordinates reset for each opening.",
        "An empty item array renders nothing, offscreen coordinates clamp, and disabled actions cannot invoke the owner.",
        "No persistence; selected action owns any durable effect.",
        "Document mousedown/keydown capture listeners and Vue select/close emits; Teleport to body.",
        "Placement, listener lifetime, disabled behavior, and close ordering are source-specific contracts.",
    ),
    "src/renderer/src/components/DialogHost.vue": contract(
        "Singleton prompt, action-sheet, alert, and confirm host",
        "COMFY-DESKTOP-176;COMFY-DESKTOP-181",
        "Any renderer service using useDialogs and the user resolving its request.",
        "useDialogs opens one of prompt, actionSheet, alert, or confirm.",
        "A single DialogHost is mounted beside ModalDialog; useDialogs state supplies kind-specific labels, values, items, tones, details, and optional restore diff.",
        "The useDialogs discriminated state supplies the active kind and its prompt, action-sheet, alert, or confirm payload.",
        "Only the active kind is open; alert backdrop/Escape acknowledges, confirm preserves primary/secondary/cancel distinctions, and restore confirmations include SnapshotDiffView.",
        "BasePrompt, BaseActionSheet, and BaseAlert own dialog roles, labels, keyboard dismissal, focus, and button semantics.",
        "The singleton projects one reactive dialog state and resolves exactly the corresponding useDialogs promise/action.",
        "Closed state renders no active dialog; cancellation remains distinct for prompt/action/confirm, while alert dismissal is acknowledgement by design.",
        "Dialog state is transient; the action resolved by a caller may subsequently persist data.",
        "useDialogs submitPrompt/selectActionSheet/acknowledgeAlert/confirmPrimary/confirmSecondary/cancel; optional SnapshotDiffView.",
        "Resolution semantics across four dialog kinds are independently observable.",
    ),
    "src/renderer/src/components/DownloadThumbnail.vue": contract(
        "Lazy completed-image download thumbnail with fallback",
        "COMFY-DESKTOP-136;COMFY-DESKTOP-142",
        "A user viewing a download row.",
        "A completed image entry becomes eligible for thumbnail fetch or the rendered image reports an error.",
        "entry carries isImage, status, savePath, and filename; injected fetcher supports panel and title-popup bridges.",
        "Non-image/incomplete/unavailable thumbnails render the caller's fallback slot; images load lazily and asynchronously.",
        "A fetched thumbnail renders with localized filename alt text; a broken/moved image falls back, and changing the thumbnail source clears the failure latch.",
        "The image has meaningful localized alt text, is non-draggable, and fallback accessibility is owned by the slot provider.",
        "useDownloadThumbnail owns async fetch/caching lifetime; a local failed flag prevents repeated broken rendering until the source changes.",
        "Fetch rejection, missing file, or image decode error preserves the normal status icon rather than a broken image.",
        "No durable write; the component reads an existing completed download through the injected fetcher.",
        "Injected ThumbnailFetcher, useDownloadThumbnail, lazy img decode, and fallback slot.",
        "Thumbnail eligibility and post-download file-missing recovery are user-visible.",
        availability="conditional",
    ),
    "src/renderer/src/components/ImportPreviewModal.vue": contract(
        "Snapshot-file import preview confirmation modal",
        "COMFY-DESKTOP-129;COMFY-DESKTOP-131;COMFY-DESKTOP-176",
        "A user importing a snapshot file.",
        "The parent opens the legacy modal while preview loading progresses, then the user cancels or confirms.",
        "preview may be null and loading is required; SnapshotFilePreviewContent renders a valid preview.",
        "Confirm is absent without a preview and disabled while loading; the close affordance is cancellation.",
        "Loading text, full preview content, cancel, and conditional confirm states are mutually consistent; confirm/cancel emit once to the owner.",
        "ModalShell supplies a labeled close button and button keyboard semantics, but this legacy shell does not itself declare dialog role/focus capture.",
        "The parent owns async parse state; this component never confirms a null or still-loading preview.",
        "Malformed files are represented by the parent's absent/error preview; closing or cancelling emits no import mutation.",
        "No direct persistence; confirmed import is performed by the snapshot service.",
        "Vue cancel/confirm emits, ModalShell, and SnapshotFilePreviewContent.",
        "Preview gating prevents importing an unparsed artifact and is independently testable.",
        availability="conditional",
    ),
    "src/renderer/src/components/InstallNamePath.vue": contract(
        "Installation name, location, path validation, and disk-space form",
        "COMFY-DESKTOP-013;COMFY-DESKTOP-014;COMFY-DESKTOP-018;COMFY-DESKTOP-035;COMFY-DESKTOP-186;COMFY-DESKTOP-187",
        "A user naming an installation and selecting its location.",
        "The user types a name, opens/browses/resets the path, or path/disk validation changes.",
        "name, path, defaultPath, pathIssues, diskSpaceLoading, diskSpace, and estimatedSize are required; hideInstallPath optionally suppresses the entire path surface.",
        "Name input emits on each input; reset exists only when path differs from default; open exists only for a nonempty path.",
        "The exact name/path changes are emitted, path text opens the selected directory, and PathDiskInfo exposes current validation and capacity state.",
        "The name is label-associated; open-path has a localized accessible label containing the path; browse/reset are native buttons.",
        "Validation/disk state is parent-owned and may load concurrently with edits; this component performs no stale-result reconciliation.",
        "Empty paths cannot be opened; hidden path mode emits no path action; invalid/capacity issues remain visible without mutating the form.",
        "No direct persistence; values are emitted to the installation wizard/record owner.",
        "Vue update:name/update:path/browse/open emits and PathDiskInfo projection.",
        "This is the concrete renderer contract for path boundaries and form state.",
    ),
    "src/renderer/src/components/MigrationBanner.vue": contract(
        "Legacy-install migration decision and progress launch surface",
        "COMFY-DESKTOP-042;COMFY-DESKTOP-043;COMFY-DESKTOP-044;COMFY-DESKTOP-047;COMFY-DESKTOP-050",
        "A user with a detected legacy installation.",
        "The user starts migration, chooses quick install instead, or opens telemetry settings.",
        "A legacy Installation and functional useMigrateAction/window.api bridge are required.",
        "Migration confirmation may return no result; while confirmation is pending, migrating blocks duplicate starts.",
        "An accepted migration emits a cancellable show-progress operation whose apiCall runs migrate-to-standalone for the exact installation id and confirmed options.",
        "All branches use labeled buttons; the primary action disables during confirmation, while quick-install and telemetry settings remain explicit alternatives.",
        "migrating is reset in finally; the emitted progress owner controls long-running cancellation and terminal state.",
        "Cancelled confirmation emits no operation; confirmation error clears the duplicate guard; migration failure is surfaced by the progress owner.",
        "The component writes nothing directly; successful runAction performs the separately cataloged installation migration.",
        "useMigrateAction.confirmMigration, window.api.runAction, and show-progress/show-settings/show-quick-install emits.",
        "The confirm-to-cancellable-operation handoff and duplicate suppression are observable migration behavior.",
        availability="conditional",
    ),
    "src/renderer/src/components/Modal.vue": contract(
        "Legacy overlay/inline modal frame and dismissal primitive",
        "COMFY-DESKTOP-076;COMFY-DESKTOP-176",
        "A renderer composing a legacy modal and its user.",
        "The modal is mounted, Escape is pressed, or a pointer down-and-click completes on the backdrop.",
        "binding defaults false, width wide, contentClass empty, inline false; opacity defaults opaque for binding and dim otherwise.",
        "inline omits Teleport/backdrop/Escape; binding prevents backdrop/Escape dismissal; backdrop close requires both down and click on the overlay.",
        "Nonbinding overlay mode emits close on valid backdrop click or Escape and never closes from a drag that began inside content.",
        "This legacy primitive declares no dialog role, accessible name, focus capture/restore, or scroll lock; those are explicit source limitations versus BaseModal.",
        "A document keydown listener exists only for non-inline mounts and is removed on unmount.",
        "Binding/inline suppress dismissal, content clicks do not close, and an incomplete backdrop gesture resets without emitting.",
        "No persistence.",
        "Teleport to body, document keydown listener, and Vue close emit.",
        "Despite legacy status, backdrop and binding semantics are consumed by active surfaces and remain functional compatibility behavior.",
    ),
    "src/renderer/src/components/ModalDialog.vue": contract(
        "Legacy useModal singleton for confirm, option, prompt, select, and alert flows",
        "COMFY-DESKTOP-095;COMFY-DESKTOP-123;COMFY-DESKTOP-176",
        "Any renderer caller using useModal and the user resolving its modal.",
        "useModal publishes visible state and the user confirms, cancels, selects, submits text/options, follows a safe link, or dismisses.",
        "State supplies type, labels, message/details, loading, validators, choices, checkboxes, snapshot preview, and confirmation style.",
        "Simple alert/confirm uses BaseAlert; richer migration, confirmWithOptions, prompt, and select variants use legacy modal markup.",
        "Each modal resolves with the source-specific boolean/null/value/options result; prompt validation blocks invalid submission; snapshot/migration detail and safe http(s) links render where supplied.",
        "Escape resolves the applicable cancellation path, prompt Enter submits, and BaseAlert variants own focus/a11y; richer legacy branches have incomplete role/focus semantics that must remain explicit.",
        "Input/error/checkbox/disclosure state resets on visibility/type changes; overlay gesture tracking prevents accidental drag-to-dismiss.",
        "Loading disables terminal actions, validation errors remain visible, non-http(s) links are not externalized, and cancellation preserves the caller's prior state.",
        "Modal state is transient; selected checkbox values are retained only long enough for useModal result retrieval.",
        "useModal state/close/dismiss/getLastCheckboxValues, window.api.openExternal for validated links, and SnapshotFilePreviewContent.",
        "The singleton's result typing, validation, link boundary, and legacy/source variant split are independently observable.",
    ),
    "src/renderer/src/components/ModalShell.vue": contract(
        "Legacy modal header, body, close, and pinned-footer shell",
        "COMFY-DESKTOP-076;COMFY-DESKTOP-176",
        "A renderer composing Modal and the user invoking its close affordance.",
        "The wrapped Modal emits close or the user activates the corner close control.",
        "binding false, width wide, contentClass empty, inline false, title empty, hideClose false, and closeGlyph multiplication sign are defaults.",
        "Header/title/body always render; footer renders only when slotted; hideClose removes the corner control.",
        "The shell forwards Modal dismissal as close and emits close from a localized labeled button while preserving sizing/backdrop props.",
        "The close control has localized title/aria-label; inherited legacy Modal role/focus limitations remain.",
        "Stateless; close emits synchronously and the owner controls unmount.",
        "Binding can require the parent to provide another explicit close path; absent footer has no empty region.",
        "No persistence.",
        "Modal props/close emit and Vue named slots.",
        "Active legacy modals depend on this close and layout contract, so it is functional rather than decorative.",
    ),
    "src/renderer/src/components/RestoreModal.vue": contract(
        "Snapshot restore diff preview and destructive confirmation",
        "COMFY-DESKTOP-124;COMFY-DESKTOP-125;COMFY-DESKTOP-176",
        "A user previewing a snapshot restore.",
        "The diff loads, the user cancels/outside-dismisses, or confirms a nonempty restore.",
        "diffData may be null and loading is required; a nonempty SnapshotDiffData supplies summary badges and details.",
        "Loading, empty, and nonempty diff states are distinct; confirm renders only for a nonempty diff.",
        "The modal summarizes ComfyUI/channel/node/package changes, renders SnapshotDiffView, and emits confirm or cancel without performing restore directly.",
        "Native buttons are keyboard operable and shared overlay handling supports Escape/outside cancellation; the legacy markup lacks a local role/name/focus declaration.",
        "Diff state is parent-owned; overlay gesture tracking prevents content-originated clicks from cancelling.",
        "Null/loading/empty diffs cannot confirm; cancellation has no restore side effect; owner surfaces restore failure/partial recovery.",
        "No direct persistence; confirmed restore invokes the separately cataloged snapshot service.",
        "useModalOverlay, SnapshotDiffView, and Vue cancel/confirm emits.",
        "The nonempty-only destructive gate and exact diff projection are independent restore contracts.",
        availability="conditional",
    ),
    "src/renderer/src/components/SnapshotDiffView.vue": contract(
        "Snapshot ComfyUI, channel, custom-node, and package diff projection",
        "COMFY-DESKTOP-124;COMFY-DESKTOP-130",
        "A user inspecting or confirming a snapshot difference.",
        "A SnapshotDiffResult renders, and optional collapsible node/package headings are clicked.",
        "diff is required; collapsible is optional and false when absent.",
        "Only changed sections render; collapsed node/package groups start closed and noncollapsible groups are always shown.",
        "Added, removed, changed, version, commit-shortening, enabled-state, ComfyUI-version, and update-channel transitions render with exact before/after semantics.",
        "The source collapsible headings are click-only divs with no button role, aria-expanded, or key handler; this is an explicit accessibility gap for the Sim acceptance test.",
        "Node and package expansion are independent local booleans; input diff changes do not explicitly reset them.",
        "Empty change classes render nothing; absent version falls back to a seven-character commit or question mark.",
        "No persistence; disclosure state is local/transient.",
        "Pure SnapshotDiffResult projection and local disclosure state.",
        "Diff formatting and the click-only source disclosure behavior must be traceable rather than hidden under snapshot support.",
        availability="conditional",
    ),
    "src/renderer/src/components/SnapshotFilePreviewContent.vue": contract(
        "Imported snapshot-file timeline and newest-state preview",
        "COMFY-DESKTOP-129;COMFY-DESKTOP-131",
        "A user reviewing a parsed snapshot import file.",
        "A SnapshotFilePreview renders and the user toggles custom-node or package detail.",
        "preview is required and includes installation identity, ordered snapshots, and newest snapshot environment.",
        "Custom nodes start expanded and packages collapsed; the first timeline record receives the current tag.",
        "The surface shows source name/count, trigger/date/version/node/package timeline, newest environment, enabled node state, and package versions.",
        "The source disclosure headings are click-only divs without button/aria-expanded/keyboard behavior; textual state remains selectable/readable.",
        "Two independent local disclosure booleans persist for the component mount.",
        "Empty node/package lists render an em dash; malformed data is rejected by the parent parser rather than silently accepted here.",
        "No persistence; this is a read-only preview of the candidate import.",
        "SnapshotFilePreview projection plus snapshots formatting helpers.",
        "Timeline/current/newest semantics and disclosure defaults are visible import-decision behavior.",
        availability="conditional",
    ),
    "src/renderer/src/components/SnapshotInspector.vue": contract(
        "Snapshot detail, diff-mode, environment, and searchable dependency inspector",
        "COMFY-DESKTOP-123;COMFY-DESKTOP-124",
        "A user expanding a snapshot history row.",
        "Detail/diff data arrives, the user switches previous/current diff, searches, or toggles node/package sections.",
        "Detail/loading, diff mode/data/loading, snapshot index/count, and optional context are required.",
        "Nodes start expanded, packages collapsed, and searches empty; previous/current controls disable at the corresponding history boundary.",
        "The inspector shows loading, no-change, exact diff, environment facts, filtered custom nodes, and filtered pip packages; changing detail resets both searches and disclosures.",
        "Diff controls and search fields are keyboard-operable; source section disclosures are click-only divs lacking button/expanded keyboard semantics and require explicit parity remediation.",
        "Detail changes synchronously reset local state; diff loading is independent and stale-result prevention belongs to the owner.",
        "Boundary diff controls cannot fire, empty searches show no-results text, and absent detail never projects stale previous content.",
        "No direct persistence; inspection/search/disclosure state is transient.",
        "Vue toggle-diff(mode) emit, SnapshotDiffView, diffHasChanges, and formatting helpers.",
        "Search reset, boundary gating, and diff/detail concurrency are independently observable.",
        availability="conditional",
    ),
    "src/renderer/src/components/TitleBar.vue": contract(
        "Minimal draggable custom title bar",
        "COMFY-DESKTOP-076;COMFY-DESKTOP-085",
        "A Desktop window user.",
        "A renderer mounts the title bar and the user drags its drag region.",
        "Optional title; platform is inferred from navigator.userAgent.",
        "Absent title leaves only the drag region; macOS uses traffic-light spacing and other platforms reserve right-side window controls.",
        "The header paints the localized/provided title with ellipsis and exposes an app-region drag target to move the native window.",
        "The title text is noninteractive; native OS window controls and their keyboard/accessibility behavior remain outside this component.",
        "Platform computation is mount-local and stateless after render.",
        "A missing title does not remove window dragging; platform misidentification affects spacing but not title state.",
        "No persistence; native window bounds are persisted by the host service.",
        "CSS app-region: drag and navigator user-agent platform branch.",
        "Window dragging and platform control spacing are functional native-shell behavior.",
    ),
    "src/renderer/src/components/WhyTryCloudModal.vue": contract(
        "Cloud-benefits choice modal",
        "COMFY-DESKTOP-037;COMFY-DESKTOP-162;COMFY-DESKTOP-165;COMFY-DESKTOP-176",
        "A first-use user deciding between local and Comfy Cloud.",
        "The modal is shown and the user closes, continues locally, tries cloud, clicks the backdrop, or presses Escape.",
        "Localized title/benefit array and cloud/local action labels are available.",
        "The benefit list comes from i18n messages; backdrop dismiss requires both pointer down and click on the overlay.",
        "Close and continue-local both emit close; try-cloud emits its distinct action; benefit text and both choices remain visible before any account operation.",
        "role=dialog, aria-modal, localized aria-label, focusable container, labeled close, Escape, and native buttons are declared; focus capture/restore is not implemented locally.",
        "Only backdrop gesture tracking is local; cloud authentication/capacity state belongs to the owner.",
        "Dismissal performs no cloud mutation; try-cloud failure must return to an actionable first-use state through the owner.",
        "No direct persistence; the onboarding owner records the selected path only after the emitted action succeeds.",
        "Vue close/try-cloud emits and localized benefit messages; cloud/auth effects are external.",
        "This is the explicit cloud/paid decision boundary and cannot be omitted from the renderer inventory.",
        availability="cloud/paid",
    ),
    "src/renderer/src/components/icons/ComfyCLogo.vue": contract(
        "Decorative Comfy C logo primitive",
        "COMFY-DESKTOP-095",
        "A renderer composing branded status or empty-state chrome.",
        "The parent renders the icon with an optional size.",
        "The consuming surface supplies any semantic label because the SVG is deliberately decorative.",
        "size defaults to 24 and accepts number or string.",
        "The SVG renders at the requested size and emits no action.",
        "aria-hidden=true makes the glyph decorative; the consuming surface must provide the accessible label.",
        "Stateless and synchronous.",
        "Invalid styling affects only presentation and cannot invoke an operation.",
        "No persistence.",
        "Inline SVG only.",
        "Explicit presentational/infrastructure classification: the glyph has no independently invokable capability.",
        availability="infrastructure-only",
        classification="presentational/infrastructure-only",
    ),
    "src/renderer/src/components/settings/ComfyUISettingsContent.vue": contract(
        "ComfyUI instance settings tab shell, navigation, operations, and primary actions",
        "COMFY-DESKTOP-024;COMFY-DESKTOP-030;COMFY-DESKTOP-067;COMFY-DESKTOP-084;COMFY-DESKTOP-149;COMFY-DESKTOP-165;COMFY-DESKTOP-181;COMFY-DESKTOP-185",
        "A panel or title-popup user managing one Comfy installation.",
        "The installation/operation/settings snapshot changes or the user navigates tabs/subpages, edits fields, runs actions, opens paths/windows, or controls an operation.",
        "Installation, host mode, refresh/operation/action props, and useComfyUISettings/useInstanceNavState services determine tabs, sections, action decisions, and update behavior.",
        "Settings/details are refreshed per install/channel with a ten-second main-side staleness guard; operation states distinguish inflight, success, error, and cancellation.",
        "The shell renders status/storage/update/snapshots/console/section tabs, args subpage, progress overlay, notices, capacity block, primary/split/more actions, and emits every service request with the selected install/action/field.",
        "role=tablist/tab, aria-selected, roving tabindex, arrow/Home/End handling, progressbar values, status/live regions, disabled-state explanations, and focus restoration for menus are source-defined.",
        "Watchers reject stale install/action transitions, preserve explicit local tab state, prevent navigation during blocking operations, and auto-dismiss successful operations after the source countdown.",
        "No installation shows empty state; load errors remain visible; cloud disabled blocks primary/window actions; cancellation/retry/dismiss remain distinct; failed field updates surface owner-provided errors.",
        "Field edits persist through useComfyUISettings; nav/tab/menu/progress state is transient, while installation settings and pending-restart state survive through host storage.",
        "window.api.openPath plus useComfyUISettings/useInstanceNavState/useCloudCapacity; emits show-progress, update installation, navigation/close/dismiss, primary/open-window, field/action, operation cancel/retry/dismiss.",
        "This is the concrete cross-host settings workflow, not merely a registry projection.",
    ),
    "src/renderer/src/components/ui/BaseModal.vue": contract(
        "Accessible modal primitive with focus and scroll ownership",
        "COMFY-DESKTOP-176;COMFY-DESKTOP-181",
        "A renderer opening a modal and its keyboard, pointer, or assistive-technology user.",
        "open transitions true/false, the close button is activated, Escape is handled by useModalOverlay, or a valid outside gesture completes.",
        "open is required; size defaults md, Escape/outside/close-button/scroll-prevention default true, blur false, and one accessible-name prop is required by contract.",
        "Opening captures prior focus, locks body scroll, and focuses the dialog; closing/unmount restores the prior overflow/focus where possible.",
        "The teleported modal emits close only through enabled dismissal paths and presents header/body/footer slots in a size-constrained panel.",
        "role=dialog, aria-modal, aria-label/labelledby, tabindex=-1, labeled close, focus capture/restore, and focus-visible styling are declared; missing name warns in development.",
        "An immediate watch owns open/close transitions; unmount defensively releases scroll/focus; outside dismissal rechecks current props.",
        "Disabled Escape/outside paths do nothing, removed return-focus targets are safely ignored, and preexisting body overflow is restored exactly.",
        "No durable state; it temporarily mutates document.body.style.overflow and focus.",
        "Teleport, Transition, useModalOverlay, body scroll mutation, and Vue close emit.",
        "This primitive defines modal accessibility and cleanup behavior consumed by multiple functional surfaces.",
    ),
    "src/renderer/src/views/ManageInstallModal.vue": contract(
        "Manage-instance modal adapter",
        "COMFY-DESKTOP-024;COMFY-DESKTOP-076;COMFY-DESKTOP-149;COMFY-DESKTOP-176",
        "A user opening instance management from the dashboard or context action.",
        "installation changes from null to a record, or child settings requests close/progress/navigation/update.",
        "installation may be null; active operation and autoActionKey default null; sectionsRefreshSeq defaults zero.",
        "open is exactly installation != null and the child receives host=panel plus operation/refresh/action props.",
        "BaseModal opens with a localized accessible name and delegates the full settings surface while forwarding close, progress, list navigation, and installation updates.",
        "BaseModal provides dialog/focus/scroll behavior; child settings provides keyboard tab/menu behavior.",
        "The parent owns installation identity and operation lifetime; null closes the modal without retaining a stale child.",
        "A null installation renders no settings body; child errors/progress remain visible inside the modal and closing does not silently cancel owner work.",
        "Installation updates are emitted for owner persistence; modal open state is derived and transient.",
        "BaseModal, ComfyUISettingsContent, and close/show-progress/navigate-list/update:installation emits.",
        "This adapter defines the panel-hosted management lifecycle and close boundary.",
    ),
    "src/renderer/src/views/comfyUISettings/ArgsBuilderField.vue": contract(
        "Inline Comfy launch-argument field with schema-backed suggestions",
        "COMFY-DESKTOP-030;COMFY-DESKTOP-149",
        "A user editing ComfyUI launch arguments from a settings section.",
        "The installation changes, the field gains input/change, or the configure button is activated.",
        "DetailField and optional installationId are supplied; getComfyArgs returns the recognized Comfy argument schema.",
        "Local text mirrors the field unless actively changed; absent installation id/schema leaves suggestions empty without blocking raw editing.",
        "Schema loading enables inline autocomplete/validation, input updates local text, change emits the exact field/value, and configure emits open.",
        "The text input and configure button have labels; ArgsRawInput owns suggestion-list keyboard semantics.",
        "Installation changes launch async schema load; source catches load failure and clears schema, so a late-result ordering check remains required.",
        "Schema lookup failure preserves raw argument editing; field changes are not persisted until the owner accepts the update emit.",
        "The owner persists the emitted argument string; schema and local edit state are transient.",
        "window.api.getComfyArgs and Vue open/update(field,value) emits.",
        "Schema-unavailable fallback and exact raw-value preservation are argument compatibility contracts.",
    ),
    "src/renderer/src/views/comfyUISettings/ArgsRawInput.vue": contract(
        "Raw Comfy argument input, token validation, and keyboard autocomplete",
        "COMFY-DESKTOP-030;COMFY-DESKTOP-181",
        "A keyboard or pointer user editing a raw ComfyUI argument string.",
        "The user focuses/types/navigates suggestions/accepts a match/blurs, or modelValue/schema changes.",
        "modelValue, optional schema, label, disabled, and validation context feed useArgsAutocomplete.",
        "Local value tracks external modelValue outside active edits; boolean suggestions omit value metadata.",
        "Typing emits update:modelValue, committed change emits change, and autocomplete inserts the selected argument while validation lists awaiting, unsupported, missing-value, and orphaned tokens.",
        "The suggestion popup uses role=listbox/option and aria-selected; keyboard handling delegates Arrow/Enter/Escape/Tab semantics to useArgsAutocomplete; validation uses role=status.",
        "Focus gates popup visibility and local/external synchronization; selection and validation derive from the latest schema/value.",
        "Absent schema leaves raw editing available; invalid tokens remain visible and round-trippable rather than being discarded; disabled input cannot change.",
        "No direct persistence; owner persists emitted raw string.",
        "useArgsAutocomplete and Vue update:modelValue/change emits.",
        "Lossless invalid-token editing and keyboard suggestion behavior affect CLI compatibility.",
    ),
    "src/renderer/src/views/comfyUISettings/BooleanToggle.vue": contract(
        "Optimistic boolean settings switch",
        "COMFY-DESKTOP-149;COMFY-DESKTOP-181",
        "A settings user toggling a boolean field.",
        "The switch is clicked or its field value changes from the owner.",
        "A DetailField provides label and value; true is on and every other value is off.",
        "visualOn initially reflects field.value and resynchronizes when the owner changes the value.",
        "Activation flips visualOn immediately and emits the new boolean exactly once.",
        "Native button with role=switch, aria-checked, and field label supports keyboard activation and exposes state without color alone.",
        "Optimistic local state is reconciled by a watcher when persistence success/failure updates the prop.",
        "A rejected write must return the owner prop to the prior value, causing the visual state to revert; no error is swallowed locally.",
        "No direct write; owner persists the emitted boolean.",
        "Vue update(boolean) emit.",
        "Optimistic toggle and owner-driven rollback are independently testable settings behavior.",
    ),
    "src/renderer/src/views/comfyUISettings/ChannelPicker.vue": contract(
        "ComfyUI release-channel preview and action surface",
        "COMFY-DESKTOP-015;COMFY-DESKTOP-109;COMFY-DESKTOP-115;COMFY-DESKTOP-116;COMFY-DESKTOP-117;COMFY-DESKTOP-119;COMFY-DESKTOP-149;COMFY-DESKTOP-185",
        "A user choosing and applying a ComfyUI update channel.",
        "The draft selection, preview/enrichment data, current version, update actions, or running-action set changes.",
        "DetailField options plus preview/actions/current status and optional running ids are supplied; source defaults optional action state safely.",
        "Draft starts at the current field value; a distinct draft shows switch guidance; enrichment timeout reveals fallback guidance instead of an endless spinner.",
        "The surface compares current/target versions, reports channel/update status, renders option descriptions/statistics, and emits exact selected field updates or enabled actions.",
        "Select control is labeled; busy/enrichment state uses role=status/aria-live; native action buttons expose disabled and spinner state.",
        "Draft, preview, enrichment timeout, and action-running state are independent; external field changes resynchronize the draft.",
        "Disabled/running actions cannot fire, missing preview remains an explicit pending/empty state, and enrichment exceptions/timeouts do not invent a version match.",
        "Channel choice/action effects persist through the owner; preview/timer state is transient.",
        "Vue update-field/action emits and host-supplied release metadata/actions.",
        "Version comparison, preview latency, action gating, and rollback are channel compatibility behavior.",
        availability="conditional",
    ),
    "src/renderer/src/views/comfyUISettings/EnvVarsField.vue": contract(
        "Environment-variable row editor with secret masking and duplicate validation",
        "COMFY-DESKTOP-031;COMFY-DESKTOP-073;COMFY-DESKTOP-149;COMFY-DESKTOP-181",
        "A settings user adding, editing, revealing, or removing launch environment variables.",
        "The field value changes or the user edits rows, adds/removes a row, or toggles a sensitive value reveal.",
        "DetailField value is normalized into key/value rows; sensitivity is inferred from token, secret, key, password, credential, auth, cookie, and private patterns.",
        "New rows are empty and receive focus; sensitive nonempty values are masked until individually revealed.",
        "Nonblank unique rows emit a normalized object, duplicate keys render role=alert and block silent ambiguity, and removing/editing updates the owner payload.",
        "Inputs have localized labels and duplicate aria-invalid; reveal buttons expose aria-pressed and reveal/hide labels; remove/add are labeled buttons.",
        "Row refs focus the added row after nextTick; reveal state and local rows reconcile when the owner field changes.",
        "Duplicate keys remain visibly invalid, blank rows are not serialized as variables, and rejected persistence must restore owner state without leaking masked values.",
        "Owner persists the emitted environment map; reveal state is never persisted and must not enter logs/telemetry.",
        "Vue update(field,object) emit; no direct process mutation.",
        "Secret masking, duplicate handling, and serialization boundaries are independently testable security behavior.",
    ),
    "src/renderer/src/views/comfyUISettings/ExtraModelPathsModal.vue": contract(
        "Read-only extra_model_paths inspection modal",
        "COMFY-DESKTOP-032;COMFY-DESKTOP-149;COMFY-DESKTOP-186;COMFY-DESKTOP-187",
        "A user inspecting custom model search-path YAML resolution.",
        "The modal opens, the user refreshes, opens a resolved base/directory, reveals YAML, or closes.",
        "open, yamlPath, and parsed section/path status data are supplied by the owner.",
        "Missing YAML disables reveal; default sections and path status retain their source tags/details.",
        "The modal lists each section/base/path and emits exact refresh/open-path/reveal-path/close actions without editing the YAML.",
        "BaseModal supplies dialog/focus semantics; all path actions are labeled buttons and disabled reveal is explicit.",
        "Snapshot data is owner-controlled; refresh may replace the list while the modal remains open.",
        "Missing/unreadable YAML remains inspectable as an empty/error owner state; disabled reveal cannot invoke a filesystem action.",
        "Read-only surface; no direct persistence or YAML rewrite.",
        "Vue close/refresh/open-path/reveal-path emits and BaseModal.",
        "Resolved-path inspection and no-edit boundary are extension/model-path compatibility behavior.",
        availability="conditional",
    ),
    "src/renderer/src/views/comfyUISettings/MoreMenu.vue": contract(
        "Settings action menu with roving keyboard focus",
        "COMFY-DESKTOP-149;COMFY-DESKTOP-178;COMFY-DESKTOP-181",
        "A settings user choosing an overflow or split-button action.",
        "open becomes true, then the user presses arrows/Home/End/Escape/Enter/Space, clicks an item, or clicks outside.",
        "open, ordered ActionDef actions, optional heading, and anchor are supplied by the owner.",
        "focusedIndex resets to the first enabled item when opened; icon-column layout appears if any action has an icon.",
        "Enabled activation emits pick(action) then close; disabled actions remain visible but cannot activate; outside/Escape emits close.",
        "role=menu/menuitem, vertical orientation, roving tabindex, wraparound arrows, Home/End, Enter/Space, Escape, initial focus, and disabled semantics are declared.",
        "Only the current open instance owns document click/keydown listeners; nextTick focuses the chosen initial item and unmount removes listeners.",
        "No actions or closed state renders nothing; all-disabled input retains a non-invokable visible menu and closes safely.",
        "No persistence; picked action owns any effect.",
        "Document click/keydown listeners and Vue pick(action)/close emits.",
        "This component is the exact keyboard/focus contract for settings overflow menus.",
    ),
    "src/renderer/src/views/comfyUISettings/PathField.vue": contract(
        "Editable or browse-only path settings field",
        "COMFY-DESKTOP-032;COMFY-DESKTOP-033;COMFY-DESKTOP-149;COMFY-DESKTOP-186;COMFY-DESKTOP-187",
        "A user editing, browsing, or opening a configured directory.",
        "The text commits, Browse returns a directory, or the open-path affordance is activated.",
        "DetailField supplies value/label/browseOnly; null values normalize to an empty string.",
        "browseOnly suppresses free text; chooser cancellation or choosing the existing value produces no update.",
        "Browse starts at the current path, accepted choice emits update(field,path), text change emits update, and open reveals a nonempty current path.",
        "Text input and browse control use the field/localized labels; browse-only path uses the shared open-path presentation.",
        "The field is a projection of owner value; async chooser completion must be reconciled by the owner if the installation changes mid-dialog.",
        "Cancelled chooser and empty open are no-ops; bridge rejection remains a visible settings/filesystem error through the owner.",
        "No direct persistence; owner stores emitted value.",
        "window.api.browseFolder/openPath and Vue update(field,value) emit.",
        "Chooser default, browse-only behavior, cancellation, and exact path propagation are independently testable.",
    ),
    "src/renderer/src/views/comfyUISettings/SettingsSectionList.vue": contract(
        "Schema-driven settings sections, typed fields, actions, restart tags, and errors",
        "COMFY-DESKTOP-030;COMFY-DESKTOP-031;COMFY-DESKTOP-032;COMFY-DESKTOP-033;COMFY-DESKTOP-149;COMFY-DESKTOP-181;COMFY-DESKTOP-185",
        "A user reading or editing a host-provided settings section registry.",
        "Sections/fields/actions change or the user collapses, edits, opens a path/args page, or runs an action.",
        "DetailSection records, readonly state, optional running ids, pending-restart ids, and field-error map drive rendering.",
        "Collapsed sections preserve title identity across updates; editType dispatch selects env-vars, boolean, path, select, args-builder, channel-cards, text, number, or readonly rendering.",
        "Every visible field/action emits its exact typed field/value/action; restart/error/description/disabled-message states render next to the affected control; numeric empty commits null.",
        "Collapsible headings are buttons with aria-expanded; fields/actions are labeled; errors use role=alert, restart uses role=status, security descriptions role=note, and disabled-with-message remains explainable without execution.",
        "Local collapsed-title set is reconciled with incoming sections; running ids prevent duplicate action invocation and owner updates remain authoritative.",
        "Invalid/failed fields retain error text and prior effective value; readonly mode cannot emit edits; disabled actions cannot run unless the source intentionally leaves them clickable only to explain disabledMessage.",
        "No direct persistence; update-field/run-action requests go to the owner and pending-restart/error state is supplied back.",
        "Vue update-field/run-action/open-path/open-args-page emits and specialized field components.",
        "This is the concrete renderer mapping from the Desktop settings schema to observable controls and errors.",
    ),
    "src/renderer/src/views/comfyUISettings/SnapshotRow.vue": contract(
        "Snapshot history row with change pills and expansion control",
        "COMFY-DESKTOP-123;COMFY-DESKTOP-124;COMFY-DESKTOP-149;COMFY-DESKTOP-181",
        "A user browsing instance snapshot history.",
        "The user activates the row toggle or summary/expanded/latest metadata changes.",
        "SnapshotSummary is required; expanded, latest, and context labels default through props.",
        "Trigger/date/version/change deltas and manual labels derive from the snapshot; zero deltas omit their pill.",
        "The row emits toggle once, exposes exact added/removed/changed node/package counts, and distinguishes latest/current/manual/ComfyUI changes.",
        "The toggle is a native button with aria-expanded; text and signs convey deltas independently of color.",
        "Stateless projection; expansion is owner-controlled through the expanded prop.",
        "Absent deltas do not show misleading zero pills; missing optional labels use snapshot formatting fallbacks.",
        "No direct persistence; history and expanded selection are owner state.",
        "Vue toggle emit and snapshot formatting helpers.",
        "Row-level delta and expansion semantics are independently testable snapshot behavior.",
        availability="conditional",
    ),
    "src/renderer/src/views/comfyUISettings/StatusFactPanel.vue": contract(
        "Instance identity, remote URL, status facts, paths, copy, and inline commit surface",
        "COMFY-DESKTOP-024;COMFY-DESKTOP-055;COMFY-DESKTOP-149;COMFY-DESKTOP-181;COMFY-DESKTOP-187",
        "A user inspecting or renaming an instance, editing a remote URL, opening/copying facts, or viewing session state.",
        "Installation/section/session facts change or contenteditable name/URL blurs after keyboard/paste editing.",
        "Installation, detail sections, and async onRename/onUpdateUrl callbacks are supplied; cloud names are read-only and only remote URLs are editable.",
        "Name/URL DOM text is synchronized from owner state; Enter commits by blur, Escape restores, Ctrl/Cmd+A selects all, and name paste is normalized to plain single-line text.",
        "Successful callbacks retain the edited value; false/rejection reverts to owner text; valid remote URL changes show restart-needed status; path/date/copy facts use source-specific formatting.",
        "Editable values use role=textbox with labels and explicit edit buttons; keyboard select/commit/cancel and copy controls are available; bdi preserves path direction.",
        "Blur awaits the commit callback before deciding retain/revert; watchers avoid clobbering an active edit with stale display text.",
        "Invalid non-http(s) connection URLs do not commit; callback false/exception reverts; cloud name editing is absent; empty facts do not create open/copy actions.",
        "Owner persists accepted name/URL; inline edit and restart badge are transient projections.",
        "Async onRename/onUpdateUrl props, window.api.openPath, sessionStore, BaseCopyButton.",
        "Commit/revert, URL validation, keyboard editing, and cloud/remote differences are critical settings contracts.",
    ),
    "src/renderer/src/views/comfyUISettings/StorageDirRow.vue": contract(
        "Storage directory open, browse, reset, and shared/tag row",
        "COMFY-DESKTOP-027;COMFY-DESKTOP-028;COMFY-DESKTOP-032;COMFY-DESKTOP-033;COMFY-DESKTOP-149;COMFY-DESKTOP-181;COMFY-DESKTOP-187",
        "A settings user managing one displayed storage directory.",
        "The user opens, browses, or resets the row.",
        "path is required; optional label, tag, shared, resettable, and browse visibility/style props configure affordances.",
        "Reset is absent unless resettable and browse visibility follows the supplied row mode.",
        "The exact open/browse/reset action emits once while label/path/shared/tag state remains visible.",
        "All actions are native buttons with localized labels; the path button remains the open target and icon-only controls carry aria-label.",
        "Stateless; owner updates the path after an emitted action.",
        "Unavailable reset/browse cannot be invoked and empty/error path handling belongs to the owner.",
        "No direct persistence.",
        "Vue open/browse/reset emits.",
        "This reusable row defines the concrete action visibility and labeling for storage paths.",
    ),
    "src/renderer/src/views/comfyUISettings/StoragePane.vue": contract(
        "Per-instance/shared model, input, and output storage configuration",
        "COMFY-DESKTOP-027;COMFY-DESKTOP-028;COMFY-DESKTOP-032;COMFY-DESKTOP-033;COMFY-DESKTOP-147;COMFY-DESKTOP-149;COMFY-DESKTOP-152;COMFY-DESKTOP-186;COMFY-DESKTOP-187",
        "A user selecting shared versus isolated storage and managing ordered model/media directories.",
        "Storage snapshot/fields change or the user toggles sharing, opens/browses/resets paths, manages model roots, inspects extra paths, or refreshes.",
        "Installation, storage snapshot, settings sections, title-popup bridge compatibility, and update/refresh emits are required.",
        "useSharedModels and useSharedInputOutput default on when absent; effective instance input/output paths fall back to defaults; the restart warning appears after relevant global/local changes.",
        "The pane switches between shared and instance-specific roots, performs confirmed list removal, preserves ordered primary roots, edits/resets media paths, opens/reveals paths, and exposes extra-model-path details.",
        "BooleanToggle, ModelsDirList, StorageDirRow, status note, and ExtraModelPathsModal provide labeled keyboard controls; list menus retain arrow/Escape focus behavior.",
        "Owner snapshots are authoritative; async choosers and writes use the current path/index, globalTouched tracks restart warning, and refresh replaces parsed extra-path state.",
        "Cancelled/unchanged chooser, invalid index, denied removal, or absent field is a no-op; write failure must preserve prior effective paths and remain visible; removal never deletes files.",
        "update-field persists sharing/media fields; title-popup bridge persists shared model roots; local disclosure/modal/restart-note state is transient.",
        "window.api or title-popup global-settings bridge browse/open/update/set-model-dirs methods; Vue update-field/refresh emits; useModal confirmation.",
        "Sharing defaults, fallback resolution, ordered roots, immediate persistence, and cross-host bridge behavior are storage compatibility contracts.",
    ),
}


DIRECT_TESTS = {
    "src/renderer/src/comfyTitlePopup/TitlePopupApp.vue",
    "src/renderer/src/components/InstallNamePath.vue",
    "src/renderer/src/components/MigrationBanner.vue",
    "src/renderer/src/components/settings/ComfyUISettingsContent.vue",
    "src/renderer/src/components/ui/BaseModal.vue",
    "src/renderer/src/views/comfyUISettings/ArgsBuilderField.vue",
    "src/renderer/src/views/comfyUISettings/PathField.vue",
    "src/renderer/src/views/comfyUISettings/SettingsSectionList.vue",
    "src/renderer/src/views/comfyUISettings/StatusFactPanel.vue",
    "src/renderer/src/views/comfyUISettings/StoragePane.vue",
}


FIELDS = [
    "feature_id",
    "product",
    "domain",
    "component_surface",
    "parent_feature_ids",
    "classification",
    "availability",
    "evidence_level",
    "confidence",
    "source_file",
    "source_symbol",
    "props",
    "emits",
    "handlers",
    "test_evidence",
    "actor",
    "trigger",
    "preconditions",
    "inputs_defaults",
    "observable_success",
    "interaction_accessibility",
    "state_concurrency",
    "failure_recovery",
    "persistence_serialization",
    "interfaces_side_effects",
    "disposition_reason",
    "sim_status",
    "sim_evidence",
    "parity_gap",
    "observable_sim_acceptance",
    "automated_validation",
    "manual_validation",
    "open_questions",
]


def compact(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip()


def stable_id(source_file: str) -> str:
    digest = hashlib.sha256(source_file.encode("utf-8")).hexdigest()[:12].upper()
    return f"COMFY-DESKTOP-RENDERER-{digest}"


def braced_body(text: str, marker: str) -> tuple[str, int] | None:
    match = re.search(marker, text)
    if not match:
        return None
    opening = text.find("{", match.start())
    if opening < 0:
        return None
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                line = text.count("\n", 0, opening) + 1
                return compact(text[opening + 1 : index]), line
    return None


def braced_body_at(text: str, opening: int) -> str | None:
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return compact(text[opening + 1 : index])
    return None


def prop_contract(text: str) -> str:
    body = braced_body(text, r"(?:export\s+)?interface\s+Props\s*\{")
    if body is None:
        body = braced_body(text, r"defineProps\s*<\s*\{")
    if body is None:
        return "No defineProps contract; this root/host consumes bridge or composable state."
    value, line = body
    defaults = []
    with_defaults = re.search(r"withDefaults\s*\(\s*defineProps", text)
    if with_defaults:
        tail = text[with_defaults.start() :]
        default_start = re.search(r">\s*\(\s*\)\s*,\s*\{", tail)
        if default_start:
            opening = with_defaults.start() + default_start.end() - 1
            default_body = braced_body_at(text, opening)
            if default_body is not None:
                defaults.append(default_body)
    suffix = f" Defaults: {defaults[0]}" if defaults else " Defaults: none declared here; optional props are undefined."
    return f"L{line} exact type body: {value}.{suffix}"


def emit_contract(text: str) -> str:
    body = braced_body(text, r"defineEmits\s*<\s*\{")
    if body is None:
        return "No defineEmits contract; actions use an injected bridge/composable or the component is presentational."
    value, line = body
    return f"L{line} exact emit type body: {value}."


def handler_contract(text: str) -> str:
    handlers = []
    for match in re.finditer(r"^(?:async\s+)?function\s+([A-Za-z_$][\w$]*)\s*\(", text, re.M):
        line = text.count("\n", 0, match.start()) + 1
        handlers.append(f"{match.group(1)}@L{line}")
    for hook in ("watch", "onMounted", "onUnmounted", "onBeforeUnmount"):
        for match in re.finditer(rf"\b{hook}\s*\(", text):
            line = text.count("\n", 0, match.start()) + 1
            handlers.append(f"{hook}@L{line}")
    unique = list(dict.fromkeys(handlers))
    if not unique:
        return "No locally declared named handler; template events delegate directly to typed emits or composable bindings."
    return "; ".join(unique)


def source_symbol(text: str, component: str) -> str:
    template = text.find("<template")
    template_line = text.count("\n", 0, template) + 1 if template >= 0 else 0
    return f"Vue SFC {component}; <script setup> starts L1; <template> starts L{template_line}."


def uncovered_vue_paths() -> list[str]:
    coverage_path = CATALOGS / "desktop-source-coverage.csv"
    with coverage_path.open(newline="", encoding="utf-8") as handle:
        coverage = list(csv.DictReader(handle))

    direct_evidence = []
    for path in sorted(CATALOGS.glob("desktop-*.csv")):
        if path.name in {"desktop-source-coverage.csv", OUTPUT_NAME}:
            continue
        direct_evidence.append(path.read_text(encoding="utf-8", errors="replace"))
    haystack = "\n".join(direct_evidence)
    return sorted(
        row["path"]
        for row in coverage
        if row["classification"] == "production"
        and row["path"].endswith(".vue")
        and row["path"] not in haystack
    )


def write_catalog(paths: list[str]) -> list[dict[str, str]]:
    rows = []
    for source_file in paths:
        relative = source_file.removeprefix(DESKTOP_PREFIX)
        metadata = SURFACES[relative]
        source_path = REPO_ROOT / source_file
        text = source_path.read_text(encoding="utf-8", errors="replace")
        component = source_path.stem
        feature_id = stable_id(source_file)
        test_path = source_path.with_name(f"{component}.test.ts")
        direct_test = relative in DIRECT_TESTS
        infrastructure = metadata["classification"] == "presentational/infrastructure-only"
        if direct_test and not test_path.exists():
            raise RuntimeError(f"Expected focused renderer test is missing: {test_path}")
        rows.append(
            {
                "feature_id": feature_id,
                "product": "Comfy-Desktop",
                "domain": "desktop-renderer-surface",
                **metadata,
                "evidence_level": "test-backed" if direct_test else "code-inferred",
                "confidence": "high" if direct_test else "medium",
                "source_file": source_file,
                "source_symbol": source_symbol(text, component),
                "props": prop_contract(text),
                "emits": emit_contract(text),
                "handlers": handler_contract(text),
                "test_evidence": (
                    str(test_path.relative_to(REPO_ROOT))
                    if direct_test
                    else "No focused existing test directly mounts this component; source contract is code-inferred."
                ),
                "sim_status": "deferred" if infrastructure else "missing",
                "sim_evidence": (
                    "No standalone Sim component is required; this presentational/infrastructure contract is validated through each consuming Comfy-specific GPUI surface."
                    if infrastructure
                    else "No Comfy-specific Desktop renderer surface exists in the inspected Sim target."
                ),
                "parity_gap": (
                    f"This source-specific presentational contract is not separately mapped in Sim; inventory and validate its semantic/decorative effect through every consuming Comfy GPUI surface: {metadata['disposition_reason']}"
                    if infrastructure
                    else f"Sim has no GPUI surface that reproduces the {metadata['component_surface']} contract, including its state, error, focus, and service-boundary behavior."
                ),
                "observable_sim_acceptance": (
                    f"Every consuming Sim Comfy surface shall preserve this noninteractive render contract without creating a standalone workflow requirement: {metadata['observable_success']}"
                    if infrastructure
                    else f"With deterministic props/service fixtures, the Sim GPUI surface shall reproduce this observable contract: {metadata['observable_success']} It shall also preserve the documented failure/recovery and provide a keyboard-accessible path for every action."
                ),
                "automated_validation": (
                    f"Parameterize consuming GPUI visual/accessibility tests with {feature_id}; verify the semantic grouping or decorative-hidden effect without a standalone action surface."
                    if infrastructure
                    else f"Add a deterministic GPUI interaction test parameterized by {feature_id}; assert success, empty/loading/error/cancel/retry states that apply, focus/keyboard behavior, emitted service requests, and no forbidden side effects."
                ),
                "manual_validation": f"Exercise {metadata['component_surface']} side by side in Comfy-Desktop and Sim; compare localized text/state, pointer and keyboard actions, focus, cancellation, retry, destructive confirmation, and visible errors.",
                "open_questions": "Electron renderer was not launched because dependencies are absent; native focus, screen-reader announcement, packaged bridge timing, and platform window-manager behavior remain runtime-unverified.",
            }
        )

    with (CATALOGS / OUTPUT_NAME).open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    return rows


def update_source_coverage(rows: list[dict[str, str]]) -> None:
    ids_by_path = {row["source_file"]: row["feature_id"] for row in rows}
    path = CATALOGS / "desktop-source-coverage.csv"
    with path.open(newline="", encoding="utf-8") as handle:
        coverage = list(csv.DictReader(handle))
        fields = list(coverage[0].keys())
    for row in coverage:
        feature_id = ids_by_path.get(row["path"])
        if feature_id is None:
            continue
        existing = [value for value in row["feature_ids"].split(";") if value]
        row["feature_ids"] = ";".join(dict.fromkeys([*existing, feature_id]))
        row["coverage_disposition"] = "mapped"
        row["reason"] = (
            f"Mapped to source-specific renderer contract {feature_id}; coarse parent capability IDs are retained for reverse traceability."
        )
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(coverage)


def main() -> None:
    paths = uncovered_vue_paths()
    expected = sorted(DESKTOP_PREFIX + relative for relative in SURFACES)
    if paths != expected:
        missing_metadata = sorted(set(paths) - set(expected))
        stale_metadata = sorted(set(expected) - set(paths))
        raise RuntimeError(
            "Desktop renderer candidate mismatch: "
            + json.dumps(
                {"discovered": len(paths), "expected": len(expected), "missing_metadata": missing_metadata, "stale_metadata": stale_metadata},
                indent=2,
            )
        )
    if len(paths) != 43:
        raise RuntimeError(f"Expected 43 uncovered production Vue files, found {len(paths)}")
    rows = write_catalog(paths)
    if len({row["feature_id"] for row in rows}) != len(rows):
        raise RuntimeError("Renderer surface stable-id collision")
    update_source_coverage(rows)
    counts = {
        "rows": len(rows),
        "functional": sum(row["classification"] == "functional" for row in rows),
        "presentational_infrastructure": sum(row["classification"] == "presentational/infrastructure-only" for row in rows),
        "test_backed": sum(row["evidence_level"] == "test-backed" for row in rows),
        "code_inferred": sum(row["evidence_level"] == "code-inferred" for row in rows),
    }
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
