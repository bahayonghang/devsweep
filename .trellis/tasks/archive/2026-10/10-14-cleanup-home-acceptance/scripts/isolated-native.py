"""Owned F acceptance fixtures and allowlisted processes; never executes cleanup."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

REPO = Path(__file__).resolve().parents[4]
OUTPUT = REPO / "dist/cleanup-home-final"
NODE = Path(r"C:\Users\lyh\AppData\Local\mise\installs\node\22.23.2\node.exe")
NPM = NODE.parent / "node_modules/npm/bin/npm-cli.js"
WINDOWS = Path(r"C:\Windows")


def owned(path: Path) -> Path:
    path = path.resolve()
    assert path.is_relative_to(OUTPUT.resolve()) and path.name.startswith("native-"), path
    return path


def write(path: Path, value: object) -> None:
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False)
        stream.write("\n")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def environment(root: Path) -> dict[str, str]:
    # No inherited environment/config/PATH. Runtime executable reads are the
    # qualified installed Node and system WebView2, never user cache discovery.
    env = {
        "SystemRoot": str(WINDOWS), "WINDIR": str(WINDOWS),
        "COMSPEC": str(WINDOWS / "System32/cmd.exe"),
        "PATHEXT": ".COM;.EXE;.BAT;.CMD", "PATH": f"{NODE.parent};{WINDOWS / 'System32'}",
        "PROCESSOR_ARCHITECTURE": "AMD64", "NUMBER_OF_PROCESSORS": str(os.cpu_count()),
        "NPM_CONFIG_UPDATE_NOTIFIER": "false", "NPM_CONFIG_AUDIT": "false",
        "NPM_CONFIG_FUND": "false", "NPM_CONFIG_OFFLINE": "true",
        "NPM_CONFIG_LOGS_MAX": "0", "NPM_CONFIG_LOGLEVEL": "silent",
    }
    directories = {
        "HOME": "home", "USERPROFILE": "home", "APPDATA": "roaming",
        "LOCALAPPDATA": "local", "PROGRAMDATA": "program-data", "ALLUSERSPROFILE": "program-data",
        "ProgramFiles": "program-files", "ProgramFiles(x86)": "program-files-x86",
        "TEMP": "temp", "TMP": "temp", "XDG_CONFIG_HOME": "xdg-config",
        "XDG_CACHE_HOME": "xdg-cache", "XDG_DATA_HOME": "xdg-data", "XDG_STATE_HOME": "xdg-state",
        "CARGO_HOME": "cargo-home", "RUSTUP_HOME": "rustup-home", "GRADLE_USER_HOME": "gradle",
        "NUGET_PACKAGES": "nuget", "GOCACHE": "go-cache", "GOPATH": "go-path",
        "GOMODCACHE": "go-mod-cache", "PIP_CACHE_DIR": "pip-cache", "UV_CACHE_DIR": "uv-cache",
        "BUN_INSTALL": "bun", "PNPM_HOME": "pnpm", "YARN_CACHE_FOLDER": "yarn-cache",
        "NPM_CONFIG_CACHE": "npm-cache", "NPM_CONFIG_PREFIX": "npm-prefix", "NPM_CONFIG_LOGS_DIR": "npm-logs",
        "WEBVIEW2_USER_DATA_FOLDER": "webview",
    }
    for key, relative in directories.items():
        path = root / "profile" / relative
        path.mkdir(parents=True, exist_ok=True)
        env[key] = str(path)
    env["HOMEDRIVE"] = Path(env["HOME"]).drive
    env["HOMEPATH"] = str(Path(env["HOME"]))[len(env["HOMEDRIVE"]):]
    for key, name in {"NPM_CONFIG_USERCONFIG": "npm-user.ini", "NPM_CONFIG_GLOBALCONFIG": "npm-global.ini", "PIP_CONFIG_FILE": "pip.ini"}.items():
        path = root / "profile" / name
        if not path.exists():
            path.write_text("", encoding="utf-8")
        env[key] = str(path)
    return env


def run(argv: list[str], cwd: Path, env: dict[str, str], out: Path, name: str) -> subprocess.CompletedProcess:
    assert "execute" not in argv and "--confirm" not in argv
    result = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, timeout=240 if name == "driver" else 90,
                            creationflags=subprocess.CREATE_NO_WINDOW)
    (out / f"{name}.stdout.txt").write_bytes(result.stdout)
    (out / f"{name}.stderr.txt").write_bytes(result.stderr)
    write(out / f"{name}.command.json", {"argv": argv, "cwd": str(cwd), "exit_code": result.returncode})
    assert result.returncode == 0, (name, result.returncode, result.stderr.decode(errors="replace"))
    return result


def prepare(root: Path, cli: Path) -> None:
    root = owned(root)
    root.mkdir(parents=True, exist_ok=False)
    env = environment(root)
    project = root / "project"
    sentinels = {
        project / "package.json": b'{"name":"devsweep-owned-native-fixture","version":"1.0.0","private":true}\n',
        project / "node_modules/fixture/sentinel.bin": b"dependency fixture; preserve\n" * 128,
        project / ".next/cache/sentinel.bin": b"build fixture; preserve\n" * 256,
        Path(env["NPM_CONFIG_CACHE"]) / "_cacache/content-v2/sha512/aa/bb/sentinel.bin": b"npm fixture; preserve\n" * 512,
        Path(env["CARGO_HOME"]) / "sentinel.txt": b"inspect-only fixture; preserve\n",
    }
    for path, data in sentinels.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    for path in sorted(root.rglob("*"), reverse=True):
        os.utime(path, (946684800, 946684800))
    tools = {name: shutil.which(name, path=env["PATH"]) for name in ["node", "npm", "python", "pip", "uv", "cargo", "mbx", "pnpm", "yarn", "go", "bun"]}
    assert all(tools[name] is None for name in ["python", "pip", "uv", "cargo", "mbx", "pnpm", "yarn", "go", "bun"]), tools
    probes = {}
    for name, argv in [("node", [str(NODE), "--version"]), ("npm", [str(NODE), str(NPM), "--version"]), *[(key, [str(NODE), str(NPM), "config", "get", key]) for key in ["cache", "userconfig", "globalconfig"]]]:
        probes[name] = run(argv, project, env, root, f"preflight-{name}").stdout.decode().strip()
    assert probes["node"] == "v22.23.2" and probes["npm"] == "10.9.8", probes
    for key, variable in [("cache", "NPM_CONFIG_CACHE"), ("userconfig", "NPM_CONFIG_USERCONFIG"), ("globalconfig", "NPM_CONFIG_GLOBALCONFIG")]:
        assert Path(probes[key]).resolve() == Path(env[variable]).resolve(), probes
    manifest = {"schema_version": 1, "root": str(root), "project": str(project), "cli": str(cli.resolve()),
                "python": sys.executable, "node": str(NODE), "npm": str(NPM), "environment": env,
                "tools": tools, "probes": probes,
                "tool_hashes": {str(p): digest(p) for p in [cli, NODE, NPM, NODE.parent / "npm.cmd"]},
                "sentinels": [{"path": str(p), "bytes": p.stat().st_size, "sha256": digest(p)} for p in sentinels],
                "prohibited": ["clean execute", "real user global discovery", "uv 0.13 execution", "installer replacement"],
                "native_launch_note": "measure-readonly overwrites private appdata/temp paths and restores inherited HOME/USERPROFILE; this allowlist already pins both to fixture paths"}
    write(root / "manifest.json", manifest)
    print(json.dumps({"preflight": "PASS", "manifest": str(root / "manifest.json"), "tools": tools, "probes": probes}))


def check_sentinels(manifest: dict) -> None:
    for row in manifest["sentinels"]:
        assert digest(Path(row["path"])) == row["sha256"], row["path"]


def cli_acceptance(manifest: dict, out: Path) -> None:
    out = owned(out)
    out.mkdir(parents=True, exist_ok=False)
    env, cli, project = manifest["environment"], manifest["cli"], Path(manifest["project"])
    records = []
    for locale in ["en", "zh-CN"]:
        for scope, rule in [("global", "npm.cache.clean"), ("projects", "node.next_cache")]:
            name = f"{locale}-{scope}"
            arguments = [cli, "clean", "scan", "--scope", scope, "--format", "json"]
            if scope == "projects":
                arguments += ["--root", str(project)]
            result = run(arguments, project, env, out, name + "-scan")
            report = json.loads(result.stdout)["data"]
            assert all(t["scope"]["type"] == ("project" if scope == "projects" else "global") for t in report["plan"]["targets"])
            if scope == "projects":
                assert all(not t["selected_by_default"] for t in report["plan"]["targets"])
            selected = next(t for t in report["plan"]["targets"] if t["rule_id"] == rule)
            observation, plan = out / f"{name}-scan.stdout.txt", out / f"{name}-plan.json"
            run([cli, "clean", "plan", "--observation", str(observation), "--select", selected["id"], "--output", str(plan)], project, env, out, name + "-plan")
            preview = json.loads(run([cli, "clean", "preview", "--plan", str(plan), "--format", "json"], project, env, out, name + "-preview").stdout)["data"]
            run([cli, "--language", locale, "clean", "preview", "--plan", str(plan)], project, env, out, name + "-preview-human")
            assert re.fullmatch(r"sha256:[0-9a-f]{64}", preview["digest"]), preview
            assert preview["report"]["dry_run"] and preview["report"]["selected"] == 1 and preview["report"]["attempted"] == 1 and preview["report"]["succeeded"] == 0 and preview["report"]["skipped"] == 1 and preview["report"]["failed"] == 0, preview
            assert preview["report"]["outcomes"][0]["status"] == {"type": "skipped", "reason": "dry run; no cleanup action executed"}, preview
            assert [row["target_id"] for row in preview["report"]["outcomes"]] == [selected["id"]], preview
            records.append({"locale": locale, "scope": scope, "target_id": selected["id"], "digest": preview["digest"], "preview_succeeded": True})
    check_sentinels(manifest)
    write(out / "summary.json", {"records": records, "sentinels_unchanged": True, "cleanup_executed": False})
    print(json.dumps({"cli_acceptance": "PASS", "out": str(out)}))


def native(manifest: dict, exe: Path, out: Path, port: int, idle: bool) -> None:
    out = owned(out)
    out.mkdir(parents=True, exist_ok=False)
    sample = out / "measurement"
    env = manifest["environment"]
    # The worker inherits this exact allowlist before the older runner restores HOME.
    argv = [sys.executable, "-I", str(REPO / "tools/measure-readonly.py"), "run", "--exe", str(exe.resolve()),
            "--fixtures", manifest["project"], "--out", str(sample), "--workload", "desktop-idle",
            "--duration", "548" if idle else "300", "--deadline", "580" if idle else "330"]
    if not idle:
        argv += ["--debug-port", str(port)]
    else:
        write(out / "protocol.json", {"source": "RT-AC7 and dist/performance/cleanup-earth/C2-idle-final-01/protocol.json",
              "startup_seconds": 8, "discarded_warmup_seconds": 90, "measured_repetitions": 5,
              "settle_seconds_per_repetition": 30, "retained_seconds_per_repetition": 60, "sample_period_ms": 200,
              "cpu_aggregation": "median of five per-repetition CPU medians; 100% is one logical core",
              "private_and_threads_aggregation": "maximum of every retained sample in all five repetitions",
              "gates": {"median_cpu_percent": 1, "peak_private_bytes": 192 * 1024 * 1024, "peak_threads": 40},
              "exe_sha256": digest(exe), "sampler_sha256": digest(REPO / "tools/measure-readonly.py"),
              "analysis_sha256": digest(Path(__file__).with_name("idle-analysis.py")),
              "debugger": "none; no CDP, additional WebView arguments removed by sampler",
              "window": "visible, no interaction; actual OS reduced-motion preference unchanged"})
    write(out / "isolation.json", {"outer_environment": env, "argv": argv, "exe_sha256": digest(exe),
                                  "measurement_kind": "true idle; no interactions" if idle else "instrumented interaction; never idle acceptance"})
    write(out / "effective-fixture.json", manifest)
    with (out / "sampler.stdout.txt").open("xb") as stdout, (out / "sampler.stderr.txt").open("xb") as stderr:
        process = subprocess.Popen(argv, cwd=REPO, env=env, stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
        driver_error = None
        if not idle:
            started = time.monotonic()
            while not (sample / "launch.json").exists():
                assert process.poll() is None and time.monotonic() - started < 30, "native launch did not appear"
                time.sleep(0.1)
            driver = Path(__file__).with_name("native-scopes.mjs")
            try:
                run([str(NODE), str(driver), str(sample / "launch.json"), str(out / "captures"), str(out / "effective-fixture.json"), str(REPO)], REPO, env, out, "driver")
            except Exception as error:
                driver_error = str(error)
                write(out / "driver-failed.json", {"error": driver_error})
            finally:
                # Normal close, guarded by the same exact PID/creation identity.
                launch = json.loads((sample / "launch.json").read_text())
                import importlib.util
                spec = importlib.util.spec_from_file_location("owned_measure", REPO / "tools/measure-readonly.py")
                module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(module)
                write(out / "normal-close.json", module.WindowsSampler().close_window(launch["pid"], int(launch["created"])))
        process.wait(timeout=600 if idle else 340)
    assert process.returncode == 0, "sampler/lifecycle failed; preserve raw artifacts"
    summary = json.loads((sample / "summary.json").read_text())
    assert summary["lifecycle_pass"] and not summary["owned_after_exit"] and not summary["forced_teardown"]
    check_sentinels(manifest)
    assert driver_error is None, driver_error
    if idle:
        run([sys.executable, "-I", str(Path(__file__).with_name("idle-analysis.py")), str(out)], REPO, env, out, "idle-analysis")
    write(out / "acceptance.json", {"lifecycle_pass": True, "sentinels_unchanged": True, "measurement_kind": "idle" if idle else "interactive", "cleanup_executed": False})
    print(json.dumps({"native": "PASS", "out": str(out), "idle": idle}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["prepare", "cli", "native", "idle"])
    parser.add_argument("--root", type=Path)
    parser.add_argument("--cli", type=Path, default=REPO / "dist/release/devsweep.exe")
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--exe", type=Path)
    parser.add_argument("--port", type=int, default=19314)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.root, args.cli)
    else:
        fixture = json.loads(args.manifest.read_text(encoding="utf-8"))
        check_sentinels(fixture)
        if args.command == "cli":
            cli_acceptance(fixture, args.out)
        else:
            native(fixture, args.exe, args.out, args.port, args.command == "idle")
