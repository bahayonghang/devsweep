# Restyle remaining desktop modes

## Goal

Put Software, Optimize, Analyze, and Status on the immersive canvas with
denser first screens, then record bilingual native evidence for the whole
desktop. No new IPC and no forbidden Mole capabilities.

## Requirements

- R1: Software keeps MSIX-only execute and manual MSI. Denser rows, existing
  details disclosure, sticky summary/action bar on the dark canvas.
- R2: Optimize shows the closed eight-id catalogue on the canvas. Running
  state lists existing operations. No new ids, no UAC.
- R3: Analyze keeps list + treemap + breadcrumbs. Read-only. No trash from
  the treemap.
- R4: Status uses existing snapshot/live metrics as a denser bento. No
  health score. Unavailable stays unavailable. No GPU zero.
- R5: Record EN/zh-CN at 390/800/1024/1440, reduced motion, high contrast,
  keyboard, and native Windows chrome. User-operated 100/125/150/200%
  scaling if a native capture is run.

## Acceptance Criteria

- [x] AC1 (R1-R4): Each mode's first screen is the dark canvas with one
      primary action. Existing parity tests still pass.
- [x] AC2 (R3, R4): Analyze has no execute control. Status has no health
      score and no invented GPU zero.
- [ ] AC3 (R5): CSS and tests lock 390/800/1024/1440, reduced motion, and
      high contrast. Native Windows screenshot capture at 100/125/150/200%
      is still outstanding. Desktop lint/typecheck/test/build pass.

## Out of Scope

- Tray HUD, updates, startup items, leftover-file matching, fan control.
- Clean first-screen rewrite (other child).

## Dependency

Needs spec-shell tokens. May overlap Clean child.
