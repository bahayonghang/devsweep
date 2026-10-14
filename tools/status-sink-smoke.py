"""Read-only release CLI slow-pipe and create-new JSON output acceptance.

Owns only the launched child and a fresh output/profile directory. A failed
natural-exit deadline is reported; it is never counted as cancellation success.
Raw process observations remain under ignored dist/.
"""

from __future__ import annotations

import argparse
import ctypes
from ctypes import wintypes
import hashlib
import importlib.util
import json
import msvcrt
import os
from pathlib import Path
import subprocess
import time


def read_record(pipe, deadline: float) -> bytes:
    """Read one bounded record without a thread blocked in readline/close."""
    peek = ctypes.WinDLL("kernel32", use_last_error=True).PeekNamedPipe
    peek.argtypes = [wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD,
                     ctypes.POINTER(wintypes.DWORD), ctypes.POINTER(wintypes.DWORD),
                     ctypes.POINTER(wintypes.DWORD)]
    peek.restype = wintypes.BOOL
    handle = msvcrt.get_osfhandle(pipe.fileno())
    buffer = ctypes.create_string_buffer(65536)
    pending = bytearray()
    while time.perf_counter() < deadline:
        copied = wintypes.DWORD()
        if not peek(handle, buffer, len(buffer), ctypes.byref(copied), None, None):
            error = ctypes.get_last_error()
            if error == 109:  # ERROR_BROKEN_PIPE: drain was completed.
                return bytes(pending)
            raise ctypes.WinError(error)
        if copied.value:
            visible = buffer.raw[:copied.value]
            newline = visible.find(b"\n")
            count = newline + 1 if newline >= 0 else len(visible)
            pending.extend(os.read(pipe.fileno(), count))
            if len(pending) > 16 * 1024 * 1024:
                raise RuntimeError("one stream record exceeded the harness 16 MiB limit")
            if newline >= 0:
                return bytes(pending)
        else:
            time.sleep(min(0.05, max(0, deadline - time.perf_counter())))
    raise TimeoutError("five-record read deadline expired")


def snapshot_attempt(argv: list[str], out: Path, environment: dict, label: str,
                     result: dict, sampler, timeout: float = 30) -> int:
    # File sinks cannot fill a PIPE while wait runs. Popen.wait does not kill
    # on timeout, unlike subprocess.run; preserve the owned PID if it fails.
    with (out / f"{label}-stdout.txt").open("xb") as stdout, \
            (out / f"{label}-stderr.txt").open("xb") as stderr:
        child = subprocess.Popen(argv, cwd=out, env=environment, stdin=subprocess.DEVNULL,
                                 stdout=stdout, stderr=stderr,
                                 creationflags=subprocess.CREATE_NO_WINDOW)
        attempt = {"pid": child.pid, "exit_code": None, "natural_exit": False}
        result[label] = attempt
        try:
            identity = sampler.inspect({"pid": child.pid})
            attempt["created"] = str(identity["created"])
        finally:
            try:
                attempt["exit_code"] = child.wait(timeout=timeout)
                attempt["natural_exit"] = True
            except subprocess.TimeoutExpired:
                attempt["left_running"] = child.poll() is None
                raise TimeoutError(f"{label} owned PID {child.pid} did not exit naturally")
    return attempt["exit_code"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exe", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    exe = args.exe.resolve(strict=True)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    spec = importlib.util.spec_from_file_location(
        "measure_readonly", Path(__file__).with_name("measure-readonly.py")
    )
    assert spec and spec.loader
    measurement = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(measurement)
    environment = measurement.isolated_environment(out / "profile")
    argv = [str(exe), "status", "live", "--format", "ndjson", "--interval", "1"]
    sampler = measurement.WindowsSampler()
    result = {
        "executable_sha256": measurement.sha256(exe),
        "driver_sha256": measurement.sha256(Path(__file__)),
        "argv": argv,
        "method": "22s unread pipe, then five complete records at 500ms intervals; close reader and await natural exit",
        "read_deadline_seconds": 10,
        "natural_exit_deadline_seconds": 30,
        "forced_termination": False,
        "pass": False,
    }
    try:
        run_probe(exe, out, environment, sampler, result)
    except Exception as error:
        result["error"] = f"{type(error).__name__}: {error}"
    finally:
        measurement.write_json(out / "result.json", result)
    print(json.dumps({"pass": result["pass"], "result": str(out / "result.json")}))
    if not result["pass"]:
        raise SystemExit(1)


def run_probe(exe: Path, out: Path, environment: dict, sampler, result: dict) -> None:
    argv = result["argv"]
    started = time.perf_counter()
    with (out / "stderr.txt").open("xb") as stderr:
        child = subprocess.Popen(
            argv, cwd=out, env=environment, stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=stderr,
            bufsize=0,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        result["identity"] = {"pid": child.pid}
        assert child.stdout
        raw = []
        try:
            identity = sampler.inspect({"pid": child.pid})
            result["identity"]["created"] = str(identity["created"])
            time.sleep(22)
            result["before_read_ms"] = (time.perf_counter() - started) * 1000
            deadline = time.perf_counter() + 10
            for _ in range(5):
                line = read_record(child.stdout, deadline)
                if not line:
                    break
                raw.append(line)
                time.sleep(0.5)
        finally:
            child.stdout.close()
            closed = time.perf_counter()
            try:
                result["exit_code"] = child.wait(timeout=30)
            except subprocess.TimeoutExpired:
                result["exit_code"] = None
                result["left_running"] = child.poll() is None
                raise TimeoutError(f"live owned PID {child.pid} did not exit naturally within 30s of pipe closure")
            finally:
                result["pipe_close_to_exit_ms"] = (time.perf_counter() - closed) * 1000
                (out / "received.ndjson").write_bytes(b"".join(raw))
    records = [json.loads(line) for line in raw]
    sequences = [record["sequence"] for record in records]
    result["received"] = [{"sequence": row["sequence"], "event": row["event"]} for row in records]
    result["stream_pass"] = (
        result["exit_code"] == 0 and len(records) == 5
        and all(line.endswith(b"\n") for line in raw)
        and all(row["schema_version"] == 1 for row in records)
        and len({row["operation_id"] for row in records}) == 1
        and sequences[0] == 0
        and all(right == left + 1 for left, right in zip(sequences, sequences[1:]))
        and records[0]["event"] == "status_started"
    )
    snapshot = out / "snapshot.json"
    snapshot_argv = [str(exe), "status", "snapshot", "--format", "json", "--output", str(snapshot)]
    first = snapshot_attempt(snapshot_argv, out, environment, "snapshot-first", result, sampler)
    original = snapshot.read_bytes()
    document = json.loads(original)
    second = snapshot_attempt(snapshot_argv, out, environment, "snapshot-retry", result, sampler)
    result["snapshot"] = {
        "argv": snapshot_argv, "first_exit": first,
        "second_exit": second, "sha256": hashlib.sha256(original).hexdigest(),
        "single_v1_envelope": document.get("schema_version") == 1,
        "unchanged_on_retry": snapshot.read_bytes() == original,
    }
    result["pass"] = (
        result["stream_pass"] and first in (0, 5) and second != 0
        and result["snapshot"]["single_v1_envelope"]
        and result["snapshot"]["unchanged_on_retry"]
    )


if __name__ == "__main__":
    main()
