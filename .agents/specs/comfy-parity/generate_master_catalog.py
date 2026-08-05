#!/usr/bin/env python3

from __future__ import annotations

import csv
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

csv.field_size_limit(16 * 1024 * 1024)

from regenerate_native_sim_evidence import (
    ACCESSIBLE_COMFY_BOOTSTRAP,
    EXECUTION_UI_OWNER,
    TASK_18_COMPONENT_OWNERS,
    TASK_18_EXECUTION_COMMANDS,
    TASK_18_JOB_RUN_MENU_OWNERS,
    TASK_18_QUEUE_DISPOSITIONS,
    validate_task18_disposition_ledger,
)


ROOT = Path(__file__).resolve().parent
CATALOGS = ROOT / "catalogs"
REPO_ROOT = ROOT.parents[2]
COMFYUI_ROOT = REPO_ROOT / "projects/comfy/ComfyUI"
FRONTEND_ROOT = REPO_ROOT / "projects/comfy/ComfyUI-Frontend"
COMFY_CLI_ROOT = REPO_ROOT / "projects/comfy/comfy-cli"
DOCS_ROOT = REPO_ROOT / "projects/comfy/docs"
EMBEDDED_DOCS_ROOT = REPO_ROOT / "projects/comfy/embedded-docs"


A11Y_FOUNDATION_FEATURES = {
    "COMFY-A11Y-001",
    "COMFY-A11Y-002",
    "COMFY-A11Y-003",
    "COMFY-A11Y-004",
    "COMFY-A11Y-005",
}
ACCESSIBILITY_INVENTORY_SUMMARY = (
    "The accessibility bootstrap, native graph semantics, and implemented graph keybinding rows "
    "are `partial`: production now enables GPUI accessibility without an environment gate. "
    "Exact later-owned accessibility rows retain their prior missing or conflicting status until "
    "their surface tasks and whole-application audits pass."
    if ACCESSIBLE_COMFY_BOOTSTRAP
    else "Dedicated accessibility rows are `conflicting` because current production Sim defaults "
    "to `Application::new_inaccessible` unless `SIM_EXPERIMENTAL_A11Y=1`."
)

FEATURE_FIELDS = [
    "feature_id",
    "product",
    "domain",
    "name",
    "classification",
    "availability",
    "evidence_level",
    "confidence",
    "source_evidence",
    "source_symbol",
    "test_evidence",
    "documentation",
    "runtime_observation",
    "actor",
    "trigger",
    "preconditions",
    "inputs_defaults",
    "permissions_flags",
    "observable_success",
    "interaction_accessibility",
    "state_concurrency",
    "failure_recovery",
    "persistence_serialization",
    "interfaces_side_effects",
    "platform_localization_variants",
    "current_sim_status",
    "sim_evidence",
    "parity_gap",
    "parity_decision",
    "observable_sim_acceptance",
    "requirement_criteria",
    "design_coverage",
    "task_id",
    "validation_id",
    "automated_validation",
    "manual_validation",
    "open_questions",
    "source_catalog",
    "source_row",
]

MAPPING = json.loads((CATALOGS / "native-spec-mapping.json").read_text(encoding="utf-8"))
REQUIREMENT_CRITERIA_COUNTS = {
    int(number): count for number, count in MAPPING["requirement_criteria_counts"].items()
}
REQUIREMENT_TASKS = {
    int(number): values for number, values in MAPPING["requirement_tasks"].items()
}
CRITERION_TASKS = MAPPING["criterion_tasks"]
CRITERION_DESIGNS = MAPPING["criterion_designs"]
CRITERION_VALIDATIONS = MAPPING["criterion_validations"]
FEATURE_CRITERION_OVERRIDES = MAPPING["feature_criterion_overrides"]
FEATURE_VALIDATION_OVERRIDES = MAPPING.get("feature_validation_overrides", {})
FEATURE_SCOPED_TASK_IDS = set(MAPPING["feature_scoped_task_ids"])
SPECIAL_FEATURE_TASKS = MAPPING["special_feature_tasks"]


def clean(value: object, fallback: str = "") -> str:
    if value is None:
        return fallback
    text = re.sub(r"\s+", " ", str(value)).strip()
    return text or fallback


def first(row: dict[str, str], *names: str, fallback: str = "") -> str:
    for name in names:
        value = clean(row.get(name, ""))
        if value:
            return value
    return fallback


def joined(*values: object, fallback: str = "") -> str:
    result = []
    for value in values:
        text = clean(value)
        if text and text not in result:
            result.append(text)
    return "; ".join(result) or fallback


def read_rows(name: str) -> list[dict[str, str]]:
    path = CATALOGS / name
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def rewrite_catalog(name: str, rows: list[dict[str, str]], fieldnames: list[str]) -> None:
    with (CATALOGS / name).open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def source_lines(path: Path) -> list[str]:
    try:
        return path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return []


def compact_excerpt(lines: list[str], start: int, end: int, limit: int = 1200) -> str:
    if not lines:
        return "Source file unavailable during static extraction."
    lower = max(0, start - 1)
    upper = min(len(lines), max(lower + 1, end))
    excerpts = []
    for offset in range(lower, upper):
        text = clean(lines[offset])
        if text:
            excerpts.append(f"L{offset + 1}: {text}")
    result = " | ".join(excerpts)
    if len(result) > limit:
        return result[: limit - 1].rstrip() + "…"
    return result or f"No nonblank source text at lines {start}-{end}."


def statement_excerpt(path: Path, line_number: int, limit_lines: int = 18) -> str:
    lines = source_lines(path)
    if not lines or line_number < 1 or line_number > len(lines):
        return f"Unresolved source statement at {path}:{line_number}."

    start = line_number
    end = min(len(lines), start + limit_lines - 1)
    first_text = lines[start - 1].strip()
    first_balance = (
        first_text.count("(") + first_text.count("[") + first_text.count("{")
        - first_text.count(")") - first_text.count("]") - first_text.count("}")
    )
    if first_balance <= 0 and (
        re.search(r"\)\s*:\s*[^=].+$", first_text)
        or re.search(r"^[A-Za-z_$][A-Za-z0-9_$]*\??\s*:\s*[^=].+$", first_text)
        or first_text.endswith((",", ";"))
    ):
        return compact_excerpt(lines, start, start)
    balance = 0
    saw_delimiter = False
    for current in range(start, end + 1):
        text = lines[current - 1]
        balance += text.count("(") + text.count("[") + text.count("{")
        balance -= text.count(")") + text.count("]") + text.count("}")
        saw_delimiter = saw_delimiter or any(token in text for token in ("(", "=>", ":", "="))
        stripped = text.strip()
        if current > start and saw_delimiter and balance <= 0 and (
            stripped.endswith((",", ";", "}", ")"))
            or (":" in stripped and not stripped.endswith(("(", "{", "[", ",")))
        ):
            end = current
            break
    return compact_excerpt(lines, start, end)


def resolve_named_member_line(path: Path, line_number: int, member: str) -> int:
    lines = source_lines(path)
    if not lines:
        return line_number
    escaped = re.escape(member)
    pattern = re.compile(
        rf"^\s*(?:readonly\s+)?(?:['\"])?{escaped}(?:['\"])?\s*(?:\??\s*[:(=]|,\s*$|$)"
    )
    candidates = [index for index, line in enumerate(lines, 1) if pattern.search(line)]
    if not candidates:
        return line_number
    return min(candidates, key=lambda candidate: abs(candidate - line_number))


def resolve_channel_literal_line(path: Path, line_number: int, channel: str) -> int:
    lines = source_lines(path)
    if not lines:
        return line_number
    patterns = (f"'{channel}'", f'"{channel}"', f"`{channel}`")
    candidates = [
        index for index, line in enumerate(lines, 1)
        if any(pattern in line for pattern in patterns)
    ]
    if not candidates:
        return line_number
    return min(candidates, key=lambda candidate: abs(candidate - line_number))


def parse_source_references(value: str) -> list[tuple[Path, int, str]]:
    references = []
    pattern = re.compile(r"(projects/comfy/Comfy-Desktop/[^:;]+):(\d+)(?: \(([^)]+)\))?")
    for match in pattern.finditer(value):
        references.append((REPO_ROOT / match.group(1), int(match.group(2)), match.group(3) or "source"))
    return references


def handler_block(path: Path, line_number: int, language: str, limit_lines: int = 90) -> tuple[list[str], int, int]:
    lines = source_lines(path)
    if not lines or line_number < 1 or line_number > len(lines):
        return lines, line_number, line_number

    start = line_number
    end = min(len(lines), start + limit_lines - 1)
    if language == "python":
        for current in range(start + 1, end + 1):
            stripped = lines[current - 1].lstrip()
            if stripped.startswith("@routes."):
                end = current - 1
                break
    else:
        for current in range(start + 1, end + 1):
            stripped = lines[current - 1].lstrip()
            if stripped.startswith(("ipcMain.handle(", "ipcMain.on(")):
                end = current - 1
                break
    return lines, start, end


def enrich_backend_http_catalog() -> None:
    name = "backend-http-routes.csv"
    rows = read_rows(name)
    if not rows:
        return

    canonical_rows: dict[str, dict[str, str]] = {}
    for row in sorted(rows, key=lambda candidate: bool(clean(candidate.get("alias_of", "")))):
        source_file = row["source_file"]
        source_path = COMFYUI_ROOT / source_file
        line_number = int(first(row, "source_line", fallback="1"))
        alias_of = clean(row.get("alias_of", ""))
        canonical = canonical_rows.get(alias_of)
        if alias_of and canonical is not None:
            alias_lines, alias_start, alias_end = handler_block(source_path, line_number, "python", limit_lines=28)
            alias_excerpt = compact_excerpt(alias_lines, alias_start, alias_end)
            row["request_schema_detail"] = joined(
                f"Compatibility alias of {alias_of}.",
                canonical["request_schema_detail"],
            )
            row["response_schema_detail"] = joined(
                f"Compatibility alias of {alias_of}.",
                canonical["response_schema_detail"],
            )
            row["status_content_types"] = canonical["status_content_types"]
            row["schema_confidence"] = canonical["schema_confidence"]
            row["unresolved_schema"] = joined(
                f"The /api compatibility alias is installed by the cited route-copy loop and delegates to {alias_of}.",
                canonical["unresolved_schema"],
            )
            row["source_excerpt"] = joined(
                f"alias installation={alias_excerpt}",
                f"canonical handler={canonical['source_excerpt']}",
            )
            canonical_rows[f"{row['method']} {row['path']}"] = row
            continue
        if source_file == "openapi.yaml":
            lines = source_lines(source_path)
            excerpt = compact_excerpt(lines, max(1, line_number - 4), line_number + 28)
            row["request_schema_detail"] = joined(
                f"OpenAPI operation={first(row, 'openapi_operation_id', fallback=row['method'] + ' ' + row['path'])}",
                f"catalog body={first(row, 'request_body', fallback='none')}",
                f"path={first(row, 'path_parameters', fallback='none')}",
                f"query={first(row, 'query_parameters', fallback='none')}",
                f"exact YAML excerpt={excerpt}",
            )
            row["response_schema_detail"] = joined(
                f"documented behavior={row['success_behavior']}",
                f"exact YAML excerpt={excerpt}",
            )
            row["status_content_types"] = "Status/content types are those declared in the exact OpenAPI excerpt; no matching local runtime handler was found."
            row["schema_confidence"] = "documented-only"
            row["unresolved_schema"] = (
                f"{row['method']} {row['path']} is OpenAPI-only in this baseline; runtime payload variants, error bodies, headers, and hosted authorization behavior remain unverified."
            )
            row["source_excerpt"] = excerpt
            canonical_rows[f"{row['method']} {row['path']}"] = row
            continue

        lines, start, end = handler_block(source_path, line_number, "python")
        block = lines[max(0, start - 1):end]
        block_text = "\n".join(block)
        excerpt = compact_excerpt(lines, start, end)
        request_containers = {"request.match_info", "request.query"}
        for variable, _operation in re.findall(
            r"\b([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:await\s+)?request\.(json|post)\s*\(",
            block_text,
        ):
            request_containers.add(variable)
        request_keys = set()
        for container in request_containers:
            escaped_container = re.escape(container)
            request_keys.update(re.findall(
                rf"{escaped_container}\s*\.get\s*\(\s*[\"']([^\"']+)",
                block_text,
            ))
            request_keys.update(re.findall(
                rf"{escaped_container}\s*\[\s*[\"']([^\"']+)[\"']\s*\]",
                block_text,
            ))
        request_keys = sorted(request_keys)
        request_operations = sorted(set(re.findall(r"request\.([A-Za-z_][A-Za-z0-9_]*)", block_text)))
        returns = [clean(text) for text in block if re.search(r"\breturn\b", text)][:12]
        response_constructors = sorted(set(re.findall(
            r"web\.(json_response|FileResponse|Response|StreamResponse)\b", block_text
        )))
        statuses = sorted(set(re.findall(r"status\s*=\s*(\d{3})", block_text)))
        content_types = sorted(set(re.findall(r"content_type\s*=\s*[\"']([^\"']+)", block_text)))
        row["request_schema_detail"] = joined(
            f"catalog body={first(row, 'request_body', fallback='none')}",
            f"path={first(row, 'path_parameters', fallback='none')}",
            f"query={first(row, 'query_parameters', fallback='none')}",
            f"request operations={','.join(request_operations) if request_operations else 'none found in static handler'}",
            f"referenced keys={','.join(request_keys) if request_keys else 'none resolved statically'}",
            f"exact handler excerpt={excerpt}",
        )
        row["response_schema_detail"] = joined(
            f"constructors={','.join(response_constructors) if response_constructors else 'plain/dynamic response'}",
            f"return branches={' || '.join(returns) if returns else 'no explicit return resolved in static block'}",
            f"catalog success={row['success_behavior']}",
        )
        row["status_content_types"] = joined(
            f"explicit statuses={','.join(statuses) if statuses else 'none; successful return implies framework default 200'}",
            f"response constructors={','.join(response_constructors) if response_constructors else 'dynamic/plain'}",
            f"explicit content types={','.join(content_types) if content_types else 'constructor/framework dependent'}",
        )
        row["schema_confidence"] = "static-handler-extracted"
        row["unresolved_schema"] = (
            f"Static extraction for {row['method']} {row['path']} cannot prove dynamic Python value types, dependency-defined objects, every exception body, middleware headers, streaming boundaries, or runtime-only branches; capture them with VAL-HTTP-001."
        )
        row["source_excerpt"] = excerpt
        canonical_rows[f"{row['method']} {row['path']}"] = row

    rewrite_catalog(name, rows, list(rows[0].keys()))


def enrich_desktop_ipc_catalog() -> None:
    name = "desktop-ipc.csv"
    rows = read_rows(name)
    if not rows:
        return

    for row in rows:
        channel = row["channel"]
        references = parse_source_references(row["registration_and_use"])
        request_excerpts = []
        response_excerpts = []
        all_excerpts = []
        resolved_references = []
        typed = False
        for path, line_number, role in references:
            line_number = resolve_channel_literal_line(path, line_number, channel)
            relative = path.relative_to(REPO_ROOT) if path.is_absolute() else path
            resolved_references.append(f"{relative}:{line_number} ({role})")
            statement = statement_excerpt(path, line_number)
            labeled = f"{relative}:{line_number} ({role}) => {statement}"
            all_excerpts.append(labeled)
            typed = typed or bool(re.search(r"\b(?:string|number|boolean|Record|Promise|unknown|void|null|[A-Z][A-Za-z0-9_<>|\[\]]+)\b", statement))
            if any(token in role for token in ("invoke", "send", "main-handle", "main-on")):
                request_excerpts.append(labeled)
            if any(token in role for token in ("main-handle", "main-send", "renderer-listen", "main-reference")):
                response_excerpts.append(labeled)

        request_schema = joined(
            f"channel={channel}",
            f"mechanism={row['mechanism']}",
            f"sender/handler signatures={' || '.join(request_excerpts) if request_excerpts else 'no sender/handler signature resolved'}",
        )
        if row["mechanism"] == "request-response":
            response_schema = joined(
                f"channel={channel}",
                f"handler/consumer signatures={' || '.join(response_excerpts) if response_excerpts else 'no return signature resolved'}",
                "Promise resolution/rejection is part of the contract.",
            )
        else:
            response_schema = joined(
                f"channel={channel}",
                f"event payload/listener signatures={' || '.join(response_excerpts or request_excerpts) if (response_excerpts or request_excerpts) else 'no event signature resolved'}",
                "One-way event: no request-response result is expected.",
            )
        row["request_or_event_schema"] = request_schema
        row["response_or_callback_schema"] = response_schema
        row["schema_confidence"] = "typed-static-signature" if typed else "static-callsite-only"
        row["unresolved_schema"] = (
            f"For IPC channel {channel}, static call sites do not prove every dynamic object field, structured-clone normalization, thrown error shape, callback ordering, or version-skew branch; contract tests must capture valid, malformed, rejected, cancelled, and unsubscribe cases."
        )
        row["source_excerpts"] = " || ".join(all_excerpts) or f"No parseable source reference for {channel}."
        if resolved_references:
            row["registration_and_use"] = "; ".join(resolved_references)
        row["payload_result_schema"] = joined(request_schema, response_schema, row["unresolved_schema"])

    rewrite_catalog(name, rows, list(rows[0].keys()))


def enrich_desktop_preload_catalog() -> None:
    name = "desktop-preload-apis.csv"
    rows = read_rows(name)
    if not rows:
        return

    for row in rows:
        references = parse_source_references(row["source"])
        signatures = []
        resolved_sources = []
        typed = False
        for path, line_number, role in references:
            line_number = resolve_named_member_line(path, line_number, row["member"])
            relative = path.relative_to(REPO_ROOT) if path.is_absolute() else path
            resolved_sources.append(f"{relative}:{line_number}")
            signature = statement_excerpt(path, line_number)
            signatures.append(f"{relative}:{line_number} ({role}) => {signature}")
            typed = typed or bool(re.search(r"(?:Promise<|\):|:\s*[A-Za-z_{]|\bvoid\b)", signature))
        exact = " || ".join(signatures) or f"No parseable TypeScript declaration at {row['source']}."
        row["source_signature"] = exact
        if resolved_sources:
            row["source"] = "; ".join(resolved_sources)
        row["signature_confidence"] = "typed-static-signature" if typed else "static-declaration-only"
        row["unresolved_schema"] = (
            f"The static declaration for {row['surface']}.{row['member']} does not prove runtime structured-clone values, rejected-error serialization, callback timing/unsubscribe cleanup, or version skew; validate those through the mapped IPC channel and VAL-DESKTOP-001."
        )
        row["contract"] = joined(
            f"Exact source signature: {exact}",
            "Callback members return the source-declared unsubscribe/result type; properties and calls preserve the declared argument and return types.",
            row["unresolved_schema"],
        )

    rewrite_catalog(name, rows, list(rows[0].keys()))


