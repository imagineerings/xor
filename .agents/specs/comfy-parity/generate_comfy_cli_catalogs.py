#!/usr/bin/env python3
"""Generate deterministic Comfy CLI evidence catalogs without importing it.

The vendored snapshot deliberately has no installed Python environment in this
workspace.  This generator therefore treats the source tree as data: Python is
parsed with ``ast``, JSON with the standard library, and the two small YAML
registries with their narrow, indentation-stable source grammar.  The bundled
OpenAPI document is inventoried without claiming that documentation is runtime
behavior; only the executable allowlist in ``spec.py`` becomes a supported
partner endpoint row.
"""

from __future__ import annotations

import ast
import csv
import hashlib
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterable


SPEC_DIR = Path(__file__).resolve().parent
WORKSPACE = SPEC_DIR.parents[2]
SOURCE_ROOT = WORKSPACE / "projects/comfy/comfy-cli"
PACKAGE_ROOT = SOURCE_ROOT / "comfy_cli"
TEST_ROOT = SOURCE_ROOT / "tests"
CATALOG_DIR = SPEC_DIR / "catalogs"

EXPECTED_FILE_COUNT = 312
EXPECTED_FINGERPRINT = "09d0b5f262bce3105f83777a310f1e391c4624f95142da5e3230626b68a276e6"


def relative(path: Path) -> str:
    return path.relative_to(SOURCE_ROOT).as_posix()


def source_ref(path: Path, line: int | None = None) -> str:
    ref = f"projects/comfy/comfy-cli/{relative(path)}"
    return f"{ref}:{line}" if line else ref


def literal(node: ast.AST | None) -> Any:
    if node is None:
        return None
    try:
        return ast.literal_eval(node)
    except (ValueError, TypeError, SyntaxError):
        return None


def dotted(node: ast.AST | None) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        base = dotted(node.value)
        return f"{base}.{node.attr}" if base else node.attr
    return None


def call_name(node: ast.AST | None) -> str | None:
    return dotted(node.func) if isinstance(node, ast.Call) else None


def keyword_literals(call: ast.Call) -> dict[str, Any]:
    return {kw.arg: literal(kw.value) for kw in call.keywords if kw.arg}


def clean(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (dict, list, tuple)):
        return json.dumps(value, sort_keys=True, ensure_ascii=False)
    return str(value).replace("\n", " ").strip()


def stable_id(prefix: str, *identity: Any) -> str:
    canonical = "\x1f".join(clean(value) for value in identity)
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:12].upper()
    return f"{prefix}-{digest}"


def verify_stable_id_regressions() -> None:
    fixtures = {
        ("COMFY-CLI-CMD", ("comfy install",)): "COMFY-CLI-CMD-1D725BCB02CF",
        ("COMFY-CLI-MODULE", ("comfy_cli/http.py",)): "COMFY-CLI-MODULE-A830A19E7CB9",
        ("COMFY-CLI-PARAM", ("comfy install", "command", "workspace", "argument")): "COMFY-CLI-PARAM-A549141EADBA",
    }
    for (prefix, identity), expected in fixtures.items():
        actual = stable_id(prefix, *identity)
        if actual != expected:
            raise RuntimeError(f"stable ID regression for {prefix} {identity}: {actual} != {expected}")


def write_csv(name: str, fields: list[str], rows: Iterable[dict[str, Any]]) -> None:
    path = CATALOG_DIR / name
    normalized = list(rows)
    for identifier_field in ("feature_id", "source_id"):
        if identifier_field not in fields:
            continue
        identifiers = [clean(row.get(identifier_field)) for row in normalized]
        if any(not identifier for identifier in identifiers) or len(identifiers) != len(set(identifiers)):
            raise RuntimeError(f"{name} has blank or colliding {identifier_field} values")
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, extrasaction="ignore", lineterminator="\n")
        writer.writeheader()
        for row in normalized:
            writer.writerow({field: clean(row.get(field)) for field in fields})


