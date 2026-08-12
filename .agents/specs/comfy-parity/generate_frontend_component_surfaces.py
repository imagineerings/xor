#!/usr/bin/env python3

from __future__ import annotations

import csv
import hashlib
import re
from collections import defaultdict
from pathlib import Path


SPEC_ROOT = Path(__file__).resolve().parent
CATALOGS = SPEC_ROOT / "catalogs"
REPO_ROOT = SPEC_ROOT.parents[2]
FRONTEND_ROOT = REPO_ROOT / "projects/comfy/ComfyUI-Frontend"

REFERENCE_CATALOGS = (
    "frontend-menus.csv",
    "frontend-persisted-state.csv",
    "frontend-telemetry.csv",
    "frontend-http-usage.csv",
    "frontend-routes.csv",
    "frontend-commands.csv",
    "frontend-settings.csv",
    "frontend-extensions.csv",
    "frontend-formats-migrations.csv",
    "frontend-feature-flags.csv",
    "frontend-keybindings.csv",
    "frontend-websocket.csv",
)

FIELDS = (
    "feature_id",
    "selection_basis",
    "source_file",
    "source_distribution_classification",
    "primary_coverage_anchor",
    "product",
    "domain",
    "surface",
    "name",
    "classification",
    "availability",
    "evidence_level",
    "confidence",
    "source_evidence",
    "source_symbol",
    "source_excerpt",
    "test_evidence",
    "actor",
    "trigger",
    "preconditions",
    "inputs_defaults",
    "observable_success",
    "state_transitions_concurrency",
    "failure_recovery",
    "interaction_accessibility",
    "persistence_serialization",
    "interfaces_side_effects",
    "platform_localization_variants",
    "feature_flags_permissions",
    "infrastructure_disposition_reason",
    "observable_sim_acceptance",
    "automated_validation",
    "manual_validation",
    "open_questions",
    "props",
    "models",
    "emits",
    "template_event_bindings",
    "handlers",
    "conditional_states",
)

REQUIRED_FUNCTIONAL_OVERRIDES = {
    "src/components/sidebar/tabs/AssetsSidebarTab.vue",
}


