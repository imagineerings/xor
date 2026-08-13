#!/usr/bin/env python3

import argparse
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path


REQUIREMENT_ID = re.compile(r"^\s*\d+\.\s+\*\*(\d+\.\d+)\*\*", re.MULTILINE)
REFERENCE_ID = re.compile(r"\b\d+\.\d+\b")
TASK_ID_REFERENCE = re.compile(r"\b\d+(?:\.\d+)?\b")
TASK_HEADER = re.compile(
    r"^(?P<indent>[ \t]*)-\s+\[[ xX]\]\s+(?P<id>\d+(?:\.\d+)*)\.\s+",
    re.MULTILINE,
)
HEADING = re.compile(r"^#{2,6}\s+(?P<title>.+?)\s*$", re.MULTILINE)
METADATA_ENTRY = re.compile(
    r"^(?P<indent>[ \t]*)-\s+_"
    r"(?P<label>Requirements|Depends on|Reads|Writes|Validation):\s*"
    r"(?P<value>.*?)_\s*$",
    re.IGNORECASE | re.MULTILINE,
)
KEBAB_CASE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
REQUIRED_METADATA = ("Requirements", "Depends on", "Reads", "Writes", "Validation")
CONTAINER_ROOTS = {
    "apps",
    "components",
    "crates",
    "libs",
    "modules",
    "packages",
    "plugins",
    "services",
}


def referenced_ids(value):
    return set(REFERENCE_ID.findall(value))


def design_references(design):
    references = set()
    traceability = re.search(
        r"^## Requirements traceability\s*$\n(?P<body>.*?)(?=^##\s|\Z)",
        design,
        re.IGNORECASE | re.MULTILINE | re.DOTALL,
    )
    if traceability:
        for line in traceability.group("body").splitlines():
            cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
            if len(cells) >= 3:
                references.update(referenced_ids(cells[0]))

    for match in re.finditer(
        r"(?:Validates|Addresses):\s*(?:Requirements?\s*)?([^\n]+)",
        design,
        re.IGNORECASE,
    ):
        references.update(referenced_ids(match.group(1)))
    return references


def task_blocks(tasks):
    headers = list(TASK_HEADER.finditer(tasks))
    blocks = []
    for index, header in enumerate(headers):
        end = headers[index + 1].start() if index + 1 < len(headers) else len(tasks)
        blocks.append((header.group("id"), tasks[header.start():end]))
    return blocks


def task_metadata(tasks, headers):
    owned = defaultdict(lambda: defaultdict(list))
    orphaned = []
    for entry in METADATA_ENTRY.finditer(tasks):
        entry_indent = len(entry.group("indent").expandtabs(4))
        owners = [
            header
            for header in headers
            if header.start() < entry.start()
            and len(header.group("indent").expandtabs(4)) < entry_indent
        ]
        if not owners:
            orphaned.append(entry.group("label").capitalize())
            continue
        owner = owners[-1].group("id")
        label = entry.group("label").capitalize()
        owned[owner][label].append(entry.group("value").strip())
    return owned, orphaned


def dependency_ids(value):
    if value.strip().lower() == "none":
        return set(), None
    dependency_list = r"\d+(?:\.\d+)?(?:\s*,\s*\d+(?:\.\d+)?)*"
    if not re.fullmatch(dependency_list, value.strip()):
        return set(), "use 'none' or a comma-separated list of task IDs"
    return set(TASK_ID_REFERENCE.findall(value)), None


def path_items(value):
    return [
        item.strip().strip("`")
        for item in value.split(",")
        if item.strip().strip("`").lower() not in {"", "none", "n/a"}
    ]


def broad_write_glob(path):
    normalized = path.replace("\\", "/").removeprefix("./")
    parts = [part for part in normalized.split("/") if part]
    return "**" in normalized or any(
        "*" in part or "?" in part or "[" in part for part in parts[:-1]
    ) or normalized in {"*", "**"}


def subsystem(path):
    parts = [
        part
        for part in path.replace("\\", "/").removeprefix("./").split("/")
        if part
    ]
    if not parts:
        return None
    if len(parts) > 1 and parts[0] in CONTAINER_ROOTS:
        return "/".join(parts[:2])
    if len(parts) == 1:
        return "<repository root>"
    return parts[0]


