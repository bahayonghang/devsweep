# GPUI steps 5–8 implementation evidence

Date: 2026-11-15. Scope stops before step 9. No commit, push, archive,
Computer Use, native screenshot, or real host mutation was performed.

## Delivered boundary

- `host_review.rs` owns the shared immutable review snapshot, live digest,
  explicit second confirmation, generation invalidation and per-item results.
  `view/host_review.rs` uses the existing `sheet_frame` fixed footer and tokens,
  with keyboard/a11y confirmation, cancellation and reverse-review controls.
- `Session` schedules preparation and execution on workers. A pending/running
  host job is single-flight; selection, search, refresh, scope/root changes,
  navigation and dismissal invalidate UI authority. Old deliveries cannot
  overwrite a newer selection. Cancel prevents undispatched items; a dispatched
  call remains locked until a real terminal. Closing waits for outstanding host
  work; an idle active keep-awake lease does not prevent normal app exit.
- Worker-start failure is explicit. A worker panic after possible dispatch is
  unknown, never success or rollback. No render path calls an installer or OS
  power backend.
- `Session::unbacked` has no production executor. Display-only Clean fixtures
  cannot escape into the new actions through modes, row activation or polling.
  Update-file fixtures remain read-only. Startup hive actions use only their
  explicit isolated file; no environment value installs an executor.
- Startup uses the existing strict core plan/preview/execute functions. The
  restore affordance creates a new inverse toggle plan, obtains a new live
  digest, and requires confirmation again; it is not an unaudited registry write.
- Keep awake uses the shared core typed plan/live preview/execute API and the
  same finite session throughout its lifecycle. First preparation retains the
  exact service, while restart reuses the same endpoint/token/owner. A one-hour
  request is stated before confirmation. Background queries supply cached UI
  status; acquire/clear/audit failures cannot display a fabricated active/off
  state. Session drop joins and releases through the core owner thread.
- Uninstall, process termination, leftovers and new Settings/elevation links
  remain outside this implementation. Existing refusal paths remain.

## Update authority repaired in the shared core

The original update helper bound only the spelling of `program`, had no durable
dispatch journal, and accepted a textual restart sentinel. That was insufficient
for R5/R10/R12, so the implementation also repairs the core/CLI update path.

- Default Windows resolution queries the current process SID's registered
  `Microsoft.DesktopAppInstaller_8wekyb3d8bbwe` Store-signed package and its
  `InstalledPath/winget.exe`. It does not follow the WindowsApps app execution
  alias or search PATH. Unsupported, ambiguous, inaccessible or unhealthy package
  layouts are unavailable. An explicitly selected absolute native CLI provider
  is separately bound; relative names other than the default winget selector
  are refused.
- The shared `software/file_authority.rs::BoundFile` retains no-reparse file
  and ancestor handles, physical identity and content hash. Preparation,
  preview, dispatch and post-dispatch observation verify the same binding.
- A strict stable `v1.x.y[.z]` response plus successful, bounded, untruncated
  `upgrade --help` must expose every required exact flag token. No guessed
  minimum minor version substitutes for a capability query. The actual version
  and help response hash participate in the provider fingerprint. Scripts,
  preview versions, unknown major versions, missing flags and truncated output
  are unavailable.
- `source export --name winget --disable-interactivity` must return one strict
  JSON object for the official Community source: `Arg` is the documented CDN
  URL, `Type` is `Microsoft.PreIndexed.Package`, `Data` and `Identifier` are
  `Microsoft.Winget.Source_8wekyb3d8bbwe`, and trust levels are Trusted plus
  StoreOrigin. The related identity fields, including Explicit, are hashed.
  Same-name URL/type/identifier replacement is refused. No URL, source secrets,
  environment dump, program or argv is written to the journal.
- All row observations use only the no-target read request
  `upgrade --disable-interactivity --scope user --source winget`. Selection is
  exact equality over the parsed ID/source/version, never a targeted upgrade
  disguised as a preview.
- Only after digest/confirmation validation and a durable `Update` started
  record does dispatch use `upgrade --id ID --version VERSION --source winget
  --disable-interactivity --exact --silent --scope user`. There is no agreement,
  force, reboot, override, shell or elevation fallback. The core rechecks live
  binding/row after the started record and before dispatch.
