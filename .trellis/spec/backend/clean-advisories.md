# Project Capacity Advisories

## Source and authority boundary

`crates/devsweep-core/src/clean_advisories/` owns the pure
`project_capacity_advisories(&ScanReport, threshold_gib)` projection and its
separate settings store. It consumes the completed report and the crate-private
`project_footprints` ownership/overlap projection. It must not probe paths, run
commands, rescan, change selection, build cleanup actions, or schedule work.

`CleanAdvisoriesV1` is a closed observational DTO. It contains `version`,
`report_id`, `threshold_bytes`, `included_kinds`, and `projects`. The SHA-256
`report_id` identifies the serialized input report; it is never a preview digest
or cleanup authority. Changing only the threshold preserves this identity and
the original report, plan, selection and confirmation state. Saved ScanReport
V1 and CleanupPlan V2 shapes remain unchanged.

## Totals, confidence and threshold

- The inclusive default is 5 GiB = 5,368,709,120 bytes. Thresholds are positive
  whole GiB, at most 8,388,607 GiB so the byte value remains a JavaScript safe
  integer. Validate before persistence or projection with checked multiplication.
- Attribute by reported project owner roots. Count `build_artifacts`,
  `test_cache`, and `tool_cache` in `artifacts`; dependency directories and
  virtual environments have a separate `dependencies` subtotal. These are
  logical observed bytes, not promised physical recovery.
- The footprint layer deduplicates equal/nested observations. Shared,
  unresolved or mixed-category footprints are excluded from known sums and
  their IDs remain in `excluded_target_ids`; affected owners stay incomplete.
  Global targets, including unmeasured shared mbx storage, are not project bytes.
- Exact totals require successful project discovery coverage as well as complete
  owned footprints. Missing legacy coverage, skipped/failed discovery, or an
  ambiguous footprint cannot become healthy zero. A covered zero-target root
  can be exact zero only for `complete`/`empty` coverage. Ancestor lookup uses
  normalized path components, including volume roots; sibling prefixes do not
  imply containment.
- `confidence` is `exact`, `lower_bound`, or `unknown`. A partial positive sum
  is a lower bound; an incomplete zero is unknown. Addition overflow or a sum
  above the JavaScript safe boundary caps at that boundary and stays a lower
  bound. Each subtotal carries the contributing target IDs.
- `assessment` is `oversized` whenever known artifact bytes reach the threshold,
  including lower bounds; `below_threshold` requires exact bytes below it;
  otherwise it is `incomplete`. UI copy must retain the at-least/unknown meaning.

## Focused settings store

The fixed `clean-advisories-v1.json` file lives beside existing desktop settings.
Its closed document is `{schema_version: 1, threshold_bytes: <whole GiB bytes>}`.
The public preferences DTO is `{version: 1, threshold_bytes}`. Do not add fields
to scan roots V1 or appearance V2.

Missing-file reads return the default without creating files or directories.
Corrupt, future-version, unknown-field, non-whole-GiB and out-of-range documents
fail while preserving original bytes. Saves validate before I/O, acquire the
existing transaction lock, reread the stored document under lock, then use the
shared atomic replacement helper. Failed replacement preserves the prior value
and removes temporary bytes. The pure `clean_advisory_preferences` validator
allows a caller to offer session-only recovery without overwriting corrupt data.

## Desktop contract and verification

The thin Tauri module exposes `clean_advisory_preferences_get`,
`clean_advisory_preferences_set(threshold_gib)`, and
`clean_advisories(report, threshold_gib)`. Each dispatches blocking work through
`spawn_blocking`. The existing bridge uses camelCase `thresholdGib` IPC arguments;
components only call the typed DesktopBridge methods. No new command belongs to
the HUD allowlist.

Closed decoders reject unknown fields, unsafe values, incorrect included kinds,
invalid SHA identities, duplicate owners/IDs, contradictory confidence and
assessment, and threshold-response mismatches. Generate `types.gen.ts` from
fixtures; never edit it directly.

`cargo run -p devsweep-core --example clean_advisory_fixture` regenerates the
native Windows fixture corpus using public core functions. It also emits exact
scoped report/advisory replay pairs for the browser fixture. Fixture scan honors
global/project options and explicit supported roots, emits the same rows in
progress and completion, and keeps project defaults false. It rejects advisory
requests for unrecorded reports rather than inventing frontend ownership or
reusing an unrelated report hash. Historical saved fixtures remain unchanged.

Required tests cover T-1/T/T+1, accumulated small outputs, partial/unknown
assessments, dependencies, owner aliases/overlap/shared storage, zero-target
coverage and volume-root boundaries, no report mutation, integer boundaries,
missing/corrupt/future stores, failed replacement, concurrent whole-value writes,
closed decoder and native Rust wire/projection parity. Windows-only archived
path parity is separate from portable core tests and cross-platform wire
roundtrips. Parent integration owns native UI and `just ci` evidence.
