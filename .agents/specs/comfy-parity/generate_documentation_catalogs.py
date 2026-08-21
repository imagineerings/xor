#!/usr/bin/env python3

from __future__ import annotations

import csv
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Iterable


SPEC_DIR = Path(__file__).resolve().parent
REPO = SPEC_DIR.parents[2]
CATALOGS = SPEC_DIR / "catalogs"
DOCS = REPO / "projects/comfy/docs"
EMBEDDED = REPO / "projects/comfy/embedded-docs"
EMBEDDED_NODE_DOCS = EMBEDDED / "comfyui_embedded_docs/docs"
COMFYUI = REPO / "projects/comfy/ComfyUI"


def stable_id(prefix: str, key: str) -> str:
    digest = hashlib.sha256(key.encode("utf-8")).hexdigest()[:12].upper()
    return f"{prefix}-{digest}"


def write_csv(path: Path, fieldnames: list[str], rows: Iterable[dict[str, object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames, lineterminator="\n")
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field, "") for field in fieldnames})


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def source_files(root: Path) -> list[Path]:
    files = [
        path
        for path in root.rglob("*")
        if path.is_file()
        and "node_modules" not in path.parts
        and "__pycache__" not in path.parts
        and path.suffix != ".pyc"
        and path.name != ".DS_Store"
    ]
    return sorted(files, key=lambda path: path.relative_to(root).as_posix().encode("utf-8"))


