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
    r"^(?P<indent>\s*)-\s+\[[ xX]\]\s+(?P<id>\d+(?:\.\d+)?)\.\s+",
    re.MULTILINE,
)
KEBAB_CASE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


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


def metadata(block, label):
    match = re.search(rf"_{re.escape(label)}:\s*(.*?)_", block, re.IGNORECASE)
    return match.group(1).strip() if match else None


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

    blocks = task_blocks(tasks)
    task_ids = [task_id for task_id, _ in blocks]
    known_task_ids = set(task_ids)
    duplicate_tasks = sorted(item for item, count in Counter(task_ids).items() if count > 1)
    if duplicate_tasks:
        errors.append(f"duplicate task IDs: {', '.join(duplicate_tasks)}")
    if not blocks:
        errors.append("tasks.md contains no checkbox tasks with numeric IDs")

    leaf_blocks = [
        (task_id, block)
        for task_id, block in blocks
        if not any(other.startswith(f"{task_id}.") for other in task_ids)
    ]
    task_requirement_ids = set()
    dependencies = {}
    write_owners = defaultdict(list)
    required_metadata = ("Requirements", "Depends on", "Reads", "Writes", "Validation")

    for task_id, block in leaf_blocks:
        values = {label: metadata(block, label) for label in required_metadata}
        missing = [label for label, value in values.items() if value is None]
        if missing:
            errors.append(f"task {task_id} is missing metadata: {', '.join(missing)}")
            continue

        task_requirement_ids.update(referenced_ids(values["Requirements"]))
        dependency_ids = set(TASK_ID_REFERENCE.findall(values["Depends on"]))
        dependencies[task_id] = dependency_ids
        unknown_dependencies = sorted(dependency_ids - known_task_ids)
        if unknown_dependencies:
            errors.append(
                f"task {task_id} depends on unknown tasks: {', '.join(unknown_dependencies)}"
            )
        if task_id in dependency_ids:
            errors.append(f"task {task_id} depends on itself")
        writes = [item.strip() for item in values["Writes"].split(",")]
        for path in writes:
            if path.lower() not in {"", "none", "n/a"}:
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

    for path, owners in sorted(write_owners.items()):
        if len(owners) > 1:
            warnings.append(
                f"review sequencing for repeated write '{path}' in tasks {', '.join(owners)}"
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
- [ ] 1. Implement storage
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

    broken_tasks = tasks.replace("_Depends on: none_", "_Depends on: 9_")
    errors, _ = validate_contents("example-feature", requirements, design, broken_tasks)
    if not any("unknown tasks" in error for error in errors):
        raise AssertionError("invalid fixture did not report unknown dependencies")

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
        f"Validated {args.feature_dir}: "
        f"{len(REQUIREMENT_ID.findall(requirements))} acceptance criteria, "
        f"{len(task_blocks(tasks))} tasks"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
