#!/usr/bin/env python3

from __future__ import annotations

import ast
import copy
import csv
import hashlib
import io
import json
import os
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[2]
CATALOGS = ROOT / "catalogs"
SOURCE_ROOT = WORKSPACE / "projects/comfy/ComfyUI"
INPUT = CATALOGS / "backend-nodes.csv"
OUTPUT = CATALOGS / "backend-node-contracts.json"
SCHEMA_VERSION = 2
MAX_SCHEMA_SOURCE_BYTES = 4 * 1024 * 1024


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def source_path(relative: str) -> Path:
    if not relative or relative.startswith(("/", ".")) or "\\" in relative:
        raise RuntimeError(f"invalid node source path: {relative!r}")
    candidate = SOURCE_ROOT / relative
    resolved_root = SOURCE_ROOT.resolve()
    resolved = candidate.resolve(strict=True)
    if resolved_root not in resolved.parents or candidate.is_symlink():
        raise RuntimeError(f"node source escapes the pinned snapshot: {relative}")
    return candidate


def dotted_name(node: ast.AST) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        parent = dotted_name(node.value)
        if parent is not None:
            return f"{parent}.{node.attr}"
    return None


def expression_projection(node: ast.AST, source: str) -> dict[str, Any]:
    segment = ast.get_source_segment(source, node)
    projection: dict[str, Any] = {}
    if isinstance(node, ast.Constant) and isinstance(
        node.value, (str, int, float, bool, type(None))
    ):
        projection["kind"] = "literal"
        projection["value"] = node.value
        if type(node.value) in {int, float}:
            projection["source"] = segment if segment is not None else ""
    elif isinstance(node, ast.Name):
        projection["kind"] = "name"
        projection["name"] = node.id
    elif isinstance(node, ast.Attribute):
        projection["kind"] = "attribute"
        projection["name"] = dotted_name(node) or projection["source"]
    elif isinstance(node, ast.Call):
        projection["kind"] = "call"
        projection["name"] = dotted_name(node.func) or (
            ast.get_source_segment(source, node.func) or ""
        )
        projection["arguments"] = [
            expression_projection(argument, source) for argument in node.args
        ]
        projection["keywords"] = [
            {
                "name": keyword.arg,
                "value": expression_projection(keyword.value, source),
            }
            for keyword in node.keywords
        ]
    elif isinstance(node, (ast.List, ast.Tuple, ast.Set)):
        projection["kind"] = type(node).__name__.casefold()
        projection["items"] = [
            expression_projection(element, source) for element in node.elts
        ]
    elif isinstance(node, ast.Dict):
        projection["kind"] = "dict"
        projection["entries"] = [
            {
                "key": expression_projection(key, source) if key is not None else None,
                "value": expression_projection(value, source),
            }
            for key, value in zip(node.keys, node.values, strict=True)
        ]
    elif isinstance(node, ast.UnaryOp):
        projection["kind"] = "unary"
        projection["operator"] = type(node.op).__name__
        projection["operand"] = expression_projection(node.operand, source)
    elif isinstance(node, ast.BinOp):
        projection["kind"] = "binary"
        projection["operator"] = type(node.op).__name__
        projection["left"] = expression_projection(node.left, source)
        projection["right"] = expression_projection(node.right, source)
    elif isinstance(node, ast.Subscript):
        projection["kind"] = "subscript"
        projection["value"] = expression_projection(node.value, source)
        projection["slice"] = expression_projection(node.slice, source)
    else:
        projection["kind"] = type(node).__name__.casefold()
        projection["source"] = segment if segment is not None else ""
        projection["sha256"] = sha256_bytes(projection["source"].encode("utf-8"))
    return projection


def schema_expression(schema_source: str) -> tuple[str, ast.AST] | None:
    encoded = schema_source.encode("utf-8")
    if len(encoded) > MAX_SCHEMA_SOURCE_BYTES:
        raise RuntimeError("node schema source exceeds its bound")
    candidates = [schema_source]
    if "return " in schema_source:
        returned = schema_source.split("return ", 1)[1]
        candidates.insert(0, returned.split(" | ", 1)[0])
    for candidate in candidates:
        try:
            return candidate, ast.parse(candidate, mode="eval").body
        except SyntaxError:
            continue
    return None


