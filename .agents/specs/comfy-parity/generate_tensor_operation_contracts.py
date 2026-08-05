#!/usr/bin/env python3

from __future__ import annotations

import ast
import csv
import hashlib
import json
import re
from collections import defaultdict
from functools import lru_cache
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[2]
CATALOGS = ROOT / "catalogs"
SOURCE = WORKSPACE / "projects/comfy/ComfyUI"
INVENTORY = CATALOGS / "backend-tensor-operations.csv"
LEDGER = CATALOGS / "native-tensor-operation-contracts.csv"
RUST_TABLE = WORKSPACE / "crates/comfy_tensor/src/operation_contract_records.rs"
FIXTURE_DIRECTORY = WORKSPACE / "crates/comfy_test_support/fixtures/tensor_signatures"
FIXTURE = FIXTURE_DIRECTORY / "resolution-environment.json"
CONTRACT_FIXTURE_DIRECTORY = FIXTURE_DIRECTORY / "contracts"
SOURCE_FINGERPRINT = "21de8fece20d8d5bfa94daaa52d6ccfe2db6726ca0803ca3b383ad164cbd1d5f"


FIELDS = [
    "operation_id",
    "overload_id",
    "inventory_kind",
    "canonical_target",
    "resolution_state",
    "blocker_reason",
    "call_style",
    "ordered_parameters_json",
    "output_arity",
    "output_types_json",
    "exact_rust_signature",
    "reference_semantic",
    "resolution_owner_task_id",
    "expected_resolution_module",
    "release_closure_required",
    "mutation_rule",
    "alias_rule",
    "shape_rule",
    "dtype_rule",
    "accumulation_dtype",
    "layout_rule",
    "device_rule",
    "numeric_rule",
    "tolerance",
    "determinism",
    "cancellation_points",
    "vjp_rule",
    "jvp_rule",
    "source_call_sites",
    "oracle_fixture",
    "oracle_fixture_sha256",
    "evidence",
]


DTYPE_REFERENCES = {
    "torch.bfloat16",
    "torch.bool",
    "torch.complex64",
    "torch.float",
    "torch.float16",
    "torch.float32",
    "torch.float64",
    "torch.float8_e4m3fn",
    "torch.float8_e4m3fnuz",
    "torch.float8_e5m2",
    "torch.float8_e5m2fnuz",
    "torch.float8_e8m0fnu",
    "torch.int",
    "torch.int16",
    "torch.int32",
    "torch.int64",
    "torch.int8",
    "torch.long",
    "torch.uint16",
    "torch.uint32",
    "torch.uint64",
    "torch.uint8",
}
LAYOUT_REFERENCES = {"torch.channels_last", "torch.preserve_format"}
BOOLEAN_CAPABILITY_REFERENCES = {
    "torch.backends.cuda.matmul.allow_fp16_accumulation",
    "torch.backends.cuda.matmul.allow_tf32",
    "torch.backends.cudnn.allow_tf32",
    "torch.backends.cudnn.benchmark",
    "torch.backends.cudnn.enabled",
    "torch.xpu.get_device_properties().has_fp16",
    "xformers._has_cpp_library",
}
NUMERIC_CONSTANT_REFERENCES = {
    "torch.finfo().bits",
    "torch.finfo().eps",
    "torch.finfo().max",
    "torch.finfo().min",
    "torch.inf",
    "torch.pi",
}
FUNCTION_REFERENCES = {
    "torch.autograd.function.once_differentiable",
    "torch.log10",
    "torch.nn.Hardswish",
    "torch.nn.Hardtanh",
    "torch.nn.Mish",
    "torch.nn.SELU",
    "torch.nn.Softsign",
    "torch.xpu.stream",
}
NAMESPACE_REFERENCES = {"comfy.ops", "torch.__path__", "torch.nn"}
TENSOR_PROPERTY_REFERENCES = {
    "torch.fft.ifftn().real",
    "torch.median().values",
    "torch.unique().shape",
    "torch.vander().T",
}
DEVICE_PROPERTY_REFERENCES = {
    "torch.cuda.get_device_properties().gcnArchName",
    "torch.empty().device",
    "torch.xpu.get_device_properties().total_memory",
}
ENUM_VARIANT_REFERENCES = {
    "torch.nn.attention.SDPBackend.CUDNN_ATTENTION",
    "torch.nn.attention.SDPBackend.EFFICIENT_ATTENTION",
    "torch.nn.attention.SDPBackend.FLASH_ATTENTION",
    "torch.nn.attention.SDPBackend.MATH",
    "torchvision.transforms.InterpolationMode.NEAREST",
    "torchvision.transforms.functional.InterpolationMode.BICUBIC",
}
VERSION_VALUE_REFERENCES = {
    "torch.version.__version__",
    "torch.version.cuda",
    "torch.version.hip",
    "xformers.__version__",
    "xformers.version.__version__",
}
TYPE_MARKER_VALUE_REFERENCES = {"torch.AcceleratorError", "torch.cuda.OutOfMemoryError"}

