# Independent contract review summary

This implementation-facing summary distills
`research/independent-contract-review.md`; the full review retains the complete
path/line evidence and external-reference inventory.

## Required boundaries

- Treat this as a cross-layer scan-contract change, not desktop-only styling.
  Project/global scanners must expose fully constructed targets before phase
  completion; `Sweeper` remains the only merge, dedupe, and ranking owner.
- Keep progress truthful: use phase/message plus an indeterminate progress element.
  Target count is evidence discovered so far, not a percentage denominator.
- Add a core-owned `ScanPreviewSnapshot` with purpose-built observation-only
  targets. Omit trusted actions, program/argv/cwd, cleanup intent,
  `selected_by_default`, and any submit-ready `UntrustedPlan` envelope.
- Carry desktop progress through a command-scoped ordered Tauri channel. Include an
  opaque scan id and monotonic sequence, reject stale/out-of-order messages, coalesce
  bursts, publish the first useful target promptly, and force phase/terminal flushes.
- Return explicit completed versus canceled terminal semantics. Do not infer cancel
  from partial scan health; a completed partial-health report remains reviewable.
- Keep active and completed desktop state separate. Cancel/failure retains the last
  read-only preview, while a failed rescan does not destroy the previous completed
  report. Only a matching completed report creates selection/dry-run authority.
- Thread cancellation into ordinary project target sizing, not only directory
  traversal; otherwise a large size walk can stall both preview and cancellation.
- Preserve plain CLI stdout/JSON compatibility. Because continuous TUI partials
  enlarge the current authorization window, staged TUI targets must be preview-only
  until `ScanFinished`; block selection, dry-run, and cleanup while scanning.

## Presentation contract

- Replace the active scan's completed-empty copy with a dedicated scanning surface.
- Group by typed scope (`Projects`, `Global caches`), show typed ecosystem counts,
  retain target kind as row detail, and preserve core-ranked deterministic order.
- Preview rows omit selection and action affordances. Show observation facts such as
  path, kind, measured capacity/confidence, risk, evidence, and warnings.
- Use native indeterminate progress semantics, a coalesced polite status node,
  `aria-busy` on the preview region, stable keyboard focus for cancellation, and no
  row-insertion animation. Verify reduced motion, 800x600, and 390x844 manually.

## Mandatory regressions

- Core: in-phase cumulative updates, dedupe/rank/final parity, safe serialization,
  deterministic coalescing, cancellation during target sizing, explicit terminal
  classification, and complete-empty versus partial-health behavior.
- CLI/TUI: one clean JSON document, latest-job rejection, snapshot replacement,
  active-preview action blocking, cancellation, and final promotion.
- Tauri/API: command/channel correlation, id/sequence ordering, bounded/latest
  delivery, stale cancel ids, no authority-bearing fields, strict decoding, and
  deterministic generated types.
- Reducer/React: preview/report isolation, prior-report fallback, stale terminal
  rejection, explicit cancel/error/partial/empty states, no preview action controls,
  grouped rows, accessible progress/status, and responsive overflow containment.

## Implementation gates

- The manifest declares `tauri = "2.11.3"` as a caret requirement; `Cargo.lock`
  resolves Tauri 2.11.5. Implementation verified the lock-resolved Rust
  `Channel<T>` command argument/`send(T)` API, the frontend `Channel` serialization,
  and a forced latest preview send before returning the terminal command result.
- Native Windows webview accessibility, narrow-window rendering, and reduced-motion
  behavior remain `UNVERIFIED` until built-app manual checks.
