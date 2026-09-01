# Optimize Mode Parent Acceptance Review

Date: 2026-08-31
Reviewer role: coordination-parent acceptance reviewer (umbrella `08-29-optimize-mode`, review-only; no sub-agents spawned, no product files touched, no commits, parent not `task.py start`ed).
Review basis: `HEAD = 7f70f168f7af2d88baef7130b3a59b2a15ef0245` (archive commit of the last leaf). Archived child evidence under `.trellis/tasks/archive/2026-08/` was traced clause by clause; product state was independently grepped at HEAD.

## Reviewed leaf product commits and archived task paths

| Child | Product commit | Archive commit | Archived path |
| --- | --- | --- | --- |
| `08-29-optimize-catalog-execution` | `e36974e` (`e36974e97d75989b446daae0b67484f6048f0c20`, feat(optimize): Optimize catalogue, authorizer, audit) | `6861a65` (`6861a65fca702b30a7d37f1f1838dd6ec272cf9d`) | `.trellis/tasks/archive/2026-08/08-29-optimize-catalog-execution/` |
| `08-29-optimize-cli-tui-desktop-native` | `f88eda3` (`f88eda3c47b22fa40e7f737ce26b88801513c05e`, feat(desktop): Optimize three-surface workbench + native evidence) | `7f70f16` (`7f70f168f7af2d88baef7130b3a59b2a15ef0245`) | `.trellis/tasks/archive/2026-08/08-29-optimize-cli-tui-desktop-native/` |

Ordering verified from `git log -8 --oneline`: HEAD is `7f70f16`; each product commit is immediately followed by its planning evidence commit then its independent `chore(task): archive` commit, in the mandated child order catalogue/execution -> presentation/native (R3 ordering). Product times: `e36974e` 13:36, `f88eda3` 14:57. Both child `task.json` files carry `status: completed` and `parent: 08-29-optimize-mode`. No umbrella product-code commit exists; every product commit is child-tagged (`Agent-Task:` trailer). Archived status alone was not used — product state was verified at HEAD.

Independent HEAD greps (product, not archive markdown):

- `CleanupPlan` in `crates/devsweep-core/src/optimize/`: comment-only (`mod.rs:5`); no construction. CLI `commands/optimize.rs` has zero matches. Desktop `src-tauri/src/optimize.rs` has zero matches (`CleanupPlan` remains in scan/clean only).
- Extra catalogue ids: compile-time `ENTRIES: [MaintenanceCatalogueEntryV1; 8]` is exactly `dns.flush`, `settings.storage_recommendations`, `settings.search`, `settings.energy_recommendations`, `guidance.drive_optimize`, `guidance.system_integrity`, `guidance.filesystem_check`, `guidance.network_reset`. Desktop `OPTIMIZE_IDS` is the same ordered set. Hostile ids (`firewall.reset`, `pagefile.change`, `hibernation.disable`, `cmd.run`, `DNS.FLUSH`, URI-as-id) cannot resolve.
- URI substitution: `settings_handoff` maps only the three Settings ids to the three `&'static` literals `ms-settings:storagerecommendations` / `ms-settings:search` / `ms-settings:energyrecommendations`. `MaintenancePlanV1` is `deny_unknown_fields` (extra `uri` cannot enter). `ResolvedMaintenanceAction::SettingsHandoff.uri` is `&'static str`. Presentation IPC does not accept a URI.
- `GetVersionExW` / env version fallback: the only `GetVersionExW` hit in `crates/` is the comment that there is none. `os_build()` calls `RtlGetVersion` and returns `None` on nonzero status. `desktop/src-tauri/src/optimize.rs` has zero `RtlGetVersion` / `GetVersionExW` / `os_build` hits (presentation consumes typed capability only).
- `runas` / UAC: zero `runas` / elevation / `consent` references in optimize product code. Settings verb is literal `open` with `SEE_MASK_FLAG_NO_UI` (no `SEE_MASK_NOCLOSEPROCESS`). Native journals: `consent.exe=none`, Medium `S-1-16-8192`.
- Settings-as-completion copy: i18n, CLI renderer tests, desktop decoder (`settings_handoff` cannot decode `succeeded`), native `claims_completion=False`, and `docs/guide/optimize.md` all state launched is not maintenance completion.
- Guidance dispatch: `ResolvedMaintenanceAction` has only `DnsFlush` and `SettingsHandoff`. `plan_operation` returns `GuidanceNotExecutable`. TUI `dispatchable()` excludes guidance. Desktop footer renders `No run action` with no preview/run controls. Native CLI `optimize plan --operation guidance.drive_optimize` exits 6, `file_exists=False`.