MANUAL_CONTRACTS = {
    "apps/desktop-ui/src/components/install/GpuPicker.vue": {
        "observable": "Choosing a platform-filtered Apple Metal, NVIDIA, AMD, CPU, or Manual Install option updates the required device model; Metal/NVIDIA/AMD selections show the recommended badge and the selected option's localized description.",
        "state": "Each HardwareOption click calls pickGpu with one TorchDeviceType and replaces device exactly once; the Darwin branch offers Metal while other platforms offer NVIDIA and AMD, and CPU/manual remain available in both branches.",
        "failure": "No asynchronous failure path exists in this component; an unknown/null selection shows neither recommended state nor a device description, while platform acquisition is delegated to the Desktop preload bridge.",
        "accessibility": "Selection is delegated to HardwareOption click semantics; Sim must expose the alternatives as a single keyboard-operable choice group with selected state, labels, recommendation, and description programmatically associated.",
        "interfaces": "Reads electronAPI().getPlatform() and writes the parent-owned device model; it performs no install or filesystem mutation.",
    },
    "apps/desktop-ui/src/components/install/InstallLocationPicker.vue": {
        "observable": "On mount the picker obtains the Desktop default install path, validates it, and chooses regional mirror defaults; editing or browsing revalidates the path and exposes error, existing-path, and non-default-drive warnings while migration and Python/PyPI/Torch mirror models remain editable.",
        "state": "Mount and every path update transition through Desktop validation; stale validation errors are cleared before the new result, mirror validation states update independently, and accordion panels reveal migration and mirror controls without discarding their models.",
        "failure": "Desktop path or chooser failures populate the localized pathError instead of advancing installation; invalid, existing, and non-default-drive results remain distinct visible states, and mirror validation remains independently reportable.",
        "accessibility": "The path field, folder chooser, accordion headers, migration controls, mirror controls, and error/warning messages must be keyboard reachable, labeled, and expose invalid state and status relationships.",
        "interfaces": "Calls Desktop getSystemPaths, validateInstallPath, and directory-chooser bridge methods; writes parent-owned install, migration, and mirror models but does not create directories itself.",
    },
    "apps/desktop-ui/src/components/install/MigrationPicker.vue": {
        "observable": "Editing or choosing a source directory validates it through the Desktop bridge; a valid source reveals individually toggleable migration items and continuously projects selected item IDs into the migrationItemIds model, while an absent/invalid source shows the optional or error state.",
        "state": "validateSource clears the prior error before awaiting validateComfyUISource; browsePath updates sourcePath only when a directory is returned, then validates it; watchEffect derives migrationItemIds from the currently selected MigrationItems.",
        "failure": "Bridge rejection logs diagnostic detail and shows localized validation or chooser failure text; cancelling the chooser leaves the previous source and selections unchanged.",
        "accessibility": "Each migration checkbox is associated with its label by input-id/for; the directory field, chooser, row toggles, validation message, and optional state require equivalent keyboard and announced semantics.",
        "interfaces": "Calls Desktop validateComfyUISource and showDirectoryPicker; writes sourcePath and migrationItemIds models without copying files at this stage.",
    },
    "apps/desktop-ui/src/components/maintenance/TaskListPanel.vue": {
        "observable": "The maintenance panel renders the selected filter's tasks as list items or cards according to display mode, and renders the localized no-tasks state when the filtered task collection is empty.",
        "state": "Changing the filter or layout replaces the rendered task projection without mutating task runners; each TaskListItem or TaskCard receives the same task runner and current loading state.",
        "failure": "Task error and loading states remain owned by each runner/item; an empty error filter is represented as an explicit empty result rather than a stale prior list.",
        "accessibility": "The list/card projection must retain task names, status, progress, errors, and per-task controls in reading and tab order; the empty state must be announced as text.",
        "interfaces": "Consumes the maintenance task filter and runner models; execution and terminal side effects are delegated to TaskListItem/TaskCard.",
    },
    "apps/desktop-ui/src/components/maintenance/TerminalOutputDrawer.vue": {
        "observable": "Opening the bottom drawer creates a read-only terminal, writes the default message and buffered log history, auto-sizes it, then streams subsequent Desktop log messages live; unmounting clears only the visible terminal reference while retaining the hidden buffer.",
        "state": "created copies buffer-to-terminal and disables stdin; every onLogMessage writes to the buffer and the terminal when mounted; unmounted sets the terminal reference to null so later logs remain buffered for the next open.",
        "failure": "Closing or unmounting does not cancel maintenance and does not lose buffered logs; bridge or terminal write failures have no component-local visible recovery and must be surfaced by the owning maintenance view in Sim.",
        "accessibility": "The drawer header and close control must be keyboard operable; terminal output is read-only and requires an accessible log transcript/status alternative rather than relying on the canvas terminal alone.",
        "interfaces": "Subscribes to electron.onLogMessage and writes xterm/terminal-buffer state; it sends no terminal input and starts no subprocess.",
    },
    "src/components/sidebar/tabs/AppsSidebarTab.vue": {
        "observable": "The Apps sidebar lists only workflows whose suffix is app.json; when none exist it shows mode-sensitive localized guidance and, outside App Mode, an action that switches the application mode to app.",
        "state": "isAppWorkflow is the list filter; the empty-state action calls setMode('app') exactly once, while the empty-state button is omitted when App Mode is already active.",
        "failure": "There is no component-local asynchronous failure path; workflow-load failures remain with BaseWorkflowsSidebarTab and mode-switch failure must preserve the prior mode with visible feedback in Sim.",
        "accessibility": "The filtered tree/search and empty action require keyboard operation, focus retention, localized accessible names, and a textual beta state; the omitted button must not leave a dead focus target.",
        "interfaces": "Reads workflow suffixes and useAppMode state; writes only the frontend application mode.",
    },
    "src/components/sidebar/tabs/AssetsSidebarTab.vue": {
        "observable": "The Assets sidebar switches generated/imported collections, filters and sorts media, toggles grid/list presentation, pages near the end, selects single or bulk assets, previews supported media, enters/exits job output stacks, copies a job ID, and exposes download, delete, add/open/export-workflow, and context actions with loading and empty states.",
        "state": "Tab/filter/sort/view changes recompute the projection; selection, lightbox index, context target, and job-folder state are independent; approach-end loads more results; bulk destructive actions refresh assets only after their action path completes.",
        "failure": "Initial/incremental loading uses skeletons, an empty filtered result uses localized no-results text, unavailable previews fall back to the file action, destructive actions respect their confirmation/error paths, and leaving a job stack restores the parent collection.",
        "accessibility": "Tabs, filter controls, list/grid items, selection bar, copy/back buttons, context menu, lightbox, bulk actions, loading state, and destructive confirmations require keyboard, focus, selected-state, labels, and announced result/error semantics.",
        "interfaces": "Reads assets/output APIs and queue ResultItem metadata; writes the Comfy.Assets.Sidebar.ViewMode browser key, clipboard on job-ID copy, downloads/files through asset actions, and emits assetSelected to its parent.",
        "persistence": "Persists list/grid view mode under Comfy.Assets.Sidebar.ViewMode; filters, selection, context target, lightbox, and folder view are session component state.",
    },
    "src/components/dialog/content/setting/AboutPanel.vue": {
        "observable": "The About panel renders store-provided version/resource badges as labelled external links and conditionally renders current system statistics when the system-stats store has data.",
        "state": "Badge rows track aboutPanelStore.badges and the SystemStatsPanel appears only while systemStatsStore.systemStats is present.",
        "failure": "Absent system statistics omits the stats panel without inventing values; external-link failure remains in the browser/host and must not mutate About state.",
        "accessibility": "Each badge is a native external link with title and visible label; Sim must preserve external-destination indication, keyboard focus, and semantic system-stat labels.",
        "interfaces": "Reads About and system-stat stores and opens badge URLs in a new noopener/noreferrer browsing context.",
    },
    "src/components/dialog/content/setting/UsageLogsTable.vue": {
        "observable": "The usage-log table loads workspace or legacy billing events, shows spinner/error/table states, lazily paginates seven rows at a time, formats event type/detail/time, exposes additional information through an accessible tooltip button, and refreshes from page one when the billing route changes.",
        "state": "Each load increments latestLoadToken; only the newest request may mutate events, pagination, error, or loading, while completion telemetry still checks superseded responses; page changes update the one-based API page before loading.",
        "failure": "Null legacy/workspace responses and thrown requests become localized visible errors; superseded responses are discarded; finally clears loading only for the winning token so an older request cannot hide a newer spinner.",
        "accessibility": "Spinner/error/table transitions need announced busy/error state; pagination and additional-info controls are keyboard operable, and the info button already carries the localized aria-label.",
        "interfaces": "Calls workspaceApi.getBillingEvents or customerEventService.getMyEvents and telemetry completion checking; no usage event is mutated from this table.",
    },
    "src/components/dialog/content/setting/UserPanel.vue": {
        "observable": "The User panel switches between signed-in identity/provider details and a sign-in action; email-auth users can open password update, signed-in users can sign out, and non-API-key users see the support address for account deletion.",
        "state": "isLoggedIn selects account versus login content, loading replaces sign-out controls with a spinner and drives sign-in loading, and provider/API-key state gates password and deletion guidance.",
        "failure": "Authentication and dialog failures are delegated to useCurrentUser/dialogService; Sim must retain the current session state, stop loading, and expose a recoverable localized error.",
        "accessibility": "Identity fields remain labelled text; sign-in, sign-out, password update, and mail link are keyboard reachable with visible focus and loading/disabled state, and the icon-only password button requires its localized tooltip/name.",
        "interfaces": "Calls useCurrentUser authentication handlers and dialogService.showUpdatePasswordDialog; the support link uses mailto:support@comfy.org.",
    },
    "src/components/helpcenter/HelpCenterPopups.vue": {
        "observable": "When Help Center is visible, the component teleports its popup and click-outside backdrop to the document body; it always projects release and What's New surfaces into the graph container, closes on menu/backdrop actions, and records What's New dismissal through useHelpCenter.",
        "state": "Visibility inserts/removes both popup and backdrop; sidebar location and compact mode determine left/right/small positioning; close and dismissal handlers update the shared Help Center state.",
        "failure": "A missing teleport target prevents the affected child from rendering and requires an owning-shell error/fallback; backdrop close is idempotent and must not dismiss unrelated release state.",
        "accessibility": "The popup requires dialog/menu semantics, focus entry/return, Escape and click-outside parity, and a labelled close path; the transparent backdrop cannot be the only keyboard close mechanism.",
        "interfaces": "Reads and writes useHelpCenter state and hosts ReleaseNotificationToast/WhatsNewPopup; it performs no network call directly.",
    },
    "src/components/sidebar/tabs/queue/ResultAudio.vue": {
        "observable": "An audio queue result opens an expanded WaveAudioPlayer using result.url with a 120-pixel, 80-bar waveform presentation and the player's playback controls.",
        "state": "Replacing the result prop replaces the player source; playback, seeking, duration, loading, and media errors remain owned by WaveAudioPlayer.",
        "failure": "Invalid/unavailable result URLs follow WaveAudioPlayer's load/error contract; the wrapper has no fallback or retry of its own.",
        "accessibility": "WaveAudioPlayer must expose keyboard playback/seeking, labelled current/duration state, and a text error; the wrapper adds no separate focus target.",
        "interfaces": "Passes the queue ResultItem URL to a browser audio player; media retrieval occurs through that URL.",
    },
    "src/components/sidebar/tabs/queue/ResultText.vue": {
        "observable": "A text queue result loads and displays its text with whitespace preserved in a bounded scrollable article, or shows the localized text-load failure when useTextFileContent reports an error.",
        "state": "useTextFileContent tracks the current result; hasError selects error text instead of stale content and successful text updates the article projection.",
        "failure": "Fetch/decode failure renders g.textFailedToLoad and does not expose partial stale text; retry behavior is owned by the composable/result reopen flow.",
        "accessibility": "The result is semantic article text and remains keyboard scrollable; the failure is textual and must be announced when it replaces content.",
        "interfaces": "Passes the queue ResultItem to useTextFileContent, which performs the media text retrieval; the component has no write side effect.",
    },
    "src/components/sidebar/tabs/queue/ResultVideo.vue": {
        "observable": "A video queue result renders native playback controls and chooses either the normal result URL/type or the VideoHelperSuite advanced WebM preview when the extension is installed, enabled, and its VHS.AdvancedPreviews setting is not Never.",
        "state": "Extension installation/enabled state and the VHS setting recompute URL and MIME type reactively; source fallback text remains present for unsupported playback.",
        "failure": "Unavailable media or unsupported MIME playback uses the native video failure path and localized fallback text; an unavailable/disabled extension deterministically falls back to the standard result URL.",
        "accessibility": "Native video controls require keyboard playback, seeking, volume, captions where available, and an accessible fallback/error; the media result needs a contextual label in the owning viewer.",
        "interfaces": "Reads extension and setting stores, including VHS.AdvancedPreviews, and asks the browser media element to retrieve the selected result URL.",
    },
}


