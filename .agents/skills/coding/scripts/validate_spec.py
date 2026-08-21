#!/usr/bin/env python3

from __future__ import annotations

import argparse
import posixpath
import re
import sys
from dataclasses import dataclass
from pathlib import Path


REQUIREMENT_HEADING = re.compile(r"^### Requirement\s+([0-9]+)\b")
ACCEPTANCE_CRITERION = re.compile(r"^\s*([0-9]+)\.\s+\S")
DECISION_HEADING = re.compile(r"^###\s+(D[0-9]+):\s+\S")
PROPERTY_HEADING = re.compile(r"^### Property\s+([0-9]+)\b")
PROPERTY_VALIDATES = re.compile(r"Validates:\s*Requirement\s+([0-9]+\.[0-9]+)\b")
CRITERION_ID = re.compile(r"^[0-9]+\.[0-9]+$")
DECISION_IN_TEXT = re.compile(r"\bD[0-9]+\b")
TASK_HEADING = re.compile(r"^- \[([ xX~-])\]\s+(?:(\d+(?:\.\d+)*)[.)]\s+)?(.+)$")
NESTED_TASK = re.compile(r"^\s+- \[[ xX~-]\]\s+")
TASK_ID = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
FEATURE_NAME_SEGMENT = r"(?:[a-z0-9]+|v[0-9]+(?:\.[0-9]+)+)"
FEATURE_NAME = re.compile(rf"^{FEATURE_NAME_SEGMENT}(?:-{FEATURE_NAME_SEGMENT})*$")
NO_TASK = re.compile(r"^\s*-\s+No task:\s+([0-9]+\.[0-9]+)\s+(?:-|—)\s+\S", re.IGNORECASE)
PLACEHOLDERS = {"none", "tbd", "todo", "unknown", "n/a"}


@dataclass
class Task:
    title: str
    marker: str
    line: int
    body: list[str]
    metadata: dict[str, str]

    @property
    def task_id(self) -> str | None:
        return self.metadata.get("_id")


@dataclass
class DesignCoverage:
    criteria: set[str]
    decisions: set[str]


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate a .agents/specs spec pack")
    parser.add_argument("spec_dir", type=Path)
    parser.add_argument(
        "--require-complete",
        action="store_true",
        help="require requirements.md, design.md, and tasks.md",
    )
    return parser.parse_args()


def read_file(path: Path, errors: list[str]) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        errors.append(f"{path}: cannot read file: {error}")
        return ""


def structural_lines(text: str) -> list[str]:
    result: list[str] = []
    in_fence = False
    in_comment = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            in_fence = not in_fence
            result.append("")
            continue
        if in_fence:
            result.append("")
            continue
        visible = line
        while True:
            if in_comment:
                end = visible.find("-->")
                if end == -1:
                    visible = ""
                    break
                visible = visible[end + 3 :]
                in_comment = False
                continue
            start = visible.find("<!--")
            if start == -1:
                break
            end = visible.find("-->", start + 4)
            if end == -1:
                visible = visible[:start]
                in_comment = True
                break
            visible = visible[:start] + visible[end + 3 :]
        result.append(visible)
    return result


def parse_requirements(text: str, errors: list[str]) -> set[str]:
    headings: set[str] = set()
    criteria: set[str] = set()
    criteria_by_heading: dict[str, set[str]] = {}
    current_heading: str | None = None
    in_acceptance_criteria = False

    for line_number, line in enumerate(structural_lines(text), 1):
        heading_match = REQUIREMENT_HEADING.match(line)
        if heading_match:
            current_heading = heading_match.group(1)
            if current_heading in headings:
                errors.append(f"requirements.md:{line_number}: duplicate requirement {current_heading}")
            headings.add(current_heading)
            criteria_by_heading.setdefault(current_heading, set())
            in_acceptance_criteria = False
            continue

        if line.casefold() == "#### acceptance criteria":
            in_acceptance_criteria = True
            continue
        if line.startswith("#"):
            in_acceptance_criteria = False

        if current_heading and in_acceptance_criteria:
            criterion_match = ACCEPTANCE_CRITERION.match(line)
            if criterion_match:
                identifier = f"{current_heading}.{criterion_match.group(1)}"
                if identifier in criteria:
                    errors.append(f"requirements.md:{line_number}: duplicate criterion {identifier}")
                criteria.add(identifier)
                criteria_by_heading[current_heading].add(identifier)

    if not headings:
        errors.append("requirements.md: no '### Requirement N' headings found")
    for heading in sorted(headings, key=int):
        if not criteria_by_heading[heading]:
            errors.append(f"requirements.md: requirement {heading} has no acceptance criteria")
    return criteria


