# Native Evidence - CLI Contract and Localization

## Frozen environment

- Captured: `2026-08-30T01:48:52.8086944+08:00`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Windows Terminal package: `1.24.11911.0`
- Redirected shell: PowerShell Core `7.6.5`
- Culture/UI culture: `en-US` / `en-US`
- Windows system locale: `zh-CN`
- Active console code page: `65001`
- Binary: `target/debug/devsweep.exe`
- Binary bytes: `6707712`
- Binary SHA-256:
  `556F30789EAABC6AB247BE10A383B7F8D6546C87DC2A6CF3205C2F01B629E4C6`

No cleanup, uninstall, installation, system-setting change, or permanent delete
was run. Bare-TUI scans and provider probes were inspect-only. Every process was
allowed to finish or was canceled through the TUI's own `q` -> `c` flow; no
process was killed. Final process audit returned zero `devsweep` and zero
Windows Terminal processes.

## Matrix

| ID | Native command/scenario | Result | Direct evidence |
| --- | --- | --- | --- |
| N01 | Bare `devsweep` with redirected stdin/stdout | PASS: exit `2`; stdout empty; stderr was `tty_required: Bare devsweep requires interactive stdin and stdout...` | Captured with separate redirected stdin/stdout/stderr process handles. |
| N02 | Bare `devsweep` in real Windows Terminal | PASS: TUI rendered in a real terminal; an early `q` while the inspect-only scan was active displayed the wait/cancel safety dialog; the final run reached a terminal scan state, `q` returned exit `0` | `windows-terminal-bare-tui-20260830-v2.png`, `windows-terminal-bare-tui-exit-20260830.png`, `windows-terminal-bare-final-state-20260830.png`, `windows-terminal-bare-final-exit-20260830.png`, `bare-tui-exit-code-20260830.txt` |
| N03 | `status live --format human` in real Windows Terminal | PASS for this staged task: TTY precondition accepted; truthful unregistered handler returned `mode_unavailable`, exit `4`, code page 65001 | `windows-terminal-status-live-human-20260830.png` |
| N04 | Redirected `status live --format human` | PASS: exit `2`; stdout empty; stderr `tty_required: status live --format human requires interactive stdout; use --format ndjson` | Redirected process capture. |
| N05 | Redirected `status live --format ndjson` | PASS: exit `4`; stdout was one locale-neutral `status_terminal/producer_error` event with `error_code:"mode_unavailable"`; stderr empty | Redirected process capture. |
| N06 | Redirected `status snapshot --format json` | PASS: exit `4`; stdout contained one locale-neutral V1 envelope; stderr empty | Redirected process capture. |
| N07 | `--language zh-CN status snapshot --format json` | PASS: exit `2`; stdout remained a locale-neutral `invalid_cli` V1 envelope; stderr empty | Redirected process capture. |
| N08 | English `clean execute --help` | PASS: exit `0`; contextual usage showed value-less `--confirm`, required plan/digest, inherited language, human/json, and create-new output | Redirected process capture. |
| N09 | Chinese `--language zh-CN clean execute --help` in redirected PowerShell and Windows Terminal | PASS: exit `0`; contextual Chinese usage/options/constraints rendered correctly at code page 65001 | `windows-terminal-zh-help-long-path-20260830.png`; redirected capture. |
| N10 | Chinese Analyze invocation with a 355-character Unicode path | PASS: parser accepted the path without corruption; staged handler returned the expected localized `mode_unavailable`, exit `4` | `windows-terminal-long-path-20260830.png` |
| N11 | `--language zh-CN clean execute --help \| Select-Object -First 1` in Windows Terminal | PASS: downstream pipeline closed after the first line; process exited `0` and emitted only `DevSweep 中文帮助` to the visible sink | `windows-terminal-broken-pipe-20260830.png` |
| N12 | First `status snapshot --format json --output <task evidence file>` | PASS: exclusive create-new produced the failure envelope and exited `4`; file bytes `145`, SHA-256 `CF2BB398...D432C6` | `create-new-native-20260830.json` |
| N13 | Repeat N12 against the existing file | PASS: exit `6`; stdout empty; stderr `output_exists`; SHA-256 remained exactly `CF2BB398...D432C6` | Redirected process capture plus post-run hash. |
| N14 | Cancel/join and orphan audit | PASS: an early TUI quit attempt could not abandon an active scan; `c` requested cancellation and the worker process exited. A final clean run exited `0`. Final audits found zero `devsweep` and zero Windows Terminal processes | `windows-terminal-bare-hwnd-7934762-20260830.png`, `windows-terminal-bare-hwnd-8785196-20260830.png`, final process polls. |

