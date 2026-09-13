"""Isolated skill source/copy sync, drift, and dest-boundary checks."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType

from test_release_contract import JUSTFILE, just_recipes

REPO_ROOT = Path(__file__).resolve().parents[2]
OLD_VERSION_TEXT = (
    "Locate the DevSweep program. In this repo use "
    "cargo run --locked -p devsweep-cli --bin devsweep --. "
    "Prefer cargo run this repo when testing cleanup.\n"
)


def _load_skill_distribution() -> ModuleType:
    path = REPO_ROOT / "tools" / "skill_distribution.py"
    spec = importlib.util.spec_from_file_location("skill_distribution", path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


sd = _load_skill_distribution()


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class IsolatedSkillRepo:
    def __init__(self, tmp: Path) -> None:
        self.root = tmp
        self.source = tmp / "skills" / "demo-skill"
        _write(self.source / "SKILL.md", "demo skill v2\nuse PATH global binary\n")
        _write(self.source / "scripts" / "helper.py", "print('ok')\n")

    def dest(self, relative_root: str, filename: str = "SKILL.md") -> Path:
        return self.root / relative_root / "demo-skill" / filename


class TestSkillDistribution(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="devsweep-skill-dist-")
        self.repo = IsolatedSkillRepo(Path(self._tmp.name))
        self.assertNotEqual(self.repo.root.resolve(), REPO_ROOT)
        self.assertFalse(
            str(self.repo.root.resolve()).startswith(str(REPO_ROOT / ".agents"))
        )
        self.assertFalse(
            str(self.repo.root.resolve()).startswith(str(REPO_ROOT / ".claude"))
        )

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _install(self, dest_roots: list[Path] | None = None) -> None:
        sd.install_skills(self.repo.root, dest_roots=dest_roots)

    def _check(self, dest_roots: list[Path] | None = None) -> int:
        return sd.check_skills(self.repo.root, dest_roots=dest_roots)

    def test_matching_trees_pass_check(self) -> None:
        self._install()
        self.assertEqual(self._check(), 0)
        for relative in (".agents/skills", ".claude/skills"):
            dest = self.repo.root / relative / "demo-skill"
            self.assertEqual(
                sd.relative_file_hashes(self.repo.source),
                sd.relative_file_hashes(dest),
            )

    def test_old_version_copy_fails_and_does_not_repair(self) -> None:
        self._install()
        target = self.repo.dest(".agents/skills")
        target.write_text(OLD_VERSION_TEXT, encoding="utf-8")
        self.assertNotEqual(self._check(), 0)
        self.assertEqual(target.read_text(encoding="utf-8"), OLD_VERSION_TEXT)
        self.assertIn("cargo run this repo", target.read_text(encoding="utf-8"))

    def test_deleted_source_file_in_copy_fails(self) -> None:
        self._install()
        helper = self.repo.dest(".claude/skills", "scripts/helper.py")
        helper.unlink()
        self.assertNotEqual(self._check(), 0)
        self.assertFalse(helper.exists())

    def test_stale_extra_file_in_copy_fails(self) -> None:
        self._install()
        extra = self.repo.dest(".agents/skills", "stale-extra.md")
        extra.write_text("leftover from v1\n", encoding="utf-8")
        self.assertNotEqual(self._check(), 0)
        self.assertTrue(extra.is_file())

    def test_resync_restores_match_and_leaves_source_hash_unchanged(self) -> None:
        self._install()
        source_before = sd.tree_fingerprint(self.repo.source)
        agents_skill = self.repo.root / ".agents" / "skills" / "demo-skill"
        claude_skill = self.repo.root / ".claude" / "skills" / "demo-skill"
        (agents_skill / "SKILL.md").write_text(OLD_VERSION_TEXT, encoding="utf-8")
        (claude_skill / "scripts" / "helper.py").unlink()
        (claude_skill / "stale-extra.md").write_text("old\n", encoding="utf-8")
        _write(self.repo.source / "scripts" / "new_tool.py", "print('new')\n")
        source_with_new = sd.tree_fingerprint(self.repo.source)
        self.assertNotEqual(source_before, source_with_new)
        self.assertNotEqual(self._check(), 0)

        self._install()
        self.assertEqual(self._check(), 0)
        self.assertEqual(sd.tree_fingerprint(self.repo.source), source_with_new)
        for dest in (agents_skill, claude_skill):
            self.assertEqual(
                sd.relative_file_hashes(self.repo.source),
                sd.relative_file_hashes(dest),
            )
            self.assertTrue((dest / "scripts" / "new_tool.py").is_file())
            self.assertFalse((dest / "stale-extra.md").exists())
            self.assertNotIn(
                "cargo run this repo",
                (dest / "SKILL.md").read_text(encoding="utf-8"),
            )

    def test_out_of_repo_dest_is_rejected(self) -> None:
        source_before = sd.tree_fingerprint(self.repo.source)
        with tempfile.TemporaryDirectory(prefix="devsweep-skill-outside-") as outside_raw:
            outside = Path(outside_raw)
            self.assertFalse(sd.is_inside(outside.resolve(), self.repo.root.resolve()))
            mixed = [self.repo.root / ".agents" / "skills", outside]
            with self.assertRaises(sd.SkillDistributionError) as raised:
                self._install(dest_roots=mixed)
            self.assertIn("outside this repo", str(raised.exception))
            self.assertFalse((self.repo.root / ".agents" / "skills" / "demo-skill").exists())
            self.assertFalse((outside / "demo-skill").exists())
        self.assertEqual(sd.tree_fingerprint(self.repo.source), source_before)
        self.assertEqual(
            list((self.repo.root / "skills" / "demo-skill").rglob("SKILL.md")),
            [self.repo.source / "SKILL.md"],
        )

    def test_check_does_not_create_missing_copies(self) -> None:
        agents = self.repo.root / ".agents" / "skills"
        self.assertFalse(agents.exists())
        self.assertNotEqual(self._check(), 0)
        self.assertFalse(agents.exists())

    def test_cli_check_and_install_use_repo_root(self) -> None:
        install = sd.main(["install", "--repo-root", str(self.repo.root)])
        self.assertEqual(install, 0)
        check = sd.main(["check", "--repo-root", str(self.repo.root)])
        self.assertEqual(check, 0)
        (self.repo.root / ".agents" / "skills" / "demo-skill" / "SKILL.md").write_text(
            OLD_VERSION_TEXT,
            encoding="utf-8",
        )
        self.assertNotEqual(sd.main(["check", "--repo-root", str(self.repo.root)]), 0)


class TestJustfileSkillRecipes(unittest.TestCase):
    def test_install_and_check_recipes_call_distribution_tool(self) -> None:
        recipes = just_recipes(JUSTFILE.read_text(encoding="utf-8"))
        install = recipes["install-skill"]
        check = recipes["check-skills"]
        self.assertIn("tools/skill_distribution.py", install.replace("\\", "/"))
        self.assertIn("install", install)
        self.assertIn("--repo-root", install)
        self.assertIn("$LASTEXITCODE", install)
        self.assertIn("tools/skill_distribution.py", check.replace("\\", "/"))
        self.assertRegex(check, r"skill_distribution\.py[^\n]*\bcheck\b")
        self.assertIn("--repo-root", check)
        self.assertIn("$LASTEXITCODE", check)
        self.assertNotIn("Copy-Item", install)
        self.assertNotIn("Remove-Item", check)

    def test_install_all_and_unrelated_recipes_not_expanded(self) -> None:
        recipes = just_recipes(JUSTFILE.read_text(encoding="utf-8"))
        install_all = recipes["install-all"]
        self.assertIn("install tinstall install-skill", install_all)
        self.assertNotIn("check-skills", install_all)
        self.assertNotIn("check-skills", recipes["ci"])
        self.assertNotIn("install-skill", recipes["ci"])
        self.assertNotIn("check-skills", recipes["dev"])
        self.assertNotIn("check-skills", recipes["tdev"])
        self.assertNotIn("check-skills", recipes["desktop-web-check"])
        self.assertNotIn("check-skills", recipes["release-archive"])
        self.assertNotIn("check-skills", recipes["release-smoke"])


if __name__ == "__main__":
    sys.exit(unittest.main())
