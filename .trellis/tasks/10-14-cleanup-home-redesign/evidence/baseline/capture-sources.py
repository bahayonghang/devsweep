"""Freeze the pre-edit product tree and build inputs without changing source."""
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import zipfile

repo = Path(__file__).resolve().parents[5]
out = repo / "dist/cleanup-home-baseline/20261014-preedit"
out.mkdir(parents=True, exist_ok=False)

def git(*args):
    return subprocess.check_output(["git", *args], cwd=repo)

def digest(data):
    return hashlib.sha256(data).hexdigest()

diff = git("diff", "HEAD", "--binary")
(out / "initial-working-tree.patch").write_bytes(diff)
(out / "initial-status.txt").write_bytes(git("status", "--porcelain=v1"))
names = git("ls-files", "-z").decode().split("\0")
inputs = sorted(name for name in names if name and (name.startswith(("desktop/", "crates/", "resources/", "tools/")) or name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "justfile")))
rows = []
with zipfile.ZipFile(out / "source-snapshot.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    for name in inputs:
        data = (repo / name).read_bytes()
        rows.append({"path": name, "bytes": len(data), "sha256": digest(data)})
        archive.writestr(name, data)
manifest = {
    "schema_version": 1,
    "git_head": git("rev-parse", "HEAD").decode().strip(),
    "git_branch": git("branch", "--show-current").decode().strip(),
    "working_tree_diff_sha256": digest(diff),
    "source_inventory_sha256": digest(json.dumps(rows, sort_keys=True).encode()),
    "source_archive_sha256": digest((out / "source-snapshot.zip").read_bytes()),
    "source_files": rows,
    "host": {"platform": platform.platform(), "machine": platform.machine(), "python": platform.python_version()},
    "binary_preexisting": [],
}
for name in ("dist/release/devsweep-desktop.exe", "dist/release/devsweep.exe"):
    path = repo / name
    if path.exists():
        manifest["binary_preexisting"].append({"path": name, "sha256": digest(path.read_bytes()), "bytes": path.stat().st_size})
(out / "source-identity.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(json.dumps({key: value for key, value in manifest.items() if key != "source_files"}, indent=2))