def tree_fingerprint(root: Path, files: list[Path]) -> str:
    records: list[str] = []
    for path in files:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        records.append(f"{digest}  ./{path.relative_to(root).as_posix()}\n")
    return hashlib.sha256("".join(records).encode("utf-8")).hexdigest()


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def normalize_identifier(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


def parse_frontmatter(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    if not text.startswith("---\n"):
        heading = re.search(r"^#\s+(.+)$", text, re.MULTILINE)
        return {"title": heading.group(1).strip() if heading else path.stem}
    end = text.find("\n---\n", 4)
    if end < 0:
        return {"title": path.stem}
    result: dict[str, str] = {}
    for line in text[4:end].splitlines():
        match = re.match(r"^([A-Za-z][A-Za-z0-9_-]*):\s*[\"']?(.*?)[\"']?\s*$", line)
        if match:
            result[match.group(1)] = match.group(2)
    return result


backend_nodes = read_csv(CATALOGS / "backend-nodes.csv")
backend_inactive_nodes = read_csv(CATALOGS / "backend-inactive-nodes.csv")
all_backend_nodes = backend_nodes + backend_inactive_nodes
backend_node_id = {row["node_identifier"]: row for row in all_backend_nodes}
backend_class: dict[str, list[dict[str, str]]] = defaultdict(list)
backend_node_normalized: dict[str, list[dict[str, str]]] = defaultdict(list)
backend_class_normalized: dict[str, list[dict[str, str]]] = defaultdict(list)
for backend_node in all_backend_nodes:
    backend_class[backend_node["class_name"]].append(backend_node)
    backend_node_normalized[normalize_identifier(backend_node["node_identifier"])].append(backend_node)
    backend_class_normalized[normalize_identifier(backend_node["class_name"])].append(backend_node)


def map_node_name(name: str) -> tuple[str, list[dict[str, str]]]:
    if name in backend_node_id:
        return "registered-node-id-exact", [backend_node_id[name]]
    if name in backend_class:
        return "registered-class-name-exact", backend_class[name]
    normalized = normalize_identifier(name)
    if normalized in backend_node_normalized:
        return "registered-node-id-case-or-punctuation", backend_node_normalized[normalized]
    if normalized in backend_class_normalized:
        return "registered-class-name-case-or-punctuation", backend_class_normalized[normalized]
    return "unmatched", []


frontend_virtual_nodes = {
    "Note": "COMFY-FRONTEXT-EXT-019",
    "MarkdownNote": "COMFY-FRONTEXT-EXT-019",
    "Reroute": "COMFY-GRAPH-009; COMFY-FRONTEXT-EXT-023",
}
replacement_nodes = {
    "Load3DAnimation": "COMFY-NODE-0334; COMFY-EXEC-004; COMFY-EXT-022",
    "Preview3DAnimation": "COMFY-NODE-0487; COMFY-EXEC-004; COMFY-EXT-022",
}
conditional_frontend_consumers = {
    "Save3DAdvanced": "COMFY-FRONTEXT-EXT-016; COMFY-FRONTEXT-EXT-024",
    "SaveGaussianSplat": "COMFY-FRONTEXT-EXT-016; COMFY-FRONTEXT-EXT-024",
    "SavePointCloud": "COMFY-FRONTEXT-EXT-016; COMFY-FRONTEXT-EXT-024",
    "SaveText": "COMFY-FRONTEXT-EXT-022",
}
unregistered_executable_nodes = {
    "SetModelHooksOnCond": "projects/comfy/ComfyUI/comfy_extras/nodes_hooks.py:602",
}


def node_corroboration(name: str) -> tuple[str, str, str]:
    match_kind, rows = map_node_name(name)
    if rows:
        feature_ids = "; ".join(sorted({row["feature_id"] for row in rows}))
        return match_kind, feature_ids, "executable-backend-registry"
    if name in frontend_virtual_nodes:
        return "frontend-native-virtual-node", frontend_virtual_nodes[name], "executable-frontend-registry"
    if name in replacement_nodes:
        return "legacy-node-replacement", replacement_nodes[name], "executable-replacement-registry"
    if name in conditional_frontend_consumers:
        return (
            "frontend-consumer-without-snapshot-provider",
            conditional_frontend_consumers[name],
            "partial-executable-corroboration",
        )
    if name in unregistered_executable_nodes:
        return "unregistered-executable-class", "", "code-present-not-registered"
    return "provider-unverified", "", "documented-only"


docs_files = source_files(DOCS)
embedded_files = source_files(EMBEDDED)
assert len(docs_files) == 5800, len(docs_files)
assert len(embedded_files) == 10298, len(embedded_files)
docs_fingerprint = tree_fingerprint(DOCS, docs_files)
embedded_fingerprint = tree_fingerprint(EMBEDDED, embedded_files)
assert docs_fingerprint == "1f4c9c460b8f5b35e30eb4d2d64bc201a958f247ab21af6c68743cce28c33931"
assert embedded_fingerprint == "5aebf925cf36fe7b8df3c89466ad96ffa42110542a392ec6156b88fc807ec956"


locale_roots = {"ja", "zh", "ko"}
media_suffixes = {".jpg", ".jpeg", ".png", ".webp", ".gif", ".svg", ".ico", ".mp4"}


def is_docs_localized(relative: Path) -> bool:
    return relative.parts[0] in locale_roots or (
        relative.parts[0] == "snippets"
        and len(relative.parts) > 1
        and relative.parts[1] in locale_roots
    )


def docs_disposition(relative: Path) -> tuple[str, str]:
    relative_string = relative.as_posix()
    if is_docs_localized(relative):
        return "localized generated content", "Generated translation; English MDX is authoritative."
    if relative_string.startswith(".github/scripts/cms/staging/") and relative.suffix == ".mdx":
        return "CMS staging content", "Separate staged release-note pipeline; not canonical product behavior."
    if relative.parts[0] == "built-in-nodes" and relative.suffix == ".mdx":
        return "English built-in-node documentation", "Node claim source requiring executable registry corroboration."
    if relative.parts[0] == "snippets" and relative.suffix == ".mdx":
        return "English reusable snippet", "Reusable English documentation claim container."
    if relative.suffix == ".mdx":
        return "English product documentation", "English source-of-truth claim container."
    if relative.suffix in media_suffixes:
        return "media asset", "Static documentation media; not executable feature evidence."
    if relative.suffix in {".ts", ".mjs", ".py", ".js"}:
        return "executable automation/tooling", "Repository tooling, not Comfy production behavior."
    if relative_string.startswith(".github/workflows/") and relative.suffix in {".yml", ".yaml"}:
        return "CI workflow", "Repository lifecycle automation, not Comfy production behavior."
    if relative.suffix in {".json", ".yaml", ".yml", ".toml"}:
        return "configuration/schema/lock/registry", "Structured documentation/tooling configuration or schema."
    if relative.suffix in {".md", ".mdc"}:
        return "governance/tool documentation", "Repository governance or tooling documentation."
    return "repository/site infrastructure", "Repository metadata or site infrastructure."


docs_page_paths = sorted(
    path for path in DOCS.rglob("*.mdx") if not is_docs_localized(path.relative_to(DOCS))
)
assert len(docs_page_paths) == 1273
docs_page_record_by_path: dict[str, str] = {}
docs_page_rows: list[dict[str, object]] = []


def page_role(relative: Path) -> str:
    value = relative.as_posix()
    if value.startswith(".github/scripts/cms/staging/"):
        return "CMS staging English" if "/en/" in value else "CMS staging localization"
    if relative.parts[0] == "built-in-nodes":
        return "built-in node reference"
    if relative.parts[0] == "snippets":
        return "reusable English snippet"
    return "English product page"


page_feature_map = {
    "custom-nodes/backend/lifecycle.mdx": "COMFY-EXT-003; COMFY-EXT-006; COMFY-EXT-009",
    "custom-nodes/backend/node-replacement.mdx": "COMFY-EXEC-004; COMFY-EXT-022",
    "custom-nodes/v3_migration.mdx": "COMFY-EXT-007; COMFY-EXT-015; COMFY-EXT-016",
    "custom-nodes/js/javascript_hooks.mdx": "; ".join(
        f"COMFY-FRONTEXT-EXT-{number:03d}" for number in range(46, 60)
    ),
    "custom-nodes/help_page.mdx": "COMFY-API-0090; COMFY-EXT-009",
    "specs/workflow_json.mdx": "COMFY-FORMAT-042",
    "specs/workflow_json_0.4.mdx": "COMFY-FORMAT-041",
    "development/api-development/workflow-api-format.mdx": "COMFY-FORMAT-044; COMFY-FORMAT-045",
}


for page_path in docs_page_paths:
    relative = page_path.relative_to(DOCS)
    relative_string = relative.as_posix()
    record_id = stable_id("COMFY-DOC", relative_string)
    docs_page_record_by_path[relative_string] = record_id
    frontmatter = parse_frontmatter(page_path)
    mapped_feature_ids = page_feature_map.get(relative_string, "")
    corroboration = "documented-only-claim-container"
    if relative.parts[0] == "built-in-nodes" and len(relative.parts) == 2 and relative.stem != "overview":
        match_kind, mapped_feature_ids, corroboration = node_corroboration(relative.stem)
        corroboration = f"{match_kind}; {corroboration}"
    elif mapped_feature_ids:
        corroboration = "linked-to-executable-catalog; individual prose claims still require field-level corroboration"
    elif relative_string.startswith("comfy-cli/") or relative_string == "agent-tools/comfy-cli.mdx":
        corroboration = "pending-executable-comfy-cli-catalog-reconciliation"
    availability = "active"
    if relative_string.startswith(("cloud/", "development/cloud/", "api-reference/cloud/")):
        availability = "cloud/paid; experimental"
    elif relative_string.startswith(".github/"):
        availability = "infrastructure-only"
    docs_page_rows.append(
        {
            "record_id": record_id,
            "path": relative_string,
            "title": frontmatter.get("title", relative.stem),
            "description": frontmatter.get("description", ""),
            "role": page_role(relative),
            "domain": relative.parts[0],
            "availability": availability,
            "document_evidence_level": "documented-only",
            "corroboration_status": corroboration,
            "corroborated_feature_ids": mapped_feature_ids,
            "native_parity_treatment": "Use as conformance-oracle prose only; implement the observable contract natively in Rust/GPUI or preserve an explicit uncertainty.",
        }
    )


write_csv(
    CATALOGS / "docs-pages.csv",
    [
        "record_id",
        "path",
        "title",
        "description",
        "role",
        "domain",
        "availability",
        "document_evidence_level",
        "corroboration_status",
        "corroborated_feature_ids",
        "native_parity_treatment",
    ],
    docs_page_rows,
)


embedded_names = sorted(path.name for path in EMBEDDED_NODE_DOCS.iterdir() if path.is_dir())
assert len(embedded_names) == 855
docs_top_level_by_case = {
    path.stem.casefold(): path for path in (DOCS / "built-in-nodes").glob("*.mdx")
}
fingerprint_pattern = re.compile(r"\*\*Source fingerprint \(SHA-256\):\*\* `([0-9a-f]{64})`")
embedded_node_rows: list[dict[str, object]] = []
embedded_record_by_name: dict[str, str] = {}


for node_name in embedded_names:
    node_directory = EMBEDDED_NODE_DOCS / node_name
    record_id = stable_id("COMFY-EMBEDDOC", node_name)
    embedded_record_by_name[node_name] = record_id
    markdown_files = sorted(path.stem for path in node_directory.glob("*.md"))
    assert markdown_files == sorted(["ar", "en", "es", "fa", "fr", "ja", "ko", "pt-BR", "ru", "tr", "zh", "zh-TW"])
    asset_files = sorted(
        path.relative_to(node_directory).as_posix()
        for path in node_directory.rglob("*")
        if path.is_file() and path.suffix != ".md"
    )
    visual_media_count = sum(Path(asset_file).suffix in media_suffixes for asset_file in asset_files)
    en_text = (node_directory / "en.md").read_text(encoding="utf-8", errors="replace")
    declared_match = fingerprint_pattern.search(en_text)
    declared_fingerprint = declared_match.group(1) if declared_match else ""
    docs_copy = docs_top_level_by_case.get(node_name.casefold())
    docs_copy_fingerprint = ""
    docs_sync_status = "absent-from-docs-site"
    docs_path = ""
    if docs_copy:
        docs_path = docs_copy.relative_to(DOCS).as_posix()
        docs_text = docs_copy.read_text(encoding="utf-8", errors="replace")
        docs_match = fingerprint_pattern.search(docs_text)
        docs_copy_fingerprint = docs_match.group(1) if docs_match else ""
        if declared_fingerprint and docs_copy_fingerprint:
            docs_sync_status = "fingerprint-match" if declared_fingerprint == docs_copy_fingerprint else "fingerprint-mismatch"
        else:
            docs_sync_status = "present-no-comparable-fingerprint"
        if docs_copy.stem != node_name:
            docs_sync_status += "; path-case-differs"
    match_kind, mapped_feature_ids, corroboration = node_corroboration(node_name)
    embedded_node_rows.append(
        {
            "record_id": record_id,
            "node_document_name": node_name,
            "locales": "; ".join(markdown_files),
            "locale_count": len(markdown_files),
            "asset_files": "; ".join(asset_files),
            "asset_count": len(asset_files),
            "visual_media_count": visual_media_count,
            "all_english_docs_ai_generated_marker": "true" if "This documentation was AI-generated" in en_text else "false",
            "declared_source_fingerprint": declared_fingerprint,
            "docs_site_path": docs_path,
            "docs_site_declared_fingerprint": docs_copy_fingerprint,
            "docs_sync_status": docs_sync_status,
            "registry_match_kind": match_kind,
            "corroboration_status": corroboration,
            "corroborated_feature_ids": mapped_feature_ids,
            "evidence_level": "documented-only",
            "native_parity_treatment": "Map the legacy identifier to a versioned native Rust/WASM node descriptor; never execute Python or JavaScript in production.",
        }
    )


write_csv(
    CATALOGS / "embedded-docs-nodes.csv",
    [
        "record_id",
        "node_document_name",
        "locales",
        "locale_count",
        "asset_files",
        "asset_count",
        "visual_media_count",
        "all_english_docs_ai_generated_marker",
        "declared_source_fingerprint",
        "docs_site_path",
        "docs_site_declared_fingerprint",
        "docs_sync_status",
        "registry_match_kind",
        "corroboration_status",
        "corroborated_feature_ids",
        "evidence_level",
        "native_parity_treatment",
    ],
    embedded_node_rows,
)


docs_node_rows: list[dict[str, object]] = []
for path in sorted((DOCS / "built-in-nodes").rglob("*.mdx")):
    relative = path.relative_to(DOCS)
    node_candidate = path.stem
    match_kind = "section-or-overview"
    feature_ids = ""
    corroboration = "documented-only"
    if len(relative.parts) == 2 and node_candidate != "overview":
        match_kind, feature_ids, corroboration = node_corroboration(node_candidate)
    embedded_matches = [name for name in embedded_names if name.casefold() == node_candidate.casefold()]
    embedded_record_ids = "; ".join(embedded_record_by_name[name] for name in embedded_matches)
    docs_node_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-NODE", relative.as_posix()),
            "path": relative.as_posix(),
            "node_candidate": node_candidate,
            "layout": "top-level-synced-or-node" if len(relative.parts) == 2 else "nested-legacy-or-partner-page",
            "registry_match_kind": match_kind,
            "corroboration_status": corroboration,
            "corroborated_feature_ids": feature_ids,
            "embedded_doc_record_ids": embedded_record_ids,
            "evidence_level": "documented-only",
        }
    )