## Per-requirement verdicts

| Clause | Verdict | Evidence |
| --- | --- | --- |
| R1 (separate non-elevated Optimize domain: closed versioned catalogue, plan/preview digest, authorizer, fixed execution/Settings handoff, durable audit, CLI/TUI/Desktop, native no-UAC evidence) | PASS (with sanctioned UNVERIFIED: live `ipconfig` process sample; native unsupported-build host; native CLI cancel/timeout) | Catalogue/execution: `crates/devsweep-core/src/optimize/` (catalogue/plan/windows/execution/audit) in `e36974e`; child-1 independent check PASS. Native DNS digest `sha256:3deb8ab1399acf3f832ac4886c2e1b646ef3d7a9f6c2035283c4a8961d926b1d`, confirmed run 1 succeeded, journal `validated → dispatch_started → adapter_succeeded → succeeded`, `REDACTED_OK`. Settings `settings.search` launched PID 48704 in child 1; child 2 recaptured all three URIs with unique PIDs 43148 / 27748 / 52684. Presentation: `f88eda3` adds CLI renderer extensions, `tui/modes/optimize/`, `desktop/src/modes/optimize/`, `desktop/src-tauri/src/optimize.rs`; native WebView EN/ZH 390–1440, keyboard, AX tree, reduced motion, forced-colors, device-scale 100–200%. No UAC: Medium integrity, `consent.exe` none on DNS and Settings. |
| R2 (executable scope is only fixed DNS cache refresh; Storage/Search/Energy are allowlisted Settings handoffs; drive optimize / integrity / filesystem check / network reset are guidance only; every other tweak/command absent or rejected) | PASS | Closed eight-id registry and three action classes. DNS identity is native `ipconfig.exe` + argv `[/flushdns]`, 10 s, no shell/PATH/`SysWOW64`. Settings URIs and floors (22000 / 22000 / 22624) are catalogue literals. Guidance has no executor variant. Exhaustive refusal matrix in `optimize/tests.rs` (`security.uac`, `firewall.reset`, `windows.update.reset`, `registry.tweak`, `service.restart`, `pagefile.change`, `power.plan`, `hibernation.disable`, `storage_sense.configure`, `script.run`, `cmd.run`, hostile `dns.flush` spellings). Native hostile/URI-as-id/guidance plan/preview/run all exit 6 with zero plan files. |
| R3 (catalogue/execution before presentation; umbrella owns contract parity, recursive evidence, and rollback only) | PASS | Commit/archive ordering above. `git log e36974e^..HEAD -- crates/devsweep-cli/src/application/cli.rs` is empty (frozen parser). `git diff --name-only e36974e f88eda3 -- crates/devsweep-core` is empty (presentation did not query OS build, add ids, change URIs, or patch path identity). Umbrella owns this acceptance review and the contract handoffs; no umbrella-authored product change. |

## Per-AC verdicts

