# Build Software CLI, TUI, desktop, and native evidence

## Goal

Own Software presentation, selection/detail hierarchy, bilingual output, IPC wiring, machine-scope refusals, and no-UAC native acceptance.

## Requirements

- R1: Implement Software CLI/TUI/Desktop from frozen inventory, preview, executor,
  and audit DTOs. Present grouped apps, exact source/scope, the upstream typed
  size and last-used evidence states, eligibility/refusal, selection
  count/impact, detail disclosure, and stable bottom preview/uninstall actions.
- R2: Every MSI context, registry-only, framework/resource/dependency/stub,
  update/system/hidden/NoRemove/unhealthy/conflicting/incomplete/protected entry
  is visible but unselectable with the frozen reason. Only eligible current-user
  MSIX can enter a plan. Never render guessed related files or claim complete
  removal/free space. User must review preview and second confirmation.
- R3: Bilingual search/sort/filter/expand/select, keyboard, focus, cancel,
  partial-source, stale, reboot, failure, still-present, removed, and
  unknown-after-dispatch states must preserve exact identities and discard stale
  IPC events. There is no Software execution `partial` view/fixture.
- R4: Native standard-user validation proves no UAC and truthful post-state.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): CLI/TUI/Desktop fixture parity covers mixed sources, duplicate display
      names, every ordered manual/refusal state, partial inventory, size basis/
      lower-bound/unknown, truthful last-used unknown, stale preview, and the
      five execution terminals without leaking registry command fields or
      installed paths.
- [ ] AC2 (R1, R2, R3): Selection and confirmation tests prove only exact eligible identities
      enter plans, changes invalidate preview, one uninstall runs, and navigation
      cancellation/join does not misreport outcome.
- [ ] AC3 (R1, R3): English/Chinese dense rows, expanded detail, sticky summary, long app
      names, keyboard/screen-reader, target widths, and native scaling pass.
- [ ] AC4 (R4): Disposable current-user MSIX native scenarios plus all-MSI/manual fixtures, process tree/no-UAC,
      frontend/TUI/CLI gates, desktop build, and `just ci` pass; unsupported host
      capabilities stay explicit.

## Out of Scope

- Direct registry command execution, fuzzy app grouping, admin prompt, leftover
  cleanup, update/startup tabs, fake activity/size data, or silent auto-selection.
- UserAssist/event/telemetry last-used inference or treating install/update time
  as activity.
