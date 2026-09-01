#!/usr/bin/env python3
"""Resolve the globally installed DevSweep binary. Refuse repo-local builds."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path


def is_repo_build(binary: Path, repo_root: Path) -> bool:
    try:
        resolved = binary.resolve()
        root = repo_root.resolve()
    except OSError:
        return False
    try:
        relative = resolved.relative_to(root)
    except ValueError:
        return False
    parts = {part.lower() for part in relative.parts}
    return "target" in parts


def resolve(repo_root: Path) -> Path:
    found = shutil.which("devsweep")
    if found is None:
        raise FileNotFoundError(
            "global devsweep.exe was not found on PATH; install it, do not cargo run this repo"
        )
    binary = Path(found)
    if is_repo_build(binary, repo_root):
        raise FileNotFoundError(
            f"refusing repo-local DevSweep binary: {binary}. Use the PATH install outside this checkout."
        )
    return binary


def main() -> int:
    parser = argparse.ArgumentParser(description="Print the globally installed DevSweep executable.")
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root that must not supply the binary (default: cwd).",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON instead of a bare path.")
    args = parser.parse_args()
    repo_root = Path(args.repo_root)
    try:
        binary = resolve(repo_root)
    except FileNotFoundError as error:
        payload = {"ok": False, "error": str(error)}
        print(json.dumps(payload, ensure_ascii=False) if args.json else str(error))
        return 2
    if args.json:
        print(json.dumps({"ok": True, "path": str(binary)}, ensure_ascii=False))
    else:
        print(binary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
