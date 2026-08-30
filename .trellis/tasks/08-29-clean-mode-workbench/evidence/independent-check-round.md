# Independent check — 08-29-clean-mode-workbench

Verdict: **PASS**

Checker: CleanRecheck (trellis-check). No commit, no archive, no edits to
`.trellis/.gitignore`, `README.md`, `justfile`, or other task dirs.

Previous round REJECTED AC2/R4 because English ScanPage chrome sat under a
zh-CN shell. This recheck verifies the claimed catalogue cutover.

## Verdict

PASS on the prior bilingual REJECT. App now passes `presentation.locale` into
`CleanWorkbench`. Scan/Review/Preview/Confirm/Execute/search consume
`clean.v1.*`. zh-CN idle chrome is 项目/全局缓存/就绪/扫描. Regression tests
fail if Projects/Scan/Search/Ready remain. `implement.jsonl:4` now points at
the archived sizing design.

## Gate results (independent re-run)

| Gate | Result | Log |
| --- | --- | --- |
| `python -X utf8 ./.trellis/scripts/task.py validate .trellis/tasks/08-29-clean-mode-workbench` | PASS | `independent-check-recheck-task-validate.log` |
| `just desktop-web-check` | PASS 103 frontend tests + lint/typecheck/build | `independent-check-recheck-desktop-web-check.log` |
| `git diff --check` | PASS exit 0 (no whitespace errors) | `independent-check-recheck-git-diff-check.log` |
| `just ci` | SKIPPED | prior `independent-check-just-ci.log` is `ci complete` / `exit=0`; this recheck did not change Rust; `desktop-web-check` is 0 |

`just ci` is still the backend/TUI completion gate, but it is not required
again for this bilingual frontend repair.

## AC2 / R4 — bilingual desktop copy (PASS)

- `desktop/src/App.tsx:188` — `CleanWorkbench` receives `locale={presentation.locale}`.
- `desktop/src/modes/clean/CleanWorkbench.tsx:28,165-174` — required `locale`
  prop; ScanPage, search label, ReviewPage, ScanPreviewPage, ExecutePage, and
  ConfirmDialog all receive it. Search uses `clean.v1.search.label`.
- ScanPage / ReviewPage / ScanPreviewPage / ExecutePage / ConfirmDialog render
  chrome through `message(locale, "clean.v1.*")` rather than hardcoded
  Projects/Scan/Search/Ready.
- `desktop/src/App.test.tsx:94-102` — zh-CN App render asserts 扫描/项目/就绪
  and `queryByText` fails if Projects/Scan/Search/Ready appear.
- `desktop/src/pages/ScanPage.test.tsx:44-53` — same Chinese chrome vs English
  Projects/Scan/Ready.
- `evidence/screenshots/zh-CN-workbench-1024.webp` — shell 清理/语言/帮助;
  workbench 项目, 全局缓存, 就绪, 扫描; empty state 没有扫描结果 /
  运行扫描以复核清理目标。 Matches `resources/i18n/zh-CN.json` catalogue
  (`clean.v1.scope.projects` 项目, `clean.v1.action.scan` 扫描,
  `clean.v1.status.ready` 就绪).
- `implement.jsonl:4` —
  `.trellis/tasks/archive/2026-08/08-29-bound-sizing-concurrency/design.md`
  (file exists; `task.py validate` now passes).

## Residual gaps (not this REJECT)

- **TargetTable chrome:** `desktop/src/components/TargetTable.tsx:43-44` still
  hardcodes English `Target` / `Category` / `Capacity` / `Risk` / `Evidence`
  and `Select all executable targets`. Review/preview tables after a scan
  would still show those English headers. No matching `clean.v1.*` keys exist.
  Idle workbench (the prior REJECT surface) does not show the table.
- **AC3 scaling:** prior `evidence/native-cli-record.md` still
  `WAIVED/UNVERIFIED` for 100/125/150/200%. Not re-proven here.
- **TUI adapter not live:** `crates/devsweep-cli/src/tui/mod.rs` still
  `#[allow(dead_code)] mod modes`.
- **Dedicated CLI renderer unused:** `presentation/clean.rs` remains
  `#![allow(dead_code)]`.
- Working tree still contains out-of-ownership edits (`README.md`, `justfile`,
  `.trellis/.gitignore`). Left untouched per contract.
