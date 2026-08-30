Active task: .trellis/tasks/08-29-analyze-tui-desktop-treemap

VERIFICATION ROUND 5 — final locale-labeling confirmation, minimal scope. You are the trellis-check sub-agent again, same repo, same standing constraints.

## Why round 5
Your round-4 verdict failed AC3/AC4 solely because the reused profile persisted zh-CN, making the r3-01–r3-07 captures Chinese while labeled "en". That is fixed and re-run (exit 0): the driver now forces English through the real settings select before the first capture and audits the persisted store in-log (`force_en`: store `{"schema_version":1,"language":"en"}`); per-tag scan summaries in `evidence/native-20260831-round3/capture-log.jsonl` prove the UI language per capture ("Analysis complete…" for r3-01–r3-07 and the four DPI launches; "分析完成…" for r3-08); r3-08 re-switches to zh-CN with the store audited. Dimensions unchanged and reconciled (1800×1075 = 1440×860 CSS @ DPR 1.25; 1280/1000/488 for 1024/800/390; zh 390 = 488). The evidence index (`evidence/native-evidence.md`, round-5 section) documents all of it.

## Round-5 scope (do not expand)
1. Audit the log and screenshots for the language-per-capture claims (store audits, per-tag summaries, and the visible PNG content).
2. Confirm the round-3/4 conclusions still hold (nothing product-side changed; only the driver and the captured artifacts changed).
3. Update `evidence/independent-check-report.md` with a round-5 section and flip AC3/AC4 if satisfied.
4. Deliver the final overall PASS or FAIL verdict with per-AC statuses; if a genuinely blocking defect remains, report it precisely.
