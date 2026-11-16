"""Create an isolated native fallback fixture, never a host-action plan."""

from __future__ import annotations

import copy
import hashlib
import json
import time
from pathlib import Path


def digest(value: object) -> str:
    wire = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()
    return hashlib.sha256(wire).hexdigest()


def fingerprint(inventory: dict) -> str:
    return "sha256:" + digest([
        "devsweep.software.inventory.v1",
        inventory["version"],
        inventory["observed_at_unix_ms"],
        inventory["sources"],
        inventory["entries"],
    ])


def main() -> None:
    evidence = Path(__file__).resolve().parent
    repo = next(
        parent for parent in evidence.parents
        if (parent / "code_map.md").is_file() and (parent / "Cargo.toml").is_file()
    )
    source = repo / "crates/devsweep-core/tests/fixtures/software/inventory-evidence.json"
    inventory = json.loads(source.read_text(encoding="utf-8"))
    assert fingerprint(inventory) == inventory["fingerprint"], "core fixture encoding drift"
    template = inventory["entries"][0]
    inventory["entries"] = []
    for name, package, path, date in [
        ("Missing icon fixture", "MissingIcon", r"C:\DevSweep-evidence-missing-a9f0\icon.png", "2024-02-29"),
        ("Network icon fixture", "NetworkIcon", r"\\devsweep.invalid\share\icon.png", "invalid-date"),
    ]:
        entry = copy.deepcopy(template)
        entry["identity"]["package_full_name"] = package + "_1.0.0.0_x64__publisher"
        entry["id"] = "software:v1:msix:" + digest([
            "devsweep.software.identity.v1", entry["identity"]
        ])
        entry["display_name"] = name
        entry["publisher"] = "DevSweep native fixture"
        entry["icon"] = {"path": path, "index": 0}
        entry["installed_on"] = date
        inventory["entries"].append(entry)
    inventory["observed_at_unix_ms"] = time.time_ns() // 1_000_000
    inventory["fingerprint"] = fingerprint(inventory)
    output = evidence / "runtime" / "inventory-fallback.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")
    print(output)


if __name__ == "__main__":
    main()