def normalize_source_catalog_corrections() -> None:
    frontend_feature_name = "frontend-features.csv"
    frontend_feature_rows = read_rows(frontend_feature_name)
    frontend_feature_changed = False
    desktop_route_contracts = {
        "COMFY-UI-043": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/DesktopDialogView.vue",
            "DesktopDialogView",
            "Resolve dialogId through the Desktop dialog registry, render localized title/message and source-ordered buttons, and send the chosen returnValue through electronAPI().Dialog.clickButton.",
            "An unknown or malformed dialog identifier follows the dialog registry's explicit fallback/error; bridge rejection leaves the dialog open and must surface a recoverable action error without inventing a return value.",
        ),
        "COMFY-UI-044": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/DesktopStartView.vue",
            "DesktopStartView",
            "Render the localized initialising StartupDisplay while the Desktop shell owns startup and the next navigation decision.",
            "Startup failure and retry are owned by the Desktop lifecycle bridge; the waiting route must not navigate or claim readiness on its own.",
        ),
        "COMFY-UI-045": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/DesktopUpdateView.vue",
            "DesktopUpdateView",
            "Render localized update progress with a spinner and keyboard-operable console-log drawer; dispose the Desktop validation subscription when the route unmounts.",
            "Update errors remain visible through Desktop progress/terminal output; closing the drawer does not cancel the update, and unmount cleanup prevents stale validation updates.",
        ),
        "COMFY-UI-046": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/DownloadGitView.vue",
            "DownloadGitView",
            "Explain the missing Git prerequisite, open the platform Git download destination on request, or let the user skip and navigate to the install route.",
            "External-open failure remains on the page with recovery; skipping performs no Git installation and proceeds only through the explicit install navigation.",
        ),
        "COMFY-UI-047": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/InstallView.vue",
            "InstallView",
            "Require every install step to be visited, collect install path, device/GPU and Desktop settings, validate path state, track step changes, invoke installComfyUI once, then navigate to manual configuration for unsupported/custom setup or server start otherwise.",
            "Invalid path or incomplete selections prevent installation; bridge failure retains entered choices and exposes the source error; repeated activation must not start duplicate installs.",
        ),
        "COMFY-UI-048": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/MaintenanceView.vue",
            "MaintenanceView",
            "Project maintenance task status, all/error filters, unsafe-migration reason, terminal drawer and validation refresh; continue only after the refreshed task set contains no unresolved error.",
            "Refresh/validation errors keep the user on maintenance, expose localized error/toast and terminal evidence, and do not mark maintenance complete; unsafe migration requires its explicit action.",
        ),
        "COMFY-UI-049": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/ManualConfigurationView.vue",
            "ManualConfigurationView",
            "Show localized custom Python/venv requirements and the virtual-environment path, then relaunch only from the explicit Manual configuration complete action.",
            "Missing manual prerequisites or relaunch rejection retain the instructions and path; the route never asserts that the environment was configured automatically.",
        ),
        "COMFY-UI-050": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/MetricsConsentView.vue",
            "MetricsConsentView",
            "Show current metrics consent, privacy-policy link and enable/disable choice; persist through the Desktop settings bridge and return to the root welcome flow.",
            "A consent write failure emits the localized error toast, preserves the prior effective value, and must not emit analytics under the requested but uncommitted choice.",
        ),
        "COMFY-UI-051": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/NotSupportedView.vue",
            "NotSupportedView",
            "Explain unsupported hardware/platform status and offer documentation, issue reporting, or an explicit continue-to-install path.",
            "External-open/report failure retains the page; continuing does not relabel unsupported hardware as supported and navigates only after explicit user activation.",
        ),
        "COMFY-UI-052": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/ServerStartView.vue",
            "ServerStartView",
            "Subscribe to Desktop progress and install-stage updates, render stage/message/percentage and terminal output, distinguish critical error state, and expose report issue, logs and troubleshooting actions.",
            "Install/start errors remain in an actionable error state with logs; listener cleanup prevents late updates after navigation, and retry/relaunch occurs only through the owning Desktop action.",
        ),
        "COMFY-UI-053": (
            "apps/desktop-ui/src/router.ts; apps/desktop-ui/src/views/WelcomeView.vue",
            "WelcomeViewAlias",
            "Render the localized first-run welcome at /welcome and navigate to /install only from the explicit primary action; the empty child / route is the equivalent canonical WelcomeView entry.",
            "Lazy-route failure preserves first-run state and surfaces a Desktop load error; repeated navigation does not duplicate an installation operation.",
        ),
    }
    directly_tested_extension_members = {
        "COMFY-FRONTEXT-EXT-038",  # commands
        "COMFY-FRONTEXT-EXT-039",  # keybindings
        "COMFY-FRONTEXT-EXT-040",  # menuCommands and command ownership
        "COMFY-FRONTEXT-EXT-041",  # settings and disabled attributes
        "COMFY-FRONTEXT-EXT-043",  # aboutPageBadges
        "COMFY-FRONTEXT-EXT-050",  # getSelectionToolboxCommands
    }
    for row in frontend_feature_rows:
        corrected_feature_flag_path = re.sub(
            r"clientFeatureFlags\.(?:js|json(?:on)*)\b",
            "clientFeatureFlags.json",
            row["source_file"],
        )
        if corrected_feature_flag_path != row["source_file"]:
            row["source_file"] = corrected_feature_flag_path
            frontend_feature_changed = True

        route_contract = desktop_route_contracts.get(row["feature_id"])
        if route_contract is not None:
            source_file, symbol, success_behavior, error_recovery = route_contract
            route_updates = {
                "source_file": source_file,
                "symbol": symbol,
                "success_behavior": success_behavior,
                "error_recovery": error_recovery,
                "preconditions": "The Desktop UI router, context-isolated bridge, owning lifecycle state, and route-specific data are available.",
                "interaction_accessibility": "The route exposes localized headings/status, logical tab order, keyboard-operable actions, visible focus, accessible names/roles, and focus restoration after drawers, dialogs, toasts, or external-open failure.",
                "state_concurrency": "Route-local subscriptions and asynchronous actions are owned until unmount, cleaned exactly once, and reject stale or duplicate bridge updates.",
                "interfaces_side_effects": f"Vue Router Desktop route and {symbol}; privileged effects cross only the typed Desktop preload bridge.",
                "platform_localization_variants": "Desktop-only; file protocol uses hash history and hosted/non-file builds use base-aware history; visible text is localized.",
            }
            for field, value in route_updates.items():
                if row[field] != value:
                    row[field] = value
                    frontend_feature_changed = True

        match = re.fullmatch(r"COMFY-FRONTEXT-EXT-(\d{3})", row["feature_id"])
        if not match or not 37 <= int(match.group(1)) <= 63:
            continue

        corrected_test = ""
        if row["feature_id"] in directly_tested_extension_members:
            corrected_test = "browser_tests/tests/extensionAPI.spec.ts"
            corrected_evidence = "test-backed"
        else:
            corrected_test = "No focused existing test was located for this interface member."
            corrected_evidence = "code-inferred"

        if row["test"] != corrected_test or row["evidence_level"] != corrected_evidence:
            row["test"] = corrected_test
            row["evidence_level"] = corrected_evidence
            frontend_feature_changed = True

    if frontend_feature_changed:
        rewrite_catalog(
            frontend_feature_name,
            frontend_feature_rows,
            list(frontend_feature_rows[0].keys()),
        )

    frontend_route_name = "frontend-routes.csv"
    frontend_route_rows = read_rows(frontend_route_name)
    frontend_route_changed = False
    for row in frontend_route_rows:
        route_contract = desktop_route_contracts.get(row["feature_id"])
        if route_contract is None:
            continue
        source_file, _symbol, success_behavior, _error_recovery = route_contract
        if row["source_file"] != source_file or row["behavior"] != success_behavior:
            row["source_file"] = source_file
            row["behavior"] = success_behavior
            frontend_route_changed = True
    if frontend_route_changed:
        rewrite_catalog(frontend_route_name, frontend_route_rows, list(frontend_route_rows[0].keys()))

    frontend_flag_name = "frontend-feature-flags.csv"
    frontend_flag_rows = read_rows(frontend_flag_name)
    frontend_flag_changed = False
    for row in frontend_flag_rows:
        corrected_feature_flag_path = re.sub(
            r"clientFeatureFlags\.(?:js|json(?:on)*)\b",
            "clientFeatureFlags.json",
            row["source_file"],
        )
        if corrected_feature_flag_path != row["source_file"]:
            row["source_file"] = corrected_feature_flag_path
            frontend_flag_changed = True
    if frontend_flag_changed:
        rewrite_catalog(
            frontend_flag_name,
            frontend_flag_rows,
            list(frontend_flag_rows[0].keys()),
        )

    frontend_source_name = "frontend-source-files.csv"
    frontend_source_rows = read_rows(frontend_source_name)
    frontend_source_changed = False
    frontend_config_pattern = re.compile(
        r"(^|/)(?:astro\.config\.|eslint\.config\.|knip\.config\.|"
        r"playwright(?:\.[^.]+)?\.config\.|tsconfig(?:\.[^.]+)?\.json$|"
        r"vite(?:\.[^.]+)?\.config\.|vitest(?:\.[^.]+)?(?:\.config)?\.|vitest\.setup\.)",
        re.IGNORECASE,
    )
    for row in frontend_source_rows:
        source_file = row["source_file"]
        corrected_classification = ""
        corrected_reason = ""
        if source_file.startswith("browser_tests/"):
            if source_file.endswith(".md"):
                corrected_classification = "documented-only"
                corrected_reason = "Browser-test guidance/documentation; not executable evidence by itself."
            else:
                corrected_classification = "test-only"
                corrected_reason = (
                    "Playwright test, harness, fixture, workflow/media sample, snapshot, or test configuration; "
                    "retained as test evidence and never counted as production behavior."
                )
        elif source_file.startswith(".storybook/"):
            corrected_classification = "infrastructure-only"
            corrected_reason = "Storybook configuration/build support; consuming stories remain test-only evidence."
        elif source_file == "apps/website/e2e/viewports.ts":
            corrected_classification = "test-only"
            corrected_reason = "Website end-to-end viewport fixture; not Cloud production behavior."
        elif not source_file.endswith(".md") and frontend_config_pattern.search(source_file):
            corrected_classification = "infrastructure-only"
            corrected_reason = (
                "Lint, format, typecheck, test-runner, package, or build configuration; "
                "not an independently observable production capability."
            )

        if corrected_classification and row["classification"] != corrected_classification:
            row["classification"] = corrected_classification
            row["reason"] = corrected_reason
            frontend_source_changed = True
    if frontend_source_changed:
        rewrite_catalog(
            frontend_source_name,
            frontend_source_rows,
            list(frontend_source_rows[0].keys()),
        )

    backend_feature_name = "backend-features.csv"
    backend_feature_rows = read_rows(backend_feature_name)
    backend_feature_changed = False
    for row in backend_feature_rows:
        if row["feature_id"] == "COMFY-EXT-028":
            corrected = "comfy_api_nodes/util/upload_helpers.py; comfy_api_nodes/util/download_helpers.py"
            if row["source_evidence"] != corrected or row["protocols_dependencies"] != corrected:
                row["source_evidence"] = corrected
                row["protocols_dependencies"] = corrected
                backend_feature_changed = True
    if backend_feature_changed:
        rewrite_catalog(backend_feature_name, backend_feature_rows, list(backend_feature_rows[0].keys()))

    backend_format_name = "backend-formats.csv"
    backend_format_rows = read_rows(backend_format_name)
    backend_format_changed = False
    format_sources = {
        "COMFY-FORMAT-008": "comfy_api/latest/_input/basic_types.py:ImageInput; comfy_api/latest/_io.py:Image; nodes.py:LoadImage",
        "COMFY-FORMAT-009": "comfy_api/latest/_input/basic_types.py:MaskInput; comfy_api/latest/_io.py:Mask; nodes.py:LoadImage/LoadImageMask",
        "COMFY-FORMAT-010": "nodes.py; comfy/latent_formats.py; comfy_api/latest/_input/basic_types.py:LatentInput; comfy_api/latest/_io.py:Latent",
        "COMFY-FORMAT-011": "comfy_api/latest/_input/basic_types.py:AudioInput; comfy_api/latest/_io.py:Audio; comfy_api/latest/_ui.py:AudioSaveHelper",
        "COMFY-FORMAT-014": "comfy_api/latest/_util/geometry_types.py:File3D; comfy_api/latest/_io.py:Voxel/Mesh/Splat/File3D*; comfy_extras/nodes_load_3d.py; comfy_extras/nodes_save_3d.py",
    }
    for row in backend_format_rows:
        corrected = format_sources.get(row["feature_id"])
        if corrected is not None and row["source_evidence"] != corrected:
            row["source_evidence"] = corrected
            backend_format_changed = True
    if backend_format_changed:
        rewrite_catalog(backend_format_name, backend_format_rows, list(backend_format_rows[0].keys()))

    backend_source_name = "backend-source-coverage.csv"
    backend_source_rows = read_rows(backend_source_name)
    backend_source_changed = False
    source_feature_additions = {
        "comfy_api_nodes/util/upload_helpers.py": {"COMFY-EXT-028"},
        "comfy_api/latest/_input/basic_types.py": {"COMFY-FORMAT-008", "COMFY-FORMAT-009", "COMFY-FORMAT-010", "COMFY-FORMAT-011"},
        "comfy_api/latest/_io.py": {"COMFY-FORMAT-008", "COMFY-FORMAT-009", "COMFY-FORMAT-010", "COMFY-FORMAT-011", "COMFY-FORMAT-014"},
        "comfy_api/latest/_util/geometry_types.py": {"COMFY-FORMAT-014"},
    }
    for row in backend_source_rows:
        additions = source_feature_additions.get(row["source_file"])
        if not additions:
            continue
        existing = {part.strip() for part in row["mapped_feature_ids"].split("|") if part.strip()}
        corrected_ids = " | ".join(sorted(existing | additions))
        if row["mapped_feature_ids"] != corrected_ids:
            row["mapped_feature_ids"] = corrected_ids
            row["reason"] = "Mapped to feature IDs through direct source evidence."
            if row["classification"] == "infrastructure-only":
                row["classification"] = "production source"
            backend_source_changed = True
    if backend_source_changed:
        rewrite_catalog(backend_source_name, backend_source_rows, list(backend_source_rows[0].keys()))

    backend_reconciliation_path = CATALOGS / "backend-reconciliation.json"
    backend_reconciliation = json.loads(backend_reconciliation_path.read_text(encoding="utf-8"))
    source_classification = dict(sorted(Counter(
        row["classification"] for row in backend_source_rows
    ).items()))
    source_mapped = sum(bool(clean(row["mapped_feature_ids"])) for row in backend_source_rows)
    if (
        backend_reconciliation.get("source_coverage_classification") != source_classification
        or backend_reconciliation.get("source_coverage_mapped") != source_mapped
        or backend_reconciliation.get("source_coverage_rows") != len(backend_source_rows)
    ):
        backend_reconciliation["source_coverage_classification"] = source_classification
        backend_reconciliation["source_coverage_mapped"] = source_mapped
        backend_reconciliation["source_coverage_rows"] = len(backend_source_rows)
        backend_reconciliation_path.write_text(
            json.dumps(backend_reconciliation, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    config_name = "backend-config.csv"
    config_rows = read_rows(config_name)
    config_changed = False
    for row in config_rows:
        if row["feature_id"] in {"COMFY-CONFIG-0102", "COMFY-CONFIG-0103"}:
            observation = (
                "Observed by python3 main.py --list-feature-flags (exit 0) on 2026-07-12: "
                f"{row['name']}=false."
            )
            if row["evidence_level"] != "observed" or row["runtime_observation"] != observation:
                row["evidence_level"] = "observed"
                row["runtime_observation"] = observation
                config_changed = True
    if config_changed:
        rewrite_catalog(config_name, config_rows, [
            "kind", "name", "default", "choices", "value_shape", "mutual_exclusion_group",
            "availability", "evidence_level", "confidence", "behavior", "source_file",
            "source_symbol", "source_line", "test_evidence", "runtime_observation", "sim_status",
            "parity_gap", "feature_id",
        ])

    node_name = "backend-nodes.csv"
    node_rows = read_rows(node_name)
    node_changed = False
    category_corrections = {
        "COMFY-NODE-0672": "text",
        "COMFY-NODE-0757": "(empty root category declared by source)",
    }
    text_generate_parent = next(row for row in node_rows if row["feature_id"] == "COMFY-NODE-0671")
    for row in node_rows:
        corrected = category_corrections.get(row["feature_id"])
        if corrected is not None and row["category"] != corrected:
            row["category"] = corrected
            node_changed = True
        if row["feature_id"] == "COMFY-NODE-0672":
            for field in ("inputs", "outputs", "input_is_list", "output_is_list"):
                if row[field] != text_generate_parent[field]:
                    row[field] = text_generate_parent[field]
                    node_changed = True
    if node_changed:
        rewrite_catalog(node_name, node_rows, [
            "node_identifier", "class_name", "display_name", "category", "product",
            "classification", "availability", "evidence_level", "confidence", "schema_api",
            "schema_source", "inputs", "outputs", "input_is_list", "output_is_list",
            "lazy_inputs", "output_node", "execution_function", "validation", "caching",
            "change_detection", "execution_blocking", "error_behavior", "source_file",
            "source_symbol", "source_line", "test_evidence", "registration_evidence", "feature_id",
        ])

    desktop_config_name = "desktop-cli-environment.csv"
    desktop_config_rows = read_rows(desktop_config_name)
    desktop_config_changed = False
    for row in desktop_config_rows:
        if row["kind"] == "CLI-flag" and (
            "/renderer/" in row["source"] or row["name"] in {"--comfy-menu-bg", "--descrip-text"}
        ):
            row["kind"] = "CSS custom property"
            row["behavior"] = (
                "Renderer CSS custom property consumed at the cited Vue/CSS boundary; "
                "it affects theme, layout, terminal, title-bar, menu, or chooser presentation rather than child-process argv."
            )
            desktop_config_changed = True
    if desktop_config_changed:
        rewrite_catalog(desktop_config_name, desktop_config_rows, [
            "kind", "name", "availability", "feature_id", "default", "behavior", "source", "evidence_level"
        ])

    desktop_menu_name = "desktop-menu-actions.csv"
    desktop_menu_rows = read_rows(desktop_menu_name)
    desktop_menu_changed = False
    for row in desktop_menu_rows:
        if row["evidence_level"] != "code-inferred":
            row["evidence_level"] = "code-inferred"
            desktop_menu_changed = True
    if desktop_menu_changed:
        rewrite_catalog(desktop_menu_name, desktop_menu_rows, list(desktop_menu_rows[0].keys()))

    backend_test_name = "backend-tests.csv"
    backend_test_rows = read_rows(backend_test_name)
    backend_test_changed = False
    for row in backend_test_rows:
        if clean(row["mapped_feature_ids"]):
            continue
        source_file = row["source_file"]
        if source_file.endswith("multicombo_serialization_test.py"):
            row["mapped_feature_ids"] = "COMFY-EXEC-012 | COMFY-EXT-016"
            row["classification"] = "existing parity test evidence for combo/multiselect validation and V3-to-V1 schema serialization"
            backend_test_changed = True
        elif source_file.endswith("utils/json_util_test.py"):
            row["mapped_feature_ids"] = "COMFY-EXT-012"
            row["classification"] = "existing infrastructure test evidence for custom-node localization recursive JSON merge"
            backend_test_changed = True
    if backend_test_changed:
        rewrite_catalog(backend_test_name, backend_test_rows, [
            "test_id", "source_file", "test_class", "test_symbol", "source_line", "async",
            "decorators_parametrization", "mapped_feature_ids", "classification",
        ])

    enrich_backend_http_catalog()
    enrich_desktop_ipc_catalog()
    enrich_desktop_preload_catalog()
    generate_frontend_functional_modules()
    from generate_frontend_component_surfaces import augment_source_ledger

    augment_source_ledger()


def stable_id(prefix: str, *parts: object) -> str:
    identity = "\x1f".join(clean(part).casefold() for part in parts)
    suffix = hashlib.sha256(identity.encode("utf-8")).hexdigest()[:12].upper()
    return f"{prefix}-{suffix}"


FRONTEND_FUNCTIONAL_MODULE_FIELDS = [
    "feature_id",
    "product",
    "domain",
    "module",
    "primary_symbol",
    "exported_symbols",
    "classification",
    "disposition",
    "availability",
    "source_availability",
    "evidence_level",
    "confidence",
    "source_evidence",
    "test_evidence",
    "broad_anchor",
    "candidate_predicate",
    "actor",
    "trigger",
    "preconditions",
    "inputs_defaults_flags",
    "observable_success",
    "state_concurrency",
    "error_cancel_retry_recovery",
    "persistence_serialization",
    "interfaces_side_effects",
    "platform_cloud_variants",
    "independent_validation",
    "infrastructure_disposition",
    "source_sha256",
]


# These modules are deliberately retained in the semantic-granularity ledger even
# though they do not own a user workflow. Each disposition names the source-specific
# contract that consuming capabilities must preserve and test.
FRONTEND_FUNCTIONAL_INFRASTRUCTURE = {
    "src/composables/billing/types.ts": (
        "Type-only billing adapter contract for BillingType, subscription/balance state, "
        "and asynchronous BillingActions; it emits no JavaScript and is validated through its adapters."
    ),
    "src/composables/boundingBoxes/boundingBoxesUtil.ts": (
        "Pure normalized-region geometry and hit-testing primitives; bounding-box interaction owns the observable workflow."
    ),
    "src/composables/maskeditor/brushDrawingUtils.ts": (
        "Reusable Canvas2D brush, dirty-rectangle, premultiplication, and bounded texture-cache primitives; the mask editor owns interaction state."
    ),
    "src/composables/maskeditor/brushUtils.ts": (
        "Pure brush-size and hardness calculations with no state, I/O, or independently reachable surface."
    ),
    "src/composables/maskeditor/gpu/brushShaders.ts": (
        "TypeGPU-resolved vertex, fragment, and blit shader definitions consumed by the GPU brush renderer; it owns no editor lifecycle."
    ),
    "src/composables/maskeditor/gpu/gpuSchema.ts": (
        "TypeGPU struct declarations for brush uniforms and stroke-point instance data; behavior is exercised by the GPU renderer."
    ),
    "src/composables/maskeditor/gpuUtils.ts": (
        "Pure dirty-rectangle clamping and stroke-point resampling primitives consumed by GPU mask drawing."
    ),
    "src/composables/maskeditor/panZoomUtils.ts": (
        "Pure fit-view, clamped zoom, focal zoom, drag-pan, touch-pan, and pinch calculations consumed by mask-editor gestures."
    ),
    "src/composables/maskeditor/splineUtils.ts": (
        "Pure Catmull-Rom interpolation and fixed-spacing curve resampling primitives consumed by brush-stroke processing."
    ),
    "src/composables/node/canvasImagePreviewTypes.ts": (
        "Canvas-preview widget identifier, supported-node set, and compatibility predicate consumed by node preview rendering."
    ),
    "src/platform/assets/composables/media/IAssetsProvider.ts": (
        "Type-only provider interface for media, loading/error state, pagination, fetch, refresh, and load-more operations."
    ),
    "src/platform/assets/composables/media/assetMappers.ts": (
        "Deterministic queue/file-to-AssetItem mapping and URL construction primitives; provider and asset-browser flows own I/O and state."
    ),
    "src/renderer/extensions/vueNodes/widgets/composables/domWidgetTestUtils.ts": (
        "Vitest-only fake DOM/media widget node factories using vi.fn; it is test plumbing despite its production-source manifest classification."
    ),
}


def frontend_functional_domain(source_file: str, broad_anchor: str) -> str:
    key = source_file.casefold()
    if any(token in key for token in ("queue", "job", "execution", "resultgallery")):
        return "queue-execution-state"
    if any(token in key for token in ("workflow", "template", "builder", "appmode", "sharing")):
        return "workflow-lifecycle-sharing"
    if any(token in key for token in ("auth", "oauth", "turnstile", "secret")):
        return "authentication-secrets"
    if any(token in key for token in ("billing", "subscription", "pricing", "credit", "topup")) or "/platform/workspace/" in key:
        return "cloud-billing-workspace"
    if "/workbench/extensions/manager/" in key or any(
        token in key for token in ("nodepack", "registry", "comfyregistry")
    ):
        return "frontend-extension-manager"
    if any(token in key for token in ("mask", "painter", "bounding", "asset", "media", "image", "audio", "load3d", "hdr", "glsl")):
        return "assets-specialized-media"
    if any(token in key for token in ("node", "graph", "canvas", "minimap", "layout", "widget", "subgraph", "slot", "linear")):
        return "graph-editing-widgets"
    if any(token in key for token in ("setting", "keybinding", "palette", "shortcut")):
        return "settings-keybindings"
    if any(token in key for token in ("terminal", "log", "diagnostic", "error")):
        return "diagnostics-errors"
    if any(token in key for token in ("update", "release", "version")):
        return "updates-versioning"
    if source_file.startswith("apps/website/"):
        return "website-interaction"
    if broad_anchor.startswith("COMFY-CLOUD-"):
        return "cloud-account-state"
    return "application-ui-state"


def frontend_functional_product(source_file: str) -> str:
    if source_file.startswith("apps/desktop-ui/"):
        return "ComfyUI-Frontend desktop-ui"
    if source_file.startswith("apps/website/"):
        return "ComfyUI-Frontend website"
    return "ComfyUI-Frontend"


def humanize_typescript_symbol(symbol: str) -> str:
    text = re.sub(r"^(?:use)", "", symbol)
    text = re.sub(r"(?:Store|Service|Manager)$", "", text)
    text = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", text)
    text = re.sub(r"[_-]+", " ", text)
    return clean(text).casefold()


def ordered_unique(values: list[str], limit: int = 12) -> list[str]:
    result = []
    for value in values:
        value = clean(value)
        if value and value not in result:
            result.append(value)
        if len(result) >= limit:
            break
    return result


def frontend_exported_symbols(source_text: str, source_file: str) -> list[str]:
    symbols = re.findall(
        r"\bexport\s+(?:default\s+)?(?:declare\s+)?(?:async\s+)?"
        r"(?:function|class|const|let|var|interface|type|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        source_text,
    )
    for block in re.findall(r"\bexport\s*\{([^}]+)\}", source_text, re.DOTALL):
        for item in block.split(","):
            symbol = clean(item).split(" as ")[-1].strip()
            if re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*", symbol):
                symbols.append(symbol)
    return ordered_unique(symbols, limit=24) or [Path(source_file).stem]


def frontend_primary_symbol(exports: list[str], source_file: str, source_text: str) -> str:
    stem = Path(source_file).stem
    for symbol in exports:
        if symbol.casefold() == stem.casefold():
            return symbol
    for symbol in exports:
        symbol_lower = symbol.casefold()
        stem_lower = stem.casefold()
        if symbol_lower == f"use{stem_lower}" or stem_lower in symbol_lower:
            return symbol
    exported_runtime_symbols = re.findall(
        r"\bexport\s+(?:default\s+)?(?:async\s+)?(?:function|class)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        source_text,
    )
    return first(
        {"symbol": next((symbol for symbol in exported_runtime_symbols if symbol in exports), "")},
        "symbol",
        fallback=exports[0],
    )


def frontend_module_test_evidence(source_file: str) -> list[str]:
    path = FRONTEND_ROOT / source_file
    candidates = []
    for suffix in (".test.ts", ".spec.ts"):
        sibling = path.with_name(f"{path.stem}{suffix}")
        if sibling.exists():
            candidates.append(sibling.relative_to(FRONTEND_ROOT).as_posix())
        nested = path.parent / "__tests__" / f"{path.stem}{suffix}"
        if nested.exists():
            candidates.append(nested.relative_to(FRONTEND_ROOT).as_posix())
    return ordered_unique(candidates, limit=6)


def frontend_module_source_signals(source_text: str) -> dict[str, list[str]]:
    state_tokens = [
        token for token in
        ("defineStore", "ref(", "shallowRef(", "reactive(", "computed(", "watch(", "watchEffect(", "provide(", "inject(")
        if token in source_text
    ]
    concurrency_tokens = [
        token for token in
        ("async ", "Promise", "await ", "AbortController", "setTimeout", "setInterval", "requestAnimationFrame", "onMounted", "onUnmounted", "subscribe", "WebSocket")
        if token in source_text
    ]
    failure_tokens = [
        token for token in
        ("try {", "catch (", ".catch(", "finally", "throw ", "onError", "retry", "AbortController", "cancel", "toast.add")
        if token in source_text
    ]
    persistence_tokens = [
        token for token in
        ("localStorage", "sessionStorage", "useStorage", "useLocalStorage", "useSessionStorage", "indexedDB")
        if token in source_text
    ]
    effect_token_map = (
        ("fetch(", "window fetch"),
        ("api.fetchApi", "Comfy HTTP client"),
        ("api.apiURL", "Comfy URL construction"),
        ("axios", "Axios HTTP client"),
        ("router.push", "router push"),
        ("router.replace", "router replace"),
        ("window.location", "browser navigation"),
        ("navigator.clipboard", "system clipboard"),
        ("addEventListener", "event listener registration"),
        ("removeEventListener", "event listener removal"),
        ("toast.add", "toast notification"),
        ("useDialogService", "dialog service"),
        ("trackEvent", "telemetry event"),
        ("HTMLCanvasElement", "Canvas2D rendering"),
        ("GPU", "GPU/WebGPU resource"),
        ("AudioContext", "Web Audio context"),
        ("MediaRecorder", "media recording"),
        ("URL.createObjectURL", "object URL allocation"),
        ("URL.revokeObjectURL", "object URL release"),
    )
    effects = [label for token, label in effect_token_map if token in source_text]
    flags = ordered_unique(
        re.findall(r"\b(?:is|enable|disable|supports|has)[A-Z][A-Za-z0-9_]*", source_text)
        + re.findall(r"\bfeatureFlags?\.?[A-Za-z0-9_]*", source_text, re.IGNORECASE),
        limit=10,
    )
    return {
        "state": ordered_unique(state_tokens),
        "concurrency": ordered_unique(concurrency_tokens),
        "failure": ordered_unique(failure_tokens),
        "persistence": ordered_unique(persistence_tokens),
        "effects": ordered_unique(effects),
        "flags": flags,
    }


def generate_frontend_functional_modules(augment_source_mapping: bool = True) -> list[dict[str, str]]:
    """Generate source-specific contracts for broad-anchor-only functional TS modules.

    Predicate: normalized production/cloud/paid/platform-specific `.ts` source; a
    `composable(s)`, `store(s)`, or `service(s)` directory segment; after removing
    this ledger's own IDs, exactly the source manifest's primary broad anchor; and
    no exact source-path evidence in another frontend catalog. Normalized tests,
    types/config/build inputs outside these source directories never enter the set.
    The explicit infrastructure table retains helpers/type/test plumbing that the
    path predicate finds so their exclusion from feature counts stays auditable.
    """
    source_rows = read_rows("frontend-source-files.csv")
    direct_evidence = "\n".join(
        path.read_text(encoding="utf-8", errors="replace")
        for path in sorted(CATALOGS.glob("frontend-*.csv"))
        if path.name not in {"frontend-source-files.csv", "frontend-functional-modules.csv"}
    )
    anchor_rows = {row["feature_id"]: row for row in read_rows("frontend-features.csv")}
    family_segments = {"composable", "composables", "store", "stores", "service", "services"}
    candidates = []
    for source_row in source_rows:
        source_file = source_row["source_file"]
        mapped_ids = [
            feature_id.strip()
            for feature_id in source_row["feature_ids"].split("|")
            if feature_id.strip() and not feature_id.strip().startswith("COMFY-FRONTMOD-")
        ]
        path_segments = set(Path(source_file).parts)
        if not (
            source_row["classification"] in {"production", "cloud/paid", "platform-specific"}
            and source_file.endswith(".ts")
            and path_segments.intersection(family_segments)
            and mapped_ids == [source_row["primary_anchor"]]
            and source_file not in direct_evidence
        ):
            continue
        candidates.append(source_row)

    rows = []
    for source_row in sorted(candidates, key=lambda row: row["source_file"]):
        source_file = source_row["source_file"]
        source_path = FRONTEND_ROOT / source_file
        source_bytes = source_path.read_bytes()
        source_text = source_bytes.decode("utf-8", errors="replace")
        source_digest = hashlib.sha256(source_bytes).hexdigest()
        broad_anchor = source_row["primary_anchor"]
        exports = frontend_exported_symbols(source_text, source_file)
        primary_symbol = frontend_primary_symbol(exports, source_file, source_text)
        source_line = next(
            (
                line_number
                for line_number, line in enumerate(source_text.splitlines(), 1)
                if re.search(rf"\b{re.escape(primary_symbol)}\b", line)
            ),
            1,
        )
        tests = frontend_module_test_evidence(source_file)
        signals = frontend_module_source_signals(source_text)
        imports = ordered_unique(
            re.findall(r"\bfrom\s+['\"]([^'\"]+)['\"]", source_text),
            limit=8,
        )
        declared_functions = re.findall(
            r"\b(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)",
            source_text,
        )
        arrow_functions = re.findall(
            r"\b(?:const|let)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s+)?"
            r"(?:\([^=]{0,500}\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>",
            source_text,
            re.DOTALL,
        )
        action_names = ordered_unique(
            [
                name
                for name in declared_functions + arrow_functions
                if re.match(
                    r"(?:load|fetch|save|set|clear|add|remove|delete|update|toggle|open|close|create|cancel|retry|refresh|submit|select|move|resize|start|stop|handle|apply|restore|switch|publish|upload|download|initialize|execute|run|navigate|show|hide|copy|paste|sync|compute|calculate|parse|map)",
                    name,
                    re.IGNORECASE,
                )
            ],
            limit=12,
        )
        signature_lines = ordered_unique(
            [
                clean(line)
                for line in source_text.splitlines()
                if re.search(
                    r"\bexport\s+(?:default\s+)?(?:declare\s+)?(?:async\s+)?(?:function|class|const|let|var|interface|type|enum)\b",
                    line,
                )
            ],
            limit=8,
        )
        store_ids = ordered_unique(
            re.findall(r"defineStore\(\s*['\"]([^'\"]+)['\"]", source_text),
            limit=4,
        )
        route_literals = ordered_unique(
            [
                value
                for value in re.findall(r"['\"](/[^'\"\s]{2,120})['\"]", source_text)
                if any(token in value for token in ("api", "user", "workflow", "queue", "asset", "subscription", "billing", "workspace", "view"))
            ],
            limit=8,
        )
        infrastructure_reason = FRONTEND_FUNCTIONAL_INFRASTRUCTURE.get(source_file, "")
        disposition = "infrastructure-only" if infrastructure_reason else "functional-capability"
        if source_row["classification"] in {"cloud/paid", "platform-specific"}:
            source_availability = source_row["classification"]
        elif broad_anchor.startswith("COMFY-CLOUD-"):
            source_availability = "cloud/paid"
        else:
            source_availability = first(
                anchor_rows.get(broad_anchor, {}),
                "availability",
                fallback=source_row["classification"],
            )
        availability = "infrastructure-only" if infrastructure_reason else source_availability
        module_words = humanize_typescript_symbol(Path(source_file).stem)
        if "/stores/" in f"/{source_file}" or source_file.endswith("Store.ts"):
            behavior_verb = f"own and transition {module_words} state"
        elif "/services/" in f"/{source_file}" or source_file.endswith("Service.ts"):
            behavior_verb = f"perform {module_words} service operations"
        else:
            behavior_verb = f"provide {module_words} behavior"
        exact_contract = joined(
            f"exports={', '.join(exports)}",
            f"actions={', '.join(action_names)}" if action_names else "actions=no separately named action helper",
            f"store_ids={', '.join(store_ids)}" if store_ids else "store_ids=none",
            f"effects={', '.join(signals['effects'])}" if signals["effects"] else "effects=returned values or reactive state only",
        )
        if infrastructure_reason:
            observable_success = (
                f"A consuming {broad_anchor} capability imports {', '.join(exports)} and receives the exact source-defined "
                f"types, pure values, mapping result, shader/schema object, or test fake without this module owning a separate user state transition. {exact_contract}."
            )
        else:
            observable_success = (
                f"{primary_symbol} shall {behavior_verb}; its independently testable contract exposes {', '.join(exports)}"
                f"{'; named transitions ' + ', '.join(action_names) if action_names else ''}. {exact_contract}."
            )
        if signals["state"] or signals["concurrency"]:
            state_concurrency = joined(
                f"state primitives={', '.join(signals['state'])}" if signals["state"] else "state primitives=plain TypeScript values",
                f"async/lifetime primitives={', '.join(signals['concurrency'])}" if signals["concurrency"] else "async/lifetime primitives=none",
                "The caller must reject stale completions and dispose watchers/listeners/tasks at the same source-defined lifecycle boundary.",
            )
        else:
            state_concurrency = (
                f"{primary_symbol} is synchronous at this module boundary; repeated calls are independently testable and the module declares no watcher, timer, subscription, or asynchronous owner."
            )
        if signals["failure"]:
            failure_recovery = (
                f"Source branches or propagation primitives: {', '.join(signals['failure'])}. Tests shall cover rejected dependencies, invalid/empty values, cancellation where declared, retry where declared, cleanup, and preservation of the last committed state."
            )
        else:
            failure_recovery = (
                f"{primary_symbol} declares no local catch/retry/cancel primitive; thrown dependency or validation failures propagate to its cited consumer, which owns visible recovery and must not commit a partial transition."
            )
        if signals["persistence"]:
            persistence = (
                f"Direct browser persistence primitives: {', '.join(signals['persistence'])}; serialized key/value details remain governed by the dedicated persisted-state catalog and the exact source expressions."
            )
        elif "defineStore" in signals["state"]:
            persistence = (
                f"Pinia store state ({', '.join(store_ids) if store_ids else primary_symbol}) is in-memory unless a cited consumer/plugin serializes it; this module declares no direct durable storage primitive."
            )
        else:
            persistence = f"{primary_symbol} declares no direct durable storage primitive; returned values/state are owned by its consumer."
        source_dependencies = ", ".join(imports) if imports else "no imported module"
        interfaces_side_effects = joined(
            f"imports={source_dependencies}",
            f"effects={', '.join(signals['effects'])}" if signals["effects"] else "effects=no direct I/O token identified",
            f"route_literals={', '.join(route_literals)}" if route_literals else "route_literals=none",
        )
        if source_file.startswith("apps/desktop-ui/"):
            variants = "Desktop UI only; privileged effects require the context-isolated Desktop bridge and OS/platform state remains shell-owned."
        elif source_file.startswith("apps/website/"):
            variants = "Website distribution; browser viewport, reduced-motion, marketing asset, and deployment-route differences apply."
        elif broad_anchor.startswith("COMFY-CLOUD-") or "cloud" in source_file.casefold():
            variants = "Cloud/account capability; entitlement, authentication, workspace role, region, billing state, and remote feature flags can disable or vary behavior."
        else:
            variants = "Main frontend distribution; local, remote, Desktop-hosted, server capability, browser, localization, and source feature-flag variants apply where imported dependencies expose them."
        actor_by_domain = {
            "queue-execution-state": "Workflow operator or queue/history surface",
            "workflow-lifecycle-sharing": "Workflow author or sharing/import consumer",
            "authentication-secrets": "Signing-in user or authentication provider callback",
            "cloud-billing-workspace": "Authenticated account/workspace member or billing administrator",
            "frontend-extension-manager": "Extension author, Manager user, or registry consumer",
            "assets-specialized-media": "Workflow author editing or inspecting media/assets",
            "graph-editing-widgets": "Graph editor user, node/widget renderer, or pointer/keyboard handler",
            "settings-keybindings": "User customizing settings, palette, or shortcuts",
            "diagnostics-errors": "User or operator inspecting terminal/error state",
            "updates-versioning": "User or update/version service",
            "website-interaction": "Website visitor",
            "cloud-account-state": "Authenticated cloud user",
            "application-ui-state": "Frontend user or owning application surface",
        }
        domain = frontend_functional_domain(source_file, broad_anchor)
        evidence_level = "test-backed" if tests else "code-inferred"
        feature_id = stable_id("COMFY-FRONTMOD", source_file)
        rows.append({
            "feature_id": feature_id,
            "product": frontend_functional_product(source_file),
            "domain": domain,
            "module": source_file,
            "primary_symbol": primary_symbol,
            "exported_symbols": ", ".join(exports),
            "classification": "infrastructure-only" if infrastructure_reason else "frontend functional module",
            "disposition": disposition,
            "availability": availability,
            "source_availability": source_availability,
            "evidence_level": evidence_level,
            "confidence": "high" if tests else "medium",
            "source_evidence": f"{source_file}:{source_line}; sha256={source_digest}",
            "test_evidence": "; ".join(tests) if tests else "No same-module .test.ts or .spec.ts file was located; behavior is statically inferred.",
            "broad_anchor": broad_anchor,
            "candidate_predicate": "normalized production/cloud/paid/platform-specific .ts in composable(s)/store(s)/service(s); own IDs removed; only primary broad anchor remains; exact source path absent from every other frontend catalog",
            "actor": actor_by_domain[domain],
            "trigger": (
                f"A consuming module imports {primary_symbol} from {source_file} and invokes/reads it under the source signature."
                if infrastructure_reason
                else f"The actor invokes {primary_symbol}, dispatches one of its named transitions, or consumes its reactive state from {source_file}."
            ),
            "preconditions": joined(
                f"broad anchor={broad_anchor}",
                f"dependencies={source_dependencies}",
                f"availability={source_availability}",
            ),
            "inputs_defaults_flags": joined(
                f"export declarations={'; '.join(signature_lines) if signature_lines else ', '.join(exports)}",
                f"flags/guards={', '.join(signals['flags'])}" if signals["flags"] else "flags/guards=no statically named feature guard",
            ),
            "observable_success": observable_success,
            "state_concurrency": state_concurrency,
            "error_cancel_retry_recovery": failure_recovery,
            "persistence_serialization": persistence,
            "interfaces_side_effects": interfaces_side_effects,
            "platform_cloud_variants": variants,
            "independent_validation": (
                f"Import {source_file}; exercise {primary_symbol} and {', '.join(action_names) if action_names else 'each exported member'} with default, empty, boundary, invalid, rejected-dependency, duplicate, stale-completion, cleanup, and applicable cancellation/retry fixtures; assert {exact_contract}."
            ),
            "infrastructure_disposition": infrastructure_reason or "Not infrastructure-only: this module owns the named independently observable state, interaction, or service transition.",
            "source_sha256": source_digest,
        })

    rewrite_catalog("frontend-functional-modules.csv", rows, FRONTEND_FUNCTIONAL_MODULE_FIELDS)
    if augment_source_mapping:
        feature_by_source = {row["module"]: row["feature_id"] for row in rows}
        changed = False
        for source_row in source_rows:
            feature_id = feature_by_source.get(source_row["source_file"])
            old_mapped_ids = [part.strip() for part in source_row["feature_ids"].split("|") if part.strip()]
            mapped_ids = [
                mapped_id
                for mapped_id in old_mapped_ids
                if not mapped_id.startswith("COMFY-FRONTMOD-")
            ]
            if feature_id:
                mapped_ids.append(feature_id)
            old_reason = source_row["reason"]
            reason = clean(re.sub(
                r"(?:;\s*)?Source-specific functional-module disposition is COMFY-FRONTMOD-[A-F0-9]{12}\.",
                "",
                old_reason,
            ))
            if feature_id:
                reason = joined(
                    reason,
                    f"Source-specific functional-module disposition is {feature_id}.",
                )
            if mapped_ids != old_mapped_ids or reason != old_reason:
                source_row["feature_ids"] = " | ".join(mapped_ids)
                source_row["reason"] = reason
                changed = True
        if changed:
            rewrite_catalog(
                "frontend-source-files.csv",
                source_rows,
                list(source_rows[0].keys()),
            )
    return rows


def markdown(value: object, limit: int | None = None) -> str:
    text = clean(value, "Not separately identified.").replace("|", "\\|")
    if limit is not None and len(text) > limit:
        return text[: limit - 1].rstrip() + "…"
    return text


def sim_status(availability: str, evidence_level: str) -> str:
    availability_lower = availability.casefold()
    evidence_lower = evidence_level.casefold()
    if "uncertain" in availability_lower or "unverified" in evidence_lower:
        return "uncertain"
    if any(token in availability_lower for token in ("cloud/paid", "deprecated/dead", "infrastructure-only")):
        return "deferred"
    return "missing"


def parity_decision(product: str, domain: str, feature_id: str, availability: str) -> str:
    availability_lower = availability.casefold()
    key = f"{product} {domain} {feature_id}".casefold()
    if "uncertain" in availability_lower:
        return "investigate-and-preserve"
    if "cloud/paid" in availability_lower:
        return "defer-pending-service-contract-and-authorization"
    if "deprecated/dead" in availability_lower:
        return "recognize-for-compatibility-or-migration-only"
    if "infrastructure-only" in availability_lower:
        return "inventory-and-test-through-consuming-capabilities"
    if any(token in key for token in ("python", "javascript", "litegraph", "web extension", "frontend-ext", "frontext-ext")):
        return "replace-production-execution-with-versioned-rust-wasm-ports-and-preserve-legacy-identifiers"
    if any(token in key for token in ("http", "websocket", "comfy-api", "comfy-ws", "comfy-cli", " cli ")):
        return "implement-as-native-rust-compatibility-host-or-cli-over-the-native-runtime"
    if any(token in key for token in ("node", "model", "exec", "sampler", "scheduler", "latent", "tensor", "device", "memory", "cache", "media")) and "frontend" not in key:
        return "implement-entirely-in-native-rust-with-per-contract-conformance"
    if "desktop" in key:
        return "map-observable-lifecycle-to-native-rust-workers-artifacts-plugins-and-gpui-surfaces"
    if "comfy-config" in key or " configuration " in f" {key} ":
        return "map-source-configuration-to-native-runtime-policy-or-inactive-legacy-migration"
    if "cross-product" in key or "compat" in key or "format" in key:
        return "implement-lossless-native-adapter-with-versioned-migration"
    return "implement-as-native-rust-gpui-capability"


NATIVE_COMPONENT_SURFACE_DISPOSITIONS = {
    row["feature_id"]: row for row in read_rows("native-component-dispositions.csv")
}
if len(NATIVE_COMPONENT_SURFACE_DISPOSITIONS) != 805:
    raise RuntimeError(
        "native component disposition ledger must contain 805 unique feature rows"
    )


def component_surface_disposition(feature_id: str, _domain: str) -> str:
    try:
        row = NATIVE_COMPONENT_SURFACE_DISPOSITIONS[feature_id]
    except KeyError as error:
        raise RuntimeError(
            f"native component disposition ledger is missing {feature_id}"
        ) from error
    return (
        f"{row['disposition']}:{row['placement']};owner:{row['owner_task_id']}"
    )


def requirement_numbers(feature: dict[str, str]) -> list[int]:
    feature_id = feature["feature_id"].upper()
    product = feature["product"].casefold()
    domain = feature["domain"].casefold()
    name = feature["name"].casefold()
    classification = feature["classification"].casefold()
    availability = feature["availability"].casefold()
    source_catalog = feature["source_catalog"].casefold()
    key = " ".join((feature_id.casefold(), product, domain, name, classification, source_catalog))

    requirements = {1, 32, 42}

    prefix_requirements = {
        "COMFY-EXEC-": {4, 5, 10, 33, 34, 38},
        "COMFY-EXT-": {12, 23, 39},
        "COMFY-SEC-": {28},
        "COMFY-NODE-": {6, 34, 38, 44},
        "COMFY-API-EXT-": {12, 22, 39, 40, 44},
        "COMFY-API-": {8, 40},
        "COMFY-WS-": {9, 40},
        "COMFY-CONFIG-": {3, 7, 28, 33, 35},
        "COMFY-MODEL-": {7, 34, 35, 36, 37},
        "COMFY-TENSOR-": {7, 34, 35, 38},
        "COMFY-AUTOGRAD-": {7, 34, 38},
        "COMFY-RNG-": {7, 34, 37, 38},
        "COMFY-FORMAT-SCHEMA-": {8, 16, 30, 40},
        "COMFY-FORMAT-": {11, 16, 17, 30, 36, 41},
        "COMFY-GRAPH-": {13, 14, 15},
        "COMFY-ASSET-": {11, 19, 41},
        "COMFY-WORKFLOW-": {16, 17},
        "COMFY-CLOUD-": {22},
        "COMFY-UI-": {15, 20},
        "COMFY-SETTING-": {21},
        "COMFY-QUEUE-": {10, 18},
        "COMFY-FRONTEND-EXT-": {12, 23, 39},
        "COMFY-FRONTEXT-EXT-": {12, 23, 39},
        "COMFY-A11Y-": {15},
        "COMFY-PERSIST-": {21, 29},
        "COMFY-TELEMETRY-": {22},
        "COMFY-MENU-": {15, 20},
        "COMFY-ROUTE-": {20, 27},
        "COMFY-HTTP-CLIENT-": {8, 17},
        "COMFY-COMPAT-": {2, 12, 23, 30},
        "COMFY-DESKTOP-TELEMETRY-": {22, 28},
        "COMFY-DESKTOP-RENDERER-": {27},
    }
    for prefix, mapped in prefix_requirements.items():
        if feature_id.startswith(prefix):
            requirements.update(mapped)

    if feature_id.startswith("COMFY-FRONTMOD-"):
        if "queue-execution" in domain:
            requirements.update({10, 18, 38})
        elif "workflow" in domain:
            requirements.update({16, 17})
        elif any(token in domain for token in ("authentication", "cloud", "billing", "workspace")):
            requirements.update({22, 28})
        elif "extension-manager" in domain:
            requirements.update({12, 23, 39})
        elif "assets-specialized-media" in domain:
            requirements.update({11, 19, 41})
        elif "graph-editing" in domain:
            requirements.update({13, 14, 15})
        elif "settings-keybindings" in domain:
            requirements.update({20, 21})
        elif "diagnostics-errors" in domain:
            requirements.update({20, 26})
        elif "updates-versioning" in domain:
            requirements.add(25)
        elif "website-interaction" in domain:
            requirements.update({20, 22})
        else:
            requirements.update({15, 20})

    if feature_id.startswith("COMFY-FRONTEND-SURFACE-"):
        surface_requirements = {
            "graph-editor": {13, 14, 15},
            "workflow-experience": {16, 17},
            "queue-execution-ui": {10, 18},
            "asset-viewer-editor": {11, 19, 41},
            "settings": {20, 21},
            "cloud-account-workspace": {22, 28},
            "website-cloud": {20, 22},
            "frontend-extension-manager": {12, 23, 39},
            "application-ui": {15, 20},
            "desktop-installation": {24, 27},
            "desktop-update": {25, 27},
            "desktop-diagnostics": {26, 27},
            "desktop-native-ui": {24, 27},
        }
        requirements.update(surface_requirements.get(domain, {15, 20}))

    if "cloud/paid" in availability:
        requirements.add(22)
    if feature_id.startswith("COMFY-PERSIST-") and "secret-bearing" in feature["permissions_flags"].casefold():
        requirements.add(28)
    if feature_id.startswith("COMFY-MENU-"):
        if any(token in key for token in ("workflow", "template", "blueprint")):
            requirements.update({16, 17})
        if any(token in key for token in ("graph", "node", "group", "canvas", "subgraph", "reroute")):
            requirements.update({13, 14})
        if any(token in key for token in ("asset", "image", "3d", "media")):
            requirements.add(19)
        if any(token in key for token in ("queue", "job", "execution")):
            requirements.add(18)
        if any(token in key for token in ("workspace", "member", "subscription")):
            requirements.add(22)
        if "keybinding" in key:
            requirements.update({20, 21})

    if feature_id.startswith("COMFY-NODE-INACTIVE-"):
        requirements.update({6, 30, 44})
    if feature_id.startswith("COMFY-BACKEND-SOURCE-"):
        if "workflow-template" in domain:
            requirements.update({16, 17})
        if "model-support" in domain:
            requirements.update({7, 34, 35, 36, 37})
        if "database-migration" in domain:
            requirements.update({16, 29})
        if "filesystem-layout" in domain:
            requirements.update({3, 7, 11})
        if "package-test" in domain:
            requirements.add(32)
    if "api node" in classification or "comfy_api_nodes" in feature["source_evidence"]:
        requirements.update({12, 22, 39, 40, 44})

    if "comfy-desktop" in product or feature_id.startswith("COMFY-DESKTOP-"):
        if any(token in key for token in ("source", "installation", "onboarding", "migration", "adoption")):
            requirements.add(24)
        if any(token in key for token in ("update", "snapshot", "rollback", "download")):
            requirements.add(25)
        if any(token in key for token in ("process", "launch", "lifecycle", "terminal", "log", "crash", "diagnostic", "port", "health")):
            requirements.update({3, 26, 33, 38})
        if any(token in key for token in ("ipc", "preload", "menu", "window", "navigation", "shell", "keybinding", "gesture", "title")):
            requirements.add(27)
        if any(token in key for token in ("platform", "packaging", "security", "permission", "auth", "path", "url", "native")):
            requirements.update({28, 33})
        if any(token in key for token in ("setting", "persistence", "store", "json", "yaml", "marker", "database", "cache")):
            requirements.update({21, 29})
        if any(token in key for token in ("cloud", "billing", "telemetry", "feature flag", "secret")):
            requirements.add(22)
        if len(requirements) == 2:
            requirements.add(24)

    if "cross-product" in product:
        if any(token in key for token in ("workflow", "schema", "json", "migration", "legacy")):
            requirements.update({16, 30})
        if any(token in key for token in ("png", "webp", "flac", "media", "metadata", "template", "app mode")):
            requirements.update({17, 41})
        if any(token in key for token in ("rest", "http")):
            requirements.update({8, 40})
        if "websocket" in key:
            requirements.update({9, 40})
        if any(token in key for token in ("ipc", "preload")):
            requirements.add(27)
        if any(token in key for token in ("extension", "custom node", "litegraph")):
            requirements.update({12, 23, 39})
        if any(token in key for token in ("local", "remote", "cloud", "portable", "desktop")):
            requirements.update({2, 33, 40})

    if any(token in key for token in ("performance", "memory", "large", "bounded", "rate limit", "preview", "progress", "concurrency", "cache")):
        requirements.add(31)
    if any(token in key for token in ("tensor", "dtype", "operator", "autograd", "gradient", "random", "seed", "noise")):
        requirements.add(34)
    if any(token in key for token in ("device", "cuda", "rocm", "metal", "directml", "xpu", "npu", "mlu", "corex", "memory", "offload", "oom")):
        requirements.add(35)
    if any(token in key for token in ("model family", "checkpoint", "safetensor", "gguf", "lora", "loha", "lokr", "vae", "clip", "controlnet", "quantization", "model loader")):
        requirements.add(36)
    if any(token in key for token in ("sampler", "scheduler", "sigma", "latent", "conditioning", "guidance")):
        requirements.add(37)
    if any(token in key for token in ("execution", "queue", "history", "cache", "cancellation", "worker", "recovery")):
        requirements.add(38)
    if any(token in key for token in ("plugin", "extension", "custom node", "python", "javascript", "litegraph", "wasm")):
        requirements.add(39)
    if any(token in key for token in ("http", "websocket", " api ", "cli", "automation", "route")):
        requirements.add(40)
    if any(token in key for token in ("media", "image", "mask", "audio", "video", "3d", "codec", "png", "webp", "flac", "preview", "output")):
        requirements.add(41)
    if any(token in key for token in ("workspace", "persist", "background task", "entity", "focus", "error propagation")):
        requirements.add(29)
    if any(token in key for token in ("security", "permission", "cors", "remote access", "path traversal", "secret", "auth")):
        requirements.add(28)
    if any(token in availability for token in ("deprecated/dead", "experimental", "developer-only", "uncertain")):
        requirements.add(30)

    if "comfy-cli" in product or feature_id.startswith("COMFY-CLI-"):
        requirements.update({33, 40, 43})
        if any(token in key for token in ("python", "custom node", "manager", "frontend pr", "child process", "install")):
            requirements.update({3, 12, 39})
    if product in {"comfy documentation", "comfy embedded documentation"} or any(
        token in source_catalog for token in ("docs-", "embedded-docs-")
    ):
        requirements.add(43)

    return sorted(requirements)


def design_titles() -> dict[int, str]:
    titles: dict[int, str] = {}
    text = (ROOT / "design.md").read_text(encoding="utf-8")
    for match in re.finditer(r"^### D(\d+):\s*(.+)$", text, re.MULTILINE):
        titles[int(match.group(1))] = clean(match.group(2))
    return titles


def decorate_trace(feature: dict[str, str], titles: dict[int, str]) -> None:
    requirements = requirement_numbers(feature)
    all_criteria = [
        f"{requirement}.{criterion}"
        for requirement in requirements
        for criterion in range(1, REQUIREMENT_CRITERIA_COUNTS[requirement] + 1)
    ]
    criterion_override = FEATURE_CRITERION_OVERRIDES.get(feature["feature_id"], [])
    overridden_requirements = {
        int(criterion.split(".", 1)[0]) for criterion in criterion_override
    }
    criteria = [
        criterion
        for criterion in all_criteria
        if int(criterion.split(".", 1)[0]) not in overridden_requirements
        or criterion in criterion_override
    ]
    for criterion in criterion_override:
        if criterion not in criteria:
            criteria.append(criterion)
    criteria.sort(key=lambda criterion: tuple(int(part) for part in criterion.split(".")))
    designs = sorted({design for criterion in criteria for design in CRITERION_DESIGNS[criterion]})
    tasks: list[str] = []
    for requirement in requirements:
        anchor_task = next(
            (
                task
                for task in REQUIREMENT_TASKS[requirement]
                if task not in FEATURE_SCOPED_TASK_IDS
            ),
            None,
        )
        if anchor_task is not None and anchor_task not in tasks:
            tasks.append(anchor_task)
    for task in SPECIAL_FEATURE_TASKS.get(feature["feature_id"], []):
        if task not in tasks:
            tasks.append(task)
    validations: list[str] = []
    for criterion in criteria:
        for validation in CRITERION_VALIDATIONS[criterion]:
            if validation not in validations:
                validations.append(validation)
    for validation in FEATURE_VALIDATION_OVERRIDES.get(feature["feature_id"], []):
        if validation not in validations:
            validations.append(validation)

    feature["requirement_criteria"] = "; ".join(criteria)
    feature["design_coverage"] = "; ".join(
        f"D{design}: {titles.get(design, 'design decision')}" for design in designs
    )
    feature["task_id"] = "; ".join(tasks)
    feature["validation_id"] = "; ".join(validations)
    if feature["automated_validation"].startswith("Not separately"):
        feature["automated_validation"] = (
            f"Run {', '.join(validations)} with a deterministic fixture for {feature['feature_id']}."
        )
    if feature["manual_validation"].startswith("Not separately"):
        feature["manual_validation"] = (
            f"Exercise {feature['feature_id']} in its applicable local, remote, cloud, desktop, and platform variants; compare visible states and side effects to the cited source."
        )


def base_feature(**values: object) -> dict[str, str]:
    explicit_sim_status = clean(values.get("current_sim_status", ""))
    explicit_parity_decision = clean(values.get("parity_decision", ""))
    defaults = {
        "product": "Source product not separately identified",
        "domain": "uncategorized",
        "name": "Unnamed source contract",
        "classification": "source capability",
        "availability": "uncertain",
        "evidence_level": "unverified",
        "confidence": "low",
        "source_evidence": "Source location not separately resolved; retained as an uncertainty.",
        "source_symbol": "Source symbol not separately resolved.",
        "test_evidence": "No focused existing test was located.",
        "documentation": "No separate documentation-only claim was used.",
        "runtime_observation": "Not observed in this audit; see baseline constraints.",
        "actor": "Source-defined user, operator, automation client, extension, or system actor.",
        "trigger": "Invoke the named source capability.",
        "preconditions": "The applicable source distribution, version, dependency, entitlement, and feature gate are available.",
        "inputs_defaults": "Use the exact values and defaults in the cited source contract.",
        "permissions_flags": "No additional permission or feature flag was identified beyond the recorded availability and cited source.",
        "observable_success": "The named source contract reaches its source-defined success state.",
        "interaction_accessibility": "No direct keyboard, pointer, drag, clipboard, focus, or accessibility contract was identified at this boundary; consuming UI behavior is cataloged separately.",
        "state_concurrency": "Use the ordering, ownership, and concurrency behavior in the cited source; no additional transition was inferred.",
        "failure_recovery": "Invalid, unavailable, cancelled, timed-out, conflicting, permission-denied, and recovery states remain deterministic validation targets where the source permits them.",
        "persistence_serialization": "No additional durable representation or migration was identified beyond the cited source contract.",
        "interfaces_side_effects": "No additional route, event, IPC, subprocess, filesystem, or external side effect was identified beyond the cited source contract.",
        "platform_localization_variants": "Use the source-defined platform, localization, local/remote/cloud, and version variants; none were generalized beyond the evidence.",
        "sim_evidence": "Repository-wide search and the target architecture audit found no Comfy-specific implementation outside projects/comfy/** and this specification; generic Sim primitives are reusable infrastructure only.",
        "parity_gap": "Sim has no Comfy-specific implementation of this observable contract.",
        "observable_sim_acceptance": "With the same deterministic source fixture and preconditions, Sim shall reproduce the cited success, boundary, failure, cancellation, persistence, compatibility, and recovery observations for this feature, or expose the recorded deliberate defer decision without data loss.",
        "automated_validation": "Not separately specified; route through the trace-linked deterministic validation suite.",
        "manual_validation": "Not separately specified; compare the source and Sim side by side in every applicable distribution and platform variant.",
        "open_questions": "No feature-specific question beyond the recorded runtime, platform, dependency, cloud, and extension uncertainties.",
    }
    feature = {field: "" for field in FEATURE_FIELDS}
    feature.update(defaults)
    for key, value in values.items():
        if key in feature:
            feature[key] = clean(value, feature.get(key, ""))
    feature["current_sim_status"] = explicit_sim_status or sim_status(
        feature["availability"], feature["evidence_level"]
    )
    feature["parity_decision"] = explicit_parity_decision or parity_decision(
        feature["product"], feature["domain"], feature["feature_id"], feature["availability"]
    )
    if (
        not explicit_sim_status
        and feature["parity_decision"].startswith("replace-production-execution")
    ):
        feature["current_sim_status"] = "conflicting"
        feature["parity_gap"] = joined(
            "The source contract depends on Python or JavaScript execution that production Sim explicitly forbids.",
            "A versioned Rust/WASM replacement, explicit ports, deterministic legacy identifier mapping, and lossless unresolved placeholder behavior are not implemented.",
            feature["parity_gap"],
        )
    if feature["feature_id"].startswith("COMFY-A11Y-"):
        if ACCESSIBLE_COMFY_BOOTSTRAP and feature["feature_id"] in A11Y_FOUNDATION_FEATURES:
            feature["current_sim_status"] = "partial"
            feature["sim_evidence"] = (
                "crates/sim/src/main.rs constructs GPUI with Application::with_platform without "
                "an accessibility environment gate; crates/comfy_ui/src/graph_render.rs exposes "
                "the native graph application/key context and semantic controls; "
                "VAL-GPUI-012/013 validate the bootstrap."
            )
            feature["parity_gap"] = (
                "The production accessibility conflict is resolved for the application bootstrap "
                "and this native graph foundation row; later route, panel, dialog, settings, "
                "execution, media, Desktop, localization, and platform screen-reader behavior "
                "remains assigned to its executable tasks and VAL-GPUI-011."
            )
            feature["parity_decision"] = (
                "retain-accessible-bootstrap-and-complete-later-surface-audits"
            )
        elif not ACCESSIBLE_COMFY_BOOTSTRAP:
            feature["current_sim_status"] = "conflicting"
            feature["sim_evidence"] = (
                "crates/sim/src/main.rs:build_application defaults to "
                "Application::new_inaccessible unless SIM_EXPERIMENTAL_A11Y=1; "
                "the Sim architecture audit classifies this as a production accessibility conflict."
            )
            feature["parity_gap"] = (
                "The source accessibility contract requires an operable semantic surface, "
                "while current production Sim defaults to an inaccessible GPUI application."
            )
            feature["parity_decision"] = "enable-and-verify-production-gpui-accessibility-before-release"
    elif explicit_sim_status == "conflicting" and not explicit_parity_decision:
        feature["parity_decision"] = parity_decision(
            feature["product"], feature["domain"], feature["feature_id"], feature["availability"]
        )
    elif explicit_sim_status == "uncertain":
        feature["parity_decision"] = "investigate-and-preserve-without-guessing"
    if feature["current_sim_status"] == "missing" and "no comfy-specific" not in feature["parity_gap"].casefold():
        feature["parity_gap"] = joined(
            "No Comfy-specific Sim implementation exists for this contract.",
            f"Source-specific gap detail: {feature['parity_gap']}",
        )
    elif feature["current_sim_status"] == "deferred" and "deliberately defer" not in feature["parity_gap"].casefold():
        feature["parity_gap"] = joined(
            "No Comfy-specific Sim implementation exists; the current parity decision deliberately defers this contract while retaining its data and trace links.",
            f"Source-specific gap detail: {feature['parity_gap']}",
        )
    elif feature["current_sim_status"] == "uncertain" and "remains uncertain" not in feature["parity_gap"].casefold():
        feature["parity_gap"] = joined(
            "No verified Comfy-specific Sim implementation exists, and the source or target behavior remains uncertain.",
            f"Uncertainty detail: {feature['parity_gap']}",
        )
    for field in FEATURE_FIELDS:
        if not clean(feature[field]):
            feature[field] = f"Not separately identified for {feature['feature_id'] or 'this feature'}."
    return feature


def add_backend_features(features: list[dict[str, str]]) -> None:
    name = "backend-features.csv"
    for index, row in enumerate(read_rows(name), 2):
        source_contract = joined(
            row["feature_id"],
            row["domain"],
            row["name"],
            row["classification"],
            row["success_behavior"],
            row["protocols_dependencies"],
        ).casefold()
        prohibited_extension_ids = {
            "COMFY-EXT-002",
            "COMFY-EXT-003",
            "COMFY-EXT-005",
            "COMFY-EXT-006",
            "COMFY-EXT-007",
            "COMFY-EXT-009",
            "COMFY-EXT-010",
            "COMFY-EXT-011",
            "COMFY-EXT-015",
        }
        legacy_extension_contract = row["feature_id"] in prohibited_extension_ids or any(
            token in source_contract
            for token in (
                "prestartup script",
                "importlib",
                "web_directory",
                "legacy web directory",
                "python extension api",
                "python module discovery",
                "manager extension policy",
            )
        )
        target_status = "conflicting" if legacy_extension_contract else ""
        target_decision = (
            "replace-production-execution-with-versioned-rust-wasm-ports-and-preserve-legacy-identifiers"
            if legacy_extension_contract
            else ""
        )
        target_gap = (
            "The source contract executes Python or JavaScript extension code, which the production-native boundary forbids; the versioned Rust/WASM port, explicit socket translation, legacy identifier mapping, and unresolved placeholder are not implemented."
            if legacy_extension_contract
            else f"Sim has no native Rust implementation of the backend contract `{row['name']}`."
        )
        target_acceptance = joined(
            f"Using the pinned source only as a development-time oracle, native Rust Sim shall reproduce this source-observable contract: {row['success_behavior']}",
            f"It shall also reproduce the cataloged boundary and recovery behavior: {row['failure_recovery']}",
            (
                "Legacy Python/JavaScript registrations shall resolve only through a versioned Rust/WASM plugin manifest with explicit input/output ports and deterministic legacy identifier mappings; unsupported imperative hooks remain lossless, visible placeholders."
                if legacy_extension_contract
                else "Execution, state, persistence, protocol effects, cancellation, and recovery shall be owned by native Rust services and shall not launch, connect to, embed, or forward to ComfyUI or Python."
            ),
        )
        source_question = clean(row["open_questions"])
        if any(
            token in source_question.casefold()
            for token in ("managed python", "external server", "rust reimplementation", "hybrid", "backend strategy")
        ):
            source_question = (
                "The production-native boundary is fixed; remaining uncertainty is limited to native backend selection, reviewed vendor FFI, device/platform certification, model/codec/provider availability, and missing source-runtime observations."
            )
        features.append(base_feature(
            feature_id=row["feature_id"], product=first(row, "product", fallback="ComfyUI"),
            domain=first(row, "domain", fallback="backend"), name=row["name"],
            classification=row["classification"], availability=row["availability"],
            evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=row["source_evidence"], source_symbol=row["source_evidence"],
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            actor=row["actor"], trigger=row["trigger"], preconditions=row["preconditions"],
            inputs_defaults=row["inputs_defaults"], permissions_flags=row["permissions_flags"],
            observable_success=row["success_behavior"],
            interaction_accessibility="Backend or protocol capability; direct GUI interaction is represented by the frontend and Desktop ledgers.",
            state_concurrency=joined(row.get("state_concurrency"), row.get("variants")),
            failure_recovery=row["failure_recovery"], persistence_serialization=row["persistence_side_effects"],
            interfaces_side_effects=row["protocols_dependencies"],
            platform_localization_variants=row["platform_version"],
            current_sim_status=target_status,
            parity_decision=target_decision,
            parity_gap=target_gap,
            observable_sim_acceptance=target_acceptance, automated_validation=row["automated_validation"],
            manual_validation=row["manual_validation"], open_questions=source_question,
            source_catalog=name, source_row=index,
        ))


def add_backend_source_anchors(features: list[dict[str, str]]) -> None:
    name = "backend-source-coverage.csv"
    path = CATALOGS / name
    rows = read_rows(name)
    changed = False
    for row in rows:
        if row["classification"].startswith("production") and not clean(row["mapped_feature_ids"]):
            row["mapped_feature_ids"] = stable_id("COMFY-BACKEND-SOURCE", row["source_file"])
            row["reason"] = joined(
                row["reason"],
                "Mapped by a stable source-coverage contract so production data, templates, layout markers, and support files are not left unexplained.",
            )
            changed = True
    if changed:
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=[
                "source_file", "classification", "mapped_feature_ids", "reason", "sha256", "size_bytes"
            ])
            writer.writeheader()
            writer.writerows(rows)

    for index, row in enumerate(rows, 2):
        feature_id = clean(row["mapped_feature_ids"])
        if not feature_id.startswith("COMFY-BACKEND-SOURCE-"):
            continue

        source_file = row["source_file"]
        lower = source_file.casefold()
        suffix = Path(source_file).suffix.casefold()
        if lower.startswith("blueprints/") and suffix == ".json":
            domain = "workflow-template"
            classification = "bundled workflow template"
            availability = "active"
            actor = "Workflow author or frontend template consumer."
            success = f"The bundled blueprint {source_file} is discoverable and parseable as its checked-in workflow/template contract."
            failure = "Missing, malformed, schema-incompatible, or dependency-incomplete template data is reported without overwriting the source artifact; unavailable nodes/models remain explicit."
            acceptance = f"A deterministic catalog/import test shall enumerate {source_file}, verify its checksum and schema/version, preserve every node/link/widget/model/unknown field, and expose missing dependencies without data loss."
        elif lower.startswith("blueprints/"):
            domain = "workflow-template-support"
            classification = "bundled template support asset"
            availability = "conditional"
            actor = "Blueprint/template loader or renderer."
            success = f"The blueprint support artifact {source_file} is available to its consuming template or shader path with the checked-in bytes."
            failure = "Missing, unreadable, malformed, unsupported, or version-skewed support data fails the consuming template visibly while leaving the original workflow intact."
            acceptance = f"A deterministic template-support test shall verify {source_file} checksum, consumer resolution, valid load, malformed/missing behavior, and platform-safe path handling."
        elif lower.startswith("models/") or "text_encoder" in lower or "tokenizer" in lower:
            domain = "model-support-data"
            classification = "bundled model configuration, tokenizer, or search-path artifact"
            availability = "conditional"
            actor = "Model loader, text encoder, tokenizer, node, or operator."
            success = f"The authoritative Python engine resolves {source_file} for its model family, tokenizer, configuration, or model search-path contract."
            failure = "Absent, corrupt, incompatible, or permission-denied model support data produces the engine's explicit unavailable/load error and does not fabricate model capability."
            acceptance = f"A Python-authoritative fixture shall verify discovery and checksum for {source_file}, one valid consumer path, missing/corrupt behavior, and Sim's matching availability/error projection."
        elif lower.startswith("alembic"):
            domain = "database-migration-support"
            classification = "database migration support template"
            availability = "infrastructure-only"
            actor = "ComfyUI database migration tooling."
            success = f"Migration tooling can resolve and render {source_file} when generating or applying the source database contract."
            failure = "Missing or invalid migration support fails generation/migration explicitly and does not silently create an empty database."
            acceptance = f"Migration fixtures shall verify {source_file} resolution/rendering and explicit failure/recovery when absent or malformed."
        elif lower in {"pyproject.toml", "pytest.ini"}:
            domain = "package-test-configuration"
            classification = "package or test configuration coverage anchor"
            availability = "infrastructure-only"
            actor = "Package, build, or test tooling."
            success = f"Tooling reads {source_file} with the checked-in package/test configuration."
            failure = "Invalid configuration fails the invoking tool explicitly; it is not treated as a user workflow."
            acceptance = f"The source ledger shall verify {source_file} checksum and the applicable package/test parser; no user-facing capability is inferred beyond its consumers."
        else:
            domain = "filesystem-layout-configuration"
            classification = "bundled filesystem layout or configuration artifact"
            availability = "active"
            actor = "Operator, file service, model-path service, or workflow author."
            success = f"ComfyUI resolves {source_file} at its cataloged input/output/model/configuration boundary."
            failure = "Missing, invalid, unsafe, or permission-denied path/configuration state surfaces through the owning file/model/config operation without unsafe fallback."
            acceptance = f"A deterministic filesystem/config fixture shall verify {source_file} checksum, expected location/consumer, safe-path behavior, missing/permission errors, and restart behavior."

        features.append(base_feature(
            feature_id=feature_id,
            product="ComfyUI",
            domain=domain,
            name=f"Source artifact: {source_file}",
            classification=classification,
            availability=availability,
            evidence_level="code-inferred",
            confidence="high",
            source_evidence=f"projects/comfy/ComfyUI/{source_file}; sha256={row['sha256']}; size={row['size_bytes']} bytes",
            source_symbol=source_file,
            actor=actor,
            trigger=f"The source product enumerates, loads, parses, renders, or writes the contract associated with {source_file}.",
            preconditions="The pinned source snapshot and applicable loader, template, model family, database, or filesystem boundary are available.",
            inputs_defaults=f"Checked-in path={source_file}; sha256={row['sha256']}; size_bytes={row['size_bytes']}.",
            permissions_flags=f"Original source classification: {row['classification']}; filesystem, model, custom-code, remote, and platform policy follow the consuming contract.",
            observable_success=success,
            interaction_accessibility="Support-file behavior has no direct focus target; template, model, error, chooser, or file surfaces expose the consuming keyboard/focus/accessibility contract.",
            state_concurrency="The consuming loader uses one pinned/versioned artifact snapshot; background parsing/loading must not apply stale results to a changed profile, workflow, or model registry.",
            failure_recovery=failure,
            persistence_serialization=f"The file path, bytes, SHA-256 {row['sha256']}, and consuming format/version remain deterministic source evidence; no migration is invented.",
            interfaces_side_effects=row["reason"],
            platform_localization_variants="Availability varies only where the consuming model, template, package, filesystem, database, or platform contract states it; filenames are not localized.",
            parity_gap=f"Sim has no Comfy-specific consumer, catalog, or compatibility treatment for {source_file}.",
            observable_sim_acceptance=acceptance,
            automated_validation=acceptance,
            manual_validation=f"Inspect {source_file} in the pinned source and its consuming source surface; compare discovery, visible availability/error, and retained data in Sim.",
            open_questions="Runtime consumption remains unobserved where the required dependency, model, accelerator, migration environment, or frontend is unavailable.",
            source_catalog=name,
            source_row=index,
        ))


def add_backend_nodes(features: list[dict[str, str]], name: str, inactive: bool = False) -> None:
    for index, row in enumerate(read_rows(name), 2):
        inputs = first(row, "inputs", fallback="No declared inputs.")
        outputs = first(row, "outputs", fallback="No declared outputs.")
        identifier = row["node_identifier"]
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="node",
            name=f"{first(row, 'display_name', 'class_name', fallback=identifier)} [{identifier}]",
            classification=row["classification"], availability=row["availability"],
            evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=joined(
                f"{row['source_file']}:{first(row, 'source_line', fallback='line not resolved')}",
                row.get("registration_evidence"), row.get("reason")
            ),
            source_symbol=first(row, "source_symbol", "class_name", fallback=identifier),
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            actor="Workflow author, automation client, or custom-node compatibility consumer.",
            trigger=(f"Prompt execution reaches node {identifier}." if not inactive else f"Source discovery encounters unregistered schema-bearing node {identifier}."),
            preconditions=("Node is present in the effective registry and required model, device, service, or extension dependencies are available." if not inactive else "Node remains absent from the effective runtime registry."),
            inputs_defaults=inputs,
            permissions_flags=f"Availability: {row['availability']}; schema API: {first(row, 'schema_api', fallback='not registered')}",
            observable_success=(f"Category {first(row, 'category', fallback='uncategorized')}; outputs {outputs}." if not inactive else row["reason"]),
            interaction_accessibility="The server schema supplies graph labels, sockets, and widgets; native graph keyboard, pointer, focus, and accessibility behavior is cataloged under frontend graph features.",
            state_concurrency=joined(
                f"input_is_list={first(row, 'input_is_list', fallback='not declared')}",
                f"output_is_list={first(row, 'output_is_list', fallback='not declared')}",
                f"lazy_inputs={first(row, 'lazy_inputs', fallback='not declared')}",
                f"output_node={first(row, 'output_node', fallback='not declared')}",
                row.get("execution_blocking"), row.get("caching"), row.get("change_detection")
            ),
            failure_recovery=joined(row.get("validation"), row.get("error_behavior"), row.get("reason")),
            persistence_serialization=f"Workflow serialization uses class_type={identifier}; input literals, links, list flags, widgets, and unknown data must remain lossless.",
            interfaces_side_effects=joined(first(row, "schema_source", fallback="Schema-bearing class"), row.get("registration_evidence")),
            platform_localization_variants="Availability can depend on Python package, model, hardware, hosted service, feature flag, and server version; display name/category are server-authoritative.",
            parity_gap=f"Sim has no native node-registry or execution implementation for {identifier}.",
            observable_sim_acceptance=(f"The compiled native registry and object-info projection shall reproduce every cataloged schema field for {identifier}; its exact Rust or native-provider implementation shall independently match success, boundaries, output, list/lazy, validation, cache/change, blocker/effect, cancellation, error, persistence, and recovery behavior." if not inactive else f"Sim shall not activate {identifier} from this snapshot; import shall retain its serialized data and report its inactive compatibility status."),
            automated_validation=f"Run VAL-NODE-001 and VAL-NODE-CLOSURE-001 for {identifier}" + (" plus VAL-NODE-002 for this exact row; representative-only family evidence is insufficient." if not inactive else "; verify it is absent from the active native registry."),
            manual_validation=f"Inspect {identifier} in source object-info and the Sim node library under each applicable dependency/profile state.",
            open_questions=("Whether a future upstream registry activates this node; baseline policy must detect that delta." if inactive else "Execution remains unobserved when the required model, accelerator, package, account, or external service is unavailable."),
            source_catalog=name, source_row=index,
        ))


