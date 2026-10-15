"""Summarize retained pre-edit artifacts; never infer an unobserved pass."""
import hashlib
import json
from pathlib import Path
import subprocess

repo = Path(__file__).resolve().parents[5]
evidence = Path(__file__).resolve().parent
out = repo / "dist/cleanup-home-baseline/20261014-preedit"
node = Path(r"C:\Users\lyh\AppData\Local\mise\installs\node\22.23.2\node.exe")
npm = node.parent / "node_modules/npm/bin/npm-cli.js"
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def inventory(directory):
    return [{"path": p.relative_to(repo).as_posix(), "bytes": p.stat().st_size, "sha256": digest(p)}
            for p in sorted(directory.rglob("*")) if p.is_file()]
source = json.loads((out / "source-identity.json").read_text())
frozen = out / "frozen-build-manifest.json"
source_drift = (json.loads(frozen.read_text())["source_drift_at_finalize"] if frozen.exists() else
                [row["path"] for row in source["source_files"] if digest(repo / row["path"]) != row["sha256"]])
manifest = {
    "schema_version": 1,
    "evidence_root": out.relative_to(repo).as_posix(),
    "git_head": source["git_head"],
    "git_branch": source["git_branch"],
    "source_inventory_sha256": source["source_inventory_sha256"],
    "source_archive_sha256": source["source_archive_sha256"],
    "initial_working_tree_diff_sha256": source["working_tree_diff_sha256"],
    "source_drift_at_finalize": source_drift,
    "source_freeze_note": "Drift checked at build freeze before A product edits; later evidence uses preserved outputs only.",
    "source_note": "Initial git status reports Cargo.toml M, but normalized git diff HEAD is empty. Exact working bytes are in the source archive and per-file hash inventory.",
    "runtime": {
        "node_executable": str(node),
        "node": subprocess.check_output([node, "--version"], text=True).strip(),
        "npm_entrypoint": str(npm),
        "npm": subprocess.check_output([node, npm, "--version"], text=True).strip(),
        "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "warning": "mise exec node@22 fell back to PATH Node 26.7.0 after DNS failure; builds used the explicit installed Node 22.23.2 and bundled npm 10.9.8.",
    },
    "host": source["host"],
    "builds": {name: inventory(out / name) for name in ["production-web", "clean-benchmark-web", "native"]},
    "method": {
        "benchmark_driver": "tools/clean-render-benchmark.mjs",
        "harness": "desktop/src/modes/clean/render-benchmark.tsx",
        "warmups": 5,
        "samples": 30,
        "percentile": "nearest-rank p95",
        "fixture_counts": [100, 1000, 10000],
        "scope": "Synthetic reports only; no user scans, cleanup, uninstall or maintenance.",
    },
    "benchmark_runs": [],
    "native_status": "NOT_RUN",
}
for name in ["clean-render-measure", "clean-render-measure-retry", "clean-render-measure-final", "clean-render-visual"]:
    directory = out / name
    if not directory.exists():
        continue
    record = directory / "record.json"
    teardown = directory / "teardown.json"
    item = {"directory": directory.relative_to(repo).as_posix(), "status": "CAPTURED" if record.exists() else "FAILED_STARTUP"}
    if teardown.exists(): item["teardown"] = json.loads(teardown.read_text())
    if record.exists(): item["record_sha256"] = digest(record)
    if name == "clean-render-measure-retry": item["qualification"] = "Initial successful run overlapped release compilation; retain as diagnostic, prefer final run."
    manifest["benchmark_runs"].append(item)
native = out / "native-captures-final"
if (native / "identity.json").exists():
    manifest["native_status"] = "CAPTURED" if (native / "no-scan-assertion.json").exists() else "PARTIAL"
    manifest["native_files"] = inventory(native)
    manifest["native_note"] = "Final captures use DPI-aware client geometry. Earlier native-captures and native-captures-corrected remain retained diagnostics for DPI virtualization and fractional-pixel rounding; do not use their filename dimensions as actual viewport dimensions."
    manifest["native_geometry"] = [{"name": p.name, **{key: json.loads(p.read_text(encoding="utf-8"))["page"][key] for key in ["width", "height", "dpr", "overflow"]}} for p in sorted(native.glob("*x*.json"))]
native_summary = out / "native-launch/summary.json"
if native_summary.exists(): manifest["native_launch_summary"] = json.loads(native_summary.read_text(encoding="utf-8"))
manifest["raw_records"] = [str(p.relative_to(repo)).replace("\\", "/") for p in out.glob("*.log")]
(evidence / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"manifest": str(evidence / "manifest.json"), "source_drift": source_drift, "native_status": manifest["native_status"]}))
