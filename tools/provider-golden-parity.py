"""Read-only B3/C2 full-envelope parity on isolated repaired-provider fixtures.

Prepare creates fixtures without launching a process. Run launches only clean
scan with the fixed native query fixture; no cleanup, shell, or forced shutdown.
All command logs and output stay outside every scanned/cache/home/temp root.
"""

from __future__ import annotations

import argparse
import ctypes as ct
from ctypes import wintypes as wt
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


REPO = Path(__file__).resolve().parent.parent
BUILDS = {
    "B3": ("B3/devsweep.exe", "1cf858d35b1be1cddaaae6284b605e89b6055024d7162121c09531b5fdc91266"),
    "C2": ("C2-runtime/devsweep.exe", "a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277"),
}
EPOCH = 946684800


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def file(path, content="retained sentinel\n"):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def tree(root):
    return [{"path": str(path.relative_to(root)), "directory": path.is_dir(),
             "mtime_ns": path.stat().st_mtime_ns,
             **({} if path.is_dir() else {"bytes": path.stat().st_size, "sha256": digest(path)})}
            for path in sorted(root.rglob("*"))]


def provider(case, name, version, cache, build_cache):
    binary = Path(case["bin"])
    script = {"npm": "node_modules/npm/bin/npm-cli.js", "yarn": "node_modules/yarn/bin/yarn.js"}.get(name)
    executable = binary / ("node.exe" if script else f"{name}.exe")
    shutil.copy2(REPO / "dist/debug/process_fixture.exe", executable)
    if script:
        file(binary / f"{name}.cmd", "THIS SHIM MUST NEVER BE EXECUTED")
        file(binary / script, "#!/usr/bin/env node\n// fixed entrypoint\n")
    save(binary / "provider.json", {"version": version, "cache": str(cache), "build_cache": str(build_cache)})
    case["provider"] = name
    case["script"] = str(binary / script) if script else None