SITE_PATTERN = re.compile(
    r"^(?P<path>.*?\.py):(?P<line>[0-9]+):(?P<column>[0-9]+)(?: \((?P<scope>.*)\))?$"
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compact_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def slug(value: str) -> str:
    normalized = value.casefold().replace("3d", "three-d")
    normalized = re.sub(r"[^a-z0-9]+", "-", normalized).strip("-")
    return normalized or "root"


def rust_string(value: str) -> str:
    escaped = []
    for character in value:
        if character == "\\":
            escaped.append("\\\\")
        elif character == '"':
            escaped.append('\\"')
        elif character == "\n":
            escaped.append("\\n")
        elif character == "\r":
            escaped.append("\\r")
        elif character == "\t":
            escaped.append("\\t")
        elif ord(character) < 0x20 or ord(character) == 0x7F:
            escaped.append(f"\\u{{{ord(character):x}}}")
        else:
            escaped.append(character)
    return '"' + "".join(escaped) + '"'


@lru_cache(maxsize=None)
def parse_source(path: Path) -> ast.Module:
    return ast.parse(path.read_text(encoding="utf-8"), filename=path.as_posix())


def direct_named_node(body: list[ast.stmt], name: str) -> ast.AST | None:
    for node in body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)) and node.name == name:
            return node
        if isinstance(node, (ast.Assign, ast.AnnAssign)):
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            if any(isinstance(target, ast.Name) and target.id == name for target in targets):
                return node
    return None


@lru_cache(maxsize=None)
def source_parent_map(path: Path) -> dict[ast.AST, ast.AST]:
    module = parse_source(path)
    return {child: parent for parent in ast.walk(module) for child in ast.iter_child_nodes(parent)}


@lru_cache(maxsize=None)
def source_call_index(path: Path) -> dict[tuple[int, int], list[ast.Call]]:
    calls: dict[tuple[int, int], list[ast.Call]] = defaultdict(list)
    for node in ast.walk(parse_source(path)):
        if isinstance(node, ast.Call):
            calls[(node.lineno, node.col_offset)].append(node)
    return calls


def source_location(site: str) -> tuple[Path, int, int, str] | None:
    match = SITE_PATTERN.fullmatch(site)
    if match is None:
        return None
    path = SOURCE / match.group("path")
    if not path.is_file():
        return None
    return (
        path,
        int(match.group("line")),
        int(match.group("column")) - 1,
        match.group("scope") or "module",
    )


def split_sites(value: str) -> list[str]:
    return [site.strip() for site in value.split(" | ") if site.strip()]


def static_expression(node: ast.AST) -> dict[str, object]:
    return {
        "syntax": ast.unparse(node),
        "syntax_kind": type(node).__name__,
    }


def consumer_output_expectation(call: ast.Call, parents: dict[ast.AST, ast.AST]) -> dict[str, object]:
    parent = parents.get(call)
    if isinstance(parent, ast.Assign) and parent.value is call and len(parent.targets) == 1:
        target = parent.targets[0]
        if isinstance(target, (ast.Tuple, ast.List)):
            return {"kind": "destructure", "arity": len(target.elts)}
        return {"kind": "single-binding", "arity": 1}
    if isinstance(parent, ast.AnnAssign) and parent.value is call:
        return {"kind": "annotated-single-binding", "arity": 1}
    if isinstance(parent, ast.NamedExpr) and parent.value is call:
        return {"kind": "named-expression", "arity": 1}
    if isinstance(parent, ast.Attribute) and parent.value is call:
        return {"kind": "attribute-access", "attribute": parent.attr}
    if isinstance(parent, ast.Subscript) and parent.value is call:
        return {"kind": "subscript"}
    if isinstance(parent, ast.Return) and parent.value is call:
        return {"kind": "returned"}
    if isinstance(parent, ast.Expr):
        return {"kind": "discarded"}
    return {"kind": "nested-expression"}


def call_target_name(call: ast.Call) -> str:
    if isinstance(call.func, ast.Attribute):
        return call.func.attr
    if isinstance(call.func, ast.Name):
        return call.func.id
    return ""


def call_arguments(call: ast.Call) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    positional = []
    for index, argument in enumerate(call.args):
        value = argument.value if isinstance(argument, ast.Starred) else argument
        positional.append(
            {
                "index": index,
                "starred": isinstance(argument, ast.Starred),
                **static_expression(value),
            }
        )
    keywords = [
        {
            "name": keyword.arg,
            "expanded_mapping": keyword.arg is None,
            **static_expression(keyword.value),
        }
        for keyword in call.keywords
    ]
    return positional, keywords


