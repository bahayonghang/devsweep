"""Isolated regressions for local validation-gate failure propagation."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
JUSTFILE = REPO_ROOT / "justfile"
GENERATOR = REPO_ROOT / "desktop" / "scripts" / "generate-types.mjs"
TYPES_GEN = REPO_ROOT / "desktop" / "src" / "api" / "types.gen.ts"
FIXTURES = REPO_ROOT / "desktop" / "src" / "api" / "fixtures"
DESKTOP_NODE_MODULES = REPO_ROOT / "desktop" / "node_modules"

PROBE_FAIL_CODE = 23
NAMED_STEPS = ("types:generate", "lint", "typecheck", "test", "build")
DRIFT_MARKER = b"\n// validation-gates-drift-probe\n"

MOCK_NPM_CMD = """@echo off
echo MOCK npm %*
if "%DEVSWEEP_PROBE_FAIL%"=="" exit /b 0
if "%~1"=="test" if "%DEVSWEEP_PROBE_FAIL%"=="test" exit /b 23
if "%~2"=="%DEVSWEEP_PROBE_FAIL%" exit /b 23
exit /b 0
"""


def _which_or_fail(name: str) -> str:
    found = shutil.which(name)
    if not found:
        raise AssertionError(f"{name} executable not found on PATH")
    return found


def _link_directory(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    try:
        os.symlink(src, dest, target_is_directory=True)
        return
    except OSError:
        pass
    completed = subprocess.run(
        ["cmd", "/c", "mklink", "/J", str(dest), str(src)],
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        detail = (completed.stderr or completed.stdout or "").strip()
        raise AssertionError(f"failed to link {src} -> {dest}: {detail}")


def _run_types_check(desktop_dir: Path) -> subprocess.CompletedProcess[str]:
    node = _which_or_fail("node")
    return subprocess.run(
        [node, str(desktop_dir / "scripts" / "generate-types.mjs"), "--check"],
        cwd=str(desktop_dir),
        capture_output=True,
        text=True,
    )


class TestDesktopWebCheckFailurePropagation(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.just = _which_or_fail("just")

    def _run_desktop_web_check(
        self, probe_root: Path, mock_bin: Path, fail_step: str | None
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["PATH"] = str(mock_bin) + os.pathsep + env.get("PATH", "")
        if fail_step is None:
            env.pop("DEVSWEEP_PROBE_FAIL", None)
        else:
            env["DEVSWEEP_PROBE_FAIL"] = fail_step
        return subprocess.run(
            [
                self.just,
                "--justfile",
                str(JUSTFILE),
                "--working-directory",
                str(probe_root),
                "desktop-web-check",
            ],
            env=env,
            capture_output=True,
            text=True,
        )

    def test_injected_failures_make_entry_nonzero_and_success_is_zero(self) -> None:
        with tempfile.TemporaryDirectory(prefix="devsweep-gate-probe-") as tmp:
            probe = Path(tmp)
            (probe / "desktop").mkdir()
            mock_bin = probe / "mock-bin"
            mock_bin.mkdir()
            (mock_bin / "npm.cmd").write_text(MOCK_NPM_CMD, encoding="ascii")

            success = self._run_desktop_web_check(probe, mock_bin, None)
            self.assertEqual(
                success.returncode,
                0,
                msg=(
                    "all-success desktop-web-check should exit 0\n"
                    f"stdout:\n{success.stdout}\nstderr:\n{success.stderr}"
                ),
            )
            self.assertIn("MOCK npm run types:generate", success.stdout)
            self.assertIn("MOCK npm run lint", success.stdout)
            self.assertIn("MOCK npm run typecheck", success.stdout)
            self.assertIn("MOCK npm test", success.stdout)
            self.assertIn("MOCK npm run build", success.stdout)

            for step in NAMED_STEPS:
                with self.subTest(step=step):
                    result = self._run_desktop_web_check(probe, mock_bin, step)
                    self.assertNotEqual(
                        result.returncode,
                        0,
                        msg=(
                            f"desktop-web-check must be non-zero when {step} exits {PROBE_FAIL_CODE}\n"
                            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                        ),
                    )


class TestLockFidelity(unittest.TestCase):
    def test_ci_recipe_does_not_depend_on_sync_lock(self) -> None:
        text = JUSTFILE.read_text(encoding="utf-8")
        self.assertRegex(text, r"(?m)^sync-lock:\s*$")
        match = re.search(r"(?m)^ci:(.*)$", text)
        self.assertIsNotNone(match, "justfile is missing a ci recipe")
        deps = match.group(1).split()
        self.assertNotIn("sync-lock", deps)
        self.assertEqual(deps, ["fmt", "check", "test", "clippy"])

    def test_desktop_web_check_invokes_named_scripts_with_types_check(self) -> None:
        text = JUSTFILE.read_text(encoding="utf-8")
        start = text.find("desktop-web-check:")
        self.assertNotEqual(start, -1)
        body = text[start:]
        next_recipe = re.search(r"(?m)^[A-Za-z0-9_-]+:", body[len("desktop-web-check:") :])
        if next_recipe:
            body = body[: len("desktop-web-check:") + next_recipe.start()]
        self.assertIn("npm run types:generate -- --check", body)
        self.assertIn("npm run lint", body)
        self.assertIn("npm run typecheck", body)
        self.assertIn("npm test", body)
        self.assertIn("npm run build", body)
        self.assertEqual(body.count("if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }"), 5)
        self.assertNotRegex(
            body,
            r"npm run types:generate(?! -- --check)",
        )


class TestTypesDriftCheck(unittest.TestCase):
    def _isolated_desktop(self, tmp: Path) -> Path:
        desktop = tmp / "desktop"
        scripts = desktop / "scripts"
        api = desktop / "src" / "api"
        scripts.mkdir(parents=True)
        api.mkdir(parents=True)
        shutil.copy2(GENERATOR, scripts / "generate-types.mjs")
        shutil.copytree(FIXTURES, api / "fixtures")
        shutil.copy2(TYPES_GEN, api / "types.gen.ts")
        if not DESKTOP_NODE_MODULES.is_dir():
            raise AssertionError(
                "desktop/node_modules is missing; types --check isolation needs quicktype-core"
            )
        _link_directory(DESKTOP_NODE_MODULES, desktop / "node_modules")
        return desktop

    def test_committed_types_pass_check(self) -> None:
        if not DESKTOP_NODE_MODULES.is_dir():
            raise AssertionError(
                "desktop/node_modules is missing; cannot run generate-types.mjs --check"
            )
        result = _run_types_check(REPO_ROOT / "desktop")
        self.assertEqual(
            result.returncode,
            0,
            msg=(
                "committed types.gen.ts should match --check\n"
                f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            ),
        )

    def test_mutated_types_fail_without_rewrite(self) -> None:
        with tempfile.TemporaryDirectory(
            prefix="devsweep-types-drift-", ignore_cleanup_errors=True
        ) as tmp_name:
            desktop = self._isolated_desktop(Path(tmp_name))
            types_path = desktop / "src" / "api" / "types.gen.ts"
            mutated = types_path.read_bytes() + DRIFT_MARKER
            types_path.write_bytes(mutated)
            result = _run_types_check(desktop)
            after = types_path.read_bytes()
            self.assertNotEqual(
                result.returncode,
                0,
                msg=(
                    "types --check must fail on a mutated types.gen.ts\n"
                    f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                ),
            )
            self.assertEqual(
                after,
                mutated,
                "types --check must not rewrite a drifted types.gen.ts",
            )
        self.assertEqual(
            TYPES_GEN.read_bytes().find(DRIFT_MARKER),
            -1,
            "the repository types.gen.ts must stay clean after the drift probe",
        )

    def test_crlf_working_copy_still_passes_check(self) -> None:
        with tempfile.TemporaryDirectory(
            prefix="devsweep-types-crlf-", ignore_cleanup_errors=True
        ) as tmp_name:
            desktop = self._isolated_desktop(Path(tmp_name))
            types_path = desktop / "src" / "api" / "types.gen.ts"
            lf = types_path.read_bytes().replace(b"\r\n", b"\n")
            types_path.write_bytes(lf.replace(b"\n", b"\r\n"))
            result = _run_types_check(desktop)
            self.assertEqual(
                result.returncode,
                0,
                msg=(
                    "types --check must accept a CRLF working copy of the LF blob\n"
                    f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                ),
            )


def _unbounded_on_event(header: str, event: str) -> bool:
    match = re.search(rf"(?m)^  {re.escape(event)}:\s*(.*)$", header)
    if match is None:
        return False
    inline = match.group(1).strip()
    if inline not in ("", "~", "null"):
        return False
    remainder = header[match.end() :].lstrip("\n")
    if not remainder:
        return True
    next_line = remainder.split("\n", 1)[0]
    if not next_line.strip():
        return True
    return not next_line.startswith("    ")


class TestHostedCiTriggers(unittest.TestCase):
    def test_ci_runs_on_pull_request_not_every_push(self) -> None:
        text = (REPO_ROOT / ".github" / "workflows" / "ci.yml").read_text(
            encoding="utf-8"
        )
        header = text.split("\njobs:", 1)[0]
        self.assertRegex(header, r"(?m)^on:\s*$")
        self.assertRegex(header, r"(?m)^  pull_request:\s*$")
        self.assertRegex(header, r"(?m)^  workflow_dispatch:\s*$")
        self.assertFalse(
            _unbounded_on_event(header, "push"),
            "hosted CI must not use unbounded on.push; keep pull_request and workflow_dispatch",
        )


if __name__ == "__main__":
    unittest.main()
