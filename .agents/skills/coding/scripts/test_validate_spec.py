#!/usr/bin/env python3

from __future__ import annotations

import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate_spec.py")


REQUIREMENTS = """
# Requirements: Sample

### Requirement 1: Save value

#### Acceptance criteria

1. WHEN submitted THEN THE system SHALL save the value.
2. IF saving fails THEN THE system SHALL report the failure.
"""

DESIGN = """
# Design: Sample

## Decisions

### D1: Store through the repository

- Choice: Reuse the repository abstraction.

## Traceability

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D1 / Repository | Integration | `cargo test -p sample save` passes |
| 1.2 | D1 / Repository | Integration | `cargo test -p sample failure` passes |
"""

TASKS = """
# Implementation plan: Sample

## Tasks

- [ ] 1. Implement storage
  - _id: sample-storage_
  - _priority: P1_
  - _value: high_
  - _wave: 1_
  - _reads: src/repository.rs_
  - _writes: src/storage.rs, tests/storage.rs_
  - _validation: cargo test -p sample storage_
  - _Requirements: 1.1, 1.2_
  - Outcome: Values are saved and failures are visible.
  - Design: D1 / Repository
  - Done when: Focused storage tests pass.
"""


class ValidateSpecTest(unittest.TestCase):
    def create_pack(
        self,
        root: Path,
        requirements: str = REQUIREMENTS,
        design: str | None = DESIGN,
        tasks: str | None = TASKS,
        feature_name: str = "sample",
    ) -> Path:
        spec_dir = root / ".agents" / "specs" / feature_name
        spec_dir.mkdir(parents=True)
        (spec_dir / "requirements.md").write_text(textwrap.dedent(requirements), encoding="utf-8")
        if design is not None:
            (spec_dir / "design.md").write_text(textwrap.dedent(design), encoding="utf-8")
        if tasks is not None:
            (spec_dir / "tasks.md").write_text(textwrap.dedent(tasks), encoding="utf-8")
        return spec_dir

    def run_validator(
        self, spec_dir: Path, require_complete: bool = False
    ) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(VALIDATOR), str(spec_dir)]
        if require_complete:
            command.append("--require-complete")
        return subprocess.run(command, capture_output=True, check=False, text=True)

    def test_accepts_complete_traceable_pack(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory))
            result = self.run_validator(spec_dir, require_complete=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Validated spec pack", result.stdout)

    def test_accepts_requirements_only_pack_but_complete_mode_rejects_it(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), design=None, tasks=None)
            partial = self.run_validator(spec_dir)
            complete = self.run_validator(spec_dir, require_complete=True)

        self.assertEqual(partial.returncode, 0, partial.stderr)
        self.assertEqual(complete.returncode, 1)
        self.assertIn("design.md: required file is missing", complete.stderr)
        self.assertIn("tasks.md: required file is missing", complete.stderr)

    def test_requires_exact_criterion_references(self) -> None:
        tasks = TASKS.replace("1.1, 1.2", "1")
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 1)
        self.assertIn("invalid criterion reference '1'", result.stderr)
        self.assertIn("criterion 1.1 has no coverage", result.stderr)

    def test_numeric_tables_outside_traceability_do_not_count_as_coverage(self) -> None:
        design = """
        # Design: Sample

        ## Decisions

        ### D1: Store through the repository

        ## Status codes

        | Code | Meaning |
        | --- | --- |
        | 200 | Saved |
        """
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), design=design, tasks=None)
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 1)
        self.assertIn("no '## Traceability' section found", result.stderr)
        self.assertNotIn("missing criterion 200", result.stderr)

    def test_ignores_fenced_and_commented_structures(self) -> None:
        requirements = """
        ```markdown
        ### Requirement 1: Fake
        #### Acceptance criteria
        1. Fake criterion.
        ```
        <!--
        ### Requirement 2: Also fake
        #### Acceptance criteria
        1. Fake criterion.
        -->
        """
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(
                Path(temporary_directory), requirements=requirements, design=None, tasks=None
            )
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 1)
        self.assertIn("no '### Requirement N' headings found", result.stderr)

    def test_requires_canonical_task_metadata_and_valid_design_links(self) -> None:
        tasks = TASKS.replace("  - _validation: cargo test -p sample storage_\n", "").replace(
            "D1 / Repository", "D9 / Missing"
        )
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 1)
        self.assertIn("lacks _validation", result.stderr)
        self.assertIn("references missing decision D9", result.stderr)

    def test_detects_dependency_cycles_wave_errors_and_path_conflicts(self) -> None:
        tasks = """
        # Implementation plan: Sample

        ## Tasks

        - [~] 1. First task
          - _id: first-task_
          - _wave: 1_
          - _blocked_by: second-task_
          - _writes: src/shared_
          - _validation: cargo test -p sample first_
          - _Requirements: 1.1_
          - Outcome: First result.
          - Design: D1 / Repository
          - Done when: First test passes.

        - [-] 2. Second task
          - _id: second-task_
          - _wave: 1_
          - _blocked_by: first-task_
          - _reads: src/shared/file.rs_
          - _validation: cargo test -p sample second_
          - _Requirements: 1.2_
          - Outcome: Second result.
          - Design: D1 / Repository
          - Done when: Second test passes.
        """
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 1)
        self.assertIn("must be in an earlier wave", result.stderr)
        self.assertIn("dependency cycle", result.stderr)
        self.assertIn("conflicting paths in wave 1", result.stderr)

    def test_supports_explicit_no_task_coverage(self) -> None:
        tasks = TASKS.replace("1.1, 1.2", "1.1") + "\n- No task: 1.2 — validation-only behavior\n"
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_rejects_nested_tasks_and_invalid_spec_directory_names(self) -> None:
        tasks = TASKS + "\n  - [ ] Nested executable task\n"
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks, feature_name="Bad_Name")
            result = self.run_validator(spec_dir)

        self.assertEqual(result.returncode, 1)
        self.assertIn("feature directory must use kebab-case", result.stderr)
        self.assertIn("executable tasks must be top-level checkboxes", result.stderr)

    def test_accepts_semantic_version_feature_name_segment(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(
                Path(temporary_directory), feature_name="zed-v1.11.3-port"
            )
            result = self.run_validator(spec_dir, require_complete=True)

        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
