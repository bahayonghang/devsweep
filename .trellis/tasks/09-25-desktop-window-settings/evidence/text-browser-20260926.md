# Text Browser Acceptance: 2026-09-26

## Method

The approved work resumed from Codex thread
`01a0dbdb-4978-78d1-8ddf-00265e4785a0`. The user explicitly excluded image
features and screenshot verification. All observations below use DOM values,
computed CSS values, accessibility text, and keyboard actions. No screenshot,
image capture, image generation, or image inspection ran in this session.

The existing fixture server served the current repository at
`http://127.0.0.1:4180`. The Codex in-app browser opened dedicated temporary
main and HUD tabs. The fixture stores are in-memory and separate per entry.
These results do not establish disk persistence or native cross-window events.

An attempted PowerShell launch of headless Chrome was rejected by automatic
approval review with `blocked by policy`. The browser checks then used the
provided browser interface. The temporary viewport override was reset and
both temporary tabs were closed after the checks.

## Observed Results

| Check | Cases | Result |
| --- | ---: | --- |
| Settings: en/zh-CN, dark/light/system, 100/125 percent text, 390/800/1024/1440 CSS px | 48 | No document horizontal overflow, alert, raw translation key, or unnamed select |
| Nine routes: en/zh-CN, dark/light/system, 125 percent text, same four widths | 216 | No document horizontal overflow, alert, raw translation key, undefined text, or NaN text |
| All three fonts and 100/110/125 percent text, both locales | 18 | The configured font stack and actual body/select font size changed |
| Appearance reset | 1 | Restored dark/system-font/100 percent; retained interval 10, rows 50, HUD interval 5, and zh-CN |
| Performance reset | 1 | Restored system motion, 30 FPS, interval 2, rows 15, HUD interval 2; retained light theme and zh-CN |
| Keyboard Settings/back navigation | 2 locales | Enter opened Settings; Enter on Back returned to Status and focused the Status tab |
| HUD fixture at 280 by 360 CSS px | 1 | Dark palette, 13 px base text, no document horizontal overflow or alert |
| Main browser console errors | 1 read after matrix | No error entries |

The nine routes were `clean`, `software`, `optimize`, `analyze`, `status`,
`protection`, `rules`, `history`, and `settings`. Navigation used the actual
tabs and brand menu. Settings changes remained in effect during navigation.
The initial `#/settings` deep link and the menu opened the same Settings page.

The font results were identical in both locales:

| Preset | Computed body font-family |
| --- | --- |
| system | Segoe UI Variable, system-ui, Microsoft YaHei UI, sans-serif |
| segoe_ui | Segoe UI, Microsoft YaHei UI, system-ui, sans-serif |
| microsoft_yahei_ui | Microsoft YaHei UI, Segoe UI, system-ui, sans-serif |

| Text scale | Root font size | Main body and select font size |
| --- | --- | --- |
| 100 percent | 16 px | 14 px |
| 110 percent | Not recorded | 15.4 px |
| 125 percent | 20 px | 17.5 px |

Dark resolved to dark, light resolved to light, and system resolved to the
browser's current dark preference. The browser reported reduced motion; the
Planet frame-rate control was disabled with the localized reason.

## Evidence Boundaries

- DOM overflow checks establish document geometry. They do not establish
  glyph rendering, all component clipping, native DPI, or image appearance.
- Main/HUD synchronization, persisted restart behavior, failed writes,
  stale snapshots, actual sampler cadence, and renderer scheduling rely on
  the separately identified component/Rust test evidence. The separate HUD
  fixture cannot establish those native behaviors.
- Live OS theme changes, forced-colors rendering, and native drag, physical
  double-click, border resize, taskbar restore, Win+Arrow, Alt+F4, tray/HUD
  interactions, and native scale remain unverified in this browser session.
- Native CUA APIs are disabled. Native UI actions were not replaced by
  unsupported desktop automation. No native acceptance row is marked passed
  from fixture evidence.
- No Cleanup Target execution, software removal, optimization, real user
  preference write, OS theme change, or OS scale change ran.

## Validation Scripts

The existing `verify/cdp.mjs`, `verify/ui-matrix.mjs`, and
`verify/native-launch.mjs` no longer contain screenshot operations. Their
syntax checks passed. The unused `about` route was removed from the matrix
and the shipped `rules` route was included. Those scripts were not used to
claim the browser results above; the supported browser interface produced
the observations.