def validate_contents(feature_name, requirements, design, tasks):
    errors = []
    warnings = []

    if not KEBAB_CASE.fullmatch(feature_name):
        errors.append(f"feature directory must be kebab-case: {feature_name}")

    requirement_list = REQUIREMENT_ID.findall(requirements)
    requirement_counts = Counter(requirement_list)
    requirement_ids = set(requirement_list)
    if not requirement_ids:
        errors.append("requirements.md contains no explicit acceptance criterion IDs")
    duplicates = sorted(item for item, count in requirement_counts.items() if count > 1)
    if duplicates:
        errors.append(f"duplicate acceptance criterion IDs: {', '.join(duplicates)}")

    design_ids = design_references(design)
    unknown_design = sorted(design_ids - requirement_ids)
    if unknown_design:
        errors.append(f"design.md references unknown requirements: {', '.join(unknown_design)}")
    missing_design = sorted(requirement_ids - design_ids)
    if missing_design:
        errors.append(f"design.md does not trace requirements: {', '.join(missing_design)}")

    headers = list(TASK_HEADER.finditer(tasks))
    blocks = task_blocks(tasks)
    task_ids = [header.group("id") for header in headers]
    known_task_ids = set(task_ids)
    duplicate_tasks = sorted(item for item, count in Counter(task_ids).items() if count > 1)
    if duplicate_tasks:
        errors.append(f"duplicate task IDs: {', '.join(duplicate_tasks)}")
    if not blocks:
        errors.append("tasks.md contains no checkbox tasks with numeric IDs")

    epic_ids = {task_id for task_id in task_ids if "." not in task_id}
    leaf_ids = {task_id for task_id in task_ids if task_id.count(".") == 1}
    deep_ids = sorted(task_id for task_id in task_ids if task_id.count(".") > 1)
    if deep_ids:
        errors.append(
            "tasks use unsupported nesting below implementation leaves: "
            + ", ".join(deep_ids)
        )

    headings = list(HEADING.finditer(tasks))
    for header in headers:
        task_id = header.group("id")
        indent = header.group("indent")
        if task_id in epic_ids:
            if indent:
                errors.append(f"epic task {task_id} must be a top-level checkbox")
            if not any(leaf.startswith(f"{task_id}.") for leaf in leaf_ids):
                errors.append(f"epic task {task_id} has no implementation leaves")
            preceding_headings = [
                heading for heading in headings if heading.start() < header.start()
            ]
            if not preceding_headings or not preceding_headings[-1].group(
                "title"
            ).lower().startswith("milestone"):
                errors.append(f"epic task {task_id} must appear under a milestone heading")
        elif task_id in leaf_ids:
            parent_id = task_id.split(".", 1)[0]
            if not indent:
                errors.append(
                    f"implementation leaf {task_id} must be indented under epic {parent_id}"
                )
            if parent_id not in epic_ids:
                errors.append(f"implementation leaf {task_id} has no parent epic {parent_id}")
            preceding_epics = [
                candidate
                for candidate in headers
                if candidate.start() < header.start()
                and candidate.group("id") in epic_ids
            ]
            if not preceding_epics or preceding_epics[-1].group("id") != parent_id:
                errors.append(
                    f"implementation leaf {task_id} is not nested under epic {parent_id}"
                )

    owned_metadata, orphaned_metadata = task_metadata(tasks, headers)
    if orphaned_metadata:
        errors.append(
            "task metadata is not nested under a task: "
            + ", ".join(sorted(set(orphaned_metadata)))
        )
    for task_id in sorted(epic_ids):
        labels = sorted(owned_metadata.get(task_id, {}))
        if labels:
            errors.append(
                f"metadata must belong to implementation leaves, but epic {task_id} has: "
                + ", ".join(labels)
            )

    task_requirement_ids = set()
    dependencies = {}
    write_owners = defaultdict(list)

    for task_id in sorted(leaf_ids):
        task_values = owned_metadata.get(task_id, {})
        duplicate_metadata = sorted(
            label for label, values in task_values.items() if len(values) > 1
        )
        if duplicate_metadata:
            errors.append(
                f"task {task_id} has duplicate metadata: {', '.join(duplicate_metadata)}"
            )
        values = {
            label: task_values[label][0] if len(task_values.get(label, [])) == 1 else None
            for label in REQUIRED_METADATA
        }
        missing = [label for label, value in values.items() if value is None]
        if missing:
            errors.append(f"task {task_id} is missing metadata: {', '.join(missing)}")
            continue
        empty = [label for label, value in values.items() if not value]
        if empty:
            errors.append(f"task {task_id} has empty metadata: {', '.join(empty)}")
            continue

        task_requirement_ids.update(referenced_ids(values["Requirements"]))
        parsed_dependencies, dependency_error = dependency_ids(values["Depends on"])
        if dependency_error:
            errors.append(f"task {task_id} has invalid dependencies: {dependency_error}")
        dependencies[task_id] = parsed_dependencies
        unknown_dependencies = sorted(parsed_dependencies - known_task_ids)
        if unknown_dependencies:
            errors.append(
                f"task {task_id} depends on unknown tasks: {', '.join(unknown_dependencies)}"
            )
        epic_dependencies = sorted(parsed_dependencies & epic_ids)
        if epic_dependencies:
            errors.append(
                f"task {task_id} depends on parent epics instead of leaves: "
                + ", ".join(epic_dependencies)
            )
        if task_id in parsed_dependencies:
            errors.append(f"task {task_id} depends on itself")
        writes = path_items(values["Writes"])
        broad_globs = sorted(path for path in writes if broad_write_glob(path))
        if broad_globs:
            warnings.append(
                f"task {task_id} uses broad write globs: {', '.join(broad_globs)}"
            )
        if len(writes) > 5:
            warnings.append(
                f"task {task_id} has {len(writes)} write targets; review whether outcomes should split"
            )
        subsystems = sorted(
            item for item in {subsystem(path) for path in writes} if item is not None
        )
        if len(subsystems) > 1:
            warnings.append(
                f"task {task_id} spans top-level subsystems: {', '.join(subsystems)}"
            )
        for path in writes:
            if task_id not in write_owners[path]:
                write_owners[path].append(task_id)

    unknown_tasks = sorted(task_requirement_ids - requirement_ids)
    if unknown_tasks:
        errors.append(f"tasks.md references unknown requirements: {', '.join(unknown_tasks)}")
    missing_tasks = sorted(requirement_ids - task_requirement_ids)
    if missing_tasks:
        errors.append(f"tasks.md does not cover requirements: {', '.join(missing_tasks)}")

    visiting = set()
    visited = set()

    def visit(task_id):
        if task_id in visiting:
            return True
        if task_id in visited:
            return False
        visiting.add(task_id)
        if any(visit(dependency) for dependency in dependencies.get(task_id, set())):
            return True
        visiting.remove(task_id)
        visited.add(task_id)
        return False

    if any(visit(task_id) for task_id in dependencies if task_id not in visited):
        errors.append("tasks.md contains a dependency cycle")

    def transitively_depends_on(task_id, expected_dependency, seen=None):
        if seen is None:
            seen = set()
        if task_id in seen:
            return False
        seen.add(task_id)
        direct = dependencies.get(task_id, set())
        return expected_dependency in direct or any(
            dependency in leaf_ids
            and transitively_depends_on(dependency, expected_dependency, seen)
            for dependency in direct
        )

    for path, owners in sorted(write_owners.items()):
        unsequenced = []
        for index, owner in enumerate(owners):
            for other in owners[index + 1 :]:
                if not transitively_depends_on(owner, other) and not transitively_depends_on(
                    other, owner
                ):
                    unsequenced.extend((owner, other))
        if unsequenced:
            warnings.append(
                f"sequence overlapping write '{path}' in tasks "
                + ", ".join(sorted(set(unsequenced)))
            )

    return errors, warnings


