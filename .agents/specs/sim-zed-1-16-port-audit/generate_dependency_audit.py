#!/usr/bin/env python3

import argparse
import json
import pathlib
import subprocess
import sys
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[3]
OUTPUT = pathlib.Path(__file__).resolve().parent / "dependency-audit.md"
UPSTREAM = "eb8e1c8b5502b7007465fbbc465f4a736fa39210"
DEPENDENCY_TABLES = {"dependencies", "dev-dependencies", "build-dependencies"}


def run_git(*arguments: str) -> bytes:
    return subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout


def load_current(path: str) -> dict | None:
    full_path = ROOT / path
    if not full_path.exists():
        return None
    with full_path.open("rb") as file:
        return tomllib.load(file)


def load_upstream(path: str) -> dict:
    return tomllib.loads(run_git("show", f"{UPSTREAM}:{path}").decode())


def collect_entries(document: dict) -> dict[tuple[str, ...], object]:
    entries: dict[tuple[str, ...], object] = {}

    def visit(value: object, path: tuple[str, ...]) -> None:
        if not isinstance(value, dict):
            return
        for key, child in value.items():
            child_path = (*path, key)
            if key in DEPENDENCY_TABLES and isinstance(child, dict):
                for dependency, declaration in child.items():
                    entries[(*child_path, dependency)] = declaration
            elif path == ("patch",) and isinstance(child, dict):
                for dependency, declaration in child.items():
                    entries[(*child_path, dependency)] = declaration
            else:
                visit(child, child_path)

    visit(document, ())
    return entries