def returned_expression(definition: ast.AST) -> ast.AST | None:
    if not isinstance(definition, (ast.FunctionDef, ast.AsyncFunctionDef)):
        return None
    returns = [
        node
        for node in ast.walk(definition)
        if isinstance(node, ast.Return) and node.value is not None
    ]
    if not returns:
        return None
    value = returns[0].value
    if isinstance(value, ast.Name) and isinstance(
        definition, (ast.FunctionDef, ast.AsyncFunctionDef)
    ):
        for statement in reversed(definition.body):
            if isinstance(statement, ast.Assign) and any(
                isinstance(target, ast.Name) and target.id == value.id
                for target in statement.targets
            ):
                return statement.value
            if (
                isinstance(statement, ast.AnnAssign)
                and isinstance(statement.target, ast.Name)
                and statement.target.id == value.id
                and statement.value is not None
            ):
                return statement.value
    return value


class CatalogCorrelationNormalizer(ast.NodeTransformer):
    def visit_Constant(self, node: ast.Constant) -> ast.AST:
        if isinstance(node.value, str):
            return ast.copy_location(ast.Constant(value=" ".join(node.value.split())), node)
        return node


def correlation_dump(node: ast.AST) -> str:
    normalized = CatalogCorrelationNormalizer().visit(copy.deepcopy(node))
    return ast.dump(normalized, include_attributes=False)


def statement_projection(statement: ast.stmt, source: str) -> dict[str, Any]:
    segment = ast.get_source_segment(source, statement) or ""
    projection: dict[str, Any] = {
        "kind": type(statement).__name__.casefold(),
        "source": segment,
        "sha256": sha256_bytes(segment.encode("utf-8")),
    }
    if isinstance(statement, (ast.Assign, ast.AnnAssign)):
        targets = statement.targets if isinstance(statement, ast.Assign) else [statement.target]
        projection["targets"] = [
            dotted_name(target) or ast.get_source_segment(source, target) or ""
            for target in targets
        ]
        value = statement.value
        if value is not None:
            projection["value"] = expression_projection(value, source)
    elif isinstance(statement, ast.Return) and statement.value is not None:
        projection["value"] = expression_projection(statement.value, source)
    return projection


def literal_string(node: ast.AST | None) -> str | None:
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    return None


def dict_entries(node: ast.AST | None) -> list[tuple[ast.AST, ast.AST]]:
    if not isinstance(node, ast.Dict):
        return []
    return [
        (key, value)
        for key, value in zip(node.keys, node.values, strict=True)
        if key is not None
    ]


def v3_port_projection(node: ast.AST, source: str) -> dict[str, Any]:
    projection = expression_projection(node, source)
    if isinstance(node, ast.Call):
        projection["constructor"] = dotted_name(node.func) or ""
        projection["name"] = literal_string(node.args[0]) if node.args else None
    return projection


def v3_schema_contract(
    definition: ast.AST, source: str, expression: ast.AST
) -> dict[str, Any]:
    if not isinstance(expression, ast.Call) or not (
        dotted_name(expression.func) or ""
    ).endswith("Schema"):
        return {
            "status": "preserved_source_definition",
            "expression": expression_projection(expression, source),
        }
    keywords = {keyword.arg: keyword.value for keyword in expression.keywords if keyword.arg}
    inputs = keywords.get("inputs")
    outputs = keywords.get("outputs")
    hidden = keywords.get("hidden")
    body = definition.body if isinstance(definition, (ast.FunctionDef, ast.AsyncFunctionDef)) else []
    return {
        "status": "normalized_v3",
        "bindings": [
            statement_projection(statement, source)
            for statement in body
            if isinstance(statement, (ast.Assign, ast.AnnAssign))
        ],
        "inputs": [v3_port_projection(value, source) for value in inputs.elts]
        if isinstance(inputs, (ast.List, ast.Tuple))
        else ([expression_projection(inputs, source)] if inputs is not None else []),
        "outputs": [v3_port_projection(value, source) for value in outputs.elts]
        if isinstance(outputs, (ast.List, ast.Tuple))
        else ([expression_projection(outputs, source)] if outputs is not None else []),
        "hidden": [expression_projection(value, source) for value in hidden.elts]
        if isinstance(hidden, (ast.List, ast.Tuple))
        else ([expression_projection(hidden, source)] if hidden is not None else []),
        "node_options": [
            {"name": keyword.arg, "value": expression_projection(keyword.value, source)}
            for keyword in expression.keywords
            if keyword.arg not in {"inputs", "outputs", "hidden"}
        ],
    }


