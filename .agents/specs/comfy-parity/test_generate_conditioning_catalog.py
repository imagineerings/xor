#!/usr/bin/env python3

from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import generate_conditioning_catalog as catalog


class ConditioningArtifactClosureTests(unittest.TestCase):
    def create_temporary_implementation(self, relative_path: str) -> dict[str, str]:
        if self.workspace.resolve() == catalog.WORKSPACE.resolve():
            raise RuntimeError("closure fixtures must never use the repository workspace")
        path = self.workspace / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        if not path.exists():
            path.write_bytes(relative_path.encode("utf-8"))
        return {
            "path": relative_path,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.workspace = Path(self.temporary_directory.name)
        self.implementation = self.workspace / "crates/comfy_model/src/patch_graph.rs"
        self.implementation.parent.mkdir(parents=True)
        self.implementation.write_bytes(b"canonical implementation\n")
        self.implementation_digest = hashlib.sha256(
            self.implementation.read_bytes()
        ).hexdigest()
        self.patch_implementations = [
            self.create_temporary_implementation(relative_path)
            for relative_path in sorted(
                catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.PATCH_GRAPH_TASK]
            )
        ]
        self.row = {
            "contract_id": "conditioning-patch-payload-example",
            "source_sha256": "a" * 64,
            "symbol_sha256": "b" * 64,
            "implementation_task": catalog.PATCH_GRAPH_TASK,
        }
        self.declared_writes = frozenset(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.PATCH_GRAPH_TASK]
        )
        self.declared_writes_by_task = {
            catalog.PATCH_GRAPH_TASK: self.declared_writes
        }
        self.payload = {
            "schema_version": 1,
            "validation_id": "VAL-PATCH-001",
            "overall_status": "partial",
            "environment": {
                "os": "test-os",
                "arch": "test-arch",
                "backend": "native-rust-cpu",
                "device": "cpu",
                "dtype": "F32",
            },
            "summary": {"passed": 1, "failed": 0, "skipped": 0},
            "implementation": {
                "path": "crates/comfy_model/src/patch_graph.rs",
                "sha256": self.implementation_digest,
            },
            "task_results": {
                catalog.PATCH_GRAPH_TASK: {
                    "status": "passed",
                    "passed": 1,
                    "failed": 0,
                    "skipped": 0,
                    "implementations": self.patch_implementations,
                }
            },
            "contracts": [
                {
                    "contract_id": self.row["contract_id"],
                    "task_id": self.row["implementation_task"],
                    "source_sha256": self.row["source_sha256"],
                    "symbol_sha256": self.row["symbol_sha256"],
                    "status": "passed",
                    "case_ids": ["source-derived-positive", "typed-invalid"],
                }
            ],
            "remaining_pending_tasks": ["a-truthful-later-task"],
        }

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write_artifact(self, payload: object | None = None) -> Path:
        path = self.workspace / "target/comfy-parity/val-patch-001.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(self.payload if payload is None else payload),
            encoding="utf-8",
        )
        return path

    def assert_rejected(self, payload: object) -> None:
        self.write_artifact(payload)
        self.assertEqual(
            catalog.validation_artifact(
                "VAL-PATCH-001",
                self.row,
                self.workspace,
                self.declared_writes_by_task,
            ),
            ("", ""),
        )

    def test_partial_artifact_promotes_only_an_exact_passed_contract(self) -> None:
        self.write_artifact()
        path, artifact_digest = catalog.validation_artifact(
            "VAL-PATCH-001", self.row, self.workspace, self.declared_writes_by_task
        )
        self.assertEqual(path, "target/comfy-parity/val-patch-001.json")
        self.assertRegex(artifact_digest, r"^[0-9a-f]{64}$")

        unrelated = dict(self.row)
        unrelated["contract_id"] = "conditioning-patch-payload-unrelated"
        self.assertEqual(
            catalog.validation_artifact(
                "VAL-PATCH-001",
                unrelated,
                self.workspace,
                self.declared_writes_by_task,
            ),
            ("", ""),
        )

    def test_wrong_task_digest_status_or_cases_fail_closed(self) -> None:
        mutations = (
            ("wrong task", ("contracts", 0, "task_id"), "another-task"),
            ("wrong source", ("contracts", 0, "source_sha256"), "c" * 64),
            ("wrong symbol", ("contracts", 0, "symbol_sha256"), "c" * 64),
            ("failed contract", ("contracts", 0, "status"), "failed"),
            ("empty cases", ("contracts", 0, "case_ids"), []),
        )
        for label, path, value in mutations:
            with self.subTest(label=label):
                payload = copy.deepcopy(self.payload)
                target = payload
                for part in path[:-1]:
                    target = target[part]
                target[path[-1]] = value
                self.assert_rejected(payload)

    def test_duplicate_contracts_and_duplicate_case_ids_fail_closed(self) -> None:
        duplicate_contract = copy.deepcopy(self.payload)
        duplicate_contract["contracts"].append(
            copy.deepcopy(duplicate_contract["contracts"][0])
        )
        self.assert_rejected(duplicate_contract)

        duplicate_case = copy.deepcopy(self.payload)
        duplicate_case["contracts"][0]["case_ids"] = ["same", "same"]
        self.assert_rejected(duplicate_case)

    def test_malformed_summary_environment_and_implementation_fail_closed(self) -> None:
        invalid_payloads = []
        for field, value in (("failed", 1), ("skipped", 1), ("passed", True)):
            payload = copy.deepcopy(self.payload)
            payload["summary"][field] = value
            invalid_payloads.append(payload)
        payload = copy.deepcopy(self.payload)
        payload["environment"]["backend"] = ""
        invalid_payloads.append(payload)
        payload = copy.deepcopy(self.payload)
        payload["implementation"]["sha256"] = "f" * 64
        invalid_payloads.append(payload)
        payload = copy.deepcopy(self.payload)
        payload["implementation"]["path"] = "../outside.rs"
        invalid_payloads.append(payload)
        for payload in invalid_payloads:
            self.assert_rejected(payload)

    def test_duplicate_json_keys_and_artifact_symlinks_fail_closed(self) -> None:
        path = self.write_artifact()
        path.write_text(
            '{"schema_version":1,"schema_version":1}', encoding="utf-8"
        )
        self.assertEqual(
            catalog.validation_artifact(
                "VAL-PATCH-001",
                self.row,
                self.workspace,
                self.declared_writes_by_task,
            ),
            ("", ""),
        )

        real_path = path.with_name("real-val-patch-001.json")
        real_path.write_text(json.dumps(self.payload), encoding="utf-8")
        path.unlink()
        path.symlink_to(real_path)
        self.assertEqual(
            catalog.validation_artifact(
                "VAL-PATCH-001",
                self.row,
                self.workspace,
                self.declared_writes_by_task,
            ),
            ("", ""),
        )

    def test_symlinked_implementation_fails_closed(self) -> None:
        link = self.workspace / "crates/comfy_model/src/linked_patch_graph.rs"
        link.symlink_to(self.implementation)
        payload = copy.deepcopy(self.payload)
        payload["implementation"]["path"] = (
            "crates/comfy_model/src/linked_patch_graph.rs"
        )
        self.assert_rejected(payload)

    def test_owning_task_implementation_must_be_current_and_declared(self) -> None:
        stale = copy.deepcopy(self.payload)
        stale["task_results"][catalog.PATCH_GRAPH_TASK]["implementations"][0][
            "sha256"
        ] = "f" * 64
        self.assert_rejected(stale)

        missing = copy.deepcopy(self.payload)
        missing["task_results"][catalog.PATCH_GRAPH_TASK]["implementations"].pop()
        self.assert_rejected(missing)

        undeclared_path = "crates/comfy_model/src/unrelated.rs"
        undeclared = self.workspace / undeclared_path
        undeclared.write_bytes(b"unrelated implementation\n")
        payload = copy.deepcopy(self.payload)
        payload["task_results"][catalog.PATCH_GRAPH_TASK]["implementations"].append(
            {
                "path": undeclared_path,
                "sha256": hashlib.sha256(undeclared.read_bytes()).hexdigest(),
            }
        )
        self.assert_rejected(payload)

    def test_tokenizer_task_requires_its_complete_current_implementation_closure(self) -> None:
        implementations = []
        for relative_path in sorted(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.TOKENIZER_TASK]
        ):
            path = self.workspace / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"implementation for {relative_path}\n", encoding="utf-8")
            implementations.append(
                {
                    "path": relative_path,
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                }
            )
        row = dict(self.row)
        row["implementation_task"] = catalog.TOKENIZER_TASK
        payload = copy.deepcopy(self.payload)
        payload["validation_id"] = "VAL-CLIP-001"
        task_result = payload["task_results"].pop(catalog.PATCH_GRAPH_TASK)
        task_result["implementations"] = implementations
        task_result["case_ids"] = sorted(
            catalog.TASK_REQUIRED_CASES[catalog.TOKENIZER_TASK]
        )
        payload["task_results"][catalog.TOKENIZER_TASK] = task_result
        payload["contracts"][0]["task_id"] = catalog.TOKENIZER_TASK
        declared_writes = frozenset(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.TOKENIZER_TASK]
        )
        self.assertTrue(
            catalog.artifact_covers_row(
                payload,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.TOKENIZER_TASK: declared_writes},
            )
        )

        missing = copy.deepcopy(payload)
        missing["task_results"][catalog.TOKENIZER_TASK]["implementations"].pop()
        self.assertFalse(
            catalog.artifact_covers_row(
                missing,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.TOKENIZER_TASK: declared_writes},
            )
        )

        stale = copy.deepcopy(payload)
        stale["task_results"][catalog.TOKENIZER_TASK]["implementations"][0][
            "sha256"
        ] = "f" * 64
        self.assertFalse(
            catalog.artifact_covers_row(
                stale,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.TOKENIZER_TASK: declared_writes},
            )
        )

        missing_task_case = copy.deepcopy(payload)
        missing_task_case["task_results"][catalog.TOKENIZER_TASK]["case_ids"].pop()
        self.assertFalse(
            catalog.artifact_covers_row(
                missing_task_case,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.TOKENIZER_TASK: declared_writes},
            )
        )

    def test_vision_task_requires_current_implementation_and_case_closure(self) -> None:
        implementations = []
        for relative_path in sorted(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.VISION_TASK]
        ):
            path = self.workspace / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"implementation for {relative_path}\n", encoding="utf-8")
            implementations.append(
                {
                    "path": relative_path,
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                }
            )
        row = dict(self.row)
        row["implementation_task"] = catalog.VISION_TASK
        payload = copy.deepcopy(self.payload)
        payload["validation_id"] = "VAL-CLIP-001"
        task_result = payload["task_results"].pop(catalog.PATCH_GRAPH_TASK)
        task_result["implementations"] = implementations
        task_result["case_ids"] = sorted(
            catalog.TASK_REQUIRED_CASES[catalog.VISION_TASK]
        )
        payload["task_results"][catalog.VISION_TASK] = task_result
        payload["contracts"][0]["task_id"] = catalog.VISION_TASK
        declared_writes = frozenset(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.VISION_TASK]
        )
        self.assertTrue(
            catalog.artifact_covers_row(
                payload,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.VISION_TASK: declared_writes},
            )
        )

        missing_case = copy.deepcopy(payload)
        missing_case["task_results"][catalog.VISION_TASK]["case_ids"].pop()
        self.assertFalse(
            catalog.artifact_covers_row(
                missing_case,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.VISION_TASK: declared_writes},
            )
        )

    def test_text_task_requires_current_implementation_and_case_closure(self) -> None:
        implementations = []
        for relative_path in sorted(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.TEXT_TASK]
        ):
            path = self.workspace / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"implementation for {relative_path}\n", encoding="utf-8")
            implementations.append(
                {
                    "path": relative_path,
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                }
            )
        row = dict(self.row)
        row["implementation_task"] = catalog.TEXT_TASK
        payload = copy.deepcopy(self.payload)
        payload["validation_id"] = "VAL-CLIP-001"
        task_result = payload["task_results"].pop(catalog.PATCH_GRAPH_TASK)
        task_result["implementations"] = implementations
        task_result["case_ids"] = sorted(
            catalog.TASK_REQUIRED_CASES[catalog.TEXT_TASK]
        )
        payload["task_results"][catalog.TEXT_TASK] = task_result
        payload["contracts"][0]["task_id"] = catalog.TEXT_TASK
        declared_writes = frozenset(
            catalog.TASK_IMPLEMENTATION_CLOSURES[catalog.TEXT_TASK]
        )
        self.assertTrue(
            catalog.artifact_covers_row(
                payload,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.TEXT_TASK: declared_writes},
            )
        )

        missing_case = copy.deepcopy(payload)
        missing_case["task_results"][catalog.TEXT_TASK]["case_ids"].pop()
        self.assertFalse(
            catalog.artifact_covers_row(
                missing_case,
                row,
                "VAL-CLIP-001",
                self.workspace,
                {catalog.TEXT_TASK: declared_writes},
            )
        )

    def test_visual_preprocess_symbols_route_to_vision_owner(self) -> None:
        self.assertEqual(
            catalog.VISION_SYMBOLS,
            frozenset(
                {
                    "clip_preprocess",
                    "siglip2_flex_calc_resolution",
                    "siglip2_preprocess",
                    "siglip2_pos_embed",
                    "Siglip2Embeddings",
                    "CLIPVisionEmbeddings",
                    "CLIPVision",
                    "LlavaProjector",
                    "CLIPVisionModelProjection",
                }
            ),
        )
        self.assertTrue(
            {
                "clip_preprocess",
                "siglip2_flex_calc_resolution",
                "siglip2_preprocess",
            }.issubset(catalog.VISION_SYMBOLS)
        )
        self.assertFalse(
            {"CLIPAttention", "CLIPTextModel", "SD1ClipModel"}
            & catalog.VISION_SYMBOLS
        )

    def test_exact_text_symbols_route_to_text_transformer_owner(self) -> None:
        self.assertEqual(
            catalog.TEXT_SYMBOLS,
            frozenset(
                {
                    "CLIPAttention",
                    "CLIPMLP",
                    "CLIPLayer",
                    "CLIPEncoder",
                    "CLIPEmbeddings",
                    "CLIPTextModel_",
                    "CLIPTextModel",
                    "SDClipModel",
                    "SD1CheckpointClipModel",
                    "SD1ClipModel",
                }
            ),
        )
        self.assertFalse(catalog.TEXT_SYMBOLS & catalog.VISION_SYMBOLS)

    def test_text_encoder_sources_have_one_exact_architecture_owner(self) -> None:
        declared_files: set[str] = set()
        for task_id, source_files in catalog.TEXT_ENCODER_SOURCE_GROUPS.items():
            self.assertFalse(
                declared_files & source_files,
                f"text encoder source assigned to multiple owners for {task_id}",
            )
            declared_files.update(source_files)

        source_root = catalog.WORKSPACE / "projects/comfy/ComfyUI/comfy/text_encoders"
        actual_files = {
            path.name
            for path in source_root.glob("*.py")
            if path.name != "__init__.py"
        }
        self.assertEqual(declared_files, actual_files)

        rows = [
            row
            for row in catalog.generate_rows()
            if row["kind"] == "clip_text_encoder_architecture"
        ]
        expected_counts = {
            catalog.TEXT_ENCODER_T5_TASK: 19,
            catalog.TEXT_ENCODER_DECODER_TASK: 127,
            catalog.TEXT_ENCODER_MULTIMODAL_TASK: 53,
            catalog.TEXT_ENCODER_COMPOSITE_TASK: 199,
        }
        self.assertEqual(len(rows), 398)
        self.assertEqual(len({row["contract_id"] for row in rows}), 398)
        for task_id, expected_count in expected_counts.items():
            owned_rows = [
                row for row in rows if row["implementation_task"] == task_id
            ]
            self.assertEqual(len(owned_rows), expected_count)
            self.assertTrue(
                all(
                    row["native_owner"] == catalog.TEXT_ENCODER_GROUP_OWNERS[task_id]
                    for row in owned_rows
                )
            )

    def test_cumulative_artifact_checks_each_task_against_its_own_declared_writes(self) -> None:
        payload = copy.deepcopy(self.payload)
        second_task = "second-task"
        second_implementation = self.create_temporary_implementation(
            "crates/comfy_model/src/second.rs"
        )
        payload["task_results"][second_task] = {
            "status": "passed",
            "passed": 1,
            "failed": 0,
            "skipped": 0,
            "implementation": second_implementation,
        }
        payload["contracts"].append(
            {
                "contract_id": "conditioning-second-contract",
                "task_id": second_task,
                "source_sha256": "c" * 64,
                "symbol_sha256": "d" * 64,
                "status": "passed",
                "case_ids": ["second-source-derived-case"],
            }
        )
        payload["summary"]["passed"] = 2
        declared_writes_by_task = {
            catalog.PATCH_GRAPH_TASK: self.declared_writes,
            second_task: frozenset({second_implementation["path"]}),
        }
        self.assertTrue(
            catalog.artifact_covers_row(
                payload,
                self.row,
                "VAL-PATCH-001",
                self.workspace,
                declared_writes_by_task,
            )
        )
        declared_writes_by_task.pop(second_task)
        self.assertFalse(
            catalog.artifact_covers_row(
                payload,
                self.row,
                "VAL-PATCH-001",
                self.workspace,
                declared_writes_by_task,
            )
        )

    def test_only_explicit_closure_artifacts_are_selected(self) -> None:
        self.assertEqual(
            catalog.closure_artifact_for(
                "comfy_model::patch_graph::tests", catalog.PATCH_GRAPH_TASK
            ),
            "VAL-PATCH-001",
        )
        self.assertEqual(
            catalog.closure_artifact_for(
                "comfy_model::conditioning::tests",
                "comfy-parity-conditioning-value-foundation",
            ),
            "",
        )
        self.assertEqual(
            catalog.closure_artifact_for(
                "VAL-VAE-001", "comfy-parity-vae-domain-loader-foundation"
            ),
            "VAL-VAE-001",
        )

    def test_task_state_requires_one_current_non_stale_evidence_record(self) -> None:
        tasks = self.workspace / "tasks.md"
        tasks.write_text(
            "- [x] 1. Exact\n"
            "  - _id: exact\n"
            "  - Writes: exact.rs\n"
            "  - _validation_evidence: fresh passing evidence\n"
            "- [x] 2. Stale\n"
            "  - _id: stale\n"
            "  - Writes: stale.rs\n"
            "  - _validation_evidence: STALE after audit\n"
            "- [x] 3. Duplicate\n"
            "  - _id: duplicate\n"
            "  - Writes: duplicate.rs\n"
            "  - _validation_evidence: old evidence\n"
            "  - _validation_evidence: new evidence\n",
            encoding="utf-8",
        )
        states = catalog.task_states(tasks)
        self.assertEqual(
            states["exact"],
            (True, "fresh passing evidence", frozenset({"exact.rs"})),
        )
        self.assertEqual(states["stale"], (True, "", frozenset({"stale.rs"})))
        self.assertEqual(
            states["duplicate"], (True, "", frozenset({"duplicate.rs"}))
        )


if __name__ == "__main__":
    unittest.main()
