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
        projection["name"] = dotted_name(node) or segment or ""
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
        projection["callee"] = expression_projection(node.func, source)
        if isinstance(node.func, ast.Attribute):
            projection["source_type"] = expression_projection(node.func.value, source)
        projection["name"] = literal_string(node.args[0]) if node.args else None
    return projection


def portable_expression(value: dict[str, Any]) -> dict[str, Any]:
    kind = value.get("kind")
    if kind == "literal":
        literal = value.get("value")
        if literal is None:
            return {"kind": "null"}
        if isinstance(literal, bool):
            return {"kind": "boolean", "value": literal}
        if isinstance(literal, int):
            if literal < 0:
                if literal < -(2**63):
                    return preserved_expression(value)
                return {"kind": "signed_integer", "value": literal}
            if literal <= 2**64 - 1:
                return {"kind": "unsigned_integer", "value": literal}
            return preserved_expression(value)
        if isinstance(literal, float):
            return {
                "kind": "finite_decimal",
                "value": value.get("source") or repr(literal),
            }
        if isinstance(literal, str):
            return {"kind": "string", "value": literal}
    if kind in {"list", "tuple", "set"}:
        return {
            "kind": "list",
            "values": [portable_expression(item) for item in value.get("items", [])],
        }
    if kind == "dict":
        fields = []
        for entry in value.get("entries", []):
            key = entry.get("key")
            if not (
                isinstance(key, dict)
                and key.get("kind") == "literal"
                and isinstance(key.get("value"), str)
            ):
                return preserved_expression(value)
            fields.append(
                {
                    "name": key["value"],
                    "value": portable_expression(entry["value"]),
                }
            )
        return {"kind": "object", "fields": fields}
    return preserved_expression(value)


def preserved_expression(value: dict[str, Any]) -> dict[str, Any]:
    source = value.get("source")
    if not isinstance(source, str) or not source:
        source = json.dumps(value, separators=(",", ":"), sort_keys=True)
    return {
        "kind": "preserved_expression",
        "source": source,
        "sha256": sha256_bytes(source.encode("utf-8")),
    }


def literal_boolean(value: dict[str, Any] | None, default: bool = False) -> bool:
    return (
        value.get("value")
        if isinstance(value, dict)
        and value.get("kind") == "literal"
        and isinstance(value.get("value"), bool)
        else default
    )


def literal_text(value: dict[str, Any] | None) -> str | None:
    if (
        isinstance(value, dict)
        and value.get("kind") == "literal"
        and isinstance(value.get("value"), str)
    ):
        return value["value"]
    return None


def source_type_name(value: dict[str, Any]) -> str:
    literal = literal_text(value)
    if literal:
        return literal
    name = value.get("name") if value.get("kind") in {"name", "attribute"} else None
    if isinstance(name, str) and name:
        return name.rsplit(".", 1)[-1].upper()
    if value.get("kind") == "call":
        call_name = value.get("name")
        if isinstance(call_name, str) and call_name:
            terminal = call_name.rsplit(".", 1)[-1]
            if terminal.casefold() == "custom" and value.get("arguments"):
                custom = literal_text(value["arguments"][0])
                if custom:
                    return custom
            return terminal.upper()
    return "PRESERVED_EXPRESSION"


RECOGNIZED_INPUT_OPTIONS = {
    "default",
    "min",
    "max",
    "step",
    "options",
    "display_name",
    "tooltip",
    "multiline",
    "socketless",
    "widget_type",
    "force_input",
    "raw_link",
    "advanced",
    "image_upload",
    "audio_upload",
    "video_upload",
    "model_upload",
    "optional",
}


