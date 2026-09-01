# Rebuild Clean mode workbench and safety flow

## Goal

Adapt the Mole reference information hierarchy into an original Clean workbench while preserving scan, selection, preview digest, confirmation, cancellation, and execution safety.

## Requirements

- R1: Preserve the current cleanup safety chain: scan observation -> completed
  report -> exact selection -> saved untrusted CleanupPlan -> live validation ->
  dry-run preview digest -> second confirmation -> execution -> audit. Rescan or
  selection change invalidates preview authority.
- R2: Consume the foundation sizing child's bounded/cancelable estimator only
  after its same-host throughput gate passes. Display verified/partial/unknown
  evidence and estimated recoverable capacity; never show incomplete totals as
  exact or trash moves as already freed space.
- R3: Adapt dense grouped rows, progressive detail disclosure, search/filter,
  sort, select-all-within-safe-scope, and stable bottom selection/action summary.
  Inspect-only Cargo and protected targets are visible but unselectable with a
  reason. The running scan remains cancelable and results remain non-selectable
  until a final report is ready.
- R4: Implement equivalent CLI, TUI, and desktop Clean states using the frozen
  bilingual catalogue and command contract. Domain state remains typed and IPC
  events are rejected after cancellation or operation-id replacement.
- R5: Preserve dry-run default, permanent-delete disabled, reparse no-follow,
  command program/argv separation, no Docker, and no new deletion source.
- R6: Own the Clean V1 audit envelope/writer at the fixed
  `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl` path. The closed record-kind
  union covers Clean execution transitions and a redacted protection-mutation
  handoff; appends use an exclusive cross-process lock and fail closed. Unknown
  versions and corrupt bytes are retained; removed explicit audit paths and the
  legacy default are never discovered, imported, converted, or rewritten.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R5): Unit/integration tests cover incomplete and canceled sizing, protected
      and inspect-only targets, selection invalidation, stale plans/digests,
      reparse points, partial execution, retry boundaries, and audit outcomes.
- [ ] AC2 (R1, R4, R5): CLI/TUI/Desktop state tests prove no selection during scan, two-step
      confirmation, cancellation/join behavior, stale-event rejection, and
      bilingual truthful copy.
- [ ] AC3 (R3, R4): Dense workbench, expanded rows, action summary, keyboard selection,
      focus, long paths, empty/error/partial states pass 390/800/1024/1440 widths
      and native 100/125/150/200% scaling.
- [ ] AC4 (R2, R4, R5): The accepted sizing A/B evidence, focused frontend/TUI/backend tests,
      native dry-run and trash evidence, `git diff --check`, and `just ci` pass.
- [ ] AC5 (R1, R6): Exact-path, version, lock/flush failure, interrupted append,
      redaction, unknown-version/corrupt-byte preservation, and legacy
      non-discovery fixtures pass; History consumes the accepted V1 fixture and
      cannot recover argv, raw action paths, or executable plan data.

## Out of Scope

- Software uninstall, maintenance commands, disk treemap, status metrics, or new
  cleanup rules. Protection/rule management lives in its supporting-surface task.
