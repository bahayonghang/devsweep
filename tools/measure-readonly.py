"""Owned-process Windows performance evidence; no cleanup or maintenance actions.

Generated binaries, fixtures, profiles and raw output belong under ignored dist/.
This runner does not import the historical maintenance benchmark. Desktop normal
shutdown posts WM_CLOSE to its own root window. Deadline teardown may terminate
only recorded owned PID/creation identities and is always a failed measurement,
never successful cooperative cancellation or lifecycle evidence.
"""

from __future__ import annotations

import argparse
import ctypes as ct
from ctypes import wintypes as wt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import time


def write_json(path: Path, data: object) -> None:
    with path.open("x", encoding="utf-8") as stream:
        json.dump(data, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def fixture_manifest(root: Path) -> dict:
    rows = []
    for path in sorted(root.rglob("*")):
        if path.is_file() and path.name != "fixture-manifest.json":
            rows.append({"path": path.relative_to(root).as_posix(),
                         "bytes": path.stat().st_size, "sha256": sha256(path)})
    identity = hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()
    return {"version": 1, "generator": "measure-readonly.py/v1", "seed": 0,
            "mtime_epoch_seconds": 946684800, "files": rows,
            "file_count": len(rows), "sha256": identity}


def prepare(root: Path, counts: tuple[int, ...] = (100, 1000), wide_count: int = 5000) -> None:
    root.mkdir(parents=True, exist_ok=False)
    def put(relative: str, content: bytes) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
    for count in counts:
        for index in range(count):
            base = f"many-{count}/project-{index:05d}"
            put(f"{base}/package.json", b'{"private":true}\n')
            put(f"{base}/node_modules/cache.bin", bytes([index % 251]) * 256)
    put("wide/package.json", b'{"private":true}\n')
    for index in range(wide_count):
        put(f"wide/node_modules/entry-{index:05d}.bin", b"fixture\n")
    deep = "deep/" + "/".join(f"d{i}" for i in range(25))
    put(f"{deep}/pyproject.toml", b'[project]\nname="fixture"\nversion="0.0.0"\n')
    put(f"{deep}/.pytest_cache/cache.bin", b"fixture\n")
    put("Unicode path \u6d4b\u8bd5/package.json", b'{"private":true}\n')
    put("Unicode path \u6d4b\u8bd5/node_modules/cache.bin", b"fixture\n")
    for path in sorted(root.rglob("*"), reverse=True):
        os.utime(path, (946684800, 946684800))
    write_json(root / "fixture-manifest.json", fixture_manifest(root))


def workload(name: str, fixtures: Path) -> list[str]:
    if name in ("many-100", "many-1000", "wide", "deep", "Unicode path \u6d4b\u8bd5"):
        return ["clean", "scan", "--root", str(fixtures / name),
                "--scope", "projects", "--format", "json"]
    if name == "overlap":
        return ["clean", "scan", "--root", str(fixtures / "many-100"),
                "--root", str(fixtures / "many-100/project-00000"),
                "--scope", "projects", "--format", "json"]
    if name == "analyze":
        return ["analyze", "scan", "--root", str(fixtures / "wide"), "--format", "json"]
    if name == "optimize":
        return ["optimize", "list", "--format", "json"]
    if name == "software":
        return ["software", "inventory", "--format", "json"]
    if name in ("software-arp", "software-msi", "software-msix"):
        return ["software", "inventory", "--source", name.removeprefix("software-"), "--format", "json"]
    if name == "desktop-idle":
        return []
    raise ValueError(f"Unsupported read-only workload: {name}")


class ProcessEntry(ct.Structure):
    _fields_ = [("size", wt.DWORD), ("usage", wt.DWORD), ("pid", wt.DWORD),
                ("heap", ct.c_size_t), ("module", wt.DWORD), ("threads", wt.DWORD),
                ("parent", wt.DWORD), ("priority", wt.LONG), ("flags", wt.DWORD),
                ("name", wt.WCHAR * 260)]


class MemoryCounters(ct.Structure):
    _fields_ = [("size", wt.DWORD), ("faults", wt.DWORD)] + [
        (field, ct.c_size_t) for field in ("peak_working", "working", "peak_paged",
        "paged", "peak_nonpaged", "nonpaged", "pagefile", "peak_pagefile", "private")]


class IoCounters(ct.Structure):
    _fields_ = [(field, ct.c_ulonglong) for field in
               ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]


class ThreadEntry(ct.Structure):
    _fields_ = [("size", wt.DWORD), ("usage", wt.DWORD), ("tid", wt.DWORD),
                ("owner", wt.DWORD), ("priority", wt.LONG), ("delta", wt.LONG),
                ("flags", wt.DWORD)]


class WindowsSampler:
    def __init__(self) -> None:
        self.kernel = ct.WinDLL("kernel32", use_last_error=True)
        self.psapi = ct.WinDLL("psapi", use_last_error=True)
        signatures = {
            "CreateToolhelp32Snapshot": ([wt.DWORD, wt.DWORD], wt.HANDLE),
            "Process32FirstW": ([wt.HANDLE, ct.POINTER(ProcessEntry)], wt.BOOL),
            "Process32NextW": ([wt.HANDLE, ct.POINTER(ProcessEntry)], wt.BOOL),
            "OpenProcess": ([wt.DWORD, wt.BOOL, wt.DWORD], wt.HANDLE),
            "CloseHandle": ([wt.HANDLE], wt.BOOL),
            "GetProcessTimes": ([wt.HANDLE] + [ct.POINTER(wt.FILETIME)] * 4, wt.BOOL),
            "GetProcessHandleCount": ([wt.HANDLE, ct.POINTER(wt.DWORD)], wt.BOOL),
            "GetProcessIoCounters": ([wt.HANDLE, ct.POINTER(IoCounters)], wt.BOOL),
        }
        for name, (args, result) in signatures.items():
            function = getattr(self.kernel, name)
            function.argtypes, function.restype = args, result
        self.psapi.GetProcessMemoryInfo.argtypes = [wt.HANDLE, ct.POINTER(MemoryCounters), wt.DWORD]
        self.psapi.GetProcessMemoryInfo.restype = wt.BOOL

    def processes(self) -> dict[int, dict]:
        handle = self.kernel.CreateToolhelp32Snapshot(2, 0)
        if handle == ct.c_void_p(-1).value:
            raise ct.WinError(ct.get_last_error())
        rows = {}
        try:
            entry = ProcessEntry(size=ct.sizeof(ProcessEntry))
            ok = self.kernel.Process32FirstW(handle, ct.byref(entry))
            while ok:
                rows[entry.pid] = {"pid": entry.pid, "parent": entry.parent,
                                   "name": entry.name, "threads": entry.threads}
                ok = self.kernel.Process32NextW(handle, ct.byref(entry))
        finally:
            self.kernel.CloseHandle(handle)
        return rows

    def inspect(self, entry: dict) -> dict:
        result = dict(entry, present=False, unavailable=[])
        handle = self.kernel.OpenProcess(0x1000 | 0x10, False, entry["pid"])
        if not handle:
            result["unavailable"].append(f"open_process:{ct.get_last_error()}")
            return result
        try:
            stamps = [wt.FILETIME() for _ in range(4)]
            if not self.kernel.GetProcessTimes(handle, *(ct.byref(stamp) for stamp in stamps)):
                result["unavailable"].append(f"process_times:{ct.get_last_error()}")
                return result
            values = [stamp.dwHighDateTime * 2**32 + stamp.dwLowDateTime for stamp in stamps]
            result.update(present=True, created=values[0], kernel_ms=values[2] / 10000,
                          user_ms=values[3] / 10000, cpu_ms=sum(values[2:]) / 10000)
            memory = MemoryCounters(size=ct.sizeof(MemoryCounters))
            if self.psapi.GetProcessMemoryInfo(handle, ct.byref(memory), memory.size):
                result.update(private_bytes=memory.private, working_bytes=memory.working)
            else:
                result["unavailable"].append(f"memory:{ct.get_last_error()}")
            handles = wt.DWORD()
            if self.kernel.GetProcessHandleCount(handle, ct.byref(handles)):
                result["handles"] = handles.value
            else:
                result["unavailable"].append(f"handles:{ct.get_last_error()}")
            io = IoCounters()
            if self.kernel.GetProcessIoCounters(handle, ct.byref(io)):
                result["io"] = {key: getattr(io, key) for key, _ in io._fields_}
            else:
                result["unavailable"].append(f"io:{ct.get_last_error()}")
            return result
        finally:
            self.kernel.CloseHandle(handle)

    def tree(self, root_pid: int, owned: dict[int, int]) -> list[dict]:
        entries = self.processes()
        roots = [self.inspect(entries[pid]) for pid in owned if pid in entries]
        rows = {row["pid"]: row for row in roots
                if row.get("created") == owned[row["pid"]] and row["present"]}
        if root_pid not in owned and root_pid in entries:
            root = self.inspect(entries[root_pid])
            if root["present"]:
                rows[root_pid] = root
        changed = True
        while changed:
            changed = False
            for pid, entry in entries.items():
                parent = rows.get(entry["parent"])
                if pid not in rows and parent:
                    child = self.inspect(entry)
                    if child.get("created", -1) >= parent["created"]:
                        rows[pid] = child
                        changed = True
        owned.update({pid: row["created"] for pid, row in rows.items()})
        return list(rows.values())

    def windows(self, root_pid: int, created: int) -> list[dict]:
        current = self.inspect({"pid": root_pid})
        if current.get("created") != created:
            return []
        user = ct.WinDLL("user32", use_last_error=True)
        callback_type = ct.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
        user.EnumWindows.argtypes = [callback_type, wt.LPARAM]
        user.GetWindowThreadProcessId.argtypes = [wt.HWND, ct.POINTER(wt.DWORD)]
        user.GetClassNameW.argtypes = [wt.HWND, wt.LPWSTR, ct.c_int]
        user.GetWindowTextW.argtypes = [wt.HWND, wt.LPWSTR, ct.c_int]
        user.IsWindowVisible.argtypes, user.IsWindowVisible.restype = [wt.HWND], wt.BOOL
        user.IsIconic.argtypes, user.IsIconic.restype = [wt.HWND], wt.BOOL
        windows = []
        @callback_type
        def collect(window, _):
            owner = wt.DWORD()
            user.GetWindowThreadProcessId(window, ct.byref(owner))
            if owner.value != root_pid:
                return True
            window_class = ct.create_unicode_buffer(256)
            title = ct.create_unicode_buffer(256)
            user.GetClassNameW(window, window_class, 256)
            user.GetWindowTextW(window, title, 256)
            if window_class.value in ("Tauri Window", "tray_icon_app"):
                windows.append({"handle": int(window), "visible": bool(user.IsWindowVisible(window)),
                                "minimized": bool(user.IsIconic(window)),
                                "class": window_class.value, "title": title.value})
            return True
        user.EnumWindows(collect, 0)
        return windows

    def close_window(self, root_pid: int, created: int) -> int:
        user = ct.WinDLL("user32", use_last_error=True)
        user.PostMessageW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
        count = 0
        for window in self.windows(root_pid, created):
            if window["class"] == "Tauri Window" and window["title"] == "DevSweep":
                count += bool(user.PostMessageW(window["handle"], 0x10, 0, 0))
        return count

    def main_window_state(self, root_pid: int, created: int, state: str,
                          window_handle: int | None = None) -> int:
        commands = {"minimized": 6, "restored": 9}
        command = commands[state]
        windows = [window for window in self.windows(root_pid, created)
                   if window["class"] == "Tauri Window" and window["title"] == "DevSweep"
                   and (window_handle is None or window["handle"] == window_handle)]
        if len(windows) != 1:
            raise RuntimeError("Expected exactly one owned main window")
        user = ct.WinDLL("user32", use_last_error=True)
        user.ShowWindow.argtypes, user.ShowWindow.restype = [wt.HWND, ct.c_int], wt.BOOL
        user.ShowWindow(windows[0]["handle"], command)
        return windows[0]["handle"]

    def toggle_owned_hud(self, root_pid: int, created: int) -> int:
        windows = [window for window in self.windows(root_pid, created)
                   if window["class"] == "tray_icon_app"]
        if len(windows) != 1:
            raise RuntimeError("Expected exactly one owned tray window")
        # Pinned tray-icon 0.24.2 windows/mod.rs: WM_USER_TRAYICON=6002,
        # lparam=WM_LBUTTONUP. Its own window userdata supplies the tray id and
        # Shell_NotifyIconGetRect; wparam is unused. This follows the real tray
        # event callback without moving the user's cursor or touching Explorer.
        user = ct.WinDLL("user32", use_last_error=True)
        user.PostMessageW.argtypes, user.PostMessageW.restype = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM], wt.BOOL
        if not user.PostMessageW(windows[0]["handle"], 6002, 0, 0x0202):
            raise ct.WinError(ct.get_last_error())
        return windows[0]["handle"]

    def named_threads(self, root_pid: int, created: int) -> list[dict]:
        """Read thread descriptions only for this exact owned root process."""
        if self.inspect({"pid": root_pid}).get("created") != created:
            raise RuntimeError("Owned process creation identity changed")
        kernel = self.kernel
        kernel.Thread32First.argtypes = kernel.Thread32Next.argtypes = [wt.HANDLE, ct.POINTER(ThreadEntry)]
        kernel.Thread32First.restype = kernel.Thread32Next.restype = wt.BOOL
        kernel.OpenThread.argtypes, kernel.OpenThread.restype = [wt.DWORD, wt.BOOL, wt.DWORD], wt.HANDLE
        kernel.GetProcessIdOfThread.argtypes, kernel.GetProcessIdOfThread.restype = [wt.HANDLE], wt.DWORD
        kernel.GetThreadDescription.argtypes, kernel.GetThreadDescription.restype = [wt.HANDLE, ct.POINTER(wt.LPWSTR)], wt.LONG
        kernel.LocalFree.argtypes, kernel.LocalFree.restype = [wt.HANDLE], wt.HANDLE
        snapshot = kernel.CreateToolhelp32Snapshot(4, 0)
        if snapshot == ct.c_void_p(-1).value:
            raise ct.WinError(ct.get_last_error())
        rows = []
        try:
            entry = ThreadEntry(size=ct.sizeof(ThreadEntry))
            present = kernel.Thread32First(snapshot, ct.byref(entry))
            while present:
                if entry.owner == root_pid:
                    row = {"tid": entry.tid, "description": None}
                    handle = kernel.OpenThread(0x0800, False, entry.tid)
                    if handle:
                        try:
                            description = wt.LPWSTR()
                            if kernel.GetProcessIdOfThread(handle) == root_pid and kernel.GetThreadDescription(handle, ct.byref(description)) >= 0:
                                try:
                                    row["description"] = description.value
                                finally:
                                    kernel.LocalFree(ct.cast(description, wt.HANDLE))
                        finally:
                            kernel.CloseHandle(handle)
                    rows.append(row)
                present = kernel.Thread32Next(snapshot, ct.byref(entry))
        finally:
            kernel.CloseHandle(snapshot)
        if self.inspect({"pid": root_pid}).get("created") != created:
            raise RuntimeError("Owned process exited during thread inspection")
        return rows

    def terminate_owned(self, owned: dict[int, int]) -> list[dict]:
        """Failure teardown only; check creation time on the same held handle."""
        self.kernel.TerminateProcess.argtypes = [wt.HANDLE, wt.UINT]
        self.kernel.TerminateProcess.restype = wt.BOOL
        attempts = []
        for pid, created in sorted(owned.items(), key=lambda pair: pair[1], reverse=True):
            handle = self.kernel.OpenProcess(0x1000 | 0x0001, False, pid)
            row = {"pid": pid, "created": created, "terminated": False}
            if not handle:
                row["error"] = f"open_process:{ct.get_last_error()}"
                attempts.append(row)
                continue
            try:
                stamps = [wt.FILETIME() for _ in range(4)]
                valid = self.kernel.GetProcessTimes(handle, *(ct.byref(stamp) for stamp in stamps))
                actual = stamps[0].dwHighDateTime * 2**32 + stamps[0].dwLowDateTime
                if valid and actual == created:
                    row["terminated"] = bool(self.kernel.TerminateProcess(handle, 124))
                    if not row["terminated"]:
                        row["error"] = f"terminate_process:{ct.get_last_error()}"
                else:
                    row["error"] = "creation_identity_unavailable_or_changed"
            finally:
                self.kernel.CloseHandle(handle)
            attempts.append(row)
        return attempts