def call_site_observation(site: str, canonical_target: str) -> dict[str, object]:
    location = source_location(site)
    if location is None:
        return {"site": site, "status": "unparsed-location"}
    path, line, column, scope = location
    try:
        module = parse_source(path)
    except SyntaxError as error:
        source_line = path.read_text(encoding="utf-8").splitlines()[line - 1]
        return {
            "site": site,
            "status": "source-requires-newer-python-grammar",
            "source_path": path.relative_to(SOURCE).as_posix(),
            "source_sha256": sha256(path),
            "line": line,
            "column": column + 1,
            "scope": scope,
            "source_line": source_line,
            "parser_error_line": error.lineno,
        }
    calls = source_call_index(path).get((line, column), [])
    canonical_name = canonical_target.rsplit(".", 1)[-1]
    named_calls = [call for call in calls if call_target_name(call) == canonical_name]
    if len(named_calls) == 1:
        calls = named_calls
    if len(calls) != 1:
        return {
            "site": site,
            "status": "call-node-not-unique" if calls else "call-node-not-found",
            "matching_call_nodes": len(calls),
            "candidate_calls": [
                {
                    "target_syntax": ast.unparse(call.func),
                    "target_name": call_target_name(call),
                    "positional_arguments": call_arguments(call)[0],
                    "keyword_arguments": call_arguments(call)[1],
                }
                for call in calls
            ],
        }
    call = calls[0]
    positional, keywords = call_arguments(call)
    return {
        "site": site,
        "status": "static-call-observed",
        "source_path": path.relative_to(SOURCE).as_posix(),
        "source_sha256": sha256(path),
        "line": line,
        "column": column + 1,
        "scope": scope,
        "target_syntax": ast.unparse(call.func),
        "positional_arguments": positional,
        "keyword_arguments": keywords,
        "consumer_output_expectation": consumer_output_expectation(call, source_parent_map(path)),
    }


def observed_call_sites(source: dict[str, str]) -> list[dict[str, object]]:
    sites = []
    for field in ("production_call_sites", "test_call_sites", "support_call_sites"):
        for site in split_sites(source.get(field, "")):
            observation = call_site_observation(site, source["symbol"])
            observation["catalog_field"] = field
            sites.append(observation)
    return sites


def source_definition(symbol: str) -> tuple[Path, ast.AST] | None:
    parts = symbol.split(".")
    if not parts or parts[0] not in {"comfy", "comfy_extras", "nodes"}:
        return None
    for split in range(len(parts), 0, -1):
        module_path = SOURCE.joinpath(*parts[:split])
        candidates = [module_path.with_suffix(".py"), module_path / "__init__.py"]
        source_path = next((path for path in candidates if path.is_file()), None)
        if source_path is None:
            continue
        remaining = parts[split:]
        if not remaining:
            return source_path, parse_source(source_path)
        current: ast.AST = parse_source(source_path)
        for name in remaining:
            body = getattr(current, "body", None)
            if not isinstance(body, list):
                current = None  # type: ignore[assignment]
                break
            next_node = direct_named_node(body, name)
            if next_node is None:
                current = None  # type: ignore[assignment]
                break
            current = next_node
        if current is not None:
            return source_path, current
    return None


def expression(node: ast.AST | None) -> str | None:
    return ast.unparse(node) if node is not None else None


def function_parameters(
    function: ast.FunctionDef | ast.AsyncFunctionDef,
    skip_receiver: bool,
) -> list[dict[str, object]]:
    positional = list(function.args.posonlyargs) + list(function.args.args)
    defaults: list[ast.AST | None] = [None] * (len(positional) - len(function.args.defaults)) + list(
        function.args.defaults
    )
    parameters = []
    for index, (argument, default) in enumerate(zip(positional, defaults)):
        if skip_receiver and index == 0 and argument.arg in {"self", "cls"}:
            continue
        parameters.append(
            {
                "name": argument.arg,
                "kind": "positional_only" if index < len(function.args.posonlyargs) else "positional_or_keyword",
                "type": expression(argument.annotation) or "dynamic",
                "default": expression(default),
                "keyword_only": False,
            }
        )
    if function.args.vararg is not None:
        parameters.append(
            {
                "name": function.args.vararg.arg,
                "kind": "variadic_positional",
                "type": expression(function.args.vararg.annotation) or "dynamic",
                "default": None,
                "keyword_only": False,
            }
        )
    for argument, default in zip(function.args.kwonlyargs, function.args.kw_defaults):
        parameters.append(
            {
                "name": argument.arg,
                "kind": "keyword_only",
                "type": expression(argument.annotation) or "dynamic",
                "default": expression(default),
                "keyword_only": True,
            }
        )
    if function.args.kwarg is not None:
        parameters.append(
            {
                "name": function.args.kwarg.arg,
                "kind": "variadic_keyword",
                "type": expression(function.args.kwarg.annotation) or "dynamic",
                "default": None,
                "keyword_only": True,
            }
        )
    return parameters


def return_expression_contract(node: ast.AST | None) -> dict[str, object]:
    if node is None or (isinstance(node, ast.Constant) and node.value is None):
        return {"arity": 0, "types": []}
    if isinstance(node, (ast.Tuple, ast.List)):
        return {
            "arity": len(node.elts),
            "types": [{"syntax_kind": type(element).__name__} for element in node.elts],
        }
    return {"arity": 1, "types": [{"syntax_kind": type(node).__name__}]}


def definition_return_contract(node: ast.AST) -> dict[str, object]:
    if isinstance(node, ast.ClassDef):
        return {"arities": [1], "types": [{"kind": "class-instance", "name": node.name}]}
    if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
        return {"arities": [], "types": []}
    returns = [
        return_expression_contract(candidate.value)
        for candidate in ast.walk(node)
        if isinstance(candidate, ast.Return)
        and next(
            (
                ancestor
                for ancestor in source_parent_map_for_node(node).get(candidate, [])
                if isinstance(ancestor, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda))
            ),
            node,
        )
        is node
    ]
    return {
        "arities": sorted({int(contract["arity"]) for contract in returns}),
        "types": [contract["types"] for contract in returns],
    }


