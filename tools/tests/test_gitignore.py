"""Generated and local-only paths must stay ignored by a repository .gitignore."""

from __future__ import annotations

import shutil
import subprocess
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

IGNORED = (
    "target/debug/devsweep.exe",
    "desktop/src-tauri/target/release/bundle/nsis/setup.exe",
    "dist/devsweep-x86_64-pc-windows-msvc.zip",
    "desktop/dist/index.html",
    "node_modules/.package-lock.json",
    "desktop/node_modules/react/index.js",
    "desktop/src-tauri/gen/schemas/desktop-schema.json",
    "docs/.vitepress/cache/deps.json",
    "docs/.vitepress/dist/index.html",
    "docs/.vitepress/.temp/foo",
    "tools/__pycache__/skill_distribution.cpython-314.pyc",
    "tools/tests/__pycache__/test_gitignore.cpython-314.pyc",
    ".worktrees/feature-x/.git",
    ".DS_Store",
    "Thumbs.db",
    "ehthumbs.db",
    "desktop.ini",
    ".env",
    ".env.production",
    "desktop/.env.local",
    ".ruff_cache/0.16.8/foo",
    "crates/devsweep-core/src/lib.rs.bk",
    "devsweep.pdb",
    "rustc-ice-2026-09-23.txt",
    "npm-debug.log",
    "desktop/.eslintcache",
    "desktop/coverage/index.html",
    "desktop/src/api/types.gen.tsbuildinfo",
    ".grok/skills/x",
    ".kimi-code/skills/x",
    ".omp/roles/x",
    ".idea/workspace.xml",
    "notes.swp",
    "crash.stackdump",
    "secret.pfx",
    ".trellis/hooks.local.json",
    ".trellis/tasks/09-20-desktop-operation-performance/evidence/resources/raw-20260923T021341Z/analyze-250k-accounted.json",
    ".trellis/tasks/09-20-desktop-operation-performance/evidence/resources/raw-latest.txt",
)

TRACKED = (
    "tools/skill_distribution.py",
    "tools/tests/test_gitignore.py",
    "crates/devsweep-core/src/lib.rs",
    ".trellis/spec/backend/quality-guidelines.md",
    "skills/devsweep-inspect/SKILL.md",
    "desktop/src/api/types.gen.ts",
    "desktop/.env.fixture",
    ".env.example",
    "Cargo.lock",
    "package-lock.json",
    "desktop/package-lock.json",
    ".trellis/tasks/09-20-desktop-operation-performance/evidence/record.md",
)


def _git(*args: str) -> subprocess.CompletedProcess[str]:
    git = shutil.which("git")
    if git is None:
        raise AssertionError("git executable not found on PATH")
    return subprocess.run(
        [git, *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )


def _matched_pattern(stdout: str) -> str:
    """Return the deciding exclude pattern from `git check-ignore -v`."""
    line = stdout.splitlines()[0]
    source, _pathname = line.split("\t", 1)
    return source.split(":", 2)[2]


class TestGitignore(unittest.TestCase):
    def test_generated_and_local_paths_are_ignored(self) -> None:
        for path in IGNORED:
            with self.subTest(path=path):
                result = _git("check-ignore", "-v", "--no-index", "--", path)
                self.assertEqual(
                    result.returncode,
                    0,
                    msg=(
                        f"{path} must be ignored by a repository .gitignore\n"
                        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                    ),
                )
                self.assertIn(
                    ".gitignore:",
                    result.stdout.replace("\\", "/"),
                    msg=f"{path} was ignored by {result.stdout!r}, not a repo .gitignore",
                )
                self.assertFalse(
                    _matched_pattern(result.stdout).startswith("!"),
                    msg=f"{path} matched a negation, so it is trackable: {result.stdout!r}",
                )

    def test_source_paths_are_not_ignored(self) -> None:
        for path in TRACKED:
            with self.subTest(path=path):
                result = _git("check-ignore", "-v", "--no-index", "--", path)
                if result.returncode == 0 and _matched_pattern(result.stdout).startswith("!"):
                    continue
                self.assertNotEqual(
                    result.returncode,
                    0,
                    msg=f"{path} must remain trackable; check-ignore said {result.stdout!r}",
                )
