#!/usr/bin/env python3

import csv
import json
import tempfile
import unittest
from pathlib import Path

import generate_provider_component_catalog as generator


class ProviderComponentCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog, cls.fieldnames, cls.routes = generator.build_catalog()

    def test_catalog_closes_every_provider_node_route_and_vendor(self) -> None:
        summary = self.catalog["summary"]
        self.assertEqual(summary["provider_nodes"], 224)
        self.assertEqual(summary["vendors"], 33)
        self.assertEqual(summary["route_rows"], 217)
        self.assertEqual(summary["resolved_unknown_methods"], 61)
        self.assertEqual(summary["unknown_methods"], 0)
        self.assertEqual(len({item["feature_id"] for item in self.catalog["nodes"]}), 224)
        self.assertEqual(len({item["feature_id"] for item in self.catalog["routes"]}), 217)
        self.assertTrue(all(not item["namespace"].startswith("comfy-node-") for item in self.catalog["nodes"]))
        self.assertTrue(all(not item["namespace"].startswith("comfy-node-") for item in self.catalog["routes"]))

    def test_reviewed_aliases_and_claim_counts_are_exact(self) -> None:
        vendors = {item["vendor"]: item for item in self.catalog["vendors"]}
        self.assertEqual(vendors["bytedance"]["aliases"], ["byteplus", "byteplus-seedance2", "seedance"])
        self.assertEqual(vendors["gemini"]["aliases"], ["vertexai"])
        self.assertEqual(vendors["grok"]["aliases"], ["xai"])
        self.assertEqual(vendors["magnific"]["aliases"], ["freepik"])
        self.assertEqual(vendors["veo2"]["aliases"], ["veo"])
        for vendor, node_count, route_count, aliases in generator.VENDOR_SPECS:
            entry = vendors[vendor]
            self.assertEqual(entry["aliases"], list(aliases))
            self.assertEqual(len(entry["node_feature_ids"]), node_count)
            self.assertEqual(len(entry["route_feature_ids"]), route_count)

    def test_unknown_methods_are_source_reviewed_and_tombstones_are_explicit(self) -> None:
        routes = {item["feature_id"]: item for item in self.catalog["routes"]}
        self.assertEqual(routes["COMFY-API-EXT-0005"]["method"], "POST")
        self.assertEqual(routes["COMFY-API-EXT-0024"]["method"], "GET")
        self.assertEqual(routes["COMFY-API-EXT-0126"]["method"], "GET")
        self.assertEqual(routes["COMFY-API-EXT-0139"]["method"], "POST")
        self.assertEqual(routes["COMFY-API-EXT-0199"]["method"], "GET")
        self.assertEqual(routes["COMFY-API-EXT-0003"]["disposition"], "synthetic_prefix_tombstone")
        self.assertEqual(routes["COMFY-API-EXT-0024"]["disposition"], "executable")
        self.assertTrue(all(item["method"] != "UNKNOWN" for item in routes.values()))
        self.assertEqual(
            {
                feature_id
                for feature_id, item in routes.items()
                if item["disposition"] == "synthetic_prefix_tombstone"
            },
            generator.SYNTHETIC_PREFIX_TOMBSTONES,
        )
        for item in routes.values():
            source = item["source"]
            self.assertTrue(source["path"].startswith("comfy_api_nodes/"))
            self.assertTrue(source["symbol"])
            self.assertGreater(source["line"], 0)
            self.assertRegex(source["sha256"], r"^[0-9a-f]{64}$")

    def test_generation_is_byte_stable_and_rejects_unreviewed_unknown(self) -> None:
        first = json.dumps(self.catalog, indent=2, sort_keys=True) + "\n"
        second = generator.encoded_catalog().decode("utf-8")
        self.assertEqual(first, second)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "routes.csv"
            rows = [dict(item) for item in self.routes]
            rows[0]["method"] = "UNKNOWN"
            rows[0]["feature_id"] = "COMFY-API-EXT-9999"
            path.write_bytes(generator.encode_csv(self.fieldnames, rows))
            with self.assertRaisesRegex(ValueError, "unreviewed UNKNOWN methods"):
                generator.build_catalog(external_services_path=path)


if __name__ == "__main__":
    unittest.main()