def class_assignment(class_definition: ast.ClassDef, name: str) -> ast.AST | None:
    for statement in class_definition.body:
        if isinstance(statement, ast.Assign):
            if any(isinstance(target, ast.Name) and target.id == name for target in statement.targets):
                return statement.value
        elif isinstance(statement, ast.AnnAssign):
            if isinstance(statement.target, ast.Name) and statement.target.id == name:
                return statement.value
    return None


def inherited_v3_context(
    source: str,
    class_definition: ast.ClassDef,
    method: ast.AST,
    catalog_expression: ast.AST,
) -> tuple[ast.AST, ast.AST, list[dict[str, Any]]] | None:
    if not isinstance(method, (ast.FunctionDef, ast.AsyncFunctionDef)):
        return None
    return_nodes = [statement for statement in method.body if isinstance(statement, ast.Return)]
    if len(return_nodes) != 1 or not isinstance(return_nodes[0].value, ast.Name):
        return None
    schema_name = return_nodes[0].value.id
    initialization = next(
        (
            statement
            for statement in method.body
            if isinstance(statement, ast.Assign)
            and any(
                isinstance(target, ast.Name) and target.id == schema_name
                for target in statement.targets
            )
        ),
        None,
    )
    if initialization is None or not isinstance(initialization.value, ast.Call):
        return None
    function = initialization.value.func
    if not isinstance(function, ast.Attribute) or function.attr != "define_schema":
        return None
    if not (
        isinstance(function.value, ast.Call)
        and isinstance(function.value.func, ast.Name)
        and function.value.func.id == "super"
    ):
        return None
    tree = ast.parse(source)
    classes = {
        node.name: node for node in tree.body if isinstance(node, ast.ClassDef)
    }
    base_names = [dotted_name(base) for base in class_definition.bases]
    base_class = next(
        (classes[name] for name in base_names if name is not None and name in classes),
        None,
    )
    if base_class is None:
        return None
    base_methods = [
        statement
        for statement in base_class.body
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef))
        and statement.name == "define_schema"
    ]
    if len(base_methods) != 1:
        return None
    base_expression = returned_expression(base_methods[0])
    if base_expression is None or correlation_dump(base_expression) != correlation_dump(
        catalog_expression
    ):
        return None
    overrides = []
    allowed_statement_ids = {id(initialization), id(return_nodes[0])}
    for statement in method.body:
        if id(statement) in allowed_statement_ids:
            continue
        if not isinstance(statement, (ast.Assign, ast.AnnAssign)):
            return None
        targets = statement.targets if isinstance(statement, ast.Assign) else [statement.target]
        if len(targets) != 1 or not isinstance(targets[0], ast.Attribute):
            return None
        target = targets[0]
        if not isinstance(target.value, ast.Name) or target.value.id != schema_name:
            return None
        value = statement.value
        if value is None:
            return None
        overrides.append(
            {
                "name": target.attr,
                "value": expression_projection(value, source),
            }
        )
    return base_methods[0], base_expression, overrides


def v1_schema_contract(
    class_definition: ast.ClassDef,
    method: ast.AST,
    source: str,
    expression: ast.AST,
) -> dict[str, Any]:
    groups = []
    for group_key, group_value in dict_entries(expression):
        group_name = literal_string(group_key)
        fields = []
        for field_key, field_value in dict_entries(group_value):
            fields.append(
                {
                    "name": literal_string(field_key),
                    "contract": expression_projection(field_value, source),
                }
            )
        groups.append(
            {
                "name": group_name,
                "fields": fields,
                "expression": expression_projection(group_value, source),
            }
        )
    metadata_names = (
        "RETURN_TYPES",
        "RETURN_NAMES",
        "OUTPUT_IS_LIST",
        "INPUT_IS_LIST",
        "FUNCTION",
        "CATEGORY",
        "DESCRIPTION",
        "OUTPUT_NODE",
        "DEPRECATED",
        "EXPERIMENTAL",
    )
    return {
        "status": "normalized_v1" if isinstance(expression, ast.Dict) else "preserved_v1",
        "bindings": [
            statement_projection(statement, source)
            for statement in method.body
            if isinstance(statement, (ast.Assign, ast.AnnAssign))
        ]
        if isinstance(method, (ast.FunctionDef, ast.AsyncFunctionDef))
        else [],
        "input_groups": groups,
        "input_expression": expression_projection(expression, source),
        "class_metadata": [
            {
                "name": name,
                "value": expression_projection(value, source),
            }
            for name in metadata_names
            if (value := class_assignment(class_definition, name)) is not None
        ],
    }


