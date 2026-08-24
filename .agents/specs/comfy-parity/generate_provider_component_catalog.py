#!/usr/bin/env python3

from __future__ import annotations

import argparse
import ast
import csv
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[2]
CATALOGS = ROOT / "catalogs"
NODE_CONTRACTS = CATALOGS / "backend-node-contracts.json"
EXTERNAL_SERVICES = CATALOGS / "backend-external-services.csv"
OUTPUT = CATALOGS / "provider-component-contracts.json"
SOURCE_ROOT = WORKSPACE / "projects/comfy/ComfyUI/comfy_api_nodes"

VENDOR_SPECS: tuple[tuple[str, int, int, tuple[str, ...]], ...] = (
    ("anthropic", 1, 1, ()),
    ("beeble", 2, 3, ()),
    ("bfl", 10, 9, ()),
    ("bria", 6, 6, ()),
    ("bytedance", 14, 12, ("byteplus", "byteplus-seedance2", "seedance")),
    ("elevenlabs", 8, 9, ()),
    ("gemini", 8, 4, ("vertexai",)),
    ("grok", 7, 7, ("xai",)),
    ("hitpaw", 2, 3, ()),
    ("hunyuan3d", 6, 10, ("tencent",)),
    ("ideogram", 2, 3, ()),
    ("kling", 25, 29, ()),
    ("krea", 2, 6, ()),
    ("ltxv", 2, 2, ("ltx",)),
    ("luma", 15, 7, ("luma_2",)),
    ("magnific", 5, 15, ("freepik",)),
    ("meshy", 7, 18, ()),
    ("minimax", 3, 3, ()),
    ("openai", 8, 7, ()),
    ("openrouter", 1, 1, ()),
    ("pixverse", 4, 6, ()),
    ("quiver", 2, 2, ()),
    ("recraft", 18, 9, ()),
    ("reve", 3, 3, ()),
    ("rodin", 7, 3, ()),
    ("runway", 7, 4, ()),
    ("sonilo", 2, 2, ()),
    ("topaz", 3, 9, ()),
    ("tripo", 12, 4, ()),
    ("veo2", 3, 3, ("veo",)),
    ("vidu", 13, 7, ()),
    ("wan", 14, 5, ()),
    ("wavespeed", 2, 5, ()),
)

RESOLVED_UNKNOWN_METHODS = {
    "COMFY-API-EXT-0003": "GET",
    "COMFY-API-EXT-0005": "POST",
    "COMFY-API-EXT-0007": "POST",
    "COMFY-API-EXT-0015": "GET",
    "COMFY-API-EXT-0024": "GET",
    "COMFY-API-EXT-0027": "POST",
    "COMFY-API-EXT-0031": "POST",
    "COMFY-API-EXT-0035": "GET",
    "COMFY-API-EXT-0038": "GET",
    "COMFY-API-EXT-0042": "GET",
    "COMFY-API-EXT-0044": "GET",
    "COMFY-API-EXT-0046": "POST",
    "COMFY-API-EXT-0055": "POST",
    "COMFY-API-EXT-0060": "GET",
    "COMFY-API-EXT-0063": "GET",
    "COMFY-API-EXT-0066": "GET",
    "COMFY-API-EXT-0069": "GET",
    "COMFY-API-EXT-0072": "GET",
    "COMFY-API-EXT-0075": "GET",
    "COMFY-API-EXT-0085": "POST",
    "COMFY-API-EXT-0086": "POST",
    "COMFY-API-EXT-0087": "POST",
    "COMFY-API-EXT-0088": "GET",
    "COMFY-API-EXT-0093": "GET",
    "COMFY-API-EXT-0097": "GET",
    "COMFY-API-EXT-0100": "GET",
    "COMFY-API-EXT-0103": "GET",
    "COMFY-API-EXT-0106": "GET",
    "COMFY-API-EXT-0109": "GET",
    "COMFY-API-EXT-0112": "GET",
    "COMFY-API-EXT-0115": "GET",
    "COMFY-API-EXT-0124": "GET",
    "COMFY-API-EXT-0126": "GET",
    "COMFY-API-EXT-0130": "GET",
    "COMFY-API-EXT-0137": "POST",
    "COMFY-API-EXT-0138": "POST",
    "COMFY-API-EXT-0139": "POST",
    "COMFY-API-EXT-0140": "POST",
    "COMFY-API-EXT-0141": "POST",
    "COMFY-API-EXT-0142": "POST",
    "COMFY-API-EXT-0143": "POST",
    "COMFY-API-EXT-0152": "GET",
    "COMFY-API-EXT-0156": "GET",
    "COMFY-API-EXT-0160": "GET",
    "COMFY-API-EXT-0174": "GET",
    "COMFY-API-EXT-0177": "GET",
    "COMFY-API-EXT-0185": "GET",
    "COMFY-API-EXT-0187": "POST",
    "COMFY-API-EXT-0190": "POST",
    "COMFY-API-EXT-0191": "POST",
    "COMFY-API-EXT-0194": "POST",
    "COMFY-API-EXT-0195": "POST",
    "COMFY-API-EXT-0196": "POST",
    "COMFY-API-EXT-0197": "POST",
    "COMFY-API-EXT-0198": "POST",
    "COMFY-API-EXT-0199": "GET",
    "COMFY-API-EXT-0200": "POST",
    "COMFY-API-EXT-0204": "GET",
    "COMFY-API-EXT-0206": "GET",
    "COMFY-API-EXT-0208": "POST",
    "COMFY-API-EXT-0213": "GET",
}

