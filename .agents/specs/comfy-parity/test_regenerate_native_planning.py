#!/usr/bin/env python3

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import regenerate_native_planning as planning


class ValidationGenerationTests(unittest.TestCase):
    def test_native_node_runtime_foundation_orders_disjoint_leaves_and_registry(self) -> None:
        tasks, _ = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        foundation_id = "comfy-parity-native-node-runtime-foundation"
        compute_id = "comfy-parity-native-compute-breadth-integration"
        registry_id = "comfy-parity-native-registry-integration"
        node_ids = sorted(
            identifier
            for identifier in tasks_by_id
            if identifier.startswith("comfy-parity-native-nodes-")
        )

        self.assertEqual(len(tasks), 517)
        self.assertEqual(len(node_ids), 102)
        self.assertEqual(tasks_by_id[foundation_id]["dependencies"], [compute_id])
        self.assertTrue(
            all(foundation_id in tasks_by_id[identifier]["dependencies"] for identifier in node_ids)
        )
        self.assertEqual(sorted(tasks_by_id[registry_id]["dependencies"]), node_ids)

        waves = planning.task_waves(tasks)
        self.assertEqual(waves[foundation_id], waves[compute_id] + 1)
        self.assertTrue(
            all(waves[identifier] == waves[foundation_id] + 1 for identifier in node_ids)
        )
        self.assertEqual(waves[registry_id], waves[foundation_id] + 2)

    def test_native_node_foundation_and_registry_own_runtime_reachability_paths(self) -> None:
        tasks, _ = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        foundation_writes = set(
            tasks_by_id["comfy-parity-native-node-runtime-foundation"]["writes"]
        )
        registry_writes = set(
            tasks_by_id["comfy-parity-native-registry-integration"]["writes"]
        )

        self.assertIn("crates/comfy_nodes/src/execution.rs", foundation_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", foundation_writes)
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", foundation_writes)
        self.assertIn("crates/comfy_api/src/services.rs", foundation_writes)
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs", registry_writes
        )
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", registry_writes)
        self.assertIn("crates/comfy_api/src/services.rs", registry_writes)
        self.assertIn("crates/sim/src/sim.rs", registry_writes)

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
