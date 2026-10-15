# F integration result

Status: **partial acceptance; parent remains in progress**. Main implementation,
current code gates and component measurements are complete. mbx executable
cleanup and the native/manual items below are not delivered acceptance.

| Area | Result | Evidence |
| --- | --- | --- |
| Canonical Rust gate | PASS: 791 passed, 0 failed, 5 existing ignored; fmt/check/Clippy pass | `evidence/gates.md`, `gates/just-ci-final.log` |
| Desktop gate | PASS: generated types/lint/TypeScript, 471 Vitest + 5 Node tests, production build | `evidence/gates.md`, `gates/desktop-web-check-verified.log` |
| Real CLI scoped preview | PASS: global npm and project Node artifact, EN/zh human preview, saved plans and live SHA256 digests | `native-acceptance.md`, `native-cli-04/summary.json` |
| Cleanup during acceptance | None; preview reports dry_run=true, one selected/attempted, skipped=1, succeeded=0, failed=0; sentinels unchanged | CLI raw preview records and fixture manifests |
| Frozen 10k review/preview interaction budget | PASS: row 14.1, all 14.3, preview 13.6 ms p95 against 100 ms | `evidence/render-performance.md/json` |
| New 10k-owner home workload | MEASURED: first view 14.0, page 14.1, threshold 14.2 ms p95; 10k retained, 10 mounted. No new latency budget claimed | `evidence/home-performance.md`, manifest and raw samples |
| Fixture visual matrix | 23 layout/theme/locale captures + 4 flow captures + oversized display capture, with stated coverage limits | D `evidence/README.md`; F `evidence/fixture-flows.md` and home evidence |
| Native geometry | Three actual sizes at Windows DPR 1.25, Settings/Optimize comparison; captured on the pre-npm-fix binary with the same frontend asset names | `native-functional-08/captures/`; see binary boundary in `native-acceptance.md` |
| Final native scoped preview | UNVERIFIED: owned WebView has the correct debug argument but no listener, including an earlier candidate control binary in the same environment. Both have the new Clean UI; this is not the pre-edit baseline | `native-acceptance.md`, functional 09–12 records |
| Final no-debugger native idle | Protocol and inherited host-PID gates PASS; five complete windows, normal exit, unchanged sentinels. Whole-application idle remains OPEN: owned-tree CPU is 70.08–82.49% of one logical core, concentrated in a late WebView child of unknown role | `native-idle-final-01/`, `evidence/idle-protocol-review.md` |
| Matched short native diagnostics | Both 180 s runs close normally with no survivors or changed sentinels; neither reproduces the late child. Current versus earlier candidate (both new Clean UI), not a pre-edit baseline comparison; original anomaly remains OPEN | `matched-native-diagnostics.md` |
| Real taskbar/tray/installer visuals | NOT_RUN; source consumer and reproducible image exports are verified separately | E evidence and asset manifest |

Raw paths abbreviated as `gates/`, `native-*/`, `render-performance/` and
`home-performance/` above are under `dist/cleanup-home-final/`. These ignored
artifacts are retained locally; concise evidence and manifests live in this
task. Main production frontend inputs exclude benchmark pages.

## Open acceptance

1. mbx 1.24.0 does not expose the complete public ownership/expected-state and
   command footprint contract required by B. Location-only inspection is a
   safe partial result, not workspace cleanup or measured shared capacity.
2. Installed uv 0.13 has not been qualified against the existing provider
   envelope. Unsupported status is preserved rather than widening dispatch.
3. Current native successful dual-scope/dual-locale preview must be rerun when
   the owned WebView debugging channel becomes available, or via an equally
   verifiable native automation path. No cause or product regression is asserted
   from the unavailable listener; changing ports repeatedly did not fix it.
4. Complete native/manual visuals (taskbar/tray/installer, assistive technology,
   installed fonts and every state/locale combination) remain open. The inherited
   English target-checkbox accessible labels in Chinese views are documented.
5. Historical unrelated Software/Optimize/Analyze timings are not remeasured by
   this task's component and idle checks. Old failures remain old failures.
6. The formal idle run's late WebView child consumed substantial CPU. Read-only
   Clean source review found no self-sustaining update loop, which does not
   establish a runtime cause or an overall idle PASS. Preserve the raw run and
   read the matched diagnostic results before further investigation.

No user data cleanup, installer replacement, commit, archive, journal or
publication was performed. The pre-existing Tauri Cargo.toml edit is preserved.
Parent `acceptance-matrix.md` is the authoritative requirement map.
`evidence/final-integrity.md` records 603 matching frozen source files, unchanged
user-edited Cargo.toml, the six parent/child backlinks and all seven task context
validations. The active Trellis pointer is restored to the parent for continuation.