write_csv(
    CATALOGS / "docs-node-docs.csv",
    [
        "record_id",
        "path",
        "node_candidate",
        "layout",
        "registry_match_kind",
        "corroboration_status",
        "corroborated_feature_ids",
        "embedded_doc_record_ids",
        "evidence_level",
    ],
    docs_node_rows,
)


def parse_openapi_operations(path: Path) -> list[dict[str, str]]:
    operations: list[dict[str, str]] = []
    current_path = ""
    current_operation: dict[str, str] | None = None
    collecting_tags = False
    for line in path.read_text(encoding="utf-8").splitlines():
        path_match = re.match(r"^  (/[^:]*):\s*$", line)
        if path_match:
            current_path = path_match.group(1)
            current_operation = None
            collecting_tags = False
            continue
        method_match = re.match(r"^    (get|post|put|patch|delete|head|options|trace):\s*$", line)
        if method_match and current_path:
            current_operation = {
                "method": method_match.group(1).upper(),
                "path": current_path,
                "operation_id": "",
                "summary": "",
                "tags": "",
            }
            operations.append(current_operation)
            collecting_tags = False
            continue
        if not current_operation:
            continue
        operation_id_match = re.match(r"^      operationId:\s*(.*)\s*$", line)
        if operation_id_match:
            current_operation["operation_id"] = operation_id_match.group(1).strip('"\'')
        summary_match = re.match(r"^      summary:\s*(.*)\s*$", line)
        if summary_match:
            current_operation["summary"] = summary_match.group(1).strip('"\'')
        if re.match(r"^      tags:\s*$", line):
            collecting_tags = True
            continue
        if collecting_tags:
            tag_match = re.match(r"^        -\s*(.*)\s*$", line)
            if tag_match:
                current_operation["tags"] = "; ".join(
                    filter(None, [current_operation["tags"], tag_match.group(1).strip('"\'')])
                )
            elif line.strip():
                collecting_tags = False
    return operations


def normalize_route(path: str) -> str:
    path = path.split("?", 1)[0]
    path = re.sub(r"\{[^}]+\}", "{}", path)
    if path.startswith("/api"):
        path = path[4:] or "/"
    return path


backend_routes = read_csv(CATALOGS / "backend-http-routes.csv")
frontend_routes = read_csv(CATALOGS / "frontend-http-usage.csv")
backend_route_set = {(row["method"], normalize_route(row["path"])) for row in backend_routes}
frontend_route_set = {(row["method"], normalize_route(row["route"])) for row in frontend_routes}
openapi_operations = parse_openapi_operations(DOCS / "openapi-cloud.yaml")
assert len(openapi_operations) == 42
openapi_rows: list[dict[str, object]] = []
for operation in openapi_operations:
    key = (operation["method"], normalize_route(operation["path"]))
    backend_match = key in backend_route_set
    frontend_match = key in frontend_route_set or ("HTTP", key[1]) in frontend_route_set
    if backend_match and frontend_match:
        reconciliation = "backend-and-frontend-route-shape-corroborated"
    elif backend_match:
        reconciliation = "backend-route-shape-corroborated"
    elif frontend_match:
        reconciliation = "frontend-route-shape-corroborated"
    else:
        reconciliation = "no-executable-route-shape-corroboration"
    feature_ids = sorted(
        {
            row["feature_id"]
            for row in backend_routes
            if row["method"] == operation["method"]
            and normalize_route(row["path"]) == key[1]
        }
        | {
            row["feature_id"]
            for row in frontend_routes
            if row["method"] in {operation["method"], "HTTP"}
            and normalize_route(row["route"]) == key[1]
        }
    )
    openapi_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-OPENAPI", f"{operation['method']} {operation['path']}"),
            **operation,
            "openapi_version": "3.0.3",
            "api_info_version": "1.0.0",
            "availability": "cloud/paid; experimental",
            "document_evidence_level": "documented-only",
            "route_shape_reconciliation": reconciliation,
            "corroborated_feature_ids": "; ".join(feature_ids),
            "native_parity_treatment": "Do not implement paid cloud semantics from documentation alone; retain as an explicit provider/defer decision.",
        }
    )


write_csv(
    CATALOGS / "docs-openapi-cloud.csv",
    [
        "record_id",
        "method",
        "path",
        "operation_id",
        "summary",
        "tags",
        "openapi_version",
        "api_info_version",
        "availability",
        "document_evidence_level",
        "route_shape_reconciliation",
        "corroborated_feature_ids",
        "native_parity_treatment",
    ],
    openapi_rows,
)


docs_json = json.loads((DOCS / "docs.json").read_text(encoding="utf-8"))
redirect_rows = [
    {
        "record_id": stable_id("COMFY-DOC-REDIRECT", f"{row['source']}->{row['destination']}"),
        "source": row["source"],
        "destination": row["destination"],
        "evidence_level": "code-inferred",
        "availability": "documentation-site",
    }
    for row in docs_json["redirects"]
]
assert len(redirect_rows) == 65
write_csv(
    CATALOGS / "docs-redirects.csv",
    ["record_id", "source", "destination", "evidence_level", "availability"],
    redirect_rows,
)


