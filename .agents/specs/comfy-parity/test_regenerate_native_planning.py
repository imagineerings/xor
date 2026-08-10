#!/usr/bin/env python3

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import regenerate_native_planning as planning


class ValidationGenerationTests(unittest.TestCase):
    def test_schema_foundation_reopens_until_source_identity_evidence_is_fresh(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tasks.md").write_text(
                "- [x] 368. Preserve exact native node schema and source metadata\n"
                "  - _id: comfy-parity-native-node-schema-metadata-foundation\n"
                "  - _validation_evidence: prior catalog evidence\n",
                encoding="utf-8",
            )
            with patch.object(planning, "ROOT", root):
                stale = planning.existing_task_annotations()[
                    "comfy-parity-native-node-schema-metadata-foundation"
                ]
            self.assertFalse(stale["complete"])
            self.assertIn("STALE AFTER NODE SOURCE IDENTITY REVALIDATION", stale["evidence"])

            (root / "tasks.md").write_text(
                "- [x] 368. Preserve exact native node schema and source metadata\n"
                "  - _id: comfy-parity-native-node-schema-metadata-foundation\n"
                "  - _validation_evidence: POST-NODE-SOURCE-IDENTITY-REVALIDATION fresh evidence\n",
                encoding="utf-8",
            )
            with patch.object(planning, "ROOT", root):
                fresh = planning.existing_task_annotations()[
                    "comfy-parity-native-node-schema-metadata-foundation"
                ]
            self.assertTrue(fresh["complete"])

    def test_native_node_runtime_foundation_orders_disjoint_leaves_and_registry(self) -> None:
        tasks, mapping = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        foundation_id = "comfy-parity-native-node-runtime-foundation"
        schema_id = "comfy-parity-native-node-schema-metadata-foundation"
        value_id = "comfy-parity-native-node-compute-value-foundation"
        asset_id = "comfy-parity-native-node-asset-effect-foundation"
        provider_id = "comfy-parity-native-node-provider-invocation-foundation"
        compute_id = "comfy-parity-native-compute-breadth-integration"
        registry_id = "comfy-parity-native-registry-integration"
        node_ids = sorted(
            identifier
            for identifier in tasks_by_id
            if identifier.startswith("comfy-parity-native-nodes-")
        )

        self.assertEqual(len(tasks), 521)
        self.assertEqual(len(node_ids), 102)
        self.assertEqual(tasks_by_id[foundation_id]["dependencies"], [compute_id])
        for identifier in (schema_id, value_id, asset_id, provider_id):
            self.assertTrue(tasks_by_id[identifier]["feature_scoped"])
        self.assertEqual(tasks_by_id[schema_id]["dependencies"], [foundation_id])
        self.assertEqual(
            tasks_by_id[value_id]["dependencies"],
            [
                schema_id,
                compute_id,
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        self.assertEqual(
            tasks_by_id[asset_id]["dependencies"],
            [
                value_id,
                "comfy-parity-artifact-owner-consolidation",
                "comfy-parity-execution-output-owner-consolidation",
            ],
        )
        self.assertEqual(
            tasks_by_id[provider_id]["dependencies"],
            [asset_id, "comfy-parity-extension-host-plugin-adapter"],
        )
        dependencies = {
            identifier: set(tasks_by_id[identifier]["dependencies"])
            for identifier in node_ids
        }
        self.assertEqual(sum(schema_id in value for value in dependencies.values()), 102)
        self.assertEqual(sum(value_id in value for value in dependencies.values()), 84)
        self.assertEqual(sum(asset_id in value for value in dependencies.values()), 84)
        self.assertEqual(sum(provider_id in value for value in dependencies.values()), 25)
        self.assertEqual(
            sum(
                value_id in value and provider_id in value
                for value in dependencies.values()
            ),
            7,
        )
        self.assertEqual(
            sum(
                value_id in value and provider_id not in value
                for value in dependencies.values()
            ),
            77,
        )
        self.assertEqual(
            sum(
                provider_id in value and value_id not in value
                for value in dependencies.values()
            ),
            18,
        )
        mapped_values = {
            identifier: sum(
                identifier in task_ids for task_ids in mapping.values()
            )
            for identifier in (schema_id, value_id, asset_id, provider_id)
        }
        self.assertEqual(
            mapped_values,
            {schema_id: 789, value_id: 575, asset_id: 189, provider_id: 214},
        )
        self.assertEqual(sorted(tasks_by_id[registry_id]["dependencies"]), node_ids)

        waves = planning.task_waves(tasks)
        self.assertEqual(waves[foundation_id], waves[compute_id] + 1)
        self.assertEqual(waves[schema_id], waves[foundation_id] + 1)
        self.assertEqual(waves[value_id], waves[schema_id] + 1)
        self.assertEqual(waves[asset_id], waves[value_id] + 1)
        self.assertEqual(waves[provider_id], waves[asset_id] + 1)
        self.assertEqual(
            waves[registry_id], max(waves[identifier] for identifier in node_ids) + 1
        )

    def test_native_node_foundation_and_registry_own_runtime_reachability_paths(self) -> None:
        tasks, _ = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        foundation_writes = set(
            tasks_by_id["comfy-parity-native-node-runtime-foundation"]["writes"]
        )
        schema_writes = set(tasks_by_id["comfy-parity-native-node-schema-metadata-foundation"]["writes"])
        value_reads = set(tasks_by_id["comfy-parity-native-node-compute-value-foundation"]["reads"])
        value_writes = set(tasks_by_id["comfy-parity-native-node-compute-value-foundation"]["writes"])
        asset_reads = set(tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]["reads"])
        asset_writes = set(tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]["writes"])
        provider_reads = set(
            tasks_by_id["comfy-parity-native-node-provider-invocation-foundation"]["reads"]
        )
        provider_writes = set(tasks_by_id["comfy-parity-native-node-provider-invocation-foundation"]["writes"])
        registry_writes = set(
            tasks_by_id["comfy-parity-native-registry-integration"]["writes"]
        )

        self.assertIn("crates/comfy_nodes/src/execution.rs", foundation_writes)
        self.assertIn("crates/comfy_nodes/src/object_info.rs", foundation_writes)
        self.assertIn("Cargo.lock", foundation_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", foundation_writes)
        self.assertIn("crates/comfy_runtime/src/cache.rs", foundation_writes)
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", foundation_writes)
        self.assertIn("crates/comfy_api/src/services.rs", foundation_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", foundation_writes)
        self.assertIn("crates/comfy_ui/src/execution_model.rs", foundation_writes)
        self.assertIn("crates/sim/src/sim.rs", foundation_writes)
        self.assertIn(
            "crates/comfy_test_support/tests/plugin_e2e.rs", foundation_writes
        )
        self.assertIn(
            ".agents/specs/comfy-parity/ownership-policy.json", foundation_writes
        )
        self.assertIn("crates/comfy_nodes/src/execution.rs", schema_writes)
        self.assertIn(
            "crates/comfy_nodes/src/families/empty_root_category_declared_by_source_01.rs",
            schema_writes,
        )
        self.assertIn("crates/comfy_nodes/src/slices/native_image.descriptors.json", schema_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", schema_writes)
        self.assertIn("crates/comfy_runtime/src/graph.rs", schema_writes)
        self.assertIn("crates/comfy_runtime/src/workflow_formats.rs", schema_writes)
        self.assertIn("crates/comfy_api/src/services.rs", schema_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", schema_writes)
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", schema_writes)
        schema_task = tasks_by_id["comfy-parity-native-node-schema-metadata-foundation"]
        self.assertEqual(
            schema_task["criterion_ids"],
            [
                "4.1", "4.2", "4.3", "6.1", "6.2", "6.3", "6.5",
                "16.3", "16.4", "32.1", "32.3", "32.5", "32.8", "44.2",
            ],
        )
        self.assertNotIn("VAL-NODE-002", schema_task["validations"])
        self.assertIn("crates/comfy_model/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_model/src/clip_vision.rs", value_writes)
        self.assertIn("crates/comfy_model/src/vision_models.rs", value_writes)
        for path in [
            "projects/comfy/ComfyUI/comfy_api/latest/_input/basic_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_input/video_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_input_impl/video_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_util/video_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_util/geometry_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_io.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_hunyuan3d.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_load_3d.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_gaussian_splat.py",
        ]:
            self.assertIn(path, asset_reads)
        for path in [
            "crates/comfy_tensor/src/native_node_payload.rs",
            "crates/comfy_tensor/src/image_ops.rs",
            "crates/comfy_tensor/src/operation.rs",
            "crates/comfy_tensor/src/cpu_backend.rs",
            "crates/comfy_nodes/src/source_type.rs",
        ]:
            self.assertIn(path, asset_writes)
        self.assertIn("crates/comfy_model/tests/model_families.rs", value_writes)
        self.assertIn("crates/comfy_model/src/controlnet.rs", value_writes)
        self.assertIn("crates/comfy_model/src/conditioning.rs", value_writes)
        self.assertIn("crates/comfy_tensor/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_sampler/src/native_diffusion_payload.rs", value_writes)
        self.assertIn("crates/comfy_sampler/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_plugin_host/Cargo.toml", value_writes)
        self.assertIn("crates/comfy_plugin_sdk/Cargo.toml", value_writes)
        self.assertIn("crates/comfy_plugin_sdk/src/type_ids.rs", value_writes)
        self.assertIn("crates/comfy_media/Cargo.toml", value_writes)
        self.assertIn("crates/comfy_media/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", value_writes)
        self.assertIn("crates/comfy_nodes/src/source_type.rs", value_writes)
        self.assertIn("crates/comfy_nodes/src/stored_payload.rs", value_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", value_writes)
        self.assertIn("crates/comfy_test_support/tests/plugin_e2e.rs", value_writes)
        value_task = tasks_by_id["comfy-parity-native-node-compute-value-foundation"]
        self.assertIn(".agents/specs/comfy-parity/catalogs/backend-node-contracts.json", value_reads)
        self.assertIn(39, value_task["requirements"])
        self.assertIn(35, value_task["designs"])
        self.assertIn("39.3", value_task["criterion_ids"])
        self.assertIn("39.6", value_task["criterion_ids"])
        self.assertNotIn("VAL-NODE-002", value_task["validations"])
        for validation in [
            "VAL-PLUGIN-HOST-001",
            "VAL-E2E-003",
            "VAL-WORKER-PLUGIN-001",
        ]:
            self.assertIn(validation, value_task["validations"])
        self.assertIn("crates/comfy_media/src/native_node_payload.rs", asset_writes)
        self.assertIn("crates/comfy_media/Cargo.toml", asset_writes)
        self.assertIn("crates/comfy_media/src/gaussian_splat.rs", asset_writes)
        self.assertIn("crates/comfy_nodes/src/execution.rs", asset_writes)
        self.assertIn("crates/comfy_nodes/src/stored_payload.rs", asset_writes)
        self.assertIn("crates/comfy_runtime/src/output_committer.rs", asset_writes)
        self.assertIn("crates/comfy_runtime/src/permissions.rs", asset_reads)
        self.assertIn("crates/comfy_runtime/src/permissions.rs", asset_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", asset_writes)
        self.assertIn("crates/comfy_plugin_host/tests/component_contract.rs", asset_writes)
        self.assertNotIn("crates/comfy_runtime/src/providers.rs", provider_reads)
        self.assertIn("crates/comfy_runtime/src/trust.rs", provider_reads)
        self.assertIn("crates/comfy_runtime/src/permissions.rs", provider_reads)
        self.assertIn("crates/comfy_runtime/src/plugin_services.rs", provider_reads)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_host/src/capabilities.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_sdk/wit/comfy-plugin.wit", provider_writes)
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs", registry_writes
        )
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", registry_writes)
        self.assertIn("crates/comfy_api/src/services.rs", registry_writes)
        self.assertIn("crates/sim/src/sim.rs", registry_writes)

        schema_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-schema-metadata-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_nodes val_node_001 -- --nocapture",
            "cargo test --locked -p comfy_nodes val_node_registry_001 -- --nocapture",
            "cargo test --locked -p comfy_runtime val_domain_004 -- --nocapture",
            "ownership_consolidation val_ownership_001 -- --exact --nocapture",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_generate_node_contract_catalog.py",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
        ]:
            self.assertIn(command, schema_commands)

        value_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-compute-value-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_runtime val_domain_004 -- --nocapture",
            "cargo test --locked -p comfy_tensor val_tensor_001 -- --nocapture",
            "cargo test --locked -p comfy_model val_model_family_001 -- --nocapture",
            "native_image_e2e val_native_e2e_001 -- --exact --nocapture",
            "cargo test --locked -p comfy_model --lib clip_vision",
            "cargo test --locked -p comfy_model --lib raft_ -- --nocapture",
            "cargo test --locked -p comfy_model --lib controlnet -- --nocapture",
            "cargo test --locked -p comfy_sampler --lib native_node_payload",
            "cargo test --locked -p comfy_media --lib native_node_payload",
            "cargo test --locked -p comfy_plugin_sdk --lib type_ids -- --nocapture",
            "cargo test --locked -p comfy_test_support --test native_conditioning_integration -- --nocapture",
            "registry_adapter::tests::explicit_stored_variants_are_exhaustively_projected_or_rejected -- --exact",
            "val_ownership_001_native_stored_payload_boundary_is_closed",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
        ]:
            self.assertIn(command, value_commands)
        asset_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_plugin_host --lib registry_adapter -- --nocapture",
            "cargo test --locked -p comfy_plugin_host --test component_contract -- --nocapture",
        ]:
            self.assertIn(command, asset_commands)

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
