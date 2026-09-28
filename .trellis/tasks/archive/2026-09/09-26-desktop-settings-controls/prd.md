# Settings Controls and Layout

## Goal and scope

Provide consistent, readable Settings rows and popup choices. Own parent R2 and the shared control part of R3/R4.

Approved for sequential implementation on 2026-09-26. Parent: ../09-26-desktop-settings-experience/. Read the parent PRD, design, research, and approval record before this child. The user approved @base-ui/react and the Windows-only windows crate DirectWrite binding, and authorized the documented task order.

Current evidence: parent research/repository-findings.md E4, E8, and E12. The screenshot reports the current native popup presentation as unacceptable.

## Requirements

- C1: Replace Settings native select presentation with theme-aware accessible Select controls. Fonts use a searchable Combobox interface; themes use visible radio previews.
- C2: Align headings, labels, descriptions, controls, preview, and reset actions in one responsive layout.
- C3: Show the last committed preference during save/failure. Filtering/highlight/dismissal do not persist values. Keep language saving separate.
- C4: Support pointer, keyboard, screen reader, Chinese IME, forced colors, reduced motion, and existing text scales.

## Acceptance criteria

- [ ] C-AC1 (C1): All remaining Settings choice popups have consistent trigger/popup/selected/highlight/disabled styles, accessible labels, and value announcements. A check indicator distinguishes selected options without color alone.
- [ ] C-AC2 (C1, C4): Popups escape parent clipping, flip/constrain at viewport edges, scroll, and restore focus on dismissal. Escape preserves the committed value; Enter/pointer choice sends one field patch.
- [ ] C-AC3 (C2): Appearance, language, and performance share aligned rows. At 390/800/1024/1440 widths and 125 percent text, all controls, labels, reset actions, and notes remain reachable.
- [ ] C-AC4 (C3): Loading, saving, failed save, unavailable store, and reduced-motion-disabled frame cap remain accurate. Selecting the current value does not send a redundant update.
- [ ] C-AC5 (C4): Keyboard/typeahead/search and IME interactions work with long English/Chinese names. High contrast and reduced motion work while the popup is open.

## Exclusions and dependencies

Navigation child first. The user approved @base-ui/react on 2026-09-26. This child creates the reusable font-picker interface with existing choices; the fonts child replaces the source with the complete OS catalogue. This child supports the existing three themes; the palettes child supplies the four new tiles/tokens. Do not claim full parent font/theme acceptance from these intermediate controls. Do not replace selects outside Settings.