def write_json(name: str, value: Any) -> None:
    (CATALOG_DIR / name).write_text(
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def source_files() -> list[Path]:
    files = []
    for path in SOURCE_ROOT.rglob("*"):
        if not path.is_file():
            continue
        parts = set(path.parts)
        if ".git" in parts or "node_modules" in parts or "__pycache__" in parts:
            continue
        if path.suffix == ".pyc" or path.name == ".DS_Store":
            continue
        files.append(path)
    return sorted(files, key=lambda item: relative(item).encode("utf-8"))


def tree_fingerprint(files: list[Path]) -> str:
    stream = "".join(
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  ./{relative(path)}\n" for path in files
    )
    return hashlib.sha256(stream.encode("utf-8")).hexdigest()


def tests_containing(token: str, test_text: dict[str, str]) -> list[str]:
    if not token:
        return []
    return sorted(path for path, text in test_text.items() if token in text)


@dataclass
class Parameter:
    name: str
    kind: str
    flags: list[str]
    default: Any
    default_source: str
    required: bool
    annotation: str
    value_type: str
    nullable: bool
    value_arity: str
    cardinality: str
    repeatable: bool
    choices: list[str]
    constraints: dict[str, str]
    boolean_forms: list[str]
    type_evidence: str
    help: str
    hidden: bool
    envvar: str
    source_file: str
    line: int


@dataclass
class CommandRegistration:
    path: str
    source_file: str
    line: int
    symbol: str
    help: str
    hidden: str
    registration: str
    parameters: list[Parameter]
    notes: str = ""


PREFIXES: dict[tuple[str, str], list[str]] = {
    ("comfy_cli.auth.command", "app"): ["comfy auth"],
    ("comfy_cli.cloud.command", "app"): ["comfy cloud"],
    ("comfy_cli.cmdline", "app"): ["comfy"],
    ("comfy_cli.command.custom_nodes.bisect_custom_nodes", "bisect_app"): ["comfy node bisect"],
    ("comfy_cli.command.custom_nodes.command", "app"): ["comfy node"],
    ("comfy_cli.command.custom_nodes.command", "manager_app"): ["comfy manager"],
    ("comfy_cli.command.job_watcher", "app"): ["comfy _watch"],
    ("comfy_cli.command.jobs", "app"): ["comfy jobs"],
    ("comfy_cli.command.models.models", "app"): ["comfy model"],
    ("comfy_cli.command.models.search", "app"): ["comfy models"],
    ("comfy_cli.command.nodes", "app"): ["comfy nodes"],
    ("comfy_cli.command.pr_command", "app"): ["comfy pr-cache"],
    ("comfy_cli.command.project", "app"): ["comfy project"],
    ("comfy_cli.command.project", "assets_app"): ["comfy assets"],
    ("comfy_cli.command.templates", "app"): ["comfy templates"],
    ("comfy_cli.command.workflow", "app"): ["comfy workflow"],
    ("comfy_cli.command.workflow_fragments", "fragment_app"): ["comfy workflow fragment"],
    ("comfy_cli.skills.command", "app"): ["comfy skills", "comfy skill"],
    ("comfy_cli.tracking", "app"): ["comfy tracking"],
}


def module_name(path: Path) -> str:
    return ".".join(path.relative_to(SOURCE_ROOT).with_suffix("").parts)


def parameter_call(annotation: ast.AST | None, default: ast.AST | None) -> ast.Call | None:
    if annotation is not None:
        for node in ast.walk(annotation):
            if isinstance(node, ast.Call) and call_name(node) in {"typer.Option", "typer.Argument"}:
                return node
    if isinstance(default, ast.Call) and call_name(default) in {"typer.Option", "typer.Argument"}:
        return default
    return None


def source_expression(node: ast.AST | None) -> str:
    return ast.unparse(node) if node is not None else ""


def unwrap_annotated(annotation: ast.AST | None) -> ast.AST | None:
    if not isinstance(annotation, ast.Subscript) or dotted(annotation.value) not in {"Annotated", "typing.Annotated"}:
        return annotation
    elements = annotation.slice.elts if isinstance(annotation.slice, ast.Tuple) else [annotation.slice]
    return elements[0] if elements else None


def union_members(annotation: ast.AST) -> list[ast.AST]:
    if isinstance(annotation, ast.BinOp) and isinstance(annotation.op, ast.BitOr):
        return union_members(annotation.left) + union_members(annotation.right)
    if isinstance(annotation, ast.Subscript) and dotted(annotation.value) in {"Optional", "typing.Optional"}:
        return [annotation.slice, ast.Constant(value=None)]
    if isinstance(annotation, ast.Subscript) and dotted(annotation.value) in {"Union", "typing.Union"}:
        return list(annotation.slice.elts) if isinstance(annotation.slice, ast.Tuple) else [annotation.slice]
    return [annotation]


def is_none_annotation(annotation: ast.AST) -> bool:
    return isinstance(annotation, ast.Constant) and annotation.value is None


@lru_cache(maxsize=1)
def enum_choices() -> dict[str, tuple[list[str], str]]:
    choices_by_name: dict[str, tuple[list[str], str]] = {}
    for path in sorted(PACKAGE_ROOT.rglob("*.py"), key=lambda item: relative(item)):
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        for node in tree.body:
            if not isinstance(node, ast.ClassDef):
                continue
            if not any((dotted(base) or "").rsplit(".", 1)[-1] in {"Enum", "StrEnum"} for base in node.bases):
                continue
            values = []
            for statement in node.body:
                if not isinstance(statement, (ast.Assign, ast.AnnAssign)):
                    continue
                value_node = statement.value
                value = literal(value_node)
                if isinstance(value, (str, int, float, bool)):
                    values.append(clean(value))
            if not values:
                continue
            previous = choices_by_name.get(node.name)
            current = (values, source_ref(path, node.lineno))
            if previous is not None and previous != current:
                raise RuntimeError(f"ambiguous Enum declaration for CLI parameter type {node.name}")
            choices_by_name[node.name] = current
    return choices_by_name


def parameter_type_metadata(
    annotation: ast.AST | None,
    kind: str,
    required: bool,
    flags: list[str],
    info: ast.Call,
) -> dict[str, Any]:
    unwrapped = unwrap_annotated(annotation)
    if unwrapped is None:
        raise RuntimeError("CLI parameter is missing a source type annotation")
    annotation_text = source_expression(unwrapped)
    members = union_members(unwrapped)
    nullable = any(is_none_annotation(member) for member in members)
    value_members = [member for member in members if not is_none_annotation(member)]
    if len(value_members) != 1:
        raise RuntimeError(f"unsupported CLI parameter union annotation: {annotation_text}")
    value_node = value_members[0]
    collection = False
    if isinstance(value_node, ast.Subscript) and dotted(value_node.value) in {
        "list",
        "typing.List",
        "Sequence",
        "typing.Sequence",
    }:
        collection = True
        value_node = value_node.slice
    type_name = dotted(value_node)
    scalar_types = {
        "str": "string",
        "int": "integer",
        "float": "number",
        "bool": "boolean",
        "Path": "path",
        "pathlib.Path": "path",
    }
    choices: list[str] = []
    type_evidence = "static source annotation"
    if type_name in scalar_types:
        value_type = scalar_types[type_name]
    else:
        enum_name = (type_name or "").rsplit(".", 1)[-1]
        enum_contract = enum_choices().get(enum_name)
        if enum_contract is None:
            raise RuntimeError(f"unresolved CLI parameter type annotation: {annotation_text}")
        choices, enum_source = enum_contract
        value_type = "enum"
        type_evidence = f"static source annotation; Enum values from {enum_source}"

    if kind == "option" and value_type == "boolean" and not collection:
        value_arity = "0"
        cardinality = "1 flag occurrence" if required else "0..1 flag occurrences"
    elif kind == "argument" and collection:
        value_arity = "variadic"
        cardinality = "1..* values" if required else "0..* values"
    elif collection:
        value_arity = "1 per occurrence"
        cardinality = "1..* values" if required else "0..* values"
    else:
        value_arity = "1"
        cardinality = "1 value" if required else "0..1 values"

    keyword_sources = {keyword.arg: source_expression(keyword.value) for keyword in info.keywords if keyword.arg}
    constraints = {
        name: keyword_sources[name]
        for name in ("callback", "autocompletion", "metavar", "hide_input")
        if name in keyword_sources
    }
    boolean_forms = []
    if value_type == "boolean":
        for flag in flags:
            if "/" in flag:
                boolean_forms.extend(part for part in flag.split("/") if part)
    return {
        "annotation": annotation_text,
        "value_type": value_type,
        "nullable": nullable,
        "value_arity": value_arity,
        "cardinality": cardinality,
        "repeatable": collection,
        "choices": choices,
        "constraints": constraints,
        "boolean_forms": boolean_forms,
        "type_evidence": type_evidence,
    }


def effective_default(default_node: ast.AST | None, info: ast.Call) -> tuple[Any, str, bool]:
    if default_node is None:
        return None, "<required>", True
    source_node = default_node
    if default_node is info:
        if info.args:
            source_node = info.args[0]
        else:
            default_keyword = next((keyword.value for keyword in info.keywords if keyword.arg == "default"), None)
            if default_keyword is None:
                return None, "<required>", True
            source_node = default_keyword
    value = literal(source_node)
    required = value is ...
    return (None if required else value), ("<required>" if required else source_expression(source_node)), required


def extract_parameters(function: ast.FunctionDef | ast.AsyncFunctionDef, source_file: str) -> list[Parameter]:
    args = list(function.args.posonlyargs) + list(function.args.args)
    defaults = [None] * (len(args) - len(function.args.defaults)) + list(function.args.defaults)
    output: list[Parameter] = []
    for argument, default_node in zip(args, defaults):
        if argument.arg in {"ctx", "_ctx", "context"}:
            continue
        info = parameter_call(argument.annotation, default_node)
        info_name = call_name(info)
        if info_name is None:
            continue
        meta = keyword_literals(info)
        if info_name == "typer.Option":
            flags = [value for value in (literal(item) for item in info.args) if isinstance(value, str) and value.startswith("-")]
            if not flags:
                flags = ["--" + argument.arg.replace("_", "-")]
            kind = "option"
        else:
            flags = []
            kind = "argument"
        default_value, default_source, required = effective_default(default_node, info)
        type_metadata = parameter_type_metadata(argument.annotation, kind, required, flags, info)
        output.append(
            Parameter(
                name=argument.arg,
                kind=kind,
                flags=flags,
                default=default_value,
                default_source=default_source,
                required=required,
                annotation=type_metadata["annotation"],
                value_type=type_metadata["value_type"],
                nullable=type_metadata["nullable"],
                value_arity=type_metadata["value_arity"],
                cardinality=type_metadata["cardinality"],
                repeatable=type_metadata["repeatable"],
                choices=type_metadata["choices"],
                constraints=type_metadata["constraints"],
                boolean_forms=type_metadata["boolean_forms"],
                type_evidence=type_metadata["type_evidence"],
                help=clean(meta.get("help")),
                hidden=bool(meta.get("hidden", False)),
                envvar=clean(meta.get("envvar")),
                source_file=source_file,
                line=argument.lineno,
            )
        )
    return output


def parameter_contract_fields(parameter: Parameter) -> dict[str, Any]:
    return {
        "annotation": parameter.annotation,
        "value_type": parameter.value_type,
        "nullable": parameter.nullable,
        "value_arity": parameter.value_arity,
        "cardinality": parameter.cardinality,
        "repeatable": parameter.repeatable,
        "choices": " | ".join(parameter.choices),
        "constraints": parameter.constraints,
        "boolean_forms": " | ".join(parameter.boolean_forms),
        "default_source": parameter.default_source,
        "type_evidence": parameter.type_evidence,
    }


def command_name(decorator: ast.Call, function_name: str) -> str:
    explicit = literal(decorator.args[0]) if decorator.args else None
    for keyword in decorator.keywords:
        if keyword.arg == "name":
            explicit = literal(keyword.value)
    return explicit if isinstance(explicit, str) else function_name.replace("_", "-")


def command_test_refs(command: CommandRegistration, test_text: dict[str, str]) -> list[str]:
    module_tail = Path(command.source_file).stem
    candidates = set(tests_containing(command.symbol, test_text))
    candidates.update(path for path in test_text if module_tail in Path(path).stem)
    return sorted(candidates)[:20]


def command_target_status(path: str) -> tuple[str, str]:
    cloudish = path.startswith("comfy cloud") or path == "comfy generate"
    external = path in {"comfy code-search", "comfy cs", "comfy feedback", "comfy agent-review"}
    python_process = (
        path in {"comfy install", "comfy launch", "comfy stop", "comfy logs", "comfy dependency", "comfy standalone"}
        or path.startswith("comfy manager ")
        or path.startswith("comfy node ")
    )
    if python_process:
        return "conflicting", "Replace with native Rust runtime/plugin lifecycle; production must never spawn Python or cm-cli."
    if cloudish or external:
        return "deferred", "Retain as an explicit contract; implement only through approved native service integration."
    return "missing", "Implement natively in Rust/GPUI or expose through Zed's native compatibility service."


def extract_commands(test_text: dict[str, str]) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, str]]:
    registrations: dict[str, list[CommandRegistration]] = defaultdict(list)
    function_index: dict[tuple[str, str], tuple[ast.FunctionDef | ast.AsyncFunctionDef, Path]] = {}
    root_callback: tuple[ast.FunctionDef | ast.AsyncFunctionDef, Path] | None = None
    code_search_callback: tuple[ast.FunctionDef | ast.AsyncFunctionDef, Path] | None = None

    for path in sorted(PACKAGE_ROOT.rglob("*.py"), key=lambda item: relative(item)):
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        module = module_name(path)
        for node in tree.body:
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            function_index[(module, node.name)] = (node, path)
            for decorator in node.decorator_list:
                if not isinstance(decorator, ast.Call):
                    continue
                decorator_name = call_name(decorator)
                if not decorator_name or decorator_name.rsplit(".", 1)[-1] not in {"command", "callback"}:
                    continue
                owner, kind = decorator_name.rsplit(".", 1)
                if kind == "callback":
                    if module == "comfy_cli.cmdline" and node.name == "entry":
                        root_callback = (node, path)
                    elif module == "comfy_cli.command.code_search":
                        code_search_callback = (node, path)
                    continue
                prefixes = PREFIXES.get((module, owner), [])
                if not prefixes:
                    continue
                name = command_name(decorator, node.name)
                if module == "comfy_cli.cmdline" and name == "models":
                    continue
                meta = keyword_literals(decorator)
                hidden = "true" if meta.get("hidden") is True else "false"
                for prefix in prefixes:
                    full_path = f"{prefix} {name}"
                    if prefix == "comfy skill":
                        hidden = "true"
                    registrations[full_path].append(
                        CommandRegistration(
                            path=full_path,
                            source_file=relative(path),
                            line=node.lineno,
                            symbol=node.name,
                            help=clean(meta.get("help")) or clean(ast.get_docstring(node)),
                            hidden=hidden,
                            registration="decorator",
                            parameters=extract_parameters(node, relative(path)),
                        )
                    )

    direct = [
        ("comfy preview", "comfy_cli.command.preview", "preview_cmd", "Render a previewable PNG from image, video, or audio."),
        ("comfy workflow compose", "comfy_cli.command.workflow_fragments", "compose_cmd", "Compose a YAML blueprint into API workflow JSON."),
        ("comfy workflow decompose", "comfy_cli.command.workflow_fragments", "decompose_cmd", "Project a workflow into a reusable fragment."),
    ]
    for full_path, module, symbol, help_text in direct:
        function, path = function_index[(module, symbol)]
        registrations[full_path].append(
            CommandRegistration(
                path=full_path,
                source_file=relative(path),
                line=function.lineno,
                symbol=symbol,
                help=help_text,
                hidden="false",
                registration="direct command registration",
                parameters=extract_parameters(function, relative(path)),
            )
        )

    generate_path = PACKAGE_ROOT / "command/generate/app.py"
    registrations["comfy generate"].append(
        CommandRegistration(
            path="comfy generate",
            source_file=relative(generate_path),
            line=40,
            symbol="register_with._generate_entry",
            help="Generate media through one of 52 schema-driven partner endpoints or perform list/schema/refresh/upload/resume actions.",
            hidden="false",
            registration="dynamic register_with",
            parameters=[],
        )
    )

    if code_search_callback:
        function, path = code_search_callback
        params = extract_parameters(function, relative(path))
        for full_path, hidden in (("comfy code-search", "false"), ("comfy cs", "true")):
            registrations[full_path].append(
                CommandRegistration(
                    path=full_path,
                    source_file=relative(path),
                    line=function.lineno,
                    symbol=function.name,
                    help=clean(ast.get_docstring(function)),
                    hidden=hidden,
                    registration="callback leaf alias",
                    parameters=params,
                )
            )

    if len(registrations) != 123:
        raise RuntimeError(f"expected 123 reachable command paths, found {len(registrations)}")

    command_rows: list[dict[str, Any]] = []
    parameter_rows: list[dict[str, Any]] = []
    command_ids: dict[str, str] = {}
    for full_path in sorted(registrations):
        command_id = stable_id("COMFY-CLI-CMD", full_path)
        command_ids[full_path] = command_id
        entries = registrations[full_path]
        chosen = entries[0]
        hidden_values = {entry.hidden for entry in entries}
        hidden = next(iter(hidden_values)) if len(hidden_values) == 1 else "ambiguous"
        notes = []
        if len(entries) > 1:
            notes.append(f"{len(entries)} registrations collapse to one path")
        if full_path == "comfy dependency":
            notes.append("stacked visible and hidden decorators create conflicting presentation metadata")
        status, decision = command_target_status(full_path)
        refs = command_test_refs(chosen, test_text)
        availability = "active"
        if full_path.startswith("comfy cloud") or full_path == "comfy generate":
            availability = "cloud/paid"
        elif hidden == "true" or full_path.startswith("comfy pr-cache"):
            availability = "developer-only"
        command_rows.append(
            {
                "feature_id": command_id,
                "path": full_path,
                "top_level": full_path.split()[1],
                "hidden": hidden,
                "classification": "reachable CLI leaf",
                "availability": availability,
                "evidence_level": "code-inferred",
                "confidence": "high",
                "help": chosen.help,
                "source_file": chosen.source_file,
                "symbol": chosen.symbol,
                "line": chosen.line,
                "registration": chosen.registration,
                "tests": " | ".join(refs),
                "target_status": status,
                "parity_decision": decision,
                "notes": "; ".join(notes),
            }
        )
        seen_params: set[tuple[str, str, tuple[str, ...]]] = set()
        for entry in entries:
            for parameter in entry.parameters:
                key = (parameter.name, parameter.kind, tuple(parameter.flags))
                if key in seen_params:
                    continue
                seen_params.add(key)
                parameter_rows.append(
                    {
                        "command_id": command_id,
                        "command_path": full_path,
                        "scope": "command",
                        "name": parameter.name,
                        "kind": parameter.kind,
                        "flags": " | ".join(parameter.flags),
                        "default": parameter.default,
                        "required": parameter.required,
                        **parameter_contract_fields(parameter),
                        "hidden": parameter.hidden,
                        "envvar": parameter.envvar,
                        "help": parameter.help,
                        "source_file": parameter.source_file,
                        "line": parameter.line,
                        "evidence_level": "code-inferred",
                    }
                )

    if root_callback:
        function, path = root_callback
        for parameter in extract_parameters(function, relative(path)):
            parameter_rows.append(
                {
                    "command_id": "COMFY-CLI-ROOT",
                    "command_path": "comfy",
                    "scope": "global",
                    "name": parameter.name,
                    "kind": parameter.kind,
                    "flags": " | ".join(parameter.flags),
                    "default": parameter.default,
                    "required": parameter.required,
                    **parameter_contract_fields(parameter),
                    "hidden": parameter.hidden,
                    "envvar": parameter.envvar,
                    "help": parameter.help,
                    "source_file": parameter.source_file,
                    "line": parameter.line,
                    "evidence_level": "code-inferred",
                }
            )

    generate_tree = ast.parse(generate_path.read_text(encoding="utf-8"), filename=str(generate_path))
    generate_entry = next(
        (
            node
            for node in ast.walk(generate_tree)
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == "_generate_entry"
        ),
        None,
    )
    if generate_entry is None:
        raise RuntimeError("comfy generate dynamic entry function is missing")
    generate_target = extract_parameters(generate_entry, relative(generate_path))
    if len(generate_target) != 1 or generate_target[0].name != "target":
        raise RuntimeError("comfy generate target parameter grammar changed")
    parameter = generate_target[0]
    parameter_rows.append(
        {
            "command_id": command_ids["comfy generate"],
            "command_path": "comfy generate",
            "scope": "dynamic fixed",
            "name": parameter.name,
            "kind": parameter.kind,
            "flags": " | ".join(parameter.flags),
            "default": parameter.default,
            "required": parameter.required,
            **parameter_contract_fields(parameter),
            "constraints": {
                **parameter.constraints,
                "reserved_actions": "list | schema | refresh | upload | resume",
                "otherwise": "partner model alias",
            },
            "hidden": parameter.hidden,
            "envvar": parameter.envvar,
            "help": parameter.help,
            "source_file": parameter.source_file,
            "line": parameter.line,
            "evidence_level": "code-inferred",
        }
    )

    dynamic_parameters = [
        ("download", "--download", "string", "", "", {}, "Save returned media."),
        ("async", "--async", "boolean", "false", "", {"inline_false_values": "false | 0 | no", "other_inline_values": "true"}, "Submit an asynchronous partner job without polling."),
        ("json", "--json", "boolean", "false", "", {"inline_false_values": "false | 0 | no", "other_inline_values": "true"}, "Emit partner-command JSON."),
        ("timeout", "--timeout", "number", "300.0", "", {"value_parser": "float"}, "Partner request or polling timeout."),
        ("api_key", "--api-key", "string", "", "COMFY_API_KEY", {}, "Per-call API-key override."),
        ("emit_workflow", "--emit-workflow", "path", "", "", {}, "Write a runnable workflow instead of calling the proxy."),
        ("output_prefix", "--output-prefix", "string", "generate", "", {"default_applies_when": "--emit-workflow"}, "Output filename prefix."),
        ("partner", "--partner | -p", "string", "", "", {}, "Filter `generate list` by partner."),
        ("category", "--category | --style | -c", "string", "", "", {}, "Filter `generate list` by category/style."),
        ("query", "--query | -q", "string", "", "", {}, "Filter `generate list` by query."),
    ]
    for name, flags, value_type, default_source, envvar, additional_constraints, help_text in dynamic_parameters:
        boolean = value_type == "boolean"
        parameter_rows.append(
            {
                "command_id": command_ids["comfy generate"],
                "command_path": "comfy generate",
                "scope": "dynamic fixed",
                "name": name,
                "kind": "option",
                "flags": flags,
                "default": default_source,
                "default_source": default_source or "None",
                "required": False,
                "annotation": value_type,
                "value_type": value_type,
                "nullable": not boolean,
                "value_arity": "0 or 1 inline" if boolean else "1",
                "cardinality": "0..1 flag occurrences" if boolean else "0..1 values",
                "repeatable": False,
                "choices": "",
                "constraints": {
                    "parser": "_separate_meta_flags" if name not in {"partner", "category", "query"} else "_arg_value",
                    **additional_constraints,
                },
                "boolean_forms": "",
                "type_evidence": "static dynamic-parser branches",
                "hidden": False,
                "envvar": envvar,
                "help": help_text,
                "source_file": "comfy_cli/command/generate/app.py",
                "line": 87 if name not in {"partner", "category", "query"} else 428,
                "evidence_level": "code-inferred",
            }
        )

    shadowed = {
        "comfy models (legacy hidden function)": "Shadowed by the visible `models` Typer group; retain as deprecated/dead source evidence.",
        "comfy version": "Advertised by COMMAND_SCHEMAS but no command registration exists; global --version is the executable surface.",
        "comfy query": "Only HELP_EXAMPLES/run-cli text mention it; no command registration exists.",
    }
    return command_rows, sorted(parameter_rows, key=lambda row: (row["command_path"], row["scope"], row["name"])), shadowed


