# Final static-audit convergence focused evidence

Date: 2026-08-30

This is implementer-produced evidence. Independent verification remains
pending.

## Final focused results

| Command | Exit | Result |
| --- | ---: | --- |
| `rtk cargo fmt -p devsweep-cli -- --check` | 0 | no diff |
| `rtk cargo test -p devsweep-cli i18n` | 0 | 10 passed |
| `rtk cargo test -p devsweep-cli tui::shell` | 0 | 4 passed |
| `rtk cargo test -p devsweep-cli tui::app` | 0 | 31 passed |
| `rtk cargo test -p devsweep-cli tui::runtime` | 0 | 11 passed |
| `rtk cargo test -p devsweep-cli tui` | 0 | 74 passed |
| `rtk npm test -- src/App.test.tsx src/app-shell/AppShell.test.tsx src/app-shell/operation-coordinator.test.ts src/i18n/index.test.ts src/styles.test.ts` (from `desktop/`) | 0 | 5 files, 49 passed |
| `rtk npm run lint` (from `desktop/`) | 0 | no findings |
| `rtk npm run typecheck` (from `desktop/`) | 0 | no findings |

Deterministic catalogue audit command:

```powershell
rtk python -X utf8 -c "import json,hashlib,pathlib; ..."
```

The audit removed only additive `shell.v1.*` keys before canonical
`json.dumps(..., sort_keys=True, separators=(',', ':'))` hashing. It returned:

```text
en 17 2ae2cd066c151192b86e52284faa2dc692649239a2c691af6c2fe27f3e85baa2
zh-CN 17 1988b6f0370a9cd38ece4859f861c54b861cb8e5b52754cc4f81d91a162d4485
exit=0
```

These match the recorded pre-addition baselines. The Chinese baseline already
contains the separately approved `command.clean.accelerator: null`; no other
legacy field changed.

## Repair evidence

1. `tui::app` first run: exit 1, 30 passed / 1 failed. The legacy cancellation
   test injected `JobCanceled` directly from Running, which is not a valid
   reducer transition. The test now performs the real `x` input ->
   `CancelJob`/Cancelling transition before the terminal event. Final app run:
   31/31. Full first-failure output:
   `final-static-convergence-tui-app-first-failure.log`.
2. Desktop IEC focused aggregate first run: exit 1, 48 passed / 1 failed at
   `App.test.tsx:477`, expecting the obsolete `500 MB` instead of rendered
   `500.0 MiB`. The exact assertion was corrected.
3. Desktop IEC focused aggregate second run: exit 1, 48 passed / 1 failed at
   `App.test.tsx:486`, expecting obsolete
   `500 MB + at least 128 MB`. Because the same failure category repeated, the
   diagnostic changed from point repair to a static `rg` enumeration of all
   `KB|MB|GB|TB` strings in every authorized consumer/test path. It identified
   exactly that remaining assertion. After correction, App was 23/23 and the
   focused aggregate was 49/49.
4. Full TUI first aggregate: exit 1, 73 passed / 1 failed. The real render used
   canonical `Save` while the legacy integration assertion expected `save`.
   It was corrected to the exact catalogue copy. Full first-failure output:
   `final-static-convergence-tui-render-first-failure.log`.
5. Full TUI second aggregate: exit 1, 73 passed / 1 failed. The same legacy
   render test expected the retired Chinese private-table copy
   `清理计划工作台`; a static comparison against the frozen exact catalogue
   matrix identified canonical `清理计划工作区`. After the exact correction,
   the focused render test was 1/1 and full TUI was 74/74. Full second-failure
   output: `final-static-convergence-tui-render-second-failure.log`.

No focused failure reached three attempts. No scope or Route expansion was
required.