def add_backend_routes(features: list[dict[str, str]]) -> None:
    name = "backend-http-routes.csv"
    for index, row in enumerate(read_rows(name), 2):
        route_name = f"{row['method']} {row['path']}"
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="http",
            name=route_name, classification=row["classification"], availability=row["availability"],
            evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=f"{row['source_file']}:{row['source_line']}", source_symbol=row["handler"],
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            documentation=joined(row.get("openapi_operation_id"), row.get("openapi_summary"), fallback="No separate OpenAPI description is present."),
            actor="HTTP API client, frontend, Desktop shell, custom node, or automation client.",
            trigger=f"Send {route_name}.", preconditions="ComfyUI server is listening and the route's feature, dependency, user mode, or compatibility alias is available.",
            inputs_defaults=joined(
                f"body={first(row, 'request_body', fallback='none')}",
                f"path={first(row, 'path_parameters', fallback='none')}",
                f"query={first(row, 'query_parameters', fallback='none')}",
                row.get("request_schema_detail"),
            ),
            permissions_flags=row["permissions_flags"],
            observable_success=joined(row["success_behavior"], row.get("response_schema_detail")),
            interaction_accessibility="Protocol-only contract; visible request state, errors, focus, and accessibility are cataloged on consuming frontend/Desktop features.",
            state_concurrency=joined(row.get("side_effects"), f"canonical_path={row['canonical_path']}", f"alias_of={first(row, 'alias_of', fallback='none')}") ,
            failure_recovery=joined(row["error_behavior"], row.get("unresolved_schema")), persistence_serialization=row["side_effects"],
            interfaces_side_effects=joined(
                route_name,
                f"canonical {row['canonical_path']}",
                row.get("openapi_operation_id"),
                row.get("alias_of"),
                row.get("status_content_types"),
                row.get("source_excerpt"),
            ),
            platform_localization_variants="Route availability varies by API version, feature module, user mode, server flags, installed dependencies, and local/remote deployment.",
            parity_gap=f"Sim has no native Rust handler contract for {route_name}.",
            observable_sim_acceptance=f"For the same valid and malformed fixtures, Sim's native Rust host shall serve {route_name} with matching request decoding, status, headers/content, response schema, side effects, permissions, timeout/idempotency recovery, limits, and aliases while using native runtime services and never forwarding to ComfyUI.",
            automated_validation=f"Parameterize VAL-HTTP-001 with {row['feature_id']} ({route_name}).",
            manual_validation=f"Compare the development source oracle and Sim native handler for {route_name} under every applicable permission/feature state; verify no production forwarding or alternate executor.",
            open_questions=first(row, "unresolved_schema", fallback="Runtime-dependent schema variants require deterministic capture."),
            source_catalog=name, source_row=index,
        ))


