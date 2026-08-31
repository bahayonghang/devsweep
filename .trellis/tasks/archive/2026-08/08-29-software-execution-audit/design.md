# Design - Software Execution and Audit

## Strategy and irreversibility

`ValidatedSoftwareAction` cannot deserialize and is created only by live
revalidation of an exact current-user identity and matching preview digest.
Every preview, confirmation, and audit marks uninstall as irreversible from
DevSweep's perspective. No reinstall, restore point, leftover cleanup, or
rollback outcome is represented.

## Exact adapter and V1 execution scope

Only an eligible current-user MSIX sends its revalidated exact
`PackageFullName` to the inventory task's dedicated MTA worker and calls
`PackageManager::RemovePackageAsync`. The worker and exact
`windows = =0.56.0` projection are reused; no second binding is introduced. All
MSI contexts and registry-only identities terminate at preview as manual with
the inventory task's stable refusal code. There is no MSI adapter, `msiexec`,
shell, PATH lookup, registry vendor text, or elevation fallback in this task.

## Durable dispatch and recovery state machine

The executor holds the exclusive audit lock and one-operation permit. After
live revalidation and a final cancellation check, it durably appends
`dispatch_started` and flushes it immediately before crossing the
`RemovePackageAsync` side-effect boundary. A pre-dispatch cancellation is
`canceled_before_start`; once `dispatch_started` is durable, neither normal code
nor restart recovery invokes the adapter again automatically.

The live run waits at most 120 seconds. Post-dispatch cancellation requests
WinRT cancellation at most once, waits at most five seconds, and cannot itself
decide the terminal. The executor re-enumerates the exact current-user package
identity at 0, 2, and 10 seconds. At process startup, before accepting another
Software action, the recovery pass acquires the journal lock, finds every
nonterminal `dispatch_started`, performs the same identity-only 0/2/10-second
requery, appends one terminal, and never redispatches.

Terminal classification uses this fixed first-match precedence:

| Priority | Durable/adaptor/requery evidence | Terminal |
| --- | --- | --- |
| 1 | exact identity is absent and documented adapter evidence requires reboot | `reboot_required` with `installed_state=absent` |
| 2 | exact identity is absent | `removed` |
| 3 | documented adapter evidence requires reboot and identity is present or unavailable | `reboot_required` with the observed installed state |
| 4 | adapter did not reach a documented completion because of timeout, post-dispatch cancel, crash, or restart gap | `unknown_after_dispatch` unless rows 1-3 apply |
| 5 | requery is denied, unavailable, conflicting, or cannot prove one identity | `unknown_after_dispatch` |
| 6 | exact identity remains present and adapter completed with a definitive failure | `failed` |
| 7 | exact identity remains present and adapter completed successfully | `still_present` |
| 8 | any unclassified conflict | `unknown_after_dispatch` |

`partial` is not an execution terminal. A present observation after an unfinished
adapter is not sufficient for `still_present`, because the OS operation may
still complete later. `DeploymentResult` and reboot evidence are retained, but
exact installed-state requery is the only proof of removal. No result is called
rolled back, restored, or safely canceled after dispatch.

## Audit contract

The append-only V1 Software journal contains operation id, tagged identity,
inventory fingerprint, preview digest, transition, timestamp, stable status/error
code, reboot evidence, installed-state observation, and requery result.
Transitions are `validated`, `dispatch_started`, optional `adapter_completed`,
`requery_observed`, and one closed terminal. The writer rejects a second terminal
or a transition after terminal. It excludes localized text, vendor
strings, argv, guessed paths, environment, and raw deployment output. History may
read it later but cannot reconstruct an executable strategy.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-core/src/software/execution/mod.rs` | permit, state machine, outcomes, requery |
| `crates/devsweep-core/src/software/execution/msix.rs` | MTA PackageManager removal adapter |
| `crates/devsweep-core/src/software/execution/audit.rs` | locked durable redacted V1 journal |
| `crates/devsweep-core/src/software/execution/tests.rs` | fake/native state-machine and audit tests |
| `crates/devsweep-cli/src/application/commands/software/execution.rs` | frozen preview/uninstall handlers |

The CLI contract task first creates `application/commands/mod.rs` and the empty
`application/commands/software/mod.rs` boundary; this task fills only
`software/execution.rs`.

This task does not own inventory identity files, parser grammar, Tauri/React/TUI
presentation, CleanupPlan, or the shared audit-history reader. Rollback disables
strategy construction/execution while keeping inventory and readable audits.
