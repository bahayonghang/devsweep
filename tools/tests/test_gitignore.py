"""Generated and local-only paths must stay ignored by the root .gitignore."""

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
    ".env",
)

TRACKED = (
    "tools/skill_distribution.py",
    "tools/tests/test_gitignore.py",
    "crates/devsweep-core/src/lib.rs",
    ".trellis/spec/backend/quality-guidelines.md",
    "skills/devsweep-inspect/SKILL.md",
    "desktop/src/api/types.gen.ts",
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

    def test_source_paths_are_not_ignored(self) -> None:
        for path in TRACKED:
            with self.subTest(path=path):
                result = _git("check-ignore", "-v", "--no-index", "--", path)
                self.assertNotEqual(
                    result.returncode,
                    0,
                    msg=f"{path} must remain trackable; check-ignore said {result.stdout!r}",
                )
