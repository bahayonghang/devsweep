#!/usr/bin/env python3
"""Create untrusted Cleanup Plans from selected ids. Never execute."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def load_ids(path: Path) -> list[str]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(payload, dict):
        payload = payload.get("ids")
    if not isinstance(payload, list) or not all(isinstance(item, str) for item in payload):
        raise ValueError("ids file must be a JSON array of strings")
    return payload


def extract_invalid_id(stderr: str, batch: list[str]) -> str | None:
    hits = [item for item in batch if f"invalid target {item}:" in stderr]
    if hits:
        return max(hits, key=len)
    return None


def batches(ids: list[str], budget: int) -> list[list[str]]:
    groups: list[list[str]] = []
    current: list[str] = []
    length = 0
    for item in ids:
        extra = len(item) + 3
        if current and length + extra > budget:
            groups.append(current)
            current = [item]
            length = extra
        else:
            current.append(item)
            length += extra
    if current:
        groups.append(current)
    return groups


def try_plan(devsweep: Path, observation: Path, batch: list[str], output: Path) -> subprocess.CompletedProcess[str]:
    argv = [
        str(devsweep),
        "clean",
        "plan",
        "--observation",
        str(observation),
        "--select",
        *batch,
        "--output",
        str(output),
    ]
    return subprocess.run(argv, capture_output=True, text=True, check=False)


def main() -> int:
    parser = argparse.ArgumentParser(description="Save batched untrusted Cleanup Plans.")
    parser.add_argument("--devsweep", required=True, type=Path, help="Global DevSweep executable.")
    parser.add_argument("--observation", required=True, type=Path)
    parser.add_argument("--ids", required=True, type=Path, help="JSON array of target ids.")
    parser.add_argument("--output-prefix", required=True, type=Path)
    parser.add_argument("--argv-budget", type=int, default=20000)
    args = parser.parse_args()

    remaining = load_ids(args.ids)
    skipped: list[dict[str, str]] = []
    plan_files: list[str] = []
    index = 0

    while remaining:
        batch = batches(remaining, args.argv_budget)[0]
        current = list(batch)
        index += 1
        output = Path(f"{args.output_prefix}-{index}.json")
        if output.exists():
            print(f"output exists: {output}", file=sys.stderr)
            return 2
        while current:
            result = try_plan(args.devsweep, args.observation, current, output)
            if result.returncode == 0:
                plan_files.append(str(output))
                done = set(current)
                remaining = [item for item in remaining if item not in done]
                break
            invalid = extract_invalid_id(result.stderr, current)
            if invalid is None and len(current) == 1:
                invalid = current[0]
            if invalid is None:
                print(result.stderr or result.stdout, file=sys.stderr)
                return result.returncode or 2
            skipped.append({"id": invalid, "stderr": (result.stderr or "").strip()[:400]})
            current = [item for item in current if item != invalid]
            remaining = [item for item in remaining if item != invalid]
        else:
            continue

    print(
        json.dumps(
            {"ok": True, "plans": plan_files, "skipped_invalid": skipped},
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