def assignment_literal(tree: ast.Module, name: str) -> Any:
    for node in tree.body:
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and node.target.id == name:
            return literal(node.value)
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == name:
                    return literal(node.value)
    return None


def extract_schemas() -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, str], dict[str, str]]:
    schema_dir = PACKAGE_ROOT / "schemas"
    discovery_path = PACKAGE_ROOT / "discovery.py"
    tree = ast.parse(discovery_path.read_text(encoding="utf-8"))
    command_mappings: dict[str, str] = assignment_literal(tree, "COMMAND_SCHEMAS") or {}
    stream_mappings: dict[str, str] = assignment_literal(tree, "STREAM_EVENT_SCHEMAS") or {}
    schema_rows = []
    for path in sorted(schema_dir.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        schema_rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-SCHEMA", path.stem),
                "name": path.stem,
                "schema_id": data.get("$id", ""),
                "title": data.get("title", ""),
                "draft": data.get("$schema", ""),
                "type": data.get("type", ""),
                "required": data.get("required", []),
                "top_level_properties": len(data.get("properties", {})) if isinstance(data.get("properties"), dict) else 0,
                "source_file": relative(path),
                "evidence_level": "test-backed",
                "tests": "tests/comfy_cli/output/test_envelope_schemas.py",
                "target_status": "missing",
                "parity_decision": "Implement the schema and version negotiation natively; preserve compatible JSON/NDJSON at the CLI/protocol boundary.",
            }
        )
    mappings = []
    for kind, values in (("envelope", command_mappings), ("stream", stream_mappings)):
        for command_path, schema in sorted(values.items()):
            mappings.append(
                {
                    "mapping_kind": kind,
                    "command_path": command_path,
                    "schema": schema,
                    "reachable": command_path != "comfy version",
                    "source_file": "comfy_cli/discovery.py",
                    "evidence_level": "code-inferred",
                    "notes": "Orphan mapping: no registered `comfy version` leaf; global --version emits this payload."
                    if command_path == "comfy version"
                    else "",
                }
            )
    return schema_rows, mappings, command_mappings, stream_mappings


def extract_error_codes() -> list[dict[str, Any]]:
    path = PACKAGE_ROOT / "error_codes.py"
    tree = ast.parse(path.read_text(encoding="utf-8"))
    rows = []
    calls = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call) or call_name(node) != "ErrorCode" or not node.args:
            continue
        code = literal(node.args[0])
        if isinstance(code, str):
            calls.append((code, clean(literal(node.args[1])) if len(node.args) > 1 else "", clean(literal(node.args[2])) if len(node.args) > 2 else "", node.lineno))
    if len(calls) != 99 or len({row[0] for row in calls}) != 99:
        raise RuntimeError(f"expected 99 unique error codes, found {len(calls)}")
    for code, meaning, hint, line in calls:
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-ERROR", code),
                "code": code,
                "meaning": meaning,
                "hint": hint,
                "source_file": relative(path),
                "line": line,
                "evidence_level": "test-backed",
                "tests": "tests/comfy_cli/output/test_error_code_registry.py",
                "target_status": "missing",
                "parity_decision": "Preserve stable code and exit semantics at native compatibility surfaces.",
            }
        )
    return rows


def extract_events() -> list[dict[str, Any]]:
    schema_path = PACKAGE_ROOT / "schemas/run_event.json"
    enum_values = set(json.loads(schema_path.read_text(encoding="utf-8"))["properties"]["type"]["enum"])
    code_sites: dict[str, list[str]] = defaultdict(list)
    for path in sorted(PACKAGE_ROOT.rglob("*.py")):
        tree = ast.parse(path.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute) or node.func.attr != "event":
                continue
            value = literal(node.args[0]) if node.args else None
            if isinstance(value, str):
                code_sites[value].append(f"{relative(path)}:{node.lineno}")
    union = sorted(enum_values | set(code_sites))
    rows = []
    for event in union:
        in_schema = event in enum_values
        in_code = event in code_sites
        notes = ""
        if in_code and not in_schema:
            notes = "Executable code emits this event but run_event.json rejects it; versioned contract conflict."
        elif in_schema and not in_code:
            notes = "Declared in run_event.json; emitted through a non-literal wrapper or cancellation envelope path."
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-EVENT", event),
                "event": event,
                "in_run_event_schema": in_schema,
                "literal_code_emission": in_code,
                "source_sites": " | ".join(code_sites.get(event, [])),
                "evidence_level": "code-inferred",
                "contract_status": "conflicting" if in_code and not in_schema else "aligned",
                "target_status": "missing",
                "parity_decision": "Define one native versioned event union and contract-test every emitted variant.",
                "notes": notes,
            }
        )
    return rows


def first_token_site(token: str) -> tuple[str, int]:
    for path in sorted(PACKAGE_ROOT.rglob("*.py"), key=lambda item: relative(item)):
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if token in line:
                return relative(path), line_no
    return "", 0


ENVIRONMENT_ROWS = [
    ("AI_AGENT", "Caller detection; truthy values select agentic JSON-oriented defaults.", "input"),
    ("CIVITAI_API_TOKEN", "Civitai model-download credential override.", "secret input"),
    ("CLAUDECODE", "Caller detection for Claude Code sessions.", "input"),
    ("COMFYUI_MANAGER_ARIA2_SECRET", "aria2 RPC secret.", "secret input"),
    ("COMFYUI_MANAGER_ARIA2_SERVER", "aria2 RPC server URL.", "input"),
    ("COMFYUI_PATH", "Exported to ComfyUI-Manager subprocesses as the selected workspace.", "child output"),
    ("COMFY_API_BASE_URL", "Override the partner/OpenAPI API base URL.", "input"),
    ("COMFY_API_KEY", "Partner-node/API credential and Typer envvar for run.", "secret input"),
    ("COMFY_CLI_BACKGROUND", "Marks the recursively launched background CLI process.", "child input/output"),
    ("COMFY_CLOUD_API_KEY", "Deprecated/testing cloud API-key credential fallback.", "secret input"),
    ("COMFY_CLOUD_BASE_URL", "Cloud frontend/API origin override.", "input"),
    ("COMFY_CLOUD_RESOURCE_URL", "OAuth protected-resource override.", "input"),
    ("COMFY_CLOUD_SCOPES", "Whitespace-separated OAuth scope override.", "input"),
    ("COMFY_NODE_CHANGELOG", "Custom-node publish changelog fallback.", "input"),
    ("COMFY_NO_TELEMETRY", "Any nonempty value other than 0 disables telemetry.", "input"),
    ("COMFY_NO_UPDATE_CHECK", "Disables the daily PyPI update check.", "input"),
    ("COMFY_OUTPUT", "Selects pretty/json/ndjson output when flags do not override it.", "input"),
    ("COMFY_SECRETS_PATH", "Overrides the plaintext secret-store path.", "input"),
    ("COMFY_USER_AGENT", "Explicit agent caller label and agentic-mode override.", "input"),
    ("COMFY_WHERE", "Selects local or cloud routing after an explicit flag.", "input/output"),
    ("CONDA_DEFAULT_ENV", "Displayed environment name.", "input"),
    ("CONDA_PREFIX", "Workspace Python resolution candidate.", "input"),
    ("CUDA_VISIBLE_DEVICES", "Temporarily cleared while probing CUDA driver support, then restored.", "input/mutated/restored"),
    ("DO_NOT_TRACK", "Cross-tool telemetry opt-out.", "input"),
    ("ENVIRONMENT", "Selects development, staging, or production node-registry base URL.", "input"),
    ("GITHUB_TOKEN", "Optional GitHub API bearer for release and PR lookup.", "secret input"),
    ("HF_API_TOKEN", "Hugging Face model-download credential override.", "secret input"),
    ("LOG_LEVEL", "Python logging threshold; defaults to ERROR.", "input"),
    ("POSTHOG_API_KEY", "Overrides the embedded public PostHog ingest key.", "secret-like input"),
    ("PYTHONENCODING", "Set to UTF-8 for foreground ComfyUI child execution.", "child output"),
    ("PYTHONIOENCODING", "Set to UTF-8 for background child execution.", "child output"),
    ("PYTHONUNBUFFERED", "Set for background child readiness-line delivery.", "child output"),
    ("VIRTUAL_ENV", "Workspace Python resolution candidate.", "input"),
    ("XDG_CACHE_HOME", "Overrides object-info and gallery cache roots.", "input"),
    ("__COMFY_CLI_SESSION__", "Private reboot-session marker path sent to ComfyUI/Manager children.", "child output"),
]


CONFIG_ROWS = [
    ("background", "Tuple-like host, port, pid for the owned background server.", "active"),
    ("background_log", "Absolute background logfile path.", "active"),
    ("civitai_api_token", "Legacy plaintext Civitai token in config.ini.", "deprecated/dead"),
    ("cloud_base_url", "Persisted cloud base URL.", "conditional"),
    ("cloud_resource_url", "Persisted OAuth protected-resource URL.", "conditional"),
    ("cloud_scopes", "Persisted OAuth scope string.", "conditional"),
    ("default_downloader", "Model downloader selection; fallback httpx.", "active"),
    ("default_launch_extras", "Whitespace-split default arguments for the default workspace.", "active"),
    ("default_project_dir", "Fallback project/output directory.", "active"),
    ("default_workspace", "Default ComfyUI workspace path.", "active"),
    ("enable_tracking", "Tri-state user telemetry consent.", "active"),
    ("hf_api_token", "Legacy plaintext Hugging Face token in config.ini.", "deprecated/dead"),
    ("install_event_triggered", "One-time telemetry-install marker.", "active"),
    ("manager_gui_enabled", "Legacy Manager GUI boolean migrated by precedence.", "deprecated/dead"),
    ("manager_gui_mode", "disable, enable-gui, disable-gui, or enable-legacy-gui.", "conditional"),
    ("recent_workspace", "Most recently selected valid ComfyUI workspace.", "active"),
    ("setup_nudged", "One-time first-run setup nudge marker.", "active"),
    ("user_id", "Anonymous telemetry identity, generated under documented consent rules.", "active"),
    ("uv_compile_default", "Default Manager unified dependency resolution toggle.", "conditional"),
    ("where_default", "Persisted local/cloud routing default.", "active"),
]