def source_parent_map_for_node(root: ast.AST) -> dict[ast.AST, list[ast.AST]]:
    parents: dict[ast.AST, list[ast.AST]] = defaultdict(list)
    for parent in ast.walk(root):
        for child in ast.iter_child_nodes(parent):
            parents[child] = [parent, *parents.get(parent, [])]
    return parents


def definition_evidence(symbol: str) -> dict[str, object] | None:
    resolved = source_definition(symbol)
    if resolved is None:
        return None
    path, node = resolved
    function: ast.FunctionDef | ast.AsyncFunctionDef | None = None
    skip_receiver = False
    parameter_source = "not-declared"
    kind = type(node).__name__
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
        function = node
        parameter_source = "declared-function"
    elif isinstance(node, ast.ClassDef):
        initializer = direct_named_node(node.body, "__init__")
        if isinstance(initializer, (ast.FunctionDef, ast.AsyncFunctionDef)):
            function = initializer
            skip_receiver = True
            parameter_source = "declared-initializer"
        else:
            parameter_source = "inherited-or-dynamic-constructor"
    elif isinstance(node, (ast.Assign, ast.AnnAssign)):
        parameter_source = "callable-alias-or-value"
    parameters = function_parameters(function, skip_receiver) if function is not None else []
    return_annotation = expression(function.returns) if function is not None else None
    return {
        "definition_kind": kind,
        "definition_path": path.relative_to(SOURCE).as_posix(),
        "definition_line": getattr(node, "lineno", None),
        "parameters": parameters,
        "parameter_source": parameter_source,
        "return_annotation": return_annotation,
        "return_contract": definition_return_contract(node),
    }


def reconcile_call_with_definition(
    observation: dict[str, object], definition: dict[str, object] | None
) -> dict[str, object]:
    if definition is None:
        return {"status": "definition-unavailable"}
    if observation.get("status") != "static-call-observed":
        return {"status": "call-unavailable"}
    if definition.get("parameter_source") in {
        "inherited-or-dynamic-constructor",
        "callable-alias-or-value",
        "not-declared",
    }:
        return {"status": "definition-call-signature-not-declared"}
    parameters = [dict(value) for value in definition.get("parameters", [])]
    positional_parameters = [
        parameter
        for parameter in parameters
        if parameter["kind"] in {"positional_only", "positional_or_keyword"}
    ]
    variadic_parameter = next(
        (parameter for parameter in parameters if parameter["kind"] == "variadic_positional"),
        None,
    )
    parameter_names = {str(parameter["name"]) for parameter in parameters}
    bindings = []
    for argument in observation.get("positional_arguments", []):
        argument = dict(argument)
        if bool(argument["starred"]):
            return {"status": "dynamic-starred-arguments", "bindings": bindings}
        index = int(argument["index"])
        if index < len(positional_parameters):
            parameter_name = positional_parameters[index]["name"]
        elif variadic_parameter is not None:
            parameter_name = variadic_parameter["name"]
        else:
            return {"status": "static-incompatible-too-many-positionals", "bindings": bindings}
        bindings.append({"argument": f"positional:{index}", "parameter": parameter_name})
    for keyword in observation.get("keyword_arguments", []):
        keyword = dict(keyword)
        name = keyword.get("name")
        if name is None:
            return {"status": "dynamic-expanded-keywords", "bindings": bindings}
        if name not in parameter_names and not any(
            parameter["kind"] == "variadic_keyword" for parameter in parameters
        ):
            return {
                "status": "static-incompatible-unknown-keyword",
                "keyword": name,
                "bindings": bindings,
            }
        bindings.append({"argument": f"keyword:{name}", "parameter": name})
    return {"status": "static-compatible", "bindings": bindings}


def observed_parameters(
    observations: list[dict[str, object]], definition: dict[str, object] | None
) -> list[dict[str, object]]:
    if definition is not None and definition.get("parameters"):
        return [dict(value) for value in definition["parameters"]]
    positional_count = max(
        (
            len(observation.get("positional_arguments", []))
            for observation in observations
            if observation.get("status") == "static-call-observed"
        ),
        default=0,
    )
    parameters = [
        {
            "name": f"observed_positional_{index}",
            "kind": "positional_or_keyword",
            "type": "unresolved_external_semantics",
            "default": None,
            "keyword_only": False,
            "evidence": "static call-site position only",
        }
        for index in range(positional_count)
    ]
    keyword_names = sorted(
        {
            str(keyword["name"])
            for observation in observations
            if observation.get("status") == "static-call-observed"
            for keyword in observation.get("keyword_arguments", [])
            if keyword.get("name") is not None
        }
    )
    parameters.extend(
        {
            "name": name,
            "kind": "observed_keyword",
            "type": "unresolved_external_semantics",
            "default": None,
            "keyword_only": True,
            "evidence": "static call-site spelling only",
        }
        for name in keyword_names
    )
    return parameters