package_json = json.loads((DOCS / "package.json").read_text(encoding="utf-8"))
tool_source_files = sorted(
    path
    for path in (DOCS / ".github/scripts").rglob("*")
    if path.is_file() and path.suffix in {".ts", ".mjs", ".js", ".py"}
)
flag_pattern = re.compile(r"[\"'`](-{1,2}[A-Za-z][A-Za-z0-9-]*)")
flag_evidence: dict[str, list[str]] = defaultdict(list)
for source_path in tool_source_files:
    for line_number, line in enumerate(source_path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        for match in flag_pattern.finditer(line):
            flag_evidence[match.group(1)].append(f"{source_path.relative_to(DOCS).as_posix()}:{line_number}")
assert len(flag_evidence) == 41, len(flag_evidence)
environment_names = [
    "ANALYTICS_PAGE_DELAY_MS",
    "ANALYTICS_PAGE_LIMIT",
    "CHECK_I18N_OUTPUT",
    "CMS_API_TOKEN",
    "CMS_BASE_URL",
    "CMS_PROJECT",
    "CMS_PUBLISH_ALL",
    "CMS_PUBLISH_CONFIRM",
    "CMS_SYNC_AFTER",
    "CMS_SYNC_ALL",
    "CMS_SYNC_BEFORE",
    "DASHSCOPE_API_KEY",
    "DEEPSEEK_API_KEY",
    "FRONTEND_LOCALES_PATH",
    "GITHUB_TOKEN",
    "MINTLIFY_API_KEY",
    "MINTLIFY_PROJECT_ID",
    "REVIEW_API_BASE_URL",
    "REVIEW_API_KEY",
    "REVIEW_API_MODEL",
    "REVIEW_CONCURRENCY",
    "TRANSLATE_API_BASE_URL",
    "TRANSLATE_API_KEY",
    "TRANSLATE_API_MODEL",
    "TRANSLATE_API_TIMEOUT_MS",
    "TRANSLATE_CJK_API_KEY",
    "TRANSLATE_CJK_BASE_URL",
    "TRANSLATE_CJK_CONCURRENCY",
    "TRANSLATE_CJK_MODEL",
    "TRANSLATE_CONCURRENCY",
]
assert len(environment_names) == 30


def first_evidence(name: str) -> str:
    candidates = [DOCS / ".env.local.example", *tool_source_files]
    for candidate in candidates:
        for line_number, line in enumerate(candidate.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            if name in line:
                return f"{candidate.relative_to(DOCS).as_posix()}:{line_number}"
    return ""


tooling_rows: list[dict[str, object]] = []
for name, command in sorted(package_json["scripts"].items()):
    tooling_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-TOOL", f"package-script:{name}"),
            "kind": "package script",
            "name": name,
            "contract": command,
            "source_evidence": f"package.json:scripts.{name}",
            "availability": "developer-only",
            "evidence_level": "code-inferred",
        }
    )
for flag, evidence in sorted(flag_evidence.items()):
    tooling_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-TOOL", f"flag:{flag}"),
            "kind": "tool flag literal",
            "name": flag,
            "contract": "Accepted or referenced by the cited repository tooling; validate per owning parser before invocation.",
            "source_evidence": "; ".join(evidence),
            "availability": "developer-only",
            "evidence_level": "code-inferred",
        }
    )
for environment_name in environment_names:
    tooling_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-TOOL", f"environment:{environment_name}"),
            "kind": "tool environment variable",
            "name": environment_name,
            "contract": "Documentation-repository tooling configuration or credential; not a production Zed environment variable.",
            "source_evidence": first_evidence(environment_name),
            "availability": "developer-only",
            "evidence_level": "code-inferred",
        }
    )
for workflow_path in sorted((DOCS / ".github/workflows").glob("*.y*ml")):
    name = workflow_path.name
    tooling_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-TOOL", f"workflow:{name}"),
            "kind": "CI workflow",
            "name": name,
            "contract": "Repository CI lifecycle defined in the cited workflow.",
            "source_evidence": workflow_path.relative_to(DOCS).as_posix(),
            "availability": "infrastructure-only",
            "evidence_level": "code-inferred",
        }
    )
assert len(tooling_rows) == 108
write_csv(
    CATALOGS / "docs-tooling.csv",
    [
        "record_id",
        "kind",
        "name",
        "contract",
        "source_evidence",
        "availability",
        "evidence_level",
    ],
    tooling_rows,
)


config_files = [
    "pnpm-lock.yaml",
    "docs.json",
    ".mcp.json",
    "package-lock.json",
    "package.json",
    "openapi-cloud.yaml",
    "pnpm-workspace.yaml",
    "public/workflow.json",
    "public/text-to-image.json",
    ".github/scripts/cms/published-versions.schema.json",
    ".github/scripts/cms/cms-config.json",
    ".github/scripts/cms/published-versions.json",
    ".github/scripts/cms/attention-overrides.json",
    ".github/scripts/i18n/translation-config.json",
    ".github/scripts/i18n/glossary/overrides/zh.json",
    ".github/scripts/i18n/glossary/overrides/ja.json",
    ".github/scripts/i18n/glossary/overrides/ko.json",
    ".github/scripts/i18n/glossary/frontend/zh.json",
    ".github/scripts/i18n/glossary/frontend/ja.json",
    ".github/scripts/i18n/glossary/frontend/ko.json",
]
assert len(config_files) == 20
format_contracts = [
    ("Workflow JSON 1.0", "specs/workflow_json.mdx", "COMFY-FORMAT-042", "version=1 editor graph schema"),
    ("Workflow JSON 0.4", "specs/workflow_json_0.4.mdx", "COMFY-FORMAT-041", "legacy editor graph schema"),
    ("Node definition JSON 2.0", "specs/nodedef_json.mdx", "COMFY-EXT-016", "current object-info/node-definition schema"),
    ("Node definition JSON 1.0", "specs/nodedef_json_1_0.mdx", "COMFY-EXT-016", "legacy node-definition schema"),
    ("API workflow JSON", "development/api-development/workflow-api-format.mdx", "COMFY-FORMAT-044", "execution graph keyed by node id"),
    ("Embedded node documentation layout", "custom-nodes/help_page.mdx", "COMFY-API-0090", "NodeName.md or NodeName/<locale>.md with media"),
    ("Cloud OpenAPI", "openapi-cloud.yaml", "", "OpenAPI 3.0.3 document with API info version 1.0.0"),
    ("CMS published versions registry", ".github/scripts/cms/published-versions.schema.json", "", "version registry validated by JSON Schema"),
    ("CMS staged changelog MDX", ".github/scripts/cms/staging", "", "separate staged multilingual popup release notes"),
    ("Translation hash frontmatter", ".github/scripts/i18n/README.md", "", "translationSourceHash, translationBlockHashes, and reviewSourceHash"),
    ("Analytics checkpoint/cache", ".github/scripts/analytics/README.md", "", "checkpoint, store, manifest, daily JSON, and Markdown summaries"),
    ("CLI snapshot JSON or YAML", "snippets/cli-reference/nodes.mdx", "", "custom-node environment snapshot"),
    ("CLI workflow JSON or PNG", "snippets/cli-reference/nodes.mdx", "", "workflow dependency carrier claimed by CLI docs"),
    ("Workflow blueprint YAML", "comfy-cli/getting-started.mdx", "", "workflow fragment composition input"),
    ("Documentation-site MDX", "docs.json", "", "Mintlify pages, snippets, imports, navigation, and redirects"),
]
config_format_rows: list[dict[str, object]] = []
for config_file in config_files:
    config_format_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-CONFIG", config_file),
            "kind": "configuration/schema/lock/registry file",
            "name": config_file,
            "source_evidence": config_file,
            "contract": "See source file; this is documentation or documentation-tooling configuration unless separately corroborated.",
            "corroborated_feature_ids": "",
            "evidence_level": "code-inferred",
        }
    )