def workload_deadline(name: str, duration: float, deadline: float | None) -> float:
    if not math.isfinite(duration) or duration <= 0:
        raise ValueError("Duration must be finite and positive")
    seconds = deadline if deadline is not None else (duration + 30 if name == "desktop-idle" else 900)
    if not math.isfinite(seconds) or seconds <= 0 or seconds > 86400:
        raise ValueError("Deadline must be finite, positive and at most 86400 seconds")
    if name == "desktop-idle" and seconds <= duration:
        raise ValueError("Desktop deadline must leave time after the normal close request")
    return seconds


def isolated_environment(profile: Path) -> dict[str, str]:
    environment = dict(os.environ)
    for key, relative in {"LOCALAPPDATA": "Local", "APPDATA": "Roaming",
                          "USERPROFILE": "Home", "HOME": "Home", "TEMP": "Temp",
                          "TMP": "Temp", "WEBVIEW2_USER_DATA_FOLDER": "WebView"}.items():
        path = profile / relative
        path.mkdir(parents=True, exist_ok=True)
        environment[key] = str(path)
    environment.pop("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", None)
    # Seed only this launch's private locale; never touch the user's settings.
    settings = profile / "Local/DevSweep/settings"
    settings.mkdir(parents=True)
    write_json(settings / "presentation-v1.json", {"schema_version": 1, "language": "en"})
    return environment