def portable_input(
    name: str,
    source_type: dict[str, Any],
    options: list[dict[str, Any]],
    requirement: str,
) -> dict[str, Any]:
    by_name = {option["name"]: option["value"] for option in options}
    choices = by_name.get("options")
    if choices is None and source_type.get("kind") in {"list", "tuple", "set"}:
        choices = source_type
        source_type_names = ["COMBO"]
    else:
        source_type_names = [source_type_name(source_type)]
    static_choices = []
    extra = []
    if isinstance(choices, dict) and choices.get("kind") in {"list", "tuple", "set"}:
        static_choices = [portable_expression(item) for item in choices.get("items", [])]
    elif choices is not None:
        extra.append({"name": "choices_expression", "value": portable_expression(choices)})
    for option in options:
        if option["name"] not in RECOGNIZED_INPUT_OPTIONS:
            extra.append(
                {"name": option["name"], "value": portable_expression(option["value"])}
            )
    upload = None
    for key, kind in (
        ("image_upload", "image"),
        ("audio_upload", "audio"),
        ("video_upload", "video"),
        ("model_upload", "model"),
    ):
        if literal_boolean(by_name.get(key)):
            upload = kind
            break
    return {
        "name": name,
        "source_type_names": source_type_names,
        "default": portable_expression(by_name["default"]) if "default" in by_name else None,
        "minimum": portable_expression(by_name["min"]) if "min" in by_name else None,
        "maximum": portable_expression(by_name["max"]) if "max" in by_name else None,
        "step": portable_expression(by_name["step"]) if "step" in by_name else None,
        "choices": static_choices,
        "display_name": literal_text(by_name.get("display_name")),
        "tooltip": literal_text(by_name.get("tooltip")),
        "multiline": literal_boolean(by_name.get("multiline")),
        "socketless": literal_boolean(by_name.get("socketless")),
        "widget_type": literal_text(by_name.get("widget_type")),
        "force_input": literal_boolean(by_name.get("force_input")),
        "raw_link": literal_boolean(by_name.get("raw_link")),
        "advanced": literal_boolean(by_name.get("advanced")),
        "upload": upload,
        "requirement": requirement,
        "extra": extra,
    }


def literal_u32(value: dict[str, Any] | None, default: int) -> int:
    if (
        isinstance(value, dict)
        and value.get("kind") == "literal"
        and isinstance(value.get("value"), int)
        and not isinstance(value.get("value"), bool)
        and 0 <= value["value"] <= 2**32 - 1
    ):
        return value["value"]
    return default


def v3_dynamic_input(
    port: dict[str, Any], bindings: dict[str, dict[str, Any]]
) -> dict[str, Any] | None:
    keywords = {keyword["name"]: keyword["value"] for keyword in port.get("keywords", [])}
    template = keywords.get("template")
    if isinstance(template, dict) and template.get("kind") == "name":
        template = bindings.get(template.get("name", ""), template)
    if not isinstance(template, dict) or template.get("kind") != "call":
        return None
    template_name = template.get("name", "")
    if not isinstance(template_name, str) or not template_name.endswith(
        ("TemplatePrefix", "TemplateNames")
    ):
        return None
    template_keywords = {
        keyword["name"]: keyword["value"] for keyword in template.get("keywords", [])
    }
    inner = template.get("arguments", [None])[0] if template.get("arguments") else None
    if inner is None:
        inner = template_keywords.get("input")
    if not isinstance(inner, dict) or inner.get("kind") != "call":
        return None
    inner_name = (
        literal_text(inner.get("arguments", [None])[0])
        if inner.get("arguments")
        else None
    ) or "value"
    inner_callee = inner.get("name")
    inner_source_type = {
        "kind": "attribute",
        "name": inner_callee.rsplit(".", 1)[0],
    } if isinstance(inner_callee, str) and "." in inner_callee else inner
    inner_input = portable_input(
        inner_name,
        inner_source_type,
        inner.get("keywords", []),
        "optional",
    )
    inner_input.pop("requirement", None)
    prefix = literal_text(template_keywords.get("prefix"))
    names_expression = template_keywords.get("names")
    names = []
    extra = [
        {
            "name": "autogrow_group",
            "value": {"kind": "string", "value": port["name"]},
        }
    ]
    if isinstance(names_expression, dict) and names_expression.get("kind") in {
        "list",
        "tuple",
        "set",
    }:
        names = [
            name
            for item in names_expression.get("items", [])
            if (name := literal_text(item)) is not None
        ]
        if len(names) != len(names_expression.get("items", [])):
            names = []
    elif names_expression is not None:
        extra.append(
            {
                "name": "names_expression",
                "value": portable_expression(names_expression),
            }
        )
    minimum = literal_u32(template_keywords.get("min"), 0)
    maximum = literal_u32(
        template_keywords.get("max"), len(names) if names else 65_536
    )
    if template_keywords.get("min") is not None and literal_u32(
        template_keywords.get("min"), 2**32 - 1
    ) == 2**32 - 1:
        extra.append(
            {
                "name": "minimum_expression",
                "value": portable_expression(template_keywords["min"]),
            }
        )
    if template_keywords.get("max") is not None and literal_u32(
        template_keywords.get("max"), 2**32 - 1
    ) == 2**32 - 1:
        extra.append(
            {
                "name": "maximum_expression",
                "value": portable_expression(template_keywords["max"]),
            }
        )
    if names:
        identity = "{name}"
        start_index = 0
    else:
        prefix = prefix or inner_name
        identity = f"{prefix}{{index}}"
        start_index = 1
    return {
        "identity": identity,
        "prefix": prefix,
        "names": names,
        "start_index": start_index,
        "minimum_count": minimum,
        "maximum_count": maximum,
        "input": inner_input,
        "extra": extra,
    }


