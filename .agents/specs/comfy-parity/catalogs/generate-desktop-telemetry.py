#!/usr/bin/env python3
"""Regenerate the production Comfy-Desktop telemetry/event literal ledger."""

from __future__ import annotations

import argparse
import csv
import hashlib
import re
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
DESKTOP = ROOT / "projects/comfy/Comfy-Desktop"
DEFAULT_OUTPUT = Path(__file__).resolve().parent / "desktop-telemetry.csv"
LITERAL_RE = re.compile(
    r"(['\"])(comfy\.desktop\.[A-Za-z0-9_.:-]+|app:[A-Za-z0-9_.:-]+)\1"
)
EMITTERS = {
    "capture",
    "emit",
    "emitTelemetry",
    "emitTelemetryAction",
    "trackTelemetryAction",
    "captureTelemetry",
    "_emitWarning",
    "trackedStep",
}
FIELDS = [
    "feature_id",
    "event_name",
    "event_kind",
    "classification",
    "availability",
    "evidence_level",
    "source_evidence",
    "payload_evidence",
    "consent_behavior",
    "redaction_validation",
    "rate_limit_dedup",
    "provider_side_effects",
    "derived_wire_names",
    "notes",
]

PAYLOAD_SUPPLEMENTS = {
    "comfy.desktop.app.first_launch": "Exact producer at projects/comfy/Comfy-Desktop/src/main/index.ts:1512-1515 supplies {id_class, locale}; projects/comfy/Comfy-Desktop/src/main/lib/telemetry.ts:576-596 may merge {download_token, download_token_source} before deferred/immediate capture.",
    "comfy.desktop.install.phase": "Exact construction at projects/comfy/Comfy-Desktop/src/main/sources/standalone/install.ts:43-64 is {installation_id, variant, phase, status, duration_ms?}; error status additionally supplies error_bucket.",
    "comfy.desktop.install.standalone": "Exact installContext at projects/comfy/Comfy-Desktop/src/main/lib/standaloneMigration.ts:361-365 is {installation_id, release_tag, variant_id}.",
    "comfy.desktop.install.post_install": "Exact installContext at projects/comfy/Comfy-Desktop/src/main/lib/standaloneMigration.ts:361-365 is {installation_id, release_tag, variant_id}.",
    "comfy.desktop.migrate.flow": "Exact flowContext at projects/comfy/Comfy-Desktop/src/main/lib/ipc/sessionActions/migrate.ts:299-302 is {source_id, source_installation_id}.",
    "comfy.desktop.person.set": "Exact construction at projects/comfy/Comfy-Desktop/src/main/lib/telemetry.ts:828-844 is {$set?: scrubProperties(set), $set_once?: scrubProperties(setOnce)}; empty updates are dropped.",
    "comfy.desktop.session.instance_started": "Exact construction at projects/comfy/Comfy-Desktop/src/main/lib/ipc/sessionStartTelemetry.ts:94-133 spreads buildInstallationDdContext metadata and adds {boot_time_ms, port_retries, reboot_retries, custom_nodes_count, pip_packages_count, latest_snapshot_json, latest_snapshot_json_truncated}.",
    "comfy.desktop.session.installation_started": "Deprecated shadow with the exact same instanceStartedProps as session.instance_started at projects/comfy/Comfy-Desktop/src/main/lib/ipc/sessionStartTelemetry.ts:120-137.",
    "comfy.desktop.session.started": "Exact deferred payload at projects/comfy/Comfy-Desktop/src/main/lib/telemetry.ts:516-520 is {app_env, app_version, is_packaged}; identify-time defaults add installation_id and platform/architecture context before capture.",
    "comfy.desktop.session.system_info": "Exact construction at projects/comfy/Comfy-Desktop/src/renderer/src/lib/rendererBootstrap.ts:450-473 spreads scalar system-info fields and adds {gpu_vram_mb, gpu_vram_gb, gpu_count, gpu_driver_version, gpu_tier, gpus_json, gpus_json_truncated, installations_json, installations_json_truncated}.",
    "comfy.desktop.snapshot.restore_comfyui_version": "Exact restoreContext at projects/comfy/Comfy-Desktop/src/main/lib/standaloneMigration.ts:157 is {installation_id}.",
    "comfy.desktop.snapshot.restore_custom_nodes": "Exact restoreContext at projects/comfy/Comfy-Desktop/src/main/lib/standaloneMigration.ts:157 is {installation_id}.",
    "comfy.desktop.snapshot.restore_pip_packages": "Exact restoreContext at projects/comfy/Comfy-Desktop/src/main/lib/standaloneMigration.ts:157 is {installation_id}.",
}


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def stable_id(event_name: str) -> str:
    suffix = hashlib.sha256(event_name.casefold().encode("utf-8")).hexdigest()[:12].upper()
    return f"COMFY-DESKTOP-TELEMETRY-{suffix}"


