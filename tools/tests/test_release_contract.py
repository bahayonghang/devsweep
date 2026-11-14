"""Release-entry and archive-smoke contract checks."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
JUSTFILE = REPO_ROOT / "justfile"

_RECIPE_START = re.compile(
    r"(?m)^(?:\[[^\n]+\]\r?\n)*([A-Za-z0-9_-]+)(?:\s+[A-Za-z0-9_-]+)*:"
)


def just_recipes(text: str) -> dict[str, str]:
    matches = list(_RECIPE_START.finditer(text))
    recipes: dict[str, str] = {}
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        recipes[match.group(1)] = text[match.start() : end]
    return recipes


class TestDevRecipe(unittest.TestCase):
    def test_dev_recipe_has_no_tui_argument(self) -> None:
        recipes = just_recipes(JUSTFILE.read_text(encoding="utf-8"))
        self.assertIn("dev", recipes)
        body = recipes["dev"]
        self.assertIn(
            "cargo run --locked -p devsweep-cli --bin devsweep",
            body,
        )
        self.assertNotRegex(body, r"\btui\b")
        self.assertNotIn("--help", body)


class TestReleaseSmokeRecipe(unittest.TestCase):
    def test_release_smoke_uses_scan_plan_preview_not_legacy(self) -> None:
        recipes = just_recipes(JUSTFILE.read_text(encoding="utf-8"))
        self.assertIn("release-smoke", recipes)
        body = recipes["release-smoke"]
        self.assertIn("clean scan", body)
        self.assertIn("clean plan", body)
        self.assertIn("clean preview", body)
        self.assertIn("--scope", body)
        self.assertIn("projects", body)
        self.assertIn("devsweep.exe", body)
        self.assertIn("$LASTEXITCODE", body)
        self.assertIn("finally", body)
        self.assertNotIn("scan --json", body)
        self.assertNotIn("clean --plan", body)
        self.assertNotIn("clean execute", body)
        self.assertNotIn("--execute", body)
        self.assertNotIn("software uninstall", body)
        self.assertNotIn("optimize run", body)
        self.assertNotIn("empty-plan", body)
        self.assertNotRegex(body, r"(?m)& \$bin(?:\.FullName)? --help\b")


if __name__ == "__main__":
    unittest.main()
