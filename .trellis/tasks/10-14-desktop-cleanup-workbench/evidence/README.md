# D implementation and fixture verification

The Clean home now has independent global-cache and project-artifact sessions. Global requests use no roots; project requests use explicit validated roots. CleanSession retains reports, selections, previews and session-only threshold preferences above route changes. Scope/revision/scan-id fences and the shared coordinator preserve preview/confirmation authority. The warning uses C's Rust-owned project projection and never changes selection.

## Visual evidence

- `dist/d-clean-ui-verified/`: 23 actual running fixture screenshots with `geometry.json`, `retention.json`, and normal browser teardown. `capture-ui.mjs` reproduces the matrix in an isolated owned browser and fixture server. No native/cache mutations.
- Checked: idle widths 320/390/800/900/1024/1080/1350/1440, active global scan, both completed scope reports, project review, Settings, six saved themes, English/Chinese, actual saved125%scale, emulated reduced motion and forced colors. Zero horizontal document overflow in23 cases.
- At1350: exact2:3 section widths504/756px before scrollbar. Visible text/number inputs44px. At1080x720: global scan bottom394.28px and project scan bottom694.28px, both visible.
- Actual Settings125%scale: root16→20px and Clean heading25.6→32px. No double scaling or font reset. Both completed reports retained through all Settings/theme/locale round trips.
- Iteration changed2 measured issues: threshold form collapsed behind current value to expose the project scan action; completed folder editor collapsed behind explicit folder count to expose project capacity/confidence. Shared shell geometry and other four mode implementations stayed unchanged.
- The first browser attempt could not reach a sandboxed server (`dist/d-clean-ui-first`); the second expanded-control capture is superseded. `d-clean-ui-final` stopped at an incorrect English language option selector and lacks the final manifest. Only `d-clean-ui-verified` is the accepted matrix.

## Checks and limits

- Node22 production build PASS (`dist/d-build.log`); expected existing large-chunk advisory remains.
- ESLint PASS (`dist/d-final-lint.log`), typecheck PASS in independent review. Scope/session/owner pagination tests, exact scoped scans, retained previews, threshold request ordering and protection invalidation cover the new behavior.
- Independent review: `review.md`; App/AppShell tests: `../app-test-handoff.md`.
- Full desktop suite: 63 files / 471 tests plus 5 Node script tests PASS (`dist/d-full-tests-final.log`). Subsequent Stage test strengthening passed independently; parent F owns the final combined gate.
- This screenshot matrix does not cover oversized, empty, error, canceled, confirmation, or execution-result states. Those behaviors have component/contract tests; they are not claimed as screenshot or native acceptance here. Keyboard and accessibility assertions likewise do not substitute for native assistive-technology testing.
- F subsequently captured cancellation, dry-run preview, second confirmation and the in-memory fixture execution result at 1350×900: `../../10-14-cleanup-home-acceptance/evidence/fixture-flows.json` records hashes and exact limits; raw images are in `dist/cleanup-home-final/fixture-flows/`. All four were visually inspected with zero horizontal overflow and normal owned-browser exit. Empty/error remain without screenshots. F's `home-performance.md` records the separately inspected oversized display capture, with the production home components but without the shared shell.
- Source/image hashes: `visual-manifest.json`. Original preexisting Cargo.toml SHA256 remains b7d6f6a30067850f5c4b1f3afde6450667262eb57309d230ca05fe7a0dd84915.
- Native package/taskbar/tray, full native matrix and formal production performance are F work, not claims from this fixture verification. Controlled fixture diagnostics remain backend-authored English data in Chinese pages. Native platform DPI/font behavior is not proven by browser emulation.