def clean(value, limit=1800):
    text = re.sub(r"\s+", " ", str(value or "")).strip()
    if len(text) > limit:
        return text[: limit - 1].rstrip() + "…"
    return text


def read_rows(name):
    path = CATALOGS / name
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def split_feature_ids(value):
    return {part.strip() for part in (value or "").split("|") if part.strip()}


def feature_id(source_file):
    digest = hashlib.sha256(
        ("comfyui-frontend-vue-component-surface-v1\0" + source_file).encode("utf-8")
    ).hexdigest()[:12].upper()
    return f"COMFY-FRONTEND-SURFACE-{digest}"


def balanced_block(text, start, opening, closing):
    if start < 0 or start >= len(text) or text[start] != opening:
        return ""
    depth = 0
    quote = ""
    escaped = False
    for index in range(start, len(text)):
        character = text[index]
        if quote:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = ""
            continue
        if character in "'\"`":
            quote = character
        elif character == opening:
            depth += 1
        elif character == closing:
            depth -= 1
            if depth == 0:
                return text[start + 1 : index]
    return ""


def section(text, tag):
    match = re.search(rf"<{tag}\b[^>]*>(.*?)</{tag}>", text, re.DOTALL | re.IGNORECASE)
    return match.group(1) if match else ""


def unique(values, limit=24):
    result = []
    for value in values:
        value = clean(value, 300)
        if value and value not in result:
            result.append(value)
        if len(result) >= limit:
            break
    return result


