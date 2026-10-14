"""Safety/identity checks for the opt-in read-only benchmark harness."""

import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("measure_readonly", Path(__file__).parents[1] / "measure-readonly.py")
BENCH = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCH)


class ReadonlyBenchmarkTests(unittest.TestCase):
    def test_closed_workloads_reject_mutation_and_never_scan_global(self):
        with self.assertRaises(ValueError):
            BENCH.workload("clean execute", Path("fixtures"))
        for name in ("many-100", "many-1000", "wide", "deep", "Unicode path 测试", "overlap"):
            args = BENCH.workload(name, Path("fixtures"))
            self.assertEqual(args[:2], ["clean", "scan"])
            self.assertEqual(args[args.index("--scope") + 1], "projects")

    def test_fixture_generation_is_deterministic_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = Path(directory) / "first", Path(directory) / "second"
            BENCH.prepare(first, (2,), 3)
            BENCH.prepare(second, (2,), 3)
            self.assertEqual(BENCH.fixture_manifest(first), BENCH.fixture_manifest(second))
            with self.assertRaises(FileExistsError):
                BENCH.prepare(first, (2,), 3)

    def test_environment_changes_are_child_local(self):
        before = dict(os.environ)
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory)
            env = BENCH.isolated_environment(profile)
            for key in ("LOCALAPPDATA", "APPDATA", "HOME", "USERPROFILE", "TEMP", "TMP", "WEBVIEW2_USER_DATA_FOLDER"):
                self.assertTrue(Path(env[key]).is_relative_to(profile))
            self.assertNotIn("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", env)
        self.assertEqual(dict(os.environ), before)

    def test_process_identity_excludes_reused_pid_and_unrelated_process(self):
        sampler = object.__new__(BENCH.WindowsSampler)
        table = {1: {"pid": 1, "parent": 0, "created": 100, "present": True},
                 2: {"pid": 2, "parent": 1, "created": 110, "present": True},
                 3: {"pid": 3, "parent": 0, "created": 120, "present": True},
                 4: {"pid": 4, "parent": 1, "created": 90, "present": True}}
        sampler.processes = lambda: table
        sampler.inspect = lambda row: row
        owned = {}
        self.assertEqual({row["pid"] for row in sampler.tree(1, owned)}, {1, 2})
        table[1] = dict(table[1], created=130)
        self.assertEqual({row["pid"] for row in sampler.tree(1, owned)}, {2})

    def test_workload_deadline_is_explicit_finite_and_bounded(self):
        self.assertEqual(BENCH.workload_deadline("many-1000", 30, None), 900)
        self.assertEqual(BENCH.workload_deadline("desktop-idle", 30, None), 60)
        self.assertEqual(BENCH.workload_deadline("software-msi", 30, 120), 120)
        for value in (0, -1, float("inf"), float("nan"), 86401):
            with self.assertRaises(ValueError):
                BENCH.workload_deadline("software", 30, value)
        with self.assertRaises(ValueError):
            BENCH.workload_deadline("desktop-idle", 30, 30)
        self.assertEqual(BENCH.workload("software-msi", Path("fixtures")),
                         ["software", "inventory", "--source", "msi", "--format", "json"])

    @unittest.skipUnless(sys.platform == "win32", "Windows window identity")
    def test_minimize_requires_exact_owned_window_when_hud_shares_title(self):
        sampler = object.__new__(BENCH.WindowsSampler)
        sampler.windows = lambda pid, created: [
            {"class": "Tauri Window", "title": "DevSweep", "handle": 11},
            {"class": "Tauri Window", "title": "DevSweep", "handle": 12},
        ]
        user = Mock()
        with patch.object(BENCH.ct, "WinDLL", return_value=user):
            with self.assertRaises(RuntimeError):
                sampler.main_window_state(1, 100, "minimized")
            with self.assertRaises(RuntimeError):
                sampler.main_window_state(1, 100, "minimized", 999)
            user.ShowWindow.assert_not_called()
            self.assertEqual(sampler.main_window_state(1, 100, "minimized", 11), 11)
            user.ShowWindow.assert_called_once_with(11, 6)

    @unittest.skipUnless(sys.platform == "win32", "Windows thread descriptions")
    def test_named_threads_rejects_reused_pid_and_reads_current_process(self):
        sampler = BENCH.WindowsSampler()
        created = sampler.inspect({"pid": os.getpid()})["created"]
        with self.assertRaises(RuntimeError):
            sampler.named_threads(os.getpid(), created + 1)
        rows = sampler.named_threads(os.getpid(), created)
        self.assertTrue(rows)
        self.assertTrue(all(isinstance(row["tid"], int) for row in rows))

    @unittest.skipUnless(sys.platform == "win32", "Windows owned-process teardown")
    def test_termination_rejects_wrong_creation_identity(self):
        process = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"],
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        sampler = BENCH.WindowsSampler()
        created = sampler.inspect({"pid": process.pid})["created"]
        try:
            attempts = sampler.terminate_owned({process.pid: created + 1})
            self.assertFalse(attempts[0]["terminated"])
            self.assertIsNone(process.poll())
        finally:
            sampler.terminate_owned({process.pid: created})
            process.wait(timeout=5)

    @unittest.skipUnless(sys.platform == "win32", "Windows deadline fixture")
    def test_deadline_teardown_is_failure_and_leaves_no_owned_child(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(BENCH, "workload", return_value=["-c", "import time; time.sleep(60)"]):
                report = BENCH.measure(Path(sys.executable), Path(directory),
                                       Path(directory) / "deadline", "optimize", 30, deadline=0.1)
            self.assertTrue(report["timed_out"])
            self.assertFalse(report["lifecycle_pass"])
            self.assertEqual(report["measurement_status"], "failed: deadline exceeded")
            self.assertEqual(report["owned_after_exit"], [])
            self.assertTrue(any(row["terminated"] for row in report["forced_teardown"]))


if __name__ == "__main__":
    unittest.main()