def call_before_literal(text: str, offset: int) -> str | None:
    prefix = text[max(0, offset - 180):offset]
    match = re.search(
        r"((?:[A-Za-z_$][A-Za-z0-9_$]*\.)*[A-Za-z_$][A-Za-z0-9_$]*)\s*\(\s*$",
        prefix,
        re.DOTALL,
    )
    return match.group(1) if match else None


def balanced_expression(text: str, offset: int) -> str:
    while offset < len(text) and text[offset].isspace():
        offset += 1
    if offset >= len(text):
        return ""
    if text[offset] not in "{[(":
        end = offset
        while end < len(text) and text[end] not in ",)\n":
            end += 1
        return re.sub(r"\s+", " ", text[offset:end]).strip()

    opening = text[offset]
    closing = {"{": "}", "[": "]", "(": ")"}[opening]
    depth = 0
    quote: str | None = None
    escaped = False
    line_comment = False
    block_comment = False
    end = offset
    while end < len(text):
        character = text[end]
        next_character = text[end + 1] if end + 1 < len(text) else ""
        if line_comment:
            if character == "\n":
                line_comment = False
        elif block_comment:
            if character == "*" and next_character == "/":
                block_comment = False
                end += 1
        elif quote is not None:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
        elif character == "/" and next_character == "/":
            line_comment = True
            end += 1
        elif character == "/" and next_character == "*":
            block_comment = True
            end += 1
        elif character in {"'", '"', chr(96)}:
            quote = character
        elif character == opening:
            depth += 1
        elif character == closing:
            depth -= 1
            if depth == 0:
                end += 1
                break
        end += 1
    return re.sub(r"\s+", " ", text[offset:end]).strip()


def payload_after_literal(text: str, offset: int) -> str:
    while offset < len(text) and text[offset].isspace():
        offset += 1
    if offset >= len(text) or text[offset] != ",":
        return ""
    return balanced_expression(text, offset + 1)


def availability(event_name: str, event_kind: str) -> str:
    if event_kind in {"telemetry namespace guard", "person-property transport"}:
        return "infrastructure-only"
    if event_name == "app:relaunch":
        return "active"
    if event_name == "comfy.desktop.session.installation_started":
        return "deprecated/dead"
    if event_name == "app:user_logged_in" or any(
        token in event_name
        for token in (
            ".auth.",
            ".billing.",
            ".cloud.",
            ".first_use.cloud_",
            ".first_use.why_cloud_",
        )
    ):
        return "cloud/paid"
    if ".experiment." in event_name:
        return "experimental"
    if ".app_update.startup_install" in event_name:
        return "platform-specific"
    if event_name.endswith(".error") or event_name.endswith(".failed") or any(
        token in event_name
        for token in (
            ".adopt.",
            ".migrate.",
            ".recovery.",
            ".torch_repair.",
            ".pygit2.",
            ".identity.",
            ".manager.",
        )
    ):
        return "conditional"
    return "active"