def split_table_row(line: str) -> list[str] | None:
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        return None
    return [cell.strip() for cell in stripped[1:-1].split("|")]


def is_separator_row(cells: list[str]) -> bool:
    return bool(cells) and all(re.fullmatch(r":?-{3,}:?", cell) for cell in cells)


def parse_design(text: str, requirements: set[str], errors: list[str]) -> DesignCoverage:
    lines = structural_lines(text)
    decisions: set[str] = set()
    properties: set[str] = set()
    property_validation: dict[str, bool] = {}
    current_property: str | None = None
    in_traceability = False
    saw_traceability = False
    saw_header = False
    coverage: set[str] = set()

    for line_number, line in enumerate(lines, 1):
        decision_match = DECISION_HEADING.match(line)
        if decision_match:
            identifier = decision_match.group(1)
            if identifier in decisions:
                errors.append(f"design.md:{line_number}: duplicate decision {identifier}")
            decisions.add(identifier)

        property_match = PROPERTY_HEADING.match(line)
        if property_match:
            current_property = property_match.group(1)
            if current_property in properties:
                errors.append(f"design.md:{line_number}: duplicate property {current_property}")
            properties.add(current_property)
            property_validation[current_property] = False
        elif line.startswith("### ") or line.startswith("## "):
            current_property = None

        validation_match = PROPERTY_VALIDATES.search(line)
        if validation_match and current_property:
            criterion = validation_match.group(1)
            property_validation[current_property] = True
            if criterion not in requirements:
                errors.append(f"design.md:{line_number}: property references missing criterion {criterion}")

        if line.casefold() == "## traceability":
            in_traceability = True
            saw_traceability = True
            saw_header = False
            continue
        if in_traceability and line.startswith("## "):
            in_traceability = False
        if not in_traceability:
            continue

        cells = split_table_row(line)
        if cells is None or is_separator_row(cells):
            continue
        if not saw_header:
            expected = ["criterion", "design coverage", "verification type", "planned check / expected signal"]
            if [cell.casefold() for cell in cells] != expected:
                errors.append(f"design.md:{line_number}: Traceability table has unexpected columns")
            saw_header = True
            continue
        if len(cells) != 4:
            errors.append(f"design.md:{line_number}: Traceability row must contain four columns")
            continue
        criterion, design_mapping, verification_type, planned_check = cells
        if not CRITERION_ID.fullmatch(criterion):
            errors.append(f"design.md:{line_number}: invalid criterion reference {criterion!r}")
            continue
        coverage.add(criterion)
        if criterion not in requirements:
            errors.append(f"design.md:{line_number}: references missing criterion {criterion}")
        mapped_decisions = set(DECISION_IN_TEXT.findall(design_mapping))
        if not mapped_decisions:
            errors.append(f"design.md:{line_number}: criterion {criterion} lacks a design decision reference")
        for decision in sorted(mapped_decisions - decisions):
            errors.append(f"design.md:{line_number}: references missing decision {decision}")
        if not verification_type or verification_type.casefold() in PLACEHOLDERS:
            errors.append(f"design.md:{line_number}: criterion {criterion} lacks a verification type")
        if not planned_check or planned_check.casefold() in PLACEHOLDERS:
            errors.append(f"design.md:{line_number}: criterion {criterion} lacks a planned check")

    if not saw_traceability:
        errors.append("design.md: no '## Traceability' section found")
    elif not saw_header:
        errors.append("design.md: Traceability table is missing")
    for identifier, validated in property_validation.items():
        if not validated:
            errors.append(f"design.md: property {identifier} has no Validates reference")
    validate_coverage("design.md", coverage, requirements, errors)
    return DesignCoverage(coverage, decisions)


def metadata_from_body(body: list[str], line: int, errors: list[str]) -> dict[str, str]:
    metadata: dict[str, str] = {}
    pattern = re.compile(r"^\s*-\s+(_?[A-Za-z][A-Za-z_ ]*):\s*(.*?)\s*$")
    for offset, source in enumerate(body, 1):
        match = pattern.match(source)
        if not match:
            continue
        key = match.group(1)
        canonical = key.casefold()
        value = match.group(2).strip().strip("_").strip()
        if canonical in metadata:
            errors.append(f"tasks.md:{line + offset}: duplicate metadata {key}")
        metadata[canonical] = value
    return metadata