def keyed_rows(prefix: str, values: list[tuple[str, str, str]], test_text: dict[str, str], target_status: str) -> list[dict[str, Any]]:
    rows = []
    for key, behavior, classification in sorted(values):
        path, line = first_token_site(key)
        tests = tests_containing(key, test_text)
        rows.append(
            {
                "feature_id": stable_id(prefix, key),
                "key": key,
                "behavior": behavior,
                "classification": classification,
                "source_file": path,
                "line": line,
                "evidence_level": "test-backed" if tests else "code-inferred",
                "tests": " | ".join(tests[:20]),
                "target_status": target_status,
                "parity_decision": "Replace Python-child semantics with native state/configuration while preserving externally visible precedence and safety.",
            }
        )
    return rows


FORMAT_ROWS = [
    ("envelope/1 JSON", "Versioned final machine-output envelope with ok/error and exit-code correspondence.", "comfy_cli/schemas/envelope.json"),
    ("event/1 NDJSON", "One event per line with a final envelope line.", "comfy_cli/schemas/run_event.json"),
    ("help/discovery JSON", "Self-describing command, schema, error, and capability surface.", "comfy_cli/discovery.py"),
    ("frontend workflow JSON", "LiteGraph nodes/links/definitions shape accepted for conversion and slot editing.", "comfy_cli/workflow_to_api.py"),
    ("API prompt JSON", "Node-id keyed class_type/inputs/_meta graph submitted to execution.", "comfy_cli/command/run/loader.py"),
    ("object_info JSON", "Node schema registry accepted from file, local server, cloud, or stale cache.", "comfy_cli/cql/loader.py"),
    ("fragment/1 JSON", "Reusable graph fragment with explicit named input/output/parameter ports and legacy identifiers.", "comfy_cli/fragments.py"),
    ("blueprint YAML", "Fragment composition, aliases, foreach, save, $asset, and $var references.", "comfy_cli/fragments.py"),
    ("compose/1 metadata", "Provenance block embedded in compiled workflow and stripped before submit.", "comfy_cli/command/workflow_fragments.py"),
    ("project/1 comfy.yaml", "Project marker, default routing, variables, and conventional directory layout.", "comfy_cli/project.py"),
    ("assets-lock/1 JSON", "SHA-256 keyed project asset upload lock.", "comfy_cli/project.py"),
    ("job-state JSON", "Atomic per-prompt local/cloud lifecycle record tolerant of unknown/missing fields.", "comfy_cli/jobs_state.py"),
    ("config.ini", "Per-OS DEFAULT-section configuration store.", "comfy_cli/config_manager.py"),
    ("secrets.json", "0600 atomic provider-key and OAuth-session store with stable sidecar lock.", "comfy_cli/auth/store.py"),
    ("update-check.json", "Best-effort 24-hour PyPI version cache.", "comfy_cli/update.py"),
    ("skills-manifest.json", "Installed agent-skill provenance, content SHA, and CLI version.", "comfy_cli/skills/__init__.py"),
    ("gallery index JSON", "XDG-aware cached workflow-template index.", "comfy_cli/command/templates.py"),
    ("OpenAPI cache YAML", "Seven-day partner endpoint schema cache at ~/.comfy/openapi-cache.yml.", "comfy_cli/command/generate/spec.py"),
    ("object_info cache JSON", "Atomic per-target schema cache keyed by a SHA-256 target digest.", "comfy_cli/cql/loader.py"),
    ("comfy.lock.yaml", "Workspace model/custom-node metadata; README field schema is explicitly beta/WIP.", "comfy_cli/workspace_manager.py"),
    ("background log text", "Owner-only per-workspace/per-port process log, truncated on launch and bounded in JSON output.", "comfy_cli/command/launch.py"),
    ("custom-node pyproject TOML", "Python project plus [tool.comfy] package/registry metadata.", "comfy_cli/registry/config_parser.py"),
    (".comfyignore", "Pathspec exclusions applied when packing custom nodes.", "comfy_cli/command/custom_nodes/command.py"),
    ("node.zip", "Registry upload archive for a custom-node release.", "comfy_cli/constants.py"),
    ("Manager snapshot", "ComfyUI-Manager environment snapshot accepted for save/restore and workflow dependency extraction.", "comfy_cli/command/custom_nodes/command.py"),
    ("dependency metadata", "requirements.txt, pyproject.toml, setup.cfg, setup.py, and requirements.compiled inputs.", "comfy_cli/uv.py"),
    ("media files", "Image/video/audio inputs and outputs plus preview PNGs through ffprobe/ffmpeg.", "comfy_cli/command/preview.py"),
    ("model files", ".ckpt, .pt, .bin, .pth, and .safetensors discovery/download/removal.", "comfy_cli/constants.py"),
    ("workflow metadata media", "Workflow dependency extraction accepts .json and PNG metadata through Manager.", "comfy_cli/command/custom_nodes/command.py"),
    ("python.tgz standalone", "Dehydrated standalone Python/Comfy dependency archive.", "comfy_cli/standalone.py"),
    ("PR cache metadata JSON", "Seven-day/ten-item frontend PR cache record and dist directory.", "comfy_cli/pr_cache.py"),
    ("run journal JSONL", "Append-only best-effort project run provenance.", "comfy_cli/project.py"),
    ("agent skill files", "SKILL.md, Cursor .mdc, and fenced AGENTS.md blocks with manifest state.", "comfy_cli/skills/__init__.py"),
    ("multipart transfer", "Upload/image requests and downloaded output bytes with redirect/auth controls.", "comfy_cli/command/transfer.py"),
]


FORMAT_TESTS = {
    "envelope/1 JSON": ["tests/comfy_cli/output/test_envelope_schemas.py", "tests/comfy_cli/output/test_renderer.py"],
    "event/1 NDJSON": ["tests/comfy_cli/command/test_run_json.py", "tests/comfy_cli/command/generate/test_emit.py"],
    "help/discovery JSON": ["tests/comfy_cli/output/test_discovery.py", "tests/comfy_cli/output/test_help_json.py"],
    "frontend workflow JSON": ["tests/comfy_cli/test_workflow_to_api.py"],
    "API prompt JSON": ["tests/comfy_cli/command/test_run.py", "tests/comfy_cli/command/test_run_json.py"],
    "object_info JSON": ["tests/comfy_cli/cql/test_loader.py", "tests/comfy_cli/cql/test_loader_resilient.py", "tests/comfy_cli/command/test_nodes_introspect.py"],
    "fragment/1 JSON": ["tests/comfy_cli/command/test_workflow_fragments.py"],
    "blueprint YAML": ["tests/comfy_cli/command/test_workflow_fragments.py"],
    "compose/1 metadata": ["tests/comfy_cli/command/test_workflow_fragments.py"],
    "project/1 comfy.yaml": ["tests/comfy_cli/test_project.py", "tests/comfy_cli/command/test_project_command.py"],
    "assets-lock/1 JSON": ["tests/comfy_cli/test_project.py", "tests/comfy_cli/command/test_assets_push.py"],
    "job-state JSON": ["tests/comfy_cli/test_jobs_state.py", "tests/comfy_cli/jobs/test_jobs.py"],
    "config.ini": ["tests/comfy_cli/test_config_manager.py"],
    "secrets.json": ["tests/comfy_cli/auth/test_store.py", "tests/comfy_cli/test_credentials.py"],
    "update-check.json": ["tests/comfy_cli/test_update.py"],
    "skills-manifest.json": ["tests/comfy_cli/skills/test_installer.py"],
    "gallery index JSON": ["tests/comfy_cli/command/test_templates.py"],
    "OpenAPI cache YAML": ["tests/comfy_cli/command/generate/test_spec.py"],
    "object_info cache JSON": ["tests/comfy_cli/cql/test_loader_resilient.py"],
    "comfy.lock.yaml": ["tests/comfy_cli/test_workspace_manager.py"],
    "background log text": ["tests/comfy_cli/command/test_launch_background.py", "tests/comfy_cli/command/test_logs.py"],
    "custom-node pyproject TOML": ["tests/comfy_cli/registry/test_config_parser.py"],
    ".comfyignore": ["tests/comfy_cli/command/nodes/test_pack.py"],
    "node.zip": ["tests/comfy_cli/command/nodes/test_pack.py", "tests/comfy_cli/command/nodes/test_publish.py"],
    "Manager snapshot": ["tests/comfy_cli/command/nodes/test_node_install.py"],
    "dependency metadata": ["tests/comfy_cli/command/test_cm_cli_util.py", "tests/comfy_cli/command/nodes/test_node_install.py"],
    "media files": ["tests/comfy_cli/command/test_preview.py", "tests/comfy_cli/command/test_transfer_download.py"],
    "model files": ["tests/comfy_cli/command/models/test_models.py", "tests/comfy_cli/command/models/test_search.py"],
    "workflow metadata media": ["tests/comfy_cli/command/nodes/test_node_install.py"],
    "python.tgz standalone": ["tests/comfy_cli/test_standalone.py"],
    "PR cache metadata JSON": ["tests/comfy_cli/command/test_frontend_pr.py", "tests/comfy_cli/command/test_launch_frontend_pr.py"],
    "run journal JSONL": ["tests/comfy_cli/test_project.py"],
    "agent skill files": ["tests/comfy_cli/skills/test_installer.py"],
    "multipart transfer": ["tests/comfy_cli/command/test_transfer_upload.py", "tests/comfy_cli/command/test_transfer_redirect.py"],
}


LIFECYCLE_ROWS = [
    ("first-run nudge", "Interactive pretty-only, unsigned-in users see one best-effort setup nudge; marker prevents repeats.", "comfy_cli/onboarding.py", "test_onboarding"),
    ("setup wizard", "Interactive/noninteractive routing, authentication, project directory, skills, and telemetry consent.", "comfy_cli/command/setup.py", "test_setup"),
    ("core installation", "Clone/release/PR source selection, GPU backend, Python resolution, Manager install, requirements, and restoration.", "comfy_cli/command/install.py", "test_install"),
    ("update routing", "Update core, CLI, or all; core performs git pull and requirement installation.", "comfy_cli/cmdline.py", "test_update"),
    ("foreground launch", "Run workspace main.py with selected Python; honor reboot marker until normal exit.", "comfy_cli/command/launch.py", "test_launch"),
    ("background launch", "Reject a live existing owner, validate port, capture owner-only logs, and wait for a readiness line.", "comfy_cli/command/launch.py", "test_launch_background"),
    ("background recovery", "Config load removes stale PID state and its log pointer; failed startup keeps the crash log path.", "comfy_cli/config_manager.py", "test_config_manager"),
    ("background stop", "Kill the process tree, report failure/success, and clear persisted ownership state.", "comfy_cli/cmdline.py", "test_command"),
    ("background logs", "Local-only bounded tail with missing/read/TOCTOU errors and raw pretty output.", "comfy_cli/command/launch.py", "test_logs"),
    ("native-oracle submission lifecycle", "Load/convert/validate/preview prompt, submit, stream progress, collect outputs, and classify terminal errors.", "comfy_cli/command/run/__init__.py", "test_run"),
    ("asynchronous submit", "Default run returns after submit, atomically writes job state, and detaches a watcher.", "comfy_cli/command/run/watcher.py", "test_run_execution_lifecycle"),
    ("detached watcher", "Poll every two seconds, persist state, never clear shared auth, and optionally notify.", "comfy_cli/command/job_watcher.py", "test_jobs"),
    ("watcher ceilings", "Terminate after six hours or after an unknown status stalls for 300 seconds.", "comfy_cli/command/job_watcher.py", "test_jobs"),
    ("multi-job wait", "Deduplicate IDs, poll until all terminal, emit settled events, and distinguish failed/cancelled/timed-out exits.", "comfy_cli/command/jobs.py", "test_jobs"),
    ("SIGINT cancellation", "Process-wide idempotent token closes registered resources, emits cancelled, and exits 130.", "comfy_cli/cancellation.py", "test_cancellation"),
    ("local job cancel", "Delete only the named pending prompt and call global interrupt only when that prompt is currently running.", "comfy_cli/command/jobs.py", "test_jobs"),
    ("cloud job cancel", "POST an encoded job id to the idempotent cloud cancellation endpoint.", "comfy_cli/command/jobs.py", "test_jobs"),
    ("HTTP transient retry", "Retry 429 for all methods, retry selected 5xx for GET only, honor Retry-After, then apply poll-level backoff.", "comfy_cli/comfy_client.py", "test_client"),
    ("OAuth refresh recovery", "Proactive/reactive refresh uses a cross-process lock, rotated-token persistence, replay guard, and fatal-family handling.", "comfy_cli/cloud/oauth.py", "test_oauth"),
    ("object_info recovery", "Cache successful fetches, force-refresh and retry once, then warn and serve stale cache or fail.", "comfy_cli/cql/loader.py", "test_loader_resilient"),
    ("update-check lifecycle", "At most one PyPI lookup per 24 hours; opt-out and cache corruption fail open.", "comfy_cli/update.py", "test_update"),
    ("template cache lifecycle", "Explicit gallery path outranks cache; refresh/miss fetches and replaces the XDG cache.", "comfy_cli/command/templates.py", "test_templates"),
    ("skill convergence", "Install updates current content, prunes retired skills, preserves modified/unmanaged state, and supports dry run/uninstall.", "comfy_cli/skills/__init__.py", "test_installer"),
    ("PR-cache lifecycle", "Validate age/metadata/dist, enforce seven-day and ten-entry limits, and clean selected/all entries.", "comfy_cli/pr_cache.py", "test_frontend_pr"),
]


