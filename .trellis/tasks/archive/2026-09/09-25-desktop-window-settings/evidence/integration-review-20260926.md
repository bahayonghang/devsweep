# Window And Settings Integration Review: 2026-09-26

## Implementation Status

The approved custom titlebar and desktop settings implementation is present.
The continuation reviewed the saved backend/frontend handoffs and completed
the text browser matrix. The Trellis reviewer fixed one settings-store defect:
reload previously reset the committed snapshot sequence, which allowed a late
older event to replace a saved value. Reload now retains the sequence when
the bridge instance is unchanged. A regression test covers the delayed event.

The exact configuration matrix remains in the settings child's design.md.
The implementation has eight desktop preference fields plus the separate
existing language control. No safety budget became user-configurable.

## Evidence Sources

- `../../09-25-desktop-custom-window-controls/research/implementation-evidence.md`:
  window adapter, titlebar, close lifecycle, permissions, and focused tests.
- `../../09-25-desktop-custom-window-controls/research/check-evidence.md`:
  independent window review and native evidence limits.
- `../../09-25-desktop-settings-preferences/evidence/backend-report.md`:
  closed schema/store, atomic updates, compatibility, coordinator, and sampler.
- `../../09-25-desktop-settings-preferences/evidence/frontend-report.md`:
  Settings, main/HUD appearance, renderer and Status consumers, focused tests.
- `../../09-25-desktop-settings-preferences/evidence/check-report.md`:
  final independent product review, fixes, and quality-gate results.
- `text-browser-20260926.md`: current fixture DOM, computed-style, accessibility,
  navigation, font, group-reset, and HUD observations.

## Requirement And Acceptance Map

| Parent | Child coverage | Established evidence | Remaining evidence |
| --- | --- | --- | --- |
| AC1 / R1 | W-AC1, W-AC2, W-AC3 | Custom control set, native adapter mapping, authoritative state, event/disposal tests, both-locale names | Actual Windows control actions and native input matrix |
| AC2 / R2 | S-AC1 | Settings deep link and brand menu, both-locale keyboard return to Status, component navigation/drain tests | None for code/fixture scope |
| AC3 / R3-R4 | S-AC2, S-AC3, W-AC5 | All 9 main routes at three themes/four widths in both locales, all font/scale presets, shared appearance tests, default HUD DOM | Real OS theme transition and open/reopened native HUD appearance |
| AC4 / R5 | S-AC4, S-AC5, S-AC6 | Actual Planet scheduler tests, snapshot/live argument tests, Status save/drain order, Rust HUD 2/5/10-second cadence tests | Native tray-driven show/hide behavior remains separately unverified |
| AC5 / R6 | S-AC7, S-AC8, S-AC9, S-AC10 | Store default/invalid-byte/rollback/concurrency tests, exact language compatibility, ordered snapshots, reload regression, reset checks | Native process restart and cross-window notification exercise |
| AC6 / R7 | W-AC4, W-AC6, S-AC6, S-AC9, S-AC10 | Existing domain state owners retained, close/drain tests, HUD stop/join tests, permission tests, isolated group resets | Native close during active work and actual tray/HUD lifecycle |
| AC7 / R3-R4/R7 | W-AC5, S-AC1-S-AC4 | Both-locale text matrix, actual font sizes, accessible names, keyboard return, reduced-motion control state, palette contrast tests | Native scale, physical input, and live forced-colors rendering |
| AC8 / R1-R7 | W-AC1-W-AC6, S-AC1-S-AC11 | Independent child review and frontend quality gates; final Rust results belong to check-report.md | Acceptance remains open for the native rows listed above |

## Native Rows Still Open

- Minimize/maximize/restore through the native window, external maximize state,
  physical titlebar double-click, blank-region drag, border resize, taskbar
  restore, Win+Arrow, Alt+F4, and actual Windows display scale.
- Native main/HUD appearance after a committed change, actual OS theme change,
  HUD creation/reopening, and native process restart with persisted values.
- Native active-operation close/drain and tray-driven sampler show/hide.

The user excluded image features and screenshot verification. No image proof
is requested as a follow-up. Native CUA APIs are disabled in this session.
The available browser fixture cannot establish the native rows. At the end
of the review, the task remained in progress. No native row is marked passed
from unit or fixture data.

## Quality Gate Status

Node 22 type generation, lint, typecheck, full tests, and Web build passed.
The test result was 48 Vitest files with 330 tests, plus 5 Node tests. The
initial concurrent run hit the existing 5000 ms timeout in one generated-type
test. The isolated rerun passed without changing that timeout.

`just ci` passed formatting and workspace checks, then stopped before the
core tests started because a test executable was missing (`os error 2`). A
rerun with the native Cargo executable also failed at the same boundary.
The cause is unconfirmed. The overall `just ci` result is failed. The separate
core-only attempt also could not start the unit-test executable. CLI tests
passed 203 cases, the core public-API test passed 1 case, and Tauri passed
81 tests. Two Tauri process-fixture entrypoints remained ignored by their
existing configuration. Full-workspace Clippy passed with warnings denied.
The child check report records each exact command and missing executable.

## Existing Task Boundary

No default-workload performance measurement was added. No old performance
threshold, failed result, native binary identity, or operator row was changed.
The operation-performance and native-acceptance tasks retain their scope.
No commit, archive, installer run, or user-settings mutation occurred during
the implementation review.

## User-Requested Archival

On 2026-09-26, after receiving the failed quality-gate result and the remaining
native acceptance items, the user requested commit and archive. The parent and
both child tasks are closed under that instruction. The failed gate and open
acceptance items above remain recorded without a pass claim. No additional
test run, installer run, user-settings mutation, or image feature is required
for this archival step.