def fixture_payload(inventory_digest: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "fixture_id": "tensor-signature-resolution-environment-v1",
        "feature_ids": [],
        "observation_kind": "tensor_signature_and_value_oracle",
        "source": {
            "product": "ComfyUI",
            "declared_version": "0.27.1",
            "tree_sha256": SOURCE_FINGERPRINT,
        },
        "inputs": [
            {
                "name": "catalogs/backend-tensor-operations.csv",
                "sha256": inventory_digest,
            },
            {
                "name": "ComfyUI/requirements.txt",
                "sha256": sha256(SOURCE / "requirements.txt"),
            },
            {
                "name": "ComfyUI/README.md",
                "sha256": sha256(SOURCE / "README.md"),
            },
        ],
        "command": {
            "adapter": "static-tensor-contract-resolution-v1",
            "program": "generate_tensor_operation_contracts.py",
            "arguments": ["catalogs/backend-tensor-operations.csv"],
            "configuration": {
                "live_source_runtime": "not_launched",
                "network": "disabled",
                "resolution_policy": "static evidence or explicit blocker",
            },
        },
        "environment": {
            "operating_system": "macos",
            "architecture": "aarch64",
            "device": "cpu",
            "device_details": {
                "external_accelerators": "not required for static classification",
            },
            "dependencies": {
                "python_source_runtime": "not launched",
                "torch_profile": "not pinned by source baseline",
            },
            "network_access": False,
            "account_access": False,
        },
        "normalization": {
            "remove_json_pointers": [],
            "replacements": {},
            "unordered_array_pointers": [],
        },
        "tolerance": {
            "default": {"kind": "exact"},
            "json_pointer_overrides": {},
        },
        "unresolved_nondeterminism": [],
        "observation": {
            "status": "not_observed",
            "blocker": "dependency",
            "detail": "The source snapshot permits PyTorch 2.4 and newer but does not pin an exact PyTorch, torchvision, torchaudio, torchsde, Kornia, xFormers, FlashAttention, or accelerator-extension semantics profile; no approved Python 3.10+ source environment with those dependencies is present.",
            "evidence": [
                "ComfyUI requirements.txt names torch and companion packages without versions",
                "ComfyUI README states that torch 2.4 and newer are supported and recommends newer versions",
                "the baseline forbids inferring unobserved runtime output or installing dependencies during catalog discovery",
            ],
            "uncertainty": "Exact external overloads, defaults, error text, values, gradients, and version-dependent behavior remain release blockers until a versioned development-only oracle profile is approved and recorded.",
        },
    }


def reference_semantic(source: dict[str, str]) -> dict[str, str]:
    symbol = source["symbol"]
    if source["inventory_kind"] == "type-reference" or symbol in TYPE_MARKER_VALUE_REFERENCES:
        category = "type-marker"
    elif symbol in DTYPE_REFERENCES:
        category = "dtype"
    elif symbol in LAYOUT_REFERENCES:
        category = "layout-or-memory-format"
    elif symbol in BOOLEAN_CAPABILITY_REFERENCES:
        category = "boolean-capability"
    elif symbol in NUMERIC_CONSTANT_REFERENCES:
        category = "numeric-constant"
    elif symbol in FUNCTION_REFERENCES:
        category = "function-reference"
    elif symbol in NAMESPACE_REFERENCES:
        category = "namespace"
    elif symbol in TENSOR_PROPERTY_REFERENCES:
        category = "tensor-property"
    elif symbol in DEVICE_PROPERTY_REFERENCES:
        category = "device-property"
    elif symbol in ENUM_VARIANT_REFERENCES:
        category = "enum-variant"
    elif symbol in VERSION_VALUE_REFERENCES:
        category = "version-value"
    else:
        raise RuntimeError(f"reference semantic is not explicitly classified: {symbol}")
    return {"category": category, "value": symbol}


def callable_resolution(
    source: dict[str, str], definition: dict[str, object] | None
) -> tuple[str, str]:
    receiver_unverified = (
        source.get("confidence") == "low"
        or "receiver-unverified" in source.get("resolution", "")
    )
    if receiver_unverified:
        return (
            "blocked_receiver_unverified",
            "Static syntax does not prove that the receiver is a tensor or identify an overload; "
            "a versioned source-oracle/type-flow fixture is required before a Rust callable may exist.",
        )
    if definition is not None:
        return (
            "blocked_missing_oracle_dependency",
            "The checked-in Python definition and reconciled call syntax prove parameter spelling only; "
            "dynamic tensor types, delegated external operations, output semantics, values, errors, and "
            "gradients require the unavailable versioned development oracle.",
        )
    return (
        "blocked_missing_semantics_profile",
        "The baseline identifies the external target and exact static call syntax but does not pin the "
        "dependency version whose overload, defaults, values, errors, and gradients define this contract.",
    )


def operation_owner_assignments(
    inventory_rows: list[dict[str, str]],
) -> dict[str, tuple[str, str]]:
    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in sorted(inventory_rows, key=lambda item: (item["semantic_group"], item["operation_id"])):
        grouped[row["semantic_group"]].append(row)
    assignments: dict[str, tuple[str, str]] = {}
    for group, rows in sorted(grouped.items()):
        for part, offset in enumerate(range(0, len(rows), 12), start=1):
            assigned = rows[offset : offset + 12]
            identifier = (
                f"comfy-parity-tensor-ops-{slug(group)}-"
                f"{assigned[0]['operation_id'].casefold()}"
            )
            module_name = f"{slug(group).replace('-', '_')}_{part:02d}"
            for row in assigned:
                assignments[row["operation_id"]] = (identifier, module_name)
    if len(assignments) != len(inventory_rows):
        raise RuntimeError("tensor operation resolution-owner assignment is incomplete")
    return assignments