| Clause | Verdict | Evidence |
| --- | --- | --- |
| AC1 (R1, R2, R3): recursive children prove catalogue closure, preview/execute identity, stale/digest/refusal, one operation, audit lock, cancellation, timeout/unknown, unsupported OS, fixed System32 process, and no UAC | PASS (with sanctioned UNVERIFIED: live `ipconfig` sample; native unsupported-build host; native CLI cancel/timeout) | Catalogue closure: child-1 AC1, `ENTRIES: [_; 8]`, catalogue 7 passed. Preview/execute identity: dry-run and execute share resolved action; native DNS digest `3deb8ab1…` and Settings search digest `7307cb96…` are stable across children. Stale/digest/refusal: no-confirm exit 2, wrong digest exit 3 (`invalid_optimize_authority`), hostile ids exit 6. One operation + audit lock: `MaintenancePlanV1` one `operation_id`, `EXECUTION_PERMIT` + `SidecarLock`. Cancellation / timeout / unknown: execution 14 passed (cancel-before = `CanceledBeforeStart`; after dispatch = observed or `UnknownAfterDispatch`); closed outcome enum `{ CanceledBeforeStart, Succeeded, Launched, Failed, UnknownAfterDispatch }`. Unsupported OS: `OsBuildUnavailable` / `BuildUnsupported` in core; fixtures `refusal-os-build-unavailable.json` / `refusal-build-unsupported.json`; presentation decoder `optimize_unavailable`. Fixed System32 process: `IsWow64Process2` → `GetSystemDirectoryW`+`ipconfig.exe` (or `GetWindowsDirectoryW`+`Sysnative\ipconfig.exe`); native x64 resolved `C:\Windows\System32\ipconfig.exe` exists; confirmed flushdns exit 0; i686 identity test 1 passed (System32 or Sysnative, never SysWOW64). No UAC: greps + Medium + `consent.exe` none. |
| AC2 (R1, R3): CLI/TUI/Desktop present identical operation ids, action classes, capability/refusal reasons, outcomes, and locale-invariant schemas | PASS | Child-2 independent check: EN/ZH `optimize list` shows the same eight ids and three badges (`Runs here` / `Opens Windows Settings` / `Guidance only`). TUI `catalogue_matches_closed_set` zips live `catalogue_entries()`. Desktop `decodeDesktopOptimizeListResult` requires `entries.length === 8` and exact `OPTIMIZE_IDS` order; `exact()` on plan/preview/outcome envelopes; Settings cannot decode `succeeded`; DNS cannot decode `launched`; guidance cannot be planned or previewed. Fixtures: 13 files under `desktop/src/api/fixtures/optimize/` covering catalogue, four previews, DNS succeeded, Settings launched, five-terminal table, audit, and four refusals. `just desktop-web-check` generated `types.gen.ts` from 28 named fixture files. Machine documents omit program/argv/URI (digest-bound). Bilingual i18n + TestBackend + native captures. |
| AC3 (R2): no path changes security/UAC/firewall/update/service/registry/power/pagefile/hibernation or accepts user program/argv/script input | PASS | Independent greps at HEAD: no `runas` in optimize product; Settings verb `open`; no `GetVersionExW`; no env-text version fallback; `dns_flush_request` program is the resolved absolute path and argv is exactly `[/flushdns]`; plan JSON is three fields only (`version`, `catalogue_version`, `operation_id`); extra `program`/`argv`/`uri` fields fail closed; CLI `optimize plan` of `cmd.exe /c calc`, `ipconfig`, and unregistered schemes exits 6 with zero files. Audit records omit program/argv/URI/path/output (`REDACTED_OK` on native journals). Permanent delete remains disabled; scanner/model layers create cleanup plans only — Optimize never constructs `CleanupPlan`. |

## Cross-child integration checks

- Frozen CLI grammar untouched: `git log e36974e^..HEAD -- crates/devsweep-cli/src/application/cli.rs` is empty. Child-1 fills `application/commands/optimize.rs`; child-2 fills presentation/TUI/desktop. Child-1 wiring of `commands/mod.rs` / `presentation/mod.rs` matches the Software-mode pattern and was adjudicated by the child-1 checker as required reachability, not grammar expansion.
- Catalogue DTO stability: inventory of eight ids, three action classes, three URI literals, and two build floors is unchanged across children. Presentation added no core files. Desktop floors are pinned (`22000` / `22624`) in `contract.ts`.
- Cleanup authority: the only destructive/process path is `dns.flush` through `RealMaintenanceDispatch` → bounded runner with separate program/argv. Settings is `ShellExecuteExW` launched-only. Guidance cannot enter `ResolvedMaintenanceAction`. No Optimize-to-`CleanupPlan` conversion. Cargo.toml adds exactly the design feature set (`Wdk_System_SystemServices`, `Win32_System_SystemInformation`, `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`); no `Win32_System_Registry`. `ShellExecuteExW` is hand-declared to keep that feature bound.
- Settings-as-completion never leaks: classify maps Settings adapter success to `Launched` (not `Succeeded`); CLI summary counts launched separately and always appends the non-completion sentence; sticky TUI/Desktop copy uses `optimize.v1.summary.launched` / `optimize.v1.summary.guidance`; native `claims_completion=False` on all three Settings recaptures.
- Guidance never dispatched end to end: type-level two-variant enum → `plan_operation` fail-closed → CLI exit 6 → TUI `dispatchable` false + `No run action` → desktop no preview/run buttons → native `file_exists=False`.
- Registration: Optimize mode registered in the desktop shell (`App.tsx` adds the `optimize` route) only after native standard-user evidence was captured, per design. Rollback remains “unregister presentation, keep versioned audits readable.”
- Child-1 UNVERIFIED “native launch of the other two Settings URIs” is closed at umbrella level by child-2 independent recapture (unique PIDs, launched ≠ completion).

