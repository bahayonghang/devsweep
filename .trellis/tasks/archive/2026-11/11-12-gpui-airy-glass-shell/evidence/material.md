# Window material: measurements and decision (step 7, AC4, AC5)

Measured 2026-11-12 and 2026-11-13 on Windows 10.0.26300, 120 dpi (125%
display scale), Windows transparency effects on (`EnableTransparency=1`),
normal contrast. Release binary `devsweep-gpui-spike.exe`.

## Method

`evidence/capture.ps1` launches the binary with an isolated `LOCALAPPDATA`,
places a borderless form of the same bounds directly behind the window, fills
it black and then white, and reads screen pixels with `GetPixel` on the screen
DC (the composited result, not `PrintWindow`). A sample is discarded as
`occluded` if another window owns the point.

Sample points, in logical px from the window's top-left corner:

| Name | Point | What it is |
| --- | --- | --- |
| field | (10, 400) | left margin outside the paper, mint end of the gradient |
| field-end | (width - 60, height - 16) | bottom margin, lavender end |
| paper | (40, 400) | paper padding, left of every well (default width only) |

The log prints `GetPixel` values as `BBGGRR`. The tables below give `#RRGGBB`.
Luminance is WCAG 2.x relative luminance. Contrast ratios use the sampled
composite as the background and the token value as the foreground.

Raw logs: `after/capture.txt` (before the tint), `final/capture-probe1.txt`
(first tint attempt), `final/capture-probe2.txt` (as built),
`final/capture-nocomp-probe.txt`, `final/capture-w900-probe.txt`, and
`final/capture.txt` (the full matrix on the final binary).

## Field luminance, light theme

| Build | Backdrop tint | Field alpha | field / black | field-end / black | field / white | field-end / white |
| --- | --- | --- | --- | --- | --- | --- |
| Step 5 (`after/`) | transparent black | 0.77 | `#B0B9B8` 0.4739 | not sampled | `#DCE5E4` 0.7686 | not sampled |
| Probe 1 | field colour | intermediate | `#D9E3E2` 0.7518 | not sampled | `#E4EFED` 0.8434 | not sampled |
| Probe 2, as built | `#E7F6F1` at 0x99 | 0.90 | `#DBE7E5` 0.7787 | `#E1DBEC` 0.7273 | `#E3EEEC` 0.8354 | `#E8E2F3` 0.7802 |
| Opaque fallback (nocomp) | none | 1.0 | `#E3EFEE` 0.8424 | `#E8E3F4` 0.7863 | same | same |

Probe 1's exact field alpha was not recorded by the interrupted run; only its
samples are known.

## Decision

Rule (`desktop-gpui/DESIGN.md`, "Material without backdrop-filter"): if the
light field falls below 0.70 relative luminance over black, paint the field
opaque.

