# Execution status

User authorized implementation in task order on 2026-10-14. Product changes are implemented and checked as described below. **The parent remains in progress:** mbx executable cleanup is not qualified or delivered, and some native/manual acceptance remains open. See `acceptance-matrix.md` for criterion-level limits.

| Child | Current result |
| --- | --- |
| A — scope contracts | Implemented and checked: independent exact scopes, global entry without roots, shared project opt-in ranking, preserved CLI/TUI explicit selection and saved formats. |
| B — project artifacts / mbx | Safe subset implemented: project footprint grouping, overlap/confidence handling, known shared Cargo target inspect-only refusal and mbx location observations. Workspace ownership/capacity and executable mbx cleanup remain open. |
| C — capacity warnings | Implemented and checked: inclusive default 5 GiB, whole-GiB preference, separate dependencies, lower-bound/unknown handling and atomic preference recovery. |
| D — desktop workbench | Implemented and checked: two sections and retained sessions, root/revision/preview fencing, scoped styling, warning settings, bounded pages and bilingual copy. Fixture visuals verified; native/manual limits are explicit. |
| E — animal identity | Generated original beaver and compact v2, reviewed alpha/small sizes, reproducible 16-asset native export and consumer manifest. Actual taskbar/tray/installer appearance remains NOT_RUN. |
| F — acceptance | Final Rust/desktop gates, real CLI previews and the existing production review/preview benchmark pass. The new home benchmark is measured, with retention/DOM bounds verified and no new latency budget asserted. Current native preview is UNVERIFIED because the debugging listener is unavailable on both current and earlier candidate control binaries (both use the new Clean UI). The 548 s no-debugger idle protocol and host-PID gates pass, but a hot WebView child keeps whole-application idle acceptance open. |

## Verification already complete

- `just ci`: **791 passed, 0 failed, 5 existing ignored**; format, workspace checks and Clippy pass. Log: `dist/cleanup-home-final/gates/just-ci-final.log`.
- `just desktop-web-check`: generated types, lint, TypeScript, **471 Vitest tests + 5 Node tests**, production build pass. Log: `dist/cleanup-home-final/gates/desktop-web-check-verified.log`.
- Real Node 22.23.2/npm 10.9.8 isolated CLI global/project scan, saved plan and digest-bearing preview pass, with English and Chinese human previews. Sentinels unchanged: `dist/cleanup-home-final/native-cli-04/`.
- Frozen 10k review/preview benchmark passes the existing 100 ms budget: row / select-all / preview p95 **14.1 / 14.3 / 13.6 ms**. Full 10k retention, 100 mounted rows and raw before/after method parity verified.
- New 10k-owner home benchmark measures **14.0 / 14.1 / 14.2 ms** p95 for first view / owner page / threshold refresh, retaining 10k owners and mounting 10. No new latency acceptance threshold is invented.
- D's 23 fixture screenshots cover sizes, both locales, saved themes, 125% text scale, reduced motion and forced colors. F adds four flow screenshots and an oversized display fixture. Native geometry at actual Windows DPR 1.25 was captured at 900x600, 1080x720 and 1350x900; Settings geometry/font/colors match the pre-edit baseline. Exact native binary/flow limits are in F.
- The final 548 s native idle run has five complete 60 s sample windows and normal shutdown. Host-PID gates pass (CPU median 0.0% at counter resolution, maximum private memory 4.99 MiB, maximum 14 threads). The owned tree consumes 70.08–82.49% of one logical core in those windows, mainly in a child that starts about 120 s after launch. Its cause is unproven; do not describe the whole application as low idle load. See F's `evidence/idle-protocol-review.md`.

## Fixes found during integration

Actual npm probing exposed Windows Node's inability to load a verbatim script argument. The dispatcher converts only the runtime spelling, recanonicalizes it to verify the same bound script, and preserves support hashes and authority. RED/GREEN tests and actual CLI previews verify the repair.

Two test-only repairs preserve assertions: an asynchronous Settings portal lookup waits for the accessible listbox, and MSIX tests sharing a global worker counter use one test lock. Product MSIX behavior was not changed.

## mbx and version boundaries

Pinned mbx 1.24.0 public outputs do not prove complete workspace ownership or the link/metadata/lease mutation footprint; successful exits may be busy partial outcomes. See `research/mbx-authority-qualification.md`. The integration observes the shared location with **unmeasured capacity**, not zero, and creates no mbx cleanup action. AC5 remains open.

Installed uv 0.13 remains outside the existing qualified version range. No provider version, shell dispatch, no-follow policy or cleanup authority was broadened simply to obtain a passing scan.

## Reproducibility and retained work

The pre-edit source/web/native/benchmark baseline remains frozen under `dist/cleanup-home-baseline/20261014-preedit/`; its manifest is in `evidence/baseline/`. No late baseline was substituted.

Final tracked and new working product sources are frozen in `dist/cleanup-home-final/source-final/`. Final CLI/native executables and production web inputs are in `dist/cleanup-home-final/native-fixed/manifest.json`. Benchmark pages are stored separately from packaged frontend inputs.

Use exact Node 22.23.2 and bundled npm 10.9.8. Rust builds use locked/offline dependencies and `--target-dir dist`; do not inherit CARGO_TARGET_DIR into scan fixtures. The external target link remains untouched. F's CI wrapper temporarily selects dist through a guarded Cargo config and removes its exact bytes afterward. The pre-existing Tauri Cargo.toml hash remains `b7d6f6a30067850f5c4b1f3afde6450667262eb57309d230ca05fe7a0dd84915`.

Task statuses remain in progress through the local Trellis review/closeout workflow; checked child criteria distinguish finished implementation from open integration acceptance. No commit, archive, journal, installation, publication or real user-cache cleanup was performed.
