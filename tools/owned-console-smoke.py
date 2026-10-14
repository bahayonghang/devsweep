"""Read-only release CLI checks in a new hidden, private Windows console.

Status: a slow named-file reader followed by targeted CTRL_BREAK and a natural
canceled terminal. TUI: native console help/input/resize/idle observations and Q
exit. This is a native console check, not a ConPTY or draw-call benchmark.
Never attaches to an existing console or terminates a process.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import ctypes as ct
from ctypes import wintypes as wt
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time


class Coord(ct.Structure):
    _fields_ = [("x", ct.c_short), ("y", ct.c_short)]


class Rect(ct.Structure):
    _fields_ = [(name, ct.c_short) for name in ("left", "top", "right", "bottom")]


class BufferInfo(ct.Structure):
    _fields_ = [("size", Coord), ("cursor", Coord), ("attributes", wt.WORD),
                ("window", Rect), ("maximum", Coord)]


class KeyEvent(ct.Structure):
    _fields_ = [("down", wt.BOOL), ("repeat", wt.WORD), ("virtual_key", wt.WORD),
                ("scan", wt.WORD), ("character", wt.WCHAR), ("control", wt.DWORD)]


class EventUnion(ct.Union):
    _fields_ = [("key", KeyEvent), ("padding", ct.c_byte * 16)]


class InputRecord(ct.Structure):
    _fields_ = [("kind", wt.WORD), ("event", EventUnion)]


def measurement_module():
    spec = importlib.util.spec_from_file_location(
        "measure_readonly", Path(__file__).with_name("measure-readonly.py"))
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PrivateConsole:
    def __init__(self):
        self.kernel = ct.WinDLL("kernel32", use_last_error=True)
        signatures = {
            "CreateFileW": ([wt.LPCWSTR, wt.DWORD, wt.DWORD, ct.c_void_p, wt.DWORD, wt.DWORD, wt.HANDLE], wt.HANDLE),
            "CloseHandle": ([wt.HANDLE], wt.BOOL),
            "GetConsoleWindow": ([], wt.HWND),
            "GetConsoleProcessList": ([ct.POINTER(wt.DWORD), wt.DWORD], wt.DWORD),
            "GenerateConsoleCtrlEvent": ([wt.DWORD, wt.DWORD], wt.BOOL),
            "GetConsoleScreenBufferInfo": ([wt.HANDLE, ct.POINTER(BufferInfo)], wt.BOOL),
            "ReadConsoleOutputCharacterW": ([wt.HANDLE, wt.LPWSTR, wt.DWORD, Coord,
                                               ct.POINTER(wt.DWORD)], wt.BOOL),
            "WriteConsoleInputW": ([wt.HANDLE, ct.POINTER(InputRecord), wt.DWORD,
                                     ct.POINTER(wt.DWORD)], wt.BOOL),
            "SetConsoleScreenBufferSize": ([wt.HANDLE, Coord], wt.BOOL),
            "SetConsoleWindowInfo": ([wt.HANDLE, wt.BOOL, ct.POINTER(Rect)], wt.BOOL),
        }
        for name, (arguments, result) in signatures.items():
            function = getattr(self.kernel, name)
            function.argtypes, function.restype = arguments, result
        self.user = ct.WinDLL("user32", use_last_error=True)
        self.user.IsWindowVisible.argtypes = [wt.HWND]
        self.user.IsWindowVisible.restype = wt.BOOL

    def visible(self) -> bool:
        return bool(self.user.IsWindowVisible(self.kernel.GetConsoleWindow()))

    @contextmanager
    def handle(self, name):
        handle = self.kernel.CreateFileW(name, 0xC0000000, 3, None, 3, 0, None)
        if handle == ct.c_void_p(-1).value:
            raise ct.WinError(ct.get_last_error())
        try:
            yield handle
        finally:
            self.kernel.CloseHandle(handle)

    def members(self) -> set[int]:
        values = (wt.DWORD * 32)()
        count = self.kernel.GetConsoleProcessList(values, len(values))
        if not count or count > len(values):
            raise RuntimeError("private console membership unavailable or excessive")
        return set(values[:count])

    def check_owned(self, child, identity, sampler) -> None:
        if child.poll() is not None:
            raise RuntimeError("owned client exited before console action")
        observed = sampler.inspect({"pid": child.pid})
        if observed.get("created") != identity["created"]:
            raise RuntimeError("owned child creation identity changed or unavailable")
        expected = {os.getpid(), child.pid}
        deadline = time.perf_counter() + 3
        while (members := self.members()) != expected:
            if not members.issubset(expected):
                raise RuntimeError("console contains a process outside helper/client ownership")
            if child.poll() is not None or time.perf_counter() >= deadline:
                raise RuntimeError("owned client did not attach to the private console")
            time.sleep(0.02)

    def targeted_break(self, child, identity, sampler) -> None:
        self.check_owned(child, identity, sampler)
        # CTRL_C cannot target one process group on Windows. This nonzero group
        # is the owned child created with CREATE_NEW_PROCESS_GROUP below.
        if not self.kernel.GenerateConsoleCtrlEvent(1, child.pid):
            raise ct.WinError(ct.get_last_error())

    def key(self, character: str, virtual_key: int) -> None:
        events = (InputRecord * 2)()
        for index, down in enumerate((True, False)):
            events[index].kind = 1
            events[index].event.key = KeyEvent(down, 1, virtual_key, 0, character, 0)
        count = wt.DWORD()
        with self.handle("CONIN$") as handle:
            if not self.kernel.WriteConsoleInputW(handle,
                                                   events, 2, ct.byref(count)):
                raise ct.WinError(ct.get_last_error())
        if count.value != 2:
            raise RuntimeError("incomplete console input delivery")

    def screen(self) -> tuple[str, dict]:
        # Reopen CONOUT$ so reads address the active alternate buffer.
        with self.handle("CONOUT$") as handle:
            info = BufferInfo()
            if not self.kernel.GetConsoleScreenBufferInfo(handle, ct.byref(info)):
                raise ct.WinError(ct.get_last_error())
            width = info.window.right - info.window.left + 1
            height = info.window.bottom - info.window.top + 1
            lines = []
            for y in range(info.window.top, info.window.bottom + 1):
                text = ct.create_unicode_buffer(width + 1)
                count = wt.DWORD()
                if not self.kernel.ReadConsoleOutputCharacterW(handle, text, width,
                        Coord(info.window.left, y), ct.byref(count)):
                    raise ct.WinError(ct.get_last_error())
                lines.append(text.value.rstrip())
            return "\n".join(lines), {"columns": width, "rows": height}

    def resize(self, columns: int, rows: int) -> None:
        with self.handle("CONOUT$") as handle:
            info = BufferInfo()
            if not self.kernel.GetConsoleScreenBufferInfo(handle, ct.byref(info)):
                raise ct.WinError(ct.get_last_error())
            grown = Coord(max(info.size.x, columns), max(info.size.y, rows))
            if not self.kernel.SetConsoleScreenBufferSize(handle, grown):
                raise ct.WinError(ct.get_last_error())
            window = Rect(0, 0, columns - 1, rows - 1)
            if not self.kernel.SetConsoleWindowInfo(handle, True, ct.byref(window)):
                raise ct.WinError(ct.get_last_error())
            if not self.kernel.SetConsoleScreenBufferSize(handle, Coord(columns, rows)):
                raise ct.WinError(ct.get_last_error())


def status_probe(child, identity, sampler, console, output: Path, result: dict):
    path = output / "status.ndjson"
    deadline = time.perf_counter() + 15
    while not path.exists() and time.perf_counter() < deadline:
        if child.poll() is not None:
            raise RuntimeError("Status exited before creating output")
        time.sleep(0.05)
    observations, read_records = [], []
    with path.open("rb") as stream:
        for _ in range(4):
            time.sleep(1.25)
            # One complete line per read; the regular-file writer progresses
            # independently. This does not emulate a slow filesystem writer.
            line = stream.readline()
            if line.endswith(b"\n"):
                read_records.append(json.loads(line))
            observations.append({"reader_position": stream.tell(), "file_bytes": path.stat().st_size})
    result["slow_file_reader"] = observations
    result["slow_read_sequences"] = [row["sequence"] for row in read_records]
    console.check_owned(child, identity, sampler)
    result["pre_signal_process"] = sampler.inspect({"pid": child.pid})
    result["console_members_before_signal"] = sorted(console.members())
    signal_at = time.perf_counter()
    console.targeted_break(child, identity, sampler)
    result["cooperative_signal_sent"] = True
    result["exit_code"] = child.wait(timeout=30)
    result["signal_to_exit_ms"] = (time.perf_counter() - signal_at) * 1000
    raw = path.read_bytes()
    rows = [json.loads(line) for line in raw.splitlines()]
    result["records"] = [{"sequence": row["sequence"], "event": row["event"]} for row in rows]
    result["terminal"] = rows[-1] if rows else None
    result["stdout_bytes"] = (output / "client-stdout.txt").stat().st_size
    result["pass"] = (
        result["exit_code"] == 0 and raw.endswith(b"\n") and len(rows) >= 4
        and rows[0]["event"] == "status_started"
        and rows[-1]["event"] == "status_terminal"
        and rows[-1]["data"]["reason"] == "canceled"
        and sum(row["event"] == "status_terminal" for row in rows) == 1
        and [row["sequence"] for row in rows] == list(range(len(rows)))
        and len({row["operation_id"] for row in rows}) == 1
        and all(row["schema_version"] == 1 for row in rows)
        and [row["sequence"] for row in read_records] == list(range(len(read_records)))
        and any(item["reader_position"] < item["file_bytes"] for item in observations)
        and result["stdout_bytes"] == 0
    )


def wait_screen(console, child, predicate, timeout=10):
    deadline = time.perf_counter() + timeout
    while time.perf_counter() < deadline:
        if child.poll() is not None:
            raise RuntimeError("TUI exited before expected screen")
        text, size = console.screen()
        if predicate(text, size):
            return text, size
        time.sleep(0.05)
    raise TimeoutError("expected native console screen did not appear")


def tui_probe(child, identity, sampler, console, output: Path, result: dict):
    wait_screen(console, child, lambda text, _: "DevSweep" in text)
    console.check_owned(child, identity, sampler)
    console.key("l", 0x4C)
    initial, size = wait_screen(console, child, lambda text, _: "Scan finished" in text)
    (output / "tui-initial.txt").write_text(initial, encoding="utf-8")
    result["initial_size"] = size
    idle_before = sampler.inspect({"pid": child.pid})
    time.sleep(2)
    idle_after = sampler.inspect({"pid": child.pid})
    result["idle_observation"] = {"seconds": 2, "before": idle_before, "after": idle_after,
                                  "screen_unchanged": console.screen()[0] == initial}
    console.check_owned(child, identity, sampler)
    sent = time.perf_counter()
    console.key("?", 0xBF)
    help_text, _ = wait_screen(console, child, lambda text, _: "Keyboard help" in text)
    result["help_observed_ms"] = (time.perf_counter() - sent) * 1000
    (output / "tui-help.txt").write_text(help_text, encoding="utf-8")
    console.check_owned(child, identity, sampler)
    console.resize(110, 32)
    resized, resized_size = wait_screen(console, child, lambda text, size:
        size == {"columns": 110, "rows": 32} and "Keyboard help" in text)
    (output / "tui-resized.txt").write_text(resized, encoding="utf-8")
    result["resized_size"] = resized_size
    console.check_owned(child, identity, sampler)
    console.key("\x1b", 0x1B)
    wait_screen(console, child, lambda text, _: "Keyboard help" not in text and "DevSweep" in text)
    console.check_owned(child, identity, sampler)
    sent = time.perf_counter()
    console.key("q", 0x51)
    result["exit_code"] = child.wait(timeout=15)
    result["quit_to_exit_ms"] = (time.perf_counter() - sent) * 1000
    result["pass"] = result["exit_code"] == 0


def worker(args):
    measurement = measurement_module()
    exe, output = args.exe.resolve(strict=True), args.output.resolve(strict=True)
    result = {"mode": args.mode, "executable_sha256": measurement.sha256(exe),
              "driver_sha256": measurement.sha256(Path(__file__)), "pass": False,
              "forced_termination": False, "literal_ctrl_c_tested": False,
              "signal": "targeted CTRL_BREAK_EVENT to owned client process group" if args.mode == "status" else None}
    child = None
    console = PrivateConsole()
    sampler = measurement.WindowsSampler()
    try:
        if result["executable_sha256"] != args.sha256:
            raise RuntimeError("candidate executable hash mismatch")
        if console.members() != {os.getpid()}:
            raise RuntimeError("helper did not start alone in a private console")
        result["console_visible"] = console.visible()
        if result["console_visible"]:
            raise RuntimeError("helper console was not hidden")
        result["helper_identity"] = sampler.inspect({"pid": os.getpid()})
        environment = measurement.isolated_environment(output / "profile")
        fixture = output / "fixture"
        fixture.mkdir()
        (fixture / "package.json").write_text('{"private":true}', encoding="utf-8")
        (fixture / "node_modules").mkdir()
        (fixture / "node_modules/cache.bin").write_bytes(b"readonly fixture")
        # Disable provider executables and redirect additional roots for the
        # TUI's automatic discovery. No user cache tree is part of this fixture.
        environment["PATH"] = str(Path(os.environ["SystemRoot"]) / "System32")
        for key in ("CARGO_HOME", "XDG_CACHE_HOME", "ProgramFiles", "ProgramFiles(x86)"):
            root = output / "profile" / key.replace("(", "").replace(")", "")
            root.mkdir(exist_ok=True)
            environment[key] = str(root)
        arguments = [str(exe)]
        if args.mode == "status":
            arguments += ["status", "live", "--format", "ndjson", "--interval", "1",
                          "--output", str(output / "status.ndjson")]
        result["argv"] = arguments
        with (output / "client-stderr.txt").open("xb") as stderr, \
             open("CONIN$", "rb", buffering=0) as console_in, \
             (open("CONOUT$", "wb", buffering=0) if args.mode == "tui" else
              (output / "client-stdout.txt").open("xb")) as stdout:
            child = subprocess.Popen(arguments, cwd=fixture, env=environment, stdin=console_in,
                stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NEW_PROCESS_GROUP)
            identity = sampler.inspect({"pid": child.pid})
            if not identity.get("created"):
                raise RuntimeError("client creation identity unavailable")
            result["client_identity"] = identity
            if args.mode == "status":
                status_probe(child, identity, sampler, console, output, result)
            else:
                tui_probe(child, identity, sampler, console, output, result)
        result["console_members_after_exit"] = sorted(console.members())
        remaining = sampler.processes()
        result["client_present_after_exit"] = (child.pid in remaining and
            sampler.inspect(remaining[child.pid]).get("created") == identity["created"])
        result["pass"] &= console.members() == {os.getpid()} and not result["client_present_after_exit"]
    except Exception as error:
        result["pass"] = False
        result["error"] = f"{type(error).__name__}: {error}"
        if child is not None and child.poll() is None and "client_identity" in result:
            try:
                console.check_owned(child, result["client_identity"], sampler)
                if args.mode == "status" and not result.get("cooperative_signal_sent"):
                    console.targeted_break(child, result["client_identity"], sampler)
                    result["failure_cleanup_signal"] = "targeted CTRL_BREAK_EVENT"
                elif args.mode == "tui":
                    console.key("\x1b", 0x1B)
                    time.sleep(0.2)
                    console.key("q", 0x51)
                    result["failure_cleanup_signal"] = "owned console Esc then Q"
                result["failure_cleanup_exit"] = child.wait(timeout=15)
            except Exception as cleanup_error:
                result["cleanup_error"] = str(cleanup_error)
        result["left_running"] = child is not None and child.poll() is None
    finally:
        measurement.write_json(output / "result.json", result)
    return 0 if result["pass"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("status", "tui"))
    parser.add_argument("exe", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        return worker(args)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / "driver-source.py").write_bytes(Path(__file__).read_bytes())
    info = subprocess.STARTUPINFO()
    info.dwFlags |= subprocess.STARTF_USESHOWWINDOW
    info.wShowWindow = 0  # SW_HIDE: no interactive helper window.
    argv = [sys.executable, str(Path(__file__).resolve()), args.mode,
            str(args.exe.resolve(strict=True)), str(output), "--sha256", args.sha256, "--worker"]
    with (output / "helper-stdout.txt").open("xb") as stdout, \
         (output / "helper-stderr.txt").open("xb") as stderr:
        helper = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
            creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=info)
        try:
            code = helper.wait(timeout=80)
        except subprocess.TimeoutExpired:
            print(json.dumps({"pass": False, "helper_pid": helper.pid,
                "error": "helper deadline expired; no forced termination", "output": str(output)}))
            return 1
    print(json.dumps({"pass": code == 0, "result": str(output / "result.json")}))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
