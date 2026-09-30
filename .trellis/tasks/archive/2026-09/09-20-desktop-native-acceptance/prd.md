# Validate native Windows desktop acceptance

## Goal

Run the final native Windows acceptance matrix against the frozen desktop
binary and report visual, interaction, scaling, locale, resource, process, and
restart evidence without promoting simulated or fixture results.

## Dependencies and ownership

- Parent task: `09-20-desktop-mole-tauri-performance`.
- Depends on accepted workbench UX, shared typed-service, and operation
  performance children.
- Own the native harness, run matrix, evidence report, and acceptance gaps. Do
  not patch product code in this child; route a failure back to its owning
  child.

## Requirements

### R1. Frozen native run

Record commit, release artifact hash, Windows build, architecture, locale,
display scale, window dimensions, fixture hashes, and isolated `LOCALAPPDATA`
before each run. Keep user data and real cleanup targets out of the fixture
run.

### R2. Interaction and rendering matrix

Exercise Clean, Software, Optimize, Analyze, and Status in English and
Chinese, including keyboard focus, route switching, selection, review before
execute, cancel/restart, stale-operation recovery, partial/unavailable states,
and error recovery. Check widths 390, 800, 1024, and 1440 px where supported.
Scale evidence (TPR-05): the required rows are WebView device scale 100%,
125%, 150%, and 200% through the WebView2 `--force-device-scale-factor`
launch argument, plus the unchanged current actual Windows display scale
recorded from the host. Nobody changes the user's display configuration;
other actual Windows scales are non-gating `UNVERIFIED` rows. Every row
labels its evidence level as actual OS scale, WebView device scale, or CSS
viewport emulation, and labels the width as real window size or CSS viewport
emulation.

### R3. Native safety and lifecycle

Verify no unexpected UAC prompt, orphan worker, child CLI process, stale window
state, unauthorized delete, or identity leak. Confirm the process reaches the
resource/quiescence protocol after each scenario and that evidence labels
native, local, simulated, fixture, or `UNVERIFIED` precisely.
Lifecycle rows consume the performance child's operation table (TPR-06):
each cancel/restart row records request, acknowledgement, and join for the
operation under test, and desktop Status stop rows use the same-live-PID
post-stop window from that child's design (TPR-02). A CLI process exit, a
killed process, or an absent PID is not a desktop quiescence observation.

### R4. Local package and executable identity

Verify the identity of the local build only (TPR-03): the release
executable and the unsigned NSIS installer produced by `just desktop-build`.
Record both SHA-256 hashes; check that each file's version resource carries
the product name and version from `desktop/src-tauri/tauri.conf.json`;
record the Authenticode status as unsigned; and verify that the running
executable's main window title, process name, and executable path match the
local build. Do not run the installer, sign, publish, or check installed
shortcuts; those rows are non-gating `UNVERIFIED`.

### R5. 2026-09-23 capability rows

The 09-23 children added native surfaces that the matrix must also cover:
the capsule shell and procedural planet, Clean skip/protect and the
cumulative total, Software update check / startup toggle / leftover review,
Analyze reveal and Recycle Bin move, and the Status GPU/thermal probes,
process sort/pin, tray icon, and HUD window. Rows that need a human at the
Windows shell (tray menu, Task Manager startup state, Explorer selection,
Recycle Bin restore, Quit from the tray) are operator rows. The agent
records them as `UNVERIFIED` with owner `operator` until the operator
reports a result. The agent never toggles a real startup item, moves a real
user path to the Recycle Bin, or runs a winget upgrade.

## Acceptance Criteria

- [x] AC1 (R1, R2): A matrix records every mode, locale, width/scale,
      interaction path, and result with screenshots/logs or an explicit
      unavailable reason. Each scale row is one of the four WebView device
      scales or the unchanged current actual Windows scale, labelled by
      evidence level; other actual Windows scales are `UNVERIFIED` and
      non-gating.
- [x] AC2 (R3): Clean remains review-first and requires plan, live digest, and
      explicit confirmation; no native run authorizes a different path.
- [x] AC3 (R3): Cancel/restart leaves no stale completion, orphan worker, or
      unexpected child process; each row records request, acknowledgement,
      and join per the performance operation table, and desktop Status stop
      rows show 25 same-live-PID post-stop samples meeting the performance
      child's desktop quiescence predicate (CPU final-five hold, and a
      post-stop thread floor that does not grow across stops). A resource row is `pass` or `fail` when
      measured; `UNVERIFIED` is only for evidence that could not be captured,
      with reason and owner, and never replaces a measured `fail` (TPR-07).
- [x] AC4 (R4): The local release executable and NSIS installer hashes,
      version-resource product name/version, unsigned status, and the
      running window title/process/executable path are recorded and match
      the local build; no installer is executed.
- [ ] AC5 (R1–R4): Automated gates (including the read-only generated-type
      check) and the native matrix pass, with failures routed to owning
      children and linked from the parent plan.
- [ ] AC6 (R5): The 09-23 rows are recorded. Agent-measured rows carry
      screenshots or logs. Operator rows carry the operator result or
      `UNVERIFIED` with owner `operator`.

Evidence: `evidence/record.md` (2026-09-24). AC5 is open because
`idle.max_threads_le_40` measured 45 threads. AC6 is open until the operator
records OP-1 to OP-13 in `evidence/operator-checklist.md`.

## Out of scope

- Changing the user's Windows display scale, by the agent or by the operator.
- Installer execution, signing, publishing, installed-shortcut checks, or
  packaging redesign.
- New features, provider integrations, or product-code fixes owned by
  earlier children.
