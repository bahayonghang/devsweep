# Optimize catalogue/execution — implementer verification report

Implementer evidence only. This is **not** an independent trellis-check PASS.

Task: `.trellis/tasks/08-29-optimize-catalog-execution`
Date: 2026-08-31
Host: Windows 11 25H2 build 26200.9278, x64, Medium integrity (`S-1-16-8192`), `consent.exe` absent. No signing, no UAC, no new dependencies.

## Product gaps closed this turn

Prior native Settings `launched=0 failed=1 exit=6` was **adapter ABI**, not unsigned-binary host policy.

- `windows-sys` 0.59 `SHELLEXECUTEINFOW` is `repr(C)` / 112 bytes on x64 and `repr(C, packed(1))` / 60 bytes on x86. The previous x64-only size assert of 112 would not compile for i686.
- After matching that ABI, allowlisted `ms-settings:search` returns `ShellExecuteExW` success (`hInstApp=42`, `GetLastError=0`, `hProcess=0`).
- CLI confirmed run reports **launched** (never completion) with a durable redacted journal.
- WOW64 identity test now accepts `System32` **or** `Sysnative` (never `SysWOW64`), matching design.md.

No `Win32_System_Registry` feature was added. `ShellExecuteExW` remains a hand-declared `shell32` entry with the documented layout.

## R / AC trace

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 / AC1 exhaustive eight-id catalogue; unknown/rejected cannot resolve | PASS | `evidence/01-rtk-core-tests-catalogue.log` (7 passed). Native list: `evidence/native/01-list.log`. Hostile plan ids: `evidence/native/13-rejections-recapture.log` (all exit 6, no files). |
| R2 / AC2 plan+preview digest, confirm, lock, one operation, no CleanupPlan | PASS | Authorizer 8 passed (`02-rtk-core-tests-authorizer.log`); execution 14 passed (`04-rtk-core-tests-execution.log`); audit 6 passed (`04c-rtk-core-tests-audit.log`). Native DNS digest `sha256:3deb8ab1399acf3f832ac4886c2e1b646ef3d7a9f6c2035283c4a8961d926b1d` (`12-dns-run-recapture.log`). Settings digest `sha256:7307cb96c72c6e03a340bb3b6ddd3ff90b127d0659f9b66c12453dbdfd8a7479` (`11-settings-cli-run.log`). |
| R3 / AC3 DNS System32/Sysnative, argv `[/flushdns]`, 10 s, no shell/PATH | PASS | Windows 9 passed + native `ipconfig /flushdns` (`03-rtk-core-tests-windows.log`). x64 CLI confirmed run exit 0 (`12-dns-run-recapture.log`). i686 PE `i386 (0x14c)` digest differs (`sha256:10759319…`) proving the Sysnative identity is bound; confirmed run exit 0 (`14-wow64-x86-sysnative.log`). i686 identity unit test 1 passed (`15-i686-identity-test.log`). |
| R4 guidance never dispatches; rejected categories fail closed | PASS | Catalogue + CLI rejections exit 6, no output files (`13-rejections-recapture.log`). Guidance has no `ResolvedMaintenanceAction` variant. |
| R5 cancel before = canceled; after = observed/unknown; durable redacted audit | PASS (unit) / native cancel+timeout blocked | Execution tests cover cancel-before, cancel-after, timeout→unknown. Native journals: DNS `validated → dispatch_started → adapter_succeeded → succeeded`; Settings `… → launched`. No program/argv/URI in JSONL. Frozen CLI has no cancel flag. |
| R6 three Settings URIs, RtlGetVersion floors, launched-only | PASS | Catalogue snapshot of floors 22000/22000/22624. Native allowlisted launch: `h_inst_app=42 last_error=0` plus new `SystemSettings` PID 67816 HWND 39586538 (`10-settings-probe.log`). CLI `1 launched` exit 0 (`11-settings-cli-run.log`). Launch-failure control: nonexistent drive `h_inst_app=2` (`SE_ERR_FNF`) `last_error=2` (`ERROR_FILE_NOT_FOUND`). Historical ABI failure remains in `05-settings-launch-native.log` (superseded). |
| AC4 Settings adapter + audit + focused tests + `just ci` | PASS | CLI optimize 9 passed (`05c-rtk-cli-optimize.log`). `git diff --check` exit 0 (`06c-rtk-git-diff-check.log`). `cargo fmt --check` exit 0 (`06d-rtk-cargo-fmt-check.log`). `rtk just ci` exit 0 (`07-just-ci-turn.log`). |

## Focused gates (exact commands)

