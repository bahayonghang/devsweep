# PureMac visual and interaction audit (2026-09-13)

Research only. `ref/repo/PureMac` is gitignored (`.gitignore:35`). No PureMac
source file, string, screenshot, glyph, or hex value is a production source.

## Sources

| Source                                                                                                                     | Role                                                                                                                                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ref/repo/PureMac` clone, commit `acdd38d` (2026-09-13), origin `https://github.com/momenbasel/PureMac.git`, MIT `LICENSE` | Reference repository                                                                                                                                                                                                    |
| `PureMac/Views/MainWindow.swift:33-39,515-569`                                                                             | `NavigationSplitView`, sidebar column 232 / 244 / 300, window min 980x600, `SidebarNavRow` (icon tile 24, label 12.5 semibold when selected, badge 9.75 semibold, row min height 36)                                    |
| `PureMac/Views/MainWindow.swift:100-115`                                                                                   | Sidebar sections: Overview, Cleanup, Applications, Advanced Tools                                                                                                                                                       |
| `PureMac/PureMacApp.swift:104-118`                                                                                         | Default window 1000x680, min 900x600                                                                                                                                                                                    |
| `PureMac/Views/Components/AppTheme.swift:64-117`                                                                           | `Tint` palette, `MotionTokens`, `TintGradient`                                                                                                                                                                          |
| `PureMac/Views/Components/AppTheme.swift:117-160`                                                                          | `IconTile`: rounded tile, gradient fill, hairline, glow shadow, symbol at 52% of size                                                                                                                                   |
| `PureMac/Views/Components/AppTheme.swift:190-238`                                                                          | `CardSurface`: radius 14, padding 16, hairline stroke, tint wash under 7% opacity, optional material                                                                                                                    |
| `PureMac/Views/Components/AppTheme.swift:267-305`                                                                          | `StatusChip` (capsule, 11 semibold, 14% tint fill, dot), `SectionHeader`                                                                                                                                                |
| `PureMac/Views/Components/AppTheme.swift:343-394`                                                                          | `GlowProminentButtonStyle`, `AnimatedCheckboxStyle`                                                                                                                                                                     |
| `PureMac/Views/DashboardView.swift:236-300`                                                                                | Smart Care stage: eyebrow `N-POINT LOCAL SCAN` (10.5 bold, tracking 0.7), headline `Scan first. Decide what goes.` (36 bold, tracking -1), lede 13.5, pill CTA `Scan My Mac`, trust labels Local / Private / Reviewable |
| `PureMac/Views/DashboardView.swift:47-53,994-1110`                                                                         | `Care areas` section, `SmartCareStageSurface`, `CareModuleCard` (icon tile, title, value, detail, toggle)                                                                                                               |
| `PureMac/Views/DashboardView.swift:1191-1300`                                                                              | `FindingTile`, `%lld selected`, `StatCard`                                                                                                                                                                              |
| `PureMac/Views/DashboardView.swift:1350-1564`                                                                              | `SuggestionRow`, `ScanPathTicker`, `HoverableLegendChip`, `HeroDrift`, `OrbSatellites`, `ShimmerProgressBar`, `CategoryToggleRow`                                                                                       |
| `PureMac/Views/Components/DashboardCharts.swift:148,303,409,422`                                                           | `StorageDonut`, `StackedMeter` (solid segments, 2px separators, spring reveal, legend cross-highlight), `staggered`, `CountUpBytes`                                                                                     |
| `PureMac/Views/CategoryDetailView.swift:88-160,240-300`                                                                    | `heroCard`, `selectionStrip` (`%lld of %lld selected`, select all), `FileRowView` (toggle, 20pt icon, middle-truncated name, muted path, date, right size)                                                              |
| `PureMac/Views/Components/EmptyStateView.swift`                                                                            | Symbol, title, description, optional action                                                                                                                                                                             |
| `PureMac/Models/Models.swift:6-72`                                                                                         | `CleaningCategory` raw names, SF Symbol icons, colors, descriptions                                                                                                                                                     |
| `PureMac/zh-Hans.lproj/Localizable.strings`                                                                                | zh-Hans copy. Rejected as a copy source                                                                                                                                                                                 |
| `screenshots/{smart-care,scanning,breakdown,system-junk,user-cache,xcode-junk,app-uninstaller,onboarding}.png`             | Visual topology reference. Never copied into the repository                                                                                                                                                             |
| User Image #1 (2026-09-13)                                                                                                 | Current DevSweep Clean home: dark green canvas, centered capsule, More button, 168px ring hero, `Run a scan to review cleanup targets.`, bottom toolbar with Projects / Global caches / Ready / Scan, 800x600 window    |

## License and provenance boundary

- PureMac is MIT. MIT permits reuse with attribution. DevSweep policy in
  `docs/provenance.md` is stricter: no copied source, fixtures, user-facing
  copy, or assets from any third-party cleanup tool. This task takes layout
  topology and interaction ideas only. Every string, glyph, hex value,
  component, and test is written from DevSweep contracts.
- SF Symbols are Apple-licensed and are not available to a Windows WebView.
  DevSweep draws original inline SVG glyphs.
- PureMac gradients, glow shadows, materials, and the hero orb conflict with
  `.trellis/spec/desktop-frontend/component-guidelines.md` Visual System
  (`linear-gradient`, `radial-gradient`, `backdrop-filter`, glow, and glass
  are prohibited) and with `desktop/src/styles.test.ts:18-37`. They are
  rejected, not adapted.
- `docs/provenance.md` must record PureMac as a reviewed reference with
  nothing copied.

## PureMac topology

Shell:

- Left sidebar, 232 to 300 px, sectioned: Overview (Smart Care), Cleanup
  (categories), Applications, Advanced Tools. Each row is an icon tile plus
  label plus optional size badge. Selected row uses the tint.
- Sidebar footer carries a system-status surface.
- Content column: page header (24pt bold title, subtitle, `StatusChip`), then
  cards on a flat dark or light surface.
- Window min 900x600 (app) and 980x600 (main window). Default 1000x680.

Smart Care first screen:

- Stage surface with eyebrow, two-line headline, lede, pill CTA with symbol,
  trust labels, and a storage plaque with a `StackedMeter`.
- `Care areas` grid of `CareModuleCard` (tile, title, value, detail, toggle).

Scanning:

- Stage stays. Headline changes. `ShimmerProgressBar`, `ScanPathTicker`, and a
  `Found so far` list of category rows with live sizes.

Completed:

- `CountUpBytes` total, `FindingTile` grid, `By category` bar chart with
  legend chips, category checklist rows, `%lld selected`, `Review` action.

Category detail:

- Hero card, selection strip (`%lld of %lld selected`, select all), file rows
  (toggle, icon, name, muted path, date, right-aligned size).

Empty state: symbol, title, description, one action.

## Gap versus current DevSweep desktop

| PureMac pattern                                                   | DevSweep today                                                                                      | Disposition                                                                                                                                |
| ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Sectioned left sidebar with icon tiles                            | Centered capsule `role=tablist` plus More disclosure (`desktop/src/app-shell/AppShell.tsx:300-372`) | Adapt: sidebar `role=tablist` vertical; supporting destinations as a named section; Language and Help in the footer; top strip below 800px |
| Page header title, subtitle, status chip                          | `h1.sr-only` (`AppShell.tsx:376`), no subtitle                                                      | Adapt: visible title, additive subtitle keys, mode-owned status chip                                                                       |
| Stage hero with headline, lede, CTA, trust labels, storage plaque | 168px ring, hint sentence, bottom toolbar (`desktop/src/pages/ScanPage.tsx:83-107`)                 | Adapt: original copy, scope checkboxes replace trust labels, plaque shows Ready / Scan Progress / Estimated Recoverable                    |
| `Found so far` live list                                          | `ScanPreviewPage.tsx` scope groups with ecosystem counts                                            | Adapt presentation; keep heading semantics `Projects 1`                                                                                    |
| `By category` bar and category rows                               | `ReviewPage.tsx` groups by `kind` then `scope` with a 6-column table                                | Adapt: category rows with kind glyph, count, subtotal, then file rows; single select-all stays                                             |
| `FileRowView`                                                     | `TargetTable.tsx:52-57` path strong, `ecosystem · scope`, raw `kind`, raw `risk`                    | Adapt: keep `<table>`, restyle rows; add bilingual kind and risk labels                                                                    |
| `CountUpBytes` result                                             | Result copy from `clean.v1.trash.moved`                                                             | Reject count-up animation; keep large truthful number, still under reduced motion                                                          |
| `StackedMeter`                                                    | None                                                                                                | Adapt with solid segments from verified plus partial lower bound only                                                                      |
| `IconTile` gradient and glow                                      | None                                                                                                | Reject gradient and glow; adapt as flat tinted tile at 16% opacity                                                                         |
| `CardSurface` radius 14, material                                 | Radii 8px cap, no glass                                                                             | Adapt: card radius 12, controls 8, no material                                                                                             |
| `GlowProminentButtonStyle`                                        | `.primary-button` pill                                                                              | Reject glow; keep pill                                                                                                                     |
| Sidebar size badges                                               | None                                                                                                | Defer: shell never invents counts; needs lifted mode state                                                                                 |
| `HeroDrift`, `OrbSatellites`, `ShimmerProgressBar`                | Ring hero, indeterminate progressbar                                                                | Reject drift and shimmer; keep indeterminate Scan Progress without percentage                                                              |
| Smart Care aggregate scan of 12 categories                        | Projects and Global caches scopes                                                                   | Keep DevSweep scopes; no new scan providers                                                                                                |
| Uninstaller list with leftovers                                   | Software inventory rows and eligibility                                                             | Adapt row chrome only; eligibility copy unchanged                                                                                          |
| Onboarding screen                                                 | None                                                                                                | Reject in this task                                                                                                                        |

## Accept / adapt / reject

Accept as ideas:

- Persistent sidebar with visible names; every destination reachable in one
  click.
- Page header that names the page and its state.
- Stage hero that states the review-first promise before the first action.
- Live `found so far` feedback during a scan.
- Category rows then file rows, with the selection summary always visible.
- Card surfaces with hairline borders and one type scale.
- Reduced motion and keyboard as first-class rules.

Adapt with original DevSweep tokens, copy, and glyphs:

- Sidebar navigation on Windows chrome with `role=tablist`, vertical arrow
  keys, existing Alt accelerators, and a top strip below 800px.
- Page header with additive `shell.v1.subtitle.*` copy.
- Clean stage hero, capacity plaque, `Found so far`, category rows, file rows.
- Software, Optimize, Analyze, Status, Protection, Rules, History on the same
  card chrome.
- Window default 1080x720 with a minimum size.

Reject:

- PureMac name, logo, SF Symbols, strings in any language, screenshots,
  fixtures, Swift code, hex values.
- Gradients, glow shadows, materials, orb hero, shimmer, count-up animation.
- Sidebar size badges without lifted state.
- Smart Care aggregate category catalogue, onboarding, Applications uninstall
  flow changes, `Advanced Tools`.
- Permanent delete copy such as `This will permanently delete`. DevSweep
  keeps trash-only language.