def format_declaration(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def contains_disallowed_fork(value: object) -> bool:
    return "github.com/simtropolis" in format_declaration(value) or '"package":"sim-' in format_declaration(value)


def external_lock_packages(lock: dict) -> set[tuple[str, str, str]]:
    return {
        (package["name"], package["version"], package["source"])
        for package in lock.get("package", [])
        if package.get("source") is not None
    }


def format_lock_package(package: tuple[str, str, str]) -> str:
    name, version, source = package
    return f"`{name}` `{version}` — `{source}`"


def build_report() -> tuple[str, int]:
    manifest_paths = sorted(
        path
        for path in run_git("ls-tree", "-r", "--name-only", UPSTREAM).decode().splitlines()
        if path == "Cargo.toml" or path.endswith("/Cargo.toml")
    )

    exact = 0
    drift: list[tuple[str, tuple[str, ...], object, object]] = []
    missing: list[tuple[str, tuple[str, ...], object]] = []
    additions: list[tuple[str, tuple[str, ...], object]] = []
    fork_additions: list[tuple[str, tuple[str, ...], object]] = []
    missing_manifests: list[str] = []

    for manifest_path in manifest_paths:
        upstream_entries = collect_entries(load_upstream(manifest_path))
        current_document = load_current(manifest_path)
        if current_document is None:
            missing_manifests.append(manifest_path)
            continue
        current_entries = collect_entries(current_document)

        for entry_path, upstream_declaration in upstream_entries.items():
            if entry_path not in current_entries:
                missing.append((manifest_path, entry_path, upstream_declaration))
                continue
            current_declaration = current_entries[entry_path]
            if current_declaration == upstream_declaration:
                exact += 1
            else:
                drift.append(
                    (manifest_path, entry_path, upstream_declaration, current_declaration)
                )

        for entry_path in sorted(current_entries.keys() - upstream_entries.keys()):
            declaration = current_entries[entry_path]
            row = (manifest_path, entry_path, declaration)
            additions.append(row)
            if contains_disallowed_fork(declaration):
                fork_additions.append(row)

    current_manifest_paths = sorted(
        path
        for path in run_git("ls-files", "*Cargo.toml").decode().splitlines()
        if path == "Cargo.toml" or path.endswith("/Cargo.toml")
    )
    upstream_manifest_set = set(manifest_paths)
    new_manifests = [path for path in current_manifest_paths if path not in upstream_manifest_set]
    for manifest_path in new_manifests:
        document = load_current(manifest_path)
        if document is None:
            continue
        for entry_path, declaration in collect_entries(document).items():
            if contains_disallowed_fork(declaration):
                fork_additions.append((manifest_path, entry_path, declaration))

    with (ROOT / "Cargo.lock").open("rb") as file:
        current_lock_packages = external_lock_packages(tomllib.load(file))
    upstream_lock_packages = external_lock_packages(
        tomllib.loads(run_git("show", f"{UPSTREAM}:Cargo.lock").decode())
    )
    preserved_lock_packages = upstream_lock_packages & current_lock_packages
    missing_lock_packages = sorted(upstream_lock_packages - current_lock_packages)
    added_lock_packages = sorted(current_lock_packages - upstream_lock_packages)
    lock_forks = sorted(
        package for package in current_lock_packages if "github.com/simtropolis" in package[2]
    )

    lines = [
        "# Dependency reconciliation audit",
        "",
        f"Upstream authority: `{UPSTREAM}` (`v1.16.1`).",
        "",
        "The comparison covers dependency, development-dependency, build-dependency,",
        "target dependency, and Cargo patch declarations in every manifest present at",
        "v1.16.1. Package repository metadata is not a dependency declaration.",
        "",
        "| Result | Count |",
        "| --- | ---: |",
        f"| Exact upstream declarations | {exact} |",
        f"| Drifted upstream declarations | {len(drift)} |",
        f"| Missing upstream declarations | {len(missing)} |",
        f"| Missing upstream manifests | {len(missing_manifests)} |",
        f"| Sim additions in existing manifests | {len(additions)} |",
        f"| New Sim manifests | {len(new_manifests)} |",
        f"| Unapproved Sim fork declarations | {len(fork_additions)} |",
        f"| Preserved v1.16.1 external lock records | {len(preserved_lock_packages)} |",
        f"| Missing/replaced v1.16.1 external lock records | {len(missing_lock_packages)} |",
        f"| Added external lock records | {len(added_lock_packages)} |",
        f"| Unapproved Sim fork lock records | {len(lock_forks)} |",
        "",
        "## Drifted upstream declarations",
        "",
    ]
    if drift:
        for manifest, entry_path, upstream_value, current_value in drift:
            lines.extend(
                [
                    f"- `{manifest}` / `{' > '.join(entry_path)}`",
                    f"  - v1.16.1: `{format_declaration(upstream_value)}`",
                    f"  - current: `{format_declaration(current_value)}`",
                ]
            )
    else:
        lines.append("None.")

    lines.extend(["", "## Resolved lockfile drift", ""])
    lines.append("### Missing or replaced v1.16.1 external records")
    lines.append("")
    if missing_lock_packages:
        lines.extend(f"- {format_lock_package(package)}" for package in missing_lock_packages)
    else:
        lines.append("None.")
    lines.extend(["", "### Added external records", ""])
    if added_lock_packages:
        lines.extend(f"- {format_lock_package(package)}" for package in added_lock_packages)
    else:
        lines.append("None.")
    lines.extend(["", "### Unapproved Sim fork lock records", ""])
    if lock_forks:
        lines.extend(f"- {format_lock_package(package)}" for package in lock_forks)
    else:
        lines.append("None.")

    lines.extend(["", "## Missing upstream declarations", ""])
    if missing:
        for manifest, entry_path, upstream_value in missing:
            lines.append(
                f"- `{manifest}` / `{' > '.join(entry_path)}`: `{format_declaration(upstream_value)}`"
            )
    else:
        lines.append("None.")

    lines.extend(["", "## Unapproved Sim fork declarations", ""])
    if fork_additions:
        for manifest, entry_path, declaration in fork_additions:
            lines.append(
                f"- `{manifest}` / `{' > '.join(entry_path)}`: `{format_declaration(declaration)}`"
            )
    else:
        lines.append("None.")

    lines.extend(
        [
            "",
            "## Sim additions",
            "",
            "New declarations are retained for review because absence from the upstream",
            "manifest is not itself evidence of an upstream dependency substitution.",
            "",
        ]
    )
    for manifest, entry_path, declaration in additions:
        lines.append(
            f"- `{manifest}` / `{' > '.join(entry_path)}`: `{format_declaration(declaration)}`"
        )
    lines.extend(["", "## New Sim manifests", ""])
    lines.extend(f"- `{path}`" for path in new_manifests)
    lines.append("")

    error_count = (
        len(drift)
        + len(missing)
        + len(missing_manifests)
        + len(fork_additions)
        + len(lock_forks)
    )
    return "\n".join(lines), error_count


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    report, error_count = build_report()
    if arguments.check:
        actual = OUTPUT.read_text() if OUTPUT.exists() else None
        if actual != report:
            print(f"out of date: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        if error_count:
            print(f"dependency reconciliation has {error_count} unresolved findings", file=sys.stderr)
            return 1
        return 0
    OUTPUT.write_text(report)
    print(f"wrote {OUTPUT.relative_to(ROOT)} with {error_count} unresolved findings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
