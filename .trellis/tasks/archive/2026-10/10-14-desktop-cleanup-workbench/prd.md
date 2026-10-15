# D: Two-part desktop cleanup workbench

## Goal

Replace the Clean hero/form with a clear utility workspace for independent global-cache and project-artifact inspection and review.

## Parent and dependencies

Parent: [cleanup-home-redesign](../10-14-cleanup-home-redesign/prd.md).
A; B/C frozen contracts before integrated behavior; E's reviewed mark before final visual acceptance. Token/layout work can use reviewed fixtures earlier.

## Requirements

D1. Promote parent design-system/DESIGN.md to desktop/DESIGN.md and implement its compact plain-CSS token language. Preserve root historical design.md.
D2. Render independent global/project scan actions and report slots, provider rows and project-owner groups, and explicit project-root setup.
D3. Keep one shared heavy-operation coordinator, scoped errors/progress, retained session reports, explicit cancel/join and busy feedback.
D4. Reproduce and fix Clean route/Settings retention through a stable Clean-owned controller. Root edits invalidate project state only; scope/report identity fences stale results.
D5. Integrate C's configurable warning and confidence text, project opt-in selection, and existing preview/confirmation flow. Fix input type/style mismatch.
D6. Remove mandatory large Earth from Clean only, update the conflicting specs, preserve other modes, themes, font/scale preferences and navigation.
D7. Use the E animal mark and consistent semantic Glyph meanings; no new runtime design/icon dependency.

## Acceptance criteria

- [x] First visit with no roots has an enabled global scan; exact request flags/roots are asserted; project setup errors cannot block global use.
- [x] Run one scope then the other; errors/cancel/stale completions cannot erase or authorize the other report. Route -> Settings -> back retains correct observations.
- [x] Preview/selection/root changes, protections and stale digests remain fail-closed; no synthetic merged plan is created.
- [x] Completed rows group by provider or authoritative project root; full-report search/selection works beyond pagination; mounted rows/groups remain bounded.
- [x] Warning settings show 5 GiB default and preserve state after reload; warnings never select cleanup targets.
- [x] Computed styles prove the root input gets intended type, font, background, boundary and height; saved font remains respected.
- [ ] Baseline/final screenshots, keyboard/forced-colors/reduced-motion and EN/zh-CN size/theme matrix pass with measured geometry.
- [ ] Other modes and Settings retain their existing palette and geometry in before/after comparisons, apart from the intentional shared animal logo. New palette aliases stay within Clean.

## Scope boundaries

Implementation authorized on 2026-10-14. Preserve the pre-existing desktop/src-tauri/Cargo.toml edit. No actual user-cache cleanup, unrelated modes, commits, archive or publication. Parent scope exclusions apply.

## Decision status

User product decisions and selected design are in the parent PRD. No additional product question blocks this plan; implementation depends on the stated upstream contracts.