for name, source, feature_ids, contract in format_contracts:
    config_format_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-FORMAT", name),
            "kind": "documented format contract",
            "name": name,
            "source_evidence": source,
            "contract": contract,
            "corroborated_feature_ids": feature_ids,
            "evidence_level": "documented-only",
        }
    )
assert len(config_format_rows) == 35
write_csv(
    CATALOGS / "docs-config-formats.csv",
    [
        "record_id",
        "kind",
        "name",
        "source_evidence",
        "contract",
        "corroborated_feature_ids",
        "evidence_level",
    ],
    config_format_rows,
)


frontend_extension_rows = read_csv(CATALOGS / "frontend-extensions.csv")
frontend_interface_rows = [row for row in frontend_extension_rows if row["entry_kind"] == "interface member"]
assert len(frontend_interface_rows) == 27
extension_rows: list[dict[str, object]] = []


def append_extension(
    family: str,
    name: str,
    source: str,
    feature_ids: str,
    legacy_behavior: str,
    native_port: str,
    availability: str = "active",
) -> None:
    extension_rows.append(
        {
            "record_id": stable_id("COMFY-DOC-EXT", f"{family}:{name}"),
            "family": family,
            "name": name,
            "legacy_behavior": legacy_behavior,
            "source_evidence": source,
            "corroborated_feature_ids": feature_ids,
            "evidence_level": "documented-only",
            "availability": availability,
            "native_rust_wasm_port": native_port,
            "production_legacy_execution": "prohibited",
        }
    )


legacy_v1_contracts = [
    ("custom_nodes discovery", "Scan custom_nodes and import modules", "plugin manifest discovery port"),
    ("__init__.py", "Execute Python package initializer", "never execute; ingest signed compatibility manifest only"),
    ("__all__", "Python export declaration", "legacy manifest import mapping"),
    ("NODE_CLASS_MAPPINGS", "Map globally unique legacy node identifier to Python class", "legacy identifier to native descriptor mapping"),
    ("NODE_DISPLAY_NAME_MAPPINGS", "Map legacy identifier to display name", "localized native descriptor field"),
    ("WEB_DIRECTORY", "Serve extension JavaScript and documentation", "static signed assets/docs port; no JavaScript execution"),
    ("failed import isolation", "Continue startup and report failed custom-node import", "plugin quarantine and visible diagnostic state"),
]
for name, behavior, native_port in legacy_v1_contracts:
    append_extension(
        "Python V1 legacy",
        name,
        "custom-nodes/backend/lifecycle.mdx",
        "COMFY-EXT-003; COMFY-EXT-006; COMFY-EXT-009",
        behavior,
        native_port,
        "deprecated compatibility",
    )


v3_contracts = [
    ("comfy_api.latest", "Moving development API alias", "version-negotiated Rust/WASM SDK import"),
    ("comfy_api.v0_0_2", "Pinned versioned Python API", "versioned Rust/WASM ABI namespace"),
    ("io.ComfyNode", "Node base class", "native node trait"),
    ("define_schema", "Return typed node schema", "declarative node descriptor port"),
    ("execute", "Class-method node execution", "typed native/WASM execution port"),
    ("ComfyExtension", "Extension lifecycle object", "versioned plugin descriptor and lifecycle trait"),
    ("comfy_entrypoint", "Return extension object", "signed plugin entry export"),
    ("get_node_list", "Return registered node classes", "node descriptor enumeration port"),
    ("on_load", "Register extension resources and replacements", "bounded initialization port"),
    ("validate_inputs", "Validate node inputs", "pure validation port"),
    ("check_lazy_status", "Request lazy inputs", "lazy dependency-selection port"),
    ("fingerprint_inputs", "Return change-detection key", "deterministic cache-key port"),
    ("NodeReplace", "Declare legacy node replacement", "versioned legacy identifier mapping"),
    ("old_widget_ids", "Bind positional legacy widgets", "legacy widget-index mapping"),
    ("input_mapping", "Map or default replacement inputs", "typed input migration descriptor"),
    ("output_mapping", "Map replacement output slots", "typed output migration descriptor"),
    ("GET /api/node_replacements", "Expose replacement registry", "native read-only replacement registry service"),
]
for name, behavior, native_port in v3_contracts:
    append_extension(
        "Python V3 legacy",
        name,
        "custom-nodes/v3_migration.mdx; custom-nodes/backend/node-replacement.mdx",
        "COMFY-EXT-007; COMFY-EXT-015; COMFY-EXT-016; COMFY-EXT-022",
        behavior,
        native_port,
        "active source compatibility",
    )


for row in frontend_interface_rows:
    append_extension(
        "JavaScript frontend legacy",
        row["name"],
        "custom-nodes/js; " + row["source_file"],
        row["feature_id"],
        row["behavior"],
        "Explicit capability-scoped host port or declarative contribution; DOM, prototype, global-object, and arbitrary JavaScript execution are not supported.",
        row["availability"],
    )


embedded_extension_contracts = [
    ("NodeName.md fallback", "Default node documentation fallback", "native documentation lookup port"),
    ("NodeName/<locale>.md", "Localized node documentation", "locale-aware native documentation lookup port"),
    ("locale fallback", "Select user locale then fallback document", "deterministic locale fallback port"),
    ("Markdown/media allowlist", "Markdown plus constrained video/source attributes", "sanitized native Markdown/media renderer"),
    ("/docs static route", "Conditionally serve installed package documentation", "native embedded resource registry; no Python package import"),
]
for name, behavior, native_port in embedded_extension_contracts:
    append_extension(
        "embedded node documentation",
        name,
        "custom-nodes/help_page.mdx; projects/comfy/ComfyUI/app/frontend_management.py:321; server.py:1239",
        "COMFY-API-0090; COMFY-EXT-009",
        behavior,
        native_port,
    )


assert len(extension_rows) == 56
write_csv(
    CATALOGS / "docs-extension-contracts.csv",
    [
        "record_id",
        "family",
        "name",
        "legacy_behavior",
        "source_evidence",
        "corroborated_feature_ids",
        "evidence_level",
        "availability",
        "native_rust_wasm_port",
        "production_legacy_execution",
    ],
    extension_rows,
)