def add_backend_websocket(features: list[dict[str, str]]) -> None:
    name = "backend-websocket-events.csv"
    for index, row in enumerate(read_rows(name), 2):
        event = first(row, "event_type", fallback=f"binary code {row['binary_code']}")
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="websocket",
            name=f"{row['direction']} {event}", classification=f"{row['wire_kind']} WebSocket contract",
            availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=row["source_evidence"], source_symbol=event,
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            actor="ComfyUI server, frontend, Desktop host, or protocol client.", trigger=row["trigger_success"],
            preconditions="A WebSocket session exists with the applicable SID, capability negotiation, client routing, and executing prompt state.",
            inputs_defaults=row["schema"], permissions_flags=f"Availability: {row['availability']}; per-client routing and server trust follow the cited handler.",
            observable_success=row["trigger_success"],
            interaction_accessibility="Wire event only; consuming progress, preview, queue, error, and status surfaces carry keyboard/focus/accessibility obligations.",
            state_concurrency=row["ordering_concurrency"], failure_recovery=row["error_recovery"],
            persistence_serialization=f"Wire kind {row['wire_kind']}; event type {event}; binary code {first(row, 'binary_code', fallback='none')}; schema {row['schema']}",
            interfaces_side_effects=f"WebSocket {row['direction']} {event}",
            platform_localization_variants="Binary framing and JSON schema are platform-neutral; event availability and payload additions are version/capability-dependent.",
            parity_gap=f"Sim has no native WebSocket adapter for {event}.",
            observable_sim_acceptance=joined(
                f"Sim's native event bus and Rust WebSocket host shall reproduce the source trigger/success contract for {event}: {row['trigger_success']}",
                f"It shall reproduce the source error/recovery and ordering contracts: {row['error_recovery']}; {row['ordering_concurrency']}",
                "Exact framing, client routing, reconnect projection, unknown fields, cancellation, and shutdown shall be native; no event may be consumed from or forwarded to another Comfy server.",
            ),
            automated_validation=f"Parameterize VAL-WS-001 with {row['feature_id']} ({event}), including malformed, ordering, reconnect, stale-SID, and binary-boundary fixtures.",
            manual_validation=f"Observe {event} during a deterministic source and Sim run, including reconnect and cancellation where applicable.",
            source_catalog=name, source_row=index,
        ))


def add_backend_config(features: list[dict[str, str]]) -> None:
    name = "backend-config.csv"
    for index, row in enumerate(read_rows(name), 2):
        item_name = f"{row['kind']} {row['name']}"
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="configuration",
            name=item_name, classification=row["kind"], availability=row["availability"],
            evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=f"{row['source_file']}:{row['source_line']}", source_symbol=row["source_symbol"],
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            runtime_observation=first(row, "runtime_observation", fallback="Not observed in this audit; see baseline constraints."),
            actor="Source operator, development oracle, or native Sim runtime-profile administrator.", trigger=f"Configure the source oracle or import/map {row['name']} into native Sim policy.",
            preconditions="Parser/configuration initialization runs and the applicable platform, backend, dependency, or mutually exclusive group permits the option.",
            inputs_defaults=joined(f"default={row['default']}", f"choices={first(row, 'choices', fallback='unrestricted')}", f"shape={row['value_shape']}", f"mutual_exclusion={first(row, 'mutual_exclusion_group', fallback='none')}") ,
            permissions_flags=f"Availability: {row['availability']}; privileged network, filesystem, device, and remote-access effects require explicit Sim policy.",
            observable_success=row["behavior"],
            interaction_accessibility="A mapped native setting shall expose a labeled, keyboard-operable control when user-configurable; oracle-only or inactive legacy flags shall be labeled as non-production.",
            state_concurrency="Native startup configuration is resolved before worker, device, cache, plugin, and API-host initialization; source-only flags never launch a source process.",
            failure_recovery="Invalid values, mutual-exclusion conflicts, unavailable devices/dependencies, and unsafe network/path combinations shall be rejected with parser- or UI-visible detail; prior durable configuration remains recoverable.",
            persistence_serialization=f"Preserve source name {row['name']}, shape, default, choices, and exclusion group as a native mapping or inactive legacy record; production does not serialize Comfy launch argv.",
            interfaces_side_effects=row["behavior"], platform_localization_variants="CLI spelling is stable; availability and defaults may vary by operating system, device backend, package, and upstream version.",
            parity_gap=f"Sim has no native-policy or inactive-legacy mapping for {row['name']}.",
            observable_sim_acceptance=f"Sim shall classify {row['name']} as an equivalent native runtime/API/device/memory setting, development-oracle control, or inactive legacy flag; mapped values preserve defaults/choices/exclusions/effects/errors, and production never launches ComfyUI or Python.",
            automated_validation=f"Generate native mapping, invalid-value, legacy-import, and release-boundary cases for {row['feature_id']}; oracle argv comparison runs only in development test support.",
            manual_validation=f"Inspect the native runtime settings or inactive legacy record for {row['name']} and verify the production process/network trace remains source-free.",
            source_catalog=name, source_row=index,
        ))


def add_backend_models(features: list[dict[str, str]]) -> None:
    name = "backend-models.csv"
    for index, row in enumerate(read_rows(name), 2):
        corex_fail_closed = row["feature_id"] == "COMFY-MODEL-0020"
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="model-device-format",
            name=f"{row['kind']}: {row['name']}", classification=row["classification"],
            availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=f"{row['source_file']}:{first(row, 'source_line', fallback='line not resolved')}", source_symbol=row["source_symbol"],
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            actor="Workflow author, operator, node, model loader, sampler, or device manager.",
            trigger=f"Select or detect {row['name']} for {row['kind']}.",
            preconditions=first(row, "dependencies_platform", fallback="The required model data, backend, device, dtype, package, and memory are available."),
            inputs_defaults=joined(row.get("identifier_or_format"), row.get("inputs_defaults")),
            permissions_flags=f"Availability: {row['availability']}; model-path, external-code, device, and download permissions follow profile policy.",
            observable_success=row["success_behavior"],
            interaction_accessibility="Model/device choices shall be labeled, searchable where source UI supports it, keyboard-operable, and expose unavailable/reason state without relying only on color.",
            state_concurrency="Loading, offloading, caching, device transfer, preview, and model-use work follows the source memory/device manager and may run concurrently only at its safe boundaries.",
            failure_recovery=row["failure_behavior"],
            persistence_serialization=f"Preserve identifier/format and source-relative model path semantics: {row['identifier_or_format']}.",
            interfaces_side_effects=joined(row.get("success_behavior"), row.get("dependencies_platform")),
            platform_localization_variants=row["dependencies_platform"],
            current_sim_status="partial" if corex_fail_closed else "",
            sim_evidence=(
                "crates/comfy_backend_corex contains the compiled zero-symbol provenance and structural-package adapter; its loader has no runtime loading path, every certificate projection is rejected, and canonical NativeBackendBindingStatus remains Unbound on every host."
                if corex_fail_closed
                else ""
            ),
            parity_gap=(
                "CoreX execution is intentionally unavailable in this pack. Proprietary IXRT/IXBLAS ABI, library, signing, production-integration, and hardware work is transferred to .agents/specs/comfy-corex-enablement and remains pending there."
                if corex_fail_closed
                else f"Sim has no native model/device/sampling implementation for {row['name']}."
            ),
            parity_decision=(
                "compiled-fail-closed-typed-unbound;future-spec:comfy-corex-enablement"
                if corex_fail_closed
                else ""
            ),
            observable_sim_acceptance=(
                "Sim shall preserve the CoreX identifier, compile the focused native adapter, expose canonical typed Unbound with the exact missing-evidence reason, reject every trust or availability projection, and never load CoreX libraries or fall back to CPU until the separate future specification is completely implemented and certified."
                if corex_fail_closed
                else f"Sim shall implement {row['name']} behind its native tensor/model/sampler/device contracts, preserve the exact identifier/format, publish verified availability, and match source detection/load/intermediate/success/failure/offload/cancellation/OOM behavior without a Python or external-server fallback."
            ),
            automated_validation=f"Run the applicable VAL-TENSOR-001, VAL-DEVICE-001, VAL-MEMORY-001, VAL-MODEL-FORMAT-001, VAL-MODEL-FAMILY-001, VAL-SAMPLER-001, VAL-SCHEDULER-001, or VAL-LATENT-001 fixture for {row['feature_id']}.",
            manual_validation=f"Compare source-oracle and native {row['name']} detection, selection, loading/progress, intermediate checkpoints, memory/offload, cancellation, and failure on every applicable certified hardware/platform profile.",
            source_catalog=name, source_row=index,
        ))


def add_backend_conditioning_contracts(
    features: list[dict[str, str]],
) -> dict[str, tuple[str, str, str]]:
    name = "backend-conditioning-contracts.csv"
    trace_mappings = {}
    for index, row in enumerate(read_rows(name), 2):
        feature_id = row["contract_id"]
        completed = row["current_sim_status"] == "equivalent"
        trace_mappings[feature_id] = (
            row["implementation_task"],
            row["validation_surface"],
            row["closure_artifact"],
        )
        features.append(base_feature(
            feature_id=feature_id,
            product="ComfyUI",
            domain="conditioning-model-patch contract",
            name=f"{row['kind']}: {row['source_symbol']}",
            classification=row["kind"],
            availability="pinned source contract",
            evidence_level="source-fingerprinted",
            confidence="high",
            source_evidence=(
                f"{row['source_path']}#{row['source_symbol']}; "
                f"source_sha256={row['source_sha256']}; "
                f"symbol_sha256={row['symbol_sha256']}"
            ),
            source_symbol=row["source_symbol"],
            test_evidence=joined(row["validation_surface"], row["closure_artifact"]),
            actor="Native conditioning, model, patch, VAE, CLIP, ControlNet, or guidance adapter.",
            trigger=f"Execute the pinned `{row['source_symbol']}` {row['kind']} contract.",
            preconditions="The source-derived payload and every canonical tensor, model, device, memory, cancellation, and execution context satisfy their checked native boundaries.",
            inputs_defaults="Use the exact selected-symbol contract bound by the full-source and selected-symbol SHA-256 values.",
            permissions_flags="Development-time source evidence only; production execution is native Rust and receives no Python, filesystem, network, process, or external-server authority from this row.",
            observable_success="The authoritative native owner executes source-equivalent behavior for this exact fingerprinted contract.",
            state_concurrency="The row delegates ordering, state transition, workspace, cancellation, persistence, and publication to the named canonical domain owners.",
            failure_recovery="Missing, extra, duplicate, stale-digest, retargeted, malformed, non-finite, cancelled, unsupported-device, and out-of-memory cases fail typed without partial publication or fallback.",
            persistence_serialization="The catalog row is a development-time trace record; only its checked native domain and explicitly versioned boundary DTOs may persist production state.",
            interfaces_side_effects="No Python or JavaScript code executes and no ComfyUI process or server is launched, embedded, managed, or contacted.",
            platform_localization_variants="The selected native backend must preserve the same semantics on every certified platform and fail typed where unavailable.",
            current_sim_status=row["current_sim_status"],
            sim_evidence=row["sim_evidence"],
            parity_gap=(
                "No currently observed native execution gap remains for this exact fingerprinted contract."
                if completed
                else f"Executable closure remains assigned to {row['implementation_task']} until its fresh validation evidence and artifact digest pass."
            ),
            parity_decision=f"native-rust-owner:{row['native_owner']};disposition:{row['disposition']}",
            observable_sim_acceptance=f"The exact source and symbol digests map to {row['native_owner']}, task {row['implementation_task']}, executable validation {row['validation_surface']}, and closure artifact {row['closure_artifact'] or 'none'}; generation and executable validation reject any missing, duplicate, stale, or retargeted row.",
            automated_validation=f"Run {row['validation_surface']} and verify explicit closure artifact {row['closure_artifact'] or 'none'}, then run python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice; require the exact source and symbol digests plus a non-skipped executable fixture.",
            manual_validation="No production oracle execution is permitted; inspect only the native result, typed diagnostics, ownership scan, and durable validation artifact.",
            open_questions="No ownership ambiguity is permitted; unavailable hardware remains an explicit named validation gap rather than a pass.",
            source_catalog=name,
            source_row=index,
        ))
    return trace_mappings


def add_backend_tensor_runtime(features: list[dict[str, str]]) -> None:
    for index, row in enumerate(read_rows("backend-tensor-operations.csv"), 2):
        operation_id = row["operation_id"]
        source_sites = joined(row.get("production_call_sites"), row.get("test_call_sites"), row.get("support_call_sites"), row.get("non_call_reference_sites"))
        features.append(base_feature(
            feature_id=operation_id, product="ComfyUI", domain="tensor operation", name=row["symbol"],
            classification=joined(row.get("inventory_kind"), row.get("semantic_group")), availability=row["availability"],
            evidence_level=row["evidence_level"], confidence=row["confidence"], source_evidence=source_sites,
            source_symbol=row["symbol"], test_evidence=first(row, "test_call_sites", fallback="No focused test call was resolved for this symbol."),
            actor="Native node, model, sampler, scheduler, autograd tape, or plugin tensor operation.",
            trigger=f"A cataloged execution path invokes tensor symbol {row['symbol']}.",
            preconditions="Input tensors, shapes, dtypes, layouts, devices, gradient mode, and memory plan satisfy the exact overload used at the cited call site.",
            inputs_defaults=joined(f"usage={row['usage_kinds']}", f"production_calls={row['production_call_count']}", f"test_calls={row['test_call_count']}", f"resolution={row['resolution']}"),
            permissions_flags="Worker-owned tensor capability; plugins receive only opaque handles and explicitly allowed host operations.",
            observable_success=f"The source invokes {row['symbol']} at {row['production_call_count']} production call sites with source classifications {row['source_classifications']}.",
            interaction_accessibility="Compute-only contract; progress, cancellation, capability errors, and node association are visible through consuming execution surfaces.",
            state_concurrency="Tensor storage, views, streams, gradient state, RNG, cancellation, and allocator lifetime are worker-owned and attempt-scoped.",
            failure_recovery=joined(row["shape_requirement"], row["dtype_requirement"], row["layout_requirement"], row["device_requirement"], row["numerics_requirement"], row["cancellation_requirement"]),
            persistence_serialization="Live tensor storage is not durable; operation ID/version, descriptor, inputs, artifact/cache identity, RNG phase, and committed outputs are durable where required.",
            interfaces_side_effects=joined(row.get("vjp_jvp_requirement"), row.get("cancellation_requirement")),
            platform_localization_variants=row["device_requirement"],
            current_sim_status="missing",
            parity_gap=f"Sim has no Comfy tensor facade or native implementation/conformance matrix for {row['symbol']}.",
            parity_decision=row["native_rust_decision"],
            observable_sim_acceptance=joined(row["shape_requirement"], row["dtype_requirement"], row["layout_requirement"], row["device_requirement"], row["numerics_requirement"], row["vjp_jvp_requirement"], row["cancellation_requirement"]),
            automated_validation=f"Parameterize VAL-TENSOR-001 for {operation_id}; include VAL-AUTOGRAD-001 when reachable from the autograd ledger and VAL-DEVICE-001 for every certified backend.",
            manual_validation=f"Compare {row['symbol']} at representative cited call sites using source-fingerprinted CPU and certified device fixtures; inspect effective dtype/layout/device, numerical tolerance, cancellation, and errors.",
            open_questions=row["limitations"], source_catalog="backend-tensor-operations.csv", source_row=index,
        ))

    for index, row in enumerate(read_rows("backend-autograd.csv"), 2):
        autograd_id = row["autograd_id"]
        features.append(base_feature(
            feature_id=autograd_id, product="ComfyUI", domain="autograd", name=f"{row['construct']}: {row['symbol']}",
            classification="autograd construct", availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=joined(row.get("production_sites"), row.get("method_or_state_sites"), row.get("apply_sites")), source_symbol=row["symbol"],
            test_evidence=first(row, "test_sites", fallback="No focused test site was resolved for this autograd construct."),
            actor="Native model, training node, custom operation, or gradient-dependent sampler.", trigger=f"Execution reaches autograd construct {row['symbol']}.",
            preconditions="Gradient mode, tape lifetime, saved tensors, dtype/device, memory, and source reachability permit the construct.",
            inputs_defaults=joined(f"production_uses={row['production_use_count']}", f"resolution={row['resolution']}"),
            permissions_flags="Worker-owned gradient capability; no plugin receives raw tape or device pointers.",
            observable_success=joined(row["forward_contract"], row["reverse_contract"]),
            interaction_accessibility="Compute-only; consuming node exposes progress, cancellation, training state, validation, errors, and output association.",
            state_concurrency=row["state_and_lifetime_requirement"], failure_recovery=joined(row["native_requirement"], row["state_and_lifetime_requirement"]),
            persistence_serialization="Live tapes and saved tensors are not durable; explicitly restartable training state uses versioned native model/optimizer records only.",
            interfaces_side_effects=joined(row.get("apply_sites"), row.get("method_or_state_sites")),
            platform_localization_variants="Forward and reverse contracts apply per supported dtype/device backend; unavailable combinations remain explicit.",
            current_sim_status="missing", parity_gap=f"Sim has no native Comfy autograd implementation for {row['symbol']}.",
            parity_decision="implement-in-the-sim-owned-native-autograd-engine-with-no-python-fallback",
            observable_sim_acceptance=joined(row["forward_contract"], row["reverse_contract"], row["native_requirement"], row["state_and_lifetime_requirement"]),
            automated_validation=f"Parameterize VAL-AUTOGRAD-001 for {autograd_id}, including analytical/finite-difference, repeat/backward lifetime, cancellation, and supported device cases.",
            manual_validation=f"Compare forward, VJP/gradient, tape lifetime, memory, cancellation, and error behavior for {row['symbol']} at each reachable family.",
            open_questions=row["limitations"], source_catalog="backend-autograd.csv", source_row=index,
        ))

    for index, row in enumerate(read_rows("backend-rng.csv"), 2):
        rng_id = row["rng_id"]
        features.append(base_feature(
            feature_id=rng_id, product="ComfyUI", domain="random number generation", name=f"{row['phase']}: {row['symbol']}",
            classification="RNG phase contract", availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=joined(row.get("production_call_sites"), row.get("test_call_sites"), row.get("support_call_sites")), source_symbol=row["symbol"],
            test_evidence=first(row, "test_call_sites", fallback="No focused test site was resolved for this RNG phase."),
            actor="Native node, sampler, model, augmentor, or plugin requesting randomness.", trigger=f"Execution requests phase `{row['phase']}` through {row['symbol']}.",
            preconditions="A versioned workflow/attempt/node/phase seed identity and supported device RNG contract are available.",
            inputs_defaults=joined(f"seededness={row['seededness']}", f"seeds={row['seed_expressions']}", f"generators={row['generator_expressions']}", f"devices={row['device_expressions']}"),
            permissions_flags="Plugins receive randomness only through an explicit grant and named host phase stream; global/process RNG is forbidden.",
            observable_success=f"The source uses RNG phase `{row['phase']}` at {row['production_call_count']} production sites with {row['seededness']} semantics.",
            interaction_accessibility="Compute-only; seed controls, deterministic/effective mode, cancellation, and retry identity are visible through consuming node/runtime surfaces.",
            state_concurrency=joined(row["phase_identity_requirement"], row["state_requirement"]),
            failure_recovery=row["cancellation_retry_requirement"], persistence_serialization=joined(row["phase_identity_requirement"], row["state_requirement"]),
            interfaces_side_effects=joined(row["seed_mapping_requirement"], row["device_requirement"]), platform_localization_variants=row["device_requirement"],
            current_sim_status="missing", parity_gap=f"Sim has no versioned native RNG phase implementation for `{row['phase']}` / {row['symbol']}.",
            parity_decision=row["native_rust_decision"],
            observable_sim_acceptance=joined(row["phase_identity_requirement"], row["seed_mapping_requirement"], row["state_requirement"], row["device_requirement"], row["cancellation_retry_requirement"]),
            automated_validation=f"Parameterize VAL-RNG-001 for {rng_id}; compare exact CPU sequences where specified and distribution/checkpoint/device/cancel/retry behavior elsewhere.",
            manual_validation=f"Compare phase `{row['phase']}` source and native streams across seed, batch, node, retry, cancellation, and certified devices without cross-phase perturbation.",
            open_questions=row["limitations"], source_catalog="backend-rng.csv", source_row=index,
        ))


def add_backend_formats(features: list[dict[str, str]]) -> None:
    name = "backend-formats.csv"
    for index, row in enumerate(read_rows(name), 2):
        features.append(base_feature(
            feature_id=row["feature_id"], product=first(row, "product", fallback="ComfyUI"),
            domain="format", name=row["name"], classification=row["classification"],
            availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=row["source_evidence"], source_symbol=row["source_evidence"],
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            actor="Workflow author, API client, server, frontend, importer, exporter, or media consumer.",
            trigger=f"Parse, serialize, migrate, embed, extract, or transmit {row['name']}.",
            preconditions="Input bytes or structured data are available and applicable metadata/config flags permit the operation.",
            inputs_defaults=row["schema_format"], permissions_flags=f"Availability: {row['availability']}; metadata suppression, path, content, and remote trust rules apply.",
            observable_success=row["serialization_behavior"],
            interaction_accessibility="Import/export controls shall be keyboard-operable and expose format, destructive migration, malformed data, and retained-original states in text.",
            state_concurrency="Parsing is bounded and non-destructive; saving uses one immutable snapshot and does not race dirty edits or external changes.",
            failure_recovery=row["failure_recovery"], persistence_serialization=joined(row["schema_format"], row["migration_backward_compatibility"]),
            interfaces_side_effects=row["serialization_behavior"], platform_localization_variants=row["migration_backward_compatibility"],
            parity_gap=f"Sim has no native lossless adapter for {row['name']}.",
            observable_sim_acceptance=row["acceptance_criteria"], automated_validation=row["validation"],
            manual_validation=f"Round-trip {row['name']} side by side, inspect embedded/serialized fields, and exercise corrupt, truncated, unknown-version, metadata-disabled, and external-change cases.",
            open_questions=row["open_questions"], source_catalog=name, source_row=index,
        ))


def add_backend_external_services(features: list[dict[str, str]]) -> None:
    name = "backend-external-services.csv"
    for index, row in enumerate(read_rows(name), 2):
        route_name = f"{row['method']} {row['path']}"
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="external-service",
            name=f"{row['provider']}: {route_name}", classification=row["classification"],
            availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=row["source_evidence"], source_symbol=joined(row.get("uses"), row.get("node_feature_ids")),
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            actor="API-node workflow author, Comfy hosted proxy, or provider client.", trigger=row["trigger"],
            preconditions="The API node is registered; user authorization, entitlement/credits, service availability, allowed media, and provider-specific inputs are valid.",
            inputs_defaults=joined(route_name, row.get("uses"), row.get("node_feature_ids")), permissions_flags=row["auth"],
            observable_success=row["success_behavior"],
            interaction_accessibility="API-node fields, consent/cost, progress, cancellation, and errors shall be labeled and keyboard-operable; provider wire traffic has no direct GUI contract.",
            state_concurrency="Sync, polling, upload/download, progress, retry/backoff, interruption, and optional provider cancellation follow the cited API-node client.",
            failure_recovery=row["error_retry_cancel"], persistence_serialization="Provider inputs and returned typed media/node outputs remain in workflow/prompt/output records; secret values are references and never workflow bytes.",
            interfaces_side_effects=joined(route_name, row.get("success_behavior")),
            platform_localization_variants="Hosted availability, billing, models, pricing, region, policy, provider version, and account state may differ; none was called at runtime in this audit.",
            parity_gap=f"Sim has no authorized native provider contract for {route_name}.",
            observable_sim_acceptance=f"When an approved provider fixture is enabled, {route_name} shall match authentication separation, request/response schema, progress, bounded polling/retry, interruption/cancellation, cost/error display, media handling, and secret redaction; otherwise Sim shall preserve the node and explain deferral.",
            automated_validation=f"Use a non-network provider fake for {row['feature_id']} covering success, invalid input, 408/429/5xx, Retry-After, timeout, cancellation, malformed media, and unavailable entitlement.",
            manual_validation="Do not use a real paid account in baseline validation; inspect gated UI, consent/cost text, secret handling, and disabled/offline behavior with fixtures.",
            open_questions="Live provider, billing, entitlement, regional, retention, and cancellation guarantees require an approved service contract before implementation.",
            source_catalog=name, source_row=index,
        ))


def add_backend_schemas(features: list[dict[str, str]]) -> None:
    name = "backend-schemas.csv"
    for index, row in enumerate(read_rows(name), 2):
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI", domain="schema",
            name=row["schema_name"], classification=row["classification"], availability=row["availability"],
            evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=f"{row['source_file']}:{row['source_line']}", source_symbol=row["source_symbol"],
            test_evidence=first(row, "test_evidence", fallback="No focused existing test was located."),
            documentation=("OpenAPI schema declaration." if "OpenAPI" in row["classification"] else "No separate documentation-only claim was used."),
            actor="API client, frontend, Desktop host, server, database adapter, or migration consumer.",
            trigger=f"Encode, decode, validate, persist, or migrate schema {row['schema_name']}.",
            preconditions="The route, protocol, database, migration, or persisted artifact using the schema is available.",
            inputs_defaults=joined(f"fields={row['fields']}", f"required={row['required']}", f"bases={first(row, 'bases', fallback='none')}") ,
            permissions_flags=f"Availability: {row['availability']}; field-level secrets, user scope, paths, and privileged operations follow consuming contract policy.",
            observable_success=f"Schema {row['schema_name']} accepts and emits its cataloged fields, required set, inheritance, and unknown/version behavior for consumers: {first(row, 'used_by', fallback='not statically resolved')}.",
            interaction_accessibility="Schema-only contract; visible fields and validation errors inherit the consuming surface's labels, focus, keyboard, and accessibility semantics.",
            state_concurrency="Versioned schema values are immutable per request/event/persistence transaction; stale or incompatible values do not partially mutate live state.",
            failure_recovery="Missing required, invalid type/value, malformed, future-version, and unknown-field behavior shall match the consuming contract and preserve original data when migration is unsafe.",
            persistence_serialization=joined(row["fields"], row["required"], row["bases"]),
            interfaces_side_effects=f"Used by: {first(row, 'used_by', fallback='usage not statically resolved')}",
            platform_localization_variants="Field names are protocol/persistence identifiers; availability and consumers may be version, route, database, asset, cloud, or feature dependent.",
            parity_gap=f"Sim has no native typed/lossless adapter for schema {row['schema_name']}.",
            observable_sim_acceptance=f"Valid, boundary, missing-required, wrong-type, unknown-field, malformed, and future-version fixtures for {row['schema_name']} shall encode/decode without silent loss and match every consuming route/event/persistence status and error.",
            automated_validation=f"Generate schema contract and round-trip cases for {row['feature_id']} from fields, required set, bases, and consumers.",
            manual_validation=f"Inspect a representative {row['schema_name']} payload in source and Sim developer diagnostics; verify sensitive fields are redacted.",
            open_questions=("Availability is uncertain until the documented OpenAPI component is reconciled with a reachable runtime consumer." if row["availability"] == "uncertain" else "No schema-specific question beyond unavailable runtime consumers and future-version behavior."),
            source_catalog=name, source_row=index,
        ))


def add_frontend(features: list[dict[str, str]]) -> None:
    name = "frontend-features.csv"
    for index, row in enumerate(read_rows(name), 2):
        features.append(base_feature(
            feature_id=row["feature_id"], product=row["product"], domain=row["domain"], name=row["name"],
            classification=row["classification"], availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=joined(row["source_file"], row["symbol"]), source_symbol=row["symbol"],
            test_evidence=row["test"], documentation=row["documentation"], runtime_observation=row["runtime_observation"],
            actor=row["actor"], trigger=row["trigger"], preconditions=row["preconditions"], inputs_defaults=row["inputs_defaults"],
            permissions_flags=row["permissions_flags"], observable_success=row["success_behavior"],
            interaction_accessibility=row["interaction_accessibility"], state_concurrency=row["state_concurrency"],
            failure_recovery=row["error_recovery"], persistence_serialization=row["persistence_serialization"],
            interfaces_side_effects=row["interfaces_side_effects"], platform_localization_variants=row["platform_localization_variants"],
            parity_gap=f"Sim has no native Rust/GPUI implementation of frontend contract `{row['name']}`.", observable_sim_acceptance=row["sim_acceptance"], automated_validation=row["automated_validation"],
            manual_validation=row["manual_validation"], open_questions=row["open_questions_assumptions"],
            source_catalog=name, source_row=index,
        ))


def add_frontend_component_surfaces(features: list[dict[str, str]]) -> None:
    name = "frontend-component-surfaces.csv"
    for index, row in enumerate(read_rows(name), 2):
        infrastructure = row["classification"] == "infrastructure-only"
        features.append(base_feature(
            feature_id=row["feature_id"],
            product=row["product"],
            domain=row["domain"],
            name=row["name"],
            classification=row["classification"],
            availability=row["availability"],
            evidence_level=row["evidence_level"],
            confidence=row["confidence"],
            source_evidence=row["source_evidence"],
            source_symbol=joined(row["source_symbol"], row["source_excerpt"]),
            test_evidence=row["test_evidence"],
            documentation="No documentation-only claim is used; this row closes a concrete Vue SFC source mapping.",
            runtime_observation="Not dynamically rerun because frontend dependencies are absent; exact static bindings, handlers, models, states, and adjacent tests are retained in the component-surface catalog.",
            actor=row["actor"],
            trigger=row["trigger"],
            preconditions=row["preconditions"],
            inputs_defaults=row["inputs_defaults"],
            permissions_flags=row["feature_flags_permissions"],
            observable_success=row["observable_success"],
            interaction_accessibility=row["interaction_accessibility"],
            state_concurrency=row["state_transitions_concurrency"],
            failure_recovery=row["failure_recovery"],
            persistence_serialization=row["persistence_serialization"],
            interfaces_side_effects=row["interfaces_side_effects"],
            platform_localization_variants=row["platform_localization_variants"],
            parity_gap=(
                f"This source-specific presentational contract is not separately mapped through a consuming Sim Comfy surface: {row['infrastructure_disposition_reason']}"
                if infrastructure
                else f"Sim has no Comfy-specific GPUI surface for the exact {row['source_file']} interaction and state contract."
            ),
            observable_sim_acceptance=row["observable_sim_acceptance"],
            automated_validation=row["automated_validation"],
            manual_validation=row["manual_validation"],
            open_questions=row["open_questions"],
            parity_decision=component_surface_disposition(row["feature_id"], row["domain"]),
            source_catalog=name,
            source_row=index,
        ))


