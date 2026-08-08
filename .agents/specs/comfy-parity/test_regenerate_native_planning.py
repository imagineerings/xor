#!/usr/bin/env python3

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import regenerate_native_planning as planning


class ValidationGenerationTests(unittest.TestCase):
    def test_catalog_pass_signal_is_command_only_and_other_artifact_classes_remain(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(planning, "ROOT", root):
                planning.write_validation()
            lines = (root / "validation.md").read_text(encoding="utf-8").splitlines()

        scenarios: dict[str, list[str]] = {}
        current: str | None = None
        for line in lines:
            if line.startswith("### VAL-"):
                current = line.removeprefix("### ").split(":", 1)[0]
                scenarios[current] = [line]
            elif current is not None:
                scenarios[current].append(line)

        self.assertEqual(set(scenarios), set(planning.VALIDATIONS))

        def scenario_line(identifier: str, prefix: str) -> str:
            matches = [line for line in scenarios[identifier] if line.startswith(prefix)]
            self.assertEqual(len(matches), 1, identifier)
            return matches[0]

        catalog = "\n".join(scenarios["VAL-CATALOG-001"])
        self.assertEqual(
            scenario_line("VAL-CATALOG-001", "- Command/runner: "),
            "- Command/runner: `python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice`.",
        )
        self.assertEqual(
            scenario_line("VAL-CATALOG-001", "- Pass artifact: "),
            "- Pass artifact: exit status 0 from the exact runner after the "
            "source-snapshot manifest matches and both complete regeneration passes "
            "produce no changed paths. The checked-in generated outputs and command "
            "result are the evidence; this command-only gate emits no separate target "
            "JSON artifact.",
        )
        self.assertNotIn("target/comfy-parity/val-catalog-001.json", catalog)
        command_only = [
            identifier
            for identifier in planning.VALIDATIONS
            if "command-only gate emits no separate target JSON artifact"
            in scenario_line(identifier, "- Pass artifact: ")
        ]
        self.assertEqual(command_only, ["VAL-CATALOG-001"])

        generic = scenario_line("VAL-CANCEL-001", "- Pass artifact: ")
        self.assertIn("target/comfy-parity/val-cancel-001.json", generic)
        self.assertIn("fixture digests", generic)

        cumulative = scenario_line("VAL-CLIP-001", "- Pass artifact: ")
        self.assertIn("target/comfy-parity/val-clip-001.json", cumulative)
        self.assertIn("using schema version 1", cumulative)
        self.assertIn("partial artifacts claim only their exact passed rows", cumulative)

        device = scenario_line("VAL-DEVICE-001", "- Pass artifact: ")
        self.assertIn("target/comfy-parity/val-device-001.json", device)
        self.assertIn("Apple Metal baseline retains its signed artifact", device)

        model_family = scenario_line(
            "VAL-MODEL-FAMILY-ROW-001", "- Pass artifact: "
        )
        self.assertIn("target/comfy-parity/val-model-family-row-001/", model_family)
        self.assertIn("one deterministic artifact per executed fixture", model_family)


if __name__ == "__main__":
    unittest.main()
