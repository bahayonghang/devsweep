# Settings Navigation

## Goal and scope

Make Settings directly reachable as the tab immediately after Status. Own parent R1 and navigation portions of R5.

Approved for sequential implementation on 2026-09-26. Parent: ../09-26-desktop-settings-experience/. Read the parent PRD, design, research, and approval record before this child. The user approved @base-ui/react and the Windows-only windows crate DirectWrite binding, and authorized the documented task order.

Current evidence: parent research/repository-findings.md E1-E3 and E14. Settings already has #/settings; ModeId is limited to five operational modes.

## Requirements

- N1: Show Clean, Software, Optimize, Analyze, Status, Settings in order when all five modes are available. Keep Settings available with any nonempty operational-mode registration set.
- N2: Settings participates in selected-state announcement, roving keyboard focus, panel association, locale labels, and horizontal capsule scrolling.
- N3: Preserve deep links, browser history, mode-local state, and cancel/join lifecycle. Focus movement alone does not navigate.
- N4: Remove the duplicate Settings brand-menu entry and Settings back-to-last-mode header control. Preserve supporting-page menu/back behavior.

## Acceptance criteria

- [ ] N-AC1 (N1): Both locales show Settings after Status with one selected primary tab and the correct panel role/label.
- [ ] N-AC2 (N2): Arrows, Home, End, Enter, Space, and Tab reach all six destinations. At 390 px, focused Settings scrolls into view without truncating its label. A supporting route leaves one usable primary tab stop.
- [ ] N-AC3 (N3): Direct #/settings and browser back/forward restore correct selection/focus. A coordinator rejection preserves the former route/hash and exposes the existing error. A stale transition cannot win.
- [ ] N-AC4 (N3): Clean/Software/Optimize/Analyze/Status continue through their existing cancellation and state-retention paths; Settings does not start a sampler.
- [ ] N-AC5 (N4): The menu has no Settings duplicate; History/Rules/Protection and Help still work. Supporting back behavior is unchanged.

## Exclusions and dependencies

No change to ModeId, operational capabilities, CLI/TUI navigation, or operation authority. No new Settings controls or palette/font data. This child runs first. Controls follows because both touch the shell/page composition. The parent dependency decision and sequential implementation plan were approved on 2026-09-26.