def add_frontend_functional_modules(features: list[dict[str, str]]) -> None:
    name = "frontend-functional-modules.csv"
    for index, row in enumerate(read_rows(name), 2):
        infrastructure = row["disposition"] == "infrastructure-only"
        features.append(base_feature(
            feature_id=row["feature_id"],
            product=row["product"],
            domain=row["domain"],
            name=f"{row['primary_symbol']} module contract",
            classification=row["classification"],
            availability=row["availability"],
            evidence_level=row["evidence_level"],
            confidence=row["confidence"],
            source_evidence=row["source_evidence"],
            source_symbol=row["primary_symbol"],
            test_evidence=row["test_evidence"],
            documentation="No documentation-only claim was used; this contract comes from the cited production TypeScript module and any exact same-module test.",
            runtime_observation="Not dynamically observed in this audit; source-level state, effects, and failure boundaries remain explicit side-by-side validation targets.",
            actor=row["actor"],
            trigger=row["trigger"],
            preconditions=row["preconditions"],
            inputs_defaults=row["inputs_defaults_flags"],
            permissions_flags=joined(
                f"availability={row['source_availability']}",
                f"disposition={row['disposition']}",
                row["inputs_defaults_flags"],
            ),
            observable_success=row["observable_success"],
            interaction_accessibility=(
                "No direct surface is owned by this infrastructure module; consuming graph, asset, workflow, queue, account, or Desktop UI retains its cataloged keyboard, pointer, focus, status, and accessibility behavior."
                if infrastructure
                else "The consuming surface must expose the module's loading, success, empty, disabled, error, and recovery state with keyboard-equivalent actions, visible focus, accessible names/status, and focus restoration; pointer/drag semantics follow the cited domain contract."
            ),
            state_concurrency=row["state_concurrency"],
            failure_recovery=row["error_cancel_retry_recovery"],
            persistence_serialization=row["persistence_serialization"],
            interfaces_side_effects=row["interfaces_side_effects"],
            platform_localization_variants=row["platform_cloud_variants"],
            parity_gap=(
                f"This source module is infrastructure rather than a separately ported surface; Sim must preserve {row['infrastructure_disposition']} through its consuming capability and validation fixture."
                if infrastructure
                else f"Sim has no Comfy-specific Rust/GPUI module reproducing {row['primary_symbol']}'s source-specific state, transition, failure, and side-effect contract."
            ),
            observable_sim_acceptance=row["independent_validation"],
            automated_validation=row["independent_validation"],
            manual_validation=f"Exercise {row['primary_symbol']} through {row['broad_anchor']} in the source frontend and the trace-linked Sim surface; compare visible state, ordering, errors, recovery, persistence, and side effects.",
            open_questions="Runtime-only dependency responses, browser/OS timing, remote/cloud entitlement, and dynamically registered extension behavior remain unverified unless the row cites a focused existing test.",
            source_catalog=name,
            source_row=index,
        ))


def frontend_menu_test_evidence(row: dict[str, str]) -> str:
    if row["evidence_level"] != "test-backed":
        return "No focused existing test was located."

    feature_id = row["feature_id"]
    if not feature_id.startswith("COMFY-MENU-"):
        parent = next(
            (candidate for candidate in read_rows("frontend-features.csv") if candidate["feature_id"] == feature_id),
            None,
        )
        return first(parent or {}, "test", fallback="No focused existing test was located.")

    number = int(feature_id.rsplit("-", 1)[1])
    if 1 <= number <= 11:
        return "src/composables/queue/useJobMenu.test.ts"
    if 13 <= number <= 24:
        return "browser_tests/tests/sidebar/assets.spec.ts; src/platform/assets/composables/useMediaAssetActions.test.ts"
    if 25 <= number <= 28:
        return "src/platform/workspace/components/dialogs/settings/WorkspaceMenuButton.test.ts; src/platform/workspace/components/SubscriptionPanelContentWorkspace.test.ts"
    if 29 <= number <= 32:
        return "browser_tests/tests/dialogs/memberRoleChange.spec.ts"
    if 33 <= number <= 36:
        return "browser_tests/tests/dialogs/keybindingPanel.spec.ts"
    if 45 <= number <= 56:
        return "browser_tests/tests/helpCenter.spec.ts"
    if 57 <= number <= 59:
        return "browser_tests/tests/sidebar/jobHistory.spec.ts; browser_tests/tests/dialogs/queueClearHistory.spec.ts"
    if 60 <= number <= 62:
        return "browser_tests/tests/queueButtonModes.spec.ts"
    if 63 <= number <= 64:
        return "browser_tests/tests/canvasModeSelector.spec.ts"
    if 65 <= number <= 69:
        return "src/platform/assets/components/MediaAssetFilterMenu.test.ts"
    if 70 <= number <= 75:
        return "src/platform/assets/components/MediaAssetSettingsMenu.test.ts"
    if 76 <= number <= 77:
        return "browser_tests/tests/sidebar/nodeLibraryV2.spec.ts"
    if number in {79, 80, 81, 82}:
        return "browser_tests/tests/menu.spec.ts; browser_tests/tests/customIcons.spec.ts"
    if 114 <= number <= 118:
        return "browser_tests/tests/rightClickMenu.spec.ts; browser_tests/tests/nodeSearchBox.spec.ts"
    if 119 <= number <= 133:
        return "browser_tests/tests/vueNodes/interactions/node/contextMenu.spec.ts"
    if 134 <= number <= 138:
        return "browser_tests/tests/vueNodes/groups/groups.spec.ts"
    if 139 <= number <= 141:
        return "browser_tests/tests/subgraph/subgraphSlots.spec.ts"
    if number == 142:
        return "browser_tests/tests/rerouteNode.spec.ts"
    if 144 <= number <= 146:
        return "browser_tests/tests/nodeTemplates.spec.ts"
    if 152 <= number <= 154:
        return "browser_tests/tests/rightClickMenu.spec.ts; browser_tests/tests/vueNodes/groups/groups.spec.ts"
    if number == 163:
        return "browser_tests/tests/subgraph/subgraphBreadcrumb.spec.ts"
    if number == 165:
        return "browser_tests/tests/selectionToolboxMoreActions.spec.ts; browser_tests/tests/selectionToolboxSubmenus.spec.ts"
    return "No focused existing test was located; evidence level requires reconciliation."


def add_frontend_supplemental(features: list[dict[str, str]]) -> None:
    persisted_name = "frontend-persisted-state.csv"
    for index, row in enumerate(read_rows(persisted_name), 2):
        if not row["feature_id"].startswith("COMFY-PERSIST-"):
            continue
        security = first(
            row,
            "security_classification",
            fallback="No sensitive value was identified by the source scan.",
        )
        recovery = first(
            row,
            "recovery_behavior",
            fallback="Missing or malformed state follows the source-defined fallback.",
        )
        secret_bearing = "secret-bearing" in security.casefold()
        features.append(base_feature(
            feature_id=row["feature_id"],
            product=(
                "ComfyUI-Frontend website"
                if row["key_or_pattern"] == "closedBanners"
                else "ComfyUI-Frontend"
            ),
            domain="persistence",
            name=f"Persisted browser state {row['storage']} {row['key_or_pattern']}",
            classification="persisted browser state",
            availability=row["availability"],
            evidence_level=row["evidence_level"],
            confidence="high",
            source_evidence=first(row, "source_evidence", fallback=row["purpose"]),
            source_symbol=row["key_or_pattern"],
            test_evidence=(
                first(row, "source_evidence")
                if row["evidence_level"] == "test-backed"
                else "No focused existing test was located."
            ),
            actor="User, authentication flow, workspace service, billing flow, or UI persistence service.",
            trigger=f"Read, write, consume, migrate, or clear {row['key_or_pattern']}.",
            preconditions=f"{row['storage']} is available and the {row['availability']} capability is enabled.",
            inputs_defaults=joined(
                f"key/pattern={row['key_or_pattern']}",
                f"value={first(row, 'value_shape', fallback='source-defined serialized value')}",
            ),
            permissions_flags=joined(f"availability={row['availability']}", security),
            observable_success=row["purpose"],
            interaction_accessibility="Persistence has no direct interaction surface; consuming controls retain labels, focus, keyboard operation, and visible failure feedback.",
            state_concurrency="Storage changes become visible according to the cited VueUse/watch/storage-event or explicit read/write path; one-shot markers are consumed atomically by the source flow.",
            failure_recovery=recovery,
            persistence_serialization=joined(
                f"storage={row['storage']}",
                f"key={row['key_or_pattern']}",
                first(row, "value_shape"),
            ),
            interfaces_side_effects="Writes or removes the named browser-storage entry; consuming auth, workspace, billing, editor, or UI state may update immediately.",
            platform_localization_variants="Browser storage availability and distribution gates apply; key names and serialized values are not localized.",
            parity_gap=(
                "Sim lacks a secure credential-reference import/migration path for this raw browser secret and must not reproduce insecure raw-key persistence."
                if secret_bearing
                else "Sim has no Comfy-specific persisted-state adapter for this key or dynamic pattern."
            ),
            observable_sim_acceptance=(
                f"Sim shall import or consume {row['key_or_pattern']} only through an explicit compatibility flow, move secret bytes into the platform secret provider, persist only an opaque reference, redact diagnostics, and match source expiry/clear/error behavior."
                if secret_bearing
                else f"Sim shall preserve the observable defaults, value shape, update timing, restart scope, malformed-value fallback, and clear/consume behavior of {row['key_or_pattern']}."
            ),
            automated_validation=f"Round-trip valid, absent, malformed, unavailable-storage, concurrent-update, restart, and clear/consume fixtures for {row['feature_id']}; assert secret redaction when applicable.",
            manual_validation=f"Exercise the source consumer for {row['key_or_pattern']}; inspect its visible restoration/failure behavior and compare with Sim without exposing sensitive values.",
            open_questions="Browser-to-native migration lifetime and secure-secret import policy require an explicit compatibility decision for sensitive entries." if secret_bearing else "No additional question beyond browser/native storage lifetime and migration compatibility.",
            source_catalog=persisted_name,
            source_row=index,
        ))

    telemetry_name = "frontend-telemetry.csv"
    for index, row in enumerate(read_rows(telemetry_name), 2):
        if not row["feature_id"].startswith("COMFY-TELEMETRY-"):
            continue
        if row["entry_kind"] == "ui_button_id":
            product = "ComfyUI-Frontend"
            payload = f"{{button_id: {row['identifier']}, element_group: source-defined group}}"
        elif row["feature_id"] == "COMFY-TELEMETRY-001":
            product = "ComfyUI-Frontend desktop-ui"
            payload = "{step: string | number}"
        elif row["feature_id"] == "COMFY-TELEMETRY-002":
            product = "ComfyUI-Frontend"
            payload = "{status: lowercased task display status}; also increments execution:{status}"
        elif row["feature_id"] == "COMFY-TELEMETRY-004":
            product = "ComfyUI-Frontend website"
            payload = "{platform}"
        else:
            product = "ComfyUI-Frontend website"
            payload = "PostHog page context"
        features.append(base_feature(
            feature_id=row["feature_id"], product=product, domain="telemetry",
            name=f"Telemetry event {row['wire_name']}", classification="telemetry event",
            availability=row["availability"], evidence_level=row["evidence_level"], confidence="high",
            source_evidence=row["source_file"], source_symbol=row["identifier"],
            test_evidence=("apps/website/src/scripts/posthog.test.ts" if row["evidence_level"] == "test-backed" else "No focused existing test was located."),
            actor="Frontend, Desktop UI, or website telemetry producer.", trigger=f"Reach the source transition that emits {row['wire_name']}.",
            preconditions="The applicable distribution is active and its telemetry provider/consent gate is initialized.",
            inputs_defaults=f"wire event={row['wire_name']}; payload={payload}.",
            permissions_flags=f"availability={row['availability']}; telemetry consent/configuration and provider availability apply.",
            observable_success=f"Emit {row['wire_name']} with {payload} without changing product behavior when telemetry is disabled or fails.",
            interaction_accessibility="Telemetry is noninteractive and must not alter focus, labels, keyboard behavior, timing-critical feedback, or accessibility state.",
            state_concurrency="Emission is error-isolated; duplicate prevention and transition detection follow the cited producer.",
            failure_recovery="Uninitialized or failing telemetry is ignored/logged and never blocks navigation, installation, execution completion, or download.",
            persistence_serialization="Provider-defined analytics retention only; Sim shall not place telemetry payloads in workflow/project persistence.",
            interfaces_side_effects=f"Optional analytics-provider event {row['wire_name']}.",
            platform_localization_variants="Distribution/provider/consent differences apply; event identifiers are stable and unlocalized.",
            parity_gap="Sim has no Comfy-specific consent-gated event mapping for this source event.",
            observable_sim_acceptance=f"With telemetry enabled and a fake provider, Sim emits {row['wire_name']} once with the matching payload at the same state transition; with telemetry disabled or failing, behavior is unchanged.",
            automated_validation=f"Capture {row['wire_name']} with a fake telemetry provider and assert payload, cardinality, gate, and failure isolation.",
            manual_validation="Inspect consent-disabled and provider-failure states without using a real account or external analytics mutation.",
            open_questions="Provider retention, production endpoint, and consent policy require approved service configuration.",
            source_catalog=telemetry_name, source_row=index,
        ))

    menu_name = "frontend-menus.csv"
    for index, row in enumerate(read_rows(menu_name), 2):
        if not row["feature_id"].startswith("COMFY-MENU-"):
            continue
        availability = first(row, "availability", fallback="active")
        item_kind = first(row, "item_kind", fallback="action")
        if row["menu_surface"].startswith("website"):
            product = "ComfyUI-Frontend website"
        elif row["menu_surface"].startswith("desktop-ui"):
            product = "ComfyUI-Frontend desktop-ui"
        else:
            product = "ComfyUI-Frontend"
        features.append(base_feature(
            feature_id=row["feature_id"], product=product, domain="menu",
            name=f"{row['menu_surface']}: {row['label_or_action']}",
            classification=("infrastructure-only" if availability == "infrastructure-only" else f"menu {item_kind}"),
            availability=availability, evidence_level=row["evidence_level"], confidence="high",
            source_evidence=row["source_file"], source_symbol=row["item_id"],
            test_evidence=frontend_menu_test_evidence(row),
            actor="User, extension, or menu registry consumer.",
            trigger=f"Open {row['menu_surface']} at {row['menu_path']} and activate {row['label_or_action']}.",
            preconditions=first(row, "condition", fallback="The owning surface is visible and enabled."),
            inputs_defaults=f"item={row['item_id']}; kind={item_kind}; path={row['menu_path']}.",
            permissions_flags=f"availability={availability}; condition={first(row, 'condition', fallback='none')}.",
            observable_success=first(row, "action_or_target", fallback=f"Expose and invoke {row['label_or_action']} as defined by the cited source."),
            interaction_accessibility="Match pointer/context-menu trigger, keyboard traversal/activation, focus restoration, submenu/disabled/check state, accessible name/role, Escape and outside-click dismissal of the cited surface.",
            state_concurrency="Menu visibility/check/disabled state recomputes from cited stores; async actions preserve pending/error state and do not double-dispatch.",
            failure_recovery="Disabled items do not dispatch; async failures use the cited visible error path; destructive actions retain confirmation and cancellation behavior.",
            persistence_serialization="Menu state is transient unless the action changes a separately cataloged workflow, setting, browser-storage, asset, queue, workspace, or account record.",
            interfaces_side_effects=joined(first(row, "action_or_target"), first(row, "consumer_source")),
            platform_localization_variants=f"availability={availability}; labels localize through the source catalog except protocol/developer literals.",
            parity_gap="Sim has no Comfy-specific GPUI menu/action/focus implementation for this item or infrastructure contract.",
            observable_sim_acceptance=f"Under the same condition, Sim exposes {row['label_or_action']} on the corresponding GPUI menu surface with matching state, action, focus, keyboard, dismissal, confirmation, and error behavior.",
            automated_validation=f"Add a deterministic GPUI menu test for {row['feature_id']} covering visibility, enabled/check state, keyboard/pointer activation, action cardinality, focus return, and applicable failure/confirmation.",
            manual_validation=f"Compare {row['menu_surface']} > {row['menu_path']} > {row['label_or_action']} side by side in source and Sim.",
            open_questions="Dynamic extension/data-driven entries require fixture enumeration at runtime; this row records the static producer or consumer boundary.",
            source_catalog=menu_name, source_row=index,
        ))

    route_name = "frontend-routes.csv"
    for index, row in enumerate(read_rows(route_name), 2):
        if not row["feature_id"].startswith("COMFY-ROUTE-"):
            continue
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI-Frontend desktop-ui", domain="route",
            name=f"Desktop UI route {row['route']} ({row['name']})", classification="route",
            availability=row["availability"], evidence_level=row["evidence_level"], confidence="high",
            source_evidence=row["source_file"], source_symbol=row["name"],
            actor="Desktop user or Desktop shell.", trigger=f"Navigate Desktop UI to {row['route']}.",
            preconditions="Desktop UI router is initialized under file, Electron, or hosted protocol.",
            inputs_defaults=f"surface={row['surface']}; path={row['route']}; name={row['name']}.",
            observable_success=row["behavior"],
            interaction_accessibility="Route entry moves focus to the page heading/first-run controls and remains keyboard/screen-reader operable.",
            state_concurrency="Lazy view import and navigation resolve once; repeated navigation does not duplicate first-run state.",
            failure_recovery="Failed lazy import or invalid navigation surfaces a recoverable Desktop error without losing installation state.",
            persistence_serialization="History mode depends on file protocol; installation/onboarding state persists through the Desktop bridge rather than URL alone.",
            interfaces_side_effects="Vue Router navigation and lazy WelcomeView module load.",
            platform_localization_variants="File protocol uses hash history; Electron/non-file uses base-aware web history; view text is localized.",
            parity_gap="Sim has no Comfy Desktop first-run route equivalent wired to native GPUI navigation.",
            observable_sim_acceptance="Fresh Desktop launch reaches the Welcome/source-choice surface at the root route equivalent on every supported platform and can continue/recover without URL-history assumptions.",
            automated_validation="Launch Desktop UI at file and hosted root fixtures; assert WelcomeView and first-run actions, then port as a GPUI interaction test.",
            manual_validation="Compare fresh-launch root navigation and focus on each supported Desktop platform.",
            open_questions="Native Sim may not expose URL routes; it still requires the same externally observable first-run navigation state.",
            source_catalog=route_name, source_row=index,
        ))

    http_name = "frontend-http-usage.csv"
    for index, row in enumerate(read_rows(http_name), 2):
        if not row["feature_id"].startswith("COMFY-HTTP-CLIENT-"):
            continue
        features.append(base_feature(
            feature_id=row["feature_id"], product="ComfyUI-Frontend", domain="http-client",
            name=f"Frontend contract {row['method']} {row['route']}", classification="HTTP/static-resource client contract",
            availability=row["availability"], evidence_level=row["evidence_level"], confidence="high",
            source_evidence=f"{row['source_file']}:{row['line']}", source_symbol=row["access_helper"],
            actor="Frontend API, OAuth, or template service.", trigger=f"Request or construct {row['route']} through {row['access_helper']}.",
            preconditions=f"The {row['availability']} server/static-resource surface is reachable and required credentials/path inputs are valid.",
            inputs_defaults=f"method={row['method']}; route={row['route']}; helper={row['access_helper']}.",
            permissions_flags="Same-origin credentials apply to OAuth; template paths are unprivileged but traversal/extension validation applies where cited.",
            observable_success=f"The client consumes or constructs {row['method']} {row['route']} with the source helper, response validation, fallback, and media/JSON behavior.",
            interaction_accessibility="Protocol-only contract; consuming OAuth/template UI exposes loading, failure, retry, labels, and focus behavior separately.",
            state_concurrency="Requests follow source async ordering; localized template fallback and repeated OAuth submission do not silently duplicate state changes.",
            failure_recovery="Match non-OK, malformed JSON/content-type, localized-index fallback, path rejection, cookie/auth redirect, and unavailable-resource behavior from the cited consumer.",
            persistence_serialization="OAuth challenge/decision remains transient; loaded template JSON/media enters separately cataloged workflow/template persistence.",
            interfaces_side_effects=f"{row['method']} {row['route']} via {row['access_helper']}.",
            platform_localization_variants="Localized template index selection and same-origin Cloud ingress/proxy behavior apply; Desktop/remote base URL mapping must preserve semantics.",
            parity_gap="Sim lacks a typed Comfy-specific client/static-resource adapter for this contract.",
            observable_sim_acceptance=f"Against an identical deterministic server/static fixture, Sim issues or resolves {row['method']} {row['route']} with matching credentials, path validation, parsing, fallback, error, and retry behavior.",
            automated_validation=f"Add protocol/static-resource fixtures for {row['feature_id']} covering success, missing, wrong content type, malformed body, auth redirect, traversal attempt, and fallback where applicable.",
            manual_validation="Inspect OAuth consent or template browsing with a local fake only; do not use real accounts or paid services.",
            open_questions="Hosted OAuth ingress/cookie and production template-cache headers remain runtime-unverified.",
            source_catalog=http_name, source_row=index,
        ))


def desktop_native_target(row: dict[str, str]) -> dict[str, str]:
    source_contract = joined(
        row["domain"],
        row["name"],
        row["classification"],
        row["observable_behavior"],
        row["persistence_side_effects"],
    ).casefold()
    remote_comfy = any(
        token in source_contract
        for token in (
            "external comfyui",
            "remote endpoint",
            "remote url",
            "remote entry",
            "externally managed comfyui",
        )
    )
    legacy_plugin = any(
        token in source_contract
        for token in ("custom-node", "custom node", "comfyui-manager", "manager configuration")
    )
    legacy_engine = any(
        token in source_contract
        for token in (
            "python",
            "pip",
            "main.py",
            "managed comfyui",
            "comfyui launch",
            "stop comfyui",
            "restart comfyui",
            "portable comfyui",
            "comfyui installation",
            "stable comfyui release",
            "local server",
            "core checkout",
        )
    )
    hosted_web_shell = any(
        token in source_contract
        for token in (
            "hosted frontend",
            "hosted comfy surface",
            "electron renderer",
            "electron webview",
            "javascript extension",
            "node.js runtime",
        )
    )
    legacy_source_registry = row["domain"] == "source-and-installation" and any(
        token in source_contract
        for token in ("source", "installation", "portable", "git", "remote", "legacy-desktop")
    )

    if not any((remote_comfy, legacy_plugin, legacy_engine, hosted_web_shell, legacy_source_registry)):
        return {}

    if remote_comfy:
        gap = (
            "The source connects a production UI to an external ComfyUI execution endpoint, which the native-only production boundary forbids; Sim has no inactive legacy-record migration and no verified non-transmission guard."
        )
        decision = "preserve-as-inactive-legacy-migration-data-and-never-connect-or-transmit"
        acceptance = (
            "Sim shall import the source URL/profile only as visibly inactive, non-secret legacy migration data; it shall never resolve, open, probe, authenticate to, or transmit workflow data to that endpoint. The UI shall offer native-profile migration or explicit removal, preserve unknown fields until confirmed, and cover invalid URL, secret-bearing URL, cancellation, restart, and deletion recovery."
        )
    elif legacy_plugin:
        gap = (
            "The source installs or updates Python custom nodes or ComfyUI-Manager state, which production Sim forbids; the equivalent Rust/WASM plugin lifecycle and legacy mapping are not implemented."
        )
        decision = "map-to-versioned-rust-wasm-plugin-lifecycle-and-inactive-legacy-records"
        acceptance = (
            f"Sim shall preserve the visible operation states and failures of the source observation (`{row['observable_behavior']}`) while operating only on signed/versioned Rust or WASM plugins with explicit ports, grants, snapshots, rollback, cancellation, and deterministic legacy identifier mappings. Python packages and Manager configuration remain inactive migration data and are never executed or updated."
        )
    elif hosted_web_shell:
        gap = (
            "The source behavior is implemented through an Electron/browser/JavaScript shell, which is not a production Sim execution surface; the matching native GPUI interaction is not implemented."
        )
        decision = "implement-the-observable-shell-interaction-as-native-gpui-without-browser-execution"
        acceptance = (
            f"A native GPUI surface shall reproduce the source-observable interaction (`{row['observable_behavior']}`), including focus, loading, failure, cancellation, persistence, window lifecycle, accessibility, and recovery, without loading a hosted Comfy frontend or executing JavaScript/DOM/LiteGraph extensions."
        )
    else:
        gap = (
            "The source owns a Python/ComfyUI installation, process, dependency, or release lifecycle that production Sim forbids; the corresponding native profile, Rust worker, artifact, update, and recovery lifecycle is not implemented."
        )
        decision = "map-source-lifecycle-to-native-rust-worker-artifact-profile-and-recovery-services"
        acceptance = (
            f"Sim shall reproduce the user-visible lifecycle states of the source observation (`{row['observable_behavior']}`) through native runtime profiles, Sim-owned Rust workers, artifacts, journals, logs, cancellation, rollback, and crash recovery. Legacy installation records remain read-only migration inputs; production shall not probe, create, launch, update, or delete Python/ComfyUI environments."
        )

    return {
        "current_sim_status": "conflicting",
        "parity_decision": decision,
        "parity_gap": gap,
        "observable_sim_acceptance": acceptance,
        "manual_validation": (
            f"Use {row['name']} in the source Desktop only as a development oracle, then exercise its native or inactive-migration mapping in Sim; verify visible states, persistence, cancellation/recovery, and a process/network trace proving no Python, hosted frontend, or external Comfy execution."
        ),
        "open_questions": (
            "The native-only decision is fixed; source runtime timing and platform details that were not observed remain explicit validation uncertainties."
        ),
    }


def add_desktop_features(features: list[dict[str, str]]) -> None:
    name = "desktop-features.csv"
    for index, row in enumerate(read_rows(name), 2):
        target = desktop_native_target(row)
        features.append(base_feature(
            feature_id=row["feature_id"], product=row["product"], domain=row["domain"], name=row["name"],
            classification=row["classification"], availability=row["availability"], evidence_level=row["evidence_level"], confidence=row["confidence"],
            source_evidence=row["source_evidence"], source_symbol=row["source_evidence"], test_evidence=row["test_evidence"],
            actor=row["actor_trigger"], trigger=row["actor_trigger"],
            observable_success=row["observable_behavior"], failure_recovery=row["failure_cancellation_recovery"],
            persistence_serialization=row["persistence_side_effects"], interfaces_side_effects=row["persistence_side_effects"],
            current_sim_status=target.get("current_sim_status", ""),
            parity_decision=target.get("parity_decision", ""),
            parity_gap=target.get("parity_gap", row["parity_gap"]),
            observable_sim_acceptance=target.get("observable_sim_acceptance", row["acceptance"]), automated_validation=row["validation"],
            manual_validation=target.get("manual_validation", f"Exercise {row['name']} in source Desktop and Sim on every applicable platform/source/profile; compare focus, lifecycle, errors, cancellation, persistence, and recovery."),
            open_questions=target.get("open_questions", row["open_questions"]), source_catalog=name, source_row=index,
        ))


def add_desktop_renderer_surfaces(features: list[dict[str, str]]) -> None:
    name = "desktop-renderer-surfaces.csv"
    for index, row in enumerate(read_rows(name), 2):
        features.append(base_feature(
            feature_id=row["feature_id"],
            product=row["product"],
            domain=row["domain"],
            name=row["component_surface"],
            classification=row["classification"],
            availability=row["availability"],
            evidence_level=row["evidence_level"],
            confidence=row["confidence"],
            source_evidence=row["source_file"],
            source_symbol=joined(row["source_symbol"], row["props"], row["emits"], row["handlers"]),
            test_evidence=row["test_evidence"],
            documentation="No documentation-only claim is used; this row closes a concrete production Vue renderer source mapping.",
            runtime_observation="Electron was not launched because installed dependencies are absent; packaged bridge timing, native focus, and screen-reader behavior remain runtime-unverified.",
            actor=row["actor"],
            trigger=row["trigger"],
            preconditions=row["preconditions"],
            inputs_defaults=joined(row["inputs_defaults"], row["props"]),
            permissions_flags=joined(
                f"availability={row['availability']}",
                f"parent_features={row['parent_feature_ids']}",
                "Privileged filesystem, process, network, update, snapshot, and settings effects remain behind the typed Desktop service boundary.",
            ),
            observable_success=row["observable_success"],
            interaction_accessibility=row["interaction_accessibility"],
            state_concurrency=row["state_concurrency"],
            failure_recovery=row["failure_recovery"],
            persistence_serialization=row["persistence_serialization"],
            interfaces_side_effects=joined(row["interfaces_side_effects"], row["emits"], f"parent={row['parent_feature_ids']}"),
            platform_localization_variants="Renderer text follows Desktop localization; native title-bar/window behavior retains macOS, Windows, and Linux branches, while cloud, remote, legacy, update, and operation surfaces retain their cited availability.",
            current_sim_status=row["sim_status"],
            sim_evidence=row["sim_evidence"],
            parity_gap=row["parity_gap"],
            observable_sim_acceptance=row["observable_sim_acceptance"],
            automated_validation=row["automated_validation"],
            manual_validation=row["manual_validation"],
            open_questions=row["open_questions"],
            source_catalog=name,
            source_row=index,
        ))


def desktop_derived_native_target(
    catalog_name: str,
    row: dict[str, str],
    name: str,
    behavior: str,
) -> dict[str, str]:
    contract = joined(name, behavior, *(row.values())).casefold()
    remote_source = catalog_name == "desktop-source-plugins.csv" and any(
        token in contract for token in ("remote", "externally managed", "http(s)")
    )
    legacy_installation = catalog_name == "desktop-source-plugins.csv" and any(
        token in contract for token in ("standalone", "portable", "legacy", "git")
    )
    legacy_plugin = any(
        token in contract
        for token in ("custom-node", "custom node", "comfyui-manager", "enable-manager", "manager config")
    )
    legacy_python = any(
        token in contract
        for token in (
            "python",
            "pip",
            "main.py",
            "extra-index-url",
            "comfyui process",
            "comfyui launch",
            "comfy server",
            ".comfy_environment",
        )
    )
    if not any((remote_source, legacy_installation, legacy_plugin, legacy_python)):
        return {}

    if remote_source:
        decision = "preserve-as-inactive-legacy-migration-data-and-never-connect-or-transmit"
        gap = "This source/setting opens an external ComfyUI execution endpoint, which production Sim forbids; an inactive non-transmitting migration record is not implemented."
        replacement = "preserve its typed values and unknown fields as an inactive legacy record, never resolve/probe/open/authenticate/transmit to it, and offer native-profile migration or explicit removal"
    elif legacy_plugin:
        decision = "map-to-versioned-rust-wasm-plugin-lifecycle-and-inactive-legacy-records"
        gap = "This source contract installs, updates, configures, reports, or identifies Python custom nodes/Manager state; production Sim permits only versioned Rust/WASM plugins and inactive legacy data."
        replacement = "preserve the source name/schema/progress/error data while mapping executable lifecycle only to signed versioned Rust/WASM plugins with explicit ports, grants, cancellation, snapshot, and rollback; never execute or update the Python/Manager artifact"
    elif legacy_installation:
        decision = "preserve-legacy-installation-records-read-only-and-map-assets-to-native-profiles"
        gap = "This source entry owns or adopts a Python/ComfyUI installation mode that production Sim forbids; its read-only migration and native profile mapping are not implemented."
        replacement = "retain the source record read-only, preview eligible model/workflow/output/settings adoption into a native profile, require confirmation, and never probe, launch, update, or delete the legacy installation"
    else:
        decision = "map-python-comfy-lifecycle-to-native-rust-worker-artifact-and-operation-semantics"
        gap = "This control, artifact, event, flag, or bridge contract is tied to Python/ComfyUI lifecycle behavior that production Sim forbids; its native mapping or inactive legacy treatment is not implemented."
        replacement = "preserve the source identifier, typed payload, visible progress/error/cancel/restart semantics, and persisted unknown data while mapping effects only to Sim-owned Rust worker/artifact/plugin operations or an inactive legacy record; never invoke Python, pip, main.py, Manager, or ComfyUI"

    return {
        "current_sim_status": "conflicting",
        "parity_decision": decision,
        "parity_gap": gap,
        "observable_sim_acceptance": f"For `{name}`, Sim shall {replacement}. Validation shall cover source-shaped valid/invalid payloads, permission denial, cancellation, failure, restart, migration confirmation, and a process/network trace proving the prohibited effect never occurs.",
        "manual_validation": f"Observe `{name}` in source Desktop only as a development oracle, then inspect the native or inactive migration mapping in Sim and verify no Python/ComfyUI process, package update, hosted frontend, or external execution connection occurs.",
        "open_questions": "The native-only boundary is fixed; unavailable platform timing, legacy data variants, and live service responses remain explicit source-runtime uncertainties.",
    }


