#!/usr/bin/env python3
"""Independently validate the frozen Buzz protocol compatibility corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any


FIXTURE_ROOT = Path(__file__).resolve().parent
FIELD_NAMES = {"id", "pubkey", "created_at", "kind", "tags", "content", "sig"}
HEX_DIGITS = frozenset("0123456789abcdef")
FIELD_PRIME = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
CURVE_ORDER = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
GENERATOR = (
    0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798,
    0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8,
)
AUTHOR_ONLY_KINDS = {30179, 30300, 30350}
P_GATED_KINDS = {1059, 24200, 30622, 44100, 44101, 44200}
SHARED_GATED_KINDS = {30175, 30178}


class FixtureError(Exception):
    pass


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise FixtureError(f"invalid_json:{path.name}:{error}") from error


def file_sha256(path: Path) -> str:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as error:
        raise FixtureError(f"unreadable_fixture:{path.name}:{error}") from error


def is_lower_hex(value: Any, length: int) -> bool:
    return (
        isinstance(value, str)
        and len(value) == length
        and all(character in HEX_DIGITS for character in value)
    )


def inverse(value: int) -> int:
    return pow(value, FIELD_PRIME - 2, FIELD_PRIME)


def point_add(
    left: tuple[int, int] | None, right: tuple[int, int] | None
) -> tuple[int, int] | None:
    if left is None:
        return right
    if right is None:
        return left
    left_x, left_y = left
    right_x, right_y = right
    if left_x == right_x and (left_y != right_y or left_y == 0):
        return None
    if left == right:
        slope = (3 * left_x * left_x) * inverse(2 * left_y) % FIELD_PRIME
    else:
        slope = (right_y - left_y) * inverse(right_x - left_x) % FIELD_PRIME
    output_x = (slope * slope - left_x - right_x) % FIELD_PRIME
    output_y = (slope * (left_x - output_x) - left_y) % FIELD_PRIME
    return output_x, output_y


def point_multiply(scalar: int, point: tuple[int, int]) -> tuple[int, int] | None:
    result = None
    addend: tuple[int, int] | None = point
    while scalar:
        if scalar & 1:
            result = point_add(result, addend)
        addend = point_add(addend, addend)
        scalar >>= 1
    return result


def lift_x(x_coordinate: int) -> tuple[int, int] | None:
    if x_coordinate >= FIELD_PRIME:
        return None
    candidate = (pow(x_coordinate, 3, FIELD_PRIME) + 7) % FIELD_PRIME
    y_coordinate = pow(candidate, (FIELD_PRIME + 1) // 4, FIELD_PRIME)
    if pow(y_coordinate, 2, FIELD_PRIME) != candidate:
        return None
    if y_coordinate & 1:
        y_coordinate = FIELD_PRIME - y_coordinate
    return x_coordinate, y_coordinate


def tagged_hash(tag: str, payload: bytes) -> bytes:
    tag_hash = hashlib.sha256(tag.encode("ascii")).digest()
    return hashlib.sha256(tag_hash + tag_hash + payload).digest()


def verify_schnorr(pubkey_hex: str, message: bytes, signature_hex: str) -> bool:
    public_point = lift_x(int(pubkey_hex, 16))
    if public_point is None:
        return False
    signature = bytes.fromhex(signature_hex)
    r_coordinate = int.from_bytes(signature[:32], "big")
    signature_scalar = int.from_bytes(signature[32:], "big")
    if r_coordinate >= FIELD_PRIME or signature_scalar >= CURVE_ORDER:
        return False
    challenge = int.from_bytes(
        tagged_hash(
            "BIP0340/challenge",
            signature[:32] + bytes.fromhex(pubkey_hex) + message,
        ),
        "big",
    ) % CURVE_ORDER
    challenge_point = point_multiply(CURVE_ORDER - challenge, public_point)
    result = point_add(point_multiply(signature_scalar, GENERATOR), challenge_point)
    return result is not None and result[1] % 2 == 0 and result[0] == r_coordinate


def canonical_event_bytes(event: dict[str, Any]) -> bytes:
    payload = [
        0,
        event["pubkey"],
        event["created_at"],
        event["kind"],
        event["tags"],
        event["content"],
    ]
    return json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def validate_event(event: Any) -> str | None:
    if not isinstance(event, dict) or set(event) != FIELD_NAMES:
        return "invalid_shape"
    if not is_lower_hex(event["pubkey"], 64):
        return "invalid_pubkey"
    if not isinstance(event["created_at"], int) or isinstance(event["created_at"], bool):
        return "invalid_created_at"
    if event["created_at"] < 0 or event["created_at"] > 0xFFFFFFFFFFFFFFFF:
        return "invalid_created_at"
    if not isinstance(event["kind"], int) or isinstance(event["kind"], bool):
        return "invalid_kind"
    if event["kind"] < 0 or event["kind"] > 0xFFFF:
        return "invalid_kind"
    if not isinstance(event["tags"], list) or any(
        not isinstance(tag, list) or any(not isinstance(part, str) for part in tag)
        for tag in event["tags"]
    ):
        return "invalid_tags"
    if not isinstance(event["content"], str):
        return "invalid_content"
    if not is_lower_hex(event["id"], 64):
        return "invalid_id"
    event_bytes = canonical_event_bytes(event)
    event_id = hashlib.sha256(event_bytes).hexdigest()
    if event["id"] != event_id:
        return "invalid_id"
    if lift_x(int(event["pubkey"], 16)) is None:
        return "invalid_pubkey"
    if not is_lower_hex(event["sig"], 128):
        return "invalid_signature"
    if not verify_schnorr(event["pubkey"], bytes.fromhex(event_id), event["sig"]):
        return "invalid_signature"
    return None


def tag_values(event: dict[str, Any], name: str) -> list[str]:
    return [tag[1] for tag in event["tags"] if len(tag) >= 2 and tag[0] == name]


def event_visible(event: dict[str, Any], reader: str) -> bool:
    if event["kind"] in AUTHOR_ONLY_KINDS and reader != event["pubkey"]:
        return False
    if event["kind"] in SHARED_GATED_KINDS:
        shared = [tag for tag in event["tags"] if tag and tag[0] == "shared"]
        if reader != event["pubkey"] and shared != [["shared", "true"]]:
            return False
    if event["kind"] in P_GATED_KINDS and reader not in tag_values(event, "p"):
        return False
    return True


def relay_trace_error(path: Path) -> str | None:
    try:
        steps = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]
    except (OSError, json.JSONDecodeError):
        return "invalid_json"
    if not steps:
        return "coverage_breach"
    first_state = steps[0].get("state_after")
    if not isinstance(first_state, dict):
        return "invalid_state"
    resolved = first_state.get("resolved_community")
    channel_communities = {
        "cafe0000-0000-0000-0000-000000000010": "aaaa0000-0000-0000-0000-000000000001",
        "dead0000-0000-0000-0000-000000000020": "bbbb0000-0000-0000-0000-000000000002",
    }
    for step in steps:
        if step.get("schema_version") != 1:
            return "schema_version"
        if step.get("state_after") != first_state:
            return "state_mismatch"
        action = step.get("action")
        if not isinstance(action, dict) or not isinstance(action.get("type"), str):
            return "invalid_action"
        action_type = action["type"]
        if action_type == "impl_bug":
            return "coverage_breach"
        channel = action.get("channel")
        claimed = action.get("claimed_community")
        channel_community = channel_communities.get(channel)
        if action_type == "auth_check" and action.get("verdict") == "allow":
            if channel_community != resolved or claimed != resolved:
                return "illegal_transition"
        if action_type in {"write_insert", "write_duplicate"}:
            if channel_community != resolved or claimed != resolved:
                return "illegal_transition"
        if action_type in {"read_message_rows", "read_by_id_rows", "read_host_feed_rows"}:
            if any(community != resolved for community in action.get("row_communities", [])):
                return "non_interference"
    return None


def require(condition: bool, message: str) -> None:
    if not condition:
        raise FixtureError(message)


def check_event_cases(manifest: dict[str, Any], events: dict[str, Any]) -> int:
    for case in manifest["event_cases"]:
        event = events.get(case["event"])
        require(event is not None, f"{case['id']}:missing_event")
        actual_error = validate_event(event)
        expected_error = case.get("expected_error")
        expected_result = "reject" if expected_error is not None else "accept"
        require(case.get("expected") == expected_result, f"{case['id']}:inconsistent_expectation")
        require(actual_error == expected_error, f"{case['id']}:expected={expected_error}:actual={actual_error}")
    return len(manifest["event_cases"])


def check_replaceable_cases(manifest: dict[str, Any], events: dict[str, Any]) -> int:
    for case in manifest["replaceable_cases"]:
        candidates = [events[event_id] for event_id in case["events"]]
        require(all(validate_event(event) is None for event in candidates), f"{case['id']}:invalid_candidate")
        winner = min(candidates, key=lambda event: (-event["created_at"], event["id"]))
        require(winner["id"] == events[case["winner"]]["id"], f"{case['id']}:wrong_winner")
    return len(manifest["replaceable_cases"])


def check_privacy_cases(manifest: dict[str, Any], events: dict[str, Any]) -> int:
    for case in manifest["privacy_cases"]:
        event = events[case["event"]]
        require(validate_event(event) is None, f"{case['id']}:invalid_event")
        actual = event_visible(event, case["reader"])
        require(actual is case["visible"], f"{case['id']}:expected_visible={case['visible']}:actual={actual}")
    return len(manifest["privacy_cases"])


def check_mixed_version_cases(manifest: dict[str, Any], events: dict[str, Any]) -> int:
    for case in manifest["mixed_version_cases"]:
        kinds = []
        for event_name in case["events"]:
            event = events[event_name]
            require(validate_event(event) is None, f"{case['id']}:{event_name}:invalid_event")
            kinds.append(event["kind"])
        require(kinds == case["kinds"], f"{case['id']}:expected_kinds={case['kinds']}:actual={kinds}")
        require(kinds == [9, 40002], f"{case['id']}:missing_legacy_or_v2")
    return len(manifest["mixed_version_cases"])


def check_wire_traces(document: dict[str, Any], events: dict[str, Any]) -> int:
    require(document.get("schema_version") == 1, "unsupported_wire_trace_version")
    for trace in document.get("traces", []):
        trace_id = trace.get("id")
        reader = trace.get("authenticated_pubkey")
        require(is_lower_hex(reader, 64), f"{trace_id}:invalid_authenticated_pubkey")
        pending: tuple[str, str, str | None] | None = None
        for frame in trace.get("frames", []):
            direction = frame.get("direction")
            message = frame.get("message")
            require(isinstance(message, list) and message, f"{trace_id}:invalid_frame")
            if direction == "client_to_relay" and message[0] == "EVENT":
                require(len(message) == 2 and isinstance(message[1], dict), f"{trace_id}:invalid_event_frame")
                event_name = message[1].get("$event")
                require(event_name in events, f"{trace_id}:unknown_event")
                error = validate_event(events[event_name])
                pending = ("OK", event_name, error)
            elif direction == "client_to_relay" and message[0] == "REQ":
                require(len(message) == 3 and isinstance(message[2], dict), f"{trace_id}:invalid_req_frame")
                subscription_id = message[1]
                event_filter = message[2]
                kinds = event_filter.get("kinds", [])
                reason = None
                if kinds and all(kind in AUTHOR_ONLY_KINDS for kind in kinds):
                    if event_filter.get("authors") != [reader]:
                        reason = "restricted: author-only kinds require authors=[self]"
                elif any(kind in P_GATED_KINDS for kind in kinds):
                    if event_filter.get("#p") != [reader]:
                        reason = "restricted: p-gated events require #p matching your pubkey"
                require(reason is not None, f"{trace_id}:req_did_not_fail_closed")
                pending = ("CLOSED", subscription_id, reason)
            elif direction == "relay_to_client" and message[0] == "OK":
                require(pending is not None and pending[0] == "OK", f"{trace_id}:unexpected_ok")
                _, event_name, error = pending
                require(len(message) == 4, f"{trace_id}:invalid_ok_frame")
                require(message[1] == {"$event_id": event_name}, f"{trace_id}:wrong_ok_event")
                require(message[2] is (error is None), f"{trace_id}:wrong_ok_verdict")
                event = events[event_name]
                computed_id = hashlib.sha256(canonical_event_bytes(event)).hexdigest()
                expected_message = "" if error is None else {
                    "invalid_id": f"invalid: invalid event id: computed {computed_id}, got {event['id']}",
                    "invalid_signature": "invalid: invalid schnorr signature",
                }.get(error)
                require(expected_message is not None and message[3] == expected_message, f"{trace_id}:wrong_ok_message")
                pending = None
            elif direction == "relay_to_client" and message[0] == "CLOSED":
                require(pending is not None and pending[0] == "CLOSED", f"{trace_id}:unexpected_closed")
                require(len(message) == 3, f"{trace_id}:invalid_closed_frame")
                require(message[1] == pending[1] and message[2] == pending[2], f"{trace_id}:wrong_closed_frame")
                pending = None
            else:
                raise FixtureError(f"{trace_id}:unsupported_frame")
        require(pending is None, f"{trace_id}:missing_relay_response")
    return len(document.get("traces", []))


def check_relay_cases(manifest: dict[str, Any]) -> int:
    for case in manifest["relay_trace_cases"]:
        path = FIXTURE_ROOT / case["file"]
        require(file_sha256(path) == case["sha256"], f"{case['id']}:hash_mismatch")
        actual_error = relay_trace_error(path)
        expected_error = case.get("expected_error")
        expected_result = "reject" if expected_error is not None else "accept"
        require(case.get("expected") == expected_result, f"{case['id']}:inconsistent_expectation")
        require(actual_error == expected_error, f"{case['id']}:expected={expected_error}:actual={actual_error}")
    return len(manifest["relay_trace_cases"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", help="validate one stable case ID")
    arguments = parser.parse_args()
    manifest = load_json(FIXTURE_ROOT / "manifest.json")
    events_document = load_json(FIXTURE_ROOT / "events.json")
    wire_document = load_json(FIXTURE_ROOT / "wire-traces.json")
    require(manifest.get("schema_version") == 1, "unsupported_manifest_version")
    require(events_document.get("schema_version") == 1, "unsupported_events_version")
    require(
        file_sha256(FIXTURE_ROOT / "events.json") == manifest.get("events_sha256"),
        "events_hash_mismatch",
    )
    require(
        file_sha256(FIXTURE_ROOT / "wire-traces.json") == manifest.get("wire_traces_sha256"),
        "wire_traces_hash_mismatch",
    )
    events = events_document.get("events")
    require(isinstance(events, dict), "events_map_missing")

    sections = ["event_cases", "replaceable_cases", "privacy_cases", "mixed_version_cases", "relay_trace_cases"]
    case_ids = [case.get("id") for section in sections for case in manifest.get(section, [])]
    wire_ids = [trace.get("id") for trace in wire_document.get("traces", [])]
    case_ids.extend(wire_ids)
    require(all(isinstance(case_id, str) for case_id in case_ids), "case_id_missing")
    require(len(case_ids) == len(set(case_ids)), "duplicate_case_id")
    require(
        any(case.get("expected_error") for case in manifest["event_cases"]),
        "no_malformed_event_cases",
    )
    require(
        any(case.get("expected_error") for case in manifest["relay_trace_cases"]),
        "no_malformed_relay_traces",
    )
    if arguments.case:
        selected = [(section, case) for section in sections for case in manifest[section] if case["id"] == arguments.case]
        selected_wire = [trace for trace in wire_document["traces"] if trace["id"] == arguments.case]
        require(len(selected) + len(selected_wire) == 1, f"unknown_or_duplicate_case:{arguments.case}")
        if selected:
            selected_section, selected_case = selected[0]
            for section in sections:
                manifest[section] = [selected_case] if section == selected_section else []
            wire_document["traces"] = []
        else:
            for section in sections:
                manifest[section] = []
            wire_document["traces"] = selected_wire

    counts = {
        "events": check_event_cases(manifest, events),
        "replaceable": check_replaceable_cases(manifest, events),
        "privacy": check_privacy_cases(manifest, events),
        "mixed_version": check_mixed_version_cases(manifest, events),
        "wire": check_wire_traces(wire_document, events),
        "relay": check_relay_cases(manifest),
    }
    print("Protocol fixture check passed: " + " ".join(f"{name}={count}" for name, count in counts.items()))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except FixtureError as error:
        print(f"Protocol fixture check failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