def extract_props(script):
    names = []
    for destructuring in re.findall(r"(?:const|let)\s*\{([^}]+)\}\s*=\s*(?:withDefaults\s*\()?defineProps", script):
        for item in destructuring.split(","):
            name = item.strip().split(":", 1)[0].split("=", 1)[0].strip()
            if re.fullmatch(r"[A-Za-z_$][\w$]*", name):
                names.append(name)

    offset = 0
    while True:
        match = re.search(r"defineProps\s*<", script[offset:])
        if not match:
            break
        opening = offset + match.end() - 1
        generic = balanced_block(script, opening, "<", ">")
        inline_names = re.findall(r"(?:^|[;,{\n])\s*(?:readonly\s+)?([A-Za-z_$][\w$]*)\??\s*:", generic)
        if inline_names:
            names.extend(inline_names)
        else:
            type_name = clean(generic)
            if re.fullmatch(r"[A-Za-z_$][\w$]*", type_name):
                declaration = re.search(
                    rf"(?:interface\s+{re.escape(type_name)}|type\s+{re.escape(type_name)}\s*=)\s*\{{",
                    script,
                )
                if declaration:
                    body = balanced_block(script, declaration.end() - 1, "{", "}")
                    names.extend(re.findall(r"(?:^|[;,{\n])\s*(?:readonly\s+)?([A-Za-z_$][\w$]*)\??\s*:", body))
                else:
                    names.append(f"type:{type_name}")
        offset = opening + max(1, len(generic) + 2)
    return unique(names)


def extract_models(script):
    models = []
    for match in re.finditer(r"defineModel(?:\s*<[^;\n]+?>)?\s*\(", script):
        body = balanced_block(script, match.end() - 1, "(", ")")
        name_match = re.match(r"\s*['\"]([^'\"]+)['\"]", body)
        name = name_match.group(1) if name_match else "modelValue"
        default_match = re.search(r"\bdefault\s*:\s*([^,}]+)", body)
        models.append(f"{name} default={clean(default_match.group(1))}" if default_match else name)
    return unique(models)


def extract_template_models(template):
    models = []
    for match in re.finditer(r"v-model(?::([\w-]+))?(?:\.[\w.-]+)*\s*=\s*\"([^\"]+)\"", template):
        argument = match.group(1) or "modelValue"
        models.append(f"{argument} -> {clean(match.group(2), 180)}")
    return unique(models, 24)


def extract_emits(script):
    events = list(re.findall(r"\bemit\s*\(\s*['\"]([^'\"]+)['\"]", script))
    offset = 0
    while True:
        match = re.search(r"defineEmits\s*<", script[offset:])
        if not match:
            break
        opening = offset + match.end() - 1
        generic = balanced_block(script, opening, "<", ">")
        events.extend(re.findall(r"(?:^|[;,{\n])\s*['\"]?([\w:-]+)['\"]?\??\s*[:(]", generic))
        offset = opening + max(1, len(generic) + 2)
    for match in re.finditer(r"defineEmits\s*\(\s*\[([^\]]+)\]", script):
        events.extend(re.findall(r"['\"]([^'\"]+)['\"]", match.group(1)))
    return unique(events)


def extract_template_events(template):
    bindings = []
    for match in re.finditer(r"@([\w:-]+)(?:\.[\w.:-]+)?\s*=\s*\"([^\"]+)\"", template):
        tag_start = template.rfind("<", 0, match.start())
        tag_match = re.match(r"<\s*([A-Za-z][\w.-]*)", template[tag_start:]) if tag_start >= 0 else None
        tag = tag_match.group(1) if tag_match else "component"
        bindings.append(f"<{tag}> @{match.group(1)} -> {clean(match.group(2), 240)}")
    return unique(bindings, 40)


def extract_conditionals(template):
    values = re.findall(
        r"(?:v-if|v-else-if|v-show|:loading|:disabled|:visible|:open)\s*=\s*\"([^\"]+)\"",
        template,
    )
    return unique(values, 30)


def extract_links(template):
    values = []
    for attribute, value in re.findall(r"\b(:?href|:?to)\s*=\s*\"([^\"]+)\"", template):
        values.append(f"{attribute}={clean(value, 180)}")
    return unique(values, 16)


def extract_handlers(script):
    handlers = {}
    patterns = (
        re.compile(r"(?:async\s+)?function\s+([A-Za-z_$][\w$]*)\s*\([^)]*\)[^{]*\{"),
        re.compile(r"(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*(?::[^=\n]+)?=>\s*\{"),
    )
    for pattern in patterns:
        for match in pattern.finditer(script):
            body = balanced_block(script, match.end() - 1, "{", "}")
            if not body:
                continue
            calls = unique(re.findall(r"\b([A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)\s*\(", body), 10)
            assignments = unique(
                f"{left} {operator} {clean(right, 100)}"
                for left, operator, right in re.findall(
                    r"\b([A-Za-z_$][\w$]*(?:\.value|\.[A-Za-z_$][\w$]*)?)\s*(=|\+=|-=|\+\+|--)\s*([^;\n}]*)",
                    body,
                )
            )
            throws = unique(re.findall(r"throw\s+([^;\n]+)", body), 4)
            summary = []
            if calls:
                summary.append("calls " + ", ".join(calls))
            if assignments:
                summary.append("updates " + ", ".join(assignments))
            if throws:
                summary.append("throws " + ", ".join(throws))
            handlers[match.group(1)] = "; ".join(summary) or clean(body, 420)
    return handlers


def extract_side_effects(script):
    patterns = (
        r"\b(?:electron|workspaceApi|api|router|dialogService|toast|window|navigator|localStorage|sessionStorage|fetch)\.[A-Za-z_$][\w$]*",
        r"\b(?:fetch|open|postMessage|sendBeacon)\s*\(",
        r"\b(?:useStorage|useLocalStorage|useSessionStorage|onMounted|onUnmounted|onBeforeUnmount|watch|watchEffect|defineExpose)\s*\(",
    )
    values = []
    for pattern in patterns:
        values.extend(re.findall(pattern, script))
    return unique(value.rstrip("(") for value in values)


def extract_children(template):
    html = {
        "template", "div", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6",
        "section", "article", "main", "header", "footer", "nav", "ul", "ol", "li",
        "label", "form", "button", "input", "select", "option", "textarea", "a", "img",
        "video", "audio", "source", "canvas", "svg", "path", "figure", "figcaption", "br",
        "table", "thead", "tbody", "tr", "th", "td", "i", "strong", "small", "slot",
    }
    return unique(tag for tag in re.findall(r"<\s*([A-Za-z][\w.-]*)", template) if tag.casefold() not in html)