def add_desktop_derived(features: list[dict[str, str]]) -> None:
    specifications = [
        ("desktop-ipc.csv", "COMFY-DESKTOP-IPC", "desktop-ipc", ("channel", "mechanism", "directions_observed")),
        ("desktop-preload-apis.csv", "COMFY-DESKTOP-PRELOAD", "desktop-preload", ("surface", "member", "interface")),
        ("desktop-menu-actions.csv", "COMFY-DESKTOP-MENU", "desktop-menu", ("surface", "action", "condition")),
        ("desktop-shell-actions.csv", "COMFY-DESKTOP-SHELL", "desktop-shell", ("surface", "action", "trigger")),
        ("desktop-window-events.csv", "COMFY-DESKTOP-WINDOW", "desktop-window", ("surface", "event", "source")),
        ("desktop-settings.csv", "COMFY-DESKTOP-SETTING", "desktop-setting", ("key", "section", "default_or_fallback")),
        ("desktop-persistence.csv", "COMFY-DESKTOP-PERSIST", "desktop-persistence", ("artifact", "purpose", "format_schema")),
        ("desktop-telemetry.csv", "COMFY-DESKTOP-TELEMETRY", "desktop-telemetry", ("event_name", "event_kind")),
        ("desktop-feature-flags.csv", "COMFY-DESKTOP-FLAG", "desktop-feature-flag", ("flag", "provider")),
        ("desktop-cli-environment.csv", "COMFY-DESKTOP-CONFIG", "desktop-configuration", ("kind", "name")),
        ("desktop-keybindings-gestures.csv", "COMFY-DESKTOP-INPUT", "desktop-input", ("surface", "input", "behavior")),
        ("desktop-platform-matrix.csv", "COMFY-DESKTOP-PLATFORM", "desktop-platform", ("platform", "packages")),
        ("desktop-source-plugins.csv", "COMFY-DESKTOP-SOURCE", "desktop-installation-source", ("source_id", "name", "class")),
    ]
    for catalog_name, prefix, domain, identity_fields in specifications:
        rows = read_rows(catalog_name)
        for index, row in enumerate(rows, 2):
            effective_domain = (
                "desktop-theme-style"
                if catalog_name == "desktop-cli-environment.csv" and row.get("kind") == "CSS custom property"
                else domain
            )
            source_evidence = first(row, "source", "registration_and_use", "source_evidence", fallback="Source location recorded in parent Desktop feature.")
            if catalog_name == "desktop-telemetry.csv":
                feature_id = row["feature_id"]
                source_feature = "COMFY-DESKTOP-167"
            else:
                feature_id = stable_id(
                    prefix,
                    catalog_name,
                    *(row.get(field, "") for field in identity_fields),
                    source_evidence,
                )
                source_feature = first(row, "feature_id", fallback="No parent feature ID assigned")
            availability = first(row, "availability", fallback="active")
            evidence = first(row, "evidence_level", fallback="code-inferred")
            permissions = first(row, "security_validation", "trust_boundary", "condition", fallback=f"Availability: {availability}; privileged operations remain in the native service boundary.")
            state_concurrency = "The request/action/event is ordered by the owning Desktop service/window state machine; callbacks unsubscribe and late/stale events do not mutate a replacement window or operation."
            open_questions = "Platform-native runtime behavior remains unobserved on unavailable operating systems; cloud/paid and update services require approved non-mutating fixtures."

            if catalog_name == "desktop-ipc.csv":
                name = f"IPC {row['channel']} ({row['mechanism']})"
                behavior = f"Directions: {row['directions_observed']}; payload/result: {row['payload_result_schema']}."
                failure = f"Security/validation: {row['security_validation']}; preserve handler rejection, sender validation, cancellation, and unsubscribe behavior."
                persistence = "IPC payload/result is transient unless its named handler writes a cataloged durable artifact."
                interfaces = joined(row["registration_and_use"], row.get("notes"), f"parent={source_feature}")
                interaction = "Renderer invocation and callbacks inherit the consuming panel/window's keyboard, focus, loading, error, and accessibility behavior."
            elif catalog_name == "desktop-preload-apis.csv":
                name = f"Preload {row['surface']}.{row['member']}"
                behavior = row["contract"]
                failure = f"Trust boundary: {row['trust_boundary']}; rejected calls and callback cleanup remain visible to the consuming renderer."
                persistence = "The bridge member exposes no durability beyond its typed return/event and the underlying cataloged handler."
                interfaces = joined(row["interface"], row["consumer"], row["source"], f"parent={source_feature}")
                interaction = "The consuming renderer must retain source-equivalent focus, keyboard, pointer, loading, disabled, and error behavior; bridge calls are not themselves focus targets."
            elif catalog_name == "desktop-menu-actions.csv":
                name = f"{row['surface']} menu: {row['action']}"
                behavior = row["observable_effect"]
                failure = f"Availability condition: {row['condition']}; unavailable actions are absent or disabled as source-defined and cannot invoke privileged work."
                persistence = "Menu invocation persists only through the target action's cataloged effects."
                interfaces = joined(row["source"], f"parent={source_feature}")
                interaction = "Native/context menu item supports platform keyboard navigation, activation, focus restoration, disabled/gated state, and an accessible label/role."
            elif catalog_name == "desktop-telemetry.csv":
                name = f"Desktop telemetry/event literal {row['event_name']}"
                behavior = joined(row["event_kind"], row["payload_evidence"], row["provider_side_effects"])
                failure = joined(row["consent_behavior"], row["redaction_validation"], row["rate_limit_dedup"])
                persistence = "No workflow or settings persistence; an allowed telemetry capture produces the cataloged off-device provider side effect, while denied/dropped events produce none."
                interfaces = joined(row["source_evidence"], f"derived_wire_names={row['derived_wire_names']}", f"parent={source_feature}")
                interaction = "The event/channel has no direct focus target; any consent, error, update, install, cloud, or lifecycle UI that triggers it retains its separately cataloged keyboard, focus, label, and status behavior."
                permissions = joined(row["consent_behavior"], row["redaction_validation"], f"availability={availability}")
                state_concurrency = row["rate_limit_dedup"]
                open_questions = row["notes"]
            elif catalog_name == "desktop-settings.csv":
                name = f"Desktop setting {row['key']}"
                behavior = row["behavior"]
                failure = "Invalid type/value or write failure preserves the previous effective value and surfaces a recoverable settings error."
                persistence = joined(row["persistence"], f"default={row['default_or_fallback']}")
                interfaces = joined(row["source"], row["tests"], f"parent={source_feature}")
                interaction = "The setting control is labeled, keyboard-operable, exposes its default/current/error/restart state, and follows the localized section order."
            elif catalog_name == "desktop-persistence.csv":
                name = f"Desktop persisted artifact {row['artifact']}"
                behavior = row["write_read_migration_recovery"]
                failure = row["write_read_migration_recovery"]
                persistence = joined(row["format_schema"], row["compatibility_contract"])
                interfaces = joined(row["source"], f"parent={source_feature}")
                interaction = "Recovery, conflict, import/export, or destructive actions expose keyboard-operable choices and text status where this artifact is user-visible."
            elif catalog_name == "desktop-feature-flags.csv":
                name = f"Desktop feature flag {row['flag']}"
                behavior = row["behavior"]
                failure = f"Unavailable/invalid provider values use the documented fallback: {row['variants_default']}."
                persistence = f"Flag provider/value exposure follows {row['provider']}; unknown values remain explicit."
                interfaces = joined(row["source"], f"parent={source_feature}")
                interaction = "Gated controls update availability and explanation consistently without becoming keyboard-invokable while disabled."
            elif catalog_name == "desktop-cli-environment.csv":
                name = f"Desktop {row['kind']} {row['name']}"
                behavior = row["behavior"]
                failure = "Invalid, unsafe, missing, or platform-incompatible values surface at the owning operation and retain redacted diagnostics."
                persistence = f"Default/inheritance: {row['default']}; secret values are never serialized into user-visible logs."
                interfaces = joined(row["source"], f"parent={source_feature}")
                interaction = "User-configurable values use labeled keyboard-operable settings; inherited environment behavior is visible through redacted diagnostics."
            elif catalog_name == "desktop-keybindings-gestures.csv":
                name = f"{row['surface']} input {row['input']}"
                behavior = row["behavior"]
                failure = "Reserved text input, disabled/gated context, modal priority, cancellation, and focus restoration follow the source handler."
                persistence = "Input binding persists only where a named keymap/settings contract exists; otherwise it is a platform convention."
                interfaces = joined(row["source"], f"parent={source_feature}")
                interaction = "This row is the exact keyboard, pointer, drag/drop, or gesture contract; an equivalent keyboard path and accessible state are required for pointer-only actions."
            elif catalog_name == "desktop-platform-matrix.csv":
                name = f"Desktop platform {row['platform']}"
                behavior = row["specific_behavior"]
                failure = "Unsupported packaging, permissions, bootstrap, update, process, or data-location states surface platform-specific repair guidance without damaging other installations."
                persistence = f"Data location: {row['data_location']}; bootstrap: {row['bootstrap']}; packages: {row['packages']}."
                interfaces = joined(row["source"], f"parent={source_feature}")
                interaction = f"Input conventions: {row['input_conventions']}; native menu, title bar, chooser, dialog, and focus behavior follows {row['platform']} conventions."
            else:
                name = joined(*(row.get(field, "") for field in identity_fields), fallback=f"{domain} contract")
                behavior = first(row, "observable_effect", "behavior", "contract", "description", "capabilities", fallback="The named Desktop registry entry performs its source-defined action.")
                failure = first(row, "failure_recovery", "security", "condition", fallback="Invalid, unavailable, cancelled, and recovery behavior follows the cited source entry.")
                persistence = first(row, "persistence", "format", "side_effects", fallback="No additional durable effect beyond the cited source entry.")
                interfaces = joined(source_evidence, f"parent={source_feature}")
                interaction = "The consuming Desktop surface preserves keyboard, pointer, focus, loading, disabled, error, and accessible-name behavior."

            target = desktop_derived_native_target(catalog_name, row, name, behavior)
            features.append(base_feature(
                feature_id=feature_id, product="Comfy-Desktop", domain=effective_domain, name=name,
                classification=(row["classification"] if catalog_name == "desktop-telemetry.csv" else f"individual {effective_domain} contract"), availability=availability,
                evidence_level=evidence, confidence="high" if evidence == "test-backed" else "medium",
                source_evidence=source_evidence, source_symbol=joined(*(row.get(field, "") for field in identity_fields)),
                test_evidence=first(row, "tests", fallback="Use the parent feature and Desktop test ledger; no focused test was separately attached to this row."),
                actor="Desktop user, renderer, main process, operating system, hosted Comfy content, or lifecycle service.",
                trigger=f"Invoke {name}.", preconditions=f"The applicable Desktop source, window, platform, feature gate, and parent contract {source_feature} are available.",
                inputs_defaults=joined(*(row.get(field, "") for field in row if field not in {"feature_id", "source", "tests", "evidence_level"})),
                permissions_flags=permissions,
                observable_success=behavior, interaction_accessibility=interaction,
                state_concurrency=state_concurrency,
                failure_recovery=failure, persistence_serialization=persistence, interfaces_side_effects=interfaces,
                platform_localization_variants=first(row, "platform", "availability", fallback="Platform and locale variants follow the cited Desktop source and matched localization ledger."),
                current_sim_status=target.get("current_sim_status", ""),
                parity_decision=target.get("parity_decision", ""),
                parity_gap=target.get("parity_gap", f"Sim has generic native primitives but no Comfy-specific implementation of {name}."),
                observable_sim_acceptance=target.get("observable_sim_acceptance", f"For {name}, native Sim behavior shall match the cited invocation, typed values, visible success/error/cancel states, trust boundary, side effects, focus/accessibility, platform condition, persistence, restart, and recovery; Electron mechanics may be replaced by typed Rust services."),
                automated_validation=(f"Parameterize VAL-DESKTOP-001 with {feature_id} and source parent {source_feature}." if effective_domain in {"desktop-ipc", "desktop-preload"} else f"Run the trace-linked Desktop/GPUI contract for {feature_id} with deterministic native-service fixtures."),
                manual_validation=target.get("manual_validation", f"Compare {name} in packaged source Desktop and Sim on every applicable platform, including keyboard-only, cancellation, unavailable dependency, restart, and crash recovery."),
                open_questions=target.get("open_questions", open_questions),
                source_catalog=catalog_name, source_row=index,
            ))


def cross_product_native_target(row: dict[str, str]) -> dict[str, str]:
    feature_id = clean(row.get("feature_id"))
    if feature_id in {"COMFY-COMPAT-030", "COMFY-COMPAT-035"}:
        return {
            "current_sim_status": "conflicting",
            "parity_decision": "replace-python-javascript-extension-execution-with-versioned-rust-wasm-ports",
            "parity_gap": "The source compatibility contract executes Python modules, web-directory JavaScript, DOM, or LiteGraph hooks that production Sim forbids; the Rust/WASM mapping and lossless placeholder are not implemented.",
            "observable_sim_acceptance": "Sim shall preserve every legacy extension identifier, serialized field, widget value, unknown payload, and web-directory reference without importing or executing Python/JavaScript. A supported mapping shall require a versioned Rust/WASM manifest with explicit ports, grants, deterministic provenance, and user-visible acceptance; otherwise the workflow opens with an exact unsupported placeholder and round-trips unchanged.",
        }
    if feature_id in {"COMFY-COMPAT-045", "COMFY-COMPAT-046", "COMFY-COMPAT-048", "COMFY-COMPAT-049"}:
        external = feature_id == "COMFY-COMPAT-046"
        return {
            "current_sim_status": "conflicting",
            "parity_decision": (
                "preserve-external-endpoint-as-inactive-non-transmitting-migration-data"
                if external
                else "preserve-legacy-installation-read-only-and-map-assets-to-native-profiles"
            ),
            "parity_gap": "The source mode launches, adopts, bundles, manages, or connects to a Python/ComfyUI execution environment that production Sim forbids; safe inactive migration is not implemented.",
            "observable_sim_acceptance": (
                "Sim shall import the endpoint only as visibly inactive legacy data, preserve unknown fields and secrets safely, never resolve/probe/open/authenticate/transmit to it, and offer native-profile migration or deletion with confirmation."
                if external
                else "Sim shall retain the legacy installation/profile record read-only, preview eligible model/workflow/output/settings adoption into a native runtime profile, preserve unknown fields until confirmation, and never probe, launch, update, execute, or delete the Python/ComfyUI environment."
            ),
        }
    return {}


def add_cross_product(features: list[dict[str, str]]) -> None:
    for catalog_name in ("cross-formats.csv", "cross-compatibility.csv"):
        for index, row in enumerate(read_rows(catalog_name), 2):
            feature_id = first(row, "feature_id")
            if not feature_id:
                feature_id = stable_id("COMFY-COMPAT", catalog_name, first(row, "name", "contract", "format", fallback=str(index)))
            target = cross_product_native_target(row)
            features.append(base_feature(
                feature_id=feature_id, product=first(row, "product", fallback="Cross-product"),
                domain=first(row, "domain", fallback="cross-product-compatibility"),
                name=first(row, "name", "contract", "format", fallback=feature_id),
                classification=first(row, "classification", fallback="cross-product compatibility contract"),
                availability=first(row, "availability", fallback="active"),
                evidence_level=first(row, "evidence_level", fallback="code-inferred"), confidence=first(row, "confidence", fallback="medium"),
                source_evidence=first(row, "source_evidence", "source", "sources", fallback="Cross-product source locations are recorded in the compatibility evidence report."),
                source_symbol=first(row, "source_symbol", "symbol", fallback=first(row, "source_evidence", "source", fallback="Cross-product contract")),
                test_evidence=first(row, "test_evidence", "tests", fallback="No focused cross-product test was located."),
                documentation=first(row, "documentation", "docs", fallback="No separate documentation-only claim was used."),
                runtime_observation=first(row, "runtime_observation", "observation", fallback="Not observed end to end in this audit; see baseline constraints."),
                actor=first(row, "actor", fallback="Workflow author, extension author, API client, frontend, Desktop shell, server, importer, or exporter."),
                trigger=first(row, "trigger", fallback=f"Exchange, open, save, migrate, or execute {first(row, 'name', 'contract', 'format', fallback=feature_id)} across products."),
                preconditions=first(row, "preconditions", fallback="The producing and consuming products, versions, capabilities, dependencies, and applicable mode are available."),
                inputs_defaults=first(row, "inputs_defaults", "contract", "schema", "format_schema", "wire_contract", fallback="Use the exact cross-product representation in the cited source."),
                permissions_flags=first(row, "permissions_flags", "permissions", "security", fallback="Trust, account, path, remote, extension, and feature-gate policy follows each endpoint product."),
                observable_success=first(row, "observable_success", "success_behavior", "behavior", "compatibility_contract", fallback="The source artifact/contract transfers without semantic loss across the cataloged products and versions."),
                interaction_accessibility=first(row, "interaction_accessibility", "interaction", fallback="Import/export/connect/compatibility UI exposes keyboard, focus, accessible status, error, and destructive-confirmation behavior; wire-only portions have no direct focus target."),
                state_concurrency=first(row, "state_concurrency", "ordering_concurrency", fallback="One immutable versioned artifact or epoch-scoped protocol state is consumed at a time; stale data does not overwrite newer state."),
                failure_recovery=first(row, "failure_recovery", "error_recovery", "failure_behavior", fallback="Malformed, missing, unsupported, future-version, unavailable dependency, cancellation, conflict, and restart cases retain original data and expose a deterministic recovery choice."),
                persistence_serialization=first(row, "persistence_serialization", "serialization_behavior", "migration_backward_compatibility", "format_schema", fallback="Round-trip the cited representation and preserve unknown fields/bytes unless a confirmed migration changes them."),
                interfaces_side_effects=first(row, "interfaces_side_effects", "protocols_dependencies", "interfaces", "side_effects", fallback="Crosses the source-defined REST, WebSocket, IPC, file, media metadata, extension, or custom-node boundary."),
                platform_localization_variants=first(row, "platform_localization_variants", "platform_version", "variants", "products", fallback="Local, remote, cloud, Desktop, portable, legacy/current, platform, and feature-flag variants remain explicit."),
                current_sim_status=target.get("current_sim_status", first(row, "sim_status", fallback="missing")),
                parity_decision=target.get("parity_decision", ""),
                parity_gap=target.get("parity_gap", first(row, "parity_gap", "exact_parity_gap", fallback="Sim has no Comfy-specific cross-product compatibility adapter.")),
                observable_sim_acceptance=target.get("observable_sim_acceptance", first(row, "observable_sim_acceptance", "acceptance_criteria", "acceptance", fallback="The same deterministic artifact or wire fixture shall produce semantically equivalent results in every applicable source product and Sim, with unknown data and failure state retained.")),
                automated_validation=first(row, "automated_validation", "validation", fallback=f"Run the trace-linked deterministic cross-product contract for {feature_id}."),
                manual_validation=first(row, "manual_validation", fallback=f"Exchange {feature_id} among source products and Sim; inspect structure, visible state, side effects, errors, recovery, and restart behavior."),
                open_questions=first(row, "open_questions", "assumptions", fallback="Dynamic extensions, unavailable legacy runtimes, cloud services, and platform-native paths remain explicit runtime uncertainties."),
                source_catalog=catalog_name, source_row=index,
        ))


def normalize_cross_product_native_decision() -> None:
    rows = read_rows("cross-compatibility.csv")
    changed = False
    for row in rows:
        feature_id = row["feature_id"]
        domain = row["domain"].casefold()
        name = row["name"].casefold()
        if feature_id == "COMFY-COMPAT-058":
            row.update({
                "name": "Native Rust production compatibility boundary",
                "classification": "required native parity decision",
                "contract": "Production Sim implements execution, nodes, tensors, autograd, RNG, model loading, samplers, schedulers, devices, memory, caching, media, cancellation, and recovery in Sim-owned Rust crates/workers. ComfyUI is development-oracle only. Python/JavaScript extensions are replaced by versioned Rust/WASM explicit-port APIs and deterministic legacy mappings.",
                "success_behavior": "The native image and diffusion slices run with no source trees, no Python/Node/browser, no external Comfy connection, and no network while native HTTP/WebSocket/CLI projections use the same Rust runtime.",
                "failure_recovery": "Legacy endpoint, Python installation, and extension data is preserved as inactive migration evidence or exact placeholders; no fallback process or request starts.",
                "source_evidence": "User-approved native-only architecture constraint; requirements.md Requirements 33-44; design.md D1/D12/D25-D40",
                "test_evidence": "VAL-NATIVE-BOUNDARY-001; VAL-NATIVE-E2E-001; VAL-NATIVE-E2E-002; VAL-PLUGIN-001",
                "sim_status": "missing",
                "parity_gap": "The required native runtime, worker, tensor/model/sampler/node/plugin/API/media implementation and release boundary gates do not exist in Sim.",
                "open_questions": "Backend ecosystem/vendor FFI/codec/package choices require implementation ADRs, but cannot relax the native-only boundary.",
            })
            changed = True
            continue
        if any(token in domain for token in ("python extensions", "web extensions")):
            row["sim_status"] = "conflicting"
            row["parity_gap"] = "The source executes Python or JavaScript with ambient APIs, which production Sim prohibits; versioned Rust/WASM explicit ports, legacy mappings, grants, limits, and lossless placeholders are not implemented."
            changed = True
        elif domain == "modes" and any(token in name for token in ("managed local", "external remote", "portable", "legacy desktop")):
            row["sim_status"] = "conflicting"
            row["parity_gap"] = "The source mode depends on a Python/Comfy server lifecycle or endpoint that production Sim forbids; its observable data/lifecycle needs a native profile, artifact, worker, provider, or migration mapping."
            changed = True
        elif row["sim_status"] == "missing":
            row["parity_gap"] = f"Sim has no native Rust/GPUI implementation of the cross-product contract `{row['name']}`."
            changed = True
    if changed:
        rewrite_catalog("cross-compatibility.csv", rows, list(rows[0]))


def sync_cross_product_trace(features: list[dict[str, str]]) -> None:
    by_id = {feature["feature_id"]: feature for feature in features}
    for catalog_name in ("cross-formats.csv", "cross-compatibility.csv"):
        rows = read_rows(catalog_name)
        changed = False
        for row in rows:
            feature = by_id.get(row.get("feature_id", ""))
            if feature is None:
                continue
            mappings = {
                "sim_status": feature["current_sim_status"],
                "parity_gap": feature["parity_gap"],
                "requirements": feature["requirement_criteria"],
                "design": feature["design_coverage"],
                "task": feature["task_id"],
                "validation": feature["validation_id"],
            }
            for field, value in mappings.items():
                if field in row and row[field] != value:
                    row[field] = value
                    changed = True
        if changed:
            rewrite_catalog(catalog_name, rows, list(rows[0]))


def add_comfy_cli(features: list[dict[str, str]]) -> None:
    specifications = [
        ("comfy-cli-commands.csv", "command", ("path",), ("help", "notes"), "CLI command"),
        ("comfy-cli-parameters.csv", "parameter", ("command_path", "name"), ("help", "flags", "annotation", "value_type", "nullable", "value_arity", "cardinality", "repeatable", "choices", "constraints", "boolean_forms", "default", "default_source", "type_evidence"), "CLI option or argument"),
        ("comfy-cli-schemas.csv", "schema", ("name",), ("title", "type", "required", "top_level_properties"), "CLI JSON schema"),
        ("comfy-cli-errors.csv", "error", ("code",), ("meaning", "hint"), "stable CLI error"),
        ("comfy-cli-events.csv", "event", ("event",), ("contract_status", "notes"), "CLI event contract"),
        ("comfy-cli-config.csv", "configuration", ("key",), ("behavior",), "CLI configuration key"),
        ("comfy-cli-environment.csv", "environment", ("key",), ("behavior",), "CLI environment variable"),
        ("comfy-cli-formats.csv", "format", ("name",), ("behavior",), "CLI persisted or interchange format"),
        ("comfy-cli-lifecycle.csv", "lifecycle", ("name",), ("behavior",), "CLI lifecycle state"),
        ("comfy-cli-extensions.csv", "extension", ("name",), ("source_contract", "native_decision"), "CLI extension contract"),
        ("comfy-cli-modules.csv", "module", ("module",), ("public_functions", "classes", "command_ids"), "CLI module or service contract"),
        ("comfy-cli-cql-policy.csv", "CQL policy", ("row_kind", "pack", "node_identifier"), ("labels", "version", "cloud_disabled"), "CQL registry policy"),
        ("comfy-cli-partner-openapi.csv", "partner API", ("alias", "method", "path"), ("endpoint_id", "category", "mode", "poller"), "partner endpoint mapping"),
        ("comfy-cli-documentation.csv", "documented claim", ("name",), ("claim", "corroboration"), "CLI documentation claim"),
    ]
    for filename, domain, name_fields, behavior_fields, classification in specifications:
        for index, row in enumerate(read_rows(filename), 2):
            feature_id = row["feature_id"]
            name = " ".join(first(row, field) for field in name_fields if first(row, field))
            behavior = joined(*(row.get(field, "") for field in behavior_fields), fallback=f"The source exposes {name}.")
            source_file = first(row, "source_file", "openapi_source", fallback="source location retained by the CLI reconciliation ledger")
            line = first(row, "line")
            source = f"projects/comfy/comfy-cli/{source_file}" + (f":{line}" if line else "")
            tests = first(row, "tests", fallback="No focused existing test was linked for this exact CLI row.")
            availability = first(row, "availability", fallback="active")
            if domain == "documented claim" and row.get("evidence_level") == "documented-only":
                availability = availability or "uncertain"
            features.append(base_feature(
                feature_id=feature_id,
                product="Comfy CLI",
                domain=domain,
                name=name or feature_id,
                classification=first(row, "classification", "row_kind", fallback=classification),
                availability=availability,
                evidence_level=first(row, "evidence_level", fallback="code-inferred"),
                confidence="high" if first(row, "evidence_level") == "test-backed" else "medium",
                source_evidence=source,
                source_symbol=first(row, "symbol", "endpoint_id", "schema_id", "code", "key", "event", fallback=name),
                test_evidence=tests,
                documentation=first(row, "help", "claim", fallback="No separate prose claim was used beyond this row."),
                actor="CLI user, automation client, plugin/provider author, registry operator, or native runtime operator.",
                trigger=f"Invoke or consume the comfy-cli {domain} `{name}`.",
                preconditions=f"The CLI contract is reachable and its source availability is {availability}; production Sim uses only the native `sim comfy` mapping.",
                inputs_defaults=joined(row.get("flags"), row.get("kind"), row.get("default"), row.get("required"), row.get("hidden"), row.get("envvar"), row.get("schema_id"), row.get("required"), row.get("top_level_properties"), fallback="Use the exact source declaration in the cited row."),
                permissions_flags=joined(row.get("hidden"), row.get("cloud_disabled"), row.get("mode"), row.get("availability"), fallback="Native filesystem, network, provider, plugin, and secret permissions apply before side effects."),
                observable_success=behavior,
                interaction_accessibility="Terminal help, stdout/stderr, exit status, progress/event streams, prompts, cancellation, and noninteractive behavior are CLI contracts; any GPUI projection is cataloged separately.",
                state_concurrency=joined(row.get("mode"), row.get("poller"), row.get("contract_status"), row.get("registration"), fallback="Ordering and lifecycle follow the cited command/event/schema contract."),
                failure_recovery=joined(row.get("meaning"), row.get("hint"), row.get("notes"), fallback="Invalid input, missing dependencies, offline, permission denial, cancellation, timeout, interrupted operation, and restart remain native validation cases."),
                persistence_serialization=joined(row.get("schema_id"), row.get("schema"), row.get("behavior"), row.get("version"), fallback="No separate persisted form beyond the cited CLI contract."),
                interfaces_side_effects=joined(row.get("command_path"), row.get("flags"), row.get("method"), row.get("path"), row.get("event"), row.get("key"), fallback="CLI stdout/stderr/exit and the cited native operation."),
                platform_localization_variants="The source requires Python 3.10+, but production parity maps the observable contract to native Rust on supported Sim platforms; cloud/provider and platform gates remain explicit.",
                current_sim_status=first(row, "target_status", fallback="missing"),
                parity_gap="Sim has no native `sim comfy` implementation of this exact command, flag, schema, event, error, configuration, format, registry, extension, or lifecycle contract.",
                parity_decision=first(row, "parity_decision", "native_decision", fallback=parity_decision("Comfy CLI", domain, feature_id, availability)),
                observable_sim_acceptance=f"The native `sim comfy` mapping for {feature_id} shall reproduce the cataloged input, output, event/error, side effect, invalid, offline, cancellation, retry, restart, and version behavior, or emit the recorded architecture-conflicting migration/defer response without launching Python or ComfyUI.",
                automated_validation=f"Parameterize VAL-CLI-001 and VAL-NATIVE-API-001 with {feature_id}.",
                manual_validation=f"Compare source help/behavior and native `sim comfy` for {feature_id}; inspect output, exit/error, progress, cancellation, filesystem/provider effects, and recovery.",
                open_questions=first(row, "notes", fallback="Runtime validation is absent because the host Python is below the declared minimum and required CLI dependencies are unavailable."),
                source_catalog=filename,
                source_row=index,
            ))