def row_source_files(
    source: dict[str, str], definition: dict[str, object] | None
) -> list[dict[str, str]]:
    paths = set()
    for field in (
        "production_call_sites",
        "test_call_sites",
        "support_call_sites",
        "non_call_reference_sites",
    ):
        for site in split_sites(source.get(field, "")):
            location = source_location(site)
            if location is not None:
                paths.add(location[0])
    if definition is not None:
        definition_path = SOURCE / str(definition["definition_path"])
        if definition_path.is_file():
            paths.add(definition_path)
    return [
        {"path": path.relative_to(SOURCE).as_posix(), "sha256": sha256(path)}
        for path in sorted(paths)
    ]


def contract_fixture_payload(
    source: dict[str, str],
    environment_digest: str,
    owner_task: str,
    expected_resolution_module: str,
) -> tuple[dict[str, object], dict[str, object] | None, list[dict[str, object]]]:
    definition = definition_evidence(source["symbol"])
    executable_or_reclassified = source["inventory_kind"] in {
        "callable-operation",
        "reclassified-external-operation",
    }
    observations = observed_call_sites(source) if executable_or_reclassified else []
    reconciliations = [
        {
            "site": str(observation.get("site", "")),
            **reconcile_call_with_definition(observation, definition),
        }
        for observation in observations
    ]
    callable_operation = source["inventory_kind"] == "callable-operation"
    reclassified_external = source["inventory_kind"] == "reclassified-external-operation"
    resolution_state, blocker_reason = (
        callable_resolution(source, definition)
        if callable_operation
        else (
            "reclassified_external",
            "Imported non-tensor receiver evidence proves this is not a PyTorch Tensor contract; its native behavior remains assigned to the owning source feature rather than receiving a fake tensor kernel.",
        )
        if reclassified_external
        else ("resolved_reference", "")
    )
    semantic = (
        {"category": "not-applicable", "value": source["symbol"]}
        if callable_operation or reclassified_external
        else reference_semantic(source)
    )
    inventory_projection = {key: source[key] for key in sorted(source)}
    return (
        {
            "schema_version": 1,
            "fixture_id": f"tensor-operation-contract-{source['operation_id'].casefold()}-v1",
            "operation_id": source["operation_id"],
            "canonical_target": source["symbol"],
            "inventory_kind": source["inventory_kind"],
            "resolution": {
                "state": resolution_state,
                "blocker_reason": blocker_reason,
                "release_closure_required": callable_operation,
                "resolution_owner_task_id": owner_task,
                "expected_resolution_module": expected_resolution_module,
                "policy": "static evidence or explicit blocker; external semantics are never inferred",
            },
            "reference_semantic": semantic,
            "provenance": {
                "source_product": "ComfyUI",
                "source_declared_version": "0.27.1",
                "source_tree_sha256": SOURCE_FINGERPRINT,
                "environment_fixture": "resolution-environment.json",
                "environment_fixture_sha256": environment_digest,
                "inventory_row_sha256": sha256_bytes(compact_json(inventory_projection).encode("utf-8")),
                "source_files": row_source_files(source, definition),
            },
            "static_definition": definition,
            "static_call_sites": observations,
            "definition_call_reconciliation": reconciliations,
            "observed_parameters": observed_parameters(observations, definition),
            "consumer_output_expectations": [
                {
                    "site": observation["site"],
                    "expectation": observation["consumer_output_expectation"],
                }
                for observation in observations
                if observation.get("status") == "static-call-observed"
            ],
            "normative_rules": {
                "shape": source["shape_requirement"],
                "dtype": source["dtype_requirement"],
                "layout_and_alias": source["layout_requirement"],
                "device": source["device_requirement"],
                "numeric": source["numerics_requirement"],
                "vjp_jvp": source["vjp_jvp_requirement"],
                "cancellation": source["cancellation_requirement"],
            },
        },
        definition,
        observations,
    )


def write_contract_fixture(payload: dict[str, object]) -> tuple[str, str]:
    fixture_id = str(payload["fixture_id"])
    path = CONTRACT_FIXTURE_DIRECTORY / f"{str(payload['operation_id']).casefold()}.json"
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return fixture_id, sha256(path)


def combined_sites(row: dict[str, str]) -> str:
    values = [
        row.get("production_call_sites", ""),
        row.get("test_call_sites", ""),
        row.get("support_call_sites", ""),
        row.get("non_call_reference_sites", ""),
    ]
    return " | ".join(value for value in values if value)


