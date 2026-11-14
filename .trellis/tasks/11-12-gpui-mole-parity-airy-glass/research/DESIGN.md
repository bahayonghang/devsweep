---
version: alpha
name: 轻盈毛玻璃
description: "轻盈毛玻璃 is airy frosted glass: a light field, white paper, and two semantic colors, one for input and one for output, in a colourway picked for the project."
colors:
  field-start: "#E7F6F1"
  field-end: "#EDE7F8"
  sheet: "rgba(255,255,255,0.78)"
  sheet-solid: "rgba(255,255,255,0.96)"
  sheet-edge: "rgba(255,255,255,0.80)"
  ink: "#24303A"
  muted: "#5C6B76"
  input: "#187A56"
  input-wash: "#E7F7F0"
  output: "#6D4BD6"
  output-wash: "#F3EEFC"
  field-start-dark: "#142322"
  field-end-dark: "#232032"
  sheet-dark: "rgba(32,41,47,0.90)"
  sheet-solid-dark: "#20292F"
  sheet-edge-dark: "rgba(255,255,255,0.16)"
  well-dark: "#252F36"
  ink-dark: "#EDF4F2"
  muted-dark: "#B3C2BF"
  input-dark: "#87D9BA"
  output-dark: "#C6B5FF"
  warning: "#805413"
  danger: "#9F3B45"
  warning-dark: "#EDC37E"
  danger-dark: "#FFA6A9"
typography:
  display-num:
    fontFamily: 'Figtree, "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: 32px
    fontWeight: 700
    letterSpacing: -0.02em
    fontVariantNumeric: tabular-nums
  title:
    fontFamily: 'Figtree, "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: 16px
    fontWeight: 650
    letterSpacing: 0
  body:
    fontFamily: 'Figtree, "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.7
    letterSpacing: 0
  meta:
    fontFamily: 'Figtree, "PingFang SC", "Noto Sans SC", sans-serif'
    fontSize: 12px
    fontWeight: 400
    letterSpacing: 0
rounded:
  image: 12px
  well: 16px
  sheet: 20px
  pill: 999px
spacing:
  min: 8px
  sm: 12px
  md: 16px
  lg: 24px
  xl: 32px
  xxl: 48px
material:
  field-angle: 160deg
  sheet-blur: 20px
  sheet-blur-min: 16px
  sheet-blur-max: 24px
  sheet-saturate: 1.15
  blob-blur: 80px
  blob-opacity: 0.55
  blob-max: 3
  backdrop-max: 6
  solid-breakpoint: 960px
elevation:
  sheet-shadow: "0 20px 48px rgba(70, 90, 120, 0.08)"
motion:
  press-scale: 0.98
  press-duration: 120ms
---
<!-- design-atlas: entry=style:airy-glass; source=first-party; license=MIT -->
<!-- palette: mint; reason: fallback (no palette or content signal) -->
<!-- customized: GPUI native adaptation for desktop-gpui. Added a dark colour group, risk tones and chart hues as declared extensions, a Windows system font stack, a material mapping without backdrop-filter, and layout rules for a tabbed utility. The six colourway values are unchanged. See "GPUI Native Adaptation". -->

## Overview

轻盈毛玻璃 is airy frosted glass for a workbench. The field is bright, low-chroma air. White paper floats on the field gradient, which runs from `{colors.field-start}` to `{colors.field-end}`. Read status, then the number, then the action. There is no marketing hero.

语义色只有两个. `{colors.input}` means input or completion. `{colors.output}` means output. Text sits on paper or on a solid well, never on the field.

Glass is not the default card skin. Only the paper uses `backdrop-filter`, and only once. This file does not define a component library. Do not import the glassmorphism palette, fonts, or radii.

Theme switch. The saved desktop theme picks the colour group. `light` and `catppuccin_latte` use the colourway below. `dark`, `catppuccin_mocha`, `codex` and `claude` use the dark group in "GPUI Native Adaptation". `system` follows the Windows app mode. A Windows contrast theme overrides both with the high-contrast palette. First launch stays dark (user decision, 2026-11-04).

## Colourway