def v3_portable_schema(contract: dict[str, Any]) -> dict[str, Any]:
    inputs = []
    dynamic_inputs = []
    unresolved_inputs = []
    bindings = {
        target: binding["value"]
        for binding in contract.get("bindings", [])
        if isinstance(binding.get("value"), dict)
        for target in binding.get("targets", [])
    }
    for port in contract.get("inputs", []):
        if port.get("kind") != "call" or not isinstance(port.get("name"), str):
            unresolved_inputs.append(portable_expression(port))
            continue
        if not any(port.get(field) for field in ("constructor", "source_type", "callee")):
            unresolved_inputs.append(portable_expression(port))
            continue
        source_type = port.get("source_type") or port.get("callee") or port
        if source_type_name(source_type) == "AUTOGROW":
            dynamic_input = v3_dynamic_input(port, bindings)
            if dynamic_input is None:
                unresolved_inputs.append(portable_expression(port))
            else:
                dynamic_inputs.append(dynamic_input)
            continue
        by_name = {keyword["name"]: keyword["value"] for keyword in port.get("keywords", [])}
        requirement = "optional" if literal_boolean(by_name.get("optional")) else "required"
        inputs.append(
            portable_input(
                port["name"], source_type, port.get("keywords", []), requirement
            )
        )
    outputs = []
    unresolved_outputs = []
    for index, port in enumerate(contract.get("outputs", [])):
        if port.get("kind") != "call":
            unresolved_outputs.append(portable_expression(port))
            continue
        source_type = port.get("source_type") or port.get("callee") or port
        keywords = {keyword["name"]: keyword["value"] for keyword in port.get("keywords", [])}
        outputs.append(
            {
                "source_name": port.get("name"),
                "source_type_name": source_type_name(source_type),
                "display_name": literal_text(keywords.get("display_name")),
                "tooltip": literal_text(keywords.get("tooltip")),
                "choices": [],
                "match_template": literal_text(keywords.get("match_template")),
                "extra": [
                    {"name": name, "value": portable_expression(value)}
                    for name, value in keywords.items()
                    if name not in {"display_name", "tooltip", "match_template"}
                ],
                "ordinal": index,
            }
        )
    node_options = {field["name"]: field["value"] for field in contract.get("node_options", [])}
    recognized_node_options = {
        "node_id",
        "display_name",
        "category",
        "description",
        "is_deprecated",
        "is_experimental",
        "is_api_node",
        "is_dev_only",
        "is_output_node",
        "has_intermediate_output",
        "not_idempotent",
        "enable_expand",
        "accept_all_inputs",
        "essentials_category",
        "search_aliases",
        "price",
    }
    return {
        "provenance": "source_v3",
        "inputs": inputs,
        "dynamic_inputs": dynamic_inputs,
        "outputs": outputs,
        "unresolved_inputs": unresolved_inputs,
        "unresolved_outputs": unresolved_outputs,
        "hidden": [portable_expression(value) for value in contract.get("hidden", [])],
        "node": {
            "has_intermediate_output": literal_boolean(node_options.get("has_intermediate_output")),
            "development_only": literal_boolean(node_options.get("is_dev_only")),
            "api_node": literal_boolean(node_options.get("is_api_node")),
            "not_idempotent": literal_boolean(node_options.get("not_idempotent")),
            "enable_expand": literal_boolean(node_options.get("enable_expand")),
            "accept_all_inputs": literal_boolean(node_options.get("accept_all_inputs")),
            "essentials_category": literal_text(node_options.get("essentials_category")),
            "price_badge": portable_expression(node_options["price"]) if "price" in node_options else None,
            "is_deprecated": literal_boolean(node_options.get("is_deprecated")),
            "is_experimental": literal_boolean(node_options.get("is_experimental")),
            "display_name": literal_text(node_options.get("display_name")),
            "description": literal_text(node_options.get("description")),
            "extra": [
                {"name": name, "value": portable_expression(value)}
                for name, value in node_options.items()
                if name not in recognized_node_options
            ],
        },
    }


