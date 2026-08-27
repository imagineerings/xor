#!/usr/bin/env python3

import ast
import csv
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import generate_node_contract_catalog as generator


class NodeContractCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog = generator.build_catalog()
        cls.contracts = {
            contract["feature_id"]: contract
            for contract in cls.catalog["contracts"]
        }

    def test_catalog_is_complete_source_fingerprinted_and_deterministic(self) -> None:
        self.assertEqual(self.catalog["schema_version"], 2)
        self.assertEqual(self.catalog["summary"]["rows"], 789)
        self.assertEqual(self.catalog["summary"]["executable"], 565)
        self.assertEqual(self.catalog["summary"]["provider_required"], 224)
        self.assertEqual(self.catalog["summary"]["normalized_v3"], 654)
        self.assertEqual(self.catalog["summary"]["normalized_v1"], 135)
        self.assertEqual(self.catalog["summary"]["preserved_schema_contracts"], 0)
        self.assertEqual(len(self.contracts), 789)
        self.assertGreater(self.catalog["source_snapshot"]["files"], 100)
        self.assertEqual(len(self.catalog["source_snapshot"]["manifest_sha256"]), 64)
        first = json.dumps(self.catalog, indent=2, sort_keys=True) + "\n"
        second = generator.encoded_catalog().decode("utf-8")
        self.assertEqual(first, second)

    def test_schema_calls_preserve_exact_literals_and_unsupported_expressions(self) -> None:
        brightness = self.contracts["COMFY-NODE-0004"]
        contrast = self.contracts["COMFY-NODE-0005"]
        self.assertEqual(
            brightness["schema"]["portable"]["presentation"]["display_name"],
            "Adjust Brightness",
        )
        self.assertEqual(
            contrast["schema"]["portable"]["presentation"]["display_name"],
            "Adjust Contrast",
        )

        boolean = self.contracts["COMFY-NODE-0494"]
        self.assertEqual(boolean["binding_disposition"], "executable")
        self.assertEqual(boolean["schema"]["status"], "normalized_v3")
        boolean_contract = boolean["schema"]["contract"]
        self.assertTrue(
            any(value["constructor"].endswith("Boolean.Input") for value in boolean_contract["inputs"])
        )
        self.assertTrue(
            any(value["constructor"].endswith("Boolean.Output") for value in boolean_contract["outputs"])
        )

        bounding_box = self.contracts["COMFY-NODE-0495"]
        width_call = next(
            call
            for call in bounding_box["schema"]["contract"]["inputs"]
            if call["constructor"].endswith("Int.Input")
            and call["arguments"]
            and call["arguments"][0].get("value") == "width"
        )
        keywords = {keyword["name"]: keyword["value"] for keyword in width_call["keywords"]}
        self.assertEqual(keywords["default"]["value"], 512)
        self.assertEqual(keywords["min"]["value"], 1)
        self.assertEqual(keywords["max"]["name"], "MAX_RESOLUTION")

        multiline = self.contracts["COMFY-NODE-0499"]
        multiline_call = next(
            call
            for call in multiline["schema"]["contract"]["inputs"]
            if call["constructor"].endswith("String.Input")
        )
        multiline_keywords = {
            keyword["name"]: keyword["value"] for keyword in multiline_call["keywords"]
        }
        self.assertTrue(multiline_keywords["multiline"]["value"])

        provider = self.contracts["COMFY-NODE-0462"]
        provider_inputs = provider["schema"]["contract"]["inputs"]
        width = next(value for value in provider_inputs if value.get("name") == "custom_width")
        width_keywords = {item["name"]: item["value"] for item in width["keywords"]}
        self.assertEqual(width_keywords["default"]["value"], 1024)
        self.assertEqual(width_keywords["step"]["value"], 16)
        self.assertEqual(width_keywords["max"]["value"], 3840)
        quality = next(value for value in provider_inputs if value.get("name") == "quality")
        quality_keywords = {item["name"]: item["value"] for item in quality["keywords"]}
        self.assertEqual(
            [item["value"] for item in quality_keywords["options"]["items"]],
            ["low", "medium", "high"],
        )
        portable_inputs = {
            value["name"]: value
            for value in provider["schema"]["portable"]["inputs"]
        }
        self.assertEqual(
            [choice["value"] for choice in portable_inputs["quality"]["choices"]],
            ["low", "medium", "high"],
        )
        self.assertEqual(portable_inputs["custom_width"]["step"]["value"], 16)
        self.assertEqual(
            portable_inputs["seed"]["maximum"]["kind"],
            "preserved_expression",
        )

        paid_provider = self.contracts["COMFY-NODE-0024"]
        paid_provider_node = paid_provider["schema"]["portable"]["node"]
        self.assertEqual(
            paid_provider_node["price_badge"]["kind"],
            "preserved_expression",
        )
        self.assertIn(
            "IO.PriceBadge",
            paid_provider_node["price_badge"]["source"],
        )
        self.assertNotIn(
            "price_badge",
            {item["name"] for item in paid_provider_node["extra"]},
        )

    def test_v1_and_autogrow_contracts_are_structured_without_execution(self) -> None:
        sampler = self.contracts["COMFY-NODE-0306"]
        self.assertEqual(sampler["schema"]["status"], "normalized_v1")
        required = next(
            group
            for group in sampler["schema"]["contract"]["input_groups"]
            if group["name"] == "required"
        )
        seed = next(field for field in required["fields"] if field["name"] == "seed")
        options = seed["contract"]["items"][1]
        option_entries = {
            entry["key"]["value"]: entry["value"] for entry in options["entries"]
        }
        self.assertEqual(option_entries["default"]["value"], 0)
        self.assertEqual(option_entries["max"]["value"], 18446744073709551615)
        portable_sampler_inputs = {
            value["name"]: value
            for value in sampler["schema"]["portable"]["inputs"]
        }
        self.assertEqual(
            portable_sampler_inputs["seed"]["maximum"],
            {"kind": "unsigned_integer", "value": 18446744073709551615},
        )
        self.assertEqual(
            portable_sampler_inputs["cfg"]["step"],
            {"kind": "finite_decimal", "value": "0.1"},
        )

        batch = self.contracts["COMFY-NODE-0017"]
        bindings = batch["schema"]["contract"]["bindings"]
        template = next(
            binding for binding in bindings if "autogrow_template" in binding["targets"]
        )["value"]
        template_keywords = {item["name"]: item["value"] for item in template["keywords"]}
        self.assertEqual(template["name"], "io.Autogrow.TemplatePrefix")
        self.assertEqual(template_keywords["prefix"]["value"], "image")
        self.assertEqual(template_keywords["min"]["value"], 1)
        self.assertEqual(template_keywords["max"]["value"], 50)
        dynamic = batch["schema"]["portable"]["dynamic_inputs"]
        self.assertEqual(len(dynamic), 1)
        self.assertEqual(dynamic[0]["identity"], "image{index}")
        self.assertEqual(dynamic[0]["prefix"], "image")
        self.assertEqual(dynamic[0]["minimum_count"], 1)
        self.assertEqual(dynamic[0]["maximum_count"], 50)
        self.assertEqual(dynamic[0]["input"]["source_type_names"], ["IMAGE"])
        self.assertEqual(batch["schema"]["portable"]["inputs"], [])

        string_format = self.contracts["COMFY-NODE-0644"]
        format_dynamic = string_format["schema"]["portable"]["dynamic_inputs"]
        self.assertEqual(len(format_dynamic), 1)
        self.assertEqual(format_dynamic[0]["identity"], "{name}")
        self.assertEqual(
            format_dynamic[0]["names"], list("abcdefghijklmnopqrstuvwxyz")
        )
        self.assertEqual(format_dynamic[0]["start_index"], 0)
        self.assertEqual(format_dynamic[0]["minimum_count"], 0)
        self.assertEqual(format_dynamic[0]["maximum_count"], 26)
        self.assertNotIn(
            "names_expression",
            {item["name"] for item in format_dynamic[0]["extra"]},
        )

        math_expression = self.contracts["COMFY-NODE-0083"]
        math_dynamic = math_expression["schema"]["portable"]["dynamic_inputs"]
        self.assertEqual(len(math_dynamic), 1)
        self.assertEqual(
            math_dynamic[0]["input"]["source_type_names"],
            ["FLOAT", "INT", "BOOLEAN"],
        )

        inherited = self.contracts["COMFY-NODE-0159"]
        self.assertEqual(inherited["schema"]["catalog_correlation"], "verified_inherited_base")
        override_names = {
            item["name"] for item in inherited["schema"]["contract"]["inherited_overrides"]
        }
        self.assertEqual(override_names, {"node_id", "display_name", "category"})

        inherited_method = self.contracts["COMFY-NODE-0002"]
        self.assertEqual(
            inherited_method["schema"]["catalog_correlation"],
            "verified_inherited_method",
        )
        class_targets = {
            target
            for statement in inherited_method["schema"]["contract"]["class_overrides"]
            for target in statement.get("targets", [])
        }
        self.assertIn("node_id", class_targets)
        self.assertIn("extra_inputs", class_targets)

        for feature_id, display_name in (
            ("COMFY-NODE-0542", "Resize Images by Longer Edge (DEPRECATED)"),
            ("COMFY-NODE-0543", "Resize Images by Shorter Edge (DEPRECATED)"),
        ):
            inherited_presentation = self.contracts[feature_id]["schema"]["portable"][
                "presentation"
            ]
            self.assertEqual(inherited_presentation["display_name"], display_name)
            self.assertTrue(inherited_presentation["is_deprecated"])
            self.assertTrue(inherited_presentation["is_experimental"])

        with generator.INPUT.open(newline="", encoding="utf-8") as backend_nodes_file:
            rows_by_feature = {
                row["feature_id"]: row for row in csv.DictReader(backend_nodes_file)
            }
        corrected_presentations = {
            "COMFY-NODE-0002": "Add Text Prefix (DEPRECATED)",
            "COMFY-NODE-0003": "Add Text Suffix (DEPRECATED)",
            "COMFY-NODE-0047": "Crop Image (Center)",
            "COMFY-NODE-0159": "Empty HunyuanVideo 1.0 Latent",
            "COMFY-NODE-0249": "Deduplicate Images",
            "COMFY-NODE-0252": "Make Image Grid",
            "COMFY-NODE-0366": "Context Windows (Manual)",
            "COMFY-NODE-0405": "Merge Image Lists (DEPRECATED)",
            "COMFY-NODE-0407": "Merge Text Lists (DEPRECATED)",
            "COMFY-NODE-0456": "Normalize Image Colors",
            "COMFY-NODE-0504": "Crop Image (Random)",
            "COMFY-NODE-0620": "Shuffle Images List",
            "COMFY-NODE-0649": "Strip Whitespace (DEPRECATED)",
            "COMFY-NODE-0673": "Convert Text to Lowercase (DEPRECATED)",
            "COMFY-NODE-0674": "Convert Text to Uppercase (DEPRECATED)",
            "COMFY-NODE-0701": "Truncate Text",
            "COMFY-NODE-0760": "Context Windows (Manual)",
        }
        for feature_id, display_name in corrected_presentations.items():
            row = rows_by_feature[feature_id]
            presentation = self.contracts[feature_id]["schema"]["portable"][
                "presentation"
            ]
            self.assertEqual(row["display_name"], display_name)
            self.assertEqual(presentation["display_name"], display_name)

        for feature_id, contract in self.contracts.items():
            presentation = contract["schema"]["portable"]["presentation"]
            if presentation["display_name"] is not None:
                self.assertEqual(
                    rows_by_feature[feature_id]["display_name"],
                    presentation["display_name"],
                    feature_id,
                )

    def test_provider_disposition_uses_registered_api_identity_not_deprecation(self) -> None:
        deprecated_provider = self.contracts["COMFY-NODE-0462"]
        self.assertEqual(deprecated_provider["availability"], "deprecated/dead")
        self.assertEqual(deprecated_provider["classification"], "API node")
        self.assertEqual(deprecated_provider["binding_disposition"], "provider_required")

        deprecated_builtin = self.contracts["COMFY-NODE-0498"]
        self.assertEqual(deprecated_builtin["availability"], "deprecated/dead")
        self.assertEqual(deprecated_builtin["classification"], "built-in node")
        self.assertEqual(deprecated_builtin["binding_disposition"], "executable")

        cloud_provider = self.contracts["COMFY-NODE-0408"]
        self.assertEqual(cloud_provider["availability"], "cloud/paid")
        self.assertEqual(cloud_provider["binding_disposition"], "provider_required")

        local_partner_helper = self.contracts["COMFY-NODE-0148"]
        self.assertEqual(local_partner_helper["availability"], "cloud/paid")
        self.assertEqual(local_partner_helper["binding_disposition"], "provider_required")
        node_options = {
            item["name"]: item["value"]
            for item in local_partner_helper["schema"]["contract"]["node_options"]
        }
        self.assertFalse(node_options["is_api_node"]["value"])
        outputs = local_partner_helper["schema"]["contract"]["outputs"]
        self.assertTrue(all(output["callee"]["name"] for output in outputs))
        self.assertTrue(
            all(
                output["source_type_name"] != "PRESERVED_EXPRESSION"
                for output in local_partner_helper["schema"]["portable"]["outputs"]
            )
        )
        voice_output = local_partner_helper["schema"]["portable"]["outputs"][0]
        self.assertEqual(
            voice_output["extra"],
            [
                {
                    "name": "source_identity",
                    "value": {"kind": "string", "value": "ELEVENLABS_VOICE"},
                }
            ],
        )
        style_reference = next(
            value
            for value in self.contracts["COMFY-NODE-0305"]["schema"]["portable"]["inputs"]
            if value["name"] == "style_reference"
        )
        self.assertEqual(style_reference["source_type_names"], ["CUSTOM"])
        self.assertIn(
            {
                "name": "source_identity",
                "value": {"kind": "string", "value": "KreaIO.STYLE_REF"},
            },
            style_reference["extra"],
        )

        expanded_inputs = self.contracts["COMFY-NODE-0020"]["schema"]["portable"]
        self.assertEqual(
            [value["name"] for value in expanded_inputs["inputs"]],
            ["image", "prompt", "reference_image", "alpha_mode", "max_resolution", "seed"],
        )
        self.assertEqual(expanded_inputs["unresolved_inputs"], [])

        inherited_inputs = self.contracts["COMFY-NODE-0672"]["schema"]["portable"]
        self.assertEqual(inherited_inputs["inputs"][0]["name"], "clip")
        self.assertEqual(inherited_inputs["inputs"][-1]["name"], "use_default_template")
        self.assertEqual(inherited_inputs["unresolved_inputs"], [])

        for feature_id in ("COMFY-NODE-0382", "COMFY-NODE-0405", "COMFY-NODE-0546", "COMFY-NODE-0551", "COMFY-NODE-0690"):
            self.assertEqual(
                self.contracts[feature_id]["schema"]["portable"]["unresolved_inputs"],
                [],
            )
        self.assertTrue(
            all(
                contract["schema"]["portable"]["unresolved_inputs"] == []
                for contract in self.contracts.values()
            )
        )

        file_to_splat = self.contracts["COMFY-NODE-0172"]["schema"]["portable"]
        self.assertEqual(file_to_splat["unresolved_inputs"], [])
        self.assertEqual(
            file_to_splat["inputs"][0]["source_type_names"],
            ["FILE_3D", "FILE_3D_SPLAT_ANY", "FILE_3D_PLY", "FILE_3D_SPLAT", "FILE_3D_KSPLAT", "FILE_3D_SPZ"],
        )
        preview = self.contracts["COMFY-NODE-0487"]["schema"]["portable"]
        self.assertEqual(preview["inputs"][0]["name"], "model_file")
        self.assertEqual(preview["inputs"][0]["source_type_names"][0], "STRING")
        self.assertIn("FILE_3D_GLB", preview["inputs"][0]["source_type_names"])
        save_glb = self.contracts["COMFY-NODE-0592"]["schema"]["portable"]
        self.assertEqual(save_glb["inputs"][0]["name"], "mesh")
        self.assertEqual(save_glb["inputs"][0]["source_type_names"][0], "MESH")
        self.assertEqual(save_glb["unresolved_inputs"], [])

    def test_module_custom_aliases_preserve_declared_source_identities(self) -> None:
        mediapipe = self.contracts["COMFY-NODE-0402"]["schema"]["portable"]
        self.assertEqual(
            mediapipe["inputs"][0]["source_type_names"],
            ["FACE_DETECTION_MODEL"],
        )
        self.assertEqual(
            mediapipe["outputs"][0]["source_type_name"],
            "FACE_LANDMARKS",
        )

        ic_lora = self.contracts["COMFY-NODE-0204"]["schema"]["portable"]
        self.assertEqual(
            ic_lora["outputs"][0]["source_type_name"],
            "IC_LORA_PARAMETERS",
        )

        sam3 = self.contracts["COMFY-NODE-0567"]["schema"]["portable"]
        self.assertEqual(
            sam3["outputs"][0]["source_type_name"],
            "SAM3_TRACK_DATA",
        )

        labels = {
            source_type
            for contract in self.contracts.values()
            for source_type in (
                *(
                    source_type
                    for port in contract["schema"]["portable"]["inputs"]
                    for source_type in port["source_type_names"]
                ),
                *(
                    port["source_type_name"]
                    for port in contract["schema"]["portable"]["outputs"]
                ),
            )
        }
        self.assertFalse(
            {"FACEDETECTIONTYPE", "FACELANDMARKSTYPE", "ICLORAPARAMETERS", "SAM3TRACKDATA"}
            & labels
        )

    def test_schema_source_mismatch_fails_closed(self) -> None:
        with generator.INPUT.open(newline="", encoding="utf-8") as handle:
            row = next(
                value
                for value in csv.DictReader(handle)
                if value["feature_id"] == "COMFY-NODE-0494"
            )
        source = generator.source_path(row["source_file"]).read_text(encoding="utf-8")
        _, definition = generator.source_definition(
            source, row["source_symbol"], int(row["source_line"])
        )
        self.assertIsInstance(definition, ast.ClassDef)
        with self.assertRaisesRegex(RuntimeError, "does not match pinned"):
            generator.schema_projection(
                row["schema_source"].replace("PrimitiveBoolean", "WrongBoolean", 1),
                row["schema_api"],
                source,
                definition,
            )

    def test_atomic_write_preserves_previous_catalog_on_replace_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            output = Path(temporary_directory) / "catalog.json"
            output.write_bytes(b"previous\n")
            with (
                mock.patch.object(generator, "OUTPUT", output),
                mock.patch.object(generator, "encoded_catalog", return_value=b"replacement\n"),
                mock.patch.object(generator.os, "replace", side_effect=OSError("injected")),
            ):
                with self.assertRaisesRegex(OSError, "injected"):
                    generator.main()
            self.assertEqual(output.read_bytes(), b"previous\n")
            self.assertEqual(list(output.parent.glob(f".{output.name}.*.tmp")), [])


if __name__ == "__main__":
    unittest.main()