The colourway is a dial. `apply` sets it from the project: an accent token the project already has, then the project content, then the default. The roles, the material, and the contrast gates are the same in every colourway. Only the six hex values below change.

<!-- palette:begin colourway -->
Colourway: `mint` (薄荷 → 薰衣草 / mint to lavender).

- Field: `#E7F6F1` to `#EDE7F8`.
- Input: `#187A56` on `#E7F7F0`.
- Output: `#6D4BD6` on `#F3EEFC`.
- Other colourways: `mint`, `sky`, `apricot`, `slate`, or `hue:<0-359>`.
<!-- palette:end colourway -->

To switch, rerun `apply --force --palette <preset|hue:N>`. Do not edit one hex by hand. The contrast table under Accessibility is measured for this colourway only.

## Colors

Field:

- `{colors.field-start}` (`#E7F6F1`) to `{colors.field-end}` (`#EDE7F8`), about 160deg. Both ends stay low-chroma (OKLCH C at most 0.03) in every colourway.
- At most 3 blobs. A blob keeps the hue of one field end and raises chroma slightly. Do not assign a new hex. Blur about 80px, opacity about 0.55. Keep blobs out from under text.

Paper is the only surface that may blur:

- Fill `{colors.sheet}` (`rgba(255,255,255,0.78)`).
- 模糊 20px, written `backdrop-filter: blur(20px) saturate(1.15)`. The allowed blur is 16px to 24px. The default is 20px. Saturation stays 1.15.
- 1px edge `{colors.sheet-edge}` (`rgba(255,255,255,0.80)`). This is a light edge, not an opaque fence, and not a control boundary.
- One shadow: `{elevation.sheet-shadow}`.

Wells are solid and do not blur again. `{colors.input-wash}` (`#E7F7F0`) is the input and completion well. `{colors.output-wash}` (`#F3EEFC`) is the output well.

Text:

- `{colors.ink}` (`#24303A`) for titles and body. It is the same neutral in every colourway.
- `{colors.muted}` (`#5C6B76`) for 12px secondary text. It is also fixed.
- `{colors.input}` (`#187A56`) for input, completion, links, and the focus ring.
- `{colors.output}` (`#6D4BD6`) for output and secondary figures.

窄屏、减少透明度时的不透明回退、以及没有 `backdrop-filter` 时,纸改为 `{colors.sheet-solid}` (`rgba(255,255,255,0.96)`),模糊为 none. 保持同一色相,只提高不透明度. 填充 0.78 是有意比水晶卡更实,好让低饱和字段上的文字稳定过 4.5:1. 轻来自字段的颜色、大圆角和淡阴影,不来自把纸做成全透明.

## Typography

Family: `Figtree, "PingFang SC", "Noto Sans SC", sans-serif`. Do not use Inter, Plus Jakarta Sans, or DM Sans.

| Role | Token | Size | Weight | Notes |
| --- | --- | --- | --- | --- |
| Large numbers | `{typography.display-num}` | 32px | 700 | `tabular-nums`. Latin digits may use -0.02em |
| Title | `{typography.title}` | 16px | 650 | No negative tracking |
| Body | `{typography.body}` | 14px | 400 | Chinese running text line-height 1.7 |
| Secondary | `{typography.meta}` | 12px | 400 | Color `{colors.muted}` |

The -0.02em tracking belongs only to large Latin numerals. Chinese running text does not use negative letter-spacing.

## Layout

Spacing is 8 / 12 / 16 / 24 / 32 / 48 (`{spacing.min}` through `{spacing.xxl}`). The minimum gap is 8px.

Order on the workbench is status, number, action. Do not add a marketing hero.

When the paper padding is 8px, an inner image uses a 12px radius, because outer radius = inner radius + padding.

Icons are linear, about a 1.5px stroke, in `currentColor`. Do not add illustrated characters.

## Elevation & Depth

Depth is the field showing past the paper, one soft shadow, and the large radius. Do not stack shadows or add a glow.