## Commands, exit codes, and log paths (independent, per archived evidence)

Logs are under the archived child paths above. `just ci` was not re-run; every clause traces to an archived independent log.

Child 1 (`08-29-optimize-catalog-execution/evidence/`):

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-core optimize::tests::catalogue` | 0 (7 passed) | `independent-catalogue.log` |
| `rtk cargo test -p devsweep-core optimize::tests::authorizer` | 0 (8 passed) | `independent-authorizer.log` |
| `rtk cargo test -p devsweep-core optimize::tests::windows` | 0 (9 passed, 1 ignored) | `independent-windows.log` |
| `rtk cargo test -p devsweep-core optimize::tests::execution` | 0 (14 passed) | `independent-execution.log` |
| `rtk cargo test -p devsweep-core optimize::tests::audit` | 0 (6 passed) | `independent-audit.log` |
| `rtk cargo test -p devsweep-cli optimize` | 0 (9 passed) | `independent-cli-optimize.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `independent-cargo-fmt-check.log` |
| `rtk just ci` | 0 (`ci complete`; 279 core + 149 CLI + 12 contract + 30 desktop lib among suites) | `independent-just-ci.log` |
| `rtk cargo test -p devsweep-core --target i686-pc-windows-msvc optimize::tests::windows::resolved_ipconfig_identity` | 0 (1 passed) | `independent-i686-identity.log` |
| Native host integrity (Medium, consent none) | 0 | `independent-host-integrity.log` |
| Native `dns.flush` plan/preview/no-confirm/stale/confirmed | 0 (no-confirm 2, stale 3, confirmed 0) | `independent-dns-native.log` |
| Native `settings.search` launched pair | 0 (PID 48704) | `independent-settings-native.log` |
| Hostile/unregistered URI plan/preview/run | product exits 6; harness tail 1 was an overly strict “journal must not exist” check, not a product leak | `independent-rejections.log`, `independent-rejections-journal-inspect.log` |
| Missing-drive `ShellExecuteExW` failure control | 0 (`hInstApp=2`) | `independent-settings-failure-control.log` |
| Ignored unregistered-scheme probe hang (not catalogue path) | -1 (killed; not used as proof) | `independent-settings-probe-failure.log` |