EXTENSION_ROWS = [
    ("Python custom-node project metadata", "project and tool.comfy fields cover identity, version, dependencies, license, URLs, includes, models, OS/accelerator and Comfy versions.", "conflicting"),
    ("Dynamic Python package version", "Resolve project.dynamic version from attr/file with fail-closed regex and PEP constraints.", "conflicting"),
    ("Custom-node include/exclude", "Pack configured includes while applying .comfyignore and containment checks.", "conflicting"),
    ("Custom-node registry publish", "Validate, zip, request signed upload URL, upload bytes, and carry changelog.", "conflicting"),
    ("Custom-node registry install", "Resolve node/version metadata and download URL by legacy registry id.", "conflicting"),
    ("ComfyUI-Manager cm-cli bridge", "Resolve workspace Python and execute python -m cm_cli with session safety and dependency flags.", "conflicting"),
    ("Manager GUI mode", "Persist disable/enable-gui/disable-gui/enable-legacy-gui and translate to launch flags.", "conflicting"),
    ("Manager snapshots", "Save, restore, migrate legacy Manager, restore dependencies, and inspect workflow dependencies.", "conflicting"),
    ("Dependency modes", "Per-node pip, no-deps, fast-deps, or Manager uv-compile with mutual exclusion and explicit override precedence.", "conflicting"),
    ("object_info extension schema", "Node identifiers, categories, typed required/optional/hidden inputs, outputs, list/output flags, display names, descriptions, API-node flag, and Python module metadata.", "missing"),
    ("Legacy workflow identifiers", "Preserve class_type and socket strings through UI-to-API conversion, subgraph expansion, reroute/get-set/bypass tracing, defaults, and dynamic prompts.", "missing"),
    ("Frontend PR web override", "Build/cache a frontend PR and inject --front-end-root for a temporary web extension/UI.", "conflicting"),
    ("Bundled agent skills", "Five SKILL.md resources install into Claude, Cursor, and AGENTS.md hosts.", "deferred"),
    ("Third-party skill contract", "Path-based SKILL.md requires matching slug name and nonempty description frontmatter.", "deferred"),
    ("Partner endpoint adapters", "Schema-driven 52-endpoint allowlist plus explicit Gemini and Seedance request/response adapters.", "deferred"),
    ("Cloud custom-node policy", "Ten labels annotate 87 enumerated node packs; 83 packs declare version fields, 48 have non-empty registry versions, 38 names pin Git refs, and nine labels disable server-side capabilities on cloud.", "missing"),
    ("Legacy registry lacks explicit port contracts", "Registry dataclasses carry package and node identities, versions, dependencies, platform constraints, and URLs, but no versioned typed-port or sandbox-capability contract.", "missing"),
]


EXTENSION_TESTS = [
    ["tests/comfy_cli/registry/test_config_parser.py"],
    ["tests/comfy_cli/registry/test_config_parser.py"],
    ["tests/comfy_cli/command/nodes/test_pack.py"],
    ["tests/comfy_cli/command/nodes/test_publish.py"],
    ["tests/comfy_cli/registry/test_api.py", "tests/comfy_cli/command/nodes/test_node_install.py"],
    ["tests/comfy_cli/command/test_cm_cli_util.py", "tests/comfy_cli/test_cm_cli_python_resolution.py", "tests/comfy_cli/test_custom_nodes_python_resolution.py"],
    ["tests/comfy_cli/command/test_manager_gui.py"],
    ["tests/comfy_cli/command/nodes/test_node_install.py", "tests/comfy_cli/command/test_manager_gui.py"],
    ["tests/comfy_cli/command/test_cm_cli_util.py", "tests/comfy_cli/command/nodes/test_node_install.py", "tests/comfy_cli/test_install_python_resolution.py"],
    ["tests/comfy_cli/cql/test_loader.py", "tests/comfy_cli/command/test_nodes_introspect.py"],
    ["tests/comfy_cli/test_workflow_to_api.py", "tests/comfy_cli/command/test_workflow_fragments.py", "tests/comfy_cli/test_decompose.py"],
    ["tests/comfy_cli/command/test_frontend_pr.py", "tests/comfy_cli/command/test_launch_frontend_pr.py"],
    ["tests/comfy_cli/skills/test_installer.py"],
    ["tests/comfy_cli/skills/test_installer.py"],
    ["tests/comfy_cli/command/generate/test_spec.py", "tests/comfy_cli/command/generate/test_adapters.py", "tests/comfy_cli/command/generate/test_app.py"],
    [],
    [],
]


DOCUMENTATION_ROWS = [
    ("README command overview", "README.md", "Most listed install/launch/node/model/generate/manager behavior is corroborated by executable registrations.", "code-inferred", "active"),
    ("beta comfy-lock schema", "README.md", "Field-level comfy-lock.yaml schema is labelled WIP; code reads/writes comfy.lock.yaml but does not enforce the documented field schema or spelling.", "documented-only", "experimental"),
    ("development command authoring", "DEV_README.md", "Contributor process for adding Typer commands and tests.", "documented-only", "developer-only"),
    (".comfyignore packaging", "DEV_README.md", "Packaging rules are corroborated by custom-node pack code and tests.", "test-backed", "active"),
    ("uv-compile PRD", "docs/PRD-uv-compile.md", "Seven commands, tri-state override, standalone uv-sync, and Manager minimum are corroborated by code/tests.", "test-backed", "conditional"),
    ("uv-compile design", "docs/DESIGN-uv-compile.md", "Pass-through to cm-cli and config precedence are corroborated by code/tests.", "test-backed", "conditional"),
    ("E2E guide", "docs/TESTING-e2e.md", "Opt-in install/launch/node/model/run scenarios correspond to test code but were not run here.", "test-backed", "developer-only"),
    ("JSON/NDJSON contract", "docs/json-output.md", "Envelope semantics are code/test-backed; the document names converted/prompt_preview events omitted from run_event.json.", "test-backed", "active"),
    ("Keyframe Relay technique", "docs/superpowers/specs/2026-06-14-keyframe-relay-video-design.md", "A claimed built cloud pipeline and creative result has no fixture/artifact in this source snapshot; retain as documented-only.", "documented-only", "cloud/paid"),
    ("comfy driver skill", "comfy_cli/skills/comfy/SKILL.md", "Agent operating guidance; individual CLI commands are corroborated, creative/domain recommendations are documentation.", "documented-only", "developer-only"),
    ("fragment skill", "comfy_cli/skills/comfy-fragments/SKILL.md", "Fragment format/commands are corroborated; starter pattern recommendations are documentation.", "documented-only", "developer-only"),
    ("debug skill", "comfy_cli/skills/comfy-debug/SKILL.md", "Error-code decision guidance; codes are corroborated, operational advice is documentation.", "documented-only", "developer-only"),
    ("relay skill", "comfy_cli/skills/comfy-relay/SKILL.md", "Chat/media-presentation guidance with no runtime behavior.", "documented-only", "developer-only"),
    ("director skill", "comfy_cli/skills/comfy-director/SKILL.md", "Narrative/video-production guidance with no runtime behavior.", "documented-only", "developer-only"),
    ("bundled-skill count drift", "comfy_cli/skills/__init__.py", "Module prose says four bundled skills while BUNDLED_SKILLS and package data contain five; executable registry wins.", "code-inferred", "deprecated/dead"),
    ("orphan query help", "comfy_cli/help_json.py", "HELP_EXAMPLES and run-cli mention comfy query, but no command is registered.", "documented-only", "deprecated/dead"),
]


def table_rows(prefix: str, values: list[tuple[str, ...]], test_text: dict[str, str], kind: str) -> list[dict[str, Any]]:
    rows = []
    for value in values:
        name, behavior, path = value[:3]
        source = SOURCE_ROOT / path
        tests = FORMAT_TESTS.get(name, []) if kind == "format" else tests_containing(Path(path).stem, test_text)
        target_status = "missing"
        decision = "Implement natively in Rust and preserve the observable contract."
        if kind == "format" and any(term in name for term in ("custom-node", "Manager", "python.tgz", "comfy.lock")):
            target_status = "conflicting"
            decision = "Import only for migration/conformance; production writes a versioned native equivalent and executes no Python."
        rows.append(
            {
                "feature_id": stable_id(prefix, kind, name),
                "name": name,
                "behavior": behavior,
                "source_file": path,
                "evidence_level": "test-backed" if tests else "code-inferred",
                "tests": " | ".join(tests[:20]),
                "target_status": target_status,
                "parity_decision": decision,
            }
        )
    return rows


def extract_openapi() -> tuple[list[dict[str, Any]], dict[str, int]]:
    spec_py = PACKAGE_ROOT / "command/generate/spec.py"
    tree = ast.parse(spec_py.read_text(encoding="utf-8"))
    aliases: dict[str, str] = assignment_literal(tree, "_ALIASES") or {}
    allowlist: list[tuple[str, str, str | None]] = assignment_literal(tree, "_ENDPOINT_ALLOWLIST") or []
    preferred = {endpoint: alias for alias, endpoint in aliases.items()}
    openapi_path = PACKAGE_ROOT / "command/generate/spec/openapi.yml"
    text = openapi_path.read_text(encoding="utf-8")
    path_names = set(re.findall(r"^  (/[^:]+):\s*$", text, re.MULTILINE))
    rows = []
    for endpoint, category, poller in allowlist:
        full_path = "/proxy/" + endpoint
        if full_path not in path_names:
            raise RuntimeError(f"allowlisted partner endpoint missing from OpenAPI: {full_path}")
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-PARTNER", endpoint),
                "alias": preferred.get(endpoint, ""),
                "endpoint_id": endpoint,
                "path": full_path,
                "method": "POST",
                "category": category,
                "mode": "asynchronous" if poller else "synchronous",
                "poller": poller or "",
                "source_file": "comfy_cli/command/generate/spec.py",
                "openapi_source": "comfy_cli/command/generate/spec/openapi.yml",
                "evidence_level": "test-backed",
                "tests": "tests/comfy_cli/command/generate/test_spec.py | tests/comfy_cli/command/generate/test_app.py",
                "availability": "cloud/paid",
                "target_status": "deferred",
                "parity_decision": "Use an approved native service integration only; the endpoint is not part of the offline native execution core.",
            }
        )
    counts = {
        "openapi_lines": len(text.splitlines()),
        "openapi_paths": len(path_names),
        "openapi_operations": len(re.findall(r"^      operationId:", text, re.MULTILINE)),
        "openapi_excluded_operations": len(re.findall(r"^      x-excluded: true", text, re.MULTILINE)),
        "openapi_proxy_paths": sum(path.startswith("/proxy/") for path in path_names),
        "allowlist": len(allowlist),
        "aliases": len(aliases),
    }
    if counts != {
        "openapi_lines": 31636,
        "openapi_paths": 268,
        "openapi_operations": 289,
        "openapi_excluded_operations": 234,
        "openapi_proxy_paths": 193,
        "allowlist": 52,
        "aliases": 52,
    }:
        raise RuntimeError(f"unexpected OpenAPI reconciliation: {counts}")
    return rows, counts


