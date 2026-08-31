# Independent check report — Software CLI, TUI, desktop, and native evidence

Checker: trellis-check sub-agent, 2026-08-31, branch `dev`.
Scope: independent verification of all five implemented areas in
`implement.md`; self-fix limited to in-scope presentation/evidence defects.

## Verdict: PASS per PRD AC1-AC4

All acceptance-criteria gates pass. The sanctioned UNVERIFIED set (real
uninstall/post-state/reboot/cancel-timeout/crash-restart scenarios, live
interactive TUI keyboard capture, process-tree/no-UAC probe beyond verified
medium integrity, free-space/activity claims) remains explicitly UNVERIFIED as
the task documents allow; nothing unsanctioned was silently dropped.

## Commands re-run by the checker (raw logs, real exit codes)

| Command | Exit | Log |
| --- | --- | --- |
| `cargo test -p devsweep-cli software` | 0 (13 passed) | `independent-cargo-test-cli-software.log` |
| `cargo test -p devsweep-cli tui::modes::software` | 0 (4 passed) | `independent-cargo-tui-desktop-fmt-gitdiff.log` |
| `cargo test -p devsweep-desktop software` | 0 (2 passed) | same |
| `cargo fmt --all -- --check` | 0 | same |
| `git diff --check` | 0 | same |
| `just ci` (pre-fix) | 0 | `independent-just-ci.log` |
| `npm --prefix desktop run typecheck` (pre-fix) | 0 | `independent-npm-typecheck.log` |
| `git diff --stat -- .../application/cli.rs` + `git status --porcelain` on it | 0, empty (grammar untouched) | `independent-cli-grammar-untouched.log` |
| `cargo test -p devsweep-cli --lib` (post-fix) | 0 (140 passed) | `independent-postfix-gates.log` |
| `just ci` (post-fix) | 0 | `independent-postfix-gates.log` |
| `npm --prefix desktop run typecheck` (post-fix) | 0 | `independent-postfix-gates.log` |
| `npm --prefix desktop run test` (post-fix, checker's own run) | 0 (22 files / 130 tests) | `independent-postfix-gates.log` |

Main-session logs audited, not re-run: `desktop-web-check-final.log` (types
generate, lint, typecheck, 22/22 files, 130/130 tests, vite build all recorded)
and `desktop-build-main.log` (release tauri build finished, NSIS bundle
produced). Both are internally consistent and corroborated by the checker's
own successful `npm test` run.

## Verification findings

### 1. Tauri Software IPC (`desktop/src-tauri/src/software.rs`)

- Single-flight: `SoftwareCoordinator.begin` rejects a second operation with
  `SoftwareAlreadyRunning`; `finish` clears only when operation id AND cancel
  flag Arc pointer match (stale-result rejection at the coordinator level).
- `software_cancel` only fires on the matching running id; stale cancel is a
  harmless no-op (unit-tested).
- `software_audit` performs `SoftwareExecutor::recover_startup()` (re-query
  only) plus durable record read; audit records are version/domain validated
  and never redispatch.
- Wire test proves `serde_json` rejects `"partial"` for
  `SoftwareExecutionOutcome` and that the uninstall report encoding contains
  no `partial`, `argv`, or `path`. Core enum has exactly the closed six states
  (one pre-dispatch cancellation + five post-dispatch terminals).
- Registration in `lib.rs` after the other coordinators; structured error
  mapping (`SoftwareAlreadyRunning`, `SoftwareFailed`,
  `SoftwareStaleAuthority`, `SoftwareAuditUnavailable`) added to
  `error.rs` with recovery-oriented desktop copy in `ErrorBanner.tsx`.

### 2. TUI Software mode (`crates/devsweep-cli/src/tui/modes/software/`)

- Full phase machine (Loading/Ready/Selecting/Previewing/PreviewReady/
  Confirming/Uninstalling/Terminal/Unknown/Canceling) with job-id + operation
  guards; stale completions ignored (tested).
- Selection only via `selectable()` = Selectable + EligibleCurrentUserMsix +
  CurrentUser + MSIX identity; manual rows render `[-]` and never toggle
  (tested).
- Preview completion requires selected_ids == plan.selected_ids ==
  preview.selected ids; uninstall completion requires outcome ids ==
  preview.selected ids. Any selection mutation clears plan/preview/report.
- `unknown_after_dispatch` lands in the Unknown phase; restart audit replays
  the durable terminal and never redispatches (tested).
- Runtime: all four Software effects enforce worker-registry single-flight
  ("prior work has not joined"), join on every terminal event, cancel/flag
  propagation in workers, and `Released` resets mode state on navigation
  (mirrors the existing Analyze pattern).
- Bilingual render at 100 columns via TestBackend asserted for manual refusal,
  unknown size/last-used, exact MSI identity, and irreversible copy.

### 3. Desktop SoftwareWorkbench (`desktop/src/modes/software/`)

- Same state machine over typed IPC; bridge enforces operation_id echo on all
  five commands; decoders are closed-world `exact()` with pairing invariants
  (selectable iff eligible_current_user_msix with MSIX/current-user identity,
  duplicate id rejection, strict last-used `unknown`/`no_supported_exact_source`,
  size basis/source pairing, plan==preview cross-check).
- Plan payloads contain only the frozen five fields; preview items only
  id/identity/action_class/scope/eligibility/strategy_token (no registry
  command fields, no installed paths).
- Coordinator lifecycle: typed `software` kind, cancel callback, join via
  `lease.complete()`, `mounted` guard, stale failures ignored when superseded.
- Accessible list (disabled checkboxes with aria labels for manual entries,
  `AccessibleUserData` for long data), responsive breakpoints (800px, 430px),
  `prefers-reduced-motion`, and `forced-colors` rules in `styles.css`;
  bilingual via the shared catalogue; second-confirmation dialog with exact
  identities, scope, digest, and "cannot restore or reinstall" copy
  (component-tested in both locales).

### 4. CLI human renderer / parser grammar

- `git diff` on `crates/devsweep-cli/src/application/cli.rs` is empty — frozen
  grammar untouched.
- `application/commands/software/execution.rs` changes only replace ad-hoc
  inline bilingual strings with the shared catalogue renderer
  (`presentation::software::{preview,execution}`) — presentation ownership per
  design, no handler/grammar change. The new execution summary enumerates the
  closed outcome counts instead of a lossy "other" bucket.
- Machine documents: locale-invariant bytes asserted by test; hostile
  inventory/plan fields fail closed before any adapter call; truthful
  last-used shape never relabelled.

### 5. Cross-surface parity (checker's independent probe)

Using the real native CLI inventory (`native-20260831/software-inventory-all.json`,
1,293 entries) against `desktop/src/api/fixtures/software/inventory.json`:

- Entry/identity/size/last-used key-union shapes are exactly equal between the
  hand-maintained desktop fixture and the real Rust serde output.
- All 1,293 native entries satisfy the strict TS decoder invariants
  (0 violations: eligibility pairing, selectable identity shape, closed reason
  set, strict last-used unknown, size state shapes, duplicate id rejection).
- Native output contains no `UninstallString`, `QuietUninstallString`,
  `"argv"`, or `installed_path` fields.
- Native envelope claims in `native-evidence.md` verified against the raw
  JSON: outcome `partial`, 1,293 entries, 86 selectable, 1,207 manual, 585
  unknown sizes, 1 partial size.

### 6. Authority audit

- No irreversibility wording violations: all surfaces say DevSweep cannot
  restore/reinstall and label the action irreversible ("from DevSweep's
  perspective" mirrors the frozen core model comment). Guides (en/zh) state
  size evidence is not a promise of freed space.
- No cleanup authority outside the frozen flow: the only destructive path is
  `software_uninstall` -> `SoftwareExecutor::execute` behind a confirming
  phase + dialog + core digest/confirmation checks; permanent delete remains
  disabled; scanner/model layers create plans only.
- Execution `partial` is rejected at the Rust type level, by serde test, and
  by TS union exclusion plus parity test.
- Stale events rejected at every layer (coordinator Arc/id match, TUI job-id +
  operation guards, job-registry gating, bridge operation_id echo, reducer
  id checks). Cancel/join on navigation handled by shell coordinator (desktop)
  and worker registry + `Released` (TUI).

### 7. Native evidence authenticity

- `capture-log.jsonl` (39 records): sha256 of
  `target/release/devsweep-desktop.exe` matches the current on-disk binary
  (checker recomputed); 5 app launches with PID + loopback CDP ports 9360-9364
  and 5 app_closed events; 12 screenshots present on disk matching names.
- DPI probes recorded real `devicePixelRatio` 1 / 1.25 / 1.5 / 2 with viewport
  1000x750 -> 800x600 -> 667x500 -> 500x375, consistent with WebView2 scale
  emulation (OS display setting untouched).
- Persisted-store audits: en phase store null (first profile), zh phase store
  `{"schema_version":1,"language":"zh-CN"}`.
- AX tree file genuinely contains 44,045 nodes (28.7 MB); inventory state
  probes show 1,293 rows in en, zh-CN, and all four DPI scales with the
  evidenced sticky summary.
- `integrity.txt` records the medium-integrity token (`S-1-16-8192`) via the
  explicit Windows `whoami.exe`; the doc correctly refuses to promote that into
  process-tree/no-UAC claims (probe access denied, recorded).
- Coverage = implement.md step 3 minus the sanctioned UNVERIFIED set. One
  residual gap now explicit in `native-evidence.md`: the capture set covers the
  inventory screen only; the second-confirmation dialog, reduced-motion, and
  high-contrast appearances were not captured natively (covered by component
  tests + TUI TestBackend render instead).
- Minor observation: the `keyboard_select` probe on a disabled manual row is
  consistent with the all-manual-disabled rule; the `search_narrowed` probe
  recorded 1293 rows (no demonstrated narrowing) — see `sw-04-search-en.png`
  for the visual state; recorded as an evidence-weakness note, not a defect.

### 8. `${arpDisplayName}` adjudication: faithful behavior, not a defect

The real registry values are literally `${{arpDisplayName}}` (double braces, as
stored) on 5 ARP entries, all `hidden_entry`/manual — visible but
unselectable. Under "display strings are data," DevSweep must surface the raw
registry value rather than fabricate or expand a template; fabricating a
display name would violate truthfulness, and the entries carry no cleanup
authority. The doc note's literal was imprecise (single braces) and has been
corrected to the actual stored literal.

## Fixes applied by the checker (in-scope, self-fixed)

1. Locale-store bypass in `desktop/src/modes/software/SoftwareWorkbench.tsx`:
   three hardcoded bilingual strings ("Select all eligible / 选择全部符合项",
   "Clear selection / 清除选择", "Terminal results / 终态结果", "Audit
   transitions / 审计转换") bypassed the closed locale catalogue — the only
   such strings in `desktop/src`. Added four keys
   (`software.v1.action.select_all`, `software.v1.action.clear_selection`,
   `software.v1.results.title`, `software.v1.audit.transitions`) to both
   `resources/i18n/en.json` and `resources/i18n/zh-CN.json` (en/zh parity and
   placeholder metadata preserved) and switched the workbench to
   `message(locale, ...)`. Post-fix: `cargo test -p devsweep-cli --lib` (140),
   `just ci`, desktop typecheck, and full `npm test` (22 files / 130 tests)
   all pass — see `independent-postfix-gates.log`.
2. `native-evidence.md`: corrected the `${arpDisplayName}` literal to the
   actual stored `${{arpDisplayName}}`, added the checker adjudication, and
   made the confirmation-dialog/reduced-motion/high-contrast native-capture
   boundary explicit instead of implied.

No contract, capability, or core change was made or needed. Pre-existing dirty
files (`.trellis/.gitignore`, `README.md`, `justfile`) untouched; no commits.

## Blockers

None for this task. The sanctioned UNVERIFIED destructive/field scenarios await
a user-confirmed disposable current-user MSIX target (execution checkpoint);
live interactive TUI keyboard capture and the process-tree/no-UAC probe remain
host-capability-limited and are documented as such.
