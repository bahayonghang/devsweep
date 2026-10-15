# Supplemental Clean flow screenshots

Four running fixture screenshots were captured and visually inspected at 1350×900 CSS pixels, DPR 1, dark theme, English locale. All four have zero horizontal document overflow. Raw files and geometry are in `dist/cleanup-home-final/fixture-flows/`; `fixture-flows.json` records source and artifact hashes.

| File | Observed state |
| --- | --- |
| `canceled.png` | Global scan canceled, results remain non-selectable, project scan remains independently available. |
| `preview.png` | One explicitly selected project artifact, dry-run result labeled not executed, exact preview digest and separate confirmation action. |
| `confirmation.png` | Modal second confirmation, preview digest visible, cancel and execute controls. |
| `outcome.png` | In-memory fixture result shows one successful move, estimates and the qualification that capacity is available after emptying trash. |

`capture-flows.mjs` drives the actual React route in Vite fixture mode. The first archived scan deliberately waits for cancellation; the project scan then completes, one target is explicitly selected, and the archived dry-run/execute replay is used. The fixture executor returns an in-memory report and does not reach Tauri, spawn a cleanup command, move a file, or mutate a user cache. These archived frontend fixtures do not prove production cleanup authority.

The owned Chrome process used a unique task profile and an OS-assigned loopback debugging port; the Vite server also used a unique loopback port. Teardown records PID 60404, exit code 0 and no forced termination. No native driver/profile was reused.

Reproduction from the repository root with a new output directory:

```powershell
& 'C:/Users/lyh/AppData/Local/mise/installs/node/22.23.2/node.exe' .trellis/tasks/10-14-cleanup-home-acceptance/evidence/capture-flows.mjs dist/cleanup-home-final/fixture-flows-new fixture
```

These four captures supplement D's 23-case layout/theme/locale matrix. They do not cover empty/error/oversized screenshots, native Windows UI, actual screen-reader behavior, or packaged execution. The home performance harness separately captured and visually checked the oversized display state after its measurement window; see `home-performance.md` for its component-only limits.