def extract_cql_policy() -> tuple[list[dict[str, Any]], dict[str, int]]:
    path = PACKAGE_ROOT / "cql/data/supported_nodes.yaml"
    lines = path.read_text(encoding="utf-8").splitlines()
    labels = []
    packs: list[dict[str, Any]] = []
    in_labels = False
    current: dict[str, Any] | None = None
    current_node = ""
    for line_number, line in enumerate(lines, start=1):
        if line == "labels:":
            in_labels = True
            continue
        if line == "node_packs:":
            in_labels = False
            continue
        if in_labels:
            match = re.match(r"^  - ([A-Za-z0-9_]+)", line)
            if match:
                labels.append((match.group(1), line_number))
            continue
        pack_match = re.match(r"^  - name:\s*(.+?)\s*$", line)
        if pack_match:
            current = {"name": pack_match.group(1).strip('"'), "line": line_number, "version": "", "nodes": []}
            packs.append(current)
            current_node = ""
            continue
        if current is None:
            continue
        version_match = re.match(r'^    version:\s*("[^"]*"|[^#]*?)(?:\s+#.*)?$', line)
        if version_match:
            current["has_version"] = True
            current["version"] = version_match.group(1).strip().strip('"')
            continue
        node_match = re.match(r"^      (.+):\s*$", line)
        if node_match:
            current_node = node_match.group(1).strip().strip('"')
            current["nodes"].append({"name": current_node, "line": line_number, "labels": []})
            continue
        label_match = re.match(r"^        - ([A-Za-z0-9_]+)", line)
        if label_match and current_node and current["nodes"]:
            current["nodes"][-1]["labels"].append(label_match.group(1))

    disable_text = (PACKAGE_ROOT / "cql/data/cloud_disable_config.yaml").read_text(encoding="utf-8")
    disabled = set(re.findall(r"^    - ([A-Za-z0-9_]+): true", disable_text, re.MULTILINE))
    no_gpu = json.loads((PACKAGE_ROOT / "cql/data/no_gpu_nodes.json").read_text(encoding="utf-8"))["no_gpu_nodes"]
    rows = []
    for label, line in labels:
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-CQL", "label", label),
                "row_kind": "label",
                "pack": "",
                "version": "",
                "node_identifier": "",
                "labels": label,
                "cloud_disabled": label in disabled,
                "source_file": "comfy_cli/cql/data/supported_nodes.yaml",
                "line": line,
                "evidence_level": "code-inferred",
                "target_status": "missing",
                "parity_decision": "Map this capability label into the native Rust/WASM permission manifest.",
            }
        )
    for pack in packs:
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-CQL", "node pack", pack["name"]),
                "row_kind": "node pack",
                "pack": pack["name"],
                "version": pack["version"],
                "node_identifier": "",
                "labels": "",
                "cloud_disabled": False,
                "source_file": "comfy_cli/cql/data/supported_nodes.yaml",
                "line": pack["line"],
                "evidence_level": "code-inferred",
                "target_status": "conflicting",
                "parity_decision": "Treat as a legacy compatibility/migration identity; execute only an approved Rust/WASM plugin mapping.",
            }
        )
        for node in pack["nodes"]:
            rows.append(
                {
                    "feature_id": stable_id("COMFY-CLI-CQL", "node policy", pack["name"], node["name"]),
                    "row_kind": "node policy",
                    "pack": pack["name"],
                    "version": pack["version"],
                    "node_identifier": node["name"],
                    "labels": " | ".join(node["labels"]),
                    "cloud_disabled": any(label in disabled for label in node["labels"]),
                    "source_file": "comfy_cli/cql/data/supported_nodes.yaml",
                    "line": node["line"],
                    "evidence_level": "code-inferred",
                    "target_status": "conflicting",
                    "parity_decision": "Preserve the legacy identifier in a versioned mapping; native plugin permissions are explicit and deny undeclared ports/capabilities.",
                }
            )
    counts = {
        "labels": len(labels),
        "node_packs": len(packs),
        "node_label_entries": sum(len(pack["nodes"]) for pack in packs),
        "label_assignments": sum(len(node["labels"]) for pack in packs for node in pack["nodes"]),
        "version_declarations": sum(bool(pack.get("has_version")) for pack in packs),
        "version_pinned_packs": sum(bool(pack["version"]) for pack in packs),
        "git_ref_packs": sum("@" in pack["name"] for pack in packs),
        "cloud_disabling_labels": len(disabled),
        "no_gpu_nodes": len(no_gpu),
    }
    expected = {
        "labels": 10,
        "node_packs": 87,
        "node_label_entries": 322,
        "label_assignments": 432,
        "version_declarations": 83,
        "version_pinned_packs": 48,
        "git_ref_packs": 38,
        "cloud_disabling_labels": 9,
        "no_gpu_nodes": 0,
    }
    if counts != expected:
        raise RuntimeError(f"unexpected CQL reconciliation: {counts}")
    return rows, counts


def extract_tests() -> tuple[list[dict[str, Any]], dict[str, list[str]], dict[str, str]]:
    cases = []
    per_file: dict[str, list[str]] = defaultdict(list)
    test_text = {relative(path): path.read_text(encoding="utf-8", errors="replace") for path in sorted(TEST_ROOT.rglob("*.py"))}
    raw = []
    for path_key, text in test_text.items():
        tree = ast.parse(text, filename=path_key)
        class_stack: list[str] = []

        def visit(body: list[ast.stmt], parents: list[str]) -> None:
            for node in body:
                if isinstance(node, ast.ClassDef):
                    visit(node.body, parents + [node.name])
                elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if node.name.startswith("test_"):
                        symbol = ".".join(parents + [node.name])
                        raw.append((path_key, node.lineno, symbol))
                    visit(node.body, parents + [node.name])

        visit(tree.body, class_stack)
    raw.sort(key=lambda row: (row[0], row[1], row[2]))
    for path_key, line, symbol in raw:
        feature_id = stable_id("COMFY-CLI-TEST", path_key, symbol)
        per_file[path_key].append(feature_id)
        cases.append(
            {
                "feature_id": feature_id,
                "source_file": path_key,
                "symbol": symbol,
                "line": line,
                "classification": "existing test case",
                "availability": "infrastructure-only",
                "evidence_level": "test-backed",
                "execution_status": "not run: Python 3.9.6 and required dependencies are unavailable",
                "target_status": "deferred",
                "parity_decision": "Port or replace with deterministic native unit/protocol/GPUI conformance coverage where the source behavior is selected.",
            }
        )
    if len(cases) != 2295:
        raise RuntimeError(f"expected 2295 test functions, found {len(cases)}")
    return cases, per_file, test_text


def extract_modules(command_rows: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], dict[str, str]]:
    command_ids: dict[str, list[str]] = defaultdict(list)
    for row in command_rows:
        command_ids[row["source_file"]].append(row["feature_id"])
    rows = []
    module_ids = {}
    for path in sorted(PACKAGE_ROOT.rglob("*.py"), key=lambda item: relative(item)):
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        path_key = relative(path)
        feature_id = stable_id("COMFY-CLI-MODULE", path_key)
        module_ids[path_key] = feature_id
        public_functions = [node.name for node in tree.body if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and not node.name.startswith("_")]
        classes = [node.name for node in tree.body if isinstance(node, ast.ClassDef)]
        if "/command/" in f"/{path_key}" or path_key.endswith("cmdline.py"):
            domain = "command surface"
        elif "/output/" in f"/{path_key}":
            domain = "output and presentation"
        elif "/cloud/" in f"/{path_key}" or "/auth/" in f"/{path_key}":
            domain = "cloud and authentication"
        elif "/cql/" in f"/{path_key}":
            domain = "graph schema and query"
        elif "/registry/" in f"/{path_key}":
            domain = "custom-node registry"
        elif "/skills/" in f"/{path_key}":
            domain = "agent skill integration"
        else:
            domain = "core service"
        rows.append(
            {
                "feature_id": feature_id,
                "source_file": path_key,
                "module": module_name(path),
                "domain": domain,
                "public_functions": " | ".join(public_functions),
                "classes": " | ".join(classes),
                "function_count": sum(isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) for node in ast.walk(tree)),
                "class_count": sum(isinstance(node, ast.ClassDef) for node in ast.walk(tree)),
                "command_ids": " | ".join(command_ids.get(path_key, [])),
                "classification": "production module",
                "evidence_level": "code-inferred",
            }
        )
    if len(rows) != 104:
        raise RuntimeError(f"expected 104 production Python modules, found {len(rows)}")
    return rows, module_ids


def build_lifecycle(test_text: dict[str, str]) -> list[dict[str, Any]]:
    rows = []
    for name, behavior, source_file, test_token in LIFECYCLE_ROWS:
        tests = sorted(path for path in test_text if test_token in Path(path).stem or test_token in test_text[path])
        python_conflict = name in {
            "core installation",
            "update routing",
            "foreground launch",
            "background launch",
            "background recovery",
            "background stop",
            "background logs",
        }
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-LIFECYCLE", name),
                "name": name,
                "behavior": behavior,
                "source_file": source_file,
                "evidence_level": "test-backed" if tests else "code-inferred",
                "tests": " | ".join(tests[:30]),
                "availability": "conditional" if python_conflict else "active",
                "target_status": "conflicting" if python_conflict else "missing",
                "parity_decision": "Recreate lifecycle semantics with native Rust services and child-free recovery; Python process ownership is forbidden."
                if python_conflict
                else "Implement directly in the native execution/service state machine.",
            }
        )
    return rows


EXTENSION_SOURCES = [
    "comfy_cli/registry/config_parser.py",
    "comfy_cli/registry/config_parser.py",
    "comfy_cli/command/custom_nodes/command.py",
    "comfy_cli/command/custom_nodes/command.py",
    "comfy_cli/registry/api.py",
    "comfy_cli/command/custom_nodes/cm_cli_util.py",
    "comfy_cli/command/custom_nodes/cm_cli_util.py",
    "comfy_cli/command/custom_nodes/command.py",
    "comfy_cli/command/custom_nodes/command.py",
    "comfy_cli/cql/engine.py",
    "comfy_cli/workflow_to_api.py",
    "comfy_cli/command/install.py",
    "comfy_cli/skills/__init__.py",
    "comfy_cli/skills/__init__.py",
    "comfy_cli/command/generate/spec.py",
    "comfy_cli/cql/data/supported_nodes.yaml",
    "comfy_cli/registry/types.py",
]


def build_extensions(test_text: dict[str, str]) -> list[dict[str, Any]]:
    rows = []
    for (name, behavior, target_status), source_file, tests in zip(EXTENSION_ROWS, EXTENSION_SOURCES, EXTENSION_TESTS):
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-EXT", name),
                "name": name,
                "source_contract": behavior,
                "source_file": source_file,
                "availability": "cloud/paid" if "Partner" in name else "conditional",
                "evidence_level": "test-backed" if tests else "code-inferred",
                "tests": " | ".join(tests[:30]),
                "target_status": target_status,
                "native_decision": "Replace executable Python/JavaScript hooks with a versioned Rust/WASM manifest, explicit typed ports, capability permissions, and legacy identifier mappings."
                if target_status == "conflicting"
                else "Implement as a versioned native contract; no Python or JavaScript execution in production.",
            }
        )
    return rows


def build_documentation() -> list[dict[str, Any]]:
    rows = []
    for name, source_file, claim, evidence, availability in DOCUMENTATION_ROWS:
        rows.append(
            {
                "feature_id": stable_id("COMFY-CLI-DOC", name, source_file),
                "name": name,
                "claim": claim,
                "source_file": source_file,
                "evidence_level": evidence,
                "availability": availability,
                "corroboration": "Executable source/test referenced in the claim." if evidence != "documented-only" else "No executable corroboration for this independent claim.",
                "target_status": "deferred" if availability in {"developer-only", "cloud/paid"} else "missing",
                "parity_decision": "Do not elevate this documentation claim to a native compatibility promise without executable-or-oracle conformance evidence."
                if evidence == "documented-only"
                else "Use executable behavior as authority and retain this document as supporting context only.",
            }
        )
    return rows


def source_classification(path_key: str) -> tuple[str, str]:
    if path_key.startswith("comfy_cli/"):
        return "production", "Executable package module or packaged runtime data."
    if path_key.startswith("tests/"):
        return "test-only/support", "Existing test, fixture, generated coverage data, or isolated package fixture."
    if path_key.startswith("docs/") or path_key in {"README.md", "DEV_README.md"}:
        return "documentation", "Documentation is supporting evidence only unless separately corroborated."
    if path_key.startswith(".github/"):
        return "infrastructure-only", "Repository automation, issue metadata, or CI configuration."
    if path_key.startswith("assets/"):
        return "asset", "Demonstration media; no independent executable behavior is inferred."
    return "infrastructure-only", "Packaging, dependency lock, license, lint, coverage, or repository metadata."


