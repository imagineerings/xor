#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[2]
CATALOGS = ROOT / "catalogs"
SNAPSHOT_MANIFEST = CATALOGS / "source-snapshot-manifest.json"

PIPELINE = [
    "catalogs/generate-desktop-catalogs.py",
    "catalogs/generate-desktop-telemetry.py",
    "generate_desktop_renderer_surfaces.py",
    "generate_frontend_component_surfaces.py",
    "generate_comfy_cli_catalogs.py",
    "generate_documentation_catalogs.py",
    "generate_tensor_runtime_catalogs.py",
    "generate_tensor_operation_contracts.py",
    "../../../crates/comfy_model/scripts/generate_model_family_catalog.py",
    "generate_spandrel_image_model_contract.py",
    "generate_conditioning_catalog.py",
    "generate_node_contract_catalog.py",
    "generate_provider_component_catalog.py",
    "generate_shell_catalog.py",
    "generate_frontend_extension_catalog.py",
    "regenerate_native_zed_evidence.py",
    "generate_ownership_catalog.py",
    "regenerate_native_planning.py",
    "generate_master_catalog.py",
]

CONVERGENCE_PIPELINE = [
    "generate_master_catalog.py",
]

GENERATED_EXTERNAL_OUTPUTS = [
    WORKSPACE / "crates/comfy_ui/src/generated_command_catalog.rs",
    WORKSPACE / "crates/comfy_ui/src/generated_execution_catalog.rs",
    WORKSPACE / "crates/comfy_ui/src/generated_keybinding_catalog.rs",
    WORKSPACE / "crates/comfy_ui/src/generated_menu_catalog.rs",
    WORKSPACE / "crates/comfy_ui/src/generated_frontend_extension_catalog.rs",
    WORKSPACE / "assets/keymaps/default-comfy.json",
    WORKSPACE / "crates/comfy_tensor/src/operation_contract_records.rs",
    WORKSPACE / "crates/comfy_model/catalog/model-families-v1.json",
    WORKSPACE / "crates/comfy_test_support/fixtures/tensor_signatures/resolution-environment.json",
    WORKSPACE / "crates/comfy_test_support/fixtures/tensor_signatures/contracts",
    WORKSPACE / "crates/comfy_test_support/fixtures/models/spandrel-image-model-contract",
]

SNAPSHOT_INPUTS = [
    "backend-config.csv",
    "backend-external-services.csv",
    "backend-features.csv",
    "backend-formats.csv",
    "backend-http-routes.csv",
    "backend-inactive-nodes.csv",
    "backend-models.csv",
    "backend-nodes.csv",
    "backend-reconciliation.json",
    "backend-schemas.csv",
    "backend-source-coverage.csv",
    "backend-tests.csv",
    "backend-websocket-events.csv",
    "cross-compatibility.csv",
    "cross-formats.csv",
    "frontend-commands.csv",
    "frontend-extensions.csv",
    "frontend-feature-flags.csv",
    "frontend-features.csv",
    "frontend-formats-migrations.csv",
    "frontend-http-usage.csv",
    "frontend-keybindings.csv",
    "frontend-localization.csv",
    "frontend-menus.csv",
    "frontend-persisted-state.csv",
    "frontend-reconciliation.csv",
    "frontend-routes.csv",
    "frontend-settings.csv",
    "frontend-source-files.csv",
    "frontend-summary.json",
    "frontend-telemetry.csv",
    "frontend-test-cases.csv",
    "frontend-websocket.csv",
]

