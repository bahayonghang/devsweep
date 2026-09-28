# Implementation Plan: Palettes

Prerequisites: controls and fonts/V2 complete, parent plan approved, child activated.

- [x] Freeze the four named preset proposals and source attribution; define all seven IDs and light/dark resolution.
- [x] Extend the core enum, patch/response fixtures, generated types, closed decoders, and wire-parity cases together.
- [x] Implement complete semantic tokens, including popups, selected/highlighted states, titlebar, and HUD.
- [x] Add theme radio previews and preserve commit-only behavior and appearance reset.
- [x] Replace two-palette-specific tests with exhaustive catalogue/token tests; add actual contrast-pair calculations.
- [x] Review main modes, supporting views, errors, confirmations, diagrams, and HUD for old hardcoded color leakage caused by this extension.
- [ ] Capture Settings/open-popup/HUD native evidence for the four new palettes, both locales, text-scale extremes, and forced colors.
- [x] Update only the relevant visual/preference spec clauses and record measured results.

Implementation and focused checks completed on 2026-09-26. See
evidence/implementation-report.md and evidence/contrast.json. The parent owns
native evidence, independent review, and the final integrated gates. Native
acceptance remains open until those results are recorded.

Focused commands: cargo test --locked -p devsweep-core desktop_preferences; from desktop npm run types:generate and npm test -- src/preferences/preferences.test.tsx src/styles.test.ts plus new palette tests. Run the parent's final just desktop-web-check, just desktop-test, and just ci once against the integrated change.

Record T-AC1 through T-AC5. Do not treat sample swatches or source inspection as proof of contrast or native rendering. Run the documented product tests during implementation.
