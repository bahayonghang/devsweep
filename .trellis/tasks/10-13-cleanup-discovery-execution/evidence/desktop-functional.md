# Desktop functional checkpoint — 2026-10-13

Implementation is ready for integrated reference B. This checkpoint does not
close packaged native, cross-volume scan, full repository or runtime gates.

## Implemented contracts

- Explicit project roots start empty; global-only is valid. The GUI never uses
  CWD implicitly. Core validates absolute existing directory selections,
  normalizes/deduplicates exact roots and retains nested selections. Stored
  now-missing roots stay visible; Clear folder selections permits recovery from
  multiple missing roots. Suggestions use metadata checks, never enumeration.
- `%LOCALAPPDATA%/DevSweep/settings/clean-scan-roots-v1.json` is closed
  `{schema_version:1,roots:[...]}`. Reads do not write. Corrupt/future bytes are
  retained; separately validated roots remain editable for the current session.
  Existing appearance-store lock, flush and atomic replace helpers are reused.
  GUI edits serialize validation/save, invalidate previews immediately, and
  fence delayed startup/old-bridge responses.
- Tauri commands: `clean_scan_preferences_get()`,
  `clean_scan_preferences_set(roots)`, `clean_scan_roots_validate(roots)` return
  `{version:1,roots,suggestions}`. `scan_rescan_target(scan_id,options,target_id,
  on_progress)` uses core's reviewed target-ID service and the normal scan
  coordinator. No new action/program/path authority is accepted. HUD remains
  denied these commands.
- Closed decoder/generated fixtures preserve optional legacy coverage and all
  13 coverage statuses; duplicate units and inconsistent complete health fail.
  Empty/partial reports keep diagnostics reachable. Command results remain
  separate from successful Recycle Bin occupancy estimates, including all-skipped
  and command-only outcomes. Inspect-only paths can open existing Analyze input.
- Reviewed rescan invalidates the digest before awaiting other-mode join;
  cancel during that wait prevents starting a native scan. Existing whole-plan
  validation, second confirmation and reducer correlation remain intact.

## Verification

Node 22.23.2 / npm 10.9.8-compatible entry points:

| Check | Observed result |
| --- | --- |
| Full Vitest, `run --maxWorkers=2 --reporter=dot` | 56 files, 422 tests pass, 71.40 s |
| Node dev-server/argument tests | 5 pass |
| Full ESLint | Pass |
| TypeScript `--noEmit` | Pass; independently repeated by Earth checker |
| Production Vite build | Pass; existing >500 kB chunk warning retained |
| Tauri library tests at desktop implementation checkpoint | 86 pass, 2 ignored fixture entry points |
| New core scan preference tests | 5 pass on Windows |
| Generated samples | Determinism test and Rust scope/coverage/legacy wire parity pass |

Independent review follow-up: root fieldset now uses the canonical card radius;
the all-skipped fixture uses attempted=selected=outcomes.length and both mutated
outcome fixtures pass through the production decoder. Focused outcome tests
remain 4/4 passing. Earth review subsequently added its GL initialization error
regression (27 stage tests pass); final integration build includes that later fix.

Focused tests cover late startup responses, serialized overlapping edits,
corrupt-store session recovery, invalid paths, stale selection/digest, rescan
identity/cancel-before-start, Analyze handoff, all coverage variants, partial
zero-target reports, command-only/all-skipped results and atomic store failure.

## Root-control visual evidence

Ignored raw artifacts: `dist/clean-scope-visual-20261013-keyboard-final/`.
The harness builds the real App/CleanWorkbench in production with an in-memory
fixture bridge. It does not invoke native scan, cleanup, or user settings.
Eight cases cover en/zh-CN × 1080×720 / 390×844 × empty / two selected paths
(spaces, Unicode, different drive names). All have no horizontal overflow;
Scan is visible, disabled for empty project scope and enabled for selected
roots. Keyboard Tab from the root input reaches visible Scan in all four
selected cases. Browser closes normally, exit 0, without forced teardown.

Initial 1080×720 screenshots exposed Scan below the fold (selected y=858).
The Clean-only short/wide layout now places the globe beside the scope form:
selected Scan y=549 (en), y=533 (zh-CN). Narrow layouts retain wrapping and
normal scrolling (English selected document 875 px at 844 px viewport, Scan
fully visible). Shared Stage renderer/CSS are unchanged by this fix. Screenshots
were inspected at full resolution; these are browser fixture evidence, not
packaged native or actual cross-volume filesystem evidence.

## Native test execution anomaly, not waived

The redirected shared Cargo target repeatedly produced native access violations
(`0xc0000005`); another suite produced `0xc000001d`. A crashed test EXE later
disappeared while its companion files remained. These are not assertion failures
and have no established cause. No security settings or unrelated processes were
changed. The full repository gate remains open.

The exact preserved nonredirected copy
`dist/native-test-isolation/core-preferences-20261013.exe`, SHA-256
`25a5c67b5c29011cb1671fca3c4996a86cde5300daeae368399ff7f2d851bc44`, ran directly
without recompilation: existing desktop preferences 14/14 serial, 14/14 normal
parallel, and scan preferences 5/5 normal parallel pass. This rules out an
inevitable deterministic failure in those tests, but does not identify the
external/native cause or substitute for the full gate.

## Still required

Packaged launch from unrelated CWD, actual isolated multi-volume scan, CLI/Tauri
golden normalized parity, integrated new-provider tests, immutable corrected B,
full root quality gate and operation-specific native lifecycle evidence remain
with parent/functional/runtime integration. No user cache cleanup was performed.
