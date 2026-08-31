# Implement - Software CLI/TUI/Desktop and Native Evidence

Start only after later approval, shell/spec completion, and both Software core
children pass.

## 1. Wire typed IPC and human rendering

1. Register inventory/preview/uninstall/audit commands over frozen core DTOs.
2. Generate decoders/fixtures and add the bilingual CLI human renderer without
   editing parser/handler grammar.
3. Prove tagged identities, refusal reasons, digests, outcomes, and machine
   fixtures match across surfaces, including size basis/lower-bound/unknown,
   strict last-used unknown, all-MSI/manual refusal, and rejection of execution
   `partial`.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli software
rtk cargo test -p devsweep-desktop software
rtk just desktop-web-check
```

Rollback point: unregister Software IPC/presentation while retaining inventory
and readable audits; do not patch core defects in this child.

## 2. Build mode-local TUI/Desktop workbench

Implement loading, ready/selecting, previewing, preview-ready, confirming,
uninstalling, and terminal/unknown states; exact-id rows; manual/unselectable
entries; labelled estimated/lower-bound/unknown size; explicit last-used unknown;
sticky count/evidenced-size summary; explicit irreversibility; the closed five
terminal states; crash/restart without redispatch; stale-event rejection; and
cancel/join through the shell.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli tui::modes::software
rtk just desktop-web-check
rtk just desktop-build
```

Rollback point: remove the TUI/Desktop mode modules and registration together;
never leave a selectable UI pointing at a disabled executor.

## 3. Full and native acceptance

```powershell
rtk git diff --check
rtk just ci
```

- Run disposable current-user MSIX inventory/preview/uninstall as a standard
  user and all-MSI/manual inventory fixtures; capture process/integrity tree, no
  UAC, post-state, reboot, cancel/timeout/unknown, crash/restart audit without
  redispatch, size evidence, truthful last-used unknown, and unsupported
  capability. Consume only the exact disposable current-user package identity
  separately confirmed at the execution-task checkpoint; do not install or sign
  a fixture, and pause `UNVERIFIED` if no target is supplied.
- Record English/Chinese dense rows and confirmation at all target widths/scales,
  keyboard/screen-reader paths, long/duplicate names, and partial sources.
- Stop for any guessed leftover/free-space/activity claim, reversible wording,
  MSI/machine/other-user uninstall, parser/spec change, or domain workaround.