def prepare(output):
    output.mkdir(parents=True, exist_ok=False)
    cases = []
    for name in ["go-native", "known-configured", "npm-configured", "nested-python", "partial-roots", "yarn-delegation"]:
        base = output / "cases" / name
        fixtures, binary, temporary = [base / suffix for suffix in ("fixtures", "provider-bin", "runtime-temp")]
        home = fixtures / "home"
        for path in [home, binary, temporary, home / "AppData/Local", home / "AppData/Roaming",
                     home / "ProgramData", home / ".config", fixtures / "ProgramFiles"]:
            path.mkdir(parents=True, exist_ok=True)
        environment = {key: os.environ[key] for key in ("SystemRoot", "WINDIR", "SystemDrive") if key in os.environ}
        environment.update({"PATH": str(binary), "PATHEXT": ".COM;.EXE;.BAT;.CMD", "HOME": str(home),
                            "USERPROFILE": str(home), "APPDATA": str(home / "AppData/Roaming"),
                            "LOCALAPPDATA": str(home / "AppData/Local"), "PROGRAMDATA": str(home / "ProgramData"),
                            "ProgramFiles": str(fixtures / "ProgramFiles"), "ProgramFiles(x86)": str(fixtures / "ProgramFiles"),
                            "XDG_CONFIG_HOME": str(home / ".config"), "XDG_CACHE_HOME": str(home / ".cache"),
                            "TEMP": str(temporary), "TMP": str(temporary), "TMPDIR": str(temporary),
                            "YARN_RC_FILENAME": "fixture-yarn.yml"})
        case = {"name": name, "fixtures": str(fixtures), "bin": str(binary), "temporary": str(temporary),
                "cwd": str(home), "environment": environment, "argv": ["clean", "scan", "--scope", "global", "--format", "json"],
                "expected_exit": 0, "expected": {}}
        expected = case["expected"]
        if name == "go-native":
            cache, build = fixtures / "module cache 空格", fixtures / "build cache 空格"
            file(cache / "module/sentinel"); file(build / "compiler/sentinel")
            provider(case, "go", "go version go1.26.0 windows/amd64", cache, build)
            expected.update({"go.mod_cache.clean": str(cache), "go.build_cache.clean": str(build)})
        elif name == "known-configured":
            gradle, nuget, go = [fixtures / path for path in ("custom Gradle 空格", "custom NuGet 空格", "custom Go 空格")]
            environment.update({"GRADLE_USER_HOME": str(gradle), "NUGET_PACKAGES": str(nuget), "GOMODCACHE": str(go)})
            expected.update({"gradle.caches": str(gradle / "caches"), "gradle.wrapper_dists": str(gradle / "wrapper/dists"),
                             "nuget.packages": str(nuget), "go.mod_cache": str(go), "cargo.home.inspect": str(home / ".cargo"),
                             "maven.repository": str(home / ".m2/repository"), "ivy.cache": str(home / ".ivy2/cache")})
            for path in expected.values():
                file(Path(path) / "sentinel")
            file(home / ".cargo/credentials.toml", "# dummy retained credential sentinel, no secret\n")
        elif name == "npm-configured":
            cache = fixtures / "npm custom cache 空格"
            file(cache / "_cacache/sentinel")
            file(home / ".npmrc", f"cache={cache.as_posix()}\n")
            provider(case, "npm", "11.0.0", cache, cache)
            expected["npm.cache.clean"] = str(cache)
        elif name == "nested-python":
            project = fixtures / "tox project 空格"
            file(project / "tox.ini", "[tox]\n"); file(project / "nested/tox.ini", "[tox]\n")
            expected.update({"python.tox": str(project / ".tox"), "python.pytest_cache": str(project / "nested/src/.pytest_cache"),
                             "python.__pycache__": str(project / "nested/src/__pycache__")})
            for path in expected.values():
                file(Path(path) / "sentinel")
            case["argv"] = ["clean", "scan", "--scope", "projects", "--root", str(project), "--format", "json"]
            case["project"] = str(project)
        elif name == "partial-roots":
            project, missing = fixtures / "valid Node 空格", fixtures / "missing root"
            file(project / "package.json", '{"name":"fixture","private":true}\n')
            file(project / "node_modules/package/sentinel")
            expected["node.node_modules"] = str(project / "node_modules")
            case.update(expected_exit=5, missing=str(missing))
            case["argv"] = ["clean", "scan", "--scope", "projects", "--root", str(project), "--root", str(missing), "--format", "json"]
        else:
            cache = fixtures / "yarn cache 空格"
            file(cache / "sentinel"); file(home / "fixture-yarn.yml", "  yarnPath: delegate.cjs\n")
            provider(case, "yarn", "4.0.0", cache, cache)
        # Stable timestamps make the whole envelope, including last_modified, comparable.
        for path in sorted(fixtures.rglob("*"), reverse=True):
            os.utime(path, (EPOCH, EPOCH))
        os.utime(fixtures, (EPOCH, EPOCH))
        case["fixture_manifest"] = tree(fixtures)
        case["provider_files"] = {str(path): digest(path) for path in sorted(binary.rglob("*")) if path.is_file()}
        cases.append(case)
    shutil.copy2(__file__, output / "driver.py")
    save(output / "prepared.json", {"driver_sha256": digest(Path(__file__)),
                                   "native_fixture_sha256": digest(REPO / "dist/debug/process_fixture.exe"), "cases": cases})
    print(json.dumps({"prepared": str(output), "cases": len(cases), "subprocesses_started": 0}))


def identity(child):
    function = ct.WinDLL("kernel32", use_last_error=True).GetProcessTimes
    function.argtypes = [wt.HANDLE] + [ct.POINTER(wt.FILETIME)] * 4
    function.restype = wt.BOOL
    stamps = [wt.FILETIME() for _ in range(4)]
    if not function(wt.HANDLE(int(child._handle)), *(ct.byref(stamp) for stamp in stamps)):
        raise ct.WinError(ct.get_last_error())
    return {"pid": child.pid, "created": stamps[0].dwHighDateTime * 2**32 + stamps[0].dwLowDateTime}


def same_path(left, right):
    # Expected fixture paths may differ from Rust's canonical Windows spelling.
    # This is only for expectations; B3/C2 envelope comparison stays untouched.
    def key(path):
        return os.path.normcase(os.path.normpath(str(path).removeprefix("\\\\?\\")))
    return key(left) == key(right)


