# Execution and validation plan

## Planning delivery (this turn)

- [x] Inspect screenshots, current code, task history, design language and stores.
- [x] Record causal findings and distinguish source from runtime evidence.
- [x] Create parent plus four children, each with PRD/design/implementation plan.
- [x] Curate spec/research manifests and validate references.
- [x] Present final planning summary; user approved implementation on 2026-11-06
  with "按照规范和规划开始实施". Approval covers the four planned children.

## Implementation order after approval

1. Review the final artifacts and confirm the approved scope. Activate the owning
   child, not this coordination-only parent.
2. gpui-shell-footer: remove footer and migrate focus/status. Validate no Note
   entity/accessibility node and all surviving input/navigation behavior.
3. gpui-settings-fonts-about: prove catalogue adapter, connect draft/save/render
   and About. Validate persisted rendering with at least two installed families.
4. gpui-clean-workspace: scope-first entry and viewport-aware data review;
   preserve explicit scan, preview, digest and confirmation boundaries.
5. gpui-software-workspace: default inventory, single tabs, truthful per-segment
   tools and states; migrate stale UIA assumptions.
6. Integrated native review and final requirements-to-evidence pass. Record
   remaining unsupported checks explicitly; do not mark complete on source alone.

## Commands

- just gpui-check
- just gpui-test
- just gpui-build
- just ci (canonical repository gate; does not replace GPUI standalone gates)
- If shared/Tauri contracts are actually changed, run their relevant additional
  gates from the existing specs. No Tauri change is planned.

During iteration run the affected module tests, then the complete gates once
the implementation is ready. Keep fixtures isolated from live preferences and
cleanup roots. No real cleanup, uninstall or startup mutation for verification.

## Native evidence matrix

- Viewports: 1280x800, 1536x1024, narrow 960x720 logical pixels.
- App scale: 100/125 plus a 110 smoke; en and zh-CN.
- Appearance: dark/light and high-contrast focus/contrast spot checks.
- Font: System, installed Latin family, installed CJK family; missing saved name;
  >=2000 injected catalogue entries; duplicate/localized names; empty/error.
- Clean: no roots, saved roots, global confirmation, scan/cancel, found/review,
  no targets, error/retry, preview invalidation, fixture confirmation/result.
- Software: first inventory, segment change, search/sort/filter, empty versus
  filtered-empty, missing winget, permission limitations, stale completion,
  preview/cancel/result using fixtures.
- Shell: startup, blank click, route/back, menu, titlebar, focus loss, Tab/
  Shift+Tab, Alt navigation, Enter/Space/Escape, IME/search across all five modes.

Save actual native screenshots and UIA assertions with window size, app scale,
OS DPI, locale/theme and build identity. Contrast-check real composite surfaces.
Do not claim new visual verification from the supplied screenshots.

## Review and rollback gates

A. Footer child: search and keyboard regressions block integration.
B. Settings child: JSON-only success without rendered font change blocks.
C. Clean child: any scan on passive scope selection or weakened digest gate blocks.
D. Software child: duplicate destinations or added execution authority blocks.
E. Integration: compare against research/audit.md and every parent AC.

The design-atlas checker currently reports files=0 on Rust; use judged review
and native evidence, not a zero-findings badge. Fresh-context implementation
review should use the applicable Trellis checker; this planning audit is self-review.
