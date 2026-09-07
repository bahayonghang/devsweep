"""Release-entry, metadata version, and archive-smoke contract checks."""

from __future__ import annotations

import json
import re
import shutil
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
JUSTFILE = REPO_ROOT / "justfile"
WORKSPACE_VERSION = "0.3.0"

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


def workspace_version(root: Path) -> str:
    with (root / "Cargo.toml").open("rb") as handle:
        cargo = tomllib.load(handle)
    return str(cargo["workspace"]["package"]["version"])


def metadata_versions(root: Path) -> dict[str, str]:
    package = json.loads((root / "desktop" / "package.json").read_text(encoding="utf-8"))
    lock = json.loads(
        (root / "desktop" / "package-lock.json").read_text(encoding="utf-8")
    )
    tauri = json.loads(
        (root / "desktop" / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8")
    )
    lock_pkg = (lock.get("packages") or {}).get("", {})
    return {
        "workspace": workspace_version(root),
        "package.json": str(package["version"]),
        "package-lock.json": str(lock.get("version")),
        'package-lock.packages[""]': str(lock_pkg.get("version")),
        "tauri.conf.json": str(tauri["version"]),
    }


def check_metadata_versions(root: Path) -> int:
    """Return 0 when desktop metadata versions match the workspace version."""
    versions = metadata_versions(root)
    expected = versions["workspace"]
    if not expected:
        return 1
    for value in versions.values():
        if value != expected:
            return 1
    return 0


def _copy_version_tree(dest: Path) -> None:
    desktop = dest / "desktop"
    tauri_dir = desktop / "src-tauri"
    tauri_dir.mkdir(parents=True)
    shutil.copy2(REPO_ROOT / "Cargo.toml", dest / "Cargo.toml")
    shutil.copy2(REPO_ROOT / "desktop" / "package.json", desktop / "package.json")
    shutil.copy2(
        REPO_ROOT / "desktop" / "package-lock.json",
        desktop / "package-lock.json",
    )
    shutil.copy2(
        REPO_ROOT / "desktop" / "src-tauri" / "tauri.conf.json",
        tauri_dir / "tauri.conf.json",
    )


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


class TestMetadataVersions(unittest.TestCase):
    def test_four_metadata_versions_equal_workspace_0_3_0(self) -> None:
        versions = metadata_versions(REPO_ROOT)
        self.assertEqual(versions["workspace"], WORKSPACE_VERSION)
        self.assertEqual(check_metadata_versions(REPO_ROOT), 0)
        for name, value in versions.items():
            self.assertEqual(value, WORKSPACE_VERSION, name)

    def test_mutated_isolated_copy_fails_version_check(self) -> None:
        with tempfile.TemporaryDirectory(prefix="devsweep-release-versions-") as tmp:
            root = Path(tmp)
            _copy_version_tree(root)
            self.assertEqual(check_metadata_versions(root), 0)
            package_path = root / "desktop" / "package.json"
            package = json.loads(package_path.read_text(encoding="utf-8"))
            package["version"] = "0.3.1"
            package_path.write_text(
                json.dumps(package, indent=2) + "\n",
                encoding="utf-8",
            )
            self.assertNotEqual(check_metadata_versions(root), 0)
        self.assertEqual(check_metadata_versions(REPO_ROOT), 0)
        self.assertEqual(
            metadata_versions(REPO_ROOT)["package.json"],
            WORKSPACE_VERSION,
        )


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
    if len(sys.argv) > 1 and sys.argv[1] == "--check":
        raise SystemExit(check_metadata_versions(REPO_ROOT))
    unittest.main()
