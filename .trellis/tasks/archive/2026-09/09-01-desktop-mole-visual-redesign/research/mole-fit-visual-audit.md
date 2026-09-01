# mole.fit visual and interaction audit (2026-09-01)

Research only. No Mole asset, copy, or geometry is a production source.

## Sources

| Source | Role |
| --- | --- |
| https://mole.fit/zh/ | Requested Chinese marketing surface |
| https://mole.fit/index.md | Agent-readable product index |
| https://mole.fit/blog/the-design-of-mole.md | Design philosophy |
| https://mole.fit/mac-cleaner.md | Clean review-first rules |
| https://mole.fit/img/ch/clean-1400w.webp | Clean home / result |
| https://mole.fit/img/ch/uninstall-1400w.webp | Software list + leftovers |
| https://mole.fit/img/ch/optimize-1400w.webp | Optimize running state |
| https://mole.fit/img/ch/analyze-1400w.webp | Analyze list + treemap |
| https://mole.fit/img/ch/status-1400w.webp | Status bento + processes |
| https://mole.fit/img/ch/menubar-1400w.webp | Menu-bar HUD (out of desktop window) |
| User Image #1 | Current DevSweep Clean empty state |
| User Image #2 | Mole for Mac Clean home (same topology as marketing clean shot) |

Fetched 2026-09-01. HTML `Last-Modified: Tue, 01 Sep 2026 06:37:52 GMT`. Marketing theme-color is cream `#f5f4ed`; the **app** is dark-only.

## Philosophy (adapt, do not quote in UI)

From `the-design-of-mole.md`:

- Quiet is an engineering constraint: a panel is fully rendered or hidden;
  a scan completes before the result state; short waits show nothing;
  longer waits show one calm working state; containers reserve space so
  the window does not jump.
- Review before action is the visible half of safety. Independent path
  validation is the invisible half. A warning screen alone is not a
  boundary. This already matches DevSweep's Scan Report -> dry-run digest
  -> confirmation -> execute split.
- Five planets exist so each job has a character, and so the tool is
  comfortable to sit in front of. The discipline is one system, not five
  ornaments.
- Dark only. One type scale. Shared tokens. Reduced motion stops
  decorative spin; no operation depends on watching a planet.
- Restraint includes saying no to extra Mac tricks.

From `mac-cleaner.md`:

- Named candidates with paths and sizes, not one unexplained total.
- Sort by restore cost: regenerable, expensive to rebuild, irreplaceable.
- Running-app caches skipped. Protected paths rechecked at execute time.
- Use it, then let it disappear. No scare banners. No always-on cleaner.

## Visual topology of the Mac app

Shared chrome:

- Native macOS traffic lights (reject for DevSweep; keep Windows chrome).
- Centered floating capsule: brand mark + five mode labels. Active tab is
  a white pill; inactive labels are muted on a translucent track.
- No left brand lockup, no right support cluster in the primary chrome.
- Full-window dark canvas. Content is the world, not a light pane under a
  dark header.

Per-mode worlds (page-wide tint):

| Mode | World | First screen |
| --- | --- | --- |
| Clean | Deep navy | Sphere + large estimated total + one capsule action |
| Software | Maroon | Grouped app rows, leftover disclosure, sticky remove bar |
| Optimize | Olive / mercury | Sphere + running checklist under a single status line |
| Analyze | Brown / jupiter | Sphere thumbnail + directory list + treemap |
| Status | Gold-green / sun | Bento metric cards, sparklines, process table |

Hero numbers:

- Clean result uses a large binary-looking total (`86.4 GB`) plus a
  secondary metaphor line (4K minutes, cumulative cleaned).
- DevSweep must not claim space already freed. Any hero number is
  Estimated Recoverable or a completed-scan total with confidence.

## Gap versus current DevSweep desktop

| Mole pattern | DevSweep today | Likely disposition |
| --- | --- | --- |
| Centered capsule nav | Three-column header | Adapt if immersive world is chosen |
| Per-mode dark world | Dark header + light `#eef1ef` workbench | Adapt (spec currently forbids) |
| Clean globe hero | "No scan results" empty heading | Adapt with an **original** motif |
| One primary action on home | Scan button in a toolbar next to checkboxes | Adapt |
| Category cards by restore cost | Flat Cleanup Target table | Later functionality decision |
| Software leftover expansion | Inventory rows with details | Adapt presentation only |
| Optimize planet + checklist | Catalogue list + empty heading | Adapt presentation of existing catalogue |
| Analyze planet + treemap | List + treemap, light theme | Adapt chrome; keep read-only |
| Status bento + health 100 | White cards; health score forbidden | Adapt cards; **reject** health score |
| Menu-bar HUD | None | Reject in this task |
| Traffic lights, hamster, NASA Earth | N/A | Reject |

## Accept / adapt / reject (planning baseline)

Accept as ideas:

- Five modes, one entrance.
- Review-first, then a bounded plan.
- Quiet operation states and reserved layout.
- Dark-only desktop canvas.
- Reduced motion as a first-class rule.
- Sticky selection summary that never hides authority.

Adapt after visual-world approval:

- Centered capsule navigation on Windows chrome.
- Mode-tinted dark worlds using original DevSweep tokens.
- Clean / Optimize first screens with an original mark and a large
  Estimated Recoverable or progress line.
- Software dense rows and Analyze/Status spatial density.

Reject:

- Mole name, logo, hamster, copy, GPL source, fixtures.
- NASA Blue Marble / any photographic planet as a shipped asset.
- Fake macOS traffic lights.
- Pixel-identical geometry.
- Synthetic health score, tray HUD, fan control, app-update tab,
  startup-item manager, scare "space freed" heroes.

User decisions 2026-09-01:

- Immersive recreation for the visual contract.
- This task restructures existing five-mode interaction. New Mole
  capabilities (updates, startup items, tray HUD, health score, leftover
  matching) go to a later parent.
- Hero is the truthful number plus an original CSS sweep body. No
  celestial metaphor.
- Clean Projects / Global caches stay visible on the first screen.
