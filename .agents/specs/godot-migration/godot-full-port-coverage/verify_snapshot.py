#!/usr/bin/env python3

import argparse
import hashlib
import stat
from pathlib import Path


def git_object_hash(kind: str, data: bytes) -> bytes:
    header = f"{kind} {len(data)}\0".encode()
    return hashlib.sha1(header + data).digest()


def git_tree_hash(path: Path) -> bytes:
    entries = []
    for child in sorted(path.iterdir(), key=lambda item: item.name.encode()):
        if child.is_dir():
            mode = b"40000"
            digest = git_tree_hash(child)
        elif child.is_symlink():
            mode = b"120000"
            digest = git_object_hash("blob", child.readlink().as_posix().encode())
        else:
            executable = child.stat().st_mode & stat.S_IXUSR
            mode = b"100755" if executable else b"100644"
            digest = git_object_hash("blob", child.read_bytes())
        entries.append(mode + b" " + child.name.encode() + b"\0" + digest)
    return git_object_hash("tree", b"".join(entries))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("--manifest", action="store_true")
    parser.add_argument("--index-content", action="store_true")
    args = parser.parse_args()
    if args.manifest:
        for path in sorted(args.source.rglob("*")):
            if not path.is_file():
                continue
            relative_path = path.relative_to(args.source).as_posix()
            data = path.readlink().as_posix().encode() if path.is_symlink() else path.read_bytes()
            if args.index_content and (
                path.suffix in {".bat", ".sln", ".csproj"}
                or relative_path.startswith("misc/msvs/")
            ):
                data = data.replace(b"\r\n", b"\n")
            print(f"{git_object_hash('blob', data).hex()}\t{relative_path}")
        return
    print(git_tree_hash(args.source).hex())


if __name__ == "__main__":
    main()
