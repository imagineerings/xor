#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent
GENERATOR_PATH = ROOT / "generate_spandrel_image_model_contract.py"
SPEC = importlib.util.spec_from_file_location("spandrel_contract_generator", GENERATOR_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load Spandrel contract generator")
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class SpandrelImageModelContractTests(unittest.TestCase):
    def test_checked_contract_is_exact_and_fail_closed(self) -> None:
        generated = GENERATOR.build_contract()
        checked_in = json.loads(GENERATOR.OUTPUT.read_text(encoding="utf-8"))
        self.assertEqual(generated, checked_in)

        snapshots = generated["source_snapshots"]
        self.assertEqual(
            snapshots["spandrel"]["baseline_tree_sha256"],
            "e1870c42b314fddb290f4d5322a03743076d98d0c6d288fc73691e3013994bbb",
        )
        self.assertEqual(snapshots["spandrel"]["included_file_count"], 180)
        self.assertEqual(snapshots["spandrel"]["version"], "0.4.2")
        self.assertEqual(snapshots["spandrel"]["tag"], "v0.4.2")
        self.assertEqual(
            snapshots["spandrel"]["commit"],
            "724cca389f28c38e1050689d4862a452fd644484",
        )
        self.assertEqual(
            snapshots["spandrel_extra_arches"]["baseline_tree_sha256"],
            "7c0915d2e0df7db2131117087744fa5e73954dcad72aa785386d6bf8c1efb3aa",
        )
        self.assertEqual(snapshots["spandrel_extra_arches"]["included_file_count"], 52)
        self.assertEqual(snapshots["spandrel_extra_arches"]["version"], "0.2.0")
        self.assertEqual(snapshots["spandrel_extra_arches"]["tag"], "v0.4.0")
        self.assertEqual(
            snapshots["spandrel_extra_arches"]["commit"],
            "a1db3f5debbeeacbe02fb4114c69feee56ba5e21",
        )

        rows = generated["architectures"]
        self.assertEqual(len(rows), 52)
        self.assertEqual([row["ordinal"] for row in rows], list(range(52)))
        self.assertEqual(sum(row["origin"] == "main" for row in rows), 42)
        self.assertEqual(sum(row["origin"] == "extra" for row in rows), 10)
        self.assertEqual(len({row["architecture_id"] for row in rows}), 52)
        self.assertEqual(rows[0]["architecture_id"], "Compact")
        self.assertEqual(rows[41]["architecture_id"], "AuraSR")
        self.assertEqual(rows[42]["architecture_id"], "SRFormer")
        self.assertEqual(rows[-1]["architecture_id"], "MIRNet2")
        self.assertTrue(all(row["support_disposition"] == "rejected" for row in rows))
        self.assertTrue(all(not row["license_artifacts"] for row in rows))
        self.assertTrue(
            all(
                "missing-individual-license-artifact" in row["license_disposition"]
                for row in rows[:42]
            )
        )
        self.assertTrue(
            all("reference-only-extra" in row["license_disposition"] for row in rows[42:])
        )
        self.assertEqual(generated["summary"]["admitted_count"], 0)
        self.assertEqual(generated["summary"]["rejected_count"], 52)
        self.assertEqual(generated["task_projection"]["implementation_leaves"], [])

    def test_optional_extra_outcomes_and_native_boundary_are_closed(self) -> None:
        contract = GENERATOR.build_contract()
        self.assertEqual(
            [row["outcome"] for row in contract["optional_extra_outcomes"]],
            ["absent-or-import-failure", "successful-add", "add-failure"],
        )
        self.assertEqual(
            [row["registry"] for row in contract["optional_extra_outcomes"]],
            ["MAIN only", "MAIN followed by EXTRA in source order", "MAIN only"],
        )
        boundary = contract["source_boundary"]
        self.assertIn("native Rust only", boundary["production_runtime"])
        self.assertIn("no Python", boundary["production_runtime"])
        self.assertIn("no Python", boundary["fixtures"])
        serialized = json.dumps(contract)
        for suffix in GENERATOR.WEIGHT_SUFFIXES:
            self.assertNotIn(f'"{suffix}"', serialized)

    def test_registry_parser_rejects_duplicate_add_and_unsupported_rows(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.py"
            duplicate.write_text(
                "MAIN_REGISTRY.add(ArchSupport.from_architecture(A.AArch()))\n"
                "MAIN_REGISTRY.add(ArchSupport.from_architecture(B.BArch()))\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "duplicate MAIN_REGISTRY.add"):
                GENERATOR.registry_entries(duplicate, "MAIN_REGISTRY")

            unsupported = root / "unsupported.py"
            unsupported.write_text("MAIN_REGISTRY.add(dynamic_architecture())\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "unsupported registry expression"):
                GENERATOR.registry_entries(unsupported, "MAIN_REGISTRY")

    def test_snapshot_audit_rejects_links_weights_and_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "source.py").write_text("value = 1\n", encoding="utf-8")
            files = GENERATOR.included_files(root)
            first = GENERATOR.baseline_fingerprint(root, files)
            (root / "source.py").write_text("value = 2\n", encoding="utf-8")
            second = GENERATOR.baseline_fingerprint(root, GENERATOR.included_files(root))
            self.assertNotEqual(first, second)

            (root / "model.safetensors").write_bytes(b"not a model")
            with self.assertRaisesRegex(ValueError, "model weight is forbidden"):
                GENERATOR.included_files(root)

    def test_fixture_is_json_only_and_binds_catalog(self) -> None:
        fixture = json.loads(GENERATOR.FIXTURE.read_text(encoding="utf-8"))
        catalog_bytes = GENERATOR.encoded(GENERATOR.build_contract())
        self.assertEqual(fixture["catalog_sha256"], GENERATOR.sha256(catalog_bytes))
        fixture_files = [path for path in GENERATOR.FIXTURE.parent.rglob("*") if path.is_file()]
        self.assertEqual(fixture_files, [GENERATOR.FIXTURE])
        self.assertTrue(all(path.suffix == ".json" for path in fixture_files))


if __name__ == "__main__":
    unittest.main()
