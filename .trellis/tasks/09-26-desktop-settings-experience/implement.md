# Execution Plan: Desktop Settings Experience

Status: approved for sequential execution on 2026-09-26. The latest user message authorizes implementation of all four children in the documented order. See research/approval.md.

## Before activation

- [x] Q1 resolved: both scoped production dependencies approved on 2026-09-26.
- [x] The user reviewed the latest planning summary and explicitly requested sequential implementation on 2026-09-26.
- [x] Confirm baseline fea6368 and the four sequential child dependencies. The existing operation-performance/native-acceptance tasks retain their owners.
- [x] Validate both JSONL manifests for all five tasks after recording approval. All ten manifests pass task.py validate on 2026-09-26. Activate each child before dispatch.

## Ordered work

1. Navigation child: create the primary-navigation identity; promote Settings after Status; preserve route drain, history, focus, and mode-local state. Update the shell-specific spec clauses and tests.
2. Controls child: build the approved shared Select/Combobox presentation, restyle Settings rows, preserve independent language and preference save states, and leave theme extension/font enumeration to their owners.
3. Fonts child: add native read-only catalogue; migrate core/IPC/fixtures/decoders to V2; wire searchable family selection, refresh, fallback, and main/HUD font application.
4. Palettes child: extend the V2 theme values and closed catalogue, define complete tokens, add radio previews, validate all main/HUD/portal surfaces, and update theme specs.
5. Parent integration: trace R1-R5 to acceptance evidence; check Settings tab, font selection and theme switching in one user flow; review changed specs and the V2 downgrade boundary.

Each child owns a complete reviewable change. Shared AppShell, SettingsPage, contract, and style files follow the sequence above. Do not run overlapping workers against those files. Do not release a partial V2 implementation between children.

## Validation commands after implementation

Use Node 22 as required by desktop/package.json.

- Focused component/contract tests: from desktop, npm test -- src/app-shell/AppShell.test.tsx src/app-shell/registry.test.ts src/preferences/preferences.test.tsx src/styles.test.ts, plus newly added Settings control/font tests.
- Focused core store tests: cargo test --locked -p devsweep-core desktop_preferences.
- Generated artifacts: from desktop, npm run types:generate, then npm run types:generate -- --check. Use the existing catalogue-generation process for resources/i18n changes.
- Full desktop frontend gate once after integration: just desktop-web-check.
- Native adapter and wire gate: just desktop-test.
- Canonical shared Rust gate: just ci.
- Native artifact for visual acceptance: just desktop-build when ready for final Windows evidence. Do not install the app automatically.

Inspect command definitions before execution; do not record a check as passed merely because a command is listed here. A focused test failure blocks its owning child. Existing unrelated failures retain their original owner and evidence.

## User-flow evidence

- [ ] Both locales: six ordered tabs; Settings active announcement; key navigation; direct #/settings; back/forward; failed and successful mode drain.
- [ ] Open all Settings popups at 390/800/1024/1440 CSS px and at 100/125 percent text scale; no clipped choices or inaccessible controls. Check 110 percent in focused scale tests.
- [ ] Test Select and font Combobox with pointer, arrows, Enter, Escape, Tab, Chinese IME, empty search, long names, loading, unavailable, and failed save.
- [ ] Test a 2,000-family synthetic list with all results reachable and no per-keystroke native enumeration. Record interaction measurements; do not invent a latency result or add virtualization without evidence.
- [ ] Native font evidence: OS/version, binary identity, catalogue count, system plus available installed families, refresh, restart, and HUD rendering. Do not install/uninstall fonts to manufacture evidence.
- [ ] All seven theme choices: token completeness and contrast checks. Capture the four new palettes in Settings with an open popup and HUD. Exercise representative table, risk/error, and confirmation surfaces in both schemes.
- [ ] Forced colors and reduced motion override every palette. Test full labels, text scale, focus, and saved/failed state.
- [ ] V1 read-only conversion, first V2 commit, concurrent patches, missing font, malformed/future documents, failed replacement, and stale main/HUD notifications.
- [ ] Preserve sampling controls, resets, language bytes, and cleanup authority.

The full viewport/locale matrix applies to the changed Settings surface. Other screens need focused token/state coverage; this task does not repeat unrelated operation benchmarks.

## Rollback and completion

Before migration, rollback is a normal code revert. After V2 first save, retain both preference files. A previous build uses V1; V2 choices remain for a later compatible build. Never delete user files as rollback. Revert UI/IPC/core contract changes together if a contract regression appears.

The parent can complete only after all four children pass their checks and R1-R5 have evidence. Record each implementation/check outcome before advancing. Run the documented product gates against changed code. Keep task completion and any commit/archive steps consistent with the repository workflow and actual evidence.
