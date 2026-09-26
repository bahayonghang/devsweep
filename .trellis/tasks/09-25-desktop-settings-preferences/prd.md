# Settings Page And Desktop Preferences

## Goal

Give users one Settings page for language, theme, fonts, and bounded performance preferences. Every control must affect the documented runtime consumer and survive restart.

## Ownership And Dependencies

- Parent: .trellis/tasks/09-25-desktop-window-settings. Own parent R2-R6 and the settings portion of R7.
- Own desktop preference persistence, IPC, settings UI, appearance application in main/HUD, Planet controls, and Status/HUD preference consumers.
- No product dependency on the window child for store or page work. Final appearance acceptance consumes that child's custom titlebar.
- Implement after the window child in the planned sequence because both change AppShell, lifecycle integration, and desktop specs. Parent-child tree order alone is not a dependency.
- Existing operation-performance and native-acceptance tasks retain their current scope and findings.

## Confirmed Decision

The user confirmed dark, light, and system themes across all main pages and the HUD, with dark as default. Source: the theme-scope reply in the current task conversation. The current dark-only constraint is intentionally superseded.

## Requirements

- S1: Expand the existing settings route into a dedicated page. Rename the brand-menu destination from Language to Settings. Preserve deep links, back navigation, keyboard focus, and existing language behavior.
- S2: Support the three confirmed themes. System mode follows OS changes while open. Apply complete semantic palettes to all five modes, support pages, dialogs, errors, window controls, and HUD.
- S3: Provide bounded local font presets and text scale with Chinese/English previews and fallbacks. Scale actual labels, headings, tables, and controls, while retaining separate main/HUD base sizes and readable technical text.
- S4: Provide motion reduction, Planet frame cap, Status live interval, Status returned process rows, and HUD interval. The configuration matrix in design.md owns values, defaults, and apply timing. Do not expose a control without its runtime consumer.
- S5: Keep one committed preference source. Persist desktop preferences independently from the exact shared language V1 file. Preserve unreadable/future bytes, report save errors, and apply saved values only after successful commit.
- S6: Keep Settings and the Status interval control synchronized. Changes that replace active Status work must cancel and join before restarting. HUD sampling remains stopped while hidden.
- S7: Provide separate restore-defaults actions for appearance and performance, with visible save/apply state. Reset does not alter language, Protection, audit history, mode results, or cleanup authority.

## Acceptance Criteria

- [ ] S-AC1 (S1): The brand menu and #/settings open the same page; back navigation and keyboard focus remain correct in both locales.
- [ ] S-AC2 (S2): Dark, light, and system themes cover every main destination and HUD. Changing the OS theme affects only system mode. A HUD opened after a settings change loads the committed choice.
- [ ] S-AC3 (S2, S3): All font presets and text scales apply to actual labels, tables, headings, and controls in both windows. Chinese, English, numeric values, errors, and confirmation actions remain readable under the existing viewport/forced-colors checks.
- [ ] S-AC4 (S4): Motion reduction stops decorative animation; the Planet obeys 15/30 FPS caps while preserving the 96-second turn and source-resolution cap. Hidden/unmounted surfaces own no decorative frame loop.
- [ ] S-AC5 (S4, S6): Status snapshot and live calls receive the selected returned-row limit. The live interval matches both settings surfaces. A live interval change commits, cancels, joins, and then replaces the run without stale completion.
- [ ] S-AC6 (S4, S6): Each supported HUD interval reaches the Rust sampler on the next show. Hidden HUD sampling remains stopped; opening and hiding repeatedly leaves at most one sampler and joins it on hide.
- [ ] S-AC7 (S5): Valid values survive restart. Missing files use defaults. Invalid fields, bounds, malformed/future documents, failed writes, and concurrent field updates preserve existing bytes and the last committed view as specified in design.md.
- [ ] S-AC8 (S5): Existing presentation-v1.json bytes and TUI locale readers stay compatible. Desktop preferences do not change CLI machine output or duplicate the persisted language value.
- [ ] S-AC9 (S5, S6): Main/HUD initialization, committed notifications, late responses, and unmount preserve snapshot ordering and remove listeners. HUD can read preferences but cannot update them or invoke domain operations.
- [ ] S-AC10 (S7): Group resets restore only their documented defaults. Save failure remains visible and does not report success or overwrite unreadable bytes.
- [ ] S-AC11 (S1-S7): Relevant contract, component, lifecycle, persistence, and quality gates pass. Focused native evidence is labelled separately from fixture evidence; existing performance failures are not reclassified.

## Out Of Scope

Custom font imports/downloads, OS font enumeration, language resolver redesign, live tray-language redesign, cloud sync, config import/export, hotkey remapping, close-to-tray, autostart, automatic Status pause/resume on minimization, CPU/RAM quotas, arbitrary worker counts, configurable Clean/Analyze safety budgets, and changes to cleanup authorization.

## Evidence

The evidence and current defaults are recorded in ../09-25-desktop-window-settings/research/settings-runtime.md. The new values are explicit implementation proposals; existing defaults remain unchanged. No unresolved product question blocks final plan review.
