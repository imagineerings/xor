#!/usr/bin/env python3

from __future__ import annotations

import argparse
import ast
import hashlib
import json
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[2]
SPANDREL_ROOT = WORKSPACE / "projects/comfy/Spandrel"
EXTRA_ROOT = WORKSPACE / "projects/comfy/spandrel-extra-arches"
COMFY_SOURCE = WORKSPACE / "projects/comfy/ComfyUI/comfy_extras/nodes_upscale_model.py"
COMFY_REQUIREMENTS = WORKSPACE / "projects/comfy/ComfyUI/requirements.txt"
OUTPUT = ROOT / "catalogs/spandrel-image-model-contract.json"
FIXTURE = (
    WORKSPACE
    / "crates/comfy_test_support/fixtures/models/spandrel-image-model-contract/contract-summary.json"
)

SNAPSHOTS = {
    "spandrel": {
        "root": SPANDREL_ROOT,
        "path": "projects/comfy/Spandrel",
        "package": "spandrel",
        "version": "0.4.2",
        "tag": "v0.4.2",
        "commit": "724cca389f28c38e1050689d4862a452fd644484",
        "sdist": "spandrel-0.4.2.tar.gz",
        "sdist_sha256": "fefa4ea966c6a5b7721dcf24f3e2062a5a96a395c8bedcb570fb55971fdcbccb",
        "file_count": 180,
        "tree_sha256": "e1870c42b314fddb290f4d5322a03743076d98d0c6d288fc73691e3013994bbb",
        "registry": "spandrel/__helpers/main_registry.py",
        "architecture_root": "spandrel/architectures",
        "registry_name": "MAIN_REGISTRY",
        "origin": "main",
        "code_license_disposition": (
            "development-oracle; each architecture requires an individually verifiable "
            "permissive license before native execution admission"
        ),
    },
    "spandrel_extra_arches": {
        "root": EXTRA_ROOT,
        "path": "projects/comfy/spandrel-extra-arches",
        "package": "spandrel-extra-arches",
        "version": "0.2.0",
        "tag": "v0.4.0",
        "commit": "a1db3f5debbeeacbe02fb4114c69feee56ba5e21",
        "sdist": "spandrel_extra_arches-0.2.0.tar.gz",
        "sdist_sha256": "9216877ecabc9c97e001ad5d49c4f8d2b1f6c6f82d1e77c8e2b350c586b6e64a",
        "file_count": 52,
        "tree_sha256": "7c0915d2e0df7db2131117087744fa5e73954dcad72aa785386d6bf8c1efb3aa",
        "registry": "spandrel_extra_arches/__helper.py",
        "architecture_root": "spandrel_extra_arches/architectures",
        "registry_name": "EXTRA_REGISTRY",
        "origin": "extra",
        "code_license_disposition": (
            "reference-only by default; restrictive, copyleft-incompatible, non-commercial, "
            "ambiguous, and unverified architectures are rejected"
        ),
    },
}