def semantic_errors(case, document, exit_code):
    errors = []
    report = document["data"]
    targets = report["plan"]["targets"]
    observed = {target["rule_id"]: target["path"] for target in targets}
    if (observed.keys() != case["expected"].keys() or len(targets) != len(case["expected"])
            or any(not same_path(path, observed.get(rule)) for rule, path in case["expected"].items())):
        errors.append("unexpected rule/path set or duplicate rules")
    if exit_code != case["expected_exit"]:
        errors.append("unexpected exit")
    health = report["health"]
    if health["completeness"] != ("partial" if case["expected_exit"] == 5 else "complete"):
        errors.append("unexpected health completeness")
    for target in targets:
        if not target["size_complete"] or not target["estimated_bytes"]:
            errors.append("healthy fixture target has incomplete/zero size")
        if case["name"] in ("go-native", "npm-configured"):
            if target["intent"]["type"] != "run_built_in_action" or target["selected_by_default"]:
                errors.append("command target lost conservative built-in intent")
        if target["rule_id"] in ("cargo.home.inspect", "go.mod_cache", "maven.repository"):
            if target["intent"]["type"] != "inspect_only" or target["selected_by_default"]:
                errors.append("retained root gained cleanup authority")
        if case["name"] == "nested-python":
            expected_root = Path(case["project"]) / ("nested" if target["rule_id"] != "python.tox" else "")
            if target["scope"]["type"] != "project" or not same_path(target["scope"].get("root"), expected_root):
                errors.append("nested Python did not use nearest tox marker root")
    if case["name"] == "partial-roots":
        statuses = {unit["status"] for unit in health["coverage"]}
        if not {"complete", "failed"}.issubset(statuses) or not any(same_path(item["path"], case["missing"]) for item in health["diagnostics"]):
            errors.append("missing-root partial did not preserve healthy root and failure diagnostic")
    if case["name"] == "yarn-delegation":
        yarn = [unit for unit in health["coverage"] if unit["id"] == "yarn"]
        if len(yarn) != 1 or yarn[0]["status"] != "unsupported" or "delegation" not in yarn[0]["detail"]:
            errors.append("Yarn delegation did not remain explicitly unsupported")
    return errors


def queries(case):
    path = Path(case["bin"]) / "provider-calls.jsonl"
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []


