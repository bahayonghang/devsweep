# Implementation plan

## Entry gate

- [x] User explicitly authorized a parent/child planning task.
- [x] Inspect current source, screenshot, existing specs, catalogue and relevant primary sources.
- [x] Confirm user warning policy: configurable 5 GiB, scan-triggered only.
- [x] Persist parent/child PRD, technical design, execution plans and curated contexts.
- [x] User approved the final planning package on 2026-10-14: proceed in Trellis task order.
- [x] Continue in the current dev checkout and preserve the pre-existing desktop/src-tauri/Cargo.toml modification.

## Ordered work

0. Parent-owned read-only preflight, before A/E or any child writes product files: capture the current fixture/native geometry and performance baseline with source/binary identities, existing working-tree diff identity, scope assertions, and raw-method manifest. Store it under this parent's evidence/baseline/ (large generated outputs may stay in an identified dist directory). If a required baseline cannot be captured, record NOT_RUN and resolve the evidence gate before changing that surface. This does not start or complete F.
1. A: freeze scope/capability/selection compatibility and extend existing contract tests. Confirm global with no roots and CLI grammar unchanged.
2. E in parallel with core work if desired: review generated animal concept, simplify small mark and validate asset pipeline.
3. B: qualify mbx public ownership interfaces first; implement existing project grouping/dedupe and safe supported provider. Stop the executable portion if no trustworthy footprint can be bound.
4. C: implement pure project artifact aggregation and preference-backed threshold. Test complete/partial/unknown and boundary cases before wiring UI.
5. D: promote design to desktop/DESIGN.md; update superseded Clean visual specs; implement two independent scope slots/entry points, route retention, truthful results and warning settings. Fix explicit input type styling. Integrate reviewed E assets.
6. F: run integrated gates, fixture visual matrix, native scope/parity, original and new performance workloads. Resolve regressions before parent acceptance.
7. Record results and remaining evidence honestly. Commits, archive, journal and delivery actions require their own user authorization.

## Verification commands and evidence

At repository root: just ci, git diff --check.
Desktop requires Node >=22 <23 and npm >=10 <12. Verify the effective executables. Run types:generate -- --check, lint, typecheck, test and build from desktop, following .trellis/spec/desktop-frontend/index.md's bundled-npm recipe.
Use existing core/CLI provider_authority, project scanning/ranking, frontend scope/reducer/coordinator/large-results suites rather than mirror-the-implementation tests.
Native driver must assert both flags and roots and capture submitted IPC before every scan. Use isolated settings/cache/project fixtures and owned processes; no broad user-cache scans or actual cleanup.
Record baseline before implementation, exact binary/source hashes, OS/WebView/runtime versions, host, warm-up and raw sample counts. Reuse frozen budgets/methods; no new arbitrary thresholds or median substitution.
Visual matrix: 390/800/1024/1350/1440 CSS widths, 320px controls, both locales, all themes, text scale, default and selected local font, long paths, keyboard, forced colors, reduced motion. Native configured minimum is 900x600, default 1080x720, plus screenshot size 1350x900. Separate DPR emulation from actual Windows scale evidence.
Document baseline and final region/input geometry; report checked/changed/left consumer counts. Keep fixture/native/installed-installer evidence distinct.
Image inspection: alpha, crop, dimensions, small-size silhouette, light/dark use and native taskbar/tray. New image is not automatically release-ready.

## Review/rollback points

Review A's selection and DTO compatibility before B/C consumers. Review B's mbx qualification before any executable intent. Review C's confidence math before D. Review D's state retention and stale digest fencing before native tests. Retain prior brand files until new assets pass.
Never unblock by relaxing no-follow, changing saved corrupt bytes, invoking shell cleanup, loosening frozen gates, or asserting missing native evidence passed.

## Planning closeout

Run task.py validate on all seven directories; verify parent/children backlinks, dependency names, nonempty manifests, no placeholder requirements, existing file references and planning status. No product test suite is claimed as run in this planning turn.