def scan() -> dict[str, list[dict[str, str]]]:
    occurrences: dict[str, list[dict[str, str]]] = defaultdict(list)
    source_paths = sorted(
        path
        for path in (DESKTOP / "src").rglob("*")
        if path.is_file()
        and path.suffix in {".ts", ".tsx", ".vue", ".js", ".mjs"}
        and ".test." not in path.name
        and not path.name.endswith(".spec.ts")
        and not any(part in {"test", "tests", "__tests__"} for part in path.parts)
    )
    for path in source_paths:
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in LITERAL_RE.finditer(text):
            line_start = text.rfind("\n", 0, match.start()) + 1
            line_end = text.find("\n", match.end())
            if line_end < 0:
                line_end = len(text)
            source_line = text[line_start:line_end].strip()
            if source_line.startswith(("*", "//", "/*", "<!--")):
                continue
            line_number = text.count("\n", 0, match.start()) + 1
            call = call_before_literal(text, match.start())
            call_leaf = call.rsplit(".", 1)[-1] if call else ""
            payload = payload_after_literal(text, match.end()) if call_leaf in EMITTERS else ""
            occurrences[match.group(2)].append(
                {
                    "source": f"{relative(path)}:{line_number}",
                    "call": call or "literal-reference",
                    "payload": payload,
                    "source_line": re.sub(r"\s+", " ", source_line),
                }
            )
    return occurrences