```css
.field {
  background: linear-gradient(160deg, #E7F6F1, #EDE7F8);
}
.sheet {
  background: rgba(255,255,255,0.78);
  backdrop-filter: blur(20px) saturate(1.15);
  border: 1px solid rgba(255,255,255,0.80);
  box-shadow: 0 20px 48px rgba(70, 90, 120, 0.08);
  border-radius: 20px;
}
.sheet.is-solid {
  background: rgba(255,255,255,0.96);
  backdrop-filter: none;
}
.sheet:active {
  transform: scale(0.98);
  transition: transform 120ms;
}
@media (prefers-reduced-motion: reduce) {
  .sheet:active {
    transform: none;
  }
}
```

The 1px border is the light edge only. CSS `border-style: solid` here only paints the translucent color. It is not an opaque fence and it is not the control boundary.

Pointer press uses `transform: scale(0.98)` for about 120ms, and animates `transform` only. Keyboard state changes do not animate. `prefers-reduced-motion: reduce` cancels the scale. Do not animate the blur radius.

One viewport holds at most 6 `backdrop-filter` layers. Do not nest paper inside paper. Wells, list rows, and tables stay solid.

## Shapes

| Token | Radius | Use |
| --- | --- | --- |
| `{rounded.image}` | 12px | Image inside paper when padding is 8px |
| `{rounded.well}` | 16px | Solid wells |
| `{rounded.sheet}` | 20px | Paper |
| `{rounded.pill}` | 999px | Buttons and tags |

Buttons and tags are pills. Paper is 20px. With 8px padding, the inner image is 12px so that outer = inner + padding.

## Accessibility

Meet WCAG 2.2 AA on the worst specified background, not on an average of the field. Both field ends are checked, and the worse one decides. These ratios were measured for this colourway: white 0.78 over the field end, mixed per sRGB channel, then WCAG 2.x relative luminance. Do not replace them with a new calculation in this file. Change the colourway with `apply --force --palette`, which measures again.

<!-- palette:begin contrast -->
Controls show their boundary by fill, plus a 2px focus ring in `{colors.input}` (`#187A56`). On white that ring is 5.31:1. The white edge on the field start is 1.11:1, which is below the 3:1 non-text boundary, so the edge cannot be the control boundary.

| 前景 | 背景 | 比值 | 用途 |
| --- | --- | --- | --- |
| `#24303A` | `#FFFFFF` | 13.47 | 正文,不透明回退 |
| `#24303A` | 白 0.78 叠 `#E7F6F1` | 13.16 | 正文,字段起点 |
| `#24303A` | 白 0.78 叠 `#EDE7F8` | 12.95 | 正文,字段终点 |
| `#5C6B76` | 白 0.78 叠 `#E7F6F1` | 5.37 | 次要文字,字段起点 |
| `#5C6B76` | 白 0.78 叠 `#EDE7F8` | 5.29 | 次要文字,字段终点 |
| `#187A56` | `#E7F7F0` | 4.79 | 输入或完成,正文与大号数字 |
| `#187A56` | `#FFFFFF` | 5.31 | 链接与焦点环 |
| `#6D4BD6` | `#F3EEFC` | 5.08 | 输出,正文与大号数字 |
| `#6D4BD6` | `#FFFFFF` | 5.79 | 输出色在白纸上 |
| `#FFFFFF` | `#E7F6F1` | 1.11 | 白描边,不能当控件边界 |
<!-- palette:end contrast -->

A lighter tint of `{colors.input}` or `{colors.output}` that falls under 4.5:1 on its well is not a token. Use the measured value and keep the well light.

## Writing Tone

State the status, then the number, then the action. Keep labels short. No slogan and no marketing headline. Do not tighten Chinese text to match the Latin number tracking.

## Do's and Don'ts

### Do

- Put text only on paper or on a solid well.
- Use `{colors.input}` for input, completion, links, and the 2px focus ring.
- Use `{colors.output}` for output and secondary figures.
- Blur only the paper, once, at 模糊 20px and saturate 1.15.
- Use the 不透明回退, `{colors.sheet-solid}`, below 960px, for 减少透明度, and when `backdrop-filter` is missing.
- Set numbers in `tabular-nums`.
- On pointer press, scale to 0.98 for about 120ms on `transform` only. Skip that motion for keyboard activation and for `prefers-reduced-motion`.

