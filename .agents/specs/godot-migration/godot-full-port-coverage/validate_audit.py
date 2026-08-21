#!/usr/bin/env python3

import csv
import hashlib
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

from verify_snapshot import git_object_hash


ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[3]
CATALOG = ROOT / "catalogs" / "master-coverage.csv"
EXPECTED_GODOT_COMMIT = "5b4e0cb0fd279832bbdd69fed5354d4e5ad26f88"
EXPECTED_ZED_COMMIT = "95c903d0d2feba228d73b813216c2ff2cc585119"
EXPECTED_MANIFEST_SHA256 = "3f52220d352a6156c26f75006476201b548b41b418903832f8d318eb9aca34e2"
CLASSIFICATIONS = {
    "Already implemented in Zed and reusable without changes",
    "Partially implemented in Zed and should be extended",
    "Fully covered by an existing Godot migration spec",
    "Partially covered by an existing migration spec",
    "Missing from the migration specs",
    "Intentionally excluded, with a documented rationale",
    "Internal/upstream infrastructure that does not require a direct port",
}
REQUIRED_COLUMNS = {
    "capability_id",
    "domain",
    "subdomain",
    "observable_behavior",
    "supported_modes_and_platform_differences",
    "success_failure_persistence_lifecycle",
    "godot_evidence",
    "existing_zed_evidence",
    "spec_coverage",
    "classification",
    "proposed_owner_in_sim",
    "existing_or_proposed_native_zed_owner",
    "build_time_dependency_on_godot",
    "runtime_dependency_on_godot",
    "zed_native_storage_path",
    "zed_native_execution_path",
    "zed_native_ui_path",
    "zed_native_lifecycle_path",
    "godot_compatible_file_or_api_boundary",
    "existing_zed_reuse_evidence",
    "reuse_or_extension_strategy",
    "remaining_gap",
    "verification_needed",
    "no_godot_installation_validation",
    "confidence",
    "open_questions",
}


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)


def manifest_digest(source: Path) -> tuple[int, str]:
    manifest = []
    for path in sorted(source.rglob("*")):
        if not path.is_file():
            continue
        relative_path = path.relative_to(source).as_posix()
        data = path.read_bytes()
        if path.suffix in {".bat", ".sln", ".csproj"} or relative_path.startswith("misc/msvs/"):
            data = data.replace(b"\r\n", b"\n")
        manifest.append(f"{git_object_hash('blob', data).hex()}\t{relative_path}\n")
    manifest.sort(key=lambda line: line.split("\t", 1)[1].encode())
    encoded = "".join(manifest).encode()
    return len(manifest), hashlib.sha256(encoded).hexdigest()