def extract_test_evidence(source_file):
    path = Path(source_file)
    base = path.stem
    directory = FRONTEND_ROOT / path.parent
    candidates = []
    for suffix in (".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx"):
        candidate = directory / f"{base}{suffix}"
        if candidate.exists():
            candidates.append(candidate)
    test_directory = directory / "__tests__"
    if test_directory.exists():
        candidates.extend(sorted(test_directory.glob(f"{base}*.test.*")))
        candidates.extend(sorted(test_directory.glob(f"{base}*.spec.*")))
    if not candidates:
        return "", []
    test_path = sorted(set(candidates))[0]
    text = test_path.read_text(encoding="utf-8", errors="replace")
    names = unique(re.findall(r"\b(?:it|test)\s*\(\s*['\"]([^'\"]+)['\"]", text), 8)
    relative = test_path.relative_to(FRONTEND_ROOT).as_posix()
    return relative, names


def source_domain(source_file, anchor):
    lower = source_file.casefold()
    if "apps/desktop-ui" in lower:
        if any(token in lower for token in ("install", "welcome", "migration", "hardware")):
            return "desktop-installation"
        if any(token in lower for token in ("maintenance", "terminal", "log", "metric")):
            return "desktop-diagnostics"
        if "update" in lower:
            return "desktop-update"
        return "desktop-native-ui"
    if "apps/website" in lower:
        return "website-cloud"
    if any(token in lower for token in ("asset", "media", "load3d", "mask", "crop", "painter", "image", "audio", "video", "result")):
        return "asset-viewer-editor"
    if any(token in lower for token in ("queue", "job", "execution", "progress")):
        return "queue-execution-ui"
    if any(token in lower for token in ("workflow", "template", "builder", "linearmode", "appmode", "topbar")):
        return "workflow-experience"
    if any(token in lower for token in ("graph", "node", "litegraph", "vuenodes", "canvas", "widget", "subgraph")):
        return "graph-editor"
    if any(token in lower for token in ("setting", "palette", "keybinding", "theme")):
        return "settings"
    if any(token in lower for token in ("cloud", "subscription", "billing", "credit", "auth", "workspace", "user")):
        return "cloud-account-workspace"
    if "extension" in lower or "manager" in lower:
        return "frontend-extension-manager"
    if anchor.startswith("COMFY-ASSET-"):
        return "asset-viewer-editor"
    if anchor.startswith("COMFY-GRAPH-"):
        return "graph-editor"
    if anchor.startswith("COMFY-WORKFLOW-"):
        return "workflow-experience"
    if anchor.startswith("COMFY-QUEUE-"):
        return "queue-execution-ui"
    if anchor.startswith("COMFY-CLOUD-"):
        return "cloud-account-workspace"
    if anchor.startswith("COMFY-SETTING-"):
        return "settings"
    return "application-ui"


def component_purpose(component, props, children):
    lower = component.casefold()
    if "icon" in lower:
        noun = "icon/status glyph renderer"
    elif any(token in lower for token in ("skeleton", "placeholder", "empty")):
        noun = "placeholder or loading-state renderer"
    elif any(token in lower for token in ("badge", "tag", "label", "divider", "separator")):
        noun = "label/status decoration"
    elif any(token in lower for token in ("image", "thumbnail", "avatar", "logo")):
        noun = "media/identity presentation"
    elif any(token in lower for token in ("layout", "wrapper", "container", "section", "header", "footer", "hero")):
        noun = "layout and typography wrapper"
    elif any(token in lower for token in ("card", "item", "row", "panel", "content")):
        noun = "record/content presentation"
    else:
        noun = "render-only presentation"
    details = []
    if props:
        details.append("props " + ", ".join(props[:8]))
    if children:
        details.append("child components " + ", ".join(children[:8]))
    return noun + (" for " + "; ".join(details) if details else " with static source-local markup")


def summarize_bindings(event_bindings, handler_map):
    summaries = []
    for binding in event_bindings[:12]:
        expression = binding.split("->", 1)[1].strip() if "->" in binding else ""
        identifier = re.match(r"([A-Za-z_$][\w$]*)\b", expression)
        detail = handler_map.get(identifier.group(1), "") if identifier else ""
        summaries.append(binding + (f" ({detail})" if detail else ""))
    return summaries


