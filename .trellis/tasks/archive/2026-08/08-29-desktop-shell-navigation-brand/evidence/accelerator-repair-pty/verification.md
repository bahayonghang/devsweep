# Chinese Clean accelerator corrective PTY verification

Date: 2026-08-30

## Isolation and binary

- Binary: `target/debug/devsweep.exe`
- SHA-256: `A4F8510574AC204AEEE9604D5394551791FA0F7F089044D9C9934B45E5878768`
- Length: 6,809,600 bytes
- Process-only `LOCALAPPDATA`: `evidence/accelerator-repair-pty/isolated-localappdata`
- Working directory: `evidence/accelerator-repair-pty/session-root`
- Invocation: `devsweep.exe --language zh-CN`
- The ConPTY was resized to 120 columns by 35 rows before launch. The emitted
  resize sequence was `CSI 8;35;120t`, allowing the complete navigation header
  and footer to be inspected rather than the compact 80-column variant.

## Direct ConPTY observations

1. The first final-source frame displayed `DevSweep  清理计划工作台  |  清理`.
   The Chinese Clean label remained visible and no `[Q]` appeared anywhere in
   the full header.
2. The footer displayed `[q] Quit`; the Clean action remained `[c] Clean`.
3. Sending the single key `q` did not navigate or invoke Clean. It opened the
   `Active job running` Quit guard with `w keep waiting`, `c request cancel and
   wait`, and `Esc stay in the app`.
4. Sending `c` requested cancellation, joined the active scan worker, and the
   process exited normally with code 0.

The PowerShell transcript at `final-zh-q-quit.transcript.txt` records the exact
isolated invocation, process identity, timestamps, and normal return. As
expected for an alternate-screen terminal application, PowerShell transcript
does not serialize the rendered cells; the rendered-frame facts above are the
direct ConPTY observation captured by the execution harness.

## Post-run residue and persistence boundary

- `devsweep` process count: 0
- owned listener count: 0
- task-owned lock file count: 0
- isolated `LOCALAPPDATA` file count: 0

The empty isolated store is expected: an explicit `--language zh-CN` override
is session-only and this trace did not invoke the Settings persistence action.

## Catalogue byte/field audit

- Previous SHA-256: `B806440A1AD8D664B46265693BD751D4E117146205D9DB59A9DE911601028E3A`
- Corrected SHA-256: `F7ED1FDCF88AEBA6EDD03FE502758E51A12B84CE450D89CECC475C31243F8D32`
- Corrected length: 3,040 bytes
- Recursive JSON comparison found exactly one semantic difference:
  `messages.command.clean.accelerator: "Q" -> null`.
- Replacing only the original byte sequence `"accelerator": "Q"` with
  `"accelerator": null` reproduces the corrected file byte-for-byte.