As built, the worst light sample over black is the lavender end at 0.7273.
Both ends pass, so the **tinted backdrop is kept**: acrylic tint = field start
colour at alpha 0x99 (`ui::BACKDROP_TINT_ALPHA`), field gradient at 0.90
(`ui::FIELD_GLASS_ALPHA`). The margin over the threshold is 0.027 at the
lavender end at 100% text scale and 0.018 at 125% (0.7183, see "Final
round"; the fixed sample point lands on a different part of the gradient). A different Windows build or acrylic implementation could move
it; re-run `capture.ps1` when either constant or the vendored blur call
changes.

## Dark theme

| Build | field / black | field-end / black | field / white | field-end / white | paper / black | paper / white |
| --- | --- | --- | --- | --- | --- | --- |
| Step 5 (`after/`) | `#131E20` 0.0117 | not sampled | `#3F4A4C` 0.0648 | not sampled | not sampled | not sampled |
| Probe 2, as built | `#162324` 0.0150 | `#222232` 0.0171 | `#1D2A2B` 0.0209 | `#292939` 0.0235 | `#1F282E` 0.0201 | `#20292F` 0.0210 |

The dark field over white moved from 0.0648 to 0.0209, so a bright desktop no
longer lifts the dark field. The nominal field start `#142322` is 0.0147.

## Contrast on the sampled paper (AC4)

Background is the composited paper pixel from probe 2. Gates: text 4.5:1,
focus ring and required boundaries 3:1.

| Theme | Background | Ink | Secondary text | Focus ring (`accent`) | Control edge (`border_strong`) |
| --- | --- | --- | --- | --- | --- |
| Light | paper over black `#F7F9F9` | 12.75 | 6.25 | 5.02 | 3.57 |
| Light | paper over white `#F9FBFB` | 12.97 | 6.36 | 5.11 | 3.63 |
| Light | opaque paper `#FFFFFF` (nocomp sample) | 13.47 | 6.61 | 5.31 | 3.77 |
| Dark | paper over black `#1F282E` | 13.43 | 8.13 | 9.04 | 5.22 |
| Dark | paper over white `#20292F` | 13.26 | 8.02 | 8.92 | 5.15 |

Foregrounds: light ink `#24303A`, secondary `#505F68`, accent `#187A56`,
edge `#78877F`; dark ink `#EDF4F2`, secondary `#B3C2BF`, accent `#87D9BA`,
edge `#859E94`.

Wells are opaque, so the desktop cannot change them. Token values:

| Theme | Fill | Ink | Secondary text | Focus ring | Control edge |
| --- | --- | --- | --- | --- | --- |
| Light | well `#F4F8F6` | 12.58 | 6.17 | 4.95 | 3.52 |
| Light | hovered / inset `#EAF2EE` | 11.83 | 5.80 | 4.66 | 3.31 |
| Light | selected `#DCEDE5` | 11.09 | 5.44 | 4.37 | 3.10 |
| Dark | well `#252F36` | 12.23 | 7.40 | 8.23 | 4.75 |
| Dark | hovered / inset `#29353A` | 11.31 | 6.84 | 7.61 | 4.39 |
| Dark | selected `#30443E` | 9.31 | 5.63 | 6.26 | 3.62 |

Every cell meets its gate. The lowest is the light control edge on the
selected fill, 3.10 against a 3:1 gate. The unit test
`text_and_control_edges_meet_contrast_on_paper_and_wells` asserts the same
pairs from the tokens.

## White well on white paper (glass off)

With glass off the light paper is `#FFFFFF`. In the probe builds the light
well (`surface`) was also `#FFFFFF`, so a borderless well had no edge. The
light fills are now a neutral step sequence:

| Step | Value | Ratio to the step above |
| --- | --- | --- |
| paper | `#FFFFFF` | |
| well | `#F4F8F6` | 1.07 |
| hovered row, input, tile | `#EAF2EE` | 1.06 |
| selected | `#DCEDE5` | 1.07 |

The dark group already stepped (paper `#20292F`, well `#252F36`, 1.08). The
contract records the light steps as `well`, `well-inset` and `well-selected`,
and `wells_are_a_fill_step_away_from_the_paper` guards them. A 1.07 step is a
visible tonal change, not a 3:1 boundary: wells are containers, and the
controls inside them carry their own fill or `border_strong` edge. See
"Final round" for the capture that confirms it.

## Fallbacks (AC5)

| Case | How it was driven | Result |
| --- | --- | --- |
| Direct composition off | `GPUI_DISABLE_DIRECT_COMPOSITION=1`, `capture.ps1 -NoComposition` | Field and paper read the same over black and over white: field `#E3EFEE` 0.8424, field-end `#E8E3F4` 0.7863, paper `#FFFFFF` 1.0. Opaque. All five tabs selectable by UIA. |
| Width below 960 | `capture.ps1 -LogicalWidth 900` | Field reads the same over black and over white: `#E3ECEE` 0.8249, field-end `#E6E1F2` 0.7708. Opaque. All five tabs selectable. The paper point is not sampled at this width. |
| Windows transparency effects off | not driven | Needs the Windows setting "Transparency effects" turned off, a global change this run does not make. Manual step below. |

The w900 field sample differs slightly from the nocomp one because the fixed
sample point lands on a different part of the gradient in a narrower window.

## Final round

Full matrix on the final binary (sha256 `30FE1978...A6355`), log
`final/capture.txt`; fallbacks in `final/capture-nocomp.txt` and
`final/capture-w900.txt`. The probe PNGs were deleted after this file recorded
their numbers; the probe logs remain.

| Case | field / black | field-end / black | paper / black | field / white | field-end / white | paper / white |
| --- | --- | --- | --- | --- | --- | --- |
| en-light-100 | 0.7787 | 0.7273 | `#F7F9F9` 0.9437 | 0.8354 | 0.7802 | `#F9FBFB` 0.9610 |
| zh-light-125 | 0.7731 | 0.7183 | see note | 0.8296 | 0.7708 | see note |
| en-dark-100 | 0.0150 | 0.0171 | `#1F282E` 0.0201 | 0.0209 | 0.0235 | `#20292F` 0.0210 |
| zh-dark-100 | 0.0150 | 0.0171 | `#1F282E` 0.0201 | 0.0209 | 0.0235 | `#20292F` 0.0210 |
| en-light-100, composition off | 0.8424 | 0.7863 | `#FFFFFF` 1.0 | 0.8424 | 0.7863 | `#FFFFFF` 1.0 |
| en-light-100, 900 px wide | 0.8249 | 0.7708 | not sampled | 0.8249 | 0.7708 | not sampled |

Note: at 125% text scale the paper point (40, 400) reads `#F4F8F6` over both
backdrops. That is the opaque well colour, so at this scale the point falls on
a well, not on paper padding. It is not a paper measurement and is not used.

Every light field sample over black is at or above 0.7183, so the decision
stands. The numbers match probe 2 exactly at 100%.

Glass-off wells, read from `final/en-light-100-software-nocomp.png` and
`final/en-light-100-status-nocomp.png` (PrintWindow pixels): paper `#FFFFFF`
at (50, 500), (784, 600) and (784, 900); well `#F4F8F6` at (300, 480). In
both images the well reads as a distinct rounded panel on the white paper
with no border. The probe build's status page showed no well at all.

Copy defects from the step-5 screenshots, checked in the final images:

- Clean landing first row: "No folder chosen" (`en-light-100-clean.png`).
- Status empty state title: "尚无快照" (`zh-light-125-status.png`), not the
  phase name.
- Software while loading: the sentence appears once, in the page bar; the card
  keeps the progress track (`zh-dark-100-software.png`,
  `en-light-100-software-nocomp.png`).

Not covered by these captures: the wide window (1350 px and above) of AC3,
and any state that needs a scan, an inventory result or a snapshot. The
harness is read-only and each launch has an empty profile, so Clean review
rows, Analyze breadcrumbs and Software rows with source names were not
captured; `display_path` and the source labels are verified by unit tests and
by reading the call sites.

## Manual acceptance steps (not run here)

1. Settings > Personalization > Colors > Transparency effects: off. With the
   app open at 1080 px or wider, the field and paper must become opaque with
   no restart; layout and hit targets unchanged. Turn the setting back on.
2. Turn a Windows contrast theme on and off with the app open: the
   high-contrast palette applies, wells and rows keep 1px borders, then the
   saved theme returns.
3. Theme "Follow system": switch Windows app mode between light and dark with
   the app open; the window follows without restart (AC6).

## 2026-11-13, follow-up round (check findings)

Binary sha256 `5D865A34...8D760`. Same machine and settings as above. Logs:
`final/capture.txt` (default size, regenerated on this binary; it replaces the
log of binary `30FE1978...`), `final/capture-tree.txt`,
`final/capture-1080x720.txt`, `final/capture-1400x860.txt`. The unsuffixed
PNGs in `final/` were regenerated in the same run. The `-nocomp` and `-w900`
files are from the earlier binary and were not re-run.

### Field luminance on this binary

Default size (1254 x 827 logical): every value equals the "Final round" table
(light 0.7787 / 0.7273 over black, 0.8354 / 0.7802 over white; zh-light-125
0.7731 / 0.7183 and 0.8296 / 0.7708; dark unchanged). The tab-label threshold
moved from 45rem to 720 logical px; it does not touch the field, the paper or
any sample point, and no recorded number changed.

| Case | Window (logical) | field / black | field-end / black | field / white | field-end / white |
| --- | --- | --- | --- | --- | --- |
| en-light-100 | 1080 x 720 | 0.7681 | 0.7156 | 0.8244 | 0.7702 |
| en-light-100 | 1400 x 860 | 0.8093 | 0.7626 | 0.8678 | 0.8170 |
| zh-dark-100 | 1080 x 720 | 0.0154 | 0.0166 | 0.0214 | 0.0237 |
| zh-dark-100 | 1400 x 860 | 0.0135 | 0.0167 | 0.0199 | 0.0227 |

The lowest light sample over black is now 0.7156 (1080 x 720, lavender end),
a margin of 0.016 over the 0.70 rule. The decision stands. The paper point is
not sampled at these sizes.

### AC3 window sizes

`capture.ps1 -LogicalWidth 1080 -LogicalHeight 720 -Suffix:-1080x720` and
`-LogicalWidth 1400 -LogicalHeight 860 -Suffix:-1400x860`, cases en-light-100
and zh-dark-100, five modes and Settings each. Physical sizes 1350 x 900 and
1750 x 1075 at 120 dpi. UIA reports all five tabs by name in every run, and
the five labels are visible in the images looked at
(`en-light-100-software-1080x720.png`, `zh-dark-100-software-1080x720.png`,
`en-light-100-status-1400x860.png`). At 125% text scale and the default width
(`zh-light-125-*.png`) all five labels are also visible; the old 45rem rule
needed 900 px there, the window is 1254 px, so this case passed before too.
A 125% window between 720 and 900 px was not captured.

Seen at 1080 x 720, English: the "Working…" badge ends flush against the
theme button with no gap. Nothing overlaps or clips.
Fixed after this capture: the trailing control row now has `pl(SPACE_2)`, so
the badge or the inline tabs keep a gap before the theme button when the row
has no free width. Not re-captured.

The "DevSweep" wordmark is absent at 1080 x 720 and at 125% text scale. The
rule is `!compact || narrow` with `compact` = width below 69rem (1104 px at
100%, 1380 px at 125%). It is unchanged by this task, and
`before/zh-light-125-status.png` shows the same logo-only title bar, so it is
not a regression.

### First rows

Read from the regenerated images:

- Optimize landing: "19 operations, 7 run after review"
  (`en-light-100-optimize.png`). No empty band.
- Status, not live: "Not sampling" / "未在采样" (`en-light-100-status-1400x860.png`,
  `zh-light-125-status.png`).
- Analyze landing: "2 locations" (`en-light-100-analyze.png`). The location
  cards are filled wells with no border. On the translucent paper the well
  step is faint, as for every other well.
- Analyze with a folder open: the breadcrumb appears once, in the first row;
  the row under it keeps the size and completeness tags
  (`en-light-100-analyze-treemap-tree.png`).
- Software: only the loading state was captured ("Reading installed
  software… This is read-only."). The zero-item line ("No entries") and the
  Optimize non-landing phases were not seen in a window.

### Treemaps (fixture only)

`capture.ps1 -Treemap` writes a generated folder (three small projects with
`target`, `node_modules`, `__pycache__`, `.pytest_cache`, plus plain folders,
15.3 MB) under the run's temp directory, saves it as the scan root of the
isolated profile, opens it in Analyze and runs a project-scope Clean scan on
it. No preview, confirm or cleanup is invoked and no user folder is scanned.

- Analyze, 100% and 125% (`*-analyze-treemap-tree.png`): tile names and
  sizes sit inside their tiles with horizontal padding; the 39 px tile shows
  its name only; tiles too short for a line show no text. Nothing is cut.
- Clean, 100% and 125% (`*-clean-treemap-tree.png`): tile labels fit at both
  scales. Two defects were visible in the first capture of this round and are
  fixed in the regenerated images: at 125% the group title ("service-rust ·
  8.1 MiB") was cut by the tile below it, because the title band is 18 map
  pixels while the label is rem sized; and the last tile of each group had no
  bottom edge, because the group box and the tile were both inset by one
  pixel. The title is now one line of the band height and the group box is
  the whole group rectangle.
- Clean review rows (`*-clean-review-tree.png`) show paths as
  `C:\Users\...` with no `\?\` prefix and an upper-case drive.

Still not covered: Software rows with source names, Status with a snapshot,
high contrast, and the three manual acceptance steps listed above.