def build_row(source_row, selection_basis):
    source_file = source_row["source_file"]
    path = FRONTEND_ROOT / source_file
    text = path.read_text(encoding="utf-8", errors="replace")
    script = section(text, "script")
    template = section(text, "template")
    component = Path(source_file).stem
    props = extract_props(script)
    models = extract_models(script)
    template_models = extract_template_models(template)
    emits = extract_emits(script)
    event_bindings = extract_template_events(template)
    conditionals = extract_conditionals(template)
    links = extract_links(template)
    handlers = extract_handlers(script)
    side_effects = extract_side_effects(script)
    children = extract_children(template)
    localized_keys = unique(re.findall(r"(?:\$t|\bt)\s*\(\s*['\"]([^'\"]+)['\"]", text), 12)
    has_native_control = bool(
        re.search(r"<(?:button|input|select|textarea|details|summary|dialog)\b", template, re.IGNORECASE)
        or re.search(r"\bcontenteditable(?:\s*=|\s|>)", template, re.IGNORECASE)
        or re.search(r"<component\b[^>]*:is\s*=\s*\"[^\"]*['\"](?:a|button|input)['\"]", template, re.IGNORECASE)
    )
    has_native_link = bool(re.search(r"<(?:a|routerlink|router-link)\b", template, re.IGNORECASE))
    has_media_controls = bool(re.search(r"<(?:video|audio)\b[^>]*\bcontrols\b", template, re.IGNORECASE))
    interactive_children = [
        child for child in children
        if any(token in child.casefold() for token in (
            "button", "input", "select", "checkbox", "toggle", "slider", "menu", "dialog",
            "drawer", "editor", "canvas", "terminal", "player", "viewer", "carousel", "lightbox",
            "table", "tree", "tabs", "link", "picker", "controls",
        ))
    ]
    lifecycle_or_service = bool(side_effects)
    meaningful_state = bool(conditionals)
    forced = MANUAL_CONTRACTS.get(source_file)
    semantic_state_component = bool(
        re.search(
            r"(?:Status|Progress|Alert|Notification|Error|Loading|Spinner|Toast|Banner|Empty|Placeholder|Message|Tooltip|Popover|Modal|Dialog|Drawer)",
            component,
        )
    )
    semantic_data_component = bool(
        re.search(r"(?:Shortcuts|ApiNodes|Usage|Results|History|Logs|Controls)", component)
    )
    declared_control_component = bool(
        ("Button" in component and "ButtonGroup" not in component)
        or re.search(r"\bas\s*=\s*['\"]button['\"]", script)
    )
    route_view = (
        ("/views/" in f"/{source_file}" or component.endswith("View"))
        and "/layouts/" not in f"/{source_file}"
        and not component.endswith("Skeleton")
    )
    functional = bool(
        forced
        or event_bindings
        or emits
        or models
        or template_models
        or has_native_control
        or has_native_link
        or has_media_controls
        or interactive_children
        or lifecycle_or_service
        or meaningful_state
        or semantic_state_component
        or semantic_data_component
        or declared_control_component
        or route_view
    )

    anchor = source_row["primary_anchor"]
    domain = source_domain(source_file, anchor)
    product = (
        "ComfyUI-Frontend desktop-ui"
        if source_file.startswith("apps/desktop-ui/")
        else "ComfyUI-Frontend website"
        if source_file.startswith("apps/website/")
        else "ComfyUI-Frontend"
    )
    source_availability = {
        "production": "active",
        "cloud/paid": "cloud/paid",
        "platform-specific": "platform-specific",
    }.get(source_row["classification"], "uncertain")
    availability = source_availability if functional else "infrastructure-only"
    classification = "functional Vue component surface" if functional else "infrastructure-only"
    test_file, test_names = extract_test_evidence(source_file)
    # A same-name component test often covers only a subset of the exact
    # handlers and branches represented by this source-file contract. Keep the
    # row conservatively code-inferred while retaining the focused test names.
    evidence = "code-inferred"

    summarized_bindings = summarize_bindings(event_bindings, handlers)
    transitions = []
    if summarized_bindings:
        transitions.append("; ".join(summarized_bindings))
    if models:
        transitions.append("Parent model updates: " + ", ".join(models))
    if template_models:
        transitions.append("Template two-way bindings: " + ", ".join(template_models))
    if emits:
        transitions.append("Parent events: " + ", ".join(emits))
    if conditionals:
        transitions.append("Rendered state guards: " + ", ".join(conditionals[:12]))
    if side_effects:
        transitions.append("Lifecycle/interface calls: " + ", ".join(side_effects[:12]))

    behavior = []
    if event_bindings:
        behavior.append("Exposes " + "; ".join(event_bindings[:10]))
    if emits:
        behavior.append("Emits " + ", ".join(emits))
    if models:
        behavior.append("Two-way models " + ", ".join(models))
    if template_models:
        behavior.append("Two-way child/control bindings " + ", ".join(template_models))
    if links:
        behavior.append("Navigation targets " + ", ".join(links))
    if conditionals:
        behavior.append("Selects visible variants from " + ", ".join(conditionals[:10]))
    if has_media_controls:
        behavior.append("Renders native audio/video controls")
    if interactive_children:
        behavior.append("Delegates controls to " + ", ".join(interactive_children[:10]))

    purpose = component_purpose(component, props, children)
    infrastructure_reason = (
        "Not applicable: executable interaction/state surface."
        if functional
        else (
            f"At {source_file}, {component} is a source-specific {purpose}. Static inspection found no defineModel, "
            "defineEmits/emit, template event/model binding, native control/link/media controls, interactive child, "
            "route/API/storage call, lifecycle watcher, or conditional state branch; its behavior is exhausted "
            f"by rendering the cited props/markup for {anchor}."
        )
    )

    if forced:
        observable = forced["observable"]
        state = forced["state"]
        failure = forced["failure"]
        accessibility = forced["accessibility"]
        interfaces = forced["interfaces"]
        persistence = forced.get("persistence", "No durable state is written directly; parent models and cited stores/services own persistence.")
    elif functional:
        observable = clean(
            f"{component} provides this independently testable surface: "
            + (". ".join(behavior) if behavior else f"reactively renders {purpose}")
            + "."
        )
        state = clean("; ".join(transitions) or "Prop/store changes synchronously replace the rendered projection; no component-local concurrent task was found.")
        catch_count = len(re.findall(r"\bcatch\s*(?:\([^)]*\))?\s*\{", script))
        state_words = [value for value in conditionals if re.search(r"load|error|empty|valid|pending|success|fail|visible|open|selected", value, re.IGNORECASE)]
        failure = clean(
            (f"The component has {catch_count} explicit catch branch(es); " if catch_count else "No component-local catch branch was found; ")
            + ("visible boundary states are " + ", ".join(state_words) + "." if state_words else "invalid, empty, unavailable, or rejected child/service behavior remains with the cited parent/composable and must not leave stale state.")
        )
        click_divs = bool(re.search(r"<(?:div|span|li)\b[^>]*@click", template, re.IGNORECASE))
        keyboard = bool(re.search(r"@key(?:down|up)|tabindex|role=|aria-", template, re.IGNORECASE))
        accessibility_parts = []
        if has_native_control or has_native_link or has_media_controls:
            accessibility_parts.append("Uses native control/link/media semantics for at least one interaction")
        if interactive_children:
            accessibility_parts.append("delegates additional focus/name/state semantics to " + ", ".join(interactive_children[:8]))
        if click_divs and not keyboard:
            accessibility_parts.append("static source contains a non-native click target without a colocated keyboard/role binding; this is an explicit source accessibility risk, not behavior to copy")
        if not accessibility_parts:
            accessibility_parts.append("Has no direct native focus target; the owning surface must expose its dynamic state as labelled text/status")
        accessibility = "; ".join(accessibility_parts) + ". Sim acceptance requires keyboard, focus, name, role, state, disabled/loading, and error equivalence without preserving source defects."
        interfaces = clean(
            "; ".join(
                part for part in (
                    "models=" + ", ".join(models + template_models) if (models or template_models) else "",
                    "emits=" + ", ".join(emits) if emits else "",
                    "calls=" + ", ".join(side_effects) if side_effects else "",
                    "links=" + ", ".join(links) if links else "",
                ) if part
            )
            or "Consumes source-defined props/stores and delegates side effects to the named child components."
        )
        storage_keys = unique(re.findall(r"(?:localStorage|sessionStorage|useStorage)\s*(?:\.\w+)?\s*\(\s*['\"]([^'\"]+)", script), 10)
        persistence = (
            "Writes/reads browser state keys: " + ", ".join(storage_keys) + "."
            if storage_keys
            else "No direct durable write was resolved; props/models are parent-owned and cited stores/services own any persistence."
        )
    else:
        observable = f"{component} renders {purpose}; it has no independently invokable user action or state transition beyond that source-specific presentation."
        state = "Render-only: prop/slot changes replace markup synchronously; no local mutable transition, asynchronous task, emit, model, or concurrency boundary was found."
        failure = "No component-local operation can fail; missing/invalid display data follows Vue/child rendering and the owning functional surface supplies empty/error/recovery behavior."
        accessibility = "No direct focus target or action is defined; semantic reading order, contrast, text alternatives, and child accessibility are validated through the consuming functional surface."
        persistence = "No model, storage API, durable state, migration, or restart behavior is defined."
        interfaces = "No route, HTTP, WebSocket, IPC, clipboard, filesystem, external-link, storage, or emitted-event side effect is defined."

    source_symbols = [component]
    source_symbols.extend(handlers.keys())
    source_symbols.extend(f"prop:{item}" for item in props)
    source_symbols.extend(f"model:{item}" for item in models)
    source_symbols.extend(f"emit:{item}" for item in emits)
    source_symbols = unique(source_symbols, 35)
    line_numbers = []
    for symbol in [component] + list(handlers.keys())[:8]:
        found = re.search(rf"\b{re.escape(symbol)}\b", text)
        if found:
            line_numbers.append(str(text.count("\n", 0, found.start()) + 1))
    source_evidence = f"{source_file}:{','.join(unique(line_numbers, 12)) or '1'}"

    exact_excerpt_parts = []
    if event_bindings:
        exact_excerpt_parts.append("events=" + " | ".join(event_bindings[:16]))
    if models or template_models:
        exact_excerpt_parts.append("models=" + ", ".join(models + template_models))
    if emits:
        exact_excerpt_parts.append("emits=" + ", ".join(emits))
    if handlers:
        exact_excerpt_parts.append("handlers=" + " | ".join(f"{name}: {summary}" for name, summary in list(handlers.items())[:12]))
    if conditionals:
        exact_excerpt_parts.append("states=" + ", ".join(conditionals[:14]))
    if links:
        exact_excerpt_parts.append("links=" + ", ".join(links))
    if not exact_excerpt_parts:
        exact_excerpt_parts.append(f"render={purpose}")

    flags = unique(re.findall(r"\b(?:is[A-Z][\w$]*|feature[A-Z][\w$]*|enable[A-Z][\w$]*|Comfy\.[A-Za-z0-9_.-]+|VHS\.[A-Za-z0-9_.-]+)\b", text), 16)
    default_details = []
    if props:
        default_details.append("props=" + ", ".join(props))
    if models or template_models:
        default_details.append("models=" + ", ".join(models + template_models))
    if not default_details:
        default_details.append("No explicit prop or model input; slots/parent context follow the cited template.")
    test_evidence = (
        f"{test_file}: " + ("; ".join(test_names) if test_names else "matching component test file")
        + "; focused supporting evidence is retained, but this catalog row remains code-inferred because it also records untested source branches"
        if test_file
        else "No same-component .test/.spec file was located; evidence is executable SFC code."
    )

    acceptance = (
        f"With the same props/models/store state and {source_availability} distribution, Sim shall reproduce {component}'s "
        + ("actions, emitted/model transitions, visible state variants, failure/recovery, and accessible focus semantics." if functional else "source-specific rendered information and semantic layout through the owning surface, without inventing a standalone action.")
    )
    automated = (
        f"Create deterministic component/GPUI contract fixtures for {feature_id(source_file)} covering every cataloged event/model/emit, condition, success, empty/loading/error, and keyboard/focus state; use {test_file} as source evidence."
        if functional and test_file
        else f"Create a deterministic {'interaction/state' if functional else 'render/semantic'} contract fixture for {feature_id(source_file)} from the exact bindings and states in this row."
    )
    manual = (
        f"Open {component} in its owning source surface and Sim with identical data; exercise pointer and keyboard paths, state branches, failure/recovery, localization, and applicable platform/cloud gates."
        if functional
        else f"Inspect {component} in its owning source surface and Sim for semantic reading order, localized content, responsive layout, contrast, and absence of unintended actions."
    )

    return {
        "feature_id": feature_id(source_file),
        "selection_basis": selection_basis,
        "source_file": source_file,
        "source_distribution_classification": source_row["classification"],
        "primary_coverage_anchor": anchor,
        "product": product,
        "domain": domain,
        "surface": source_file.rsplit("/", 1)[0],
        "name": f"{component} {'surface contract' if functional else 'presentation disposition'}",
        "classification": classification,
        "availability": availability,
        "evidence_level": evidence,
        "confidence": "high" if forced or not functional or event_bindings or models or emits else "medium",
        "source_evidence": source_evidence,
        "source_symbol": "; ".join(source_symbols),
        "source_excerpt": clean("; ".join(exact_excerpt_parts)),
        "test_evidence": test_evidence,
        "actor": "User or keyboard/pointer operator of the owning surface." if functional else "Owning Vue surface renderer.",
        "trigger": (f"Render {component}, then invoke its cataloged event/control or change its model/store state." if functional else f"Render {component} through its owning {anchor} surface."),
        "preconditions": f"The {source_availability} frontend distribution, parent data/slots, cited stores/services, and child components are available.",
        "inputs_defaults": "; ".join(default_details),
        "observable_success": observable,
        "state_transitions_concurrency": state,
        "failure_recovery": failure,
        "interaction_accessibility": accessibility,
        "persistence_serialization": persistence,
        "interfaces_side_effects": interfaces,
        "platform_localization_variants": (
            f"Source distribution={source_row['classification']}; localized keys={', '.join(localized_keys) if localized_keys else 'none in this component'}; responsive/platform branches are those in the cited template/script."
        ),
        "feature_flags_permissions": f"Availability={source_availability}; source gates/permission-like state={', '.join(flags) if flags else 'none resolved locally; parent route/store gates apply'}.",
        "infrastructure_disposition_reason": infrastructure_reason,
        "observable_sim_acceptance": acceptance,
        "automated_validation": automated,
        "manual_validation": manual,
        "open_questions": "Runtime-only child-component semantics, cloud service outcomes, and platform bridge results remain explicit until side-by-side execution; no behavior beyond the cited source is inferred.",
        "props": ", ".join(props) if props else "none resolved",
        "models": ", ".join(models + template_models) if (models or template_models) else "none",
        "emits": ", ".join(emits) if emits else "none",
        "template_event_bindings": " | ".join(event_bindings) if event_bindings else "none",
        "handlers": " | ".join(f"{name}: {summary}" for name, summary in handlers.items()) if handlers else "none",
        "conditional_states": ", ".join(conditionals) if conditionals else "none",
    }


