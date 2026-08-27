#!/usr/bin/env python3
"""Verify frozen Buzz SQL and desktop migration fixtures independently."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


FIXTURE_DIRECTORY = Path(__file__).resolve().parent
REPOSITORY_ROOT = FIXTURE_DIRECTORY.parents[4]
CATALOG_PATH = REPOSITORY_ROOT / ".agents/specs/collaborative-workspace/catalogs/data-sources.csv"
MANIFEST_PATH = FIXTURE_DIRECTORY / "manifest.json"
DESKTOP_FIXTURES_PATH = FIXTURE_DIRECTORY / "desktop-stores.json"

EXPECTED_DESKTOP_VERSIONS = {
    "DESKTOP-APP-DATA-001": {"sprout-legacy-v0", "buzz-release-v1", "buzz-dev-v1"},
    "DESKTOP-AGENTS-001": {"inline-fallback-v0", "key-reference-v1"},
    "DESKTOP-PERSONAS-001": {"provider-field-v0", "runtime-fold-v1"},
    "DESKTOP-TEAMS-001": {"directory-backed-v0", "detached-v1"},
    "DESKTOP-RETENTION-001": {"global-v0", "relay-owner-scoped-v1"},
    "DESKTOP-ARCHIVE-001": {
        "schema-v0",
        "schema-v1-harness",
        "schema-v2-cache-read",
        "schema-v3-pricing",
    },
    "DESKTOP-IDENTITY-KEYRING-001": {"per-entry-v0", "namespaced-blob-v1"},
    "DESKTOP-IDENTITY-FILE-001": {"owner-file-v0", "migrated-marker-v1"},
    "DESKTOP-KEY-BACKUP-001": {"nip49-v1"},
    "DESKTOP-LEGACY-WEBKIT-001": {"sprout-localstorage-v0"},
    "DESKTOP-NEST-001": {"sprout-nest-v0", "buzz-nest-v1"},
    "DESKTOP-REPOS-DIR-001": {"marker-v1"},
    "DESKTOP-RUNTIME-RECEIPTS-001": {"receipt-v1"},
    "DESKTOP-AGENT-LOGS-001": {"logs-v1"},
    "DESKTOP-EVENT-SYNC-001": {"json-retention-relay-v1"},
    "DESKTOP-MESH-IDENTITY-001": {"owner-keystore-v1"},
    "DESKTOP-WEBVIEW-LOCAL-001": {"local-storage-v1"},
    "DESKTOP-WEBVIEW-CACHE-001": {"projection-cache-v1"},
    "DESKTOP-WEBVIEW-PREFS-001": {"presentation-prefs-v1"},
    "DESKTOP-WEBVIEW-SESSION-001": {"session-storage-v1"},
}

FORBIDDEN_SECRET_FIELDS = {
    "private_key",
    "private_key_nsec",
    "secret_key",
    "seed",
    "seed_phrase",
    "mnemonic",
    "ciphertext",
}
FORBIDDEN_SECRET_TEXT = re.compile(
    r"(?:nsec1[023456789acdefghjklmnpqrstuvwxyz]+|ncryptsec1[023456789acdefghjklmnpqrstuvwxyz]+|"
    r"-----BEGIN (?:EC |OPENSSH )?PRIVATE KEY-----)",
    re.IGNORECASE,
)


class FixtureError(RuntimeError):
    pass


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise FixtureError(f"cannot load {path.relative_to(REPOSITORY_ROOT)}: {error}") from error
    if not isinstance(value, dict):
        raise FixtureError(f"{path.relative_to(REPOSITORY_ROOT)} must contain a JSON object")
    return value


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode(
        "utf-8"
    )


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_catalog() -> tuple[dict[str, dict[str, str]], dict[str, dict[str, str]]]:
    try:
        with CATALOG_PATH.open(newline="", encoding="utf-8") as handle:
            rows = list(csv.DictReader(handle))
    except OSError as error:
        raise FixtureError(f"cannot load data-source catalog: {error}") from error

    migrations = {
        row["data_source_id"]: row for row in rows if row["record_type"] == "sql_migration"
    }
    desktop = {
        row["data_source_id"]: row
        for row in rows
        if row["data_source_id"].startswith("DESKTOP-")
    }
    if len(migrations) != 30:
        raise FixtureError(f"catalog must contain 30 SQL migrations, found {len(migrations)}")
    if set(desktop) != set(EXPECTED_DESKTOP_VERSIONS):
        missing = sorted(set(EXPECTED_DESKTOP_VERSIONS) - set(desktop))
        extra = sorted(set(desktop) - set(EXPECTED_DESKTOP_VERSIONS))
        raise FixtureError(f"desktop catalog drift: missing={missing} extra={extra}")
    return migrations, desktop


def verify_source_paths(row: dict[str, str]) -> None:
    for source_path in row["source_paths"].split(";"):
        path = REPOSITORY_ROOT / source_path
        if not path.exists():
            raise FixtureError(f"{row['data_source_id']} source path does not exist: {source_path}")


def verify_sql_migrations(
    manifest: dict[str, Any], catalog: dict[str, dict[str, str]]
) -> None:
    entries = manifest.get("sql_migrations")
    if not isinstance(entries, list):
        raise FixtureError("manifest.sql_migrations must be an array")
    by_id: dict[str, dict[str, Any]] = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("data_source_id"), str):
            raise FixtureError("every SQL manifest entry needs a data_source_id")
        data_source_id = entry["data_source_id"]
        if data_source_id in by_id:
            raise FixtureError(f"duplicate SQL manifest entry: {data_source_id}")
        by_id[data_source_id] = entry

    if set(by_id) != set(catalog):
        missing = sorted(set(catalog) - set(by_id))
        extra = sorted(set(by_id) - set(catalog))
        raise FixtureError(f"SQL fixture coverage drift: missing={missing} extra={extra}")

    expected_sequences = list(range(1, 31))
    actual_sequences: list[int] = []
    for data_source_id in sorted(by_id):
        entry = by_id[data_source_id]
        row = catalog[data_source_id]
        verify_source_paths(row)
        source_path = row["source_paths"]
        if entry.get("source_path") != source_path:
            raise FixtureError(f"{data_source_id} source path differs from the catalog")
        source = (REPOSITORY_ROOT / source_path).read_bytes()
        expected_hash = sha256_bytes(source)
        if entry.get("sha256") != expected_hash:
            raise FixtureError(f"{data_source_id} SHA-256 mismatch")
        if entry.get("byte_count") != len(source):
            raise FixtureError(f"{data_source_id} byte count mismatch")
        line_count = len(source.splitlines())
        if entry.get("line_count") != line_count:
            raise FixtureError(f"{data_source_id} line count mismatch")
        sequence = int(data_source_id.removeprefix("MIG-"))
        if entry.get("sequence") != sequence:
            raise FixtureError(f"{data_source_id} sequence mismatch")
        if entry.get("name") != row["name"]:
            raise FixtureError(f"{data_source_id} name differs from the catalog")
        actual_sequences.append(sequence)
    if sorted(actual_sequences) != expected_sequences:
        raise FixtureError(f"SQL migration sequence has gaps: {sorted(actual_sequences)}")


def scan_for_secret_material(value: Any, location: str = "fixture") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            normalized_key = key.lower().replace("-", "_")
            if normalized_key in FORBIDDEN_SECRET_FIELDS:
                raise FixtureError(f"private key material field is prohibited at {location}.{key}")
            scan_for_secret_material(child, f"{location}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            scan_for_secret_material(child, f"{location}[{index}]")
    elif isinstance(value, str) and FORBIDDEN_SECRET_TEXT.search(value):
        raise FixtureError(f"private key material is prohibited at {location}")


def verify_desktop_fixtures(
    manifest: dict[str, Any],
    fixture_document: dict[str, Any],
    catalog: dict[str, dict[str, str]],
) -> None:
    fixtures = fixture_document.get("fixtures")
    if fixture_document.get("format_version") != 1 or not isinstance(fixtures, list):
        raise FixtureError("desktop-stores.json must be format version 1 with a fixtures array")

    fixture_by_id: dict[str, dict[str, Any]] = {}
    versions_by_store: dict[str, set[str]] = {data_source_id: set() for data_source_id in catalog}
    for fixture in fixtures:
        if not isinstance(fixture, dict):
            raise FixtureError("every desktop fixture must be an object")
        fixture_id = fixture.get("fixture_id")
        data_source_id = fixture.get("data_source_id")
        version = fixture.get("version")
        if not all(isinstance(value, str) and value for value in (fixture_id, data_source_id, version)):
            raise FixtureError("every desktop fixture needs non-empty fixture_id, data_source_id and version")
        if fixture_id in fixture_by_id:
            raise FixtureError(f"duplicate desktop fixture ID: {fixture_id}")
        if data_source_id not in catalog:
            raise FixtureError(f"unknown desktop data source in {fixture_id}: {data_source_id}")
        if version in versions_by_store[data_source_id]:
            raise FixtureError(f"duplicate version for {data_source_id}: {version}")
        if fixture.get("contains_private_key_material") is not False:
            raise FixtureError(f"{fixture_id} must explicitly declare no private key material")
        if not isinstance(fixture.get("migration_state"), str) or not fixture["migration_state"]:
            raise FixtureError(f"{fixture_id} must declare migration_state")
        if not isinstance(fixture.get("records"), list) or not fixture["records"]:
            raise FixtureError(f"{fixture_id} must contain at least one sanitized record")
        if not isinstance(fixture.get("expected"), dict) or not fixture["expected"]:
            raise FixtureError(f"{fixture_id} must declare expected migration behavior")
        scan_for_secret_material(fixture, fixture_id)
        fixture_by_id[fixture_id] = fixture
        versions_by_store[data_source_id].add(version)

    for data_source_id, expected_versions in EXPECTED_DESKTOP_VERSIONS.items():
        if versions_by_store[data_source_id] != expected_versions:
            raise FixtureError(
                f"{data_source_id} version coverage drift: "
                f"expected={sorted(expected_versions)} actual={sorted(versions_by_store[data_source_id])}"
            )
        verify_source_paths(catalog[data_source_id])

    entries = manifest.get("desktop_fixtures")
    if not isinstance(entries, list):
        raise FixtureError("manifest.desktop_fixtures must be an array")
    manifest_by_id: dict[str, dict[str, Any]] = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("fixture_id"), str):
            raise FixtureError("every desktop manifest entry needs a fixture_id")
        fixture_id = entry["fixture_id"]
        if fixture_id in manifest_by_id:
            raise FixtureError(f"duplicate desktop manifest entry: {fixture_id}")
        manifest_by_id[fixture_id] = entry
    if set(manifest_by_id) != set(fixture_by_id):
        missing = sorted(set(fixture_by_id) - set(manifest_by_id))
        extra = sorted(set(manifest_by_id) - set(fixture_by_id))
        raise FixtureError(f"desktop fixture index drift: missing={missing} extra={extra}")

    for fixture_id, fixture in fixture_by_id.items():
        entry = manifest_by_id[fixture_id]
        if entry.get("data_source_id") != fixture["data_source_id"]:
            raise FixtureError(f"{fixture_id} data source mismatch")
        if entry.get("version") != fixture["version"]:
            raise FixtureError(f"{fixture_id} version mismatch")
        if entry.get("record_count") != len(fixture["records"]):
            raise FixtureError(f"{fixture_id} record count mismatch")
        expected_hash = sha256_bytes(canonical_bytes(fixture))
        if entry.get("sha256") != expected_hash:
            raise FixtureError(f"{fixture_id} SHA-256 mismatch")

    document_hash = sha256_bytes(DESKTOP_FIXTURES_PATH.read_bytes())
    if manifest.get("desktop_document_sha256") != document_hash:
        raise FixtureError("desktop fixture document SHA-256 mismatch")


def run_negative_self_tests(manifest: dict[str, Any], fixtures: dict[str, Any]) -> None:
    first_fixture = fixtures["fixtures"][0]
    indexed = {entry["fixture_id"]: entry for entry in manifest["desktop_fixtures"]}
    mutated = json.loads(json.dumps(first_fixture))
    mutated["records"][0]["kind"] = "mutated"
    if sha256_bytes(canonical_bytes(mutated)) == indexed[first_fixture["fixture_id"]]["sha256"]:
        raise FixtureError("hash negative self-test did not detect a fixture mutation")

    secret_fixture = {"credential": "nsec1fixturemustneverpass"}
    try:
        scan_for_secret_material(secret_fixture, "negative-self-test")
    except FixtureError:
        return
    raise FixtureError("secret-material negative self-test did not reject an nsec")


def check() -> tuple[int, int]:
    manifest = load_json(MANIFEST_PATH)
    fixtures = load_json(DESKTOP_FIXTURES_PATH)
    if manifest.get("format_version") != 1:
        raise FixtureError("manifest format_version must be 1")
    migrations, desktop = read_catalog()
    verify_sql_migrations(manifest, migrations)
    verify_desktop_fixtures(manifest, fixtures, desktop)
    run_negative_self_tests(manifest, fixtures)
    return len(migrations), len(fixtures["fixtures"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        migration_count, desktop_fixture_count = check()
    except FixtureError as error:
        print(f"migration fixture check failed: {error}", file=sys.stderr)
        return 1
    print(
        "Migration fixture check passed: "
        f"sql_migrations={migration_count} desktop_stores={len(EXPECTED_DESKTOP_VERSIONS)} "
        f"desktop_versions={desktop_fixture_count} secret_material=absent"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