def measure(exe: Path, fixtures: Path, out: Path, name: str, duration: float,
            debug_port: int | None = None, deadline: float | None = None,
            window_state: str = "visible") -> dict:
    arguments = workload(name, fixtures)
    deadline_seconds = workload_deadline(name, duration, deadline)
    executable_hash = sha256(exe)
    out.mkdir(parents=True, exist_ok=False)
    environment = isolated_environment(out / "profile")
    if name == "desktop-idle":
        # Historical harness bootstrap compatibility choice, not a proven
        # WebView2 runtime-discovery requirement. App preferences, temporary
        # files and WebView data remain isolated, but real HOME/USERPROFILE
        # restrict this workload to runs without global user-cache discovery.
        for key in ("USERPROFILE", "HOME"):
            if key in os.environ:
                environment[key] = os.environ[key]
    if debug_port:
        environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = f"--remote-debugging-port={debug_port}"
    sampler = WindowsSampler()
    owned: dict[int, int] = {}
    samples = []
    started = time.perf_counter()
    with (out / "stdout.txt").open("xb") as stdout, (out / "stderr.txt").open("xb") as stderr:
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = 5 if name == "desktop-idle" and window_state == "visible" else 0
        process = subprocess.Popen([str(exe), *arguments], cwd=fixtures, env=environment,
                                   stdout=stdout, stderr=stderr, startupinfo=startup,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        root_identity = sampler.inspect({"pid": process.pid})
        if root_identity.get("present"):
            owned[process.pid] = root_identity["created"]
        write_json(out / "launch.json", {"pid": process.pid, "executable": str(exe),
                                         "created": str(root_identity["created"]) if root_identity.get("present") else None,
                                         "started_unix_ns": str(time.time_ns()),
                                         "workload": name, "debug_port": debug_port,
                                         "deadline_seconds": deadline_seconds,
                                         "window_state_requested": window_state})
        close_requested = None
        timed_out = False
        teardown = []
        next_sample = 0.0
        while process.poll() is None:
            elapsed = time.perf_counter() - started
            if elapsed >= next_sample:
                rows = sampler.tree(process.pid, owned)
                sample = {"wall_ms": elapsed * 1000, "sampled_unix_ns": str(time.time_ns()),
                          "root_pid": process.pid, "processes": rows}
                if name == "desktop-idle":
                    sample["windows"] = sampler.windows(process.pid, owned.get(process.pid, -1))
                samples.append(sample)
                next_sample = elapsed + 0.2
            if name == "desktop-idle" and elapsed >= duration and close_requested is None:
                close_requested = {"wall_ms": elapsed * 1000,
                                   "windows": sampler.close_window(process.pid, owned.get(process.pid, -1))}
            if elapsed >= deadline_seconds:
                timed_out = True
                # Preserve pre-teardown facts; forced termination never passes a
                # cancellation/join or elapsed-time acceptance criterion.
                write_json(out / "deadline-failure.json", {"deadline_seconds": deadline_seconds,
                           "elapsed_ms": elapsed * 1000, "owned": owned,
                           "remaining": sampler.tree(process.pid, owned)})
                teardown = sampler.terminate_owned(owned)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass
                break
            time.sleep(0.01)
        elapsed_ms = (time.perf_counter() - started) * 1000
        remaining = sampler.tree(process.pid, owned)
        # WebView helpers can exit shortly after the host. Observe their normal
        # exit for a bounded interval; never terminate them to pass lifecycle.
        settle_started = time.perf_counter()
        settle_samples = []
        if process.poll() is not None:
            while remaining:
                settle_elapsed = time.perf_counter() - settle_started
                settle_samples.append({"wall_ms": settle_elapsed * 1000,
                                       "remaining": remaining})
                if settle_elapsed >= 5:
                    break
                time.sleep(min(0.2, 5 - settle_elapsed))
                remaining = sampler.tree(process.pid, owned)
        settle_elapsed_ms = (time.perf_counter() - settle_started) * 1000
    with (out / "samples.jsonl").open("x", encoding="utf-8") as stream:
        for row in samples:
            stream.write(json.dumps(row) + "\n")
    valid_root_samples = sum(any(row["pid"] == process.pid and row.get("present")
                                 for row in sample["processes"]) for sample in samples)
    summary = {"version": 2, "workload": name, "executable": str(exe), "sha256": executable_hash,
               "argv": arguments, "cwd": str(fixtures), "pid": process.pid,
               "exit_code": process.poll(), "elapsed_ms": elapsed_ms,
               "deadline_seconds": deadline_seconds, "timed_out": timed_out,
               "measurement_status": "failed: deadline exceeded" if timed_out else ("completed" if process.returncode == 0 else "failed: nonzero exit"),
               "forced_teardown": teardown,
               "lifecycle_pass": not timed_out and process.returncode == 0 and not remaining,
               "window_state_requested": window_state if name == "desktop-idle" else None,
               "sample_count": len(samples), "sample_period_ms": 200,
               "resources_status": "available" if valid_root_samples >= 2 else "unavailable: fewer than two valid root samples",
               "cpu_definition": "100% = one logical core; delta CPU milliseconds / delta wall milliseconds * 100",
               "close_requested": close_requested, "owned_after_exit": remaining,
               "post_exit_settle": {"limit_ms": 5000, "elapsed_ms": settle_elapsed_ms,
                                    "samples": settle_samples, "forced_teardown": False},
               "profile": str(out / "profile"), "debug_port": debug_port,
               "stdout_bytes": (out / "stdout.txt").stat().st_size,
               "host": {"platform": platform.platform(), "logical_cores": os.cpu_count(),
                        "python": platform.python_version(), "os_file_cache": "uncontrolled"}}
    write_json(out / "summary.json", summary)
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    fixture = sub.add_parser("prepare")
    fixture.add_argument("root", type=Path)
    run = sub.add_parser("run")
    run.add_argument("--exe", required=True, type=Path)
    run.add_argument("--fixtures", required=True, type=Path)
    run.add_argument("--out", required=True, type=Path)
    run.add_argument("--workload", required=True)
    run.add_argument("--duration", type=float, default=30)
    run.add_argument("--debug-port", type=int)
    run.add_argument("--deadline", type=float, help="Hard workload seconds; default CLI 900, desktop duration + 30. Timeout is a failed measurement.")
    run.add_argument("--window-state", choices=("visible", "hidden"), default="visible")
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.root.resolve())
    else:
        result = measure(args.exe.resolve(), args.fixtures.resolve(), args.out.resolve(),
                         args.workload, args.duration, args.debug_port, args.deadline, args.window_state)
        print(json.dumps(result, indent=2))
        if result["measurement_status"] != "completed" or result["owned_after_exit"]:
            raise SystemExit(1)


if __name__ == "__main__":
    main()