def candidate_rows():
    ledger = read_rows("frontend-source-files.csv")
    features = read_rows("frontend-features.csv")
    anchors = {row["feature_id"] for row in features if row["classification"] == "coverage-anchor"}
    references = "\n".join(
        " ".join(value or "" for value in row.values())
        for catalog in REFERENCE_CATALOGS
        for row in read_rows(catalog)
    )
    feature_references = "\n".join(
        (row.get("source_file") or "") + " " + (row.get("test") or "")
        for row in features
    )
    previous = {
        row["source_file"]: row.get("selection_basis", "")
        for row in read_rows("frontend-component-surfaces.csv")
    }

    selected = []
    for row in ledger:
        source_file = row["source_file"]
        identifiers = split_feature_ids(row["feature_ids"])
        stable_id = feature_id(source_file)
        exact_audit_candidate = (
            row["classification"] in {"production", "cloud/paid", "platform-specific"}
            and source_file.endswith(".vue")
            and bool(identifiers)
            and identifiers <= anchors
            and source_file not in feature_references
            and source_file not in references
        )
        already_mapped = stable_id in identifiers
        required_override = source_file in REQUIRED_FUNCTIONAL_OVERRIDES
        if not (exact_audit_candidate or already_mapped or required_override):
            continue
        if not source_file.endswith(".vue") or not (FRONTEND_ROOT / source_file).exists():
            continue
        basis = previous.get(source_file)
        if not basis:
            basis = "broad-anchor-only" if exact_audit_candidate else "required-functional-override"
        selected.append((row, basis))
    return sorted(selected, key=lambda item: item[0]["source_file"])


