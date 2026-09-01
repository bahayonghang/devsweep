# Independent trellis-check report — round 2

Task: `.trellis/tasks/08-29-optimize-catalog-execution`
Checker: writable trellis-check (Shell + Write). Round 1 failed with signature `checker_workspace_readonly_no_shell`; method changed as required.
Date: 2026-08-31
Host: Windows 11 25H2 build 26200.9278, x64, Medium integrity (`S-1-16-8192`), `consent.exe` absent.
Branch: `dev`
Overall: **PASS**

Implementer `evidence/verification-report.md` was treated as a clue, not proof. All gates and native recapture below were re-run in this round. No product files were modified. No git commit.

## Round-1 skeptical items (freshly recaptured)

| Concern | This round |
| --- | --- |
| CLI log reused PID 27684; probe PID 67816 was a later mixed probe | **Closed.** Before `settings.search` run there were **zero** `SystemSettings.exe` processes. After confirmed run, **new** PID **48704**, path `C:\Windows\ImmersiveControlPanel\SystemSettings.exe`, start 2026-08-31 13:24:49, Get-Process HWND **393234**, EnumWindows HWNDs owned by 48704. Distinct from 27684 and 67816. Closed after identity re-check. Log: `independent-settings-native.log`. |
| Unregistered scheme `hInstApp=42` does not prove catalogue leak | **Closed.** CLI `plan`/`preview`/`run` of `ms-settings:search`, `ms-settings:evil`, `ms-settings:storagerecommendations`, `devsweep-not-a-protocol:test`, and extra `uri` JSON fields all exit 6 with **zero** plan files and **zero** journal records. Product URI is `&'static` from the three catalogue literals only. Log: `independent-rejections.log`, `independent-rejections-journal-inspect.log`. |

## R / AC / design trace

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 compile-time eight-id catalogue; unknown ids fail closed | **PASS** | `catalogue.rs` `ENTRIES: [_; 8]`. Tests: `independent-catalogue.log` (7 passed). |
| R2 plan one id, live preview digest, confirm, exclusive lock, one operation, no CleanupPlan | **PASS** | `MaintenancePlanV1` deny_unknown_fields + one `operation_id`. `EXECUTION_PERMIT` + sidecar lock. `optimize/` never constructs `CleanupPlan` (comment-only mention). Authorizer 8 / execution 14 / audit 6 passed. Native DNS digest `sha256:3deb8ab1399acf3f832ac4886c2e1b646ef3d7a9f6c2035283c4a8961d926b1d`. |
| R3 DNS System32/Sysnative, argv `[/flushdns]`, 10 s, no shell/PATH | **PASS** | `resolve_ipconfig` matches design algorithm (`IsWow64Process2` → Sysnative or `GetSystemDirectoryW`). `DNS_FLUSH_TIMEOUT` = 10 s. Windows 9 passed + 1 ignored. Native x64: `C:\Windows\System32\ipconfig.exe` exists; confirmed run exit 0; journal redacted. i686 identity test 1 passed (`independent-i686-identity.log`). |
| R4 guidance never dispatches; rejected categories fail closed | **PASS** | `ResolvedMaintenanceAction` has only `DnsFlush` and `SettingsHandoff`. Guidance plan → `GuidanceNotExecutable`. Native CLI hostile/guidance ids all exit 6, no files. |
| R5 cancel-before = canceled; after = observed/unknown; durable redacted audit | **PASS** (unit + native journals) | Execution tests cover cancel/timeout. Native DNS `validated → dispatch_started → adapter_succeeded → succeeded`. Settings `… → launched`. Journals: `REDACTED_OK`. Frozen CLI has no cancel flag. |
| R6 three Settings URIs, RtlGetVersion floors, launched-only | **PASS** | Floors 22000 / 22000 / 22624. Native `settings.search` **1 launched** exit 0; copy states launched is not completion. No signing; no Windows setting changed. |
| AC1 (R1, R4, R6) exhaustive snapshot + unknown/rejected cannot resolve | **PASS** | Catalogue 7 passed including eight-row snapshot, frozen URI literals, hostile ids. |
| AC2 (R2, R6) dry-run/execute byte-equivalent; stale/lock/unsupported fail before dispatch | **PASS** | Authorizer 8 + execution 14. Native no-confirm exit 2, wrong digest exit 3. |
| AC3 (R3, R5) exact binary/argv, bounds, one child, no UAC/admin | **PASS** | Windows 9 passed including native `ipconfig /flushdns`. Host Medium, consent none. |
| AC4 (R4, R5, R6) Settings allowlist + launched-only; guidance out of executor; audit; focused tests; `just ci` | **PASS** | CLI optimize 9 passed. `git diff --check` 0. `cargo fmt --check` 0. `rtk just ci` 0 (`ci complete`). Native launched pair + missing-drive failure control. |
| Design: exact Cargo.toml feature set | **PASS** | Diff adds exactly `Wdk_System_SystemServices`, `Win32_System_SystemInformation`, `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`. No `Win32_System_Registry`. ShellExecuteExW hand-declared. |
| Design: WOW64 algorithm | **PASS** | Source matches the five-step algorithm. x64 native uses System32. i686 identity test accepts System32 **or** Sysnative, never SysWOW64. |
| Design: guidance has no dispatch variant | **PASS** | Enum has two variants only. |
| Design: exclusive lock + one operation | **PASS** | `SidecarLock` + `EXECUTION_PERMIT` + plan exactly one id. |

