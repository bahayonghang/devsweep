# Clean parity: impact ordering, row skip/protect, cumulative total

## Goal

Bring the Clean flow closer to the Mole desktop flow: a planet stage that
shows the found total, a review list ordered by impact with regenerable caches
first, per-row skip and protect actions, and a big-number result screen with a
recorded cumulative total across past Clean runs. Keep the existing
plan → dry-run digest → second confirmation → execution funnel unchanged.

## Dependency and boundary

- Parent: `09-20-desktop-mole-tauri-performance`.
- Starts after `09-23-desktop-mole-capsule-shell` (uses `Stage`,
  `StageResult`, `DetailView`).
- Changes `desktop/src/modes/clean/`, `desktop/src/pages/`, Clean components,
  and for R5 only: `devsweep-core` execution audit evidence, history
  aggregation, one Tauri query command, and generated wire types.

## Requirements

- R1 Stage: the Clean first screen shows the planet, a title, the scan scope
  toggles (Projects, Global caches), and the Scan action. During scanning the
  number grows with the cumulative "found so far" value and the planet keeps
  turning. After scanning the stage shows the found total with a "Review"
  action that opens the review detail view.
- R2 Impact order: review groups are ordered by a fixed impact rank:
  regenerable caches first (`package_cache`, `tool_cache`, `test_cache`),
  then `build_artifacts`, then `dependency_directory` and `virtual_env`.
  Inside one rank, groups and rows are ordered by verified bytes descending;
  lower-bound and unknown sizes sort after verified sizes and keep their
  labels.
- R3 Row skip: each selectable row has a Skip control. A skipped row leaves
  the selection, moves to a collapsed "Skipped" group at the end of the list,
  and can be restored. Skip is not persisted and does not change the plan
  content; it changes only the selected id set, which invalidates any existing
  dry-run digest through the current reducer path.
- R4 Row protect: a row with a filesystem path has a Protect control. It opens
  a confirmation dialog, then calls the existing `protectionAdd(path, true)`
  bridge call. On success the row becomes inspect-only with the protected
  label, the selection and dry-run digest are invalidated, and the stage
  suggests a rescan. Command-backed rows without a path do not show Protect.
- R5 Result and cumulative total: after execution the result stage shows the
  bytes moved to the Recycle Bin in this run (verified plus lower bound, with
  the existing partial labels), the count of succeeded/skipped/failed targets,
  and a cumulative total "moved to Recycle Bin by DevSweep" derived from the
  Clean audit history. Core adds an optional `estimated_bytes` value to
  succeeded execution evidence; history records without the value count as
  unknown and the cumulative line says "at least" when any unknown exists.
  Copy never says the space is freed, because a Recycle Bin move does not
  release disk space until the bin is emptied.
- R6 Safety: inspect-only targets stay unselectable; skip and protect never
  add a target to the selection; execution still requires the live digest and
  second confirmation.

## Acceptance Criteria

- [ ] AC1: Stage states (home, scanning, found, empty, error, canceled) render
      in both locales at 390/800/1024/1440 CSS px.
- [ ] AC2: A reducer/selector test proves the impact order and size order for
      mixed verified, lower-bound, and unknown targets.
- [ ] AC3: Tests prove Skip removes the id from the selection, invalidates the
      dry-run digest, and Restore re-adds it; Skip is absent on inspect-only rows.
- [ ] AC4: Tests prove Protect requires confirmation, calls `protectionAdd`
      with the row path and `confirm=true`, marks the row inspect-only, and
      invalidates selection and digest; Protect is absent for path-less rows.
- [ ] AC5: Core tests prove new evidence serializes `estimated_bytes`, old
      records without it still parse, and the history aggregate returns
      `{ known_bytes, unknown_records }`; a Tauri command returns it through a
      closed decoder with a fixture.
- [ ] AC6: No production or test string says freed/released/释放 for Clean
      results.
- [ ] AC7: `just ci` and desktop lint/typecheck/test/build pass;
      `npm run types:generate -- --check` is clean after regeneration.

## Out of scope

- Emptying the Recycle Bin, permanent delete, or a "clean directly" mode that
  bypasses the Recycle Bin.
- New cleanup providers or new target kinds.