lifecycle_contracts = [
    ("Embedded docs optional load", "ComfyUI imports comfyui_embedded_docs; absence logs and omits /docs", "COMFY-API-0090", "native resource registry reports package/version absence"),
    ("Embedded docs version skew", "ComfyUI pins 0.5.6 while this source snapshot declares 0.5.7", "", "version negotiation and fixture pinning"),
    ("V1 custom-node failure isolation", "Failed Python import does not stop remaining startup", "COMFY-EXT-003", "plugin quarantine with visible error"),
    ("V3 extension on_load", "Async extension initialization precedes node availability", "COMFY-EXT-007", "bounded native/WASM initialization task"),
    ("Node replacement before validation", "Legacy identifiers and ports migrate before prompt validation", "COMFY-EXEC-004; COMFY-EXT-022", "native migration stage"),
    ("Frontend extension init sequence", "init through setup and graph hooks run in documented order", "COMFY-FRONTEXT-EXT-046; COMFY-FRONTEXT-EXT-059", "ordered capability-port lifecycle"),
    ("Translation incremental resume", "Hashes skip unchanged blocks and checkpoint each translated block", "", "documentation infrastructure only"),
    ("Translation truncation recovery", "Detect missing fences/sections and repair or retranslate", "", "documentation infrastructure only"),
    ("Analytics checkpoint resume", "Flush store/checkpoint and resume after interruption or rate errors", "", "documentation infrastructure only"),
    ("CMS staged publish", "Prepare English, translate locales, sync drafts, then explicitly publish", "", "documentation infrastructure only"),
    ("Cloud async job", "Submit, poll/watch, cancel, and collect output", "", "deferred provider contract until executable service evidence"),
    ("CLI local/cloud routing", "Resolve local or cloud with per-call/environment/persisted override", "", "reconcile with executable comfy-cli catalog"),
    ("CLI background server", "Launch in background and stop later", "", "conformance-oracle lifecycle only; production Zed remains native"),
    ("Desktop install and first run", "Platform installation/onboarding pages describe managed lifecycle", "", "reconcile against Desktop executable catalog"),
    ("Desktop snapshots and migration", "Snapshot, rollback, instance management, and migration pages", "", "reconcile against Desktop executable catalog"),
    ("Documentation redirect migration", "65 source paths redirect to current documentation destinations", "", "documentation-site compatibility only"),
    ("Cloud queue compatibility exceptions", "Cloud ignores or changes selected local-compatible fields", "", "documented-only cloud semantic delta"),
    ("Uploaded reference expiration", "CLI docs claim cloud assets delete after 24 hours", "", "documented-only external lifecycle"),
    ("OAuth token refresh", "CLI docs claim short-lived session and on-demand refresh", "", "reconcile against executable comfy-cli source"),
    ("Embedded package publication", "pyproject change or manual dispatch builds, publishes, tags, and releases", "", "repository infrastructure only"),
]
lifecycle_rows = [
    {
        "record_id": stable_id("COMFY-DOC-LIFECYCLE", name),
        "name": name,
        "documented_behavior": behavior,
        "corroborated_feature_ids": feature_ids,
        "native_parity_treatment": native_treatment,
        "evidence_level": "documented-only",
    }
    for name, behavior, feature_ids, native_treatment in lifecycle_contracts
]
assert len(lifecycle_rows) == 20
write_csv(
    CATALOGS / "docs-lifecycle-contracts.csv",
    [
        "record_id",
        "name",
        "documented_behavior",
        "corroborated_feature_ids",
        "native_parity_treatment",
        "evidence_level",
    ],
    lifecycle_rows,
)


docs_tool_ids_by_source: dict[str, list[str]] = defaultdict(list)
for row in tooling_rows:
    source = str(row["source_evidence"]).split(";", 1)[0].split(":", 1)[0]
    docs_tool_ids_by_source[source].append(str(row["record_id"]))


docs_source_rows: list[dict[str, object]] = []
for path in docs_files:
    relative = path.relative_to(DOCS)
    relative_string = relative.as_posix()
    disposition, reason = docs_disposition(relative)
    mapped_records: list[str] = []
    mapped_features: list[str] = []
    if relative_string in docs_page_record_by_path:
        mapped_records.append(docs_page_record_by_path[relative_string])
        page_row = next(row for row in docs_page_rows if row["path"] == relative_string)
        mapped_features.extend(str(page_row["corroborated_feature_ids"]).split("; ") if page_row["corroborated_feature_ids"] else [])
    if relative.parts[0] == "built-in-nodes" and relative.suffix == ".mdx":
        node_row = next(row for row in docs_node_rows if row["path"] == relative_string)
        mapped_records.append(str(node_row["record_id"]))
        mapped_features.extend(str(node_row["corroborated_feature_ids"]).split("; ") if node_row["corroborated_feature_ids"] else [])
    if is_docs_localized(relative) and relative.suffix == ".mdx":
        if relative.parts[0] in locale_roots:
            english_relative = Path(*relative.parts[1:]).as_posix()
        else:
            english_relative = Path("snippets", *relative.parts[2:]).as_posix()
        if english_relative in docs_page_record_by_path:
            mapped_records.append(docs_page_record_by_path[english_relative])
    mapped_records.extend(docs_tool_ids_by_source.get(relative_string, []))
    if relative_string == "openapi-cloud.yaml":
        mapped_records.extend(str(row["record_id"]) for row in openapi_rows)
    docs_source_rows.append(
        {
            "source_record_id": stable_id("COMFY-DOCSRC", relative_string),
            "path": relative_string,
            "disposition": disposition,
            "reason": reason,
            "mapped_record_ids": "; ".join(sorted(set(mapped_records))),
            "mapped_feature_ids": "; ".join(sorted(set(filter(None, mapped_features)))),
            "source_evidence_level": "code-inferred" if disposition in {"executable automation/tooling", "CI workflow", "configuration/schema/lock/registry"} else "documented-only",
        }
    )


write_csv(
    CATALOGS / "docs-source-coverage.csv",
    [
        "source_record_id",
        "path",
        "disposition",
        "reason",
        "mapped_record_ids",
        "mapped_feature_ids",
        "source_evidence_level",
    ],
    docs_source_rows,
)


def embedded_disposition(relative: Path) -> tuple[str, str]:
    if len(relative.parts) >= 4 and relative.parts[:2] == ("comfyui_embedded_docs", "docs") and relative.suffix == ".md":
        if relative.stem == "en":
            return "English node documentation", "English AI-generated node claim source requiring executable corroboration."
        return "localized node documentation", "Generated/localized node documentation; English is the claim source."
    if len(relative.parts) >= 3 and relative.parts[:2] == ("comfyui_embedded_docs", "docs") and relative.suffix in media_suffixes:
        return "node media asset", "Static node-documentation media asset."
    if len(relative.parts) >= 3 and relative.parts[:2] == ("comfyui_embedded_docs", "docs"):
        return "node ancillary asset", "Non-Markdown node-documentation asset."
    if relative.as_posix().startswith(".github/workflows/"):
        return "CI workflow", "Repository lifecycle automation."
    if relative.suffix in {".py", ".sh", ".ps1"}:
        return "executable/package tooling", "Packaging, linting, or validation tooling; not a production node implementation."
    if relative.name in {"pyproject.toml", "MANIFEST.in"}:
        return "package configuration", "Python package metadata retained as source evidence only."
    if relative.suffix == ".md":
        return "governance/tool documentation", "Repository tool documentation."
    return "repository/package infrastructure", "Repository metadata or non-code asset."


embedded_source_rows: list[dict[str, object]] = []
for path in embedded_files:
    relative = path.relative_to(EMBEDDED)
    disposition, reason = embedded_disposition(relative)
    mapped_records: list[str] = []
    mapped_features: list[str] = []
    if len(relative.parts) >= 3 and relative.parts[:2] == ("comfyui_embedded_docs", "docs"):
        node_name = relative.parts[2]
        if node_name in embedded_record_by_name:
            mapped_records.append(embedded_record_by_name[node_name])
            node_row = next(row for row in embedded_node_rows if row["node_document_name"] == node_name)
            mapped_features.extend(str(node_row["corroborated_feature_ids"]).split("; ") if node_row["corroborated_feature_ids"] else [])
    embedded_source_rows.append(
        {
            "source_record_id": stable_id("COMFY-EMBEDSRC", relative.as_posix()),
            "path": relative.as_posix(),
            "disposition": disposition,
            "reason": reason,
            "mapped_record_ids": "; ".join(sorted(set(mapped_records))),
            "mapped_feature_ids": "; ".join(sorted(set(filter(None, mapped_features)))),
            "source_evidence_level": "code-inferred" if disposition in {"CI workflow", "executable/package tooling", "package configuration"} else "documented-only",
        }
    )