def build_source_coverage(
    files: list[Path],
    catalogs: list[list[dict[str, Any]]],
    test_ids: dict[str, list[str]],
    module_ids: dict[str, str],
) -> list[dict[str, Any]]:
    mappings: dict[str, set[str]] = defaultdict(set)
    for rows in catalogs:
        for row in rows:
            feature_id = row.get("feature_id")
            source_file = row.get("source_file")
            if feature_id and source_file:
                mappings[str(source_file)].add(str(feature_id))
            openapi_source = row.get("openapi_source")
            if feature_id and openapi_source:
                mappings[str(openapi_source)].add(str(feature_id))
    for path_key, ids in test_ids.items():
        mappings[path_key].update(ids)
    for path_key, feature_id in module_ids.items():
        mappings[path_key].add(feature_id)
    # All CQL policy rows are jointly informed by the allow/disable/no-GPU data trio.
    cql_ids = {feature_id for feature_id in mappings["comfy_cli/cql/data/supported_nodes.yaml"]}
    mappings["comfy_cli/cql/data/cloud_disable_config.yaml"].update(cql_ids)
    mappings["comfy_cli/cql/data/no_gpu_nodes.json"].update(cql_ids)
    # The prose error-code contract documents the executable typed registry.
    mappings["comfy_cli/schemas/error_codes.md"].update(mappings["comfy_cli/error_codes.py"])

    rows = []
    for path in files:
        path_key = relative(path)
        classification, reason = source_classification(path_key)
        ids = sorted(mappings.get(path_key, set()))
        rows.append(
            {
                "source_id": stable_id("COMFY-CLI-SOURCE", path_key),
                "path": path_key,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "bytes": path.stat().st_size,
                "classification": classification,
                "feature_ids": " | ".join(ids),
                "disposition": "mapped" if ids else classification,
                "reason": reason if not ids else "Mapped to the listed stable feature/module/test identifiers.",
            }
        )
    if len(rows) != EXPECTED_FILE_COUNT:
        raise RuntimeError(f"expected {EXPECTED_FILE_COUNT} source rows, found {len(rows)}")
    production_unmapped = [row["path"] for row in rows if row["classification"] == "production" and not row["feature_ids"]]
    if production_unmapped:
        raise RuntimeError(f"unmapped production files: {production_unmapped}")
    return rows


def catalog_hashes(names: list[str]) -> dict[str, str]:
    return {name: hashlib.sha256((CATALOG_DIR / name).read_bytes()).hexdigest() for name in names}


def write_evidence(summary: dict[str, Any], shadowed: dict[str, str]) -> None:
    status_counts = summary["command_target_status"]
    text = f"""# Comfy CLI parity evidence

## Audit status

This report records the static and existing-test evidence gathered from `projects/comfy/comfy-cli` for the native Rust/GPUI parity design. Comfy CLI is evidence and a development-time conformance client; it does not authorize a production Python dependency. Production Zed must implement execution and lifecycle natively and may accept legacy Python-oriented formats only for migration or compatibility translation into versioned Rust/WASM plugins with explicit ports.

No nested `AGENTS.md` or nested Git metadata exists in this source root. README, design, skill, and guide claims are never promoted above `documented-only` without executable or test corroboration.

## Source baseline

| Property | Evidence-backed value |
|---|---|
| Source root | `projects/comfy/comfy-cli` |
| Git identity | No nested `.git`; no commit SHA asserted. |
| Manifest version | `0.0.0`, explicitly a CI release-time placeholder, not a release version. |
| Required Python | `>=3.10` |
| Source-tree files | {summary['files']} |
| Deterministic fingerprint | SHA-256 `{summary['fingerprint']}` |
| Fingerprint recipe | Sort included relative paths bytewise, hash each file, then hash lines of `<digest>  ./<relative-path>`. Excludes `.git`, `node_modules`, `__pycache__`, `*.pyc`, and `.DS_Store`. |

The 312-file closure is: 137 packaged runtime files, 141 tests/fixtures, 5 `docs/` files, 14 `.github/` files, 2 demonstration assets, and 13 root packaging/metadata files. Packaged runtime contents are 104 Python modules, 24 JSON files, 6 Markdown resources, 2 YAML registries, and one OpenAPI YAML document.

## Runtime constraints

The available interpreter is Python 3.9.6, below the declared minimum. Typer, questionary, PyYAML, pytest, and other dependencies are absent. `PYTHONPATH=projects/comfy/comfy-cli python3 -m comfy_cli --help` reaches the entry point but fails on `ModuleNotFoundError: questionary`. No dependency or network mutation was authorized, so no command behavior is labelled observed and no existing test was run. Static syntax validation parsed all 228 Python files (104 production plus 124 tests) without error. Thirty of 31 `.json` files parse as strict JSON; `pyrightconfig.json` is JSON-with-a-trailing-comma configuration, not a runtime schema.

## Registry reconciliation

| Surface | Source | Catalog | Result |
|---|---:|---:|---|
| Reachable leaf command paths | 123 | {summary['commands']} | Match; 41 top-level names. |
| Typer app objects | 20 | 20 | Match. |
| `@command` registrations / unique functions | 113 / 112 | represented by path | Duplicate is stacked `dependency` decorators. |
| Root/global options | 11 | {summary['global_parameters']} | Match. |
| Command-path parameter bindings, including aliases and fixed `generate` grammar | 370 | {summary['parameters']} | Match with zero unresolved rows; every row has typed arity/cardinality evidence, while {summary['parameter_contracts']['constraint_rows']} retain non-empty explicit parser constraints, {summary['parameter_contracts']['enum_choice_rows']} retain exact Enum choices, and {summary['parameter_contracts']['paired_boolean_rows']} retain paired boolean spellings. Schema-derived per-partner fields live in the 52 endpoint schemas and are not falsely counted as Typer flags. |
| JSON schemas | 23 | {summary['schemas']} | Match. |
| Command-schema mappings | 64 | {summary['schema_mappings_envelope']} | 63 reachable plus orphan `comfy version`. |
| Stream-schema mappings | 2 | {summary['schema_mappings_stream']} | Match. |
| Error codes | 99 | {summary['errors']} | Unique and bidirectionally ratcheted by tests. |
| Versioned event union | 12 | {summary['events']} | Four code/schema mismatches are explicit. |
| Production environment variables | 35 | {summary['environment']} | Match. |
| Persisted config keys | 20 | {summary['config']} | Match. |
| Persisted/interchange formats | 34 | {summary['formats']} | Match. |
| Lifecycle contracts | 24 | {summary['lifecycle']} | Match. |
| Extension contracts | 17 | {summary['extensions']} | Match. |
| Partner allowlist / aliases | 52 / 52 | {summary['partner_endpoints']} | Every allowlisted path exists in the vendored OpenAPI. |
| OpenAPI paths / operations / excluded operations / proxy paths | 268 / 289 / 234 / 193 | reconciled metadata | Match. |
| CQL labels / packs / node-label entries / assignments | 10 / 87 / 322 / 432 | {summary['cql_rows']} policy rows | Match; 83 packs declare a version field, 48 contain a non-empty registry pin, and 38 pack names contain a git-ref pin. |
| Test functions / classes / fixtures | 2,295 / 316 / 129 | {summary['tests']} function rows | Functions match; tests inspected, not run. |
| Production Python modules | 104 | {summary['modules']} | Match. |
| Source files | 312 | {summary['source_rows']} | Every production file maps to stable IDs; every other file has an explicit disposition. |

## Capability disposition

The behavioral capability catalogs contain {summary['capability_features']:,} stable records, alongside {summary['modules']} production module/service contracts, {summary['tests']:,} test-function records, {summary['source_rows']} source-file rows, and {summary['schema_mappings_envelope'] + summary['schema_mappings_stream']} schema-mapping relationships. Their evidence split is {summary['capability_evidence']['test-backed']:,} test-backed, {summary['capability_evidence']['code-inferred']:,} code-inferred, {summary['capability_evidence']['documented-only']:,} documented-only, {summary['capability_evidence']['observed']:,} observed, and {summary['capability_evidence']['unverified']:,} unverified. The master ledger promotes both behavioral and production module/service records so every production source row closes against a master feature ID. Test-backed means an existing test explicitly exercises the contract; it does not imply that the test ran in this audit.

Source-audit native-target dispositions are {summary['capability_target_status']['missing']:,} missing, {summary['capability_target_status']['conflicting']:,} conflicting, {summary['capability_target_status']['deferred']:,} deferred, {summary['capability_target_status']['equivalent']:,} equivalent, {summary['capability_target_status']['partial']:,} partial, and {summary['capability_target_status']['uncertain']:,} uncertain. The master generator synchronizes target-only columns against independent Zed evidence and the fixed native-only architecture before producing the pack-wide parity matrix.

## Command and machine-contract findings

The reachable tree contains 123 leaves and 41 top-level names. It includes local execution/jobs, workflow conversion/editing/fragments, node and model introspection, project assets, templates, previews, custom-node/Manager compatibility, agent skills, cloud OAuth/jobs/workflows, partner generation, and hidden/developer aliases.

Three source orphans are retained rather than normalized away:

"""
    for name, explanation in shadowed.items():
        text += f"- `{name}` — {explanation}\n"
    text += f"""

`COMMAND_SCHEMAS` contains 64 entries, but only 63 target reachable paths; `comfy version` is not registered. Sixty reachable leaves have no command-schema mapping. This is not interpreted as absence of behavior: many legacy/interactive commands simply have not migrated to the structured envelope registry.

The event contract has a concrete versioning conflict. `run_event.json` declares eight event names. Executable code additionally emits `converted`, `prompt_preview`, `settled`, and `state`; the first two are also described in `docs/json-output.md`. Native Zed must define one authoritative event union and validate every emitted line against it.

## Typed parameter contracts

The parameter ledger retains 355 distinct bindings plus 15 alias-path repetitions. Of its {summary['parameter_contracts']['rows']} rows, {summary['parameter_contracts']['source_annotation_rows']} row bindings derive from statically parsed Python annotations and {summary['parameter_contracts']['dynamic_parser_rows']} derive from the explicit `generate` tail parser branches. Value types are {json.dumps(summary['parameter_contracts']['value_types'], sort_keys=True)}; {summary['parameter_contracts']['nullable_rows']} rows are nullable, {summary['parameter_contracts']['repeatable_rows']} accept repeated or variadic values, {summary['parameter_contracts']['enum_choice_rows']} have exact statically resolved Enum choices, {summary['parameter_contracts']['constraint_rows']} retain explicit callback/autocompletion/metavar/input constraints, and {summary['parameter_contracts']['paired_boolean_rows']} expose paired boolean spellings.

This is static contract evidence, not observed Typer behavior. Callback and autocompletion expressions are retained by source name but are not executed; prose-only examples and help-text suggestions are not promoted into enforced choices; only Enum declarations become exact `choices`. The ten dynamic `generate` rows are typed from the parser's explicit boolean, numeric, string, path, and default branches. Schema-derived partner fields remain in the partner endpoint schemas instead of being misrepresented as fixed Typer bindings.

## Native architecture consequences

Command status counts are missing {status_counts.get('missing', 0)}, conflicting {status_counts.get('conflicting', 0)}, and deferred {status_counts.get('deferred', 0)}. `conflicting` commands are Python/ComfyUI-Manager process operations that cannot be copied into production. Their observable intent becomes native installation/update/runtime/plugin behavior, or a legacy import/migration surface. Cloud, partner-generation, telemetry feedback, and code-search mutations remain explicit deferred service contracts rather than disappearing.

The source defines 99 error codes, envelope/1 and event/1 machine protocols, UI/API workflow conversion, object_info schema behavior, queue/history/jobs/cancellation, local/cloud routing, durable job recovery, 34 persisted/interchange formats, and 17 extension contracts. These are high-value conformance inputs for a native Rust core and compatibility server/CLI.

Python custom-node packaging and cm-cli execution are architectural conflicts. Native parity requires:

- a versioned Rust/WASM plugin manifest;
- explicit typed input/output ports and list/lazy/output-node semantics;
- capability permissions for files, network, state, custom routes, and large outputs;
- deterministic resource/memory/cancellation boundaries;
- legacy `class_type`, socket, pack, and registry identifiers mapped to native plugin/version identifiers;
- import diagnostics for unmapped Python nodes without executing their code.

## Tests and source coverage

The test catalog contains 2,295 `test_*` functions in 124 Python files, 316 `Test*` classes, and 129 fixtures. Opt-in E2E suites cover real installation, launch, model operations, custom-node lifecycle, execution, GPU, unified dependency resolution, conflict attribution, and telemetry delivery. They were not run because the environment lacks the declared runtime/dependencies and real E2E paths would clone/download/mutate external state.

Source coverage contains one deterministic row for every vendored file. No production file is unmapped. Documentation, tests, CI, assets, manifests, and locks have explicit non-production dispositions. The machine catalogs preserve documentation-only and dead/orphan claims instead of treating them as executable evidence.

## Generated catalogs

The authoritative files are `catalogs/comfy-cli-*.csv` and `catalogs/comfy-cli-reconciliation.json`, regenerated by `generate_comfy_cli_catalogs.py`. Catalog hashes are recorded in the reconciliation JSON generated after the rows are written.
"""
    (SPEC_DIR / "evidence-comfy-cli.md").write_text(text, encoding="utf-8")