Child 2 (`08-29-optimize-cli-tui-desktop-native/evidence/`):

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-cli optimize` | 0 (16 passed) | `independent-cli-optimize.log` |
| `rtk cargo test -p devsweep-desktop optimize` | 0 (3 passed) | `independent-desktop-optimize.log` |
| `rtk cargo test -p devsweep-cli tui::modes::optimize` | 0 (6 passed) | `independent-tui-optimize.log` |
| `rtk just desktop-web-check` | 0 (146 tests; `types:generate` 28 named fixtures) | `independent-desktop-web-check.log` then `independent-postfix-desktop-web-check.log` |
| focused Vitest after CSS self-fix | 0 (67 passed) | `independent-postfix-optimize-web.log` |
| `rtk just desktop-test` | 0 (33 passed, 2 ignored) | `independent-desktop-test.log` |
| `rtk just desktop-build` | 0 | `independent-desktop-build.log` then `independent-postfix-desktop-build.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 after `rtk cargo fmt --all` | `independent-cargo-fmt.log` |
| `rtk just ci` (round 3 after header/clippy self-fixes) | 0 | `independent-just-ci.log` |
| Native EN/ZH `optimize list` | 0 (eight ids, three badges) | `independent-native-list-en.log`, `independent-native-list-zh.log` |
| Native guidance plan | 6, `file_exists=False` | `independent-native-guidance-refusal.log` |
| Native DNS flush + integrity | 0 (`run_exit=0`, Medium, consent none, `REDACTED_OK`; live process samples 0) | `independent-native-dns-process.log`, `independent-native-dns-process-round2.log` |
| Native three Settings launches | 0 (PIDs 43148 / 27748 / 52684, `claims_completion=False`) | `independent-native-settings-identity.log` |
| Native WebView catalogue/keyboard/AX/reduced-motion/HC/audit | 0 | `independent-native-desktop-webview.log`, `independent-native-desktop-webview-postfix.log`, `independent-native-desktop-hc-audit.log` |

Child-2 `just ci` rounds 1–2 failed in-scope (`Scan partial` clipped at 120 cols after Optimize nav; `clippy::if_same_then_else` in TUI cancel restore) and were repaired in presentation-owned files before archive. Round 3 is the gate used here.

## Manual/native UNVERIFIED ledger (all sanctioned, none completion-required here)

1. Native CLI cancel-before / cancel-after — frozen grammar has no cancel flag. Unit tests and TUI/Desktop reducers own the state machine. **Completion-required: no.**
2. Native DNS timeout / `unknown_after_dispatch` — real `ipconfig /flushdns` finishes inside 10 s; forcing a hang would change the approved action. Covered by execution tests and `execution-five-terminal.json`. **Completion-required: no.**
3. Native unsupported-build Settings host — host RtlGetVersion build 26200 ≥ 22624. This umbrella forbids faking `RtlGetVersion` or adding a fallback helper. Fixtures + `OsBuildUnavailable` / `BuildUnsupported` + desktop `optimize_unavailable` cover the surfaces. **Completion-required: no.**
4. Native live WMI/CIM/`GetProcessesByName` sample of `ipconfig.exe` — child lifetime shorter than user-mode poll (0 samples on two methods). Core still uses Job Object + separate program/argv; x64 native path `C:\Windows\System32\ipconfig.exe` exists and confirmed run succeeded. **Completion-required: no.**
5. Native i686 CLI confirmed `dns.flush` in this parent review — child-1 i686 identity unit test passed (1); full i686 CLI flushdns was implementer-owned. Algorithm is compile-time. **Completion-required: no.**
6. Ignored unregistered-scheme `ShellExecuteExW` completing — host can succeed or hang; not a catalogue path. Failure control recaptured via missing-drive (`hInstApp=2`). **Completion-required: no.**
7. OS-global display-scale change — forbidden to change user-global scale. Device-scale WebView2 emulation (100/125/150/200) used instead. **Completion-required: no.**
8. Live Narrator voice — AX tree (292 nodes) is the spec-protocol substitute. **Completion-required: no.**

Child-2 recorded items 3, 4, 1/2, and 7 as **BLOCKED** rather than as pass claims. They are listed here as UNVERIFIED with the same boundary language. Nothing unsanctioned was silently dropped. Per `implement.md`, overall PASS requires no completion-required UNVERIFIED: none of the above is completion-required for this umbrella.

## Overall verdict

**PASS**

No defect requires returning work to a child. The sanctioned UNVERIFIED / BLOCKED boundary is consistently documented across both children and is enforced by the frozen grammar, the no-fallback-version-helper rule, and the no-fixture-hang rule in `implement.md`; per the umbrella's own gate this does not block PASS. This umbrella stays active after PASS; archive only after `08-29-five-mode-native-integration` passes.

Reviewer note (context, not a defect): the working tree contains post-archive local dirt from other in-progress work (`README.md`, `justfile`). This umbrella directory is itself still untracked planning state. None of these were touched by this review.