### Don't

- Don't use glass as the default card skin. Wells, list rows, and tables are solid. Only paper on the field may use one `backdrop-filter`.
- Don't use the 1px `rgba(255,255,255,0.80)` edge as a control boundary or as an opaque fence. On the field it is far below 3:1. The Accessibility table has the ratio.
- Don't nest paper, and don't put more than 6 `backdrop-filter` layers in one viewport.
- Don't animate blur.
- Don't use Inter, Plus Jakarta Sans, or DM Sans.
- Don't set body text in a lighter tint of `{colors.input}` or `{colors.output}`. Under 4.5:1 on its well, it fails.
- Don't apply negative letter-spacing to Chinese running text.
- Don't add a second shadow, a neon glow, or a saturated neon gradient.
- Don't put text on the field or on a blob.
- Don't turn this field gradient off as decoration, and don't paste it onto another DESIGN.md. Choosing this style means the gradient is the material.
- Don't change one hex of the colourway by hand, and don't raise the chroma of a field end. Switch the whole colourway with `apply --force --palette`.
- Don't repeat the selected mode's name as a page heading with an icon tile. The tabs name the page.
- Don't fence a well, row or tile with a 1px border in normal contrast. Fill carries the boundary. High contrast keeps borders.
- Don't use a risk tone or a chart hue for decoration. Risk tones mark risk, failure and destructive confirmation. Chart hues colour treemap cells and their legend.
- Don't let the desktop behind the window grey out the light field. If the window backdrop cannot keep the field bright, paint the field opaque.
- Don't write a size, radius, gap or colour literal in view code. Use the named tokens in `desktop-gpui/src/ui.rs`.

## Responsive Behavior

| Condition | Paper |
| --- | --- |
| 960px and above, transparency allowed, `backdrop-filter` supported | Fill 0.78, 模糊 20px, at most 6 papers |
| Below 960px | `{colors.sheet-solid}`, blur none |
| `prefers-reduced-transparency: reduce` | 减少透明度时的不透明回退, at every width |
| No `backdrop-filter` | 不透明回退 |

Pills stay pills. The focus ring stays 2px `{colors.input}`. Keyboard focus does not scale.

## GPUI Native Adaptation

This section maps the entry onto `desktop-gpui` (GPUI on Windows). The CSS above is reference only. Production tokens live in `desktop-gpui/src/ui.rs` and follow this file.

### Dark group

The dark group is not part of the colourway dial. Do not derive it by inverting the light values.

| Role | Token | Value |
| --- | --- | --- |
| Field | `{colors.field-start-dark}` to `{colors.field-end-dark}` | `#142322` to `#232032`, 160deg |
| Paper | `{colors.sheet-dark}` | `#20292F` at 0.90. Opaque fallback `{colors.sheet-solid-dark}` |
| Paper edge | `{colors.sheet-edge-dark}` | white at 0.16 |
| Well | `{colors.well-dark}` | `#252F36`, solid |
| Text | `{colors.ink-dark}`, `{colors.muted-dark}` | `#EDF4F2`, `#B3C2BF` |
| Input | `{colors.input-dark}` | `#87D9BA` |
| Output | `{colors.output-dark}` | `#C6B5FF` |

Measured 2026-11-12, sRGB channel mix, WCAG 2.x relative luminance, worse field end:

| 前景 | 背景 | 比值 | 用途 |
| --- | --- | --- | --- |
| `#EDF4F2` | 纸 0.90 叠 `#232032` | 13.36 | 正文 |
| `#B3C2BF` | 纸 0.90 叠 `#232032` | 8.08 | 次要文字 |
| `#87D9BA` | 纸 0.90 叠 `#232032` | 8.99 | 焦点环与链接 |
| `#EDF4F2` | `#252F36` | 12.23 | 内容区正文 |
| `#B3C2BF` | `#252F36` | 7.40 | 内容区次要文字 |
| `#87D9BA` | 输入色 7% 叠 `#252F36` | 7.02 | 输入与完成 |
| `#C6B5FF` | 输出色 7% 叠 `#252F36` | 6.41 | 结果数字 |
| 白 0.16 | `#142322` | 1.66 | 纸边，不能当控件边界 |

