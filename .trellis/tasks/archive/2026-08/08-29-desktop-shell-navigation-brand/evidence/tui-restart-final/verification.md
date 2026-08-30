# Final-source bare-TUI language restart trace

Date: 2026-08-30 (Asia/Shanghai)

- Final debug executable: `target/debug/devsweep.exe`
- Bytes: `6809600`
- SHA-256: `7AD23A3067BD08CBB3D640AA2F2CF18802D0869B23FBC71DACF04A78330CE963`
- Process-local `LOCALAPPDATA`: this directory's `isolated-localappdata`
- Working directory: this directory's `session-root`

## Direct ConPTY observations

1. Bare session started in English and the first frame displayed both
   `DevSweep  Clean` and the user-visible `[p] Language` action.
2. Keys `p`, `z`, `Enter` opened the real `Language settings` surface, selected
   `Simplified Chinese`, persisted it through the runtime effect, and changed
   the frame to `DevSweep  清理` with `[p] 语言`.
3. The resulting store was exactly 39 bytes:
   `{"schema_version":1,"language":"zh-CN"}`. Its SHA-256 was
   `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`.
4. A second bare process with no explicit language opened directly as
   `DevSweep  清理` with `[p] 语言`.
5. A third process with `--language en` opened as `DevSweep  Clean` with
   `[p] Language`. No settings action was taken; the stored bytes and SHA-256
   remained identical, proving the CLI choice was session-only.
6. Each session used the visible `q` then `c` cancel-and-wait path while its
   startup scan was active. Each process restored the terminal and exited `0`.

The three corresponding process/timing transcripts are:

- `session1-write-zh.transcript.txt`
- `session2-restart-persisted-zh.transcript.txt`
- `session3-explicit-en-session-only.transcript.txt`

PowerShell records process commands and timestamps but does not duplicate a
native alternate-screen buffer into `Start-Transcript`; the frame strings above
are direct ConPTY observations. After all sessions, the sidecar lock was absent,
there were zero repository-owned DevSweep/Cargo/Rust processes, and there were
zero repository-owned listeners. No cleanup or user-global setting mutation was
performed.