def parse_tasks(text: str, errors: list[str]) -> tuple[list[Task], set[str]]:
    lines = structural_lines(text)
    starts: list[tuple[int, re.Match[str]]] = []
    exemptions: set[str] = set()
    for index, line in enumerate(lines):
        if NESTED_TASK.match(line):
            errors.append(f"tasks.md:{index + 1}: executable tasks must be top-level checkboxes")
        match = TASK_HEADING.match(line)
        if match:
            starts.append((index, match))
        exemption = NO_TASK.match(line)
        if exemption:
            exemptions.add(exemption.group(1))

    tasks: list[Task] = []
    for position, (start, match) in enumerate(starts):
        end = starts[position + 1][0] if position + 1 < len(starts) else len(lines)
        for candidate in range(start + 1, end):
            if lines[candidate].strip() and not lines[candidate].startswith((" ", "\t")):
                end = candidate
                break
        body = lines[start + 1 : end]
        metadata = metadata_from_body(body, start + 1, errors)
        tasks.append(Task(match.group(3).strip(), match.group(1), start + 1, body, metadata))
    return tasks, exemptions


def comma_values(value: str | None) -> list[str]:
    if not value:
        return []
    return [item.strip().strip("`") for item in value.split(",") if item.strip()]


def exact_criterion_references(value: str, source: str, errors: list[str]) -> set[str]:
    references: set[str] = set()
    for item in comma_values(value):
        if not CRITERION_ID.fullmatch(item):
            errors.append(f"{source}: invalid criterion reference {item!r}; use exact IDs such as 1.2")
        else:
            references.add(item)
    return references


def normalized_paths(value: str | None, source: str, errors: list[str]) -> list[str]:
    paths: list[str] = []
    for item in comma_values(value):
        normalized = posixpath.normpath(item)
        if item.startswith("/") or normalized == ".." or normalized.startswith("../"):
            errors.append(f"{source}: path must be repository-relative: {item!r}")
            continue
        paths.append(normalized.removeprefix("./"))
    return paths


def paths_overlap(left: str, right: str) -> bool:
    left = left.rstrip("/")
    right = right.rstrip("/")
    return left == right or left.startswith(f"{right}/") or right.startswith(f"{left}/")


def validate_task_graph(tasks: list[Task], errors: list[str]) -> None:
    tasks_by_id = {task.task_id: task for task in tasks if task.task_id}
    dependencies: dict[str, list[str]] = {}
    for task in tasks:
        if not task.task_id:
            continue
        task_dependencies = comma_values(task.metadata.get("_blocked_by"))
        dependencies[task.task_id] = task_dependencies
        for dependency in task_dependencies:
            if dependency not in tasks_by_id:
                errors.append(f"tasks.md:{task.line}: task {task.task_id} depends on missing task {dependency}")
            if dependency == task.task_id:
                errors.append(f"tasks.md:{task.line}: task {task.task_id} depends on itself")
            dependency_task = tasks_by_id.get(dependency)
            task_wave = task.metadata.get("_wave")
            dependency_wave = dependency_task.metadata.get("_wave") if dependency_task else None
            if (
                task_wave
                and dependency_wave
                and task_wave.isdigit()
                and dependency_wave.isdigit()
                and int(dependency_wave) >= int(task_wave)
            ):
                errors.append(
                    f"tasks.md:{task.line}: dependency {dependency} must be in an earlier wave than {task.task_id}"
                )

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(identifier: str, path: list[str]) -> None:
        if identifier in visited:
            return
        if identifier in visiting:
            cycle = " -> ".join(path[path.index(identifier) :] + [identifier])
            errors.append(f"tasks.md: dependency cycle: {cycle}")
            return
        visiting.add(identifier)
        for dependency in dependencies.get(identifier, []):
            if dependency in dependencies:
                visit(dependency, path + [dependency])
        visiting.remove(identifier)
        visited.add(identifier)

    for identifier in dependencies:
        visit(identifier, [identifier])


def validate_task_conflicts(tasks: list[Task], errors: list[str]) -> None:
    for index, left in enumerate(tasks):
        left_wave = left.metadata.get("_wave")
        if not left_wave:
            continue
        left_reads = normalized_paths(left.metadata.get("_reads"), f"tasks.md:{left.line}", errors)
        left_writes = normalized_paths(left.metadata.get("_writes"), f"tasks.md:{left.line}", errors)
        for right in tasks[index + 1 :]:
            if right.metadata.get("_wave") != left_wave:
                continue
            right_reads = normalized_paths(right.metadata.get("_reads"), f"tasks.md:{right.line}", errors)
            right_writes = normalized_paths(right.metadata.get("_writes"), f"tasks.md:{right.line}", errors)
            conflict = any(paths_overlap(a, b) for a in left_writes for b in right_writes + right_reads)
            conflict = conflict or any(paths_overlap(a, b) for a in right_writes for b in left_reads)
            if conflict:
                errors.append(
                    f"tasks.md:{right.line}: tasks {left.task_id or left.title!r} and "
                    f"{right.task_id or right.title!r} have conflicting paths in wave {left_wave}"
                )