Verbatim implement.md filters match **zero** tests because design.md uses a single `tests.rs`. Honest filters include `tests::`.

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-core optimize::catalogue` | 0 (0 passed, 281 filtered) | `00e-rtk-verbatim-catalogue.log` |
| `rtk cargo test -p devsweep-core optimize::tests::catalogue` | 0 (7 passed) | `01-rtk-core-tests-catalogue.log` |
| `rtk cargo test -p devsweep-core optimize::authorizer` | 0 (0 passed) | `00f-rtk-verbatim-authorizer.log` |
| `rtk cargo test -p devsweep-core optimize::tests::authorizer` | 0 (8 passed) | `02-rtk-core-tests-authorizer.log` |
| `rtk cargo test -p devsweep-core optimize::windows` | 0 (0 passed) | `00g-rtk-verbatim-windows.log` |
| `rtk cargo test -p devsweep-core optimize::tests::windows` | 0 (9 passed, 1 ignored) | `03-rtk-core-tests-windows.log` |
| `rtk cargo test -p devsweep-core optimize::execution` | 0 (0 passed) | `00h-rtk-verbatim-execution.log` |
| `rtk cargo test -p devsweep-core optimize::tests::execution` | 0 (14 passed) | `04-rtk-core-tests-execution.log` |
| `rtk cargo test -p devsweep-core optimize::tests::audit` | 0 (6 passed) | `04c-rtk-core-tests-audit.log` |
| `rtk cargo test -p devsweep-cli optimize` | 0 (9 passed) | `05c-rtk-cli-optimize.log` |
| `rtk git diff --check` | 0 | `06c-rtk-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `06d-rtk-cargo-fmt-check.log` |
| `cargo test -p devsweep-core --lib native_settings_shell_execute_probe -- --ignored --nocapture` | 0 (1 passed) | `native/10-settings-probe.log` |
| `rtk cargo test -p devsweep-core --target i686-pc-windows-msvc optimize::tests::windows::resolved_ipconfig_identity` | 0 (1 passed) | `native/15-i686-identity-test.log` |
| `rtk just ci` | 0 | `07-just-ci-turn.log` |

`rtk cargo test` compresses libtest output and **drops** `--ignored` / `--nocapture`. The Settings probe was therefore captured with `cargo.exe` directly.

## Native x64

- Integrity: Medium (`09-host-integrity.log`). `consent.exe` none before/after. No UAC.
- Binary: `target\debug\devsweep.exe` PE `amd64 (0x8664)` sha256 `B9932076CA128C99570D2A7EA345A2889069E1775461F8CCEC438205FC60A811`.
- DNS: `C:\Windows\System32\ipconfig.exe` exists; `Sysnative` invisible from 64-bit PowerShell (expected). Confirmed run succeeded; journal redacted (`12-dns-run-recapture.log`). WMI poll did not sample the child (flushdns is faster than 10 ms poll) — argv/tree identity is the bounded runner + unit tests, not a captured live tree.
- Settings: confirmed CLI run **launched** exit 0; probe `hInstApp=42`; new `SystemSettings` PID **67816** HWND **39586538** started 2026-08-31 13:03:56. Outcome text states a launched page is not maintenance completion.
- Launch failure (adapter control, not a catalogue id): `!:\devsweep-optimize-missing-control` → `succeeded=false h_inst_app=2 last_error=2`.
- Unregistered URL scheme still returns shell success on this host (`hInstApp=42`); that is **not** used as catalogue failure evidence.
- Rejections: guidance and hostile ids exit 6, zero files written.

## Native i686-on-x64

- Binary: `target\i686-pc-windows-msvc\debug\devsweep.exe` PE `i386 (0x14c)` sha256 `0E8537AD48C832FE646388210D71E45416F49D3FE47EEC82291364CDDF295416`.
- Packed 60-byte `SHELLEXECUTEINFOW` compiled (x86 ABI assert).
- DNS digest `sha256:10759319f8fe8607c7ee800641affeccb744719ee230a04d82791257e3e0f6ba` ≠ x64 digest (Sysnative path is part of preview identity). Confirmed flushdns exit 0 (`14-wow64-x86-sysnative.log`).

## Remaining UNVERIFIED (with blockers)

| Clause | Completion-required? | Blocker |
| --- | --- | --- |
| Native CLI cancel-before / cancel-after | No — unit tests own it | Frozen grammar has no cancel flag (`optimize run` is `--plan --preview-digest --confirm` only). |
| Native DNS timeout / `unknown_after_dispatch` | No — unit tests own it | Real `ipconfig /flushdns` finishes in milliseconds inside the fixed 10 s bound. Forcing a hang would change the approved action. |
| Native unsupported-build Settings id | No — unit tests own it | Host RtlGetVersion build 26200 ≥ 22624. Cannot lower the live OS build. |
| Native live WMI process-tree sample of `ipconfig.exe` | No | Child lifetime is shorter than WMI poll. Runner still uses Job Object + separate program/argv. |
| Native launch of the other two Settings URIs (`storagerecommendations`, `energyrecommendations`) | No | Same `ShellExecuteExW` adapter and literal-URI allowlist; one successful `settings.search` handoff plus catalogue snapshot is sufficient. Opening extra pages would still not be completion. |

Do not treat this file as trellis-check.