write_csv(
    CATALOGS / "embedded-docs-source-coverage.csv",
    [
        "source_record_id",
        "path",
        "disposition",
        "reason",
        "mapped_record_ids",
        "mapped_feature_ids",
        "source_evidence_level",
    ],
    embedded_source_rows,
)


def nav_strings(value: object) -> list[str]:
    result: list[str] = []
    if isinstance(value, str):
        result.append(value)
    elif isinstance(value, list):
        for item in value:
            result.extend(nav_strings(item))
    elif isinstance(value, dict):
        for key in ("pages", "tabs", "groups"):
            if key in value:
                result.extend(nav_strings(value[key]))
    return result


english_site_pages = {
    path.relative_to(DOCS).with_suffix("").as_posix()
    for path in docs_page_paths
    if not path.relative_to(DOCS).as_posix().startswith(".github/")
}
localization_summary: dict[str, object] = {}
for language in ("ja", "zh", "ko"):
    pages = {
        path.relative_to(DOCS / language).with_suffix("").as_posix()
        for path in (DOCS / language).rglob("*.mdx")
    }
    snippets = {
        "snippets/" + path.relative_to(DOCS / "snippets" / language).with_suffix("").as_posix()
        for path in (DOCS / "snippets" / language).rglob("*.mdx")
    }
    actual = pages | snippets
    localization_summary[language] = {
        "page_count": len(pages),
        "snippet_count": len(snippets),
        "matching_english_count": len(actual & english_site_pages),
        "exact_missing": sorted(english_site_pages - actual),
        "exact_extra": sorted(actual - english_site_pages),
    }


language_navigation: dict[str, object] = {}
for language in docs_json["navigation"]["languages"]:
    values = nav_strings(language["tabs"])
    language_navigation[language["language"]] = {
        "tabs": len(language["tabs"]),
        "page_references": len(values),
        "unique_page_references": len(set(values)),
    }

english_navigation_values = set(
    nav_strings(next(language for language in docs_json["navigation"]["languages"] if language["language"] == "en")["tabs"])
)
all_docs_page_keys = {path.relative_to(DOCS).with_suffix("").as_posix() for path in docs_page_paths}
english_navigation_exact_missing = sorted(english_navigation_values - all_docs_page_keys)
english_unlisted_mdx = sorted(all_docs_page_keys - english_navigation_values)
assert len(english_navigation_exact_missing) == 12
assert len(english_unlisted_mdx) == 119


node_match_counts = Counter(row["registry_match_kind"] for row in embedded_node_rows)
sync_counts = Counter(str(row["docs_sync_status"]).split(";", 1)[0] for row in embedded_node_rows)
docs_disposition_counts = Counter(row["disposition"] for row in docs_source_rows)
embedded_disposition_counts = Counter(row["disposition"] for row in embedded_source_rows)
openapi_reconciliation_counts = Counter(row["route_shape_reconciliation"] for row in openapi_rows)
unverified_provider_nodes = sorted(
    str(row["node_document_name"])
    for row in embedded_node_rows
    if row["registry_match_kind"] == "provider-unverified"
)


reconciliation = {
    "baselines": {
        "docs": {
            "declared_version": None,
            "tooling_lock_versions": {
                "mint": "4.2.585",
                "sharp": "0.33.5",
                "playwright_mcp": "1.0.12",
            },
            "source_files": len(docs_files),
            "fingerprint": docs_fingerprint,
            "excluded_runtime_os_artifacts": [".DS_Store"],
        },
        "embedded_docs": {
            "declared_version": "0.5.7",
            "comfyui_pinned_version": "0.5.6",
            "source_files": len(embedded_files),
            "fingerprint": embedded_fingerprint,
            "excluded_runtime_os_artifacts": ["comfyui_embedded_docs/__pycache__/__init__.cpython-312.pyc"],
        },
    },
    "docs": {
        "source_dispositions": dict(sorted(docs_disposition_counts.items())),
        "page_records": len(docs_page_rows),
        "page_roles": dict(sorted(Counter(row["role"] for row in docs_page_rows).items())),
        "english_product_domains": dict(
            sorted(
                Counter(
                    row["domain"]
                    for row in docs_page_rows
                    if row["role"] == "English product page"
                ).items()
            )
        ),
        "redirects": len(redirect_rows),
        "navigation": language_navigation,
        "english_navigation_exact_missing": english_navigation_exact_missing,
        "english_unlisted_mdx": english_unlisted_mdx,
        "localization": localization_summary,
        "tooling": {
            "package_scripts": len(package_json["scripts"]),
            "distinct_static_flag_literals": len(flag_evidence),
            "environment_variables": len(environment_names),
            "ci_workflows": len(list((DOCS / ".github/workflows").glob("*.y*ml"))),
            "tooling_rows": len(tooling_rows),
        },
        "openapi": {
            "openapi_version": "3.0.3",
            "api_info_version": "1.0.0",
            "paths": 34,
            "operations": len(openapi_rows),
            "schemas": 56,
            "security_schemes": 1,
            "tags": 8,
            "route_shape_reconciliation": dict(sorted(openapi_reconciliation_counts.items())),
        },
        "observed_validation": {
            "link_validator_files": 4988,
            "link_validator_result": "pass",
            "bun_tests_passed": 8,
            "bun_tests_failed": 0,
            "translation_truncation_issues": 51,
        },
    },
    "embedded_docs": {
        "source_dispositions": dict(sorted(embedded_disposition_counts.items())),
        "node_directories": len(embedded_node_rows),
        "markdown_locale_files": sum(int(row["locale_count"]) for row in embedded_node_rows),
        "asset_files": sum(int(row["asset_count"]) for row in embedded_node_rows),
        "visual_media_files": sum(int(row["visual_media_count"]) for row in embedded_node_rows),
        "registry_match_counts": dict(sorted(node_match_counts.items())),
        "docs_sync_counts": dict(sorted(sync_counts.items())),
        "docs_path_case_differences": sorted(
            str(row["node_document_name"])
            for row in embedded_node_rows
            if "path-case-differs" in str(row["docs_sync_status"])
        ),
        "provider_unverified_count": len(unverified_provider_nodes),
        "provider_unverified_nodes": unverified_provider_nodes,
        "observed_validation": {
            "local_resource_link_checker": "pass",
        },
    },
    "catalog_counts": {
        "docs_pages": len(docs_page_rows),
        "docs_node_docs": len(docs_node_rows),
        "embedded_docs_nodes": len(embedded_node_rows),
        "docs_openapi_cloud": len(openapi_rows),
        "docs_redirects": len(redirect_rows),
        "docs_tooling": len(tooling_rows),
        "docs_config_formats": len(config_format_rows),
        "docs_extension_contracts": len(extension_rows),
        "docs_lifecycle_contracts": len(lifecycle_rows),
        "docs_source_coverage": len(docs_source_rows),
        "embedded_docs_source_coverage": len(embedded_source_rows),
    },
}
write_json(CATALOGS / "docs-reconciliation.json", reconciliation)