The Status collector and Tauri runtime adapter are intentionally not registered
by this task. N03, N05, and N06 therefore prove the frozen parser, TTY, output,
exit, and locale-neutral failure contracts without claiming downstream runtime
collector behavior.

## Exact redirected outputs

### Status snapshot JSON

```json
{"command":"status.snapshot","data":null,"error":{"code":"mode_unavailable","message":null},"outcome":"failed","schema_version":1,"warnings":[]}
```

### Status live NDJSON

```json
{"data":{"error_code":"mode_unavailable","reason":"producer_error"},"emitted_at_unix_ms":0,"event":"status_terminal","operation_id":"not_dispatched","schema_version":1,"sequence":0}
```

### Chinese long-path failure

```text
mode_unavailable: 当前分阶段构建尚未接入 analyze.scan 命令。
LONG_PATH_EXIT=4
```

## Artifact integrity

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `bare-tui-exit-code-20260830.txt` | 17 | `FE7884F0D20ADDAECC5538A9B93F9050C2BEDE346E533C8D041128C9BBE298C9` |
| `windows-terminal-bare-tui-20260830-v2.png` | 429984 | `0D596BDC45652D2398EE45B920B3BA1E2C9D27BE6E5BCEE38DCB04991E3B017D` |
| `windows-terminal-bare-final-state-20260830.png` | 483314 | `E1A46BB357927A495546D5FB3DD381A19E0BC52C9C7150DA74C2B5E07C38E34D` |
| `windows-terminal-bare-final-exit-20260830.png` | 454768 | `3D08598A09FA25CE501502F91E572E6CCCB4EF9B9C4341A8BBD42F2BC414F2CF` |
| `windows-terminal-status-live-human-20260830.png` | 405732 | `5DDAFBEAA200F3BBD461960EAF5D5E7DA42E5B5044503C123B4B65E22C5FF5E7` |
| `windows-terminal-zh-help-long-path-20260830.png` | 437865 | `9A081BF09694C644AA8764A18D70BEC0F3851AD076B9D6C4D1359FBE03242CE9` |
| `windows-terminal-long-path-20260830.png` | 331500 | `778D4D5792992E0225CDD649BE2FAAA525CA412FE036C67176D357349655D4F6` |
| `windows-terminal-broken-pipe-20260830.png` | 265357 | `D880C8A3B203D6D2742AFC70BD2FFA4F7BE84B16AC55E5B2BBDA83207E028C8E` |
| `create-new-native-20260830.json` | 145 | `CF2BB398E0CE868254D9EE6D6E310521229A2597CB01B1FEA72D4D20F8D432C6` |
| `repair-round2-final-ci.log` | 25601 | `98C174F6505934AF0F0AF8D81DAC178C1E47D7C57FF5070641DCD9071E67878A` |
| `independent-check-final-ci.log` | 25601 | `87FE70834BD1A7AEFF83E2C601AAC46606CD7D067AD8D4A5EA59AA86E511A65B` |
| `precommit-final-just-ci.log` | 25601 | `801F1F4C9EE3C369C8845A02F87CE95E303D173F787EFD5B41F99679EC8A8C6F` |

`windows-terminal-bare-tui-20260830.png` records the first failed evidence
launcher: Windows Terminal interpreted unencoded semicolons as tab separators
and did not start the TUI. It is retained as FAIL evidence and is not used for
any PASS claim. The corrected launcher used PowerShell `-EncodedCommand`.

The PowerShell `Start-Transcript` file duplicates some CJK glyphs even though
the real Windows Terminal image displays them correctly. The transcript is used
only for command/exit chronology; screenshot and direct redirected UTF-8
captures are the glyph-fidelity evidence.

## Automated gate logs

- `repair-round2-final-ci.log`: implementation `just ci`, exit `0`.
- `independent-check-final-ci.log`: independent `trellis-check` `just ci`, exit
  `0`.
- `precommit-final-just-ci.log`: latest full-worktree pre-commit `just ci` after
  the backend specification consistency repair, exit `0`.
- Earlier failed CI/clippy cache files named by the implementation agent had
  already been rotated from `%LOCALAPPDATA%/rtk/tee` before evidence collation;
  they are not represented as retained logs. The failure and local corrections
  remain recorded in the agent transcript, but no absent log is claimed here.

## Result

All completion-required native scenarios owned by this task are `PASS`. No
native result is inferred from automation alone. Downstream Status/Tauri runtime
parity remains assigned to their owning tasks and is not claimed here.
