#!/usr/bin/env python3

from __future__ import annotations

import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate_spec.py")

CANONICAL_REQUIREMENTS = """
# Requirements: Sample

### Requirement 1: Save values

#### Acceptance criteria

1. **1.1** WHEN submitted THEN THE system SHALL save the value.
2. **1.2** IF saving fails THEN THE system SHALL report the failure.
"""

CANONICAL_DESIGN = """
# Design: Sample

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | Existing repository | Integration test |
| 1.2 | Existing repository | Error-path test |
"""

CANONICAL_TASKS = """
# Implementation Plan: Sample

## Tasks

### Milestone 1: Values are stored safely

- [ ] 1. Store values
  - [ ] 1.1. Store validated values in the existing repository
    - _Requirements: 1.1, 1.2_
    - _Depends on: none_
    - _Reads: src/repository.rs_
    - _Writes: src/storage.rs, tests/storage.rs_
    - _Validation: cargo test -p sample storage_
"""

LEGACY_REQUIREMENTS = """
# Requirements: Sample

### Requirement 1: Save values

#### Acceptance criteria

1. WHEN submitted THEN THE system SHALL save the value.
2. IF saving fails THEN THE system SHALL report the failure.
"""

LEGACY_DESIGN = """
# Design: Sample

## Decisions

### D1: Store through the repository

- Choice: Reuse the repository abstraction.

## Traceability

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D1 / Repository | Integration | storage test passes |
| 1.2 | D1 / Repository | Error path | failure test passes |
"""

LEGACY_TASKS = """
# Implementation plan: Sample

## Tasks

- [ ] 1. Implement storage
  - _id: sample-storage_
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
        requirements: str = CANONICAL_REQUIREMENTS,
        design: str | None = CANONICAL_DESIGN,
        tasks: str | None = CANONICAL_TASKS,
        feature_name: str = "sample",
    ) -> Path:
        spec_dir = root / ".agents" / "specs" / feature_name
        spec_dir.mkdir(parents=True)
        (spec_dir / "requirements.md").write_text(
            textwrap.dedent(requirements), encoding="utf-8"
        )
        if design is not None:
            (spec_dir / "design.md").write_text(
                textwrap.dedent(design), encoding="utf-8"
            )
        if tasks is not None:
            (spec_dir / "tasks.md").write_text(
                textwrap.dedent(tasks), encoding="utf-8"
            )
        return spec_dir

    def run_validator(self, spec_dir: Path, *arguments: str):
        return subprocess.run(
            [sys.executable, str(VALIDATOR), str(spec_dir), *arguments],
            capture_output=True,
            check=False,
            text=True,
        )

    def test_accepts_complete_canonical_pack(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory))
            result = self.run_validator(
                spec_dir, "--require-complete", "--dialect", "canonical"
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("(canonical dialect)", result.stdout)

    def test_auto_detects_valid_legacy_pack(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(
                Path(temporary_directory),
                requirements=LEGACY_REQUIREMENTS,
                design=LEGACY_DESIGN,
                tasks=LEGACY_TASKS,
            )
            result = self.run_validator(spec_dir, "--require-complete")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("legacy coding task dialect detected", result.stdout)
        self.assertIn("(legacy dialect)", result.stdout)

    def test_canonical_mode_rejects_legacy_packet(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(
                Path(temporary_directory),
                requirements=LEGACY_REQUIREMENTS,
                design=LEGACY_DESIGN,
                tasks=LEGACY_TASKS,
            )
            result = self.run_validator(spec_dir, "--dialect", "canonical")

        self.assertEqual(result.returncode, 1)
        self.assertIn("explicit acceptance criterion IDs", result.stderr)
        self.assertIn("has no implementation leaves", result.stderr)

    def test_canonical_mode_rejects_lowercase_metadata(self):
        tasks = CANONICAL_TASKS.replace("_Reads:", "_reads:")
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir, "--dialect", "canonical")

        self.assertEqual(result.returncode, 1)
        self.assertIn("exact canonical capitalization", result.stderr)

    def test_partial_pack_and_complete_gate(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(
                Path(temporary_directory), design=None, tasks=None
            )
            partial = self.run_validator(spec_dir, "--dialect", "canonical")
            complete = self.run_validator(
                spec_dir, "--require-complete", "--dialect", "canonical"
            )

        self.assertEqual(partial.returncode, 0, partial.stderr)
        self.assertEqual(complete.returncode, 1)
        self.assertIn("missing files: design.md, tasks.md", complete.stderr)

    def test_completed_task_without_evidence_is_compatible_warning(self):
        tasks = CANONICAL_TASKS.replace("- [ ] 1. Store", "- [x] 1. Store").replace(
            "  - [ ] 1.1.", "  - [x] 1.1."
        )
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir, "--dialect", "canonical")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("without _Evidence", result.stdout)

    def test_completed_task_with_evidence_has_no_evidence_warning(self):
        tasks = CANONICAL_TASKS.replace("- [ ] 1. Store", "- [x] 1. Store").replace(
            "  - [ ] 1.1.", "  - [x] 1.1."
        ).replace(
            "    - _Validation: cargo test -p sample storage_",
            "    - _Validation: cargo test -p sample storage_\n"
            "    - _Evidence: storage tests passed_",
        )
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir, "--dialect", "canonical")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("without _Evidence", result.stdout)

    def test_rejects_parent_epic_dependency_and_unknown_requirement(self):
        tasks = CANONICAL_TASKS.replace("_Depends on: none_", "_Depends on: 1_").replace(
            "_Requirements: 1.1, 1.2_", "_Requirements: 9.9_"
        )
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(Path(temporary_directory), tasks=tasks)
            result = self.run_validator(spec_dir, "--dialect", "canonical")

        self.assertEqual(result.returncode, 1)
        self.assertIn("parent epics instead of leaves", result.stderr)
        self.assertIn("unknown requirements", result.stderr)

    def test_accepts_semantic_version_feature_name(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            spec_dir = self.create_pack(
                Path(temporary_directory), feature_name="zed-v1.11.3-port"
            )
            result = self.run_validator(spec_dir, "--dialect", "canonical")

        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