def contract_row(
    source: dict[str, str],
    environment_digest: str,
    owner_task: str,
    expected_resolution_module: str,
) -> dict[str, str]:
    operation_id = source["operation_id"]
    symbol = source["symbol"]
    callable_operation = source["inventory_kind"] == "callable-operation"
    reclassified_external = source["inventory_kind"] == "reclassified-external-operation"
    fixture_payload_value, definition, observations = contract_fixture_payload(
        source, environment_digest, owner_task, expected_resolution_module
    )
    fixture_id, fixture_digest = write_contract_fixture(fixture_payload_value)
    if callable_operation:
        resolution_state, blocker_reason = callable_resolution(source, definition)
        blocker_reason = f"{blocker_reason} Resolution owner: {owner_task}."
        inventory_kind = "callable_operation"
        overload_id = f"{operation_id}:blocked"
        parameters = observed_parameters(observations, definition)
        return_contract = definition.get("return_contract", {}) if definition is not None else {}
        arities = list(return_contract.get("arities", []))
        output_types = list(return_contract.get("types", []))
        output_arity = "|".join(str(value) for value in arities) if arities else "unresolved"
        rust_signature = ""
        semantic = {"category": "not-applicable", "value": symbol}
        release_closure_required = "true"
    elif reclassified_external:
        resolution_state = "reclassified_external"
        blocker_reason = (
            "Imported non-tensor receiver evidence proves this source call is not a PyTorch "
            "Tensor contract; no tensor resolution may claim it."
        )
        inventory_kind = "reclassified_external_operation"
        overload_id = f"{operation_id}:external"
        parameters = observed_parameters(observations, definition)
        output_types = []
        output_arity = "not-applicable"
        semantic = {"category": "not-applicable", "value": symbol}
        rust_signature = "ExternalOperationDisposition"
        release_closure_required = "false"
    else:
        resolution_state = "resolved_reference"
        blocker_reason = ""
        inventory_kind = (
            "type_reference"
            if source["inventory_kind"] == "type-reference"
            else "namespace_value_reference"
        )
        overload_id = f"{operation_id}:reference"
        parameters = []
        output_types = []
        output_arity = "0"
        semantic = reference_semantic(source)
        rust_signature = "TypedReferenceContract"
        release_closure_required = "false"
    evidence = {
        "inventory_resolution": source.get("resolution", ""),
        "inventory_confidence": source.get("confidence", ""),
        "inventory_evidence_level": source.get("evidence_level", ""),
        "source_definition": definition,
        "static_call_observation_count": len(observations),
        "static_call_observation_statuses": sorted(
            {str(observation.get("status", "")) for observation in observations}
        ),
        "resolution_owner_task_id": owner_task,
        "expected_resolution_module": expected_resolution_module,
        "release_closure_required": callable_operation,
        "per_row_fixture": fixture_id,
    }
    return {
        "operation_id": operation_id,
        "overload_id": overload_id,
        "inventory_kind": inventory_kind,
        "canonical_target": symbol,
        "resolution_state": resolution_state,
        "blocker_reason": blocker_reason,
        "call_style": source.get("usage_kinds", ""),
        "ordered_parameters_json": compact_json(parameters),
        "output_arity": output_arity,
        "output_types_json": compact_json(output_types),
        "exact_rust_signature": rust_signature,
        "reference_semantic": compact_json(semantic),
        "resolution_owner_task_id": owner_task,
        "expected_resolution_module": expected_resolution_module,
        "release_closure_required": release_closure_required,
        "mutation_rule": (
            f"unresolved; {owner_task} must record exact mutation semantics before dispatch"
            if callable_operation
            else "not executable"
        ),
        "alias_rule": source.get("layout_requirement", ""),
        "shape_rule": source.get("shape_requirement", ""),
        "dtype_rule": source.get("dtype_requirement", ""),
        "accumulation_dtype": "unresolved until callable overload resolution" if callable_operation else "not applicable",
        "layout_rule": source.get("layout_requirement", ""),
        "device_rule": source.get("device_requirement", ""),
        "numeric_rule": source.get("numerics_requirement", ""),
        "tolerance": "unresolved until oracle recording" if callable_operation else "not applicable",
        "determinism": "versioned explicit RNG stream required" if "random" in source.get("semantic_group", "") else "must match the resolved semantics profile",
        "cancellation_points": source.get("cancellation_requirement", ""),
        "vjp_rule": source.get("vjp_jvp_requirement", ""),
        "jvp_rule": source.get("vjp_jvp_requirement", ""),
        "source_call_sites": combined_sites(source),
        "oracle_fixture": fixture_id,
        "oracle_fixture_sha256": fixture_digest,
        "evidence": compact_json(evidence),
    }


def write_ledger(rows: list[dict[str, str]]) -> None:
    with LEDGER.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def enum_variant(value: str) -> str:
    variants = {
        "callable_operation": "ContractInventoryKind::CallableOperation",
        "reclassified_external_operation": "ContractInventoryKind::ReclassifiedExternalOperation",
        "namespace_value_reference": "ContractInventoryKind::NamespaceValueReference",
        "type_reference": "ContractInventoryKind::TypeReference",
        "resolved_callable": "ContractResolutionState::ResolvedCallable",
        "resolved_reference": "ContractResolutionState::ResolvedReference",
        "reclassified_external": "ContractResolutionState::ReclassifiedExternalOperation",
        "blocked_receiver_unverified": "ContractResolutionState::BlockedReceiverUnverified",
        "blocked_missing_semantics_profile": "ContractResolutionState::BlockedMissingSemanticsProfile",
        "blocked_missing_oracle_dependency": "ContractResolutionState::BlockedMissingOracleDependency",
    }
    return variants[value]