FORBIDDEN_COMPONENTS = {
    ".git",
    ".hg",
    ".svn",
    ".mypy_cache",
    ".nox",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "__pycache__",
    "env",
    "node_modules",
    "venv",
}
WEIGHT_SUFFIXES = {
    ".bin",
    ".ckpt",
    ".gguf",
    ".npy",
    ".npz",
    ".onnx",
    ".pickle",
    ".pkl",
    ".pt",
    ".pth",
    ".safetensors",
}
PERMISSIVE_MARKERS = ("apache license", "mit license", "bsd license", "public domain")
RESTRICTIVE_MARKERS = (
    "non-commercial",
    "noncommercial",
    "gpl",
    "agpl",
    "research only",
    "no commercial",
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def included_files(root: Path) -> list[Path]:
    if not root.is_dir() or root.is_symlink():
        raise ValueError(f"missing or invalid snapshot root: {root}")
    files: list[Path] = []
    for path in sorted(root.rglob("*"), key=lambda candidate: candidate.relative_to(root).as_posix()):
        relative = path.relative_to(root)
        if path.is_symlink():
            raise ValueError(f"snapshot symlink is forbidden: {path}")
        if any(component in FORBIDDEN_COMPONENTS for component in relative.parts):
            raise ValueError(f"snapshot metadata/cache/environment is forbidden: {path}")
        if path.is_dir():
            continue
        if not path.is_file():
            raise ValueError(f"snapshot special entry is forbidden: {path}")
        if path.suffix.lower() in WEIGHT_SUFFIXES:
            raise ValueError(f"model weight is forbidden: {path}")
        if path.name == ".DS_Store" or path.suffix == ".pyc":
            continue
        files.append(path)
    return files


def baseline_fingerprint(root: Path, files: list[Path]) -> str:
    digest = hashlib.sha256()
    for path in files:
        relative = "./" + path.relative_to(root).as_posix()
        digest.update(f"{sha256(path.read_bytes())}  {relative}\n".encode("utf-8"))
    return digest.hexdigest()


def literal_version(path: Path) -> str:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    for statement in tree.body:
        if not isinstance(statement, ast.Assign):
            continue
        if any(isinstance(target, ast.Name) and target.id == "__version__" for target in statement.targets):
            if isinstance(statement.value, ast.Constant) and isinstance(statement.value.value, str):
                return statement.value.value
    raise ValueError(f"missing literal __version__ in {path}")


def registry_entries(path: Path, registry_name: str) -> list[tuple[str, str]]:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    registry_call: ast.Call | None = None
    for statement in tree.body:
        if not isinstance(statement, ast.Expr) or not isinstance(statement.value, ast.Call):
            continue
        call = statement.value
        if (
            isinstance(call.func, ast.Attribute)
            and call.func.attr == "add"
            and isinstance(call.func.value, ast.Name)
            and call.func.value.id == registry_name
        ):
            if registry_call is not None:
                raise ValueError(f"duplicate {registry_name}.add call in {path}")
            registry_call = call
    if registry_call is None:
        raise ValueError(f"missing {registry_name}.add call in {path}")
    entries: list[tuple[str, str]] = []
    for argument in registry_call.args:
        if not (
            isinstance(argument, ast.Call)
            and isinstance(argument.func, ast.Attribute)
            and argument.func.attr == "from_architecture"
            and len(argument.args) == 1
            and isinstance(argument.args[0], ast.Call)
            and isinstance(argument.args[0].func, ast.Attribute)
            and isinstance(argument.args[0].func.value, ast.Name)
        ):
            raise ValueError(f"unsupported registry expression in {path}: {ast.unparse(argument)}")
        constructor = argument.args[0].func
        entries.append((constructor.value.id, constructor.attr))
    return entries


def call_name(call: ast.Call) -> str:
    function = call.func
    if isinstance(function, ast.Name):
        return function.id
    if isinstance(function, ast.Attribute):
        return function.attr
    return ""


def architecture_class(tree: ast.Module, class_name: str) -> ast.ClassDef:
    matches = [node for node in tree.body if isinstance(node, ast.ClassDef) and node.name == class_name]
    if len(matches) != 1:
        raise ValueError(f"expected one architecture class {class_name}, found {len(matches)}")
    return matches[0]


def architecture_identity(class_node: ast.ClassDef) -> tuple[str, str, str]:
    for node in ast.walk(class_node):
        if not isinstance(node, ast.Call):
            continue
        if not (isinstance(node.func, ast.Attribute) and node.func.attr == "__init__"):
            continue
        values = {keyword.arg: keyword.value for keyword in node.keywords if keyword.arg is not None}
        identifier = values.get("id")
        name = values.get("name", identifier)
        detect = values.get("detect")
        if not (
            isinstance(identifier, ast.Constant)
            and isinstance(identifier.value, str)
            and isinstance(name, ast.Constant)
            and isinstance(name.value, str)
            and detect is not None
        ):
            raise ValueError(f"non-literal architecture identity in {class_node.name}")
        return identifier.value, name.value, ast.unparse(detect)
    raise ValueError(f"missing Architecture.__init__ in {class_node.name}")


def load_contract(class_node: ast.ClassDef) -> dict[str, object]:
    load_functions = [
        node for node in class_node.body if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == "load"
    ]
    if len(load_functions) != 1:
        raise ValueError(f"expected one load method in {class_node.name}")
    load = load_functions[0]
    descriptor_calls = [
        node
        for node in ast.walk(class_node)
        if isinstance(node, ast.Call) and call_name(node).endswith("ModelDescriptor")
    ]
    descriptor_kinds = sorted({call_name(node) for node in descriptor_calls})
    keyword_expressions: dict[str, list[str]] = {
        "scale": [],
        "input_channels": [],
        "output_channels": [],
    }
    for call in descriptor_calls:
        for keyword in call.keywords:
            if keyword.arg in keyword_expressions:
                keyword_expressions[keyword.arg].append(ast.unparse(keyword.value))

    state_keys: set[str] = set()
    imports: set[str] = set()
    for node in ast.walk(class_node):
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            if "." in node.value or node.value.endswith(("weight", "bias")):
                state_keys.add(node.value)
    normalization = []
    for function in class_node.body:
        if not isinstance(function, (ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        for statement in function.body:
            rendered = ast.unparse(statement)
            if re.search(r"\b(state|state_dict)\b", rendered):
                normalization.append(rendered)
    return {
        "descriptor_kinds": descriptor_kinds,
        "descriptor_disposition": (
            "single-image" if descriptor_kinds == ["ImageModelDescriptor"] else "rejected-non-single-image"
        ),
        "normalized_state_keys": sorted(state_keys),
        "state_normalization": normalization,
        "scale_expressions": sorted(set(keyword_expressions["scale"])),
        "input_channel_expressions": sorted(set(keyword_expressions["input_channels"])),
        "output_channel_expressions": sorted(set(keyword_expressions["output_channels"])),
    }


def architecture_source_contract(root: Path, directory: Path) -> tuple[list[dict[str, str]], str, list[str]]:
    records: list[dict[str, str]] = []
    imports: set[str] = set()
    digest = hashlib.sha256()
    for path in sorted(directory.rglob("*.py")):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"unsupported architecture source: {path}")
        relative = path.relative_to(root).as_posix()
        file_sha = sha256(path.read_bytes())
        records.append({"path": relative, "sha256": file_sha})
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_sha.encode("ascii"))
        digest.update(b"\n")
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                imports.update(alias.name for alias in node.names)
            elif isinstance(node, ast.ImportFrom):
                imports.add(("." * node.level) + (node.module or ""))
    return records, digest.hexdigest(), sorted(imports)


def license_contract(directory: Path, origin: str) -> dict[str, object]:
    artifacts = [
        path
        for path in sorted(directory.rglob("*"))
        if path.is_file() and re.search(r"(^|[-_.])(license|copying|notice)([-_.]|$)", path.name, re.IGNORECASE)
    ]
    records = []
    combined = ""
    for path in artifacts:
        data = path.read_bytes()
        text = data.decode("utf-8", errors="replace")
        combined += "\n" + text.casefold()
        records.append({"path": path.name, "sha256": sha256(data)})
    permissive = bool(artifacts) and any(marker in combined for marker in PERMISSIVE_MARKERS)
    restrictive = any(marker in combined for marker in RESTRICTIVE_MARKERS)
    admitted = origin == "main" and permissive and not restrictive
    if origin == "extra":
        disposition = "rejected-reference-only-extra-architecture"
    elif not artifacts:
        disposition = "rejected-missing-individual-license-artifact"
    elif restrictive:
        disposition = "rejected-restrictive-or-ambiguous-license"
    elif not permissive:
        disposition = "rejected-unverified-license"
    else:
        disposition = "admitted-individually-permissive-license"
    return {
        "artifacts": records,
        "disposition": disposition,
        "eligible": admitted,
    }


def contract_row(
    snapshot: dict[str, object],
    module_name: str,
    class_name: str,
    origin_ordinal: int,
    ordinal: int,
) -> dict[str, object]:
    root = snapshot["root"]
    if not isinstance(root, Path):
        raise TypeError("snapshot root must be a Path")
    architecture_root = root / str(snapshot["architecture_root"])
    directory = architecture_root / module_name
    init_path = directory / "__init__.py"
    source = init_path.read_text(encoding="utf-8")
    tree = ast.parse(source, filename=str(init_path))
    class_node = architecture_class(tree, class_name)
    identifier, display_name, detection = architecture_identity(class_node)
    load = load_contract(class_node)
    detection_keys = sorted(
        {
            node.value
            for node in ast.walk(ast.parse(detection, mode="eval"))
            if isinstance(node, ast.Constant) and isinstance(node.value, str)
        }
    )
    load["normalized_state_keys"] = sorted(
        set(load["normalized_state_keys"]) | set(detection_keys)
    )
    sources, equation_sha, imports = architecture_source_contract(root, directory)
    dependency_sha = sha256("\n".join(imports).encode("utf-8"))
    license_info = license_contract(directory / "__arch", str(snapshot["origin"]))
    reasons = []
    if load["descriptor_disposition"] != "single-image":
        reasons.append(str(load["descriptor_disposition"]))
    if not license_info["eligible"]:
        reasons.append(str(license_info["disposition"]))
    admitted = not reasons
    family_material = json.dumps(
        {
            "architecture": identifier,
            "equation_sha256": equation_sha,
            "dependencies_sha256": dependency_sha,
            "descriptor_kinds": load["descriptor_kinds"],
        },
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return {
        "ordinal": ordinal,
        "origin": snapshot["origin"],
        "origin_ordinal": origin_ordinal,
        "architecture_id": identifier,
        "architecture_class": class_name,
        "display_name": display_name,
        "source_path": init_path.relative_to(WORKSPACE).as_posix(),
        "source_sha256": sha256(init_path.read_bytes()),
        "source_files": sources,
        "detection_predicate": detection,
        "detection_state_keys": detection_keys,
        **load,
        "equation_sha256": equation_sha,
        "dependency_imports": imports,
        "dependency_sha256": dependency_sha,
        "equation_family_id": "spandrel-equation-" + sha256(family_material)[:16],
        "license_artifacts": license_info["artifacts"],
        "license_disposition": license_info["disposition"],
        "model_use_disposition": "no-model-weights-approved; evaluate model rights independently",
        "support_disposition": "admitted" if admitted else "rejected",
        "rejection_reasons": reasons,
    }


def build_contract() -> dict[str, object]:
    snapshot_records: dict[str, object] = {}
    rows: list[dict[str, object]] = []
    seen_ids: set[str] = set()
    for key, snapshot in SNAPSHOTS.items():
        root = snapshot["root"]
        if not isinstance(root, Path):
            raise TypeError("snapshot root must be a Path")
        files = included_files(root)
        actual_count = len(files)
        actual_fingerprint = baseline_fingerprint(root, files)
        if actual_count != snapshot["file_count"] or actual_fingerprint != snapshot["tree_sha256"]:
            raise ValueError(
                f"{key} source drift: expected {snapshot['file_count']}/{snapshot['tree_sha256']}, "
                f"got {actual_count}/{actual_fingerprint}"
            )
        version_path = root / (
            "spandrel/__init__.py" if key == "spandrel" else "spandrel_extra_arches/__init__.py"
        )
        actual_version = literal_version(version_path)
        if actual_version != snapshot["version"]:
            raise ValueError(f"{key} version mismatch: {actual_version}")
        registry_path = root / str(snapshot["registry"])
        entries = registry_entries(registry_path, str(snapshot["registry_name"]))
        snapshot_records[key] = {
            field: value
            for field, value in snapshot.items()
            if field not in {"root", "registry", "architecture_root", "registry_name", "origin"}
        } | {
            "source_authority": "explicit user approval",
            "archive_verification": "official PyPI sdist SHA-256 matched before extraction",
            "included_file_count": actual_count,
            "baseline_tree_sha256": actual_fingerprint,
            "registry_source": registry_path.relative_to(WORKSPACE).as_posix(),
            "registry_source_sha256": sha256(registry_path.read_bytes()),
            "registry_entry_count": len(entries),
            "model_use_disposition": (
                "no model weights approved or bundled; every model license is evaluated independently; "
                "unknown or incompatible rights fail closed"
            ),
        }
        for origin_ordinal, (module_name, class_name) in enumerate(entries):
            row = contract_row(snapshot, module_name, class_name, origin_ordinal, len(rows))
            identifier = str(row["architecture_id"])
            if identifier in seen_ids:
                raise ValueError(f"duplicate architecture id: {identifier}")
            seen_ids.add(identifier)
            rows.append(row)

    admitted_families = sorted(
        {str(row["equation_family_id"]) for row in rows if row["support_disposition"] == "admitted"}
    )
    implementation_leaves = [
        {
            "equation_family_id": family,
            "task_id": f"comfy-parity-native-upscale-equation-{family.removeprefix('spandrel-equation-')}",
        }
        for family in admitted_families
    ]
    return {
        "schema_version": 1,
        "contract_id": "zed-comfy-spandrel-image-model-contract-v1",
        "source_snapshots": snapshot_records,
        "source_boundary": {
            "comfy_source": COMFY_SOURCE.relative_to(WORKSPACE).as_posix(),
            "comfy_source_sha256": sha256(COMFY_SOURCE.read_bytes()),
            "requirements_source": COMFY_REQUIREMENTS.relative_to(WORKSPACE).as_posix(),
            "requirements_source_sha256": sha256(COMFY_REQUIREMENTS.read_bytes()),
            "production_runtime": "native Rust only; no Python or Spandrel import or execution",
            "fixtures": "JSON only; no Python, model weights, native handles, or executable payloads",
        },
        "optional_extra_outcomes": [
            {
                "outcome": "absent-or-import-failure",
                "registry": "MAIN only",
                "diagnostic": "typed extra import unavailable",
            },
            {
                "outcome": "successful-add",
                "registry": "MAIN followed by EXTRA in source order",
                "diagnostic": "none",
            },
            {
                "outcome": "add-failure",
                "registry": "MAIN only",
                "diagnostic": "typed extra registry add failure",
            },
        ],
        "architectures": rows,
        "summary": {
            "architecture_count": len(rows),
            "main_count": sum(row["origin"] == "main" for row in rows),
            "extra_count": sum(row["origin"] == "extra" for row in rows),
            "admitted_count": sum(row["support_disposition"] == "admitted" for row in rows),
            "rejected_count": sum(row["support_disposition"] == "rejected" for row in rows),
            "individual_license_artifact_count": sum(bool(row["license_artifacts"]) for row in rows),
        },
        "task_projection": {
            "shared_runtime_contract_task_id": "comfy-parity-native-upscale-runtime-contract-foundation",
            "implementation_leaves": implementation_leaves,
            "final_integration_task_id": "comfy-parity-native-upscale-model-resource-foundation",
        },
    }


def encoded(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_or_check(path: Path, content: bytes, checking: bool) -> None:
    if checking:
        if not path.is_file() or path.read_bytes() != content:
            raise ValueError(f"generated artifact is stale: {path.relative_to(WORKSPACE)}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    contract = build_contract()
    write_or_check(OUTPUT, encoded(contract), args.check)
    fixture = {
        "schema_version": contract["schema_version"],
        "contract_id": contract["contract_id"],
        "source_snapshots": contract["source_snapshots"],
        "optional_extra_outcomes": contract["optional_extra_outcomes"],
        "summary": contract["summary"],
        "task_projection": contract["task_projection"],
        "catalog_sha256": sha256(encoded(contract)),
    }
    write_or_check(FIXTURE, encoded(fixture), args.check)
    print(
        f"Generated {contract['summary']['architecture_count']} Spandrel rows "
        f"({contract['summary']['admitted_count']} admitted, {contract['summary']['rejected_count']} rejected)."
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, SyntaxError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
