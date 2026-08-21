#!/usr/bin/env python3
"""Validate frozen CLI and companion-client contracts against Buzz sources."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs, quote, urlparse


FIXTURE_ROOT = Path(__file__).resolve().parent
REPOSITORY_ROOT = FIXTURE_ROOT.parents[4]
MANIFEST_PATH = FIXTURE_ROOT / "manifest.json"
REQUIRED_FIELDS = {
    "id",
    "category",
    "client",
    "client_version",
    "input",
    "expected_output",
    "authority",
}
REQUIRED_CATEGORIES = {
    "cli_command",
    "cli_error",
    "route",
    "deep_link",
    "lifecycle",
    "negotiation",
}


class ContractError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def load_manifest() -> dict[str, Any]:
    try:
        value = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ContractError(f"manifest_invalid:{error}") from error
    require(isinstance(value, dict), "manifest_not_object")
    return value


def source(path: str) -> str:
    source_path = REPOSITORY_ROOT / path
    try:
        return source_path.read_text(encoding="utf-8")
    except OSError as error:
        raise ContractError(f"source_unreadable:{path}:{error}") from error


def contract_by_id(manifest: dict[str, Any], contract_id: str) -> dict[str, Any]:
    matches = [contract for contract in manifest["contracts"] if contract["id"] == contract_id]
    require(len(matches) == 1, f"contract_missing_or_duplicate:{contract_id}")
    return matches[0]


def validate_shape(manifest: dict[str, Any]) -> None:
    require(manifest.get("schema_version") == 1, "unsupported_schema_version")
    clients = manifest.get("clients")
    contracts = manifest.get("contracts")
    require(isinstance(clients, dict) and clients, "clients_missing")
    require(isinstance(contracts, list) and contracts, "contracts_missing")
    contract_ids: list[str] = []
    categories: set[str] = set()
    for contract in contracts:
        require(isinstance(contract, dict), "contract_not_object")
        missing = REQUIRED_FIELDS - set(contract)
        require(not missing, f"{contract.get('id', '<unknown>')}:missing_fields:{sorted(missing)}")
        contract_id = contract["id"]
        require(
            isinstance(contract_id, str) and re.fullmatch(r"CLIENT-[A-Z]+-[0-9]{3}", contract_id) is not None,
            f"invalid_contract_id:{contract_id}",
        )
        contract_ids.append(contract_id)
        categories.add(contract["category"])
        client = contract["client"]
        require(client in clients, f"{contract_id}:unknown_client:{client}")
        require(
            contract["client_version"] == clients[client].get("version"),
            f"{contract_id}:version_mismatch",
        )
        require(isinstance(contract["input"], dict) and contract["input"], f"{contract_id}:input_missing")
        require(
            isinstance(contract["expected_output"], dict) and contract["expected_output"],
            f"{contract_id}:expected_output_missing",
        )
        authorities = contract["authority"]
        require(isinstance(authorities, list) and authorities, f"{contract_id}:authority_missing")
        for authority in authorities:
            require(
                isinstance(authority, str) and (REPOSITORY_ROOT / authority).is_file(),
                f"{contract_id}:authority_not_found:{authority}",
            )
    require(len(contract_ids) == len(set(contract_ids)), "duplicate_contract_id")
    require(REQUIRED_CATEGORIES <= categories, f"missing_categories:{sorted(REQUIRED_CATEGORIES - categories)}")


def parse_cargo_workspace_version(text: str) -> str:
    workspace_package = text.split("[workspace.package]", 1)[1]
    match = re.search(r'^version\s*=\s*"([^"]+)"', workspace_package, re.MULTILINE)
    require(match is not None, "cargo_workspace_version_missing")
    return match.group(1)


def parse_pubspec_version(text: str) -> str:
    match = re.search(r"^version:\s*(\S+)", text, re.MULTILINE)
    require(match is not None, "pubspec_version_missing")
    return match.group(1)


def validate_versions(manifest: dict[str, Any]) -> None:
    clients = manifest["clients"]
    actual = {
        "buzz-cli": parse_cargo_workspace_version(source("projects/buzz/Cargo.toml")),
        "buzz-mobile": parse_pubspec_version(source("projects/buzz/mobile/pubspec.yaml")),
        "buzz-web": json.loads(source("projects/buzz/web/package.json"))["version"],
        "buzz-admin-web": json.loads(source("projects/buzz/admin-web/package.json"))["version"],
    }
    for client, version in actual.items():
        require(clients[client]["version"] == version, f"{client}:version_drift:{version}")
        require(
            (REPOSITORY_ROOT / clients[client]["version_authority"]).is_file(),
            f"{client}:version_authority_missing",
        )


def validate_cli(manifest: dict[str, Any]) -> None:
    cli_source = source("projects/buzz/crates/buzz-cli/src/lib.rs")
    error_source = source("projects/buzz/crates/buzz-cli/src/error.rs")
    inventory_match = re.search(
        r"let expected_groups: Vec<&str> = vec!\[(.*?)\];",
        cli_source,
        re.DOTALL,
    )
    require(inventory_match is not None, "cli_inventory_source_missing")
    actual_groups = re.findall(r'"([a-z-]+)"', inventory_match.group(1))
    expected_groups = contract_by_id(manifest, "CLIENT-CLI-002")["expected_output"]["groups"]
    require(actual_groups == expected_groups, f"cli_group_drift:{actual_groups}")

    missing_key = contract_by_id(manifest, "CLIENT-CLI-003")["expected_output"]
    missing_message = missing_key["json"]["message"]
    require(
        missing_message.removeprefix("auth error: ") in cli_source
        and '#[error("auth error: {0}")]' in error_source,
        "cli_missing_key_message_drift",
    )
    require(missing_key["exit_code"] == 3, "cli_missing_key_exit_drift")
    for token in (
        "CliError::Usage(_) => 1",
        "CliError::Network(_) => 2",
        "CliError::Auth(_) => 3",
        "CliError::Key(_) => 3",
        "CliError::Conflict(_) => 5",
        "CliError::NotFound(_) => 1",
        "CliError::DeliveryUnknown(_) => 2",
        "CliError::Other(_) => 4",
        '"retryable": is_retryable_error(e)',
        "429 | 502 | 503 | 504",
    ):
        require(token in error_source, f"cli_error_contract_drift:{token}")
    help_contract = contract_by_id(manifest, "CLIENT-CLI-001")["expected_output"]
    require(help_contract["exit_code"] == 0 and "Exit codes:" in cli_source, "cli_help_contract_drift")
    require("if let Cmd::Pack(ref sub) = cli.command" in cli_source, "cli_local_pack_boundary_drift")

    link_contract = contract_by_id(manifest, "CLIENT-CLI-006")
    link_input = link_contract["input"]
    expected = link_contract["expected_output"]
    owner = link_input["owner"]
    event_id = link_input["event_id"]
    repo_id = link_input["repo_id"]
    actual_links = {
        "repo": f"buzz://repo?owner={owner}&d={repo_id}",
        "pr": f"buzz://pr?id={event_id}&owner={owner}&d={repo_id}",
        "issue": f"buzz://issue?id={event_id}&owner={owner}&d={repo_id}",
    }
    require(actual_links == expected, "cli_entity_link_fixture_drift")
    links_source = source("projects/buzz/crates/buzz-cli/src/links.rs")
    for prefix in ("buzz://repo?owner=", "buzz://pr?id=", "buzz://issue?id="):
        require(prefix in links_source, f"cli_entity_link_source_drift:{prefix}")


def validate_web(manifest: dict[str, Any]) -> None:
    routes_source = source("projects/buzz/web/src/app/routes.ts")
    actual_routes = ["/"] + re.findall(r'route\("([^"]+)"', routes_source)
    expected_routes = contract_by_id(manifest, "CLIENT-WEB-001")["expected_output"]["routes"]
    require(actual_routes == expected_routes, f"web_route_drift:{actual_routes}")
    invite_source = source("projects/buzz/web/src/features/invite/ui/InvitePage.tsx")
    invite_api = source("projects/buzz/web/src/features/invite/invite-api.ts")
    for token in (
        "policy_version: policy.version",
        "age_confirmed: ageConfirmed",
        "window.location.href = `buzz://join?${query.toString()}`",
        "window.location.assign(\"/\")",
    ):
        require(token in invite_source, f"web_invite_contract_drift:{token}")
    for token in ("/api/invites/claim", "requireNip07: true", "INVITE_REQUEST_TIMEOUT_MS = 15_000"):
        require(token in invite_api, f"web_invite_api_drift:{token}")
    handoff = contract_by_id(manifest, "CLIENT-WEB-003")
    handoff_input = handoff["input"]
    query = (
        f"relay={quote(handoff_input['relay'], safe='')}"
        f"&code={quote(handoff_input['code'], safe='')}"
        f"&policy_receipt={quote(handoff_input['policy_receipt'], safe='')}"
    )
    require(handoff["expected_output"]["app_handoff"] == f"buzz://join?{query}", "web_handoff_drift")


def validate_admin(manifest: dict[str, Any]) -> None:
    app_source = source("projects/buzz/admin-web/src/App.tsx")
    api_source = source("projects/buzz/admin-web/src/api.ts")
    expected = contract_by_id(manifest, "CLIENT-ADMIN-001")["expected_output"]
    require(expected["routes"] == ["/reports", "/reports/:id", "/feedback", "/feedback/:id"], "admin_route_fixture_drift")
    for token in (
        r"^\/reports\/([^/]+)$",
        r"^\/feedback\/([^/]+)$",
        'path === "/feedback"',
        'const FEEDBACK_STATUS_KEY = "buzz-admin-feedback-status"',
        "Message content is unavailable. It may have expired or",
    ):
        require(token in app_source, f"admin_contract_drift:{token}")
    require('const PREFIX = "/api/admin/v1"' in api_source, "admin_api_prefix_drift")
    require('credentials: "same-origin"' in api_source, "admin_credentials_drift")


def validate_mobile(manifest: dict[str, Any]) -> None:
    deep_link_source = source("projects/buzz/mobile/lib/shared/deeplink/deep_link.dart")
    dispatcher_source = source("projects/buzz/mobile/lib/features/channels/deep_link_dispatcher.dart")
    session_source = source("projects/buzz/mobile/lib/shared/relay/relay_session.dart")
    pairing_source = source("projects/buzz/mobile/lib/features/pairing/pairing_crypto.dart")
    closed_source = source("projects/buzz/mobile/lib/shared/relay/relay_closed_policy.dart")

    message_contract = contract_by_id(manifest, "CLIENT-MOBILE-001")
    parsed = urlparse(message_contract["input"]["uri"])
    params = parse_qs(parsed.query)
    actual_message = {
        "type": "message",
        "channel_id": params["channel"][0],
        "message_id": params["id"][0],
        "thread_root_id": params["thread"][0],
    }
    require(parsed.scheme == "buzz" and parsed.netloc == "message", "mobile_message_uri_drift")
    require(actual_message == message_contract["expected_output"], "mobile_message_fixture_drift")

    invite_contract = contract_by_id(manifest, "CLIENT-MOBILE-002")
    invite = urlparse(invite_contract["input"]["uri"])
    actual_invite = {
        "type": "invite",
        "relay_url": f"wss://{invite.netloc}",
        "code": invite.path.removeprefix("/invite/"),
        "policy_receipt": None,
    }
    require(actual_invite == invite_contract["expected_output"], "mobile_invite_fixture_drift")
    for token in (
        "uri.scheme != 'buzz' || uri.host != 'message'",
        "relayUri.scheme != 'ws' && relayUri.scheme != 'wss'",
        "validateInviteRelayUri(relayUri)",
    ):
        require(token in deep_link_source, f"mobile_deep_link_drift:{token}")
    for token in (
        "Channels not loaded yet — keep the link parked",
        "Channel not found in this workspace",
        "Could not open this invite. Re-open the invite link to try again.",
    ):
        require(token in dispatcher_source, f"mobile_dispatch_drift:{token}")
    for token in (
        "_backgroundGraceDuration = Duration(seconds: 5)",
        "_baseReconnectDelayMs = 1000",
        "_cancelAllHistory(Exception('App moved to background'))",
        "_rejectAllPending(Exception('App moved to background'))",
    ):
        require(token in session_source, f"mobile_lifecycle_drift:{token}")
    for token in (
        "version ??= 1",
        "if (version != 1)",
        "Unsupported protocol version $version. Please update the app.",
    ):
        require(token in pairing_source, f"mobile_pairing_drift:{token}")
    for token in ("unsupported:", "RelayClosedClass.terminal", "rate-limited:"):
        require(token in closed_source, f"mobile_closed_policy_drift:{token}")
    startup_negotiation = contract_by_id(manifest, "CLIENT-MOBILE-013")["expected_output"]
    require(startup_negotiation["global_capability_endpoint"] == "absent", "mobile_negotiation_gap_drift")
    require(
        "/api/client-capabilities" not in session_source
        and "/api/capabilities" not in session_source,
        "mobile_global_negotiation_added_update_fixture",
    )


def main() -> int:
    manifest = load_manifest()
    validate_shape(manifest)
    validate_versions(manifest)
    validate_cli(manifest)
    validate_web(manifest)
    validate_admin(manifest)
    validate_mobile(manifest)
    counts: dict[str, int] = {}
    for contract in manifest["contracts"]:
        counts[contract["client"]] = counts.get(contract["client"], 0) + 1
    print(
        "Client contract check passed: "
        + " ".join(f"{client}={count}" for client, count in sorted(counts.items()))
        + f" total={len(manifest['contracts'])}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ContractError as error:
        print(f"Client contract check failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