def main() -> int:
    errors = 0
    with CATALOG.open(encoding="utf-8", newline="") as file:
        reader = csv.DictReader(file)
        rows = list(reader)
        columns = set(reader.fieldnames or [])
    if columns != REQUIRED_COLUMNS:
        fail(f"catalog columns differ: missing={sorted(REQUIRED_COLUMNS - columns)} extra={sorted(columns - REQUIRED_COLUMNS)}")
        errors += 1
    if len(rows) != 198:
        fail(f"expected 198 capability rows, found {len(rows)}")
        errors += 1
    identifiers = [row["capability_id"] for row in rows]
    if len(identifiers) != len(set(identifiers)):
        fail("capability IDs are not unique")
        errors += 1
    if any(not re.fullmatch(r"GODOT-[A-Z0-9]+-\d{3}", identifier) for identifier in identifiers):
        fail("one or more capability IDs violate GODOT-<DOMAIN>-<NUMBER>")
        errors += 1
    per_domain_numbers = defaultdict(list)
    for identifier in identifiers:
        _, domain, number = identifier.split("-")
        per_domain_numbers[domain].append(int(number))
    for domain, numbers in per_domain_numbers.items():
        if numbers != list(range(1, len(numbers) + 1)):
            fail(f"{domain} capability numbering is not stable and contiguous")
            errors += 1
    for row in rows:
        missing = [column for column in REQUIRED_COLUMNS if not row[column].strip()]
        if missing:
            fail(f"{row['capability_id']} has blank fields: {', '.join(sorted(missing))}")
            errors += 1
        if row["classification"] not in CLASSIFICATIONS:
            fail(f"{row['capability_id']} has invalid classification: {row['classification']}")
            errors += 1
        if row["confidence"] not in {"High", "Medium", "Low"}:
            fail(f"{row['capability_id']} has invalid confidence")
            errors += 1
        for evidence in row["godot_evidence"].split("; "):
            source_path = evidence.split("::", 1)[0].strip()
            if not source_path.startswith("projects/godot/"):
                fail(f"{row['capability_id']} has imprecise Godot evidence: {evidence}")
                errors += 1
            elif not (REPOSITORY / source_path).exists():
                fail(f"{row['capability_id']} Godot evidence path does not exist: {source_path}")
                errors += 1
        zed_path_found = False
        for evidence in row["existing_zed_evidence"].split("; "):
            zed_path = evidence.split("::", 1)[0].strip()
            if zed_path.startswith(("crates/", "script/", "tooling/", ".github/", "Cargo.toml", "Cargo.lock", "deny.toml")):
                zed_path_found = zed_path_found or (REPOSITORY / zed_path).exists()
        if not zed_path_found:
            fail(f"{row['capability_id']} has no existing precise Zed evidence path")
            errors += 1
        if not re.search(r"Audit closure: R\d+\.1-R\d+\.4; D-[A-Z0-9]+; T\d+\. Native gate: R23\.1-R23\.10; D-NATIVE; T200\.", row["spec_coverage"]):
            fail(f"{row['capability_id']} lacks exact audit traceability")
            errors += 1
        if not row["build_time_dependency_on_godot"].startswith("No."):
            fail(f"{row['capability_id']} does not prohibit a Godot build-time dependency")
            errors += 1
        if not row["runtime_dependency_on_godot"].startswith("No."):
            fail(f"{row['capability_id']} does not prohibit a Godot runtime dependency")
            errors += 1
        if row["existing_or_proposed_native_zed_owner"] != row["proposed_owner_in_sim"]:
            fail(f"{row['capability_id']} native owner disagrees with proposed Zed owner")
            errors += 1
        validation_text = row["no_godot_installation_validation"].lower()
        if "godot absent" not in validation_text or "process tree" not in validation_text or "dependency manifest" not in validation_text:
            fail(f"{row['capability_id']} lacks hermetic no-Godot process/link validation")
            errors += 1
    tasks = (ROOT / "tasks.md").read_text(encoding="utf-8")
    requirements = (ROOT / "requirements.md").read_text(encoding="utf-8")
    design = (ROOT / "design.md").read_text(encoding="utf-8")
    task_ids = set(re.findall(r"^- \[ \] (\d+)\.", tasks, re.MULTILINE))
    requirement_ids = set(re.findall(r"\*\*(\d+\.\d+)\*\*", requirements))
    design_ids = set(re.findall(r"^### (D-[A-Z0-9]+):", design, re.MULTILINE))
    for row in rows:
        trace = re.search(r"Audit closure: R(\d+)\.1-R\d+\.4; (D-[A-Z0-9]+); T(\d+)\. Native gate: R23\.1-R23\.10; D-NATIVE; T200\.", row["spec_coverage"])
        if trace is None:
            continue
        requirement, design_id, task = trace.groups()
        if any(f"{requirement}.{acceptance}" not in requirement_ids for acceptance in range(1, 5)):
            fail(f"{row['capability_id']} references missing requirement criteria")
            errors += 1
        if design_id not in design_ids:
            fail(f"{row['capability_id']} references missing {design_id}")
            errors += 1
        if task not in task_ids:
            fail(f"{row['capability_id']} references missing task {task}")
            errors += 1
    if any(f"23.{acceptance}" not in requirement_ids for acceptance in range(1, 11)):
        fail("native Zed acceptance criteria 23.1-23.10 are incomplete")
        errors += 1
    if "D-NATIVE" not in design_ids:
        fail("native Zed design element D-NATIVE is missing")
        errors += 1
    if "200" not in task_ids:
        fail("native Zed audit leaf task 200 is missing")
        errors += 1
    migration_root = ROOT.parent
    checked_tasks = []
    for task_file in migration_root.rglob("tasks.md"):
        for line_number, line in enumerate(task_file.read_text(encoding="utf-8").splitlines(), start=1):
            if re.match(r"^\s*- \[[xX]\]", line):
                checked_tasks.append(f"{task_file.relative_to(REPOSITORY)}:{line_number}")
    if checked_tasks:
        fail(f"checked tasks remain: {', '.join(checked_tasks[:10])}")
        errors += 1
    prohibited_active_phrases = {
        "external-command": "external-command coverage path",
        "ExternalCommand": "ExternalCommand boundary variant",
        "invoke configured external Godot": "external Godot run/debug invocation",
        "Godot executable settings": "Godot executable configuration task",
        "External Godot task integration only": "external Godot export delegation",
        "metadata and external simulation fallback": "external Godot simulation fallback",
    }
    active_files = [migration_root / "requirements.md", migration_root / "design.md", migration_root / "tasks.md", migration_root / "master-migration-plan.md"]
    for directory in migration_root.iterdir():
        if not directory.is_dir() or directory.name.startswith("comfy-") or directory == ROOT:
            continue
        active_files.extend(directory / name for name in ("requirements.md", "design.md", "tasks.md"))
    for active_file in active_files:
        if not active_file.exists():
            continue
        contents = active_file.read_text(encoding="utf-8")
        for phrase, description in prohibited_active_phrases.items():
            if phrase in contents:
                fail(f"{active_file.relative_to(REPOSITORY)} retains prohibited {description}: {phrase}")
                errors += 1
    summary = (ROOT / "coverage-summary.md").read_text(encoding="utf-8")
    classification_counts = Counter(row["classification"] for row in rows)
    overall_row = re.search(r"^\| \*\*Overall\*\* \| (.*?) \| \*\*(\d+)\*\* \|$", summary, re.MULTILINE)
    if overall_row is None or int(overall_row.group(2)) != len(rows):
        fail("coverage summary denominator does not reconcile")
        errors += 1
    if sum(classification_counts.values()) != len(rows):
        fail("classification totals do not reconcile")
        errors += 1
    baseline_exists = subprocess.run(
        ["git", "cat-file", "-e", f"{EXPECTED_ZED_COMMIT}^{{commit}}"],
        cwd=REPOSITORY,
        capture_output=True,
        text=True,
    )
    baseline_is_ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", EXPECTED_ZED_COMMIT, "HEAD"],
        cwd=REPOSITORY,
        capture_output=True,
        text=True,
    )
    if baseline_exists.returncode != 0 or baseline_is_ancestor.returncode != 0:
        current_commit = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=REPOSITORY, check=True, capture_output=True, text=True
        ).stdout.strip()
        fail(f"Zed baseline {EXPECTED_ZED_COMMIT} is missing or is not an ancestor of {current_commit}")
        errors += 1
    file_count, digest = manifest_digest(REPOSITORY / "projects" / "godot")
    if file_count != 13979 or digest != EXPECTED_MANIFEST_SHA256:
        fail(f"Godot snapshot drifted: files={file_count}, manifest={digest}, expected commit={EXPECTED_GODOT_COMMIT}")
        errors += 1
    if errors:
        return 1
    print(
        f"Validated Godot audit: {len(rows)} capabilities, {len(per_domain_numbers)} domains, "
        f"7 classifications, {file_count} Godot source paths"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
