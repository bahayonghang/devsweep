#!/usr/bin/env python3
"""List selectable Cleanup Targets from a Scan Report. Never execute cleanup."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load_scan_report(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(payload, dict) and payload.get("schema_version") == 1 and "data" in payload:
        payload = payload["data"]
    if not isinstance(payload, dict):
        raise ValueError("observation is not an object")
    return payload


def normalize_path(value: str | None) -> str | None:
    if not value:
        return None
    text = value
    if text.startswith("\\\\?\\"):
        text = text[4:]
    return text.replace("/", "\\")


def under_root(path: str | None, root: Path | None) -> bool:
    if path is None or root is None:
        return False
    try:
        candidate = Path(path)
        return candidate.resolve().is_relative_to(root.resolve())
    except (OSError, RuntimeError, ValueError):
        lowered = (normalize_path(path) or "").casefold()
        root_text = str(root).replace("/", "\\").casefold()
        return lowered.startswith(root_text)


def recoverable_class(target: dict[str, Any]) -> str:
    if not target.get("size_complete", True):
        if int(target.get("estimated_bytes") or 0) == 0:
            return "unknown"
        return "partial-lower-bound"
    return "verified"


def advice_for(target: dict[str, Any], exclude_root: Path | None) -> str:
    intent = (target.get("intent") or {}).get("type")
    if intent == "inspect_only":
        return "inspect-only"
    if target.get("path") is None:
        return "exclude"
    if under_root(target.get("path"), exclude_root):
        return "exclude"
    return "recommend"


def evidence_phrase(target: dict[str, Any]) -> str:
    rule = str(target.get("rule_id") or "")
    phrases = {
        "rust.target": "Cargo.toml marker; target/",
        "node.node_modules": "package.json marker; node_modules",
        "python.venv_dot": "Python marker; .venv",
        "python.__pycache__": "Python marker; __pycache__",
        "python.ruff_cache": "Python marker; .ruff_cache",
        "python.pytest_cache": "Python marker; .pytest_cache",
        "npm.cache.clean": "npm cache dir probe",
        "nuget.packages": "NuGet global packages",
        "cargo.home.inspect": "Cargo home; Inspect Only",
        "jetbrains.caches": "known JetBrains cache",
        "pip.cache.purge": "pip cache probe",
        "pnpm.store.prune": "pnpm store probe",
    }
    phrase = phrases.get(rule, rule)
    if not target.get("size_complete", True):
        kinds = {item.get("kind") for item in (target.get("sizing_warnings") or [])}
        if "entry_budget_exhausted" in kinds:
            phrase += "; size budget exhausted"
        if "path_unresolved" in kinds:
            phrase += "; path unresolved"
    return phrase


def select_targets(
    report: dict[str, Any],
    risks: set[str],
    exclude_root: Path | None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    plan = report.get("plan") or {}
    targets = list(plan.get("targets") or [])
    selected: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    for target in targets:
        risk = str(target.get("risk") or "")
        row = {
            "id": target.get("id"),
            "path": normalize_path(target.get("path")),
            "raw_path": target.get("path"),
            "risk": risk,
            "rule_id": target.get("rule_id"),
            "estimated_bytes": int(target.get("estimated_bytes") or 0),
            "class": recoverable_class(target),
            "evidence": evidence_phrase(target),
            "advice": advice_for(target, exclude_root),
        }
        if risk not in risks:
            row["skip_reason"] = "risk"
            skipped.append(row)
            continue
        if row["advice"] != "recommend":
            row["skip_reason"] = row["advice"]
            skipped.append(row)
            continue
        selected.append(row)
    selected.sort(key=lambda item: item["estimated_bytes"], reverse=True)
    return selected, skipped


def render_markdown(selected: list[dict[str, Any]]) -> str:
    lines = [
        "| id | path | evidence | risk | Estimated Recoverable class | advice |",
        "|---|---|---|---|---|---|",
    ]
    for row in selected:
        path = row["path"] or "(unresolved)"
        lines.append(
            f"| `{row['id']}` | `{path}` | {row['evidence']} | {row['risk']} | {row['class']} | {row['advice']} |"
        )
    if not selected:
        lines.append("| *(none)* | | | | | |")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Emit a confirmation list from a Scan Report.")
    parser.add_argument("observation", type=Path, help="Scan Report or CLI envelope JSON.")
    parser.add_argument("--risk", default="low,medium", help="Comma-separated risks to include.")
    parser.add_argument(
        "--exclude-root",
        type=Path,
        help="Do not select paths inside this tree (the current repo).",
    )
    parser.add_argument("--ids-out", type=Path, help="Write selected ids as a JSON array.")
    parser.add_argument("--json", action="store_true", help="Print JSON instead of markdown.")
    args = parser.parse_args()

    report = load_scan_report(args.observation)
    risks = {item.strip() for item in args.risk.split(",") if item.strip()}
    selected, skipped = select_targets(report, risks, args.exclude_root)
    payload = {
        "selected_count": len(selected),
        "skipped_count": len(skipped),
        "ids": [row["id"] for row in selected],
        "selected": selected,
        "skipped": skipped,
    }
    if args.ids_out:
        args.ids_out.write_text(json.dumps(payload["ids"], ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"Selectable Cleanup Targets: {len(selected)}")
        print(render_markdown(selected))
        print()
        print("等待确认. Do not run clean execute until the user confirms these ids.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
