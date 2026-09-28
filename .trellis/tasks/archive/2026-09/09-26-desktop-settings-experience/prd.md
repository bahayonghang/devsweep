# Desktop Settings Experience

## Goal

Make Settings directly reachable and let the user select readable controls, installed host fonts, and coherent application palettes. The four requested outcomes are a Settings tab after Status, improved dropdowns, all available host font families, and Catppuccin Latte/Mocha plus Codex/Claude themes.

## Authorization and status

The user approved the documented plan and both production dependencies on 2026-09-26, then requested sequential implementation. All four children are authorized in their recorded order. See research/approval.md. No product or dependency decision remains open.

## Evidence

The screenshot and repository baseline fea6368 establish the reported interface. research/repository-findings.md E1-E14 records file/line evidence, current owners, and the changes required. research/design-proposal.md records the layout, typography, palette proposals, and skill-based design review. research/external-references.md records primary technical sources.

The existing implementation has five operational tabs, a separate settings route in the brand menu, nine native selection controls, three font presets, and dark/light/system themes. Saved desktop preferences use a strict V1 schema and committed snapshots shared by main and HUD. The supported desktop MVP is Windows.

## Requirements

- R1 Navigation: Settings is an independent visible tab immediately after Status. The existing five operational modes keep their identity. Settings participates in keyboard navigation, selected state, focus restoration, panel semantics, deep links, and history. Remove its displaced brand-menu entry and back-to-mode header control.
- R2 Controls and layout: Apply frontend-design guidance to Settings. Provide coherent popup surfaces, selected/highlight/disabled states, readable labels, aligned sections, and responsive layout. Fonts have search; themes have visual previews. Preserve existing setting values, apply timing, reset boundaries, and independent language save behavior.
- R3 Host fonts: Offer all locally installed font families exposed to DevSweep by the current Windows host, without the existing preset ceiling. Include family/localized-name search, explicit choice, preview, refresh, system default, and missing-font fallback. The choice applies to actual main/HUD UI and survives restart.
- R4 Palettes: Add Catppuccin Latte and Mocha with source attribution, and Codex/Claude inspired presets. Preserve DevSweep dark, DevSweep light, and System with dark as default. Apply each theme across main pages, titlebar, popup portals, support pages, dialogs, and HUD. Preserve risk semantics, accessibility overrides, and readable contrast.
- R5 Compatibility and behavior: Preserve valid existing preference values and language bytes. Save only explicit choices, and apply only committed values. Preserve main/HUD sequence ordering, read-only HUD capability, operation drain, sampling behavior, and all cleanup authority boundaries. Reject unsupported documents without overwriting them.

## Acceptance criteria and ownership

| ID | Requirement | Observable acceptance | Child evidence |
| --- | --- | --- | --- |
| AC1 | R1 | Settings follows Status in both locales; exactly one primary tab is selected and its panel is associated correctly. | N-AC1 |
| AC2 | R1, R5 | Full keyboard navigation, direct settings hash, history, focus, small-width scroll, drain failure, and stale-transition cases work. Operational-mode state remains intact. | N-AC2 to N-AC5 |
| AC3 | R2 | Settings controls have consistent themed open/closed states and accessible names/values. Popup collision, dismissal, selection, focus, and error recovery work. | C-AC1, C-AC2, C-AC4 |
| AC4 | R2, R3 | Both locales, Chinese IME, long names, 390/800/1024/1440 widths, all existing text scales, forced colors, and reduced motion remain usable. | C-AC3, C-AC5; F-AC2 |
| AC5 | R3 | Native catalogue evidence proves discovery beyond the three old presets; all returned families remain searchable/reachable and duplicate faces collapse to families. | F-AC1, F-AC2 |
| AC6 | R3, R5 | Font selection persists and reaches main/HUD. Missing fonts fall back without erasing the saved name; names cannot inject CSS; font discovery failure is localized. | F-AC3, F-AC7 |
| AC7 | R4 | Seven theme choices are available; requested previews and actual application surfaces match the selected palette. | T-AC1, T-AC2 |
| AC8 | R4 | Text/control contrast reaches the stated thresholds; selected/risk states have non-color signals. Forced colors and reduced motion override every palette. | T-AC4, T-AC5 |
| AC9 | R4, R5 | OS scheme changes affect only System. Theme changes persist, reach main/HUD, use a valid CSS scheme, and preserve prior theme on failed save. | T-AC3 |
| AC10 | R5 | All V1 font presets and unrelated fields migrate correctly. Read alone does not write. V2 first save preserves V1/language bytes, and downgrade behavior is documented. | F-AC4 |
| AC11 | R5 | Invalid/future/unreadable bytes, failed replacements, concurrent first saves, stale snapshots, and resets retain the documented integrity boundaries. | F-AC5, F-AC6 |
| AC12 | R1-R5 | One integrated Settings navigation -> font selection -> named theme -> HUD/restart flow passes after all children; tests and native evidence are distinguished. Existing performance/acceptance tasks keep their own findings. | Parent integration report |

Text contrast means at least 4.5:1 for ordinary text. Large text and essential control/focus boundaries require at least 3:1 in their actual context. These are implementation acceptance requirements, not measured results from this turn.

## Task map and order

1. 09-26-desktop-settings-navigation: R1.
2. 09-26-desktop-settings-controls: R2 and shared font/theme controls.
3. 09-26-desktop-settings-host-fonts: R3 and R5 preference migration/native boundaries.
4. 09-26-desktop-settings-palettes: R4 and final theme contract/coverage.

The parent owns cross-child R5 and AC12. Children run in this order because shared files and V2 dependencies overlap. Each child has its own PRD, design, execution plan, and spec/research manifests. The task tree does not authorize parallel edits to shared files or partial-schema releases.

## Scope choices for review

Host means the machine running DevSweep. The plan covers installed families on each supported Windows host. It does not introduce a remote-host model. Codex uses the proposed neutral dark palette; Claude uses the proposed warm light palette. Both are DevSweep-authored inspired designs with no claim of exact official UI colors. The exact proposal is in research/design-proposal.md. Existing default theme and text/performance options remain unchanged.

## Out of scope

Remote font browsing or synchronization; font installation, file import, or download; cloud font access; new macOS/Linux desktop support; font-weight/style customization; arbitrary theme editing or imports; extra Catppuccin variants; new performance controls; other-page control redesign; domain/CLI/TUI behavior changes; global CSS refactoring; and closure of the separate 09-20 operation-performance or native-acceptance tasks.

## Confirmed dependency decision

The user approved @base-ui/react for accessible Select/Combobox and a Windows-only direct windows crate dependency for DirectWrite font enumeration. Use scoped imports/features and reviewed lockfiles. Additional production dependencies require separate approval.

Approval source: the user reply on 2026-09-26, "同意，请按照顺序开始实施任务". The reply also authorizes execution of the reviewed plan.