def write_rust_table(rows: list[dict[str, str]]) -> None:
    lines = [
        "pub static OPERATION_CONTRACTS: &[OperationContractRecord] = &[",
    ]
    for row in rows:
        lines.extend(
            [
                "    OperationContractRecord {",
                f"        operation_id: {rust_string(row['operation_id'])},",
                f"        overload_id: {rust_string(row['overload_id'])},",
                f"        inventory_kind: {enum_variant(row['inventory_kind'])},",
                f"        canonical_target: {rust_string(row['canonical_target'])},",
                f"        resolution_state: {enum_variant(row['resolution_state'])},",
                f"        blocker_reason: {rust_string(row['blocker_reason'])},",
                f"        call_style: {rust_string(row['call_style'])},",
                f"        ordered_parameters_json: {rust_string(row['ordered_parameters_json'])},",
                f"        output_arity: {rust_string(row['output_arity'])},",
                f"        output_types_json: {rust_string(row['output_types_json'])},",
                f"        rust_signature: {rust_string(row['exact_rust_signature'])},",
                f"        reference_semantic: {rust_string(row['reference_semantic'])},",
                f"        resolution_owner_task_id: {rust_string(row['resolution_owner_task_id'])},",
                f"        expected_resolution_module: {rust_string(row['expected_resolution_module'])},",
                f"        release_closure_required: {row['release_closure_required']},",
                f"        mutation_rule: {rust_string(row['mutation_rule'])},",
                f"        alias_rule: {rust_string(row['alias_rule'])},",
                f"        shape_rule: {rust_string(row['shape_rule'])},",
                f"        dtype_rule: {rust_string(row['dtype_rule'])},",
                f"        accumulation_dtype: {rust_string(row['accumulation_dtype'])},",
                f"        layout_rule: {rust_string(row['layout_rule'])},",
                f"        device_rule: {rust_string(row['device_rule'])},",
                f"        numeric_rule: {rust_string(row['numeric_rule'])},",
                f"        tolerance: {rust_string(row['tolerance'])},",
                f"        determinism: {rust_string(row['determinism'])},",
                f"        cancellation_points: {rust_string(row['cancellation_points'])},",
                f"        vjp_rule: {rust_string(row['vjp_rule'])},",
                f"        jvp_rule: {rust_string(row['jvp_rule'])},",
                f"        source_call_sites: {rust_string(row['source_call_sites'])},",
                f"        oracle_fixture: {rust_string(row['oracle_fixture'])},",
                f"        oracle_fixture_sha256: {rust_string(row['oracle_fixture_sha256'])},",
                f"        evidence: {rust_string(row['evidence'])},",
                "    },",
            ]
        )
    lines.extend(["];", ""])
    RUST_TABLE.parent.mkdir(parents=True, exist_ok=True)
    RUST_TABLE.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    inventory_digest = sha256(INVENTORY)
    FIXTURE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    FIXTURE.write_text(
        json.dumps(fixture_payload(inventory_digest), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    fixture_digest = sha256(FIXTURE)
    with INVENTORY.open(newline="", encoding="utf-8") as handle:
        inventory_rows = list(csv.DictReader(handle))
    CONTRACT_FIXTURE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    for stale_path in CONTRACT_FIXTURE_DIRECTORY.glob("*.json"):
        if stale_path.name.startswith("._"):
            continue
        stale_path.unlink()
    assignments = operation_owner_assignments(inventory_rows)
    rows = [
        contract_row(row, fixture_digest, *assignments[row["operation_id"]])
        for row in inventory_rows
    ]
    if len(rows) != 600:
        raise RuntimeError(f"expected 600 tensor inventory rows, found {len(rows)}")
    if len({row['operation_id'] for row in rows}) != len(rows):
        raise RuntimeError("tensor operation IDs are not unique")
    callable_count = sum(row["inventory_kind"] == "callable_operation" for row in rows)
    external_count = sum(
        row["inventory_kind"] == "reclassified_external_operation" for row in rows
    )
    type_count = sum(row["inventory_kind"] == "type_reference" for row in rows)
    reference_count = len(rows) - callable_count - external_count - type_count
    if (callable_count, external_count, reference_count, type_count) != (511, 7, 67, 15):
        raise RuntimeError(
            "tensor inventory classification drifted: "
            f"{callable_count} callable, {external_count} reclassified external, "
            f"{reference_count} namespace/value, {type_count} type"
        )
    write_ledger(rows)
    write_rust_table(rows)
    blocked = sum(row["resolution_state"].startswith("blocked_") for row in rows)
    owner_counts: dict[str, int] = defaultdict(int)
    for row in rows:
        owner_counts[row["resolution_owner_task_id"]] += 1
    if not owner_counts or max(owner_counts.values()) > 12:
        raise RuntimeError("tensor resolution-owner leaves are missing or exceed their 12-row boundary")
    reference_semantics = sorted(
        {
            json.loads(row["reference_semantic"])["category"]
            for row in rows
            if row["inventory_kind"]
            in {"namespace_value_reference", "type_reference"}
        }
    )
    print(
        f"Generated {len(rows)} tensor contracts: {callable_count} callable ({blocked} explicit blockers), "
        f"{external_count} reclassified external operations, "
        f"{reference_count} namespace/value references, {type_count} type references, "
        f"{len(owner_counts)} exact resolution-owner leaves, reference semantics {reference_semantics}."
    )


if __name__ == "__main__":
    main()