### Colour roles in this product

- `input`: selection, the primary action, completion, links, the focus ring.
- `output`: result figures (reclaimable, selected and moved bytes, counts after a run) and their wells.
- Risk tones are a declared extension for a cleanup tool. `{colors.warning}` and `{colors.danger}` mark risk, failure and destructive confirmation only, always with an icon or a word. On their 7% washes they measure 5.94 and 5.93 (light), 7.06 and 6.37 (dark).
- Chart hues are a declared extension. The seven category hues in `Theme::category` colour treemap cells and their legend only.
- Secondary text in the light group is `#505F68` (6.35 on paper over the worse field end), darker than `{colors.muted}`, so it holds 4.5:1 when the window backdrop is on. `text_muted` and `text_subtle` merge into this one value.

### Material without backdrop-filter

- GPUI has no per-element backdrop blur. The field is a smooth, low-chroma gradient, so paper at `{colors.sheet}` alpha over it gives the same picture as the 20px blur. That composite is the one blur. Do not put a second translucent layer inside the paper.
- The opaque fallback paints the paper at 1.0, not 0.96. Over an opaque field the difference is not visible.
- Window backdrop. At 960 logical px or wider, with Windows transparency effects on and no contrast theme, the window may request the OS backdrop. The field stays the dominant colour: tint the backdrop with the field colour, and check a field pixel over a black and a white controlled background. If the light field falls below 0.70 relative luminance on black, paint the field opaque.
- No blobs in this pass. GPUI has no element blur, and a raster blob is a new asset.
- One shadow, `{elevation.sheet-shadow}`, on the paper in both groups.
- Press feedback is one fill step. GPUI elements do not scale, so the 0.98 press scale does not apply. No underline on press.

### Type

One rem is 16px times the saved text scale.

| Role | Size | Weight | Use |
| --- | --- | --- | --- |
| `{typography.display-num}` | 2rem | 700 | The lead figure of a view or a metric tile: found, selected, moved, a reading. Tabular figures |
| `{typography.title}` | 1rem | 600 | Section and dialog titles, Settings page titles |
| `{typography.body}` | 0.875rem | 400 | Rows, controls, running text. Line-height 1.7 for Chinese paragraphs |
| `{typography.meta}` | 0.75rem | 400 | Secondary lines, column heads, tags |

No other sizes. The family is the saved installed font, otherwise the system UI font (Segoe UI Variable, with Microsoft YaHei UI through DirectWrite fallback). Title weight is 600 because the system face has no 650.

### Layout

- Spacing is 0.5, 0.75, 1, 1.5, 2 and 3rem only.
- Radii: paper 20px, well 16px, inner item (row, input, tile, treemap cell) 12px, buttons, tags and segments pill. A checkbox glyph keeps a 4px corner.
- The first row of a mode page is status, then the figure, then the action.
- The paper fills the window height. Lists scroll inside it.
- Mode tabs keep their text labels at 720 logical px and wider.

## Known Gaps

- The catalogue entry has no dark theme. The dark group above is a GPUI adaptation and is measured separately.
- The catalogue entry has no chart palette. The seven category hues are a GPUI extension for treemaps.
- Figtree is not bundled. The system UI font stands in for it.
- No element backdrop blur and no press scale in GPUI. See "Material without backdrop-filter".
- `{colors.ink}` and `{colors.muted}` do not tint with the colourway. On a warm field they read slightly cool.
- Contrast ratios above are measured on static wells and on the white-paper mix (white 0.78 over each field end, or `#FFFFFF` for the opaque fallback). They are not measured on photographs.
- If Figtree is unavailable, fall back to the system sans-serif and keep `"PingFang SC", "Noto Sans SC"` after the Latin family.
- Title weight 650 needs the Figtree variable font. Static Figtree files step by 100. If only a static file is available, use 600 for that role and do not add a second token.
- Line-height is specified for Chinese body text only (1.7). Other roles do not add a second line-height.
- Blob paints raise chroma without a separate hex. Do not invent one.
