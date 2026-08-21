#!/usr/bin/env python3

import argparse
import collections
import csv
import io
import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[3]
OUTPUT_DIRECTORY = pathlib.Path(__file__).resolve().parent
DEFAULT_OLD_BASE = "adc60ccf12e199b8828bad3abb2591e147034734"
DEFAULT_OLD_TIP = "d41ad2b582bceb6b1b49eb68f877ebed7d68eeb2"
DEFAULT_NEW_BASE = "eb8e1c8b5502b7007465fbbc465f4a736fa39210"
DEFAULT_NEW_TIP = "5ab1c4de4a35e61476ff3cb88a5bcf7d9354d35e"


def run_git(*arguments: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def resolve_commit(reference: str) -> str:
    result = run_git("rev-parse", "--verify", f"{reference}^{{commit}}")
    return result.stdout.decode().strip()


def read_tree(commit: str) -> dict[str, tuple[str, str, str]]:
    result = run_git("ls-tree", "-r", "-z", commit)
    entries: dict[str, tuple[str, str, str]] = {}
    for record in result.stdout.split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, object_type, object_id = metadata.decode().split(" ", 2)
        path = raw_path.decode("utf-8", errors="surrogateescape")
        entries[path] = (mode, object_type, object_id)
    return entries


def change_status(
    base_entry: tuple[str, str, str] | None,
    tip_entry: tuple[str, str, str] | None,
) -> str:
    if base_entry is None:
        return "added"
    if tip_entry is None:
        return "deleted"
    return "modified"


def entry_id(entry: tuple[str, str, str] | None) -> str:
    return "" if entry is None else entry[2]


def top_level(path: str) -> str:
    return path.split("/", 1)[0]


def build_outputs(old_base: str, old_tip: str, new_base: str, new_tip: str) -> tuple[str, str]:
    old_base_tree = read_tree(old_base)
    old_tip_tree = read_tree(old_tip)
    new_base_tree = read_tree(new_base)
    new_tip_tree = read_tree(new_tip)

    old_paths = {
        path
        for path in old_base_tree.keys() | old_tip_tree.keys()
        if old_base_tree.get(path) != old_tip_tree.get(path)
    }
    new_paths = {
        path
        for path in new_base_tree.keys() | new_tip_tree.keys()
        if new_base_tree.get(path) != new_tip_tree.get(path)
    }

    rows: list[dict[str, str]] = []
    for path in sorted(old_paths):
        old_base_entry = old_base_tree.get(path)
        old_tip_entry = old_tip_tree.get(path)
        new_base_entry = new_base_tree.get(path)
        new_tip_entry = new_tip_tree.get(path)

        if old_tip_entry == new_tip_entry:
            disposition = "deleted_intentionally" if old_tip_entry is None else "preserved_exactly"
        elif path in new_paths:
            disposition = "ported_with_adaptation"
        else:
            disposition = "missing_unresolved"

        rows.append(
            {
                "path": path,
                "old_status": change_status(old_base_entry, old_tip_entry),
                "new_status": (
                    change_status(new_base_entry, new_tip_entry) if path in new_paths else "unchanged"
                ),
                "disposition": disposition,
                "old_base_object": entry_id(old_base_entry),
                "old_tip_object": entry_id(old_tip_entry),
                "new_base_object": entry_id(new_base_entry),
                "new_tip_object": entry_id(new_tip_entry),
            }
        )

    for path in sorted(new_paths - old_paths):
        new_base_entry = new_base_tree.get(path)
        new_tip_entry = new_tip_tree.get(path)
        rows.append(
            {
                "path": path,
                "old_status": "unchanged",
                "new_status": change_status(new_base_entry, new_tip_entry),
                "disposition": "rebased_only",
                "old_base_object": entry_id(old_base_tree.get(path)),
                "old_tip_object": entry_id(old_tip_tree.get(path)),
                "new_base_object": entry_id(new_base_entry),
                "new_tip_object": entry_id(new_tip_entry),
            }
        )

    csv_buffer = io.StringIO()
    fieldnames = [
        "path",
        "old_status",
        "new_status",
        "disposition",
        "old_base_object",
        "old_tip_object",
        "new_base_object",
        "new_tip_object",
    ]
    writer = csv.DictWriter(csv_buffer, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)

    disposition_counts = collections.Counter(row["disposition"] for row in rows)
    area_counts: dict[str, collections.Counter[str]] = collections.defaultdict(collections.Counter)
    for row in rows:
        area_counts[top_level(row["path"])][row["disposition"]] += 1

    ancestry_result = run_git("merge-base", "--is-ancestor", old_base, old_tip, check=False)
    old_has_ancestry = ancestry_result.returncode == 0
    new_has_ancestry = run_git(
        "merge-base", "--is-ancestor", new_base, new_tip, check=False
    ).returncode == 0

    lines = [
        "# Sim Zed 1.16 port inventory",
        "",
        "## Comparison refs",
        "",
        f"- Old base: `{old_base}` (`v1.10.2`)",
        f"- Old Sim tip: `{old_tip}` (`sim-dev` at audit start)",
        f"- New base: `{new_base}` (`v1.16.1`)",
        f"- Rebased tip: `{new_tip}` (`sim-dev-reparented` at audit start)",
        "",
        "## History limitation",
        "",
    ]
    if old_has_ancestry:
        lines.append("The old base is an ancestor of the old tip; commit-range evidence is available.")
    else:
        lines.extend(
            [
                "The old base is **not** an ancestor of the old Sim tip, and the refs have no",
                "merge base. A literal `git range-diff v1.10.2..sim-dev` therefore describes",
                "the Sim repository's independent root history rather than a trustworthy port",
                "series. This inventory compares the two endpoint trees instead.",
            ]
        )
    lines.extend(
        [
            "",
            f"The new base ancestry check is `{'valid' if new_has_ancestry else 'invalid'}`.",
            "",
            "## Dispositions",
            "",
            "- `preserved_exactly`: the old and rebased final tree entries are identical.",
            "- `ported_with_adaptation`: both deltas touch the path, but final entries differ.",
            "- `deleted_intentionally`: both final trees omit a path removed by the old delta.",
            "- `missing_unresolved`: the old delta changes the path, the new delta does not, and final entries differ.",
            "- `rebased_only`: only the new delta changes the path.",
            "",
            "| Disposition | Paths |",
            "| --- | ---: |",
        ]
    )
    for disposition in [
        "preserved_exactly",
        "ported_with_adaptation",
        "deleted_intentionally",
        "missing_unresolved",
        "rebased_only",
    ]:
        lines.append(f"| `{disposition}` | {disposition_counts[disposition]} |")

    lines.extend(
        [
            "",
            "## Top-level mapping",
            "",
            "| Area | Exact | Adapted | Deleted | Missing | Rebased only |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for area in sorted(area_counts):
        counts = area_counts[area]
        lines.append(
            f"| `{area}` | {counts['preserved_exactly']} | {counts['ported_with_adaptation']} | "
            f"{counts['deleted_intentionally']} | {counts['missing_unresolved']} | "
            f"{counts['rebased_only']} |"
        )

    unresolved = [row["path"] for row in rows if row["disposition"] == "missing_unresolved"]
    lines.extend(["", "## Missing or unresolved paths", ""])
    if unresolved:
        lines.extend(f"- `{path}`" for path in unresolved)
    else:
        lines.append("None.")
    lines.extend(
        [
            "",
            "The complete path mapping and all four Git object IDs are in `port-ledger.csv`.",
            "Adapted paths require build, test, specification, or manual evidence before they",
            "can be called behaviorally equivalent.",
            "",
        ]
    )
    return csv_buffer.getvalue(), "\n".join(lines)


def update_or_check(path: pathlib.Path, expected: str, check: bool) -> bool:
    if check:
        actual = path.read_text() if path.exists() else None
        if actual != expected:
            print(f"out of date: {path.relative_to(ROOT)}", file=sys.stderr)
            return False
        return True
    path.write_text(expected)
    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--old-base", default=DEFAULT_OLD_BASE)
    parser.add_argument("--old-tip", default=DEFAULT_OLD_TIP)
    parser.add_argument("--new-base", default=DEFAULT_NEW_BASE)
    parser.add_argument("--new-tip", default=DEFAULT_NEW_TIP)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()

    old_base = resolve_commit(arguments.old_base)
    old_tip = resolve_commit(arguments.old_tip)
    new_base = resolve_commit(arguments.new_base)
    new_tip = resolve_commit(arguments.new_tip)
    ledger, inventory = build_outputs(old_base, old_tip, new_base, new_tip)

    valid = update_or_check(OUTPUT_DIRECTORY / "port-ledger.csv", ledger, arguments.check)
    valid = update_or_check(OUTPUT_DIRECTORY / "inventory.md", inventory, arguments.check) and valid
    return 0 if valid else 1


if __name__ == "__main__":
    raise SystemExit(main())