def generate():
    rows = [build_row(source_row, basis) for source_row, basis in candidate_rows()]
    path = CATALOGS / "frontend-component-surfaces.csv"
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    return rows


def augment_source_ledger():
    surfaces = {row["source_file"]: row for row in read_rows("frontend-component-surfaces.csv")}
    if not surfaces:
        return
    path = CATALOGS / "frontend-source-files.csv"
    rows = read_rows("frontend-source-files.csv")
    if not rows:
        return
    changed = False
    for row in rows:
        surface = surfaces.get(row["source_file"])
        if surface is None:
            continue
        identifiers = [part.strip() for part in row["feature_ids"].split("|") if part.strip()]
        if surface["feature_id"] not in identifiers:
            identifiers.append(surface["feature_id"])
            row["feature_ids"] = " | ".join(identifiers)
            changed = True
        if row["primary_anchor"] != surface["feature_id"]:
            row["primary_anchor"] = surface["feature_id"]
            changed = True
        disposition = (
            f"Mapped to source-specific Vue component contract {surface['feature_id']}. "
            + (
                "The component exposes independently testable interactions or state variants; its former broad coverage anchor remains as a domain cross-reference."
                if surface["classification"] == "functional Vue component surface"
                else surface["infrastructure_disposition_reason"]
            )
        )
        if row["reason"] != disposition:
            row["reason"] = disposition
            changed = True
    if changed:
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(
                handle, fieldnames=list(rows[0].keys()), lineterminator="\n"
            )
            writer.writeheader()
            writer.writerows(rows)


if __name__ == "__main__":
    generated = generate()
    functional_count = sum(row["classification"] == "functional Vue component surface" for row in generated)
    infrastructure_count = len(generated) - functional_count
    broad_count = sum(row["selection_basis"] == "broad-anchor-only" for row in generated)
    print(
        f"wrote {len(generated)} rows: broad={broad_count}, "
        f"functional={functional_count}, infrastructure={infrastructure_count}"
    )
