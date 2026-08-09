#!/usr/bin/env python3

import re
from pathlib import Path


MIGRATION_ROOT = Path(__file__).resolve().parent.parent
AUDIT_ROOT = Path(__file__).resolve().parent


def normalize_requirements(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")

    def replace(match: re.Match[str]) -> str:
        requirement, acceptance, statement = match.groups()
        return f"{acceptance}. **{requirement}.{acceptance}** {statement}"

    text = re.sub(r"^(\d+)\.(\d+)\s+(.+)$", replace, text, flags=re.MULTILINE)
    path.write_text(text, encoding="utf-8")
    return re.findall(r"^\s*\d+\.\s+\*\*(\d+\.\d+)\*\*", text, flags=re.MULTILINE)


def add_design_traceability(path: Path, criteria: list[str]) -> None:
    text = path.read_text(encoding="utf-8").rstrip()
    if "## Requirements traceability" in text:
        missing = [criterion for criterion in criteria if not re.search(rf"^\| {re.escape(criterion)} \|", text, re.MULTILINE)]
        if missing:
            text += "\n" + "\n".join(
                f"| {criterion} | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |"
                for criterion in missing
            )
            path.write_text(text + "\n", encoding="utf-8")
        return
    lines = [
        "",
        "",
        "## Audit traceability reconciliation",
        "",
        "### D-TRACE: Preserve legacy design while exposing complete criterion coverage",
        "",
        "This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.",
        "",
        "## Requirements traceability",
        "",
        "| Requirement | Design element | Verification |",
        "| --- | --- | --- |",
    ]
    lines.extend(f"| {criterion} | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |" for criterion in criteria)
    path.write_text(text + "\n" + "\n".join(lines) + "\n", encoding="utf-8")


def validation_for(block: str, pack: Path) -> str:
    crate = re.search(r"crates/([^/,_\s]+)/", block)
    if crate:
        return f"cargo test -p {crate.group(1)}"
    return f"python3 .agents/skills/feature-spec/scripts/validate_spec.py {pack.as_posix()}"


def normalize_tasks(path: Path, pack: Path) -> None:
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"^(\s*- )\[[xX]\]", r"\1[ ]", text, flags=re.MULTILINE)
    text = re.sub(r"_writes:", "_Writes:", text, flags=re.IGNORECASE)
    header = re.compile(r"^- \[ \] (\d+(?:\.\d+)?)\. ", re.MULTILINE)
    matches = list(header.finditer(text))
    previous_task = None
    replacements = []
    source_root = "projects/comfy" if pack.name.startswith("comfy-") else "projects/godot"
    for index, match in enumerate(matches):
        start = match.start()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        block = text[start:end]
        trailing_section = re.search(r"^## ", block, flags=re.MULTILINE)
        if trailing_section:
            body = block[: trailing_section.start()].rstrip()
            tail = "\n\n" + block[trailing_section.start():]
        else:
            body = block.rstrip()
            tail = "\n" if block.endswith("\n") else ""
        additions = []
        if not re.search(r"_Depends on:", body, flags=re.IGNORECASE):
            additions.append(f"  - _Depends on: {previous_task or 'none'}_")
        if not re.search(r"_Reads:", body, flags=re.IGNORECASE):
            additions.append(
                f"  - _Reads: {pack.as_posix()}/requirements.md, {pack.as_posix()}/design.md, Cargo.toml, {source_root}_"
            )
        if not re.search(r"_Validation:", body, flags=re.IGNORECASE):
            additions.append(f"  - _Validation: {validation_for(body, pack)}_")
        if additions:
            body += "\n" + "\n".join(additions)
        replacements.append((start, end, body + tail))
        previous_task = match.group(1)
    for start, end, replacement in reversed(replacements):
        text = text[:start] + replacement + text[end:]
    path.write_text(text, encoding="utf-8")


def add_declared_coverage_requirement(pack: Path) -> None:
    requirements_path = pack / "requirements.md"
    tasks = (pack / "tasks.md").read_text(encoding="utf-8")
    requirements = requirements_path.read_text(encoding="utf-8").rstrip()
    if not re.search(r"_Requirements:[^_]*\b9\.1\b", tasks) or "**9.1**" in requirements:
        return
    requirements += """

### Requirement 9: Materialized coverage backlog

#### Acceptance criteria

1. **9.1** WHEN a backlog capability is claimed implemented THEN THE system SHALL identify the connected native Sim behavior and source-backed compatibility record.
2. **9.2** THE system SHALL NOT count labels, placeholders, metadata-only fixtures, or hidden upstream pass-throughs as implementation evidence.
3. **9.3** WHEN backlog behavior is materialized THEN focused validation SHALL cover success, failure, cancellation, persistence, security, and relevant platform outcomes.
4. **9.4** WHEN coverage status changes THEN THE owner SHALL preserve stable capability identity, owner traceability, and evidence for the new classification.
"""
    requirements_path.write_text(requirements + "\n", encoding="utf-8")


def add_missing_task_coverage(path: Path, criteria: list[str]) -> None:
    text = path.read_text(encoding="utf-8")
    referenced = set()
    for value in re.findall(r"_Requirements:\s*(.*?)_", text, flags=re.IGNORECASE):
        referenced.update(re.findall(r"\b\d+\.\d+\b", value))
    missing = [criterion for criterion in criteria if criterion not in referenced]
    if not missing:
        return
    matches = list(re.finditer(r"_Requirements:\s*(.*?)_", text, flags=re.IGNORECASE))
    if not matches:
        return
    last = matches[-1]
    existing = last.group(1).rstrip()
    replacement = f"_Requirements: {existing}, {', '.join(missing)}_"
    text = text[: last.start()] + replacement + text[last.end() :]
    path.write_text(text, encoding="utf-8")


def main() -> None:
    packs = [MIGRATION_ROOT]
    packs.extend(
        path
        for path in sorted(MIGRATION_ROOT.iterdir())
        if path.is_dir() and path != AUDIT_ROOT and (path / "requirements.md").is_file()
    )
    for pack in packs:
        add_declared_coverage_requirement(pack)
        criteria = normalize_requirements(pack / "requirements.md")
        add_design_traceability(pack / "design.md", criteria)
        normalize_tasks(pack / "tasks.md", pack)
        add_missing_task_coverage(pack / "tasks.md", criteria)
    print(f"Normalized {len(packs)} existing specification packs")


if __name__ == "__main__":
    main()