unverified_lines = "\n".join(f"- `{name}`" for name in unverified_provider_nodes)
evidence = f"""# Documentation and embedded-docs evidence

## Baselines

The documentation repository declares no project version. Its package manifest
contains tooling dependencies and scripts only. The canonical fingerprint covers
{len(docs_files):,} files after excluding the discovered `.DS_Store` OS artifact:
`{docs_fingerprint}`. The lockfile resolves Mintlify `mint` 4.2.585, `sharp`
0.33.5, and `@executeautomation/playwright-mcp-server` 1.0.12.

Embedded docs declares version 0.5.7 and has {len(embedded_files):,} source files
after excluding the discovered Python bytecode/cache artifact. Its fingerprint is
`{embedded_fingerprint}`. ComfyUI `requirements.txt` pins
`comfyui-embedded-docs==0.5.6`; therefore the added 0.5.7 tree is a separate,
version-skewed evidence source rather than the exact package pinned by the
ComfyUI snapshot.

## Evidence discipline

English MDX is the documentation source of truth. Translations, CMS staging,
README prose, and generated node help remain `documented-only` unless a catalog
row names executable source or a test. Route-shape or identifier matches do not
corroborate cloud semantics, defaults, errors, billing, retention, or lifecycle.
The production design must never execute Python or JavaScript from these sources.
Legacy extension claims map to versioned Rust/WASM descriptors, explicit host
ports, and legacy identifier/port migrations.

## Docs source coverage

The {len(docs_source_rows):,}-row source ledger is reconciled as follows:

| Disposition | Files |
| --- | ---: |
{chr(10).join(f'| {name} | {count:,} |' for name, count in sorted(docs_disposition_counts.items()))}

The 1,273 non-primary-translation MDX records comprise 896 built-in-node
references, 307 English product pages, 56 English snippets, two English CMS
staging files, and twelve localized CMS staging files. The 307 product pages are
split by domain in `docs-reconciliation.json`; tutorials (139), custom nodes
(36), Registry (31), development (21), interface (19), and installation (16)
are the largest groups.

`docs.json` defines four languages and six tabs per language. It has 65 redirects.
English, Chinese, and Japanese each have 1,166 unique navigation references;
Korean has 1,120. Twelve English CLIP-related navigation paths differ from their
actual filenames only by case, which is a portability risk on case-sensitive
filesystems. English has 119 MDX files not directly listed in navigation: 56
snippets, 24 Registry API pages, 18 built-in-node pages, 14 CMS staging files,
and seven other pages.

Japanese and Chinese each have 1,202 page translations plus 56 snippet
translations and each lacks `tutorials/partner-nodes/ideogram/ideogram-v3`.
Korean has 1,151 pages plus 56 snippets, with 64 exact missing paths and 12
case-only extras. The full path lists are machine-readable.

## Embedded node documentation

All 855 node directories contain the same twelve locale files (`ar`, `en`,
`es`, `fa`, `fr`, `ja`, `ko`, `pt-BR`, `ru`, `tr`, `zh`, `zh-TW`), yielding
10,260 Markdown files, plus 23 visual media assets and one JSON ancillary asset.
Every English file contains the
AI-generated-content marker.

Registry reconciliation is:

| Match | Node documents |
| --- | ---: |
{chr(10).join(f'| {name} | {count:,} |' for name, count in sorted(node_match_counts.items()))}

The 848 embedded records represented in the docs site split into 710 matching
declared source fingerprints, one mismatch (`CreateBoundingBoxes`), and 137
records without comparable fingerprints. Seven embedded records are absent from
the docs site. The docs site additionally contains one overview and 47 nested
legacy/partner node pages.

The following {len(unverified_provider_nodes)} embedded node claims have no
registered backend identifier/class, frontend-native virtual-node registration,
explicit replacement, unregistered executable class, or conditional frontend
consumer corroboration in this baseline:

{unverified_lines}

They remain provider-unverified and `documented-only`; they must not be promoted
to active native nodes without an executable schema or a deliberate compatibility
decision.

## Cloud OpenAPI

`openapi-cloud.yaml` declares OpenAPI 3.0.3 and API info version 1.0.0. It has
34 paths, 42 operations, 56 schemas, one API-key security scheme, and eight tags.
Thirty-nine method/path shapes occur in the backend and/or frontend executable
catalogs. The uncorroborated shapes are `GET /api/assets/remote-metadata`,
`POST /api/assets/download`, and `PUT /api/assets/{{id}}`. All cloud behavior
remains experimental, cloud/paid, and documented-only even where a route shape
matches.

## Commands, configuration, formats, lifecycle, and extensions

The tooling catalog has 108 rows: 28 package scripts, 41 distinct static tool
flag literals, 30 tooling environment variables, and nine CI workflows. These
are developer/infrastructure behavior, not production Zed flags. The
configuration/format catalog has 20 source configuration/schema/lock/registry
files and 15 documented format contracts.

The extension catalog has 56 contracts: seven Python V1 legacy contracts,
seventeen Python V3 legacy contracts, all 27 executable frontend extension
interface members, and five embedded-documentation contracts. Each row names a
native Rust/WASM port and marks production legacy execution prohibited. The
lifecycle ledger has 20 separately testable documented transitions, including
load failure, replacement ordering, async jobs, interrupted tooling recovery,
version skew, token/asset expiry claims, redirects, and package publication.

## Observed validation

- `python3 .github/scripts/validate-links.py --check`: 4,988 documentation
  files checked, pass.
- `bun test ./.github/scripts/i18n/chunked-translate.test.ts
  ./.github/scripts/i18n/repair-fences.test.ts`: 8 pass, 0 fail, 17 assertions.
- `bun .github/scripts/i18n/check-translation-truncation.ts`: 51 issues
  observed. The checker-generated gitignored reports were removed, and the
  canonical fingerprint was reverified.
- `python3 .github/scripts/check_md_links.py` in embedded docs: all local
  resource links pass.

## Generated catalogs

- `catalogs/docs-pages.csv`
- `catalogs/docs-node-docs.csv`
- `catalogs/embedded-docs-nodes.csv`
- `catalogs/docs-openapi-cloud.csv`
- `catalogs/docs-redirects.csv`
- `catalogs/docs-tooling.csv`
- `catalogs/docs-config-formats.csv`
- `catalogs/docs-extension-contracts.csv`
- `catalogs/docs-lifecycle-contracts.csv`
- `catalogs/docs-source-coverage.csv`
- `catalogs/embedded-docs-source-coverage.csv`
- `catalogs/docs-reconciliation.json`
"""
(SPEC_DIR / "evidence-documentation.md").write_text(evidence, encoding="utf-8")


generated_paths = [
    CATALOGS / "docs-pages.csv",
    CATALOGS / "docs-node-docs.csv",
    CATALOGS / "embedded-docs-nodes.csv",
    CATALOGS / "docs-openapi-cloud.csv",
    CATALOGS / "docs-redirects.csv",
    CATALOGS / "docs-tooling.csv",
    CATALOGS / "docs-config-formats.csv",
    CATALOGS / "docs-extension-contracts.csv",
    CATALOGS / "docs-lifecycle-contracts.csv",
    CATALOGS / "docs-source-coverage.csv",
    CATALOGS / "embedded-docs-source-coverage.csv",
    CATALOGS / "docs-reconciliation.json",
    SPEC_DIR / "evidence-documentation.md",
]
for generated_path in generated_paths:
    digest = hashlib.sha256(generated_path.read_bytes()).hexdigest()
    print(f"{generated_path.relative_to(SPEC_DIR)}\t{digest}")
