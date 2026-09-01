# Bare-TUI presentation-language restart trace

Date: 2026-08-30 (Asia/Shanghai)

## Isolation and executable

- `LOCALAPPDATA` was set only in each child PowerShell process to
  `evidence/tui-restart/isolated-localappdata`.
- The process working directory was the task-owned `evidence/tui-restart/session-root`.
- Executable: `target/debug/devsweep.exe`
- Bytes: `6808576`
- SHA-256: `7299D59F6355B9B6F7915E4EFB8BC2C53A68814393F4FCEE5EE0E2C236956344`
- No cleanup or global setting mutation was performed.

## Session 1: real settings action writes zh-CN

Command: bare `devsweep.exe` with the isolated `LOCALAPPDATA`.

Observed ConPTY sequence:

1. Initial frame contained `DevSweep  Clean`.
2. Sent `p`; the real frame opened `Language settings` with `> [e] English`
   and `[z] Simplified Chinese`.
3. Sent `z`, then `Enter`.
4. The next frame contained `DevSweep  清理`, proving the typed persistence
   result reached the App reducer and renderer.
5. Sent `q`; because the startup scan was active, the app displayed the
   cancel-and-wait quit guard. Sent `c`; the worker reached a terminal state,
   terminal restoration completed, and the process exited `0`.

The resulting file was exactly 39 bytes:

```json
{"schema_version":1,"language":"zh-CN"}
```

SHA-256: `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`.
The sidecar lock did not remain after the transaction.

Process/timing transcript: `session1-write-zh.transcript.txt`.

## Session 2: restart consumes the persisted choice

Command: bare `devsweep.exe`, same isolated `LOCALAPPDATA`, no language flag.

- The first ConPTY frame contained `DevSweep  清理` without another settings
  action.
- Quit used the same visible cancel-and-wait guard (`q`, then `c`) and exited
  `0` after worker termination and terminal restoration.

Process/timing transcript: `session2-restart-persisted-zh.transcript.txt`.

## Session 3: explicit CLI language is session-only

Command: `devsweep.exe --language en`, same isolated `LOCALAPPDATA`.

- The first ConPTY frame contained `DevSweep  Clean`, proving explicit CLI
  precedence over the persisted `zh-CN` choice for that session.
- No settings action was taken.
- Quit used the visible cancel-and-wait guard (`q`, then `c`) and exited `0`.
- The store remained byte-for-byte identical: 39 bytes and SHA-256
  `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`.

Process/timing transcript: `session3-explicit-en-session-only.transcript.txt`.

## Residue audit

After all three sessions:

- no repository-owned `devsweep.exe`, `cargo.exe`, or `rustc.exe` process was
  present;
- no listener owned by a process under this repository was present;
- `presentation-v1.lock` was absent;
- the exact V1 document remained present as retained task evidence.

PowerShell transcripts record exact commands, host process identifiers, and
start/end times. Native alternate-screen frames were observed directly through
ConPTY; PowerShell does not duplicate those frames into `Start-Transcript`, so
the frame strings above are an explicit observation trace rather than a claim
that the transcript files contain the terminal cells.
