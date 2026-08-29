# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Developers reviewing disk space consumed by project build artifacts and global
development caches on their own machines. They need to understand what was found,
why it is considered cleanable, and what would happen before authorizing cleanup.

## Product Purpose

DevSweep is a safety-first developer cleanup planner and executor. It scans
supported project artifacts and global caches, produces an auditable cleanup
plan, and keeps dry-run, selection, confirmation, and execution explicit.

## Positioning

DevSweep uses one shared core contract across CLI, TUI, and desktop surfaces:
scanning is observational, plans carry declarative intent rather than executable
command strings, and trusted cleanup actions are reconstructed and revalidated
only at the execution boundary.

## Operating Context

- The Windows-first Tauri desktop app is an Operate surface for repeated scan,
  review, dry-run, confirmation, execution, and reporting workflows.
- CLI JSON reports support scriptable review and saved-plan workflows; the TUI
  provides an interactive view over the same scan and execution semantics.
- Active scans may be long-running and cancelable. Users need truthful phase
  feedback and useful discovered-target evidence before completion.

## Capabilities and Constraints

- Cleanup is dry-run first and requires explicit target selection plus a current
  confirmation digest before execution.
- Active-scan target previews are cumulative and read-only. They are not completed
  scan reports, do not carry cleanup intent or default selection, and cannot become
  dry-run or execution authority. The desktop and TUI keep staged results
  non-interactive until a completed report is promoted.
- Scan progress is phase-aware and indeterminate unless the backend owns a real,
  bounded total; the product does not invent percentages from elapsed time.
- Project cleanup is trash-backed by default. Supported global providers use
  registry-owned command templates with program and arguments kept separate.
- Permanent deletion and Docker cleanup are outside the current product. Cargo
  home remains inspect-only.
- Capacity remains classified as verified, partial lower bound, or unknown. The
  product describes estimated recoverable capacity and does not claim that space
  has been freed merely because an item moved to trash.

## Brand Commitments

The product name is `devsweep`. Copy is direct, calm, and operational: it states
current evidence and recovery actions without promotional claims or false
certainty.

## Evidence on Hand

- `README.md` documents the safety model, CLI/TUI/desktop workflow, and validation
  boundary.
- `DESIGN.md` records the original product and architecture direction; current
  code, tests, and Trellis specs override stale details.
- The desktop React/Tauri implementation and controlled fixtures cover scan,
  cancel, review, dry-run, confirmation, execution, and reporting states.
- No field evidence establishes that moving an item to trash immediately frees
  capacity; future UI must not fabricate such a claim.

## Product Principles

1. Show evidence before asking for authority.
2. Keep observation, preview, validation, and execution as distinct states.
3. Prefer truthful incomplete information over precise-looking guesses.
4. Reuse one domain contract across CLI, TUI, and desktop surfaces.
5. Preserve recovery and auditability over cleanup speed.

## Accessibility & Inclusion

Desktop workflows must remain keyboard operable, expose state changes to assistive
technology, retain visible focus, avoid color-only meaning, and preserve useful
scan status when reduced motion is requested.