def schema_projection(
    schema_source: str,
    schema_api: str,
    source: str,
    class_definition: ast.ClassDef,
) -> dict[str, Any]:
    digest = sha256_bytes(schema_source.encode("utf-8"))
    method_name = "define_schema" if schema_api == "V3" else "INPUT_TYPES"
    methods = [
        statement
        for statement in class_definition.body
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef))
        and statement.name == method_name
    ]
    if not methods:
        if schema_api == "V3":
            tree = ast.parse(source)
            classes = {
                node.name: node for node in tree.body if isinstance(node, ast.ClassDef)
            }
            base_class = next(
                (
                    classes[name]
                    for base in class_definition.bases
                    if (name := dotted_name(base)) is not None and name in classes
                ),
                None,
            )
            base_methods = [
                statement
                for statement in base_class.body
                if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef))
                and statement.name == method_name
            ] if base_class is not None else []
            parsed_catalog = schema_expression(schema_source)
            base_expression = returned_expression(base_methods[0]) if len(base_methods) == 1 else None
            if (
                parsed_catalog is not None
                and base_expression is not None
                and correlation_dump(parsed_catalog[1]) == correlation_dump(base_expression)
            ):
                contract = v3_schema_contract(base_methods[0], source, base_expression)
                contract["class_overrides"] = [
                    statement_projection(statement, source)
                    for statement in class_definition.body
                    if isinstance(statement, (ast.Assign, ast.AnnAssign))
                ]
                return {
                    "status": contract["status"],
                    "catalog_source": schema_source,
                    "catalog_sha256": digest,
                    "catalog_correlation": "verified_inherited_method",
                    "method": method_name,
                    "definition_sha256": sha256_bytes(
                        (
                            (ast.get_source_segment(source, base_methods[0]) or "")
                            + "\0"
                            + (ast.get_source_segment(source, class_definition) or "")
                        ).encode("utf-8")
                    ),
                    "contract": contract,
                }
        return {
            "status": "preserved_reference",
            "catalog_source": schema_source,
            "catalog_sha256": digest,
            "method": method_name,
        }
    method = methods[0]
    expression = returned_expression(method)
    if expression is None:
        return {
            "status": "preserved_source_definition",
            "catalog_source": schema_source,
            "catalog_sha256": digest,
            "method": method_name,
            "definition": statement_projection(method, source),
        }
    parsed_catalog = schema_expression(schema_source)
    inherited_context = None
    if parsed_catalog is not None and correlation_dump(parsed_catalog[1]) != correlation_dump(
        expression
    ):
        inherited_context = (
            inherited_v3_context(
                source, class_definition, method, parsed_catalog[1]
            )
            if schema_api == "V3"
            else None
        )
    if parsed_catalog is None or (
        correlation_dump(parsed_catalog[1]) != correlation_dump(expression)
        and inherited_context is None
    ):
        raise RuntimeError(
            f"catalog schema does not match pinned {class_definition.name}.{method_name} source"
        )
    contract_method = method
    contract_expression = expression
    correlation = "direct"
    overrides: list[dict[str, Any]] = []
    if inherited_context is not None:
        contract_method, contract_expression, overrides = inherited_context
        correlation = "verified_inherited_base"
    contract = (
        v3_schema_contract(contract_method, source, contract_expression)
        if schema_api == "V3"
        else v1_schema_contract(class_definition, method, source, contract_expression)
    )
    if overrides:
        contract["inherited_overrides"] = overrides
    return {
        "status": contract["status"],
        "catalog_source": schema_source,
        "catalog_sha256": digest,
        "catalog_correlation": correlation,
        "method": method_name,
        "definition_sha256": sha256_bytes(
            (ast.get_source_segment(source, method) or "").encode("utf-8")
        ),
        "contract": contract,
    }


def source_definition(
    source: str, symbol: str, expected_line: int | None
) -> tuple[dict[str, Any], ast.AST]:
    tree = ast.parse(source)
    candidates = [
        node
        for node in ast.walk(tree)
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))
        and node.name == symbol
    ]
    if not candidates:
        raise RuntimeError(f"node source symbol is absent from pinned source: {symbol}")
    selected = min(
        candidates,
        key=lambda node: abs(node.lineno - expected_line) if expected_line else node.lineno,
    )
    segment = ast.get_source_segment(source, selected) or ""
    return (
        {
            "status": "parsed_definition",
            "symbol": symbol,
            "kind": type(selected).__name__,
            "line": selected.lineno,
            "end_line": selected.end_lineno,
            "sha256": sha256_bytes(segment.encode("utf-8")),
        },
        selected,
    )