def add_documentation_features(features: list[dict[str, str]]) -> None:
    for index, row in enumerate(read_rows("docs-pages.csv"), 2):
        features.append(base_feature(
            feature_id=row["record_id"], product="Comfy documentation", domain=first(row, "domain", fallback="documentation"),
            name=first(row, "title", fallback=row["path"]), classification=row["role"], availability=row["availability"],
            evidence_level=row["document_evidence_level"], confidence="medium",
            source_evidence=f"projects/comfy/docs/{row['path']}", source_symbol=row["path"],
            documentation=joined(row.get("title"), row.get("description"), row.get("corroboration_status"), row.get("corroborated_feature_ids")),
            actor="Documentation reader, workflow author, operator, extension author, or developer.", trigger=f"Open documentation page {row['path']}.",
            preconditions="The documentation site or source page is available; executable claims require separate corroboration.",
            observable_success=joined(f"The page exposes title `{first(row, 'title', fallback=row['path'])}` and role `{row['role']}`.", row.get("description"), row.get("corroboration_status")),
            interaction_accessibility="Navigation, headings, links, code samples, media alternatives, locale, keyboard focus, and redirects are documentation-surface contracts; content prose alone is not runtime evidence.",
            state_concurrency="Static or generated documentation content follows its navigation/build snapshot; no executable product state transition is inferred.",
            failure_recovery="Missing page, broken link, path-case mismatch, locale gap, stale generated content, and uncorroborated claim remain explicit documentation validation failures.",
            persistence_serialization="The path, title, role, navigation membership, redirects, localization, and source fingerprint are versioned documentation records.",
            interfaces_side_effects="Documentation rendering and navigation only; no executable product side effect is inferred.",
            platform_localization_variants="English is source-of-truth; localized/generated mirrors and navigation differences are reconciled separately.",
            parity_gap="Sim has no evidence-linked native help/documentation surface for this page or its independently corroborated capability.",
            parity_decision=row["native_parity_treatment"],
            observable_sim_acceptance=f"Sim shall link or present the applicable native help for {row['record_id']}, preserve its documented-only classification until corroborated, and reproduce accessible navigation/error behavior without treating prose as executable evidence.",
            automated_validation=f"Run VAL-DOCS-001 for {row['record_id']} and every listed corroborated feature ID.",
            manual_validation=f"Open {row['path']} and the mapped Sim help surface; verify title, content role, links, locale, accessibility, and corroboration label.",
            open_questions="Documentation claims remain non-executable unless the corroborated feature IDs name code/test evidence.",
            source_catalog="docs-pages.csv", source_row=index,
        ))

    for index, row in enumerate(read_rows("embedded-docs-nodes.csv"), 2):
        availability = "uncertain" if row["corroboration_status"] == "provider-unverified" else "active"
        features.append(base_feature(
            feature_id=row["record_id"], product="Comfy embedded documentation", domain="node documentation",
            name=f"Embedded node documentation: {row['node_document_name']}", classification="localized embedded node documentation",
            availability=availability, evidence_level=row["evidence_level"], confidence="medium",
            source_evidence=f"projects/comfy/embedded-docs/docs/{row['node_document_name']}; declared fingerprint {row['declared_source_fingerprint']}",
            source_symbol=row["node_document_name"], documentation=joined(row.get("docs_site_path"), row.get("corroboration_status"), row.get("corroborated_feature_ids")),
            actor="Node documentation reader or workflow author.", trigger=f"Open embedded docs for {row['node_document_name']}.",
            preconditions="The embedded-docs resource version and locale are present; node execution requires separate registry evidence.",
            inputs_defaults=f"locales={row['locales']}; locale_count={row['locale_count']}; assets={row['asset_count']}; visual_media={row['visual_media_count']}.",
            permissions_flags="Read-only bundled documentation; no Python or JavaScript node execution is authorized.",
            observable_success=joined(f"All {row['locale_count']} locale documents are addressable.", f"sync={row['docs_sync_status']}", f"registry_match={row['registry_match_kind']}"),
            interaction_accessibility="Locale selection, headings, links, media alternatives, keyboard focus, and readable fallback apply; AI-generated marker is retained as provenance.",
            state_concurrency="Version and fingerprint selection is immutable for one package snapshot.",
            failure_recovery="Missing locale, asset, fingerprint mismatch, provider-unverified node, or docs-site path mismatch remains visible and does not activate a node.",
            persistence_serialization=f"Package version/fingerprint, node document name, locales, docs-site path, assets, and sync state remain deterministic records.",
            interfaces_side_effects="Read-only embedded resource lookup; no executable node, route, process, or network effect.",
            platform_localization_variants=f"Locales: {row['locales']}; English files carry AI-generated provenance={row['all_english_docs_ai_generated_marker']}.",
            parity_gap="Sim has no versioned native embedded node-help resource or reconciliation for this record.",
            parity_decision=row["native_parity_treatment"],
            observable_sim_acceptance=f"Sim shall resolve {row['record_id']} to native node help or an exact documented-only/unverified placeholder, retain locale and fingerprint provenance, and never infer execution support from the document.",
            automated_validation=f"Run VAL-DOCS-001 for {row['record_id']} and its registry/docs-site reconciliation.",
            manual_validation=f"Open {row['node_document_name']} in every available locale and compare links, assets, fallback, provenance, and mapped node availability.",
            open_questions="Provider-unverified node claims and package/docs-site version skew remain explicit until executable evidence is available.",
            source_catalog="embedded-docs-nodes.csv", source_row=index,
        ))

    documentation_catalogs = [
        ("docs-openapi-cloud.csv", "cloud OpenAPI", "cloud API operation", "cloud/paid; experimental", ("method", "path", "summary"), ("operation_id", "route_shape_reconciliation", "corroborated_feature_ids")),
        ("docs-redirects.csv", "redirect", "documentation redirect", "active", ("source", "destination"), ("source", "destination")),
        ("docs-tooling.csv", "tooling", "documentation tooling contract", "developer-only", ("kind", "name"), ("contract", "source_evidence")),
        ("docs-config-formats.csv", "configuration", "documentation configuration/format", "infrastructure-only", ("kind", "name"), ("contract", "source_evidence", "corroborated_feature_ids")),
        ("docs-extension-contracts.csv", "extension documentation", "legacy extension documentation contract", "deprecated/dead", ("family", "name"), ("legacy_behavior", "native_rust_wasm_port", "production_legacy_execution")),
        ("docs-lifecycle-contracts.csv", "lifecycle documentation", "documented lifecycle contract", "conditional", ("name",), ("documented_behavior", "native_parity_treatment", "corroborated_feature_ids")),
    ]
    for filename, domain, classification, default_availability, name_fields, behavior_fields in documentation_catalogs:
        for index, row in enumerate(read_rows(filename), 2):
            feature_id = row["record_id"]
            name = " ".join(first(row, field) for field in name_fields if first(row, field))
            behavior = joined(*(row.get(field, "") for field in behavior_fields), fallback=name)
            evidence = first(row, "document_evidence_level", "evidence_level", fallback="documented-only")
            availability = first(row, "availability", fallback=default_availability)
            if availability not in {"active", "conditional", "platform-specific", "experimental", "developer-only", "cloud/paid", "deprecated/dead", "infrastructure-only", "uncertain", "cloud/paid; experimental"}:
                availability = default_availability
            source_evidence = first(row, "source_evidence", fallback=f"projects/comfy/docs catalog row {filename}:{index}")
            features.append(base_feature(
                feature_id=feature_id, product="Comfy documentation", domain=domain, name=name or feature_id,
                classification=classification, availability=availability, evidence_level=evidence,
                confidence="medium" if evidence != "documented-only" else "low",
                source_evidence=source_evidence, source_symbol=first(row, "operation_id", "name", "source", fallback=feature_id),
                documentation=behavior,
                actor="Documentation reader, developer, extension author, operator, or cloud/API consumer.",
                trigger=f"Consume documentation contract {name or feature_id}.",
                preconditions="The documentation/configuration/tooling snapshot is available; executable behavior requires linked code or test corroboration.",
                inputs_defaults=joined(row.get("method"), row.get("path"), row.get("openapi_version"), row.get("api_info_version"), row.get("kind"), fallback="Use the exact documented values in the row."),
                permissions_flags=joined(row.get("availability"), row.get("production_legacy_execution"), fallback="Documentation grants no production execution authority."),
                observable_success=behavior,
                interaction_accessibility="Documentation navigation/link/locale/keyboard behavior applies where user-visible; tooling/config rows are developer or infrastructure contracts.",
                state_concurrency="No executable lifecycle is inferred beyond linked corroborated feature IDs; build/tool ordering follows the cited code-inferred row where applicable.",
                failure_recovery="Broken links, missing paths, case mismatch, stale schemas, unsupported versions, tooling failure, and uncorroborated claims remain explicit.",
                persistence_serialization=joined(row.get("openapi_version"), row.get("api_info_version"), row.get("source"), row.get("destination"), row.get("contract"), fallback="The catalog record and source snapshot are the durable evidence."),
                interfaces_side_effects=joined(row.get("method"), row.get("path"), row.get("contract"), fallback="No production side effect is inferred from documentation alone."),
                platform_localization_variants="English is source-of-truth; cloud, developer, version, locale, and generated-content variants remain as cataloged.",
                parity_gap=f"Sim has no explicit native mapping/help/defer treatment for documentation record {feature_id}.",
                parity_decision=first(row, "native_parity_treatment", "native_rust_wasm_port", fallback="retain-as-documented-only-until-executable-corroboration"),
                observable_sim_acceptance=f"Sim shall preserve and label {feature_id}, link corroborated runtime contracts where present, implement only code/test-supported native behavior, and retain uncorroborated prose as documented-only or explicit defer.",
                automated_validation=f"Run VAL-DOCS-001 for {feature_id} and every corroborated feature ID.",
                manual_validation=f"Inspect the source documentation/tooling/configuration and its Sim mapping; verify version, link/path, availability, accessibility, and evidence label.",
                open_questions="Any uncorroborated server-side, cloud/paid, generated, or legacy behavior remains uncertain rather than executable evidence.",
                source_catalog=filename, source_row=index,
            ))


def apply_task18_target_evidence(features: list[dict[str, str]]) -> None:
    features_by_id = {feature["feature_id"]: feature for feature in features}

    for feature_id, (kind, owner) in TASK_18_QUEUE_DISPOSITIONS.items():
        feature = features_by_id.get(feature_id)
        if feature is None:
            raise RuntimeError(f"Task 18 master feature is missing {feature_id}")
        if kind == "native":
            feature["current_sim_status"] = "partial"
            feature["sim_evidence"] = (
                "crates/comfy_ui/src/execution_catalog.rs assigns this row to the native "
                "profile-scoped Execution dock model; crates/comfy_runtime/src/"
                "execution_presentation.rs and crates/comfy_ui/src/execution_panel.rs provide "
                "the typed reducer, acknowledged actions, and GPUI projection."
            )
            feature["parity_gap"] = (
                "The Task 18 native contract is implemented; later native image/diffusion, "
                "specialized viewer, platform, and final release closure remain only where "
                "separately mapped."
            )
            feature["parity_decision"] = (
                f"native:ExecutionDockPanel;owner:{EXECUTION_UI_OWNER}"
            )
        elif kind == "shared_closure":
            feature["current_sim_status"] = "partial"
            feature["sim_evidence"] = (
                "crates/comfy_ui/src/execution_catalog.rs records an explicit SharedClosure "
                "disposition. Task 18 implements the native execution presentation portion "
                f"while `{owner}` retains the exact remaining contract."
            )
            feature["parity_gap"] = (
                f"The native Execution dock portion is implemented; `{owner}` must complete "
                "the remaining source-specific behavior before this row can be equivalent."
            )
            feature["parity_decision"] = (
                f"shared:ExecutionDockPanel;current-owner:{EXECUTION_UI_OWNER};"
                f"closure-owner:{owner}"
            )
        elif kind == "foundation":
            feature["current_sim_status"] = "partial"
            feature["sim_evidence"] = (
                "crates/comfy_ui/src/execution_catalog.rs records an explicit Foundation "
                f"disposition owned by `{owner}` and consumed by the Task 18 presentation model."
            )
            feature["parity_gap"] = (
                "The native foundation and its execution presentation are present; broader "
                "surface or release closure remains with the trace-linked later tasks."
            )
            feature["parity_decision"] = (
                f"foundation:NativeGraphOrFormat;owner:{owner};"
                f"consumer:{EXECUTION_UI_OWNER}"
            )
        elif kind == "later_owned":
            feature["current_sim_status"] = "deferred"
            feature["sim_evidence"] = (
                "crates/comfy_ui/src/execution_catalog.rs retains a typed LaterOwned "
                f"disposition for `{owner}`; Task 18 does not claim this behavior."
            )
            feature["parity_gap"] = (
                f"No current Task 18 implementation is claimed for this exact contract; `{owner}` "
                "remains the executable closure owner."
            )
            feature["parity_decision"] = f"later-owned:{owner}"
        else:
            raise RuntimeError(f"unknown Task 18 disposition {kind} for {feature_id}")

    for command_id, (feature_id, owner, native) in TASK_18_EXECUTION_COMMANDS.items():
        feature = features_by_id.get(feature_id)
        if feature is None:
            raise RuntimeError(f"Task 18 command feature is missing {feature_id}")
        if feature_id in TASK_18_QUEUE_DISPOSITIONS:
            continue
        feature["current_sim_status"] = "partial" if native else "deferred"
        feature["sim_evidence"] = (
            f"crates/comfy_ui/src/actions.rs registers `{command_id}` with an exact "
            f"Execution-dock placement and `{owner}` ownership; native action={'yes' if native else 'no'}."
        )
        feature["parity_gap"] = (
            "The command has a real acknowledged Task 18 action and visible failure path; final "
            "release closure remains."
            if native
            else f"The command remains visibly later-owned by `{owner}` and has no fake native action."
        )
        feature["parity_decision"] = (
            f"native:ExecutionDockPanel;owner:{owner}"
            if native
            else f"later-owned:{owner}"
        )
    for feature_id, owner in TASK_18_JOB_RUN_MENU_OWNERS.items():
        feature = features_by_id.get(feature_id)
        if feature is None:
            raise RuntimeError(f"Task 18 menu feature is missing {feature_id}")
        native = owner == EXECUTION_UI_OWNER
        feature["current_sim_status"] = "partial" if native else "deferred"
        feature["sim_evidence"] = (
            "crates/comfy_ui/src/shell.rs assigns this job/run menu row an exact native "
            f"placement and `{owner}` action owner."
        )
        feature["parity_gap"] = (
            "The Task 18 menu action is implemented through the Execution dock; final release "
            "closure remains."
            if native
            else f"The corresponding action remains unimplemented until `{owner}` completes it."
        )
        feature["parity_decision"] = (
            f"native:ExecutionDockPanel;owner:{owner}"
            if native
            else f"later-owned:{owner}"
        )

    component_rows = {
        row["feature_id"]: row for row in read_rows("frontend-component-surfaces.csv")
    }
    for feature_id, owner in TASK_18_COMPONENT_OWNERS.items():
        feature = features_by_id.get(feature_id)
        if feature is None:
            raise RuntimeError(f"Task 18 component feature is missing {feature_id}")
        expected_disposition = component_surface_disposition(
            feature_id, component_rows[feature_id]["domain"]
        )
        if not expected_disposition.endswith(f"owner:{owner}"):
            raise RuntimeError(f"Task 18 component owner drift for {feature_id}")
        native = owner == EXECUTION_UI_OWNER
        feature["current_sim_status"] = "partial" if native else "deferred"
        feature["sim_evidence"] = (
            "crates/comfy_ui/src/execution_panel.rs, queue_panel.rs, history_panel.rs, "
            "output_view.rs, and graph_render.rs consolidate the source component contract into "
            "one native GPUI model and projection."
            if native
            else f"The exact component remains assigned to `{owner}` and is not claimed by Task 18."
        )
        feature["parity_gap"] = (
            "The source-specific contract is implemented through the consolidated native "
            "Execution dock or graph projection; final release closure remains."
            if native
            else f"No native implementation is claimed until `{owner}` completes this surface."
        )
        feature["parity_decision"] = expected_disposition


def apply_task112_target_evidence(features: list[dict[str, str]]) -> None:
    feature = next(
        (
            candidate
            for candidate in features
            if candidate["feature_id"] == "COMFY-MODEL-0015"
        ),
        None,
    )
    if feature is None:
        raise RuntimeError("Task 112 master feature is missing COMFY-MODEL-0015")

    feature["current_sim_status"] = "partial"
    feature["sim_evidence"] = (
        "crates/comfy_tensor/src/backends/apple_metal_mps_comfy_model_0015.rs "
        "implements the exact twelve-row native Metal semantic TensorBackend over the "
        "certified opaque comfy_backend_metal::MetalRuntime; focused VAL-DEVICE-001, "
        "VAL-TENSOR-001, VAL-MEMORY-001, and VAL-OWNERSHIP-001 tests cover the adapter."
    )
    feature["parity_gap"] = (
        "The Task 112 semantic adapter is implemented; signed package trust provisioning, "
        "production worker selection and model-execution integration, and actual hardware "
        "certification remain assigned to Tasks 113-115."
    )
    feature["parity_decision"] = (
        "partial-native:MetalTensorBackend;semantic-owner:comfy_tensor;"
        "execution-owner:comfy_backend_metal;remaining:Tasks113-115"
    )


def apply_native_menu_target_evidence(features: list[dict[str, str]]) -> None:
    features_by_id = {feature["feature_id"]: feature for feature in features}
    rows = read_rows("native-menu-dispositions.csv")
    if len(rows) != 236 or len({row["feature_id"] for row in rows}) != 236:
        raise RuntimeError("native menu disposition ledger must contain 236 unique rows")
    expected_graph_context_infrastructure = {
        "COMFY-MENU-155": "RegisterMenuGroup",
        "COMFY-MENU-156": "CommandAdapter",
        "COMFY-MENU-157": "CoreMenuLoader",
        "COMFY-MENU-159": "TranslatedRegistryItems",
        "COMFY-MENU-160": "DropdownRenderer",
        "COMFY-MENU-164": "ContextMenuConverter",
        "COMFY-MENU-165": "MergedMoreOptions",
        "COMFY-MENU-166": "NativeContextRenderer",
    }
    graph_context_infrastructure = {
        row["feature_id"]: clean(row.get("context_infrastructure"))
        for row in rows
        if clean(row.get("context_infrastructure"))
    }
    if graph_context_infrastructure != expected_graph_context_infrastructure:
        raise RuntimeError(
            "graph context infrastructure disposition drift: "
            f"expected={expected_graph_context_infrastructure}, "
            f"actual={graph_context_infrastructure}"
        )
    for row in rows:
        if not clean(row.get("context_infrastructure")):
            continue
        if (
            row["owner_task_id"] != "comfy-parity-graph-context-menu-surfaces"
            or row["disposition"] != "infrastructure"
            or row["context_surface"] != "Infrastructure"
            or any(
                clean(row.get(field))
                for field in ("command_id", "native_action", "context_action")
            )
        ):
            raise RuntimeError(
                f"{row['feature_id']} must remain a prerequisite-only graph context "
                "infrastructure row"
            )
    for row in rows:
        feature_id = row["feature_id"]
        feature = features_by_id.get(feature_id)
        if feature is None:
            raise RuntimeError(f"native menu disposition has no master feature: {feature_id}")
        disposition = row["disposition"]
        context_infrastructure = clean(row.get("context_infrastructure"))
        placed = disposition in {"canonical-command", "native"} or bool(
            context_infrastructure
        )
        decision = (
            "place-prerequisite"
            if context_infrastructure
            else "place" if placed else "defer"
        )
        if context_infrastructure:
            adapter = f";adapter:consumed-prerequisite:{context_infrastructure}"
        elif disposition == "canonical-command":
            adapter = ";adapter:canonical-command"
        else:
            adapter = ";adapter:generated-menu-registry"
        feature["current_sim_status"] = "partial" if placed else "deferred"
        if context_infrastructure:
            feature["sim_evidence"] = (
                "catalogs/native-menu-dispositions.csv assigns this row as the exact "
                f"`{context_infrastructure}` prerequisite consumed by the native graph "
                "context-menu path; crates/comfy_ui/src/generated_menu_catalog.rs provides "
                "the typed binding and crates/comfy_ui/src/context_menu.rs requires it at "
                "the owning registry, adapter, conversion, or renderer boundary. This row "
                "does not claim a standalone user-visible surface."
            )
            feature["parity_gap"] = (
                "The prerequisite is implemented and consumed by the placed native graph "
                "context-menu surfaces; the named owner retains final release closure."
            )
        else:
            feature["sim_evidence"] = (
                "catalogs/native-menu-dispositions.csv assigns this row one exact placement and "
                "executable owner; crates/comfy_ui/src/generated_menu_catalog.rs is the generated "
                "typed projection and crates/comfy_ui/src/shell.rs verifies command-backed rows "
                "against the canonical command registry without heuristic fallback."
            )
            feature["parity_gap"] = (
                "The exact native placement and owner adapter are implemented; the named owner "
                "retains any capability-specific and release-closure work."
                if placed
                else "The row is preserved with an exact later executable owner and no native "
                "surface is claimed before that task completes."
            )
        feature["parity_decision"] = (
            f"{decision}:{row['placement']};owner:{row['owner_task_id']}{adapter}"
        )


def build_features() -> list[dict[str, str]]:
    validate_task18_disposition_ledger()
    normalize_source_catalog_corrections()
    normalize_cross_product_native_decision()
    features: list[dict[str, str]] = []
    add_backend_features(features)
    add_backend_source_anchors(features)
    add_backend_nodes(features, "backend-nodes.csv")
    add_backend_nodes(features, "backend-inactive-nodes.csv", inactive=True)
    add_backend_routes(features)
    add_backend_websocket(features)
    add_backend_config(features)
    add_backend_models(features)
    conditioning_trace_mappings = add_backend_conditioning_contracts(features)
    add_backend_tensor_runtime(features)
    add_backend_formats(features)
    add_backend_external_services(features)
    add_backend_schemas(features)
    add_frontend(features)
    add_frontend_functional_modules(features)
    add_frontend_supplemental(features)
    add_frontend_component_surfaces(features)
    add_desktop_features(features)
    add_desktop_renderer_surfaces(features)
    add_desktop_derived(features)
    add_cross_product(features)
    add_comfy_cli(features)
    add_documentation_features(features)

    titles = design_titles()
    for feature in features:
        decorate_trace(feature, titles)
        conditioning_trace = conditioning_trace_mappings.get(feature["feature_id"])
        if conditioning_trace is not None:
            implementation_task, validation_surface, closure_artifact = conditioning_trace
            tasks = [
                value.strip()
                for value in feature["task_id"].split(";")
                if value.strip()
            ]
            if implementation_task not in tasks:
                tasks.append(implementation_task)
            feature["task_id"] = "; ".join(tasks)
            validations = [
                value.strip()
                for value in feature["validation_id"].split(";")
                if value.strip()
            ]
            if validation_surface not in validations:
                validations.append(validation_surface)
            if closure_artifact and closure_artifact not in validations:
                validations.append(closure_artifact)
            feature["validation_id"] = "; ".join(validations)
    apply_task18_target_evidence(features)
    apply_task112_target_evidence(features)
    apply_native_menu_target_evidence(features)

    features.sort(key=lambda feature: feature["feature_id"])
    duplicates = [feature_id for feature_id, count in Counter(feature["feature_id"] for feature in features).items() if count > 1]
    if duplicates:
        raise RuntimeError(f"Duplicate feature IDs: {', '.join(duplicates[:20])}")
    for feature in features:
        missing = [field for field in FEATURE_FIELDS if not clean(feature.get(field, ""))]
        if missing:
            raise RuntimeError(f"{feature['feature_id']} has blank fields: {missing}")
    return features


def synchronize_source_catalog_targets(features: list[dict[str, str]]) -> None:
    feature_by_id = {feature["feature_id"]: feature for feature in features}
    acceptance_catalogs = {
        "backend-features.csv",
        "backend-formats.csv",
        "backend-websocket-events.csv",
        "desktop-features.csv",
        "frontend-features.csv",
    }
    question_catalogs = {
        "backend-features.csv",
        "backend-formats.csv",
        "desktop-features.csv",
        "frontend-features.csv",
    }
    for path in sorted(CATALOGS.glob("*.csv")):
        if path.name == "features.csv" or path.name.startswith("._"):
            continue
        with path.open(newline="", encoding="utf-8") as handle:
            reader = csv.DictReader(handle)
            fieldnames = list(reader.fieldnames or [])
            rows = list(reader)
        if "feature_id" not in fieldnames:
            continue

        changed = False
        for row in rows:
            feature = feature_by_id.get(clean(row.get("feature_id")))
            if feature is None:
                continue
            replacements = {
                "sim_status": feature["current_sim_status"],
                "current_sim_status": feature["current_sim_status"],
                "target_status": feature["current_sim_status"],
                "sim_evidence": feature["sim_evidence"],
                "parity_gap": feature["parity_gap"],
                "exact_parity_gap": feature["parity_gap"],
                "target_gap": feature["parity_gap"],
                "parity_decision": feature["parity_decision"],
            }
            if path.name in acceptance_catalogs:
                replacements.update({
                    "acceptance": feature["observable_sim_acceptance"],
                    "acceptance_criteria": feature["observable_sim_acceptance"],
                    "sim_acceptance": feature["observable_sim_acceptance"],
                    "observable_sim_acceptance": feature["observable_sim_acceptance"],
                    "target_acceptance": feature["observable_sim_acceptance"],
                })
            if path.name in question_catalogs:
                replacements.update({
                    "open_questions": feature["open_questions"],
                    "open_questions_assumptions": feature["open_questions"],
                })
            for field, value in replacements.items():
                if field in fieldnames and row.get(field) != value:
                    row[field] = value
                    changed = True
        if changed:
            with path.open("w", newline="", encoding="utf-8") as handle:
                writer = csv.DictWriter(handle, fieldnames=fieldnames)
                writer.writeheader()
                writer.writerows(rows)


def write_csv(features: list[dict[str, str]]) -> None:
    path = CATALOGS / "features.csv"
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FEATURE_FIELDS)
        writer.writeheader()
        writer.writerows(features)