def v1_options(field: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    contract = field["contract"]
    if contract.get("kind") != "tuple" or not contract.get("items"):
        return contract, []
    source_type = contract["items"][0]
    options = []
    if len(contract["items"]) > 1 and contract["items"][1].get("kind") == "dict":
        for entry in contract["items"][1].get("entries", []):
            name = literal_text(entry.get("key"))
            if name is not None:
                options.append({"name": name, "value": entry["value"]})
    return source_type, options


def v1_portable_schema(contract: dict[str, Any]) -> dict[str, Any]:
    inputs = []
    for group in contract.get("input_groups", []):
        requirement = group.get("name") or "preserved"
        for field in group.get("fields", []):
            if not isinstance(field.get("name"), str):
                continue
            source_type, options = v1_options(field)
            inputs.append(portable_input(field["name"], source_type, options, requirement))
    metadata = {field["name"]: field["value"] for field in contract.get("class_metadata", [])}
    return_types = metadata.get("RETURN_TYPES", {"kind": "tuple", "items": []})
    return_names = metadata.get("RETURN_NAMES", {"kind": "tuple", "items": []})
    names = [literal_text(value) for value in return_names.get("items", [])]
    outputs = [
        {
            "source_name": names[index] if index < len(names) else None,
            "source_type_name": source_type_name(value),
            "display_name": None,
            "tooltip": None,
            "choices": [],
            "match_template": None,
            "extra": [],
            "ordinal": index,
        }
        for index, value in enumerate(return_types.get("items", []))
    ]
    return {
        "provenance": "source_v1",
        "inputs": inputs,
        "dynamic_inputs": [],
        "outputs": outputs,
        "unresolved_inputs": [],
        "unresolved_outputs": [],
        "hidden": [],
        "node": {
            "has_intermediate_output": False,
            "development_only": False,
            "api_node": False,
            "not_idempotent": False,
            "enable_expand": False,
            "accept_all_inputs": False,
            "essentials_category": None,
            "price_badge": None,
            "is_deprecated": literal_boolean(metadata.get("DEPRECATED")),
            "is_experimental": literal_boolean(metadata.get("EXPERIMENTAL")),
            "display_name": None,
            "description": literal_text(metadata.get("DESCRIPTION")),
            "extra": [
                {"name": name, "value": portable_expression(value)}
                for name, value in metadata.items()
                if name not in {"RETURN_TYPES", "RETURN_NAMES", "DEPRECATED", "EXPERIMENTAL", "DESCRIPTION"}
            ],
        },
    }


def portable_schema(
    schema_api: str, schema: dict[str, Any], feature_id: str
) -> dict[str, Any]:
    contract = schema.get("contract")
    if not isinstance(contract, dict):
        raise RuntimeError("normalized node schema lacks its source contract")
    result = (
        v3_portable_schema(contract)
        if schema_api == "V3"
        else v1_portable_schema(contract)
    )
    result["schema_version"] = 2
    result["catalog_sha256"] = schema["catalog_sha256"]
    result["definition_sha256"] = schema["definition_sha256"]
    source_node = result["node"]
    result["presentation"] = {
        "is_deprecated": source_node.pop("is_deprecated"),
        "is_experimental": source_node.pop("is_experimental"),
        "display_name": source_node.pop("display_name"),
        "description": source_node.pop("description"),
    }
    source_node.update(
        {
            "schema_version": 2,
            "provenance": result["provenance"],
            "feature_id": feature_id,
            "definition_sha256": schema["definition_sha256"],
        }
    )
    return result


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
        schema = schema_projection(
            row["schema_source"], row["schema_api"], source, definition
        )
        schema["portable"] = portable_schema(
            row["schema_api"], schema, row["feature_id"]
        )
        schema["portable"]["presentation"]["is_deprecated"] |= (
            row["availability"] == "deprecated/dead"
        )
        schema["portable"]["presentation"]["is_experimental"] |= (
            row["availability"] == "experimental"
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
                "schema": schema,
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