def validate_tasks(
    text: str,
    requirements: set[str],
    decisions: set[str],
    errors: list[str],
) -> set[str]:
    tasks, exemptions = parse_tasks(text, errors)
    if not tasks:
        errors.append("tasks.md: no top-level checkbox tasks found")
        return exemptions

    task_ids: set[str] = set()
    task_references: set[str] = set(exemptions)
    required_fields = ("_id", "_validation", "_requirements", "outcome", "design", "done when")

    for task in tasks:
        for field in required_fields:
            value = task.metadata.get(field)
            if not value:
                errors.append(f"tasks.md:{task.line}: task {task.title!r} lacks {field}")
            elif value.casefold() in PLACEHOLDERS:
                errors.append(f"tasks.md:{task.line}: task {task.title!r} has placeholder {field}")

        task_id = task.task_id
        if task_id:
            if not TASK_ID.fullmatch(task_id):
                errors.append(f"tasks.md:{task.line}: invalid durable task ID {task_id!r}")
            if task_id in task_ids:
                errors.append(f"tasks.md:{task.line}: duplicate durable task ID {task_id}")
            task_ids.add(task_id)

        references = exact_criterion_references(
            task.metadata.get("_requirements") or "", f"tasks.md:{task.line}", errors
        )
        task_references.update(references)
        for reference in sorted(references - requirements):
            errors.append(f"tasks.md:{task.line}: references missing criterion {reference}")

        design_value = task.metadata.get("design") or ""
        design_references = set(DECISION_IN_TEXT.findall(design_value))
        if design_value and not design_references:
            errors.append(f"tasks.md:{task.line}: task {task.title!r} lacks a design decision reference")
        for decision in sorted(design_references - decisions):
            errors.append(f"tasks.md:{task.line}: references missing decision {decision}")

        wave = task.metadata.get("_wave")
        if wave and (not wave.isdigit() or int(wave) < 1):
            errors.append(f"tasks.md:{task.line}: _wave must be a positive integer")
        normalized_paths(task.metadata.get("_reads"), f"tasks.md:{task.line}", errors)
        normalized_paths(task.metadata.get("_writes"), f"tasks.md:{task.line}", errors)

    for exemption in sorted(exemptions - requirements):
        errors.append(f"tasks.md: no-task entry references missing criterion {exemption}")
    validate_coverage("tasks.md", task_references, requirements, errors)
    validate_task_graph(tasks, errors)
    validate_task_conflicts(tasks, errors)
    return task_references


def validate_coverage(source: str, references: set[str], requirements: set[str], errors: list[str]) -> None:
    for requirement in sorted(requirements - references):
        errors.append(f"{source}: criterion {requirement} has no coverage")


def validate_spec_directory(spec_dir: Path, errors: list[str]) -> None:
    if spec_dir.parent.name != "specs" or spec_dir.parent.parent.name != ".agents":
        errors.append(f"{spec_dir}: spec pack must be an immediate child of .agents/specs/")
    if not FEATURE_NAME.fullmatch(spec_dir.name):
        errors.append(
            f"{spec_dir}: feature directory must use kebab-case, with optional semantic-version segments"
        )


def main() -> int:
    arguments = parse_arguments()
    spec_dir = arguments.spec_dir
    errors: list[str] = []
    validate_spec_directory(spec_dir, errors)

    paths = {name: spec_dir / name for name in ("requirements.md", "design.md", "tasks.md")}
    existing_paths = {name: path for name, path in paths.items() if path.is_file()}
    if arguments.require_complete:
        for name, path in paths.items():
            if name not in existing_paths:
                errors.append(f"{path}: required file is missing")
    elif not existing_paths:
        errors.append(f"{spec_dir}: no spec documents found")

    texts = {name: read_file(path, errors) for name, path in existing_paths.items()}
    requirements = parse_requirements(texts["requirements.md"], errors) if "requirements.md" in texts else set()
    if not requirements and ("design.md" in texts or "tasks.md" in texts):
        errors.append("requirements.md: required to validate design or task references")

    design = DesignCoverage(set(), set())
    if "design.md" in texts:
        design = parse_design(texts["design.md"], requirements, errors)
    if "tasks.md" in texts:
        if "design.md" not in texts:
            errors.append("design.md: required to validate executable tasks")
        validate_tasks(texts["tasks.md"], requirements, design.decisions, errors)

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"Validated spec pack: {spec_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