def build_catalog() -> dict[str, Any]:
    input_bytes = INPUT.read_bytes()
    rows = list(csv.DictReader(io.StringIO(input_bytes.decode("utf-8"), newline="")))
    contracts = []
    source_digests: dict[str, str] = {}
    for row in sorted(rows, key=lambda value: value["feature_id"]):
        path = source_path(row["source_file"])
        source_bytes = path.read_bytes()
        source = source_bytes.decode("utf-8")
        digest = sha256_bytes(source_bytes)
        previous = source_digests.setdefault(row["source_file"], digest)
        if previous != digest:
            raise RuntimeError(f"node source digest changed during generation: {path}")
        provider_required = row["availability"] == "cloud/paid"
        source_line = int(row["source_line"]) if row["source_line"] else None
        symbol_projection, definition = source_definition(
            source, row["source_symbol"], source_line
        )
        if not isinstance(definition, ast.ClassDef):
            raise RuntimeError(
                f"registered node source symbol is not a class: {row['source_symbol']}"
            )
        contracts.append(
            {
                "feature_id": row["feature_id"],
                "node_identifier": row["node_identifier"],
                "category": row["category"],
                "classification": row["classification"],
                "availability": row["availability"],
                "binding_disposition": (
                    "provider_required" if provider_required else "executable"
                ),
                "source": {
                    "path": row["source_file"],
                    "sha256": digest,
                    "symbol": symbol_projection,
                    "catalog_line": source_line,
                },
                "schema_api": row["schema_api"],
                "schema": schema_projection(
                    row["schema_source"], row["schema_api"], source, definition
                ),
                "input_is_list": row["input_is_list"],
                "output_is_list": row["output_is_list"],
                "lazy_inputs": row["lazy_inputs"],
                "output_node": row["output_node"] == "True",
                "capability_hints": {
                    "provider": provider_required,
                    "asset_or_effect": (
                        not provider_required
                        and (
                            row["output_node"] == "True"
                            or row["category"].split("/", 1)[0]
                            in {"3d", "audio", "image", "video"}
                            or row["category"].startswith("model/loaders")
                            or "upload=" in row["schema_source"]
                            or row["change_detection"]
                            not in {"", "default input/upstream signature"}
                        )
                    ),
                },
            }
        )
    feature_ids = [contract["feature_id"] for contract in contracts]
    if len(feature_ids) != len(set(feature_ids)):
        raise RuntimeError("node contract catalog contains duplicate feature IDs")
    dispositions = Counter(contract["binding_disposition"] for contract in contracts)
    schema_statuses = Counter(contract["schema"]["status"] for contract in contracts)
    source_manifest = "\n".join(
        f"{path}\0{digest}" for path, digest in sorted(source_digests.items())
    ).encode("utf-8")
    return {
        "schema_version": SCHEMA_VERSION,
        "classification": "source-correlated AST-only normalized V1/V3 node contracts; no ComfyUI import or execution",
        "input": {
            "path": "catalogs/backend-nodes.csv",
            "sha256": sha256_bytes(input_bytes),
        },
        "source_snapshot": {
            "root": "projects/comfy/ComfyUI",
            "files": len(source_digests),
            "manifest_sha256": sha256_bytes(source_manifest),
        },
        "summary": {
            "rows": len(contracts),
            "executable": dispositions["executable"],
            "provider_required": dispositions["provider_required"],
            "normalized_v3": schema_statuses["normalized_v3"],
            "normalized_v1": schema_statuses["normalized_v1"],
            "preserved_schema_contracts": sum(
                count
                for status, count in schema_statuses.items()
                if not status.startswith("normalized_")
            ),
        },
        "contracts": contracts,
    }


def encoded_catalog() -> bytes:
    return (json.dumps(build_catalog(), indent=2, sort_keys=True) + "\n").encode("utf-8")


def main() -> None:
    encoded = encoded_catalog()
    descriptor, temporary_name = tempfile.mkstemp(
        dir=OUTPUT.parent, prefix=f".{OUTPUT.name}.", suffix=".tmp"
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, OUTPUT)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise
    print(f"Generated {OUTPUT.relative_to(WORKSPACE)}")


if __name__ == "__main__":
    main()