def run_self_test():
    requirements = """\
# Requirements: Example
### Requirement 1: Store values
1. **1.1** WHEN a value is submitted THEN THE system SHALL store it.
2. **1.2** IF the value is invalid, THEN THE system SHALL reject it.
"""
    design = """\
# Design: Example
## Requirements traceability
| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | Store | Integration test |
| 1.2 | Validate | Error-path test |
"""
    tasks = """\
# Implementation Plan: Example
## Tasks
### Milestone 1: Values are stored safely
- [ ] 1. Store values
  - [ ] 1.1. Store validated values in the existing repository
    - _Requirements: 1.1, 1.2_
    - _Depends on: none_
    - _Reads: src/lib.rs_
    - _Writes: src/lib.rs_
    - _Validation: focused test_
"""
    errors, warnings = validate_contents("example-feature", requirements, design, tasks)
    if errors or warnings:
        raise AssertionError(f"valid fixture failed: errors={errors}, warnings={warnings}")

    broken_tasks = tasks.replace("_Requirements: 1.1, 1.2_", "_Requirements: 9.9_")
    errors, _ = validate_contents("example-feature", requirements, design, broken_tasks)
    if not any("unknown requirements" in error for error in errors):
        raise AssertionError("invalid fixture did not report unknown task requirements")

    broken_tasks = tasks.replace("_Depends on: none_", "_Depends on: 9.1_")
    errors, _ = validate_contents("example-feature", requirements, design, broken_tasks)
    if not any("unknown tasks" in error for error in errors):
        raise AssertionError("invalid fixture did not report unknown dependencies")

    parent_dependency_tasks = tasks.replace("_Depends on: none_", "_Depends on: 1_")
    errors, _ = validate_contents(
        "example-feature", requirements, design, parent_dependency_tasks
    )
    if not any("parent epics instead of leaves" in error for error in errors):
        raise AssertionError("invalid fixture allowed a dependency on an epic")

    duplicate_metadata_tasks = tasks.replace(
        "    - _Validation: focused test_",
        "    - _Validation: focused test_\n    - _Validation: another test_",
    )
    errors, _ = validate_contents(
        "example-feature", requirements, design, duplicate_metadata_tasks
    )
    if not any("duplicate metadata" in error for error in errors):
        raise AssertionError("invalid fixture allowed duplicate leaf metadata")

    epic_leaf = tasks.replace(
        "  - [ ] 1.1. Store validated values in the existing repository\n", ""
    ).replace("    - _", "  - _")
    errors, _ = validate_contents("example-feature", requirements, design, epic_leaf)
    if not any("has no implementation leaves" in error for error in errors):
        raise AssertionError("epic-as-leaf fixture did not require decomposition")
    if not any("metadata must belong to implementation leaves" in error for error in errors):
        raise AssertionError("epic metadata fixture did not reject metadata on an epic")
    if not any("does not cover requirements" in error for error in errors):
        raise AssertionError("epic metadata fixture incorrectly traced requirements to an epic")

    broad_tasks = tasks.replace(
        "_Writes: src/lib.rs_",
        "_Writes: backend/**, web/view.tsx, clients/mobile.kt, migrations/1.sql, "
        "deploy/app.yaml, docs/sharing.md_",
    )
    errors, warnings = validate_contents("example-feature", requirements, design, broad_tasks)
    if errors:
        raise AssertionError(f"warning fixture failed: errors={errors}")
    expected_warnings = ("broad write globs", "6 write targets", "top-level subsystems")
    for expected in expected_warnings:
        if not any(expected in warning for warning in warnings):
            raise AssertionError(f"warning fixture did not report {expected!r}")

    overlapping_tasks = tasks.replace(
        "    - _Validation: focused test_\n",
        "    - _Validation: focused test_\n"
        "  - [ ] 1.2. Reject invalid values at the repository boundary\n"
        "    - _Requirements: 1.2_\n"
        "    - _Depends on: none_\n"
        "    - _Reads: src/lib.rs_\n"
        "    - _Writes: src/lib.rs_\n"
        "    - _Validation: focused rejection test_\n",
    )
    errors, warnings = validate_contents(
        "example-feature", requirements, design, overlapping_tasks
    )
    if errors:
        raise AssertionError(f"overlap fixture failed: errors={errors}")
    if not any("sequence overlapping write" in warning for warning in warnings):
        raise AssertionError("overlap fixture did not report unsequenced writes")

    sequenced_tasks = overlapping_tasks.replace(
        "    - _Depends on: none_\n    - _Reads: src/lib.rs_\n"
        "    - _Writes: src/lib.rs_\n    - _Validation: focused rejection test_",
        "    - _Depends on: 1.1_\n    - _Reads: src/lib.rs_\n"
        "    - _Writes: src/lib.rs_\n    - _Validation: focused rejection test_",
    )
    errors, warnings = validate_contents(
        "example-feature", requirements, design, sequenced_tasks
    )
    if errors or any("sequence overlapping write" in warning for warning in warnings):
        raise AssertionError(
            f"sequenced overlap fixture failed: errors={errors}, warnings={warnings}"
        )

    print("validate_spec self-test passed")


def main():
    parser = argparse.ArgumentParser(description="Validate a feature specification pack")
    parser.add_argument("feature_dir", nargs="?", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        run_self_test()
        return 0
    if args.feature_dir is None:
        parser.error("feature_dir is required unless --self-test is used")

    required_files = ("requirements.md", "design.md", "tasks.md")
    missing_files = [name for name in required_files if not (args.feature_dir / name).is_file()]
    if missing_files:
        print(f"ERROR: missing files: {', '.join(missing_files)}", file=sys.stderr)
        return 1

    requirements, design, tasks = [
        (args.feature_dir / name).read_text(encoding="utf-8") for name in required_files
    ]
    errors, warnings = validate_contents(args.feature_dir.name, requirements, design, tasks)
    for warning in warnings:
        print(f"WARNING: {warning}")
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    if errors:
        return 1

    print(
        "MANUAL REVIEW REQUIRED: confirm every implementation leaf is one focused, "
        "independently reviewable unit"
    )
    print(
        f"Validated {args.feature_dir}: "
        f"{len(REQUIREMENT_ID.findall(requirements))} acceptance criteria, "
        f"{len(task_blocks(tasks))} tasks"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
