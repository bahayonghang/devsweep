# Implement - Optimize CLI/TUI/Desktop and Native Evidence

Start only after later approval, shell/spec completion, and the frozen Optimize
catalogue/executor pass.

## 1. Wire exact DTOs and presentation

1. Register typed list/preview/run/audit IPC and generated decoders.
2. Add the bilingual CLI renderer without editing grammar or catalogue ids.
3. Prove all surfaces render the same eight ids, action classes, build/refusal,
   digest, and terminal state.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli optimize
rtk cargo test -p devsweep-desktop optimize
rtk just desktop-web-check
```

Rollback point: unregister presentation/IPC and retain core catalogue/audits;
do not patch core identity in this child.

## 2. Build mode-local TUI/Desktop flows

Implement checking, ready, selected, previewing, preview-ready, confirming,
running/launching, and terminal/unknown states. Guidance has no run action;
Settings buttons say open and terminal copy says launched; only DNS says running.
Reject stale events and cancel/join through the shell.

```powershell
rtk cargo test -p devsweep-cli tui::modes::optimize
rtk just desktop-web-check
rtk just desktop-build
```

Rollback point: remove the mode registration and both frontend modules together;
never leave a button that manufactures an operation id.

## 3. Full and native acceptance

```powershell
rtk git diff --check
rtk just ci
```

- Record both languages and all target widths/scales, keyboard/screen-reader,
  reduced motion, unsupported builds, policy failure, stale/cancel/timeout/
  unknown, and audit history.
- Capture exact DNS program/argv/process/integrity/no-UAC and all three literal
  Settings pages; record `launched`, never completion.
- Stop for any new id/URI/build predicate, shell/spec/parser change, admin flow,
  batch automation, or fake progress.