def build_rows() -> list[dict[str, str]]:
    occurrences = scan()
    mirrored_source = "projects/comfy/Comfy-Desktop/src/shared/datadogMirroredEvents.ts"
    rows: list[dict[str, str]] = []
    for event_name in sorted(occurrences):
        sites = occurrences[event_name]
        call_leaves = {site["call"].rsplit(".", 1)[-1] for site in sites}
        tracked_step = "trackedStep" in call_leaves
        base_name = event_name[:-6] if event_name.endswith(".error") else ""
        derived_from_tracked_step = bool(
            base_name
            and base_name in occurrences
            and any(
                site["call"].rsplit(".", 1)[-1] == "trackedStep"
                for site in occurrences[base_name]
            )
        )

        if event_name == "app:relaunch":
            event_kind = "Electron IPC lifecycle channel"
            classification = "native IPC event contract"
            payload_evidence = (
                "request=none; response=void; main handler cancels IPC operations, destroys all Comfy windows and tray, "
                "then calls app.relaunch() and app.quit()"
            )
            consent = "Not telemetry; no consent gate. Renderer access is limited to the context-isolated preload member relaunchApp()."
            redaction = "No payload crosses the channel."
            rate_limit = "Not applicable."
            provider = "Electron ipcRenderer.invoke -> ipcMain.handle; no analytics provider side effect is implied by this channel."
        elif event_name == "comfy.desktop.telemetry.":
            event_kind = "telemetry namespace guard"
            classification = "telemetry infrastructure prefix; not emitted as an event"
            payload_evidence = "No payload; exact use is event.startsWith the namespace in _bypassRateLimit()."
            consent = "Not emitted; evaluated only after an otherwise consent-allowed capture reaches the volume guard."
            redaction = "Not applicable to the prefix literal."
            rate_limit = "The prefix exempts telemetry self-events from the 60-per-60-second per-name guard; the 5,000-event process cap is checked first."
            provider = "No provider emission for the prefix itself."
        else:
            event_kind = (
                "tracked-step base deriving .start/.end/.error wire events"
                if tracked_step
                else "derived tracked-step failure telemetry event"
                if derived_from_tracked_step
                else "person-property transport"
                if event_name == "comfy.desktop.person.set"
                else "product telemetry event"
            )
            classification = (
                "telemetry identity infrastructure"
                if event_name == "comfy.desktop.person.set"
                else "tracked-step telemetry contract"
                if tracked_step or derived_from_tracked_step
                else "telemetry volume-guard infrastructure"
                if event_name in {
                    "comfy.desktop.telemetry.rate_limited",
                    "comfy.desktop.telemetry.session_cap_hit",
                }
                else "analytics event contract"
            )
            payload_sites: list[str] = []
            for site in sites:
                call_leaf = site["call"].rsplit(".", 1)[-1]
                if call_leaf not in EMITTERS:
                    continue
                payload = site["payload"] or "{} / no explicit properties"
                payload_sites.append(f"{site['source']} {site['call']} second-argument={payload}")
            if tracked_step:
                payload_sites.append(
                    "projects/comfy/Comfy-Desktop/src/main/lib/telemetry.ts:1086-1112 trackedStep wire expansion: "
                    ".start=context; .end={...context,duration_ms}; "
                    ".error={...context,duration_ms,error_bucket,error_message}, with error_message scrubbed then sliced to 500 characters"
                )
            if derived_from_tracked_step:
                base_sites = [
                    site
                    for site in occurrences[base_name]
                    if site["call"].rsplit(".", 1)[-1] == "trackedStep"
                ]
                payload_sites.extend(
                    f"derived from {site['source']} {base_name} context={site['payload'] or '{}'}"
                    for site in base_sites
                )
                payload_sites.append(
                    "projects/comfy/Comfy-Desktop/src/main/lib/telemetry.ts:1104-1109 adds duration_ms, error_bucket, and scrubbed error_message sliced to 500 characters"
                )
            payload_evidence = "; ".join(dict.fromkeys(payload_sites)) or (
                "Literal is a provider allow-list/dedup reference; no independent payload is constructed at this source site."
            )
            if event_name in PAYLOAD_SUPPLEMENTS:
                payload_evidence += "; " + PAYLOAD_SUPPLEMENTS[event_name]
            consent = (
                "Explicit pre-consent allow-list: this event may fire while consent is granted, denied, or undecided so a decline is not dropped."
                if event_name == "comfy.desktop.first_use.consent_decision"
                else "Requires packaged emission, initialized PostHog client, distinct ID, and granted telemetry consent; denied or undecided states drop it before provider capture."
            )
            redaction = (
                "Every main-process string/string-array property is passed through scrubAll before PostHog capture. Renderer events are scrubbed before IPC; the bridge accepts at most 128 keys/array items, clamps ordinary strings to 2,048 characters, and permits only five named serialized-JSON fields up to 768 KiB."
            )
            rate_limit = (
                "Per-name limit bypassed because .error and telemetry-self events bypass layer 1; the 5,000-event per-process cap still applies."
                if event_name.endswith(".error") or event_name.startswith("comfy.desktop.telemetry.")
                else "At most 60 captures per event name per rolling 60 seconds and 5,000 captured events per process; later events are dropped and a bounded warning is attempted."
            )
            mirrored = any(site["source"].split(":", 1)[0] == mirrored_source for site in sites)
            provider = (
                "PostHog plus Datadog RUM action mirror because the exact name is in DATADOG_MIRRORED_EVENT_NAMES; main-origin events relay with mainAlreadyCaptured=true to prevent PostHog duplication."
                if mirrored
                else "PostHog only; Datadog RUM rejects names absent from DATADOG_MIRRORED_EVENT_NAMES."
            )

        row_availability = availability(event_name, event_kind)
        if event_name in {
            "comfy.desktop.telemetry.rate_limited",
            "comfy.desktop.telemetry.session_cap_hit",
        }:
            row_availability = "infrastructure-only"
        source_evidence = "; ".join(
            f"{site['source']} [{site['call']}; {site['source_line']}]" for site in sites
        )
        if derived_from_tracked_step:
            source_evidence += "; " + "; ".join(
                f"{site['source']} [trackedStep base {base_name}]"
                for site in occurrences[base_name]
                if site["call"].rsplit(".", 1)[-1] == "trackedStep"
            )
        rows.append(
            {
                "feature_id": stable_id(event_name),
                "event_name": event_name,
                "event_kind": event_kind,
                "classification": classification,
                "availability": row_availability,
                "evidence_level": "code-inferred",
                "source_evidence": source_evidence,
                "payload_evidence": payload_evidence,
                "consent_behavior": consent,
                "redaction_validation": redaction,
                "rate_limit_dedup": rate_limit,
                "provider_side_effects": provider,
                "derived_wire_names": (
                    f"{event_name}.start; {event_name}.end; {event_name}.error"
                    if tracked_step
                    else event_name
                ),
                "notes": "Static production-source inventory; source dependencies were unavailable, so no provider emission was performed.",
            }
        )
    return rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    arguments = parser.parse_args()
    rows = build_rows()
    with arguments.output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=FIELDS, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