## Focused gates (this round)

Honest filters include `tests::` because design.md uses a single `tests.rs`.

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-core optimize::tests::catalogue` | 0 (7 passed) | `evidence/independent-catalogue.log` |
| `rtk cargo test -p devsweep-core optimize::tests::authorizer` | 0 (8 passed) | `evidence/independent-authorizer.log` |
| `rtk cargo test -p devsweep-core optimize::tests::windows` | 0 (9 passed, 1 ignored) | `evidence/independent-windows.log` |
| `rtk cargo test -p devsweep-core optimize::tests::execution` | 0 (14 passed) | `evidence/independent-execution.log` |
| `rtk cargo test -p devsweep-core optimize::tests::audit` | 0 (6 passed) | `evidence/independent-audit.log` |
| `rtk cargo test -p devsweep-cli optimize` | 0 (9 passed) | `evidence/independent-cli-optimize.log` |
| `rtk git diff --check` | 0 | `evidence/independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `evidence/independent-cargo-fmt-check.log` |
| `rtk just ci` | 0 (`ci complete`) | `evidence/independent-just-ci.log` |
| `rtk cargo test -p devsweep-core --target i686-pc-windows-msvc optimize::tests::windows::resolved_ipconfig_identity` | 0 (1 passed) | `evidence/independent-i686-identity.log` |

`just desktop-build` was not run (this child has no desktop product files).

## Native recapture

Isolated `LOCALAPPDATA` under `evidence/independent-fixtures/`. Binary `target\debug\devsweep.exe` PE `amd64 (0x8664)` sha256 `7373435F9B8E1E8469AC977D68CBB80B1B9E2DFDC6FCF6743DB1DC6B611B77E6`.

### Host integrity

`evidence/independent-host-integrity.log`: Medium (`S-1-16-8192`), `consent.exe` none, no leftover `SystemSettings.exe` before recapture.

### DNS `dns.flush`

`evidence/independent-dns-native.log`

- Resolved path: `C:\Windows\System32\ipconfig.exe` (exists). Sysnative invisible from native x64 PowerShell (expected). SysWOW64 not used.
- Program/argv identity: adapter `dns_flush_request` = absolute program + exactly `[/flushdns]`; preview digest binds that identity.
- Digest: `sha256:3deb8ab1399acf3f832ac4886c2e1b646ef3d7a9f6c2035283c4a8961d926b1d`
- Exits: plan 0, preview 0, no-confirm 2, wrong-digest 3, confirmed run 0 (1 succeeded).
- Journal: `independent-fixtures/lappdata-dns/DevSweep/audit/v1/optimize.jsonl` — redacted (`REDACTED_OK`); transitions validated → dispatch_started → adapter_succeeded → succeeded.

### Settings `settings.search` launched pair

