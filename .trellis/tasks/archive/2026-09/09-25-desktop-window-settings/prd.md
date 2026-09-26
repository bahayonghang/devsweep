# Desktop Window Controls And Settings

## Goal

Replace the main window's native caption controls with application-drawn controls and provide a complete Settings page. Users can choose appearance and bounded performance preferences that have real runtime effects.

## Background And Confirmed Decisions

The existing React/Tauri shell has a settings deep link and a language-only panel. The main window uses native chrome. Desktop presentation uses a shared dark-only palette, while HUD has an independent entry and stylesheet. The current shared language file is closed V1 and is also read by TUI.

The user requested custom-drawn minimize/maximize controls and a Settings page with theme, fonts, and performance configuration. The user then confirmed dark, light, and system themes across all main pages and HUD, preserving dark as the default. The native-chrome and dark-only clauses in the current desktop spec must change within the relevant child implementation.

Current-code evidence and exact anchors are in research/window-chrome.md and research/settings-runtime.md. The configuration matrix in the settings child's design.md owns the proposed values, defaults, runtime consumers, and apply timing.

## Requirements

- R1: Use one custom main-window titlebar with minimize, maximize/restore, and matching close controls. Preserve native drag/resize/keyboard behavior, authoritative maximize state, and close-after-drain semantics.
- R2: Expand the existing Settings destination into a dedicated page within the current shell. Preserve deep links, back navigation, focus, mode state, and language behavior.
- R3: Support the confirmed dark/light/system theme scope across every mode, supporting destination, error/confirmation surface, titlebar, and HUD.
- R4: Support bounded local font presets and text scale with readable Chinese, English, and numerical content. Retain the existing shared language store and resolver.
- R5: Add effective performance preferences for reduced motion, Planet frame rate, Status live interval, returned process rows, and HUD interval. Every control must reach a concrete consumer. Apply timing must be visible.
- R6: Persist validated desktop preferences with defaults, atomic field/group changes, restart behavior, error recovery, and compatibility. Keep shared language V1 bytes intact and keep both windows consistent.
- R7: Preserve operation ownership, cancellation/join, wait-only cleanup, HUD permissions, cleanup authority, accessibility, and existing bounded resource contracts.

## Task Map And Requirement Coverage

| Child | Responsibility | Parent requirements | Own acceptance |
| --- | --- | --- | --- |
| 09-25-desktop-custom-window-controls | Main titlebar, window bridge, exact capabilities, and native control behavior | R1, R7 | W-AC1 through W-AC6 |
| 09-25-desktop-settings-preferences | Settings page, store/IPC, themes/fonts, motion, and Status/HUD consumers | R2-R6, R7 | S-AC1 through S-AC11 |

The parent owns requirement coverage and combined acceptance. Both children can validate their owned behavior independently. The planned implementation sequence is window controls, then settings, then combined review. The settings child needs the finished titlebar only for integrated appearance acceptance. Do not infer dependencies from tree order.

## Acceptance Criteria

- [ ] AC1 (R1): Exactly one application-drawn main-window control set performs minimize, maximize, restore, and close. The icon follows native state. Window input and lifecycle checks pass.
- [ ] AC2 (R2): Brand-menu Settings and #/settings show the same complete page. Back/focus and existing language behavior remain correct.
- [ ] AC3 (R3, R4): All main destinations, titlebar, and HUD support dark/light/system, the selected font preset, and supported text scales. System changes and new/open HUD cases remain consistent.
- [ ] AC4 (R5): Motion and frame cap affect the actual renderer; Status interval and row limit reach the real command call sites; HUD interval reaches the Rust sampler. No UI-only performance control ships.
- [ ] AC5 (R6): Values survive restart; missing storage uses defaults; failed or incompatible storage preserves bytes; field updates do not lose unrelated changes; main/HUD reject stale snapshots; group resets stay within scope.
- [ ] AC6 (R7): Theme/font edits retain domain results and cleanup authority. Settings navigation uses the existing operation drain. Custom close waits for owned work. Hidden HUD sampling remains stopped.
- [ ] AC7 (R3, R4, R7): Both locales, keyboard-only use, reduced motion, forced colors, and the existing viewport/scale protocol pass. Browser and native evidence remain separately labelled.
- [ ] AC8 (R1-R7): Child checks and combined evidence cover every requirement. Existing language/TUI compatibility and relevant desktop/Rust quality gates pass. Existing performance/native tasks retain their own unresolved findings and artifact-bound evidence.

## Boundaries With Existing Tasks

09-20-desktop-operation-performance owns measured default-workload performance defects and thresholds. User-selectable slower sampling does not count as fixing those defects. 09-20-desktop-native-acceptance owns its existing frozen-binary matrix and pending operator rows. This task adds focused evidence for the changed features and links evidence where useful; it does not close or rewrite either older task.

## Out Of Scope

Cleanup policy changes, permanent delete, automatic cleanup, software removal, OS optimization, arbitrary scan worker/budget settings, CPU/RAM quotas, close-to-tray, autostart, automatic Status pause/resume, font imports/downloads, cloud sync, configuration import/export, and a new primary navigation layout.

## Risks And Deferred Items

Custom chrome requires native drag/resize/snap/close verification. Windows maximize-hover Snap Layouts need separate native support research and are deferred; keyboard/edge snapping remain required checks. Full light/font support affects main and HUD styles, including existing fixed font sizes. Font fallback and native scale results must be recorded on the actual host without changing OS display settings.

## Planning Status

The confirmed theme choice is incorporated. No unresolved product question blocks final plan review. Parent and children have PRD, design, implementation plan, and context manifests. The user approved implementation of both children. The parent coordinates integration; each child enters in_progress when started. Runtime acceptance remains open until verified.
