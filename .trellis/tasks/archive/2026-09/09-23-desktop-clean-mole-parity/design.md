# Design — Clean parity

## Frontend

- `modes/clean/impact.ts`: `IMPACT_RANK: Record<TargetKind, number>` and a
  pure `orderGroups(targets)` that replaces the first-seen order in
  `pages/ReviewPage.tsx:12-36`. Size key: verified bytes; lower-bound and
  unknown sort after verified, then by lower bound.
- Reducer (`modes/clean/reducer.ts`): add `skipped: ReadonlySet<string>` and
  actions `target_skipped`, `target_restored`, `target_protected`. Skipping
  uses the existing deselect path so digest invalidation stays in one place.
  `target_protected` records the id in a `protectedIds` set that the table
  treats as inspect-only until the next scan replaces the result.
- Protect flow: `ConfirmDialog` → `bridge.protectionAdd(path, true)` →
  dispatch `target_protected` on success; show `ErrorBanner` on failure. The
  call is not a heavy operation and does not use `OperationCoordinator`.
- Stage mapping: home/scanning/found use `Stage`; review, dry-run, and
  confirmation use `DetailView`; the report uses `StageResult`.

## Core (R5)

- `execution/audit.rs` `ExecutionEvidence`: add
  `#[serde(default, skip_serializing_if = "Option::is_none")] estimated_bytes: Option<u64>`.
  Set it only on `Succeeded` transitions of `MoveToTrash` actions from the
  plan target's verified or lower-bound size. Keep `deny_unknown_fields`; the
  field is additive and old lines parse with `None`.
- `history/mod.rs`: add `clean_moved_totals(paths) -> CleanMovedTotalsV1 { known_bytes: u64, unknown_records: u32, lower_bound: bool }`
  that reads the same Clean store with the existing reader and bounds.
- Tauri `support.rs`: `history_clean_totals` command → core function; add to
  `generate_handler!` and `SHIPPED_INVOKE_COMMANDS`; regenerate
  `types.gen.ts`; add decoder in `support/decode.ts` and bridge method
  `historyCleanTotals()`; fixture bridge returns a fixed value.
- CLI: `history` presentation may show the same total; not required.

## Copy

EN: "Moved to Recycle Bin", "Total moved by DevSweep", "at least".
zh-CN: "已移到回收站", "DevSweep 累计移到回收站", "至少".

## Rollback

Frontend and core parts are separable. If the audit field breaks a history
fixture, revert R5 only and keep R1–R4.