TARGET_FIELDS = {
    "zed_status",
    "current_zed_status",
    "target_status",
    "zed_evidence",
    "parity_gap",
    "exact_parity_gap",
    "target_gap",
    "parity_decision",
    "acceptance",
    "acceptance_criteria",
    "zed_acceptance",
    "observable_zed_acceptance",
    "target_acceptance",
    "requirements",
    "design",
    "task",
    "validation",
    "automated_validation",
    "manual_validation",
    "open_questions",
    "open_questions_assumptions",
}


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def snapshot_digest(path: Path) -> str:
    if path.suffix != ".csv":
        return digest_bytes(path.read_bytes())
    with path.open(newline="", encoding="utf-8") as handle:
        rows = [
            {key: value for key, value in row.items() if key not in TARGET_FIELDS}
            for row in csv.DictReader(handle)
        ]
    return digest_bytes(json.dumps(rows, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def current_snapshot_manifest() -> dict[str, object]:
    return {
        "schema_version": 1,
        "classification": "checksum-locked source snapshot inputs; target-only columns excluded",
        "inputs": {
            name: snapshot_digest(CATALOGS / name)
            for name in SNAPSHOT_INPUTS
        },
    }


def verify_snapshot_manifest(refresh: bool, allow_create: bool) -> None:
    current = current_snapshot_manifest()
    if refresh or (allow_create and not SNAPSHOT_MANIFEST.exists()):
        SNAPSHOT_MANIFEST.write_text(json.dumps(current, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return
    if not SNAPSHOT_MANIFEST.exists():
        raise RuntimeError("source-snapshot-manifest.json is missing")
    expected = json.loads(SNAPSHOT_MANIFEST.read_text(encoding="utf-8"))
    if expected != current:
        expected_inputs = expected.get("inputs", {})
        current_inputs = current["inputs"]
        changed = sorted(
            name
            for name in set(expected_inputs) | set(current_inputs)
            if expected_inputs.get(name) != current_inputs.get(name)
        )
        raise RuntimeError(f"source snapshot inputs changed without an explicit baseline refresh: {changed}")


def run_pipeline() -> None:
    for relative in PIPELINE:
        subprocess.run([sys.executable, str(ROOT / relative)], cwd=WORKSPACE, check=True)
    # The master pass synchronizes target-only columns back into checksum-locked
    # source catalogs, so a second pass must consume that normalized projection.
    for relative in CONVERGENCE_PIPELINE:
        subprocess.run([sys.executable, str(ROOT / relative)], cwd=WORKSPACE, check=True)


def pack_hashes() -> dict[str, str]:
    result = {}
    for path in sorted(ROOT.rglob("*")):
        if (
            not path.is_file()
            or "__pycache__" in path.parts
            or path.name.startswith(("audit-", "._"))
            or path.name == ".DS_Store"
        ):
            continue
        result[path.relative_to(ROOT).as_posix()] = digest_bytes(path.read_bytes())
    for path in GENERATED_EXTERNAL_OUTPUTS:
        if path.is_file():
            result[f"@workspace/{path.relative_to(WORKSPACE).as_posix()}"] = digest_bytes(path.read_bytes())
        elif path.is_dir():
            for generated_path in sorted(path.rglob("*")):
                if (
                    generated_path.is_file()
                    and not generated_path.name.startswith("._")
                    and generated_path.name != ".DS_Store"
                ):
                    result[
                        f"@workspace/{generated_path.relative_to(WORKSPACE).as_posix()}"
                    ] = digest_bytes(generated_path.read_bytes())
    return result


def changed_paths(before: dict[str, str], after: dict[str, str]) -> list[str]:
    return sorted(
        name
        for name in set(before) | set(after)
        if before.get(name) != after.get(name)
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-twice", action="store_true")
    parser.add_argument("--refresh-snapshot-manifest", action="store_true")
    args = parser.parse_args()

    checking = args.check or args.check_twice
    verify_snapshot_manifest(args.refresh_snapshot_manifest, allow_create=not checking)
    before = pack_hashes()
    run_pipeline()
    after_first = pack_hashes()
    if checking:
        changed = changed_paths(before, after_first)
        if changed:
            raise RuntimeError(f"checked-in spec artifacts were stale before regeneration: {changed}")
    if args.check_twice:
        run_pipeline()
        after_second = pack_hashes()
        changed = changed_paths(after_first, after_second)
        if changed:
            raise RuntimeError(f"second regeneration was not byte-stable: {changed}")
    verify_snapshot_manifest(False, allow_create=False)
    print("Comfy parity regeneration pipeline completed with snapshot-input closure.")


if __name__ == "__main__":
    main()