SYNTHETIC_PREFIX_TOMBSTONES = {
    "COMFY-API-EXT-0003",
    "COMFY-API-EXT-0015",
    "COMFY-API-EXT-0027",
    "COMFY-API-EXT-0031",
    "COMFY-API-EXT-0035",
    "COMFY-API-EXT-0038",
    "COMFY-API-EXT-0042",
    "COMFY-API-EXT-0044",
    "COMFY-API-EXT-0046",
    "COMFY-API-EXT-0055",
    "COMFY-API-EXT-0060",
    "COMFY-API-EXT-0063",
    "COMFY-API-EXT-0066",
    "COMFY-API-EXT-0069",
    "COMFY-API-EXT-0072",
    "COMFY-API-EXT-0075",
    "COMFY-API-EXT-0088",
    "COMFY-API-EXT-0093",
    "COMFY-API-EXT-0097",
    "COMFY-API-EXT-0100",
    "COMFY-API-EXT-0103",
    "COMFY-API-EXT-0106",
    "COMFY-API-EXT-0109",
    "COMFY-API-EXT-0112",
    "COMFY-API-EXT-0115",
    "COMFY-API-EXT-0124",
    "COMFY-API-EXT-0130",
    "COMFY-API-EXT-0156",
    "COMFY-API-EXT-0160",
    "COMFY-API-EXT-0174",
    "COMFY-API-EXT-0177",
    "COMFY-API-EXT-0185",
    "COMFY-API-EXT-0187",
    "COMFY-API-EXT-0190",
    "COMFY-API-EXT-0191",
    "COMFY-API-EXT-0204",
    "COMFY-API-EXT-0206",
    "COMFY-API-EXT-0208",
    "COMFY-API-EXT-0213",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_vendor_map() -> dict[str, str]:
    result: dict[str, str] = {}
    for vendor, _, _, aliases in VENDOR_SPECS:
        if vendor in result:
            raise ValueError(f"duplicate vendor {vendor}")
        result[vendor] = vendor
        for alias in aliases:
            if alias in result:
                raise ValueError(f"duplicate provider alias {alias}")
            result[alias] = vendor
    return result


def read_csv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"missing CSV header in {path}")
        return list(reader.fieldnames), list(reader)


def source_snapshot(source_root: Path = SOURCE_ROOT) -> dict[str, object]:
    records = []
    for path in sorted(source_root.rglob("*.py")):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"unsupported source entry {path}")
        data = path.read_bytes()
        records.append((path.relative_to(source_root).as_posix(), sha256(data)))
    digest = hashlib.sha256()
    for relative, file_sha in records:
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_sha.encode("ascii"))
        digest.update(b"\n")
    return {
        "root": "projects/comfy/ComfyUI/comfy_api_nodes",
        "files": len(records),
        "tree_sha256": digest.hexdigest(),
    }


def enclosing_symbol(source: str, line: int) -> str:
    tree = ast.parse(source)
    candidates: list[tuple[int, str]] = []
    for node in ast.walk(tree):
        if not isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        end_line = getattr(node, "end_lineno", node.lineno)
        if node.lineno <= line <= end_line:
            candidates.append((end_line - node.lineno, node.name))
    return min(candidates)[1] if candidates else "<module>"