`evidence/independent-settings-native.log`

- Pre-run SystemSettings PIDs: **none**.
- Confirmed run: **1 launched**, exit 0. Human text: "A launched Settings page is not maintenance completion."
- New process: PID **48704**, path **`C:\Windows\ImmersiveControlPanel\SystemSettings.exe`**, start **2026-08-31 13:24:49**, HWND **393234** (plus EnumWindows HWNDs for that PID).
- Digest: `sha256:7307cb96c72c6e03a340bb3b6ddd3ff90b127d0659f9b66c12453dbdfd8a7479`
- Journal redacted; terminal `launched`.
- Close: CloseMainWindow returned false (HWND 0 at close time); force-kill after path+PID identity re-check; process gone. No leftover.

### Launch-failure control (not a catalogue id)

The ignored mixed probe (`native_settings_shell_execute_probe`) printed allowlisted `hInstApp=42` then **hung >60s** on the unregistered-scheme call. Test process killed (cargo exit **-1**). That hang is **not** used as launch proof and **not** used as catalogue-leak proof. Log: `independent-settings-probe-failure.log`.

Method change (mandatory after hang): P/Invoke `ShellExecuteExW` with the **same** flags (verb `open`, `SEE_MASK_FLAG_NO_UI`, no params/cwd), **only** `!:\devsweep-optimize-missing-control`. Result: `succeeded=False hInstApp=2 last_error=2` (`SE_ERR_FNF`). Struct size 112 on x64. Log: `independent-settings-failure-control.log`.

### Hostile / unregistered URIs cannot enter plan/preview/run

`evidence/independent-rejections.log` (harness exit 1 was an overly strict "journal file must not exist" check; product behavior is fail-closed):

- `optimize plan` of URI-as-id (`ms-settings:search`, `ms-settings:evil`, …), unregistered scheme, guidance, security/UAC/firewall/update/registry, `cmd.exe /c calc`, `ipconfig`, `DNS.FLUSH`: all **exit 6**, **0 files written**.
- Preview of extra `uri` field: `invalid_optimize_plan` exit 6.
- Preview/run of `operation_id=ms-settings:search` and `devsweep-not-a-protocol:test`: `invalid_optimize_selection` exit 6.
- `optimize run` of an unknown id opens the exclusive store **before** `construct_validated_action` (empty `optimize.jsonl`, 0 bytes, no records, no `dispatch_started`). That is lock+recovery, not adapter dispatch. Inspect: `independent-rejections-journal-inspect.log`.

## UNVERIFIED ledger

| Item | Completion-required? | Blocker |
| --- | --- | --- |
| Native CLI cancel-before / cancel-after | No | Frozen grammar has no cancel flag. Unit tests own the state machine. |
| Native DNS timeout / `unknown_after_dispatch` | No | Real `ipconfig /flushdns` finishes inside 10 s. Forcing a hang would change the approved action. |
| Native unsupported-build Settings id | No | Host RtlGetVersion build 26200 ≥ 22624. |
| Native live WMI sample of `ipconfig.exe` | No | Child lifetime shorter than poll. Runner still uses Job Object + separate program/argv. |
| Native launch of `storagerecommendations` / `energyrecommendations` | No | Same adapter and literal-URI allowlist; one clean `settings.search` handoff is sufficient. Extra pages would still not be completion. |
| Native i686 CLI confirmed `dns.flush` this round | No | i686 identity unit test re-run (1 passed). Full i686 CLI flushdns was implementer-owned; algorithm is compile-time. |
| Ignored unregistered-scheme `ShellExecuteExW` completing | No | Host can succeed **or hang**; not a catalogue path. Failure control recaptured via missing-drive instead. |

## Product defects / self-fixes

None. No contract was weakened. Out-of-scope dirty files (`README.md`, `justfile`, other `.trellis/tasks/*`) were not edited. `.trellis/.gitignore` was not touched. No `git add -f .trellis/`.

CLI module-root wiring (`commands/mod.rs` dispatch + `presentation/mod.rs` visibility) is the same pattern as Software mode and is required for the frozen handler to be reachable. Not treated as a defect.

## Overall

**PASS**
