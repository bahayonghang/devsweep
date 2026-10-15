"""Preserve the final production web and native build, outside frontendDist."""
from pathlib import Path
import hashlib
import json
import shutil
import zipfile

root = Path(__file__).resolve().parents[3]
out = root / "dist/cleanup-home-final/native-fixed"
web = root / "desktop/dist"
assert sorted(path.name for path in web.iterdir()) == [
    "THIRD_PARTY_NOTICES.txt", "assets", "earth", "hud.html", "index.html"
], "Production frontendDist must contain only the main production build"
native = out / "devsweep-desktop-production.exe"
assert not native.exists()
source_manifests = [json.loads((root / f"dist/cleanup-home-final/{name}/source-manifest.json").read_text(encoding="utf-8"))
                    for name in ["source-fixed", "source-final"]]
source_maps = [{entry["path"]: entry["sha256"] for entry in manifest["entries"]} for manifest in source_manifests]
changed = {name for name in source_maps[0].keys() | source_maps[1].keys()
           if source_maps[0].get(name) != source_maps[1].get(name)}
assert changed == {"crates/devsweep-core/src/software/msix.rs", "crates/devsweep-core/src/software/mod.rs", ".cargo/config.toml"}, changed
assert ".cargo/config.toml" not in source_maps[1]
with zipfile.ZipFile(root / "dist/cleanup-home-final/source-fixed/working-sources.zip") as archive:
    assert archive.read(".cargo/config.toml") == b'[build]\ntarget-dir = "dist"\n'
shutil.copyfile(root / "dist/release/devsweep-desktop.exe", native)
shutil.copytree(web, out / "production-web")

def describe(path):
    return {"path": path.relative_to(root).as_posix(), "bytes": path.stat().st_size,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}

manifest = {
    "desktop": describe(native),
    "cli": describe(out / "devsweep-cli.exe"),
    "final_source": describe(root / "dist/cleanup-home-final/source-final/source-manifest.json"),
    "cli_source": describe(root / "dist/cleanup-home-final/source-fixed/source-manifest.json"),
    "source_difference": "MSIX cfg(test) isolation was added. The earlier snapshot also captured the temporary just-ci build.target-dir=dist config; the final snapshot excludes it and both release commands explicitly use --target-dir dist. Neither difference changes release behavior.",
    "web": [describe(path) for path in sorted((out / "production-web").rglob("*")) if path.is_file()],
    "build_logs": ["gates/cli-release-node-fixed.log", "gates/production-web-isolated.log", "gates/native-release-production.log"],
    "installed": False,
}
(out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"desktop": manifest["desktop"], "cli": manifest["cli"]}))