def run(output):
    prepared = json.loads((output / "prepared.json").read_text(encoding="utf-8"))
    if digest(Path(__file__)) != prepared["driver_sha256"]:
        raise RuntimeError("driver changed after fixture preparation")
    run_root = output / "runs"
    run_root.mkdir(exist_ok=False)
    results = []
    for case in prepared["cases"]:
        documents, exits, records = {}, {}, []
        if tree(Path(case["fixtures"])) != case["fixture_manifest"]:
            raise RuntimeError("fixture changed before paired scans")
        for label, (relative, expected_hash) in BUILDS.items():
            executable = REPO / "dist/performance/cleanup-earth" / relative
            if digest(executable) != expected_hash:
                raise RuntimeError(f"unexpected {label} executable identity")
            for path, expected in case["provider_files"].items():
                if digest(Path(path)) != expected:
                    raise RuntimeError("provider identity changed")
            before = len(queries(case))
            destination = run_root / case["name"] / label
            destination.mkdir(parents=True)
            argv = [str(executable), *case["argv"]]
            with (destination / "stdout.json").open("wb") as stdout, (destination / "stderr.txt").open("wb") as stderr:
                child = subprocess.Popen(argv, cwd=case["cwd"], env=case["environment"], stdin=subprocess.DEVNULL,
                                         stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
                record = {"build": label, "argv": argv, "executable_sha256": expected_hash, **identity(child)}
                save(destination / "process.json", record)
                try:
                    child.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    save(destination / "timeout.json", {**record, "left_running": True, "forced_termination": False})
                    raise RuntimeError("owned scan exceeded deadline; left running, no forced termination")
            document = json.loads((destination / "stdout.json").read_text(encoding="utf-8"))
            errors = semantic_errors(case, document, child.returncode)
            calls = queries(case)[before:]
            allowed = {"go": [["version"], ["env", "GOMODCACHE"], ["env", "GOCACHE"]],
                       "npm": [["--version"], ["config", "get", "cache"]], "yarn": []}.get(case.get("provider"), [])
            # The fixture strips the resolved .js prefix before logging queries.
            if any(call not in allowed for call in calls):
                errors.append("unexpected provider command query")
            if (Path(case["bin"]) / "invoked.json").exists():
                errors.append("cleanup simulation branch invoked")
            immutable = tree(Path(case["fixtures"])) == case["fixture_manifest"]
            if not immutable:
                errors.append("fixture changed during scan")
            record.update(exit=child.returncode, errors=errors, provider_queries=calls,
                          fixture_unchanged=immutable, stdout_sha256=digest(destination / "stdout.json"),
                          natural_exit=True, target_count=len(document["data"]["plan"]["targets"]))
            save(destination / "result.json", record)
            documents[label], exits[label] = document, child.returncode
            records.append(record)
        equal = documents["B3"] == documents["C2"] and exits["B3"] == exits["C2"]
        results.append({"case": case["name"], "full_envelope_and_exit_equal": equal,
                        "stdout_bytes_equal": records[0]["stdout_sha256"] == records[1]["stdout_sha256"],
                        "pass": equal and not any(record["errors"] for record in records), "runs": records})
        save(output / "result.json", {"pass": all(item["pass"] for item in results), "cases": results,
                                      "normalization": "none; JSON object key order only", "performance_claim": False})
    print(json.dumps({"pass": all(item["pass"] for item in results),
                      "cases": [{"case": item["case"], "pass": item["pass"], "parity": item["full_envelope_and_exit_equal"],
                                 "errors": [record["errors"] for record in item["runs"]]} for item in results]}))
    return 0 if all(item["pass"] for item in results) else 1


def reassess(output):
    """Recheck saved evidence only; preserve the original run and driver files."""
    prepared = json.loads((output / "prepared.json").read_text(encoding="utf-8"))
    original = json.loads((output / "result.json").read_text(encoding="utf-8"))
    results = []
    semantic_messages = {"unexpected rule/path set or duplicate rules", "nested Python did not use nearest tox marker root"}
    for case, prior in zip(prepared["cases"], original["cases"], strict=True):
        assert case["name"] == prior["case"]
        documents, records = [], []
        for previous in prior["runs"]:
            source = output / "runs" / case["name"] / previous["build"] / "stdout.json"
            if digest(source) != previous["stdout_sha256"]:
                raise RuntimeError("saved output changed")
            document = json.loads(source.read_text(encoding="utf-8"))
            errors = semantic_errors(case, document, previous["exit"])
            errors.extend(error for error in previous["errors"] if error not in semantic_messages)
            immutable = tree(Path(case["fixtures"])) == case["fixture_manifest"]
            if not immutable or not previous["fixture_unchanged"] or not previous["natural_exit"]:
                errors.append("missing immutability or natural-exit evidence")
            records.append({"build": previous["build"], "exit": previous["exit"], "errors": errors,
                            "target_count": len(document["data"]["plan"]["targets"]), "fixture_unchanged": immutable})
            documents.append(document)
        equal = documents[0] == documents[1] and records[0]["exit"] == records[1]["exit"]
        results.append({"case": case["name"], "pass": equal and not any(record["errors"] for record in records),
                        "full_envelope_and_exit_equal": equal, "stdout_bytes_equal": prior["stdout_bytes_equal"], "runs": records})
    result = {"pass": all(item["pass"] for item in results), "cases": results, "subprocesses_started": 0,
              "driver_sha256": digest(Path(__file__)), "original_result_sha256": digest(output / "result.json"),
              "envelope_normalization": "none; JSON object key order only",
              "expected_path_comparison": "Windows separators, case, and extended-length prefix only"}
    shutil.copy2(__file__, output / "reassessment-driver.py")
    save(output / "reassessment.json", result)
    print(json.dumps(result))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["prepare", "run", "reassess"])
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    destination = arguments.output.resolve()
    if not destination.is_relative_to(REPO / "dist/performance/cleanup-earth"):
        raise SystemExit("output must remain within this workspace's ignored performance fixture directory")
    if os.name != "nt":
        raise SystemExit("this provider fixture protocol targets Windows")
    if arguments.mode == "prepare":
        prepare(destination)
    elif arguments.mode == "reassess":
        raise SystemExit(reassess(destination))
    else:
        raise SystemExit(run(destination))
