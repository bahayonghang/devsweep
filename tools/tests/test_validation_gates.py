"""Regression checks for the local validation-gate recipes."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
JUSTFILE = REPO_ROOT / "justfile"


class TestLockFidelity(unittest.TestCase):
    def test_ci_recipe_does_not_depend_on_sync_lock(self) -> None:
        text = JUSTFILE.read_text(encoding="utf-8")
        self.assertRegex(text, r"(?m)^sync-lock:\s*$")
        match = re.search(r"(?m)^ci:(.*)$", text)
        self.assertIsNotNone(match, "justfile is missing a ci recipe")
        deps = match.group(1).split()
        self.assertNotIn("sync-lock", deps)
        self.assertEqual(deps, ["fmt", "check", "test", "clippy"])


if __name__ == "__main__":
    unittest.main()