def write_shell_trace_closure(features: list[dict[str, str]]) -> None:
    catalog_names = (
        "frontend-commands.csv",
        "frontend-keybindings.csv",
        "frontend-menus.csv",
        "frontend-component-surfaces.csv",
    )
    expected_ids = {
        row["feature_id"]
        for catalog_name in catalog_names
        for row in read_rows(catalog_name)
    }
    task_id = "comfy-parity-graph-shell-accessibility"
    actual_ids = {
        feature["feature_id"]
        for feature in features
        if task_id in feature["task_id"].split("; ")
    }
    if actual_ids != expected_ids:
        missing = sorted(expected_ids - actual_ids)
        extra = sorted(actual_ids - expected_ids)
        raise RuntimeError(
            "graph shell trace closure mismatch: "
            f"missing={missing[:20]}, extra={extra[:20]}"
        )
    actual_validation_ids = {
        feature["feature_id"]
        for feature in features
        if "VAL-GPUI-012" in feature["validation_id"].split("; ")
    }
    if actual_validation_ids != expected_ids:
        missing = sorted(expected_ids - actual_validation_ids)
        extra = sorted(actual_validation_ids - expected_ids)
        raise RuntimeError(
            "graph shell validation trace closure mismatch: "
            f"missing={missing[:20]}, extra={extra[:20]}"
        )
    features_by_id = {feature["feature_id"]: feature for feature in features}
    rows = []
    for feature_id in sorted(expected_ids):
        feature = features_by_id[feature_id]
        validation_ids = feature["validation_id"].split("; ")
        if "VAL-GPUI-012" not in validation_ids:
            raise RuntimeError(f"{feature_id} is missing VAL-GPUI-012 trace coverage")
        rows.append(
            {
                "feature_id": feature_id,
                "requirement_criteria": feature["requirement_criteria"].split("; "),
                "validation_ids": validation_ids,
            }
        )
    payload = {
        "catalogs": list(catalog_names),
        "feature_count": len(rows),
        "task_id": task_id,
        "validation_id": "VAL-GPUI-012",
        "rows": rows,
    }
    (CATALOGS / "native-shell-trace-closure.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    graph_context_task = "comfy-parity-graph-context-menu-surfaces"
    graph_context_validation = "VAL-GPUI-014"
    graph_context_criteria = {
        "15.1",
        "15.2",
        "15.4",
        "15.5",
        "20.1",
        "20.2",
        "20.3",
        "20.6",
    }
    expected_graph_context_ids = {
        row["feature_id"]
        for row in read_rows("native-menu-dispositions.csv")
        if row["owner_task_id"] == graph_context_task
    }
    actual_graph_context_task_ids = {
        feature["feature_id"]
        for feature in features
        if graph_context_task in feature["task_id"].split("; ")
    }
    actual_graph_context_validation_ids = {
        feature["feature_id"]
        for feature in features
        if graph_context_validation in feature["validation_id"].split("; ")
    }
    if (
        len(expected_graph_context_ids) != 63
        or actual_graph_context_task_ids != expected_graph_context_ids
        or actual_graph_context_validation_ids != expected_graph_context_ids
    ):
        raise RuntimeError(
            "graph context task/VAL-GPUI-014 trace closure must match exactly its 63 generated owner rows"
        )
    for feature_id in expected_graph_context_ids:
        if not graph_context_criteria.issubset(
            features_by_id[feature_id]["requirement_criteria"].split("; ")
        ):
            raise RuntimeError(
                f"{feature_id} does not close every graph context menu criterion"
            )


def counter_table(counter: Counter[str], label: str) -> str:
    lines = [f"| {label} | Features |", "| --- | ---: |"]
    for key, count in sorted(counter.items(), key=lambda item: (-item[1], item[0].casefold())):
        lines.append(f"| {markdown(key)} | {count:,} |")
    lines.append(f"| **Total** | **{sum(counter.values()):,}** |")
    return "\n".join(lines)


def source_coverage_table(catalog_name: str, classification_field: str) -> tuple[int, Counter[str]]:
    rows = read_rows(catalog_name)
    return len(rows), Counter(first(row, classification_field, fallback="unclassified") for row in rows)


def registry_reconciliation() -> list[tuple[str, int, int, str]]:
    backend = json.loads((CATALOGS / "backend-reconciliation.json").read_text(encoding="utf-8"))
    frontend = json.loads((CATALOGS / "frontend-summary.json").read_text(encoding="utf-8"))["counts"]
    cli = json.loads((CATALOGS / "comfy-cli-reconciliation.json").read_text(encoding="utf-8"))
    documentation = json.loads((CATALOGS / "docs-reconciliation.json").read_text(encoding="utf-8"))
    tensor_runtime = json.loads((CATALOGS / "backend-tensor-runtime-reconciliation.json").read_text(encoding="utf-8"))
    desktop_configuration_rows = read_rows("desktop-cli-environment.csv")
    desktop_launch_configuration_count = sum(row.get("kind") != "CSS custom property" for row in desktop_configuration_rows)
    desktop_css_property_count = sum(row.get("kind") == "CSS custom property" for row in desktop_configuration_rows)
    backend_http_rows = read_rows("backend-http-routes.csv")
    desktop_ipc_rows = read_rows("desktop-ipc.csv")
    desktop_preload_rows = read_rows("desktop-preload-apis.csv")
    desktop_renderer_rows = read_rows("desktop-renderer-surfaces.csv")
    desktop_source_coverage = {
        row["path"]: row for row in read_rows("desktop-source-coverage.csv")
    }
    frontend_component_rows = read_rows("frontend-component-surfaces.csv")
    frontend_component_broad_rows = [
        row for row in frontend_component_rows
        if row.get("selection_basis") == "broad-anchor-only"
    ]
    frontend_component_override_rows = [
        row for row in frontend_component_rows
        if row.get("selection_basis") == "required-functional-override"
    ]
    frontend_component_functional_rows = [
        row for row in frontend_component_rows
        if row.get("classification") == "functional Vue component surface"
    ]
    frontend_component_infrastructure_rows = [
        row for row in frontend_component_rows
        if row.get("classification") == "infrastructure-only"
    ]
    frontend_module_rows = read_rows("frontend-functional-modules.csv")
    frontend_module_functional_rows = [
        row for row in frontend_module_rows if row.get("disposition") == "functional-capability"
    ]
    frontend_module_infrastructure_rows = [
        row for row in frontend_module_rows if row.get("disposition") == "infrastructure-only"
    ]
    desktop_counts = {
        "Desktop IPC channels": len(read_rows("desktop-ipc.csv")),
        "Desktop preload API members": len(read_rows("desktop-preload-apis.csv")),
        "Desktop menu actions": len(read_rows("desktop-menu-actions.csv")),
        "Desktop shell actions": len(read_rows("desktop-shell-actions.csv")),
        "Desktop window/application/WebContents/updater events": len(read_rows("desktop-window-events.csv")),
        "Desktop settings": len(read_rows("desktop-settings.csv")),
        "Desktop persisted formats/stores": len(read_rows("desktop-persistence.csv")),
        "Desktop telemetry/event literals": len(read_rows("desktop-telemetry.csv")),
        "Desktop feature flags": len(read_rows("desktop-feature-flags.csv")),
        "Desktop CLI/environment entries": desktop_launch_configuration_count,
        "Desktop renderer CSS custom properties": desktop_css_property_count,
        "Desktop keybindings/gestures": len(read_rows("desktop-keybindings-gestures.csv")),
        "Desktop installation source modes": len(read_rows("desktop-source-plugins.csv")),
        "Desktop platform matrix rows": len(read_rows("desktop-platform-matrix.csv")),
        "Desktop renderer surface contracts": len(desktop_renderer_rows),
    }
    items = [
        ("ComfyUI registered nodes", backend["nodes"]["registered"], len(read_rows("backend-nodes.csv")), "No unresolved registrations; inactive schema-bearing classes are counted separately."),
        ("ComfyUI inactive schema-bearing nodes", backend["nodes"]["inactive_schema_bearing"], len(read_rows("backend-inactive-nodes.csv")), "Not active runtime registry entries."),
        ("ComfyUI runtime-effective HTTP paths", backend["http"]["runtime_effective_rows"], backend["http"]["runtime_effective_rows"], f"Master route catalog has {backend['http']['catalog_rows']} rows after {backend['http']['compatibility_alias_rows']} compatibility aliases and {backend['http']['openapi_only_rows']} OpenAPI-only operations are represented."),
        ("ComfyUI HTTP catalog rows", backend["http"]["catalog_rows"], len(read_rows("backend-http-routes.csv")), "Runtime, alias, static, and OpenAPI-only contracts use distinct rows."),
        ("ComfyUI HTTP rows with route-specific request/response detail", len(backend_http_rows), sum(bool(clean(row.get("request_schema_detail"))) and bool(clean(row.get("response_schema_detail"))) and bool(clean(row.get("unresolved_schema"))) for row in backend_http_rows), "Each row retains a static handler/OpenAPI excerpt and states runtime-only uncertainty explicitly."),
        ("ComfyUI WebSocket event contracts", backend["websocket_events"], len(read_rows("backend-websocket-events.csv")), "JSON and binary/client/server directions are represented."),
        ("ComfyUI config/CLI/environment rows", backend["config_rows"], len(read_rows("backend-config.csv")), "Observed help/flag probes are marked per row."),
        ("ComfyUI model/format/hardware rows", backend["model_rows"], len(read_rows("backend-models.csv")), "Conditional families remain visible."),
        ("ComfyUI tensor operation/reference contracts", tensor_runtime["tensor_operations"]["rows"], len(read_rows("backend-tensor-operations.csv")), "Callable operations, type references, namespace/value references, and receiver-unverified candidates remain distinct; static calls do not imply semantic closure."),
        ("ComfyUI autograd contracts", tensor_runtime["autograd"]["rows"], len(read_rows("backend-autograd.csv")), "Custom Functions, gradient modes/state, checkpointing, mixed precision, optimizers/scalers, and reverse-mode execution are explicit native obligations."),
        ("ComfyUI phase-scoped RNG contracts", tensor_runtime["rng"]["rows"], len(read_rows("backend-rng.csv")), "Named phases retain seed/generator/device sites, retry/cancellation policy, and no-global-RNG native decisions."),
        ("ComfyUI Python files scanned for tensor runtime", tensor_runtime["source_closure"]["python_files_scanned"], sum(tensor_runtime["source_closure"]["parser_modes"].values()), "Every scanned file maps to the canonical 949-row backend source ledger; one Python 3.10 match/case file uses syntax-only normalization."),
        ("ComfyUI persisted formats/migrations", backend["format_rows"], len(read_rows("backend-formats.csv")), "Prompt, queue/history, database, users, media, model/tensor, and migration contracts are represented."),
        ("ComfyUI schema rows", backend["schema_rows"], len(read_rows("backend-schemas.csv")), "Executable and OpenAPI component schemas are retained."),
        ("ComfyUI hosted external endpoints", backend["external_endpoint_rows"], len(read_rows("backend-external-services.csv")), "No live provider was called."),
        ("Frontend commands", frontend["commands"], len(read_rows("frontend-commands.csv")), "Literal and dynamically registered command IDs reconciled."),
        ("Frontend default keybindings", frontend["default_keybindings"], len(read_rows("frontend-keybindings.csv")), "Each binding resolves to a command."),
        ("Frontend menus", frontend["menu_rows"], len(read_rows("frontend-menus.csv")), "Command-backed and local actions are distinct."),
        ("Frontend settings", frontend["settings"], len(read_rows("frontend-settings.csv")), "149 literal definitions plus three explicit schema-only uncertain entries."),
        ("Frontend routes", frontend["routes"], len(read_rows("frontend-routes.csv")), "Main, Desktop UI, and website routes."),
        ("Frontend WebSocket/local events", frontend["websocket_and_frontend_events"], len(read_rows("frontend-websocket.csv")), "Backend-received and local events."),
        ("Frontend HTTP client contracts", frontend["http_contract_rows"], len(read_rows("frontend-http-usage.csv")), "Literal plus reconciled dynamic calls."),
        ("Frontend feature/config flags", frontend["feature_flags"], len(read_rows("frontend-feature-flags.csv")), "Client hello, server flags, and remote config."),
        ("Frontend telemetry rows", frontend["telemetry_rows"], len(read_rows("frontend-telemetry.csv")), "Events and literal button identifiers."),
        ("Frontend persisted formats/migrations", frontend["formats_and_migrations"], len(read_rows("frontend-formats-migrations.csv")), "Formats and explicit migrations."),
        ("Frontend persisted state keys", frontend["persisted_state_rows"], len(read_rows("frontend-persisted-state.csv")), "Literal and dynamic patterns."),
        ("Frontend extension contracts", frontend["extension_rows"], len(read_rows("frontend-extensions.csv")), "Interface members and core modules."),
        ("Frontend broad-anchor-only production/cloud/platform Vue files", len(frontend_component_broad_rows), len(frontend_component_broad_rows), "Every audit-predicate match has one stable source-specific component contract."),
        ("Frontend explicitly required already-referenced Vue surfaces", len(frontend_component_override_rows), len(frontend_component_override_rows), "AssetsSidebarTab is retained as a functional component contract even though another authoritative catalog already cited its source path."),
        ("Frontend functional Vue component surface contracts", len(frontend_component_functional_rows), len(frontend_component_functional_rows), "Each row records concrete props/models/emits/events/handlers, visible states, failure, accessibility, persistence, interfaces, and validation."),
        ("Frontend presentational Vue infrastructure dispositions", len(frontend_component_infrastructure_rows), len(frontend_component_infrastructure_rows), "Each row has a source-specific render-only reason and remains traceable through its consuming surface."),
        ("Frontend functional-module predicate candidates", len(frontend_module_rows), len(frontend_module_rows), "Every normalized broad-anchor-only service/store/composable candidate has one stable source-specific contract."),
        ("Frontend functional module capabilities", len(frontend_module_functional_rows), len(frontend_module_functional_rows), "Each row records exports, transitions, async lifetime, errors, persistence, side effects, source digest, and validation."),
        ("Frontend functional module infrastructure dispositions", len(frontend_module_infrastructure_rows), len(frontend_module_infrastructure_rows), "Each pure plumbing/re-export/helper row has a source-specific non-capability reason and consuming anchor."),
        ("Cross-product persisted formats and media carriers", len(read_rows("cross-formats.csv")), len(read_rows("cross-formats.csv")), "Workflow, prompt, metadata, model, output, legacy, and migration carriers are reconciled across producers/consumers."),
        ("Cross-product compatibility contracts", len(read_rows("cross-compatibility.csv")), len(read_rows("cross-compatibility.csv")), "REST, WebSocket, IPC, extension, mode, identifier, state, and source-conflict contracts are represented."),
    ]
    items.extend((label, count, count, "Every discovered row has a stable master feature ID and retains its parent Desktop feature ID.") for label, count in desktop_counts.items())
    items.extend([
        (
            "Desktop renderer rows with source-specific contracts and source-file mappings",
            len(desktop_renderer_rows),
            sum(
                all(clean(row.get(field)) for field in (
                    "feature_id", "component_surface", "parent_feature_ids", "classification",
                    "availability", "evidence_level", "source_file", "source_symbol", "props",
                    "emits", "handlers", "actor", "trigger", "preconditions", "inputs_defaults",
                    "observable_success", "interaction_accessibility", "state_concurrency",
                    "failure_recovery", "persistence_serialization", "interfaces_side_effects",
                    "disposition_reason",
                ))
                and row["feature_id"] in desktop_source_coverage.get(row["source_file"], {}).get("feature_ids", "").split(";")
                for row in desktop_renderer_rows
            ),
            "Every formerly broad-anchor-only production Vue file has one stable functional or explicit presentational/infrastructure contract and the same ID in desktop-source-coverage.csv.",
        ),
        ("Desktop menu action rows with honest per-item evidence", len(read_rows("desktop-menu-actions.csv")), sum(row.get("evidence_level") == "code-inferred" for row in read_rows("desktop-menu-actions.csv")), "No derived menu item claims test-backed evidence without a focused test for its exact action and condition."),
        ("Desktop discarded legacy settings", 4, sum(row.get("availability") == "deprecated/dead" and row.get("key") in {"primaryInstallId", "pinnedInstallIds", "maxCachedFiles", "closeDirectlyOnLastWindow"} for row in read_rows("desktop-settings.csv")), "Load-time removal and rewrite behavior is retained instead of omitting obsolete keys."),
        ("Desktop telemetry/event rows with source, payload, consent, redaction, and rate detail", len(read_rows("desktop-telemetry.csv")), sum(all(clean(row.get(field)) for field in ("source_evidence", "payload_evidence", "consent_behavior", "redaction_validation", "rate_limit_dedup", "provider_side_effects")) for row in read_rows("desktop-telemetry.csv")), "Every production comfy.desktop.* and app:* literal has a stable ID and explicit provider/infrastructure disposition."),
        ("Desktop IPC rows with request/event and response/callback detail", len(desktop_ipc_rows), sum(bool(clean(row.get("request_or_event_schema"))) and bool(clean(row.get("response_or_callback_schema"))) and bool(clean(row.get("unresolved_schema"))) for row in desktop_ipc_rows), "Exact static handler/call excerpts are retained; runtime structured-clone and error variants remain explicit contract-test work."),
        ("Desktop preload members with exact source signatures", len(desktop_preload_rows), sum(bool(clean(row.get("source_signature"))) and bool(clean(row.get("unresolved_schema"))) for row in desktop_preload_rows), "Every bridge member retains the cited TypeScript declaration and unresolved runtime boundaries."),
    ])
    items.extend([
        ("comfy-cli reachable command leaves", cli["commands"], len(read_rows("comfy-cli-commands.csv")), "Hidden aliases and the shadowed/dead command collision remain explicitly classified."),
        ("comfy-cli option/argument bindings", cli["parameters"], len(read_rows("comfy-cli-parameters.csv")), "Source declarations, alias repetitions, command scope, resolved types, nullability, value arity/cardinality, repeatability, enum choices, explicit parser constraints, paired boolean forms, defaults, envvars, hidden state, extraction evidence, and globals are retained."),
        ("comfy-cli JSON schemas", cli["schemas"], len(read_rows("comfy-cli-schemas.csv")), "Envelope/stream mappings and the orphan comfy version mapping are reconciled separately."),
        ("comfy-cli stable error codes", cli["errors"], len(read_rows("comfy-cli-errors.csv")), "Every error code retains meaning, hint, evidence, and native target decision."),
        ("comfy-cli event union", cli["events"], len(read_rows("comfy-cli-events.csv")), "The converted, prompt_preview, settled, and state schema/emitter mismatches remain explicit."),
        ("comfy-cli production environment variables", cli["environment"], len(read_rows("comfy-cli-environment.csv")), "Test/CI-only variables remain in source coverage and are not promoted to production controls."),
        ("comfy-cli configuration keys", cli["config"], len(read_rows("comfy-cli-config.csv")), "Every key has a native mapping or architecture-conflicting migration decision."),
        ("comfy-cli persisted/interchange formats", cli["formats"], len(read_rows("comfy-cli-formats.csv")), "The comfy-lock.yaml prose versus comfy.lock.yaml executable spelling conflict is retained."),
        ("comfy-cli lifecycle contracts", cli["lifecycle"], len(read_rows("comfy-cli-lifecycle.csv")), "Python child lifecycle rows remain architecture conflicts; observable stages map to native operations."),
        ("comfy-cli extension contracts", cli["extensions"], len(read_rows("comfy-cli-extensions.csv")), "Python/frontend override execution is prohibited; legacy identities map to Rust/WASM ports/placeholders."),
        ("comfy-cli CQL policy rows", cli["cql_rows"], len(read_rows("comfy-cli-cql-policy.csv")), "Pack labels, node policies, versions, Git refs, and cloud-disabling labels reconcile."),
        ("comfy-cli partner allowlist endpoints", cli["partner_endpoints"], len(read_rows("comfy-cli-partner-openapi.csv")), "52 aliases reconcile to the allowlist; excluded/proxy OpenAPI totals remain in the source ledger."),
        ("comfy-cli capability records", cli["capability_features"], sum(len(read_rows(name)) for name in ("comfy-cli-commands.csv", "comfy-cli-parameters.csv", "comfy-cli-schemas.csv", "comfy-cli-errors.csv", "comfy-cli-events.csv", "comfy-cli-config.csv", "comfy-cli-environment.csv", "comfy-cli-formats.csv", "comfy-cli-lifecycle.csv", "comfy-cli-extensions.csv", "comfy-cli-cql-policy.csv", "comfy-cli-partner-openapi.csv", "comfy-cli-documentation.csv")), "All behavioral capability rows are promoted to the master feature ledger; tests/source support remain separate closure ledgers."),
        ("comfy-cli module/service contracts", cli["modules"], len(read_rows("comfy-cli-modules.csv")), "Every production module row is promoted to a master module/service contract so source-file closure requires a master feature ID."),
        ("docs authoritative/content records", documentation["catalog_counts"]["docs_pages"], len(read_rows("docs-pages.csv")), "English source, snippets, staging, roles, corroboration, and documented-only treatment are retained."),
        ("docs built-in node page reconciliation", documentation["catalog_counts"]["docs_node_docs"], len(read_rows("docs-node-docs.csv")), "Registry and embedded-doc exact/case/normalized/unverified deltas remain explicit."),
        ("embedded-docs node records", documentation["catalog_counts"]["embedded_docs_nodes"], len(read_rows("embedded-docs-nodes.csv")), "All locales, assets, AI-generated markers, fingerprints, sync, and registry matches reconcile."),
        ("docs Cloud OpenAPI operations", documentation["catalog_counts"]["docs_openapi_cloud"], len(read_rows("docs-openapi-cloud.csv")), "Route-shape corroboration does not promote documented cloud semantics to executable evidence."),
        ("docs redirects", documentation["catalog_counts"]["docs_redirects"], len(read_rows("docs-redirects.csv")), "Path and case behavior is retained as documentation-site configuration."),
        ("docs tooling contracts", documentation["catalog_counts"]["docs_tooling"], len(read_rows("docs-tooling.csv")), "Package scripts, CI, checks, and developer tools remain code-inferred developer/infrastructure behavior."),
        ("docs configuration/format records", documentation["catalog_counts"]["docs_config_formats"], len(read_rows("docs-config-formats.csv")), "Schemas, locks, navigation, and registries remain evidence records rather than product behavior unless corroborated."),
        ("docs extension contracts", documentation["catalog_counts"]["docs_extension_contracts"], len(read_rows("docs-extension-contracts.csv")), "All legacy behaviors map to explicit Rust/WASM ports or prohibited execution."),
        ("docs lifecycle contracts", documentation["catalog_counts"]["docs_lifecycle_contracts"], len(read_rows("docs-lifecycle-contracts.csv")), "Embedded version skew and documentation-only lifecycle claims remain visible."),
    ])
    return items


def write_source_inventory(features: list[dict[str, str]]) -> None:
    product = Counter(feature["product"] for feature in features)
    domain = Counter(feature["domain"] for feature in features)
    classification = Counter(feature["classification"] for feature in features)
    evidence = Counter(feature["evidence_level"] for feature in features)
    availability = Counter(feature["availability"] for feature in features)
    status = Counter(feature["current_sim_status"] for feature in features)
    for target_status in ("equivalent", "partial", "missing", "conflicting", "deferred", "uncertain"):
        status.setdefault(target_status, 0)
    catalog_counts = Counter(feature["source_catalog"] for feature in features)
    runtime_eligible = [
        feature
        for feature in features
        if feature["classification"] not in {"coverage-anchor", "coverage reconciliation"}
    ]
    observed = sum(feature["evidence_level"] == "observed" for feature in runtime_eligible)
    runtime_percentage = 100.0 * observed / len(runtime_eligible) if runtime_eligible else 0.0

    backend_files, backend_file_classes = source_coverage_table("backend-source-coverage.csv", "classification")
    frontend_files, frontend_file_classes = source_coverage_table("frontend-source-files.csv", "classification")
    desktop_files, desktop_file_classes = source_coverage_table("desktop-source-coverage.csv", "classification")
    cli_files, cli_file_classes = source_coverage_table("comfy-cli-source-coverage.csv", "classification")
    docs_files, docs_file_classes = source_coverage_table("docs-source-coverage.csv", "disposition")
    embedded_docs_files, embedded_docs_file_classes = source_coverage_table("embedded-docs-source-coverage.csv", "disposition")

    registry_lines = ["| Registry or manifest | Discovered | Cataloged | Reconciliation |", "| --- | ---: | ---: | --- |"]
    for label, discovered, cataloged, notes in registry_reconciliation():
        registry_lines.append(f"| {markdown(label)} | {discovered:,} | {cataloged:,} | {markdown(notes)} |")

    source_lines = ["| Product | Files | Explicit classifications |", "| --- | ---: | --- |"]
    for label, count, classes in (
        ("ComfyUI", backend_files, backend_file_classes),
        ("ComfyUI-Frontend", frontend_files, frontend_file_classes),
        ("Comfy-Desktop", desktop_files, desktop_file_classes),
        ("comfy-cli", cli_files, cli_file_classes),
        ("docs", docs_files, docs_file_classes),
        ("embedded-docs", embedded_docs_files, embedded_docs_file_classes),
    ):
        details = "; ".join(f"{key}={value}" for key, value in sorted(classes.items()))
        source_lines.append(f"| {label} | {count:,} | {markdown(details)} |")

    test_counts = {
        "ComfyUI test functions": len(read_rows("backend-tests.csv")),
        "Frontend Playwright declared cases": len(read_rows("frontend-test-cases.csv")),
        "Comfy-Desktop test files": len(read_rows("desktop-tests.csv")),
        "Comfy-Desktop declared suites/cases": sum(
            int(first(row, "suite_or_case_count", fallback="0")) for row in read_rows("desktop-tests.csv")
        ),
        "comfy-cli test functions": len(read_rows("comfy-cli-tests.csv")),
        "docs executable Bun tests": 8,
        "embedded-docs local link-check suite": 1,
    }
    test_lines = ["| Test ledger | Rows | Runtime rerun in this audit |", "| --- | ---: | --- |"]
    for label, count in test_counts.items():
        if label == "docs executable Bun tests":
            run = "Yes; 8/8 passed."
        elif label == "embedded-docs local link-check suite":
            run = "Yes; the link checker passed."
        else:
            run = "No; dependency/runtime constraints are recorded in baseline.md."
        test_lines.append(f"| {label} | {count:,} | {run} |")

    orphan_frontend = json.loads((CATALOGS / "frontend-summary.json").read_text(encoding="utf-8"))["orphan_sets"]
    orphan_lines = ["| Orphan search | Result |", "| --- | --- |"]
    for name, values in orphan_frontend.items():
        orphan_lines.append(f"| Frontend {markdown(name.replace('_', ' '))} | {len(values)} retained: {markdown('; '.join(values) if values else 'none')} |")
    cli_reconciliation = json.loads((CATALOGS / "comfy-cli-reconciliation.json").read_text(encoding="utf-8"))
    for name, reason in cli_reconciliation["orphaned_surfaces"].items():
        orphan_lines.append(f"| comfy-cli {markdown(name)} | {markdown(reason)} |")
    docs_reconciliation = json.loads((CATALOGS / "docs-reconciliation.json").read_text(encoding="utf-8"))
    orphan_lines.append(f"| docs English navigation exact missing | {len(docs_reconciliation['docs']['english_navigation_exact_missing'])} path-case/content references retained in docs-reconciliation.json. |")
    orphan_lines.append("| docs translation validation | 51 reported truncation/translation issues retained; generated reports were removed and the source fingerprint restored. |")

    text = f"""# Source inventory

## Inventory boundary

The normative feature ledger is [`catalogs/features.csv`](catalogs/features.csv). It contains **{len(features):,}** stable, non-reused feature contracts derived from ComfyUI, ComfyUI-Frontend, Comfy-Desktop, comfy-cli, docs, embedded-docs, Sim target evidence, and cross-product compatibility surfaces. Separate registry, localization, telemetry, test, generated-documentation, and source-file ledgers remain authoritative for count reconciliation even where an individual row is metadata or coverage support rather than a distinct user workflow.

No source application, account, paid service, dependency set, or remote state was modified. Runtime evidence includes the safe ComfyUI parser/feature-flag probes, docs link and Bun tests, and embedded-docs link check recorded in [baseline.md](baseline.md). The comfy-cli runtime probe failed before command construction because Python 3.9.6 is below the declared 3.10 minimum and `questionary` is unavailable. Existing tests support `test-backed` classifications only where focused; a test-backed row is not represented as locally passing unless the baseline records a successful run.

## Feature counts by product

{counter_table(product, "Product")}

## Feature counts by domain

{counter_table(domain, "Domain")}

## Feature counts by source classification

{counter_table(classification, "Classification")}

## Feature counts by evidence level

{counter_table(evidence, "Evidence level")}

Direct runtime validation covers **{observed:,}/{len(runtime_eligible):,} ({runtime_percentage:.2f}%)** independently testable master rows. This percentage counts only `observed` rows, not inspected tests, and excludes explicit coverage anchors/reconciliation rows.

## Feature counts by availability

{counter_table(availability, "Availability")}

## Current Sim status

{counter_table(status, "Status")}

Generic workspace, GPUI, settings, persistence, subprocess, action, focus, Wasmtime, wgpu/Metal, media, and visual-test primitives alone are design inputs, not Comfy behavior. Native Comfy foundations now have task-level implementation and validation evidence, but master feature rows remain `missing`, `conflicting`, `deferred`, or narrowly `partial` until their exact per-feature behavior and final closure artifacts pass; planned code is never promoted. Python/JavaScript extension execution and Python/server lifecycle rows are `conflicting` with the production-native boundary and map to Rust/WASM or native lifecycle migrations. {ACCESSIBILITY_INVENTORY_SUMMARY} Cross-product disagreements remain `conflicting`. `deferred` rows are still source-traced and preserve compatibility or an explicit service/product decision.

## Registry-to-inventory reconciliation

{chr(10).join(registry_lines)}

The frontend localization ledger contains {len(read_rows('frontend-localization.csv')):,} rows; the Desktop localization ledger contains {len(read_rows('desktop-localization.csv')):,} matched scalar paths. Localization rows are count-reconciled data contracts rather than one feature per translated scalar. The master ledger maps their consuming settings, commands, menus, routes, notifications, errors, and surfaces.

## Source-file coverage

{chr(10).join(source_lines)}

Every source file in all six source repositories has a ledger row and either one or more feature/record mappings or an explicit production, infrastructure, generated, translated mirror, test-only/support, asset, documentation, staging, deprecated/dead, or placeholder classification with a reason. Infrastructure, translations, and test-support files are not promoted into fictional executable behavior. Sim target evidence is separately mapped in `catalogs/sim-architecture.csv` and [evidence-sim.md](evidence-sim.md).

## Tests, fixtures, stories, and snapshots

{chr(10).join(test_lines)}

Frontend source reconciliation additionally records 1,013 unit/component test files and 77 Storybook files. Desktop reconciliation records 3,422 suite-or-case declarations across its 232 test files. comfy-cli records 2,295 test functions, 316 classes, and 129 fixtures but none ran locally. The docs audit ran 8/8 Bun tests and checked 4,988 documentation files with a passing validator; embedded-docs passed its local link checker. These totals characterize evidence reach and are not generalized beyond the recorded runs.

## Orphan and uncertainty reconciliation

{chr(10).join(orphan_lines)}

The 40 schema settings without English labels and the three schema settings without literal definitions are retained as hidden, compatibility, extension, or uncertain state rather than omitted. Backend dynamic custom nodes and API extensions not present in the snapshot remain open-world contracts. comfy-cli retains its shadowed `models` function, orphan `comfy version` schema mapping, prose-only `comfy query`, event-union drift, filename spelling conflict, and documentation-only Keyframe Relay claim. Docs retains navigation/path-case/localization deltas, three uncorroborated Cloud OpenAPI operations, provider-unverified node pages, and embedded-docs 0.5.7 versus ComfyUI pin 0.5.6. Cloud behavior, platform-native branches, hardware inference, installed plugins, and paid provider outcomes remain explicit runtime uncertainties.

## Master-catalog provenance

{counter_table(catalog_counts, "Source catalog")}

The individual source ledgers contain richer registry-specific columns. `catalogs/features.csv` normalizes those rows into actor, trigger, conditions, observable state, failure/recovery, persistence, protocol/side effects, platform variants, target gap, acceptance, and trace fields without replacing the source ledgers.
"""
    (ROOT / "source-inventory.md").write_text(text, encoding="utf-8")


def write_parity_matrix(features: list[dict[str, str]]) -> None:
    lines = [
        "# Parity matrix",
        "",
        "Each row is one stable source contract. `Source behavior/evidence` is a concise index into the richer machine ledger and product-specific catalogs. A `deferred` status never removes the source contract or its validation obligation; it records a deliberate compatibility/service decision.",
        "",
        "| Feature ID | Product / domain / name | Source behavior / evidence | Sim status / evidence | Gap | Parity decision |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for feature in features:
        source = joined(feature["observable_success"], f"Evidence: {feature['evidence_level']} — {feature['source_evidence']}")
        sim = joined(feature["current_sim_status"], feature["sim_evidence"])
        lines.append(
            f"| `{markdown(feature['feature_id'])}` | {markdown(feature['product'])} / {markdown(feature['domain'])} / {markdown(feature['name'])} | {markdown(source, 900)} | {markdown(sim, 500)} | {markdown(feature['parity_gap'], 500)} | {markdown(feature['parity_decision'])} |"
        )
    lines.append("")
    (ROOT / "parity-matrix.md").write_text("\n".join(lines), encoding="utf-8")


def write_traceability(features: list[dict[str, str]]) -> None:
    lines = [
        "# Traceability",
        "",
        "The generated table provides forward source-to-validation traceability and the data needed for reverse coverage checks. The task column records direct feature owners plus one stable anchor per mapped requirement; `catalogs/native-spec-mapping.json` records the normalized criterion-to-task closure without repeating hundreds of task IDs in every feature row. No cell is blank; uncertainty is stated rather than represented by an empty value.",
        "",
        "| Source evidence | Feature ID | Requirement criterion | Design component / decision | Task | Validation |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for feature in features:
        lines.append(
            f"| {markdown(feature['source_evidence'])} | `{markdown(feature['feature_id'])}` | {markdown(feature['requirement_criteria'])} | {markdown(feature['design_coverage'])} | {markdown(feature['task_id'])} | {markdown(feature['validation_id'])} |"
        )
    lines.append("")

    criteria_referenced = set()
    designs_referenced = set()
    tasks_referenced = set()
    validations_referenced = set()
    for feature in features:
        feature_criteria = {
            part.strip()
            for part in feature["requirement_criteria"].split(";")
            if part.strip()
        }
        criteria_referenced.update(feature_criteria)
        designs_referenced.update(re.findall(r"\bD\d+\b", feature["design_coverage"]))
        tasks_referenced.update(part.strip() for part in feature["task_id"].split(";") if part.strip())
        for criterion in feature_criteria:
            tasks_referenced.update(CRITERION_TASKS[criterion])
        validations_referenced.update(part.strip() for part in feature["validation_id"].split(";") if part.strip())

    all_criteria = {f"{requirement}.{criterion}" for requirement, count in REQUIREMENT_CRITERIA_COUNTS.items() for criterion in range(1, count + 1)}
    all_designs = set(MAPPING["design_ids"])
    all_tasks = set(re.findall(
        r"^\s*-\s+_id:\s*([a-z0-9._-]+)\s*$",
        (ROOT / "tasks.md").read_text(encoding="utf-8"),
        re.MULTILINE,
    ))
    all_validations = set(re.findall(r"VAL-[A-Z0-9-]+-\d{3}", (ROOT / "validation.md").read_text(encoding="utf-8")))

    checks = [
        ("Feature rows with source, requirement, design, task, and validation", len(features), len(features)),
        ("Requirement criteria referenced", len(all_criteria), len(criteria_referenced & all_criteria)),
        ("Design decisions referenced", len(all_designs), len(designs_referenced & all_designs)),
        ("Tasks referenced", len(all_tasks), len(tasks_referenced & all_tasks)),
        ("Validation identifiers referenced", len(all_validations), len(validations_referenced & all_validations)),
    ]
    lines.extend(["## Reverse coverage summary", "", "| Set | Expected | Referenced | Coverage |", "| --- | ---: | ---: | ---: |"])
    for label, expected, actual in checks:
        percentage = 100.0 * actual / expected if expected else 100.0
        lines.append(f"| {label} | {expected:,} | {actual:,} | {percentage:.2f}% |")
    lines.append("")
    (ROOT / "traceability.md").write_text("\n".join(lines), encoding="utf-8")

    failures = []
    if all_criteria - criteria_referenced:
        failures.append(f"unreferenced criteria: {sorted(all_criteria - criteria_referenced)}")
    if all_designs - designs_referenced:
        failures.append(f"unreferenced designs: {sorted(all_designs - designs_referenced)}")
    if all_tasks - tasks_referenced:
        failures.append(f"unreferenced tasks: {sorted(all_tasks - tasks_referenced)}")
    if all_validations - validations_referenced:
        failures.append(f"unreferenced validations: {sorted(all_validations - validations_referenced)}")
    if failures:
        raise RuntimeError("; ".join(failures))


def write_reconciliation_json(features: list[dict[str, str]]) -> None:
    fields = ("product", "domain", "classification", "availability", "evidence_level", "current_sim_status", "source_catalog")
    runtime_eligible = [
        feature
        for feature in features
        if feature["classification"] not in {"coverage-anchor", "coverage reconciliation"}
    ]
    runtime_validated = sum(feature["evidence_level"] == "observed" for feature in runtime_eligible)
    http_rows = read_rows("backend-http-routes.csv")
    desktop_ipc_rows = read_rows("desktop-ipc.csv")
    desktop_preload_rows = read_rows("desktop-preload-apis.csv")
    backend_source_rows = read_rows("backend-source-coverage.csv")
    backend_test_rows = read_rows("backend-tests.csv")
    cli_source_rows = read_rows("comfy-cli-source-coverage.csv")
    cli_test_rows = read_rows("comfy-cli-tests.csv")
    docs_source_rows = read_rows("docs-source-coverage.csv")
    embedded_docs_source_rows = read_rows("embedded-docs-source-coverage.csv")
    master_feature_ids = {feature["feature_id"] for feature in features}
    cli_source_master_mappings = {
        row["path"]: [
            identifier.strip()
            for identifier in row["feature_ids"].split("|")
            if identifier.strip() in master_feature_ids
        ]
        for row in cli_source_rows
    }
    counts = {field: Counter(feature[field] for feature in features) for field in fields}
    for target_status in ("equivalent", "partial", "missing", "conflicting", "deferred", "uncertain"):
        counts["current_sim_status"].setdefault(target_status, 0)
    summary = {
        "feature_rows": len(features),
        "counts": {field: dict(sorted(counter.items())) for field, counter in counts.items()},
        "runtime_validation": {
            "observed_rows": runtime_validated,
            "independently_testable_rows": len(runtime_eligible),
            "percentage": round(100.0 * runtime_validated / len(runtime_eligible), 4) if runtime_eligible else 0.0,
        },
        "traceability": {
            "features_with_all_links": sum(all(feature[field] for field in ("source_evidence", "requirement_criteria", "design_coverage", "task_id", "validation_id")) for feature in features),
            "feature_rows": len(features),
        },
        "production_native_boundary": {
            "decision": "Production Sim implements Comfy execution entirely in native Rust and may use source applications only as development-time conformance oracles.",
            "python_comfy_process_allowed": False,
            "external_comfy_connection_allowed": False,
            "python_extension_execution_allowed": False,
            "javascript_extension_execution_allowed": False,
            "replacement_plugin_api": "versioned Rust source trait plus WASM Component Model WIT with explicit ports and deterministic legacy mappings",
        },
        "task18_execution_disposition_reconciliation": {
            "queue_features": {
                feature_id: {
                    "disposition": kind,
                    "current_owner": (
                        owner
                        if kind == "foundation"
                        else EXECUTION_UI_OWNER if kind != "later_owned" else None
                    ),
                    "closure_owner": EXECUTION_UI_OWNER if kind == "native" else owner,
                    "target_status": "deferred" if kind == "later_owned" else "partial",
                }
                for feature_id, (kind, owner) in sorted(TASK_18_QUEUE_DISPOSITIONS.items())
            },
            "queue_disposition_counts": dict(sorted(Counter(
                kind for kind, _owner in TASK_18_QUEUE_DISPOSITIONS.values()
            ).items())),
            "execution_commands": {
                command_id: {
                    "feature_id": feature_id,
                    "owner": owner,
                    "native_action": native,
                    "target_status": "partial" if native else "deferred",
                }
                for command_id, (feature_id, owner, native) in sorted(
                    TASK_18_EXECUTION_COMMANDS.items()
                )
            },
            "job_run_menus": {
                feature_id: {
                    "owner": owner,
                    "target_status": (
                        "partial" if owner == EXECUTION_UI_OWNER else "deferred"
                    ),
                }
                for feature_id, owner in sorted(TASK_18_JOB_RUN_MENU_OWNERS.items())
            },
            "execution_components": {
                feature_id: {
                    "owner": owner,
                    "target_status": (
                        "partial" if owner == EXECUTION_UI_OWNER else "deferred"
                    ),
                }
                for feature_id, owner in sorted(TASK_18_COMPONENT_OWNERS.items())
            },
        },
        "schema_detail_reconciliation": {
            "http_rows": len(http_rows),
            "http_rows_with_request_detail": sum(bool(clean(row.get("request_schema_detail"))) for row in http_rows),
            "http_rows_with_response_detail": sum(bool(clean(row.get("response_schema_detail"))) for row in http_rows),
            "http_rows_with_explicit_unresolved_detail": sum(bool(clean(row.get("unresolved_schema"))) for row in http_rows),
            "desktop_ipc_rows": len(desktop_ipc_rows),
            "desktop_ipc_rows_with_request_event_detail": sum(bool(clean(row.get("request_or_event_schema"))) for row in desktop_ipc_rows),
            "desktop_ipc_rows_with_response_callback_detail": sum(bool(clean(row.get("response_or_callback_schema"))) for row in desktop_ipc_rows),
            "desktop_ipc_rows_with_explicit_unresolved_detail": sum(bool(clean(row.get("unresolved_schema"))) for row in desktop_ipc_rows),
            "desktop_preload_rows": len(desktop_preload_rows),
            "desktop_preload_rows_with_source_signature": sum(bool(clean(row.get("source_signature"))) for row in desktop_preload_rows),
            "desktop_preload_rows_with_explicit_unresolved_detail": sum(bool(clean(row.get("unresolved_schema"))) for row in desktop_preload_rows),
        },
        "coverage_mapping_reconciliation": {
            "backend_source_rows": len(backend_source_rows),
            "backend_production_rows_without_feature_mapping": sum(
                row["classification"].startswith("production") and not clean(row["mapped_feature_ids"])
                for row in backend_source_rows
            ),
            "backend_test_rows": len(backend_test_rows),
            "backend_test_rows_without_feature_mapping": sum(not clean(row["mapped_feature_ids"]) for row in backend_test_rows),
            "comfy_cli_source_rows": len(cli_source_rows),
            "comfy_cli_production_rows_without_feature_mapping": sum(
                row["classification"] == "production" and not cli_source_master_mappings[row["path"]]
                for row in cli_source_rows
            ),
            "comfy_cli_test_rows": len(cli_test_rows),
            "comfy_cli_test_rows_without_source_reference": sum(not clean(row["source_file"]) for row in cli_test_rows),
            "docs_source_rows": len(docs_source_rows),
            "docs_source_rows_without_disposition_or_reason": sum(
                not clean(row["disposition"]) or not clean(row["reason"])
                for row in docs_source_rows
            ),
            "embedded_docs_source_rows": len(embedded_docs_source_rows),
            "embedded_docs_source_rows_without_disposition_or_reason": sum(
                not clean(row["disposition"]) or not clean(row["reason"])
                for row in embedded_docs_source_rows
            ),
        },
        "registry_reconciliation": [
            {"name": label, "discovered": discovered, "cataloged": cataloged, "notes": notes}
            for label, discovered, cataloged, notes in registry_reconciliation()
        ],
    }
    (CATALOGS / "master-reconciliation.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> None:
    features = build_features()
    synchronize_source_catalog_targets(features)
    write_csv(features)
    write_shell_trace_closure(features)
    write_source_inventory(features)
    write_parity_matrix(features)
    write_traceability(features)
    write_reconciliation_json(features)
    sync_cross_product_trace(features)
    print(f"Generated {len(features)} feature rows with nonblank trace fields.")


if __name__ == "__main__":
    main()
