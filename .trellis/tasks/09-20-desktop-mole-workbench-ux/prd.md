# Rebuild Mole-inspired desktop workbench UX

## Goal

Make the existing five-mode Tauri surface feel like one deliberate developer
workbench: review-first, dense where data is comparable, progressive where
evidence is detailed, and calm during long operations. Use Mole's information
hierarchy as a reference while keeping DevSweep's original Windows identity.

## 2026-09-23 revision

The user selected a Mole-style capsule shell with procedural planets. The
sidebar clauses of R1, the "no planet metaphors" clause of R2, and the shell
parts of AC1/AC2 move to `09-23-desktop-mole-capsule-shell`, which supersedes
them. This child keeps R3 (state truthfulness through existing reducers), R4
(state/locale/width coverage), and R5 (accessible long data). Close this child
after the capsule shell child lands and its remaining ACs are rechecked
against the new shell.

## Dependency and boundary

- Parent: `09-20-desktop-mole-tauri-performance`.
- Starts only after the parent planning summary is explicitly approved.
- This child may change `desktop/src/` presentation, styles, and UI fixtures.
  It must not change core safety, CLI grammar, Tauri command authority, or
  generated wire types. It consumes the existing `DesktopBridge` and
  `OperationCoordinator` contracts.

## Requirements

- R1: Keep the persistent sidebar, five primary modes, named supporting
  destinations, visible page headers, deep links, back navigation, focus
  restoration, arrow/Home/End navigation, and Alt accelerator behavior.
- R2: Establish one original visual grammar: dark mode canvas, mineral/forest
  accents, raised cards, compact status chips, grouped rows, evidence
  disclosure, and a stable action/status boundary. Remove generic light
  fallback surfaces and inconsistent spacing without introducing gradients,
  glass, fake macOS chrome, planet metaphors, or copied Mole assets/copy.
- R3: Refine Clean, Software, Optimize, Analyze, and Status states using the
  existing reducers and DTOs. Preserve Clean selection/dry-run/confirmation,
  Analyze read-only behavior, Software manual-only rows, Optimize closed
  catalogue, and Status unavailable/partial semantics.
- R4: Verify empty, loading, partial, error, canceled, preview, and result
  states in English and Simplified Chinese at 390/800/1024/1440 CSS pixels,
  keyboard-only, reduced-motion, and forced-colors conditions.
- R5: Keep all user data accessible and copyable when visually ellipsized;
  never truncate authority, warning, action, or confirmation copy.

## Acceptance Criteria

- [x] AC1: All five modes render through the existing typed registry with no
      placeholder route, and route/back/focus/keyboard tests remain green.
- [x] AC2: The shell and each mode use the same dark card/row/action grammar;
      no light workbench pane, Mole asset, copied string, or “space freed” copy
      appears in production or tests.
- [x] AC3: Clean's inspect-only, selection invalidation, dry-run digest, and
      second-confirmation behavior are unchanged by visual composition.
- [x] AC4: Analyze stays read-only; Software and Optimize expose only existing
      capabilities; Status never renders unsupported values as zero.
- [x] AC5: Frontend tests cover both locales, responsive breakpoints,
      reduced-motion/forced-colors selectors, accessible long data, and all
      meaningful mode states.
- [x] AC6: `mise exec node@22 -- npm run lint`, `typecheck`, `test`, and `build`
      pass in `desktop/`; no Rust/CLI contract files change without a traced
      parent decision.

## Out of scope

- New Tauri commands, CLI flags, providers, dependencies, or core/domain
  behavior.
- Mole planets, hamster, traffic lights, marketing copy, screenshots, or
  pixel-identical layout.
- Native Windows scaling and process/resource claims; those belong to
  `desktop-native-acceptance` and `desktop-operation-performance`.

## Closure recheck (2026-09-24)

Rechecked against the capsule shell (`2fc15c0`) and the four 09-23 parity
children. Evidence:

- AC1: `AppShell.test.tsx` covers the typed registry, deep links, back
  navigation, focus return, and keyboard movement on the capsule tablist.
- AC2: `styles.test.ts` locks the dark canvas, radius tokens, and shared
  grammar. `resources/i18n/*.json` contain no "freed" copy. The planets are
  procedural (`desktop/src/stage/`); no Mole asset or string is present.
- AC3: `App.test.tsx` "invalidates a dry run after selection changes, then
  confirms and reports execution"; `CleanWorkbench.test.tsx` covers
  skip/protect with confirmation.
- AC4: The "Analyze stays read-only" clause is superseded by the parent
  2026-09-23 revision. Analyze now reveals paths and moves them to the Recycle
  Bin only after a digest-checked confirmation (`8dd88d0`). Software and
  Optimize expose only shipped capabilities. Status renders unsupported
  GPU/thermal values as unavailable, not zero (`StatusWorkbench.test.tsx`).
- AC5: `styles.test.ts` covers the narrow layout, reduced motion, and forced
  colors. Each mode has a zh-CN render test. `AppShell.test.tsx` covers
  accessible, copyable truncated data.
- AC6: On 2026-09-24, `npm run lint`, `typecheck`, `test` (43 files, 295
  tests), and `build` passed in `desktop/`. Rust and CLI contract changes
  came from the 09-23 children under the parent revision, not from this child.

Native scaling and resource claims stay with `09-20-desktop-native-acceptance`.
