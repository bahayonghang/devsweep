"""Freeze final working sources, including new files, without touching Git state."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys
import zipfile

root = Path(__file__).resolve().parents[3]
out = root / "dist/cleanup-home-final" / sys.argv[1]
if out.parent != root / "dist/cleanup-home-final":
    raise SystemExit("Use a plain new snapshot directory name")
out.mkdir(exist_ok=False)
paths = subprocess.check_output(
    ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    cwd=root,
).decode("utf-8").split("\0")
paths = sorted(set(p for p in paths if p and not p.startswith((".trellis/", ".agents/", ".codex/"))))
entries = []
with zipfile.ZipFile(out / "working-sources.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    for name in paths:
        path = root / name
        if not path.is_file():
            continue
        data = path.read_bytes()
        entries.append({"path": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        archive.writestr(name, data)
manifest = {
    "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
    "description": "Tracked and nonignored untracked working product sources; task/process scaffolding excluded. This is not a commit.",
    "entries": entries,
}
(out / "source-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
for name in ["source-manifest.json", "working-sources.zip"]:
    print(name, hashlib.sha256((out / name).read_bytes()).hexdigest())
