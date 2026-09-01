Active task: .trellis/tasks/08-29-analyze-tui-desktop-treemap

VERIFICATION ROUND 4 — final confirmation, minimal scope. You are the trellis-check sub-agent again, same repo (Windows, Git Bash shell, branch `dev`), same standing constraints. Your round-3 verdict failed AC3/AC4 on exactly one blocker: "a fresh, auditable 1440 CSS-pixel native capture from the corrected driver."

That blocker is now resolved by the main session running YOUR corrected driver (`evidence/capture-native-analyze-round3.mjs`, unchanged from your edits) to completion (exit 0; log `native-20260831-round3/capture-log.jsonl` ends with `done`):
- `r3-01-analyze-complete-en-1440.png` — 1800×1075 device px = 1440×860 CSS px at the real window's native DPR 1.25 (rendered before the Emulation override was cleared; width override requested explicitly per your edit).
- `r3-01-analyze-complete-en-{1024,800,390}.png` — 1280/1000/488 device px wide = 1024/800/390 CSS px at DPR 1.25.
- `r3-08-analyze-complete-zh-1440.png` (1800×1075) and `r3-08-analyze-complete-zh-390.png` (488×1075) — zh-CN at the same CSS sizes.
- `r3-dpi-*-analyze-complete-en-native-window.png` — the four scaling launches' unoverridden native-window captures (1000×750 at DPR 1/1.25/1.5/2 probes in-log).
- Log integrity: app launches (PIDs in-log), scan summaries, screenshot events, DPI probes, `store_audit`, and terminal `done` are all present in order.

## Round-4 scope (do not expand)
1. Audit the fresh captures: PNG pixel dimensions vs claimed CSS sizes and DPR, log event ordering and integrity, exe hash, and that the visible content is the real Analyze UI.
2. Re-confirm your round-3 audit conclusions still hold (nothing else changed since; product files untouched by the main session).
3. Update `evidence/independent-check-report.md` with a round-4 section and flip AC3/AC4 if the evidence satisfies them.
4. Deliver the final overall PASS or FAIL verdict with per-AC statuses. If genuinely blocking defects remain, report them precisely.

Run any read-only audits you need; re-running cargo/npm gates is optional this round (no product changes since your round-3 runs).
