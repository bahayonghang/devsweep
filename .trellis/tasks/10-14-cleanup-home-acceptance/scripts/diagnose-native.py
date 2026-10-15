"""180-second owned-process diagnostic, never a formal idle acceptance run."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


base = load("isolated_native", Path(__file__).with_name("isolated-native.py"))
manifest_path, exe, output = map(Path, sys.argv[1:])
fixture = json.loads(manifest_path.read_text(encoding="utf-8"))
output = base.owned(output)
output.mkdir(parents=True, exist_ok=False)
raw = output / "measurement"
env = fixture["environment"]
base.check_sentinels(fixture)
argv = [sys.executable, "-I", str(base.REPO / "tools/measure-readonly.py"), "run", "--exe", str(exe.resolve()),
        "--fixtures", fixture["project"], "--out", str(raw), "--workload", "desktop-idle", "--duration", "180", "--deadline", "210"]
base.write(output / "protocol.json", {"kind": "diagnostic only; not formal RT-AC7", "seconds": 180, "debug_port": None,
           "snapshot_seconds": [15, 125, 170], "exe_sha256": base.digest(exe), "environment": env, "argv": argv})
sampler = load("measure_owned", base.REPO / "tools/measure-readonly.py").WindowsSampler()
launch = None
with (output / "sampler.stdout.txt").open("xb") as stdout, (output / "sampler.stderr.txt").open("xb") as stderr:
    process = subprocess.Popen(argv, cwd=base.REPO, env=env, stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        deadline = time.monotonic() + 30
        while not (raw / "launch.json").exists():
            assert process.poll() is None and time.monotonic() < deadline
            time.sleep(.1)
        launch = json.loads((raw / "launch.json").read_text(encoding="utf-8"))
        pid, created = launch["pid"], int(launch["created"])
        print(json.dumps({"launched": launch, "diagnostic": str(output)}), flush=True)
        for seconds in [15, 125, 170]:
            while (time.time_ns() - int(launch["started_unix_ns"])) / 1e9 < seconds:
                assert process.poll() is None, "owned native exited before diagnostic snapshot"
                time.sleep(.2)
            assert sampler.inspect({"pid": pid}).get("created") == created
            rows = sampler.tree(pid, {pid: created})
            identities = {row["pid"]: row["created"] for row in rows}
            where = " OR ".join(f"ProcessId = {child}" for child in identities)
            command = f'[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); Get-CimInstance Win32_Process -Filter "{where}" | Select-Object ProcessId,ParentProcessId,CommandLine | ConvertTo-Json -Compress'
            result = subprocess.run([str(base.WINDOWS / "System32/WindowsPowerShell/v1.0/powershell.exe"), "-NoProfile", "-Command", command],
                                    env=env, capture_output=True, timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)
            assert result.returncode == 0, result.stderr.decode(errors="replace")
            for child, identity in identities.items():
                assert sampler.inspect({"pid": child}).get("created") == identity, "process identity changed during command-line inspection"
            command_lines = json.loads(result.stdout.decode("utf-8-sig"))
            base.write(output / f"snapshot-{seconds}.json", {"sampled_unix_ns": str(time.time_ns()), "elapsed_seconds": (time.time_ns() - int(launch["started_unix_ns"])) / 1e9,
                       "processes": rows, "command_lines": command_lines, "windows": sampler.windows(pid, created)})
            print(json.dumps({"snapshot": seconds, "pids": list(identities)}), flush=True)
        process.wait(timeout=50)
    finally:
        if process.poll() is None and launch is not None:
            base.write(output / "early-normal-close.json", sampler.close_window(launch["pid"], int(launch["created"])))
            process.wait(timeout=40)
assert process.returncode == 0
lifecycle = json.loads((raw / "summary.json").read_text(encoding="utf-8"))
assert lifecycle["lifecycle_pass"] and not lifecycle["forced_teardown"] and not lifecycle["owned_after_exit"]
base.check_sentinels(fixture)
base.write(output / "diagnostic-complete.json", {"diagnostic_only": True, "normal_close": True, "sentinels_unchanged": True})
print(json.dumps({"diagnostic_completed": str(output)}))
