# C1 CLI named-file termination and native console observations

Candidate CLI SHA-256:
`a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277`.
The subsequent C2 fix is desktop presentation only; this evidence identifies the
unchanged C1 CLI rather than claiming to have tested a different executable.

The new `tools/owned-console-smoke.py` launches a hidden private console, verifies
the expected executable hash and process creation identity, and checks that only
its helper/client occupy that console before any signal or input. Status uses a
new process group and targeted `CTRL_BREAK_EVENT`. Windows cannot restrict
`CTRL_C_EVENT` to a nonzero process group; therefore this verifies the same CLI
cooperative control handler through Ctrl+Break, not literal Ctrl+C. The CLI handles
these events in the same cancellation branch. See Microsoft's
[GenerateConsoleCtrlEvent contract](https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent).
No broadcast event or forced process termination was used.

## Named-file Status: pass

Command: `python tools/owned-console-smoke.py status <C1 devsweep.exe> <fresh output> --sha256 <hash above>`.
Generated evidence: `dist/performance/cleanup-earth/C1-runtime/console-status-01/`.

- Real release `status live --format ndjson --interval 1 --output <fresh file>`.
- Four reader steps, one line per 1.25 seconds; the file writer continued ahead.
  Reader offsets were 175, 13,642, 27,110 and 40,578 bytes, while file sizes were
  27,110, 40,578, 54,045 and 67,515 bytes respectively.
- Final file contained eight V1 records with contiguous sequences 0–7 and one
  operation ID: Started, six Snapshots, then exactly one Terminal with reason
  `canceled`. Slow reads returned the unchanged prefix sequences 0–3.
- Targeted cooperative signal to normal exit: 43.5 ms in this single observation;
  exit code 0, no stdout data, no matching owned client identity remained.
- This is a slow reader of a regular file, **not** simulated slow filesystem
  writes or a writer-backpressure test. Existing per-record flush/sync behavior
  was unchanged. Root's separate unread-pipe test covers the pipe scenario.

## Native TUI: partial observation, resize unavailable

Command: `python tools/owned-console-smoke.py tui <C1 devsweep.exe> <fresh output> --sha256 <hash above>`.
Generated evidence: `dist/performance/cleanup-earth/C1-runtime/console-tui-02/`.

This uses actual `CONIN$`/`CONOUT$` TTY handles in a hidden Windows console, not
ConPTY. A tiny disposable Node fixture, private home/settings/cache roots and a
System32-only PATH keep automatic discovery away from user cache roots. No clean,
maintenance or uninstall action was invoked.

- Startup scan completion was observed in the native Jobs/Logs view.
- During a two-second passive observation the screen stayed unchanged, private
  memory and handles were stable, and recorded client CPU time increased by
  15.625 ms. This is a short diagnostic, not an idle performance gate or evidence
  of the number of draw calls.
- `WriteConsoleInputW` delivered the help key and the real help overlay appeared
  after 53.2 ms in one observation. This is not p95 or burst-input evidence.
- Changing the console to 110 × 32 returned Windows error 87. The helper already
  grows the backing buffer before changing the window and shrinks it afterward.
  The exact hidden-console constraint was not established; resize remains
  **unavailable**, and the overall TUI case retains `pass: false`.
- Failure cleanup used only Escape then Q on the verified owned console; the
  client exited normally with code 0. No force was used to obtain that exit.

## Preserved failures and cleanup

The first TUI attempt, `console-tui-01`, failed in the harness before observation:
Python's console I/O wrapper rejected opening the console output buffer for
reading. Its first cleanup attempt preceded the client's console attachment, so
strict ownership checks refused input. Both facts remain in its failed result.

Recovery verified the exact recorded client PID and creation identity, attached a
short-lived recovery helper only to that already-owned private console, confirmed
exclusive helper/client membership and hidden visibility, and requested normal
Escape/Q exit. The client then disappeared from the process inventory. Console
reads now use explicit Win32 handles, and ownership checks allow a bounded initial
attachment wait without accepting any extra process.

All original/recovery helpers and clients were absent by matching PID **and**
creation identity at closeout; a subsequently reused PID was correctly excluded.
Raw closeout is `dist/performance/cleanup-earth/C1-runtime/console-ownership-closeout.json`.
Raw inventories, screens and process identities remain under ignored `dist/`.
Each run has a driver snapshot whose SHA matches its recorded driver hash.

## Verification boundary

`python -m py_compile tools/owned-console-smoke.py` and scoped whitespace checks
passed. No Rust/TypeScript product source or another owner's harness was changed.
The scheduler granted this bounded native-console slot; it was released after
ownership cleanup. Actual TUI resize, literal Ctrl+C, input-burst/p95 and universal
latency claims remain unverified by this evidence.
