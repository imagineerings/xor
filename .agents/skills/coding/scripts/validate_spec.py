#!/usr/bin/env python3

from pathlib import Path
import runpy


canonical_validator = (
    Path(__file__).resolve().parents[2]
    / "feature-spec"
    / "scripts"
    / "validate_spec.py"
)
runpy.run_path(str(canonical_validator), run_name="__main__")