def route_source(row: dict[str, str], source_root: Path = SOURCE_ROOT) -> dict[str, object]:
    primary = row["source_evidence"].split(" | ", 1)[0]
    relative, line_text = primary.rsplit(":", 1)
    if not line_text.isdigit():
        raise ValueError(f"invalid source evidence for {row['feature_id']}: {primary}")
    path = source_root.parent / relative
    data = path.read_bytes()
    line = int(line_text)
    source = data.decode("utf-8")
    if line < 1 or line > len(source.splitlines()):
        raise ValueError(f"source line outside file for {row['feature_id']}")
    return {
        "path": relative,
        "symbol": enclosing_symbol(source, line),
        "line": line,
        "sha256": sha256(data),
    }


def resolve_methods(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    present_feature_ids = {row["feature_id"] for row in rows}
    missing = sorted(set(RESOLVED_UNKNOWN_METHODS) - present_feature_ids)
    if missing:
        raise ValueError(f"reviewed method rows are missing: {missing}")
    resolved = []
    for original in rows:
        row = dict(original)
        feature_id = row["feature_id"]
        expected_method = RESOLVED_UNKNOWN_METHODS.get(feature_id)
        if expected_method is not None:
            if row["method"] not in {"UNKNOWN", expected_method}:
                raise ValueError(
                    f"reviewed method drift for {feature_id}: "
                    f"expected UNKNOWN or {expected_method}, found {row['method']}"
                )
            row["method"] = expected_method
        elif row["method"] == "UNKNOWN":
            raise ValueError(f"unreviewed UNKNOWN methods: {feature_id}")
        if row["method"] not in {"GET", "POST", "PUT", "PATCH", "DELETE"}:
            raise ValueError(f"unsupported HTTP method for {row['feature_id']}: {row['method']}")
        resolved.append(row)
    return resolved


def node_vendor(contract: dict[str, object]) -> str:
    path = str(contract["source"]["path"])
    stem = Path(path).stem
    if not stem.startswith("nodes_"):
        raise ValueError(f"provider node has unsupported source owner {path}")
    source_vendor = stem.removeprefix("nodes_")
    aliases = canonical_vendor_map()
    if source_vendor == "bytedance_llm":
        return "bytedance"
    if source_vendor == "sora":
        return "openai"
    try:
        return aliases[source_vendor]
    except KeyError as error:
        raise ValueError(f"unreviewed provider node owner {source_vendor}") from error


def build_catalog(
    node_contract_path: Path = NODE_CONTRACTS,
    external_services_path: Path = EXTERNAL_SERVICES,
    source_root: Path = SOURCE_ROOT,
) -> tuple[dict[str, object], list[str], list[dict[str, str]]]:
    node_data = json.loads(node_contract_path.read_text(encoding="utf-8"))
    provider_nodes = [
        contract
        for contract in node_data["contracts"]
        if contract["binding_disposition"] == "provider_required"
    ]
    if len(provider_nodes) != 224:
        raise ValueError(f"expected 224 provider nodes, found {len(provider_nodes)}")

    fieldnames, original_routes = read_csv(external_services_path)
    if len(original_routes) != 217:
        raise ValueError(f"expected 217 external-service rows, found {len(original_routes)}")
    resolved_routes = resolve_methods(original_routes)
    aliases = canonical_vendor_map()

    nodes = []
    for contract in provider_nodes:
        vendor = node_vendor(contract)
        source = contract["source"]
        nodes.append({
            "feature_id": contract["feature_id"],
            "node_identifier": contract["node_identifier"],
            "vendor": vendor,
            "namespace": f"zed.comfy.provider.{vendor}",
            "disposition": "provider_required",
            "source": {
                "path": source["path"],
                "symbol": source["symbol"]["symbol"],
                "line": source["symbol"]["line"],
                "sha256": source["sha256"],
            },
        })

    routes = []
    for original, row in zip(original_routes, resolved_routes, strict=True):
        try:
            vendor = aliases[row["provider"]]
        except KeyError as error:
            raise ValueError(f"unreviewed route provider {row['provider']}") from error
        feature_id = row["feature_id"]
        routes.append({
            "feature_id": feature_id,
            "method": row["method"],
            "original_method": (
                "UNKNOWN" if feature_id in RESOLVED_UNKNOWN_METHODS else original["method"]
            ),
            "path": row["path"],
            "provider": row["provider"],
            "vendor": vendor,
            "namespace": f"zed.comfy.provider.{vendor}",
            "disposition": (
                "synthetic_prefix_tombstone"
                if feature_id in SYNTHETIC_PREFIX_TOMBSTONES
                else "executable"
            ),
            "source": route_source(row, source_root),
        })

    nodes.sort(key=lambda item: item["feature_id"])
    routes.sort(key=lambda item: item["feature_id"])
    node_ids = [item["feature_id"] for item in nodes]
    route_ids = [item["feature_id"] for item in routes]
    if len(node_ids) != len(set(node_ids)) or len(route_ids) != len(set(route_ids)):
        raise ValueError("duplicate provider node or route claim")
    missing_tombstones = sorted(SYNTHETIC_PREFIX_TOMBSTONES - set(route_ids))
    if missing_tombstones:
        raise ValueError(f"reviewed route tombstones are missing: {missing_tombstones}")
    unsupported_tombstones = sorted(
        SYNTHETIC_PREFIX_TOMBSTONES - set(RESOLVED_UNKNOWN_METHODS)
    )
    if unsupported_tombstones:
        raise ValueError(f"unsupported synthetic route rows: {unsupported_tombstones}")

    vendors = []
    for vendor, expected_nodes, expected_routes, reviewed_aliases in VENDOR_SPECS:
        vendor_nodes = [item["feature_id"] for item in nodes if item["vendor"] == vendor]
        vendor_routes = [item["feature_id"] for item in routes if item["vendor"] == vendor]
        if (len(vendor_nodes), len(vendor_routes)) != (expected_nodes, expected_routes):
            raise ValueError(
                f"claim count drift for {vendor}: "
                f"nodes={len(vendor_nodes)}/{expected_nodes}, routes={len(vendor_routes)}/{expected_routes}"
            )
        vendors.append({
            "vendor": vendor,
            "namespace": f"zed.comfy.provider.{vendor}",
            "aliases": list(reviewed_aliases),
            "node_feature_ids": vendor_nodes,
            "route_feature_ids": vendor_routes,
        })

    catalog = {
        "schema_version": 1,
        "classification": "source-fingerprinted provider component contract catalog",
        "input": {
            "backend_node_contracts_sha256": sha256(node_contract_path.read_bytes()),
            "backend_external_services_sha256": sha256(
                encode_csv(fieldnames, resolved_routes)
            ),
        },
        "source_snapshot": source_snapshot(source_root),
        "summary": {
            "provider_nodes": len(nodes),
            "vendors": len(vendors),
            "route_rows": len(routes),
            "resolved_unknown_methods": len(RESOLVED_UNKNOWN_METHODS),
            "unknown_methods": sum(item["method"] == "UNKNOWN" for item in routes),
            "synthetic_prefix_tombstones": sum(
                item["disposition"] == "synthetic_prefix_tombstone" for item in routes
            ),
        },
        "vendors": vendors,
        "nodes": nodes,
        "routes": routes,
    }
    return catalog, fieldnames, resolved_routes


def encode_csv(fieldnames: list[str], rows: list[dict[str, str]]) -> bytes:
    from io import StringIO

    output = StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue().encode("utf-8")


def encoded_catalog() -> bytes:
    catalog, _, _ = build_catalog()
    return (json.dumps(catalog, indent=2, sort_keys=True) + "\n").encode("utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    catalog, fieldnames, routes = build_catalog()
    catalog_bytes = (json.dumps(catalog, indent=2, sort_keys=True) + "\n").encode("utf-8")
    route_bytes = encode_csv(fieldnames, routes)
    if arguments.check:
        stale = []
        for path, expected in ((OUTPUT, catalog_bytes), (EXTERNAL_SERVICES, route_bytes)):
            if not path.exists() or path.read_bytes() != expected:
                stale.append(path.relative_to(WORKSPACE).as_posix())
        if stale:
            raise SystemExit(f"stale provider contract artifacts: {stale}")
        return
    OUTPUT.write_bytes(catalog_bytes)
    EXTERNAL_SERVICES.write_bytes(route_bytes)
    print(
        f"Generated {len(catalog['nodes'])} provider nodes, "
        f"{len(catalog['vendors'])} vendors, and {len(catalog['routes'])} routes."
    )


if __name__ == "__main__":
    main()
