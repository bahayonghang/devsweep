Active task: .trellis/tasks/08-29-mole-inspired-cli-windows-desktop-redesign

You are performing the coordination-parent acceptance review for the Analyze mode parent. This is NOT product implementation: you review commit history and archived child evidence, then record the parent's acceptance verdict. Do NOT spawn sub-agents, do NOT commit, do NOT touch product files, protected dirty files (`.trellis/.gitignore`, `README.md`, `justfile`), or any task directory except `.trellis/tasks/08-29-analyze-mode/`.

## Scope
1. Read `.trellis/tasks/08-29-analyze-mode/prd.md`, `design.md`, `implement.md`, and `task.json` (children: `08-29-analyze-core-ipc`, `08-29-analyze-tui-desktop-treemap`).
2. Trace every parent R/AC clause to the archived children's evidence under `.trellis/tasks/archive/2026-08/08-29-analyze-core-ipc/` and `.trellis/tasks/archive/2026-08/08-29-analyze-tui-desktop-treemap/` (prd/design/implement/evidence; render-budget JSONs, native-evidence.md, independent-check-report.md with final PASS AC1–AC4, gate logs).
3. Verify from git history: product commit `e934f89` (Analyze core/CLI/IPC), `7117876` (Analyze TUI + desktop treemap), and the two independent `chore(task): archive` commits; confirm no cleanup authority was added to Analyze (no CleanupPlan conversion, no deletion/trash actions anywhere in the Analyze surfaces), the frozen CLI contract was not edited, and the shared shell was not weakened.
4. If a clause is genuinely unsatisfied, record it as a FAIL item with the exact gap; do not soften verdicts.

## Output
Write `.trellis/tasks/08-29-analyze-mode/acceptance.md` containing: date, reviewer role (trellis-check), per-requirement verdict table (R1..Rn / AC1..ACn with PASS/FAIL + evidence pointer), the cross-child integration checks (contract frozen, DTO unchanged, cancellation lifecycle consistent across CLI/TUI/desktop, bilingual catalogue consistency), any observed deviations with justification, and an overall `PASS` or `FAIL` line. Also append one reading-journal line per file consulted to `.trellis/tasks/08-29-analyze-mode/check.jsonl` ({"file": ..., "reason": ...} format).

## Report back
Final message: overall PASS or FAIL, the per-AC verdicts, and the acceptance.md path.