- The support journal holds the existing cross-process Software lock and syncs
  started/terminal lifecycle records. A started write failure means no dispatch;
  terminal persistence failure returns unknown with `updated=false`. New report
  operation/error evidence is optional under V1. Old plans remain readable but
  their old path-spelling fingerprint cannot authorize the new path.
- Update dispatch uses the existing five-minute executor budget; read-only
  probes retain the provider budget. A timeout remains unknown, with no rollback
  or automatic retry claim. Post-dispatch source/tool drift also prevents an
  otherwise successful process result from becoming a verified update.
- Numeric WinGet HRESULT `0x8A150109` is restart required to finish. Ordinary
  failure, timeout, canceled/lost monitoring and unknown status do not become
  success from a later absent row. No textual `restart_required` line grants a
  success result. Cancellation is supported before dispatch only and makes no
  rollback promise afterward. Recovery does not redispatch.

Official references checked during implementation:

- [Upgrade command](https://learn.microsoft.com/en-us/windows/package-manager/winget/upgrade)
  documents exact ID matching, version/source/scope, silent and interactivity
  options. The scope is deliberately current-user only.
- [Source export](https://learn.microsoft.com/en-us/windows/package-manager/winget/source)
  documents the source JSON fields and default Community-source identity.
- [WinGet 1.8.1911 UpgradeCommand.cpp](https://github.com/microsoft/winget-cli/blob/v1.8.1911/src/AppInstallerCLICore/Commands/UpgradeCommand.cpp)
  and [Argument.cpp](https://github.com/microsoft/winget-cli/blob/v1.8.1911/src/AppInstallerCLICore/Argument.cpp)
  distinguish the safe no-target listing path from package-query/installer
  behavior. Scope is installer selection; exact/id/version are package queries.
- [Official HRESULT definitions](https://github.com/microsoft/winget-cli/blob/v1.8.1911/src/AppInstallerSharedLib/Public/AppInstallerErrors.h)
  distinguish restart required to finish, restart required before install,
  restart initiated and ordinary failures.

## Verification

- GPUI `cargo test --locked --manifest-path desktop-gpui/Cargo.toml --no-run`:
  passed after correcting one misplaced match arm, with no errors or warnings.
- `cargo test --locked -p devsweep-core software::updates`: **14 passed**.
  Covers binary/version/help/source binding, old-plan readability without new
  authority, source replacement, no-target read argv, stale/late-row refusal,
  started audit failure with zero target requests, terminal audit uncertainty,
  every result class and post-dispatch source drift.
- `cargo test --locked -p devsweep-cli --test c05_software_maintenance`:
  **2 passed**, including the update authority/CLI/durable audit command double
  and the existing isolated startup/registry maintenance flow.
- GPUI `host_review::tests`: **6 passed**. Shared confirmation/invalidation,
  per-item failure/cancel, same-session keep-awake restart/expiry/Drop,
  acquire/clear failures, real core hive stale rejection and EN/zh projection.
- GPUI `host_action_tests`: **6 passed**. Covers the unbacked/Clean-fixture fence,
  canceled or obsolete delivery, keyboard startup and a newly previewed reverse
  toggle, refresh/root invalidation, explicit Session shutdown/join while a
  query still owns an executor Arc, and a failed clear followed by a separately
  reviewed Stop retry. An unchanged poll error cannot erase that retry preview.
- GPUI `software` regression filter: **24 passed**.
- Final static inspection additionally changed a failed/unknown Startup row to
  show unknown state instead of its pre-action enabled badge, and added
  `failed_startup_result_cannot_keep_a_stale_enabled_badge`. This seventh Session
  test was added after the focused run and is delegated to the final GPUI gate;
  it is not included in the six-passed count above.
- Owned Rust files were formatted and `git diff --check` passed. The main
  session/checker owns the final `just ci` / `just gpui` gate; this agent did not
  run or claim those gates.
- Tests use fake executors/backends/clocks, synthetic native-header files,
  temporary startup hives and an isolated `process_fixture` command double.
  CLI child processes use fixture-local LOCALAPPDATA for the durable audit.

These tests establish application authority/state transitions and bilingual text
projection/source contracts. They do not establish installed App Installer
resolution/permissions, actual third-party installer UI/UAC/restart behavior,
effective OS keep-awake behavior or native EN/zh screenshots. AC10's screenshot
portion remains unverified under the user's prohibition on Computer Use.