def test_structure_counts() -> dict[str, int]:
    classes = fixtures = 0
    for path in TEST_ROOT.rglob("*.py"):
        tree = ast.parse(path.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef) and node.name.startswith("Test"):
                classes += 1
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                for decorator in node.decorator_list:
                    target = decorator.func if isinstance(decorator, ast.Call) else decorator
                    if dotted(target) in {"fixture", "pytest.fixture"}:
                        fixtures += 1
    return {"test_classes": classes, "fixtures": fixtures}


def main() -> None:
    verify_stable_id_regressions()
    CATALOG_DIR.mkdir(parents=True, exist_ok=True)
    files = source_files()
    fingerprint = tree_fingerprint(files)
    if len(files) != EXPECTED_FILE_COUNT or fingerprint != EXPECTED_FINGERPRINT:
        raise RuntimeError(f"baseline drift: files={len(files)}, fingerprint={fingerprint}")

    test_rows, test_ids, test_text = extract_tests()
    command_rows, parameter_rows, shadowed = extract_commands(test_text)
    command_targets = {
        row["feature_id"]: (row["target_status"], row["parity_decision"])
        for row in command_rows
    }
    for row in parameter_rows:
        row["feature_id"] = stable_id(
            "COMFY-CLI-PARAM",
            row["command_path"],
            row["scope"],
            row["name"],
            row["kind"],
        )
        target_status, parity_decision = command_targets.get(
            row["command_id"],
            ("missing", "Implement the global option in the native Rust CLI and GPUI settings/action surfaces."),
        )
        row["target_status"] = target_status
        row["parity_decision"] = parity_decision
    required_parameter_fields = {
        "annotation",
        "value_type",
        "nullable",
        "value_arity",
        "cardinality",
        "repeatable",
        "choices",
        "constraints",
        "boolean_forms",
        "default_source",
        "type_evidence",
    }
    unresolved_parameter_rows = [
        row["feature_id"]
        for row in parameter_rows
        if any(field not in row or row[field] is None for field in required_parameter_fields)
        or not row["annotation"]
        or not row["value_type"]
        or not row["value_arity"]
        or not row["cardinality"]
        or not row["type_evidence"]
    ]
    if len(parameter_rows) != 370 or unresolved_parameter_rows:
        raise RuntimeError(
            f"CLI parameter contract closure failed: rows={len(parameter_rows)}, unresolved={unresolved_parameter_rows}"
        )
    parameter_contracts = {
        "rows": len(parameter_rows),
        "resolved": len(parameter_rows) - len(unresolved_parameter_rows),
        "unresolved": len(unresolved_parameter_rows),
        "source_annotation_rows": sum(
            str(row["type_evidence"]).startswith("static source annotation") for row in parameter_rows
        ),
        "dynamic_parser_rows": sum(row["type_evidence"] == "static dynamic-parser branches" for row in parameter_rows),
        "value_types": dict(sorted(Counter(row["value_type"] for row in parameter_rows).items())),
        "nullable_rows": sum(bool(row["nullable"]) for row in parameter_rows),
        "repeatable_rows": sum(bool(row["repeatable"]) for row in parameter_rows),
        "enum_choice_rows": sum(bool(row["choices"]) for row in parameter_rows),
        "constraint_rows": sum(bool(row["constraints"]) for row in parameter_rows),
        "paired_boolean_rows": sum(bool(row["boolean_forms"]) for row in parameter_rows),
    }
    expected_parameter_contracts = {
        "rows": 370,
        "resolved": 370,
        "unresolved": 0,
        "source_annotation_rows": 360,
        "dynamic_parser_rows": 10,
        "value_types": {
            "boolean": 82,
            "enum": 6,
            "integer": 41,
            "number": 6,
            "path": 7,
            "string": 228,
        },
        "nullable_rows": 200,
        "repeatable_rows": 22,
        "enum_choice_rows": 6,
        "constraint_rows": 55,
        "paired_boolean_rows": 15,
    }
    if parameter_contracts != expected_parameter_contracts:
        raise RuntimeError(
            "CLI parameter contract counts changed without a reviewed extraction update: "
            f"{parameter_contracts}"
        )
    schema_rows, schema_mapping_rows, command_mappings, stream_mappings = extract_schemas()
    error_rows = extract_error_codes()
    event_rows = extract_events()
    environment_rows = keyed_rows("COMFY-CLI-ENV", ENVIRONMENT_ROWS, test_text, "missing")
    config_rows = keyed_rows("COMFY-CLI-CONFIG", CONFIG_ROWS, test_text, "missing")
    format_rows = table_rows("COMFY-CLI-FORMAT", FORMAT_ROWS, test_text, "format")
    lifecycle_rows = build_lifecycle(test_text)
    extension_rows = build_extensions(test_text)
    documentation_rows = build_documentation()
    partner_rows, openapi_counts = extract_openapi()
    cql_rows, cql_counts = extract_cql_policy()
    module_rows, module_ids = extract_modules(command_rows)

    capability_catalog_rows = [
        command_rows,
        parameter_rows,
        schema_rows,
        error_rows,
        event_rows,
        environment_rows,
        config_rows,
        format_rows,
        lifecycle_rows,
        extension_rows,
        documentation_rows,
        partner_rows,
        cql_rows,
    ]
    all_catalog_rows = [
        *capability_catalog_rows,
        test_rows,
        module_rows,
    ]
    source_rows = build_source_coverage(files, all_catalog_rows, test_ids, module_ids)

    output_specs = [
        ("comfy-cli-commands.csv", ["feature_id", "path", "top_level", "hidden", "classification", "availability", "evidence_level", "confidence", "help", "source_file", "symbol", "line", "registration", "tests", "target_status", "parity_decision", "notes"], command_rows),
        ("comfy-cli-parameters.csv", ["feature_id", "command_id", "command_path", "scope", "name", "kind", "flags", "annotation", "value_type", "nullable", "value_arity", "cardinality", "repeatable", "choices", "constraints", "boolean_forms", "default", "default_source", "required", "hidden", "envvar", "help", "source_file", "line", "type_evidence", "evidence_level", "target_status", "parity_decision"], parameter_rows),
        ("comfy-cli-schemas.csv", ["feature_id", "name", "schema_id", "title", "draft", "type", "required", "top_level_properties", "source_file", "evidence_level", "tests", "target_status", "parity_decision"], schema_rows),
        ("comfy-cli-schema-mappings.csv", ["mapping_kind", "command_path", "schema", "reachable", "source_file", "evidence_level", "notes"], schema_mapping_rows),
        ("comfy-cli-errors.csv", ["feature_id", "code", "meaning", "hint", "source_file", "line", "evidence_level", "tests", "target_status", "parity_decision"], error_rows),
        ("comfy-cli-events.csv", ["feature_id", "event", "in_run_event_schema", "literal_code_emission", "source_sites", "evidence_level", "contract_status", "target_status", "parity_decision", "notes"], event_rows),
        ("comfy-cli-environment.csv", ["feature_id", "key", "behavior", "classification", "source_file", "line", "evidence_level", "tests", "target_status", "parity_decision"], environment_rows),
        ("comfy-cli-config.csv", ["feature_id", "key", "behavior", "classification", "source_file", "line", "evidence_level", "tests", "target_status", "parity_decision"], config_rows),
        ("comfy-cli-formats.csv", ["feature_id", "name", "behavior", "source_file", "evidence_level", "tests", "target_status", "parity_decision"], format_rows),
        ("comfy-cli-lifecycle.csv", ["feature_id", "name", "behavior", "source_file", "evidence_level", "tests", "availability", "target_status", "parity_decision"], lifecycle_rows),
        ("comfy-cli-extensions.csv", ["feature_id", "name", "source_contract", "source_file", "availability", "evidence_level", "tests", "target_status", "native_decision"], extension_rows),
        ("comfy-cli-documentation.csv", ["feature_id", "name", "claim", "source_file", "evidence_level", "availability", "corroboration", "target_status", "parity_decision"], documentation_rows),
        ("comfy-cli-partner-openapi.csv", ["feature_id", "alias", "endpoint_id", "path", "method", "category", "mode", "poller", "source_file", "openapi_source", "evidence_level", "tests", "availability", "target_status", "parity_decision"], partner_rows),
        ("comfy-cli-cql-policy.csv", ["feature_id", "row_kind", "pack", "version", "node_identifier", "labels", "cloud_disabled", "source_file", "line", "evidence_level", "target_status", "parity_decision"], cql_rows),
        ("comfy-cli-tests.csv", ["feature_id", "source_file", "symbol", "line", "classification", "availability", "evidence_level", "execution_status", "target_status", "parity_decision"], test_rows),
        ("comfy-cli-modules.csv", ["feature_id", "source_file", "module", "domain", "public_functions", "classes", "function_count", "class_count", "command_ids", "classification", "evidence_level"], module_rows),
        ("comfy-cli-source-coverage.csv", ["source_id", "path", "sha256", "bytes", "classification", "feature_ids", "disposition", "reason"], source_rows),
    ]
    for name, fields, rows in output_specs:
        write_csv(name, fields, rows)

    status_counts = Counter(row["target_status"] for row in command_rows)
    evidence_counts = Counter(
        row.get("evidence_level", "") for rows in all_catalog_rows for row in rows if row.get("evidence_level")
    )
    capability_evidence_counts = Counter(
        row.get("evidence_level", "")
        for rows in capability_catalog_rows
        for row in rows
        if row.get("evidence_level")
    )
    capability_target_counts = Counter(
        row.get("target_status", "")
        for rows in capability_catalog_rows
        for row in rows
        if row.get("target_status")
    )
    source_counts = Counter(row["classification"] for row in source_rows)
    test_counts = test_structure_counts()
    summary = {
        "source_root": "projects/comfy/comfy-cli",
        "manifest_version": "0.0.0 (CI placeholder)",
        "files": len(files),
        "fingerprint": fingerprint,
        "commands": len(command_rows),
        "top_level_commands": len({row["top_level"] for row in command_rows}),
        "parameters": len(parameter_rows),
        "global_parameters": sum(row["scope"] == "global" for row in parameter_rows),
        "parameter_contracts": parameter_contracts,
        "schemas": len(schema_rows),
        "schema_mappings_envelope": len(command_mappings),
        "schema_mappings_stream": len(stream_mappings),
        "errors": len(error_rows),
        "events": len(event_rows),
        "environment": len(environment_rows),
        "config": len(config_rows),
        "formats": len(format_rows),
        "lifecycle": len(lifecycle_rows),
        "extensions": len(extension_rows),
        "documentation_claims": len(documentation_rows),
        "partner_endpoints": len(partner_rows),
        "cql_rows": len(cql_rows),
        "tests": len(test_rows),
        "modules": len(module_rows),
        "source_rows": len(source_rows),
        "capability_features": sum(len(rows) for rows in capability_catalog_rows),
        "capability_evidence": {
            level: capability_evidence_counts[level]
            for level in ("observed", "test-backed", "code-inferred", "documented-only", "unverified")
        },
        "capability_target_status": {
            status: capability_target_counts[status]
            for status in ("equivalent", "partial", "missing", "conflicting", "deferred", "uncertain")
        },
        "command_target_status": dict(sorted(status_counts.items())),
        "catalog_evidence": dict(sorted(evidence_counts.items())),
        "source_classification": dict(sorted(source_counts.items())),
        **openapi_counts,
        **cql_counts,
        **test_counts,
        "runtime": {
            "python": "3.9.6 (below >=3.10)",
            "dependencies": "Typer/questionary/PyYAML/pytest unavailable",
            "command_probe": "failed before command construction: ModuleNotFoundError questionary",
            "tests_run": 0,
            "network_or_dependency_mutation": False,
        },
        "orphaned_surfaces": shadowed,
    }

    catalog_names = [name for name, _, _ in output_specs]
    summary["catalog_sha256"] = catalog_hashes(catalog_names)
    write_json("comfy-cli-reconciliation.json", summary)
    write_evidence(summary, shadowed)

    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
