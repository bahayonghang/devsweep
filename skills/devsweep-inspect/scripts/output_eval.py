#!/usr/bin/env python3
"""Check recommendation fixtures against forbidden execute patterns."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load_cases(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("output_cases.json must be an object")
    return payload


def evaluate_text(text: str, required: list[str], forbidden: list[str]) -> dict[str, Any]:
    missing = [term for term in required if term not in text]
    hits = [pat for pat in forbidden if pat in text]
    ok = not missing and not hits
    return {"ok": ok, "missing_required": missing, "forbidden_hits": hits}


def main() -> int:
    parser = argparse.ArgumentParser(description="Evaluate recommend-only output fixtures.")
    parser.add_argument("skill_dir", nargs="?", default=".", help="Skill directory.")
    parser.add_argument(
        "--cases",
        default="evals/output_cases.json",
        help="Output case JSON path relative to the skill directory.",
    )
    parser.add_argument("--output", "-o", help="Optional JSON report path.")
    args = parser.parse_args()

    root = Path(args.skill_dir).resolve()
    cases_path = Path(args.cases)
    if not cases_path.is_absolute():
        cases_path = root / cases_path
    cases = load_cases(cases_path)
    required = list(cases.get("required_terms", []))
    forbidden = list(cases.get("forbidden_patterns", []))
    results: list[dict[str, Any]] = []
    failures: list[dict[str, Any]] = []

    for fixture in cases.get("fixtures", []):
        rel = str(fixture.get("path", ""))
        expect = str(fixture.get("expect", "pass"))
        text = (root / rel).read_text(encoding="utf-8")
        check = evaluate_text(text, required, forbidden)
        passed = check["ok"] if expect == "pass" else not check["ok"]
        record = {
            "path": rel,
            "expect": expect,
            "passed": passed,
            **check,
        }
        results.append(record)
        if not passed:
            failures.append(record)

    report = {
        "ok": not failures,
        "summary": {"total": len(results), "passed": len(results) - len(failures)},
        "failures": failures,
        "results": results,
    }
    rendered = json.dumps(report, ensure_ascii=False, indent=2)
    if args.output:
        output = Path(args.output)
        if not output.is_absolute():
            output = root / output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    return 0 if report["ok"] else 2


if __name__ == "__main__":
    sys.exit(main())
