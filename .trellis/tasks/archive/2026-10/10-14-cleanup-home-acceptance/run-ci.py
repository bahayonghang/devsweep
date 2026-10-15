"""Run the unchanged canonical gate with task-local build artifacts.

The repository's normal target is an externally managed symlink. A temporary
local Cargo config avoids inheriting CARGO_TARGET_DIR into scanner fixtures.
Only the exact config created by this invocation is removed afterward.
"""
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
config = root / ".cargo" / "config.toml"
content = '[build]\ntarget-dir = "dist"\n'
log_name = sys.argv[1] if len(sys.argv) == 2 else "just-ci-isolated.log"
if Path(log_name).name != log_name or not log_name.endswith(".log"):
    raise SystemExit("A plain .log filename is required")
log = root / "dist/cleanup-home-final/gates" / log_name
config.parent.mkdir(exist_ok=True)
if config.exists():
    raise SystemExit("Refusing to replace existing Cargo configuration")
with config.open("x", encoding="utf-8", newline="\n") as stream:
    stream.write(content)
try:
    with log.open("x", encoding="utf-8") as stream:
        result = subprocess.run(["just", "ci"], cwd=root, stdout=stream,
                                stderr=subprocess.STDOUT, check=False)
finally:
    if config.read_text(encoding="utf-8") != content:
        raise RuntimeError("Temporary Cargo configuration changed; preserve it for review")
    config.unlink()
print(f"just ci exit {result.returncode}; log: {log}")
sys.exit(result.returncode)
