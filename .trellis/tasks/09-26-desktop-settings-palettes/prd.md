# Shared Desktop Palettes

## Goal and scope

Provide coherent application colors and the four requested named presets. Own parent R4 and theme portions of R5.

Approved for sequential implementation on 2026-09-26. Parent: ../09-26-desktop-settings-experience/. Read the parent PRD, design, research, and approval record before this child. The user approved @base-ui/react and the Windows-only windows crate DirectWrite binding, and authorized the documented task order.

Current evidence: parent research/repository-findings.md E9-E10 and E14. The current app supports dark/light/system across main/HUD. The requested palette extension preserves those choices and their default behavior.

## Requirements

- T1: Add Catppuccin Latte, Catppuccin Mocha, Codex, and Claude choices with visible theme previews. Keep DevSweep dark, DevSweep light, and System.
- T2: Apply complete semantic colors across main destinations, titlebar, capsule, popup portals, support pages, errors, confirmations, tables/diagrams, and HUD.
- T3: Separate palette ID from CSS light/dark scheme; preserve commit-only updates, restart persistence, hidden-HUD reload, and system preference listening.
- T4: Preserve accessibility and risk semantics in all palettes. State source provenance accurately.

## Acceptance criteria

- [ ] T-AC1 (T1): All seven choices are available with one checked selection. Both named Catppuccin palettes use official source values; Codex/Claude are identified in the design record as DevSweep-inspired proposals.
- [ ] T-AC2 (T2): Every palette has all required semantic tokens. Representative operational/support/dialog/error/HUD surfaces and open Settings popups show readable matching colors without dark-only leakage into light themes.
- [ ] T-AC3 (T3): All theme IDs survive restart and remain synchronized across main/HUD. Failed save preserves the previous theme. OS scheme changes affect only System; CSS color-scheme always resolves to light or dark.
- [ ] T-AC4 (T4): Ordinary text reaches 4.5:1 contrast; large text and essential control/focus boundaries reach 3:1 in context. Checked/highlighted/error states also have a non-color signal. Forced colors override every palette.
- [ ] T-AC5 (T4): Both locales and 100/110/125 percent text stay readable. Reduced motion remains effective. Theme changes do not alter sampling, operation state, original planet palettes, or cleanup authority.

## Exclusions and dependencies

Controls and fonts/V2 children complete first. Parent dependency approval and planning review were completed on 2026-09-26. No arbitrary color editor, downloadable theme, theme sync, copied third-party assets, extra Catppuccin flavors, or automatic default-theme replacement. Codex dark and Claude warm light are the current review proposals; changing those proposals before implementation only changes planning artifacts.
