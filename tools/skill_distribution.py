#!/usr/bin/env python3
"""Sync and check in-repo skill discovery copies against skills/ sources.

Discovery copies live under .agents/skills and .claude/skills. This module
never writes outside the given repo root and never repairs copies during check.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import sys
from collections.abc import Iterable, Sequence
from pathlib import Path

DEFAULT_DEST_RELATIVE_ROOTS = (
    Path(".agents") / "skills",
    Path(".claude") / "skills",
)
SKIP_DIR_NAMES = frozenset({"__pycache__"})
SKIP_SUFFIXES = frozenset({".pyc", ".pyo"})


class SkillDistributionError(Exception):
    """Invalid destination or missing skill source."""


def _norm(path: Path) -> str:
    return os.path.normcase(os.path.normpath(str(path)))


def is_inside(path: Path, root: Path) -> bool:
    try:
        Path(_norm(path)).relative_to(Path(_norm(root)))
        return True
    except ValueError:
        return False


def resolve_existing(path: Path) -> Path:
    """Resolve *path*, keeping missing trailing parts after the first existing ancestor."""
    current = path
    missing: list[str] = []
    while not current.exists():
        if current.parent == current:
            break
        missing.append(current.name)
        current = current.parent
    resolved = current.resolve()
    for part in reversed(missing):
        resolved /= part
    return resolved


def _copy_ignore(directory: str, names: list[str]) -> list[str]:
    del directory
    ignored: list[str] = []
    for name in names:
        if name in SKIP_DIR_NAMES or Path(name).suffix in SKIP_SUFFIXES:
            ignored.append(name)
    return ignored


def package_dirs(skills_root: Path) -> list[Path]:
    if not skills_root.is_dir():
        raise SkillDistributionError(f"skill source directory missing: {skills_root}")
    packages = [
        path
        for path in sorted(skills_root.iterdir(), key=lambda item: item.name.lower())
        if path.is_dir() and (path / "SKILL.md").is_file()
    ]
    if not packages:
        raise SkillDistributionError(f"no skill packages with SKILL.md under {skills_root}")
    return packages


def relative_file_hashes(root: Path) -> dict[str, str]:
    hashes: dict[str, str] = {}
    if not root.exists():
        return hashes
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames[:] = sorted(
            name for name in dirnames if name not in SKIP_DIR_NAMES
        )
        base = Path(dirpath)
        for name in sorted(filenames):
            path = base / name
            if path.suffix in SKIP_SUFFIXES:
                continue
            if path.is_symlink() or not path.is_file():
                continue
            rel = path.relative_to(root).as_posix()
            hashes[rel] = hashlib.sha256(path.read_bytes()).hexdigest()
    return hashes


def tree_fingerprint(root: Path) -> str:
    """Stable hash of a tree's relative paths and per-file SHA-256 digests."""
    digest = hashlib.sha256()
    items = relative_file_hashes(root)
    for rel in sorted(items):
        digest.update(rel.encode("utf-8"))
        digest.update(b"\0")
        digest.update(items[rel].encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def default_dest_roots(repo: Path) -> list[Path]:
    return [repo / rel for rel in DEFAULT_DEST_RELATIVE_ROOTS]


def validate_dest_root(repo: Path, dest_root: Path, source_root: Path) -> Path:
    repo_r = repo.resolve()
    source_r = source_root.resolve()
    requested = dest_root if dest_root.is_absolute() else repo / dest_root
    resolved = resolve_existing(requested)
    if not is_inside(resolved, repo_r) or _norm(resolved) == _norm(repo_r):
        raise SkillDistributionError(f"refusing dest outside this repo: {resolved}")
    if _norm(resolved) == _norm(source_r) or is_inside(resolved, source_r):
        raise SkillDistributionError(
            f"refusing dest that would overwrite skill source: {resolved}"
        )
    return resolved


def validate_skill_dest(
    repo: Path, source_pkg: Path, dest: Path, dest_root: Path
) -> Path:
    dest_resolved = resolve_existing(dest)
    expected = resolve_existing(dest_root / source_pkg.name)
    if _norm(dest_resolved) != _norm(expected):
        raise SkillDistributionError(
            f"dest is not the intended skill directory {expected}: {dest_resolved}"
        )
    if dest_resolved.name != source_pkg.name:
        raise SkillDistributionError(
            f"dest name {dest_resolved.name!r} does not match package {source_pkg.name!r}"
        )
    repo_r = repo.resolve()
    source_r = source_pkg.resolve()
    if not is_inside(dest_resolved, repo_r):
        raise SkillDistributionError(f"refusing dest outside this repo: {dest_resolved}")
    if _norm(dest_resolved) == _norm(source_r) or is_inside(dest_resolved, source_r):
        raise SkillDistributionError(
            f"refusing dest that would overwrite skill source: {dest_resolved}"
        )
    return dest_resolved


def _compare_package(source: Path, dest: Path) -> list[str]:
    src = relative_file_hashes(source)
    dst = relative_file_hashes(dest)
    problems: list[str] = []
    if not dest.exists():
        problems.append(f"missing copy: {dest}")
    missing = sorted(set(src) - set(dst))
    extra = sorted(set(dst) - set(src))
    mismatch = sorted(
        rel for rel in set(src) & set(dst) if src[rel] != dst[rel]
    )
    if missing:
        problems.append("missing: " + ", ".join(missing))
    if extra:
        problems.append("extra: " + ", ".join(extra))
    if mismatch:
        problems.append("mismatch: " + ", ".join(mismatch))
    return problems


def check_skills(repo: Path, dest_roots: Sequence[Path] | None = None) -> int:
    """Read-only comparison. Never creates, deletes, or repairs copies."""
    repo = repo.resolve()
    source_root = repo / "skills"
    try:
        packages = package_dirs(source_root)
    except SkillDistributionError as error:
        print(error, file=sys.stderr)
        return 1
    roots = list(dest_roots) if dest_roots is not None else default_dest_roots(repo)
    failed = 0
    for dest_root in roots:
        requested = dest_root if dest_root.is_absolute() else repo / dest_root
        resolved_root = resolve_existing(requested)
        if not is_inside(resolved_root, repo) or _norm(resolved_root) == _norm(repo):
            print(f"check dest is outside this repo: {resolved_root}", file=sys.stderr)
            failed = 1
            continue
        for package in packages:
            dest = resolved_root / package.name
            problems = _compare_package(package, dest)
            if not problems:
                print(f"skill check ok: {dest}")
                continue
            failed = 1
            print(f"skill check failed: {dest}", file=sys.stderr)
            for problem in problems:
                print(f"  {problem}", file=sys.stderr)
    return failed


def _replace_skill_dir(source: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    staging = dest.parent / f".{dest.name}.installing"
    backup = dest.parent / f".{dest.name}.bak"
    for temp in (staging, backup):
        if temp.exists():
            shutil.rmtree(temp)
    shutil.copytree(source, staging, ignore=_copy_ignore, symlinks=False)
    if not (staging / "SKILL.md").is_file():
        shutil.rmtree(staging, ignore_errors=True)
        raise SkillDistributionError(f"skill install missing SKILL.md: {staging}")
    try:
        if dest.exists():
            dest.replace(backup)
        staging.replace(dest)
    except OSError:
        if not dest.exists() and backup.exists():
            backup.replace(dest)
        raise
    if backup.exists():
        shutil.rmtree(backup)


def install_skills(repo: Path, dest_roots: Sequence[Path] | None = None) -> None:
    """Idempotent in-repo sync. Validates every dest before replacing any skill dir."""
    repo = repo.resolve()
    source_root = repo / "skills"
    packages = package_dirs(source_root)
    roots = list(dest_roots) if dest_roots is not None else default_dest_roots(repo)
    jobs: list[tuple[Path, Path]] = []
    for dest_root in roots:
        validated_root = validate_dest_root(repo, dest_root, source_root)
        for package in packages:
            dest = validated_root / package.name
            validate_skill_dest(repo, package, dest, validated_root)
            jobs.append((package, dest))
    for package, dest in jobs:
        _replace_skill_dir(package, dest)
        print(f"installed skill: {dest}")


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Install or check in-repo skill discovery copies."
    )
    parser.add_argument("command", choices=("check", "install"))
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root that owns skills/ (default: cwd).",
    )
    args = parser.parse_args(None if argv is None else list(argv))
    repo = Path(args.repo_root).expanduser().resolve()
    try:
        if args.command == "check":
            return check_skills(repo)
        install_skills(repo)
        return 0
    except SkillDistributionError as error:
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
