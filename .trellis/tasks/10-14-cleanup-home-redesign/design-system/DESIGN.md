---
name: DevSweep cleanup workbench
colors:
  primary: "#28643F"
  on-primary: "#FFFFFF"
  surface: "#FFFFFF"
  text: "#17251C"
  neutral: "#F6F7F4"
  secondary: "#EDF2EC"
  muted: "#536359"
  border: "#CDD6CE"
  control-border: "#7C8C80"
  warning: "#865409"
  danger: "#A32D34"
  focus: "#28643F"
  primary-dark: "#8BCEA3"
  on-primary-dark: "#122019"
  canvas-dark: "#111714"
  surface-dark: "#18211B"
  raised-dark: "#202C24"
  text-dark: "#E8EEE9"
  muted-dark: "#ABB8AF"
  border-dark: "#35483B"
  control-border-dark: "#6A8071"
  focus-dark: "#A8E2BC"
  warning-dark: "#E7B66A"
  danger-dark: "#F0A09C"
typography:
  h1:
    fontFamily: 'Segoe UI Variable, Segoe UI, Microsoft YaHei UI, system-ui, sans-serif'
    fontSize: 24px
    fontWeight: 600
    lineHeight: 1.35
  body-md:
    fontFamily: 'Segoe UI Variable, Segoe UI, Microsoft YaHei UI, system-ui, sans-serif'
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.6
  label-caps:
    fontFamily: 'ui-monospace, Consolas, monospace'
    fontSize: 12px
  sourceScale: "12/14/16/20/24/32"
  weights: "400, 500, 600"
rounded:
  sm: 4px
  md: 8px
  section: 12px
spacing:
  sm: 4px
  md: 8px
  sourceScale: "4/8/12/16/24/32"
---
<!-- design-md: entry=style:shadcn; source=https://github.com/bergside/awesome-design-skills@f631a09; license=MIT -->
<!-- customized: DevSweep utility layout, original green palette, native CJK stack, light/dark tokens, independent cleanup scopes, original animal identity. -->

## Overview

Visual thesis: a precise developer utility with charcoal or warm-white surfaces, restrained green actions, aligned capacity figures, and a small original beaver identity.
Content plan: orient with Cleanup and snapshot status; present two scope sections and their scan actions; show provider/project rows; enter selection, preview, confirmation, and outcomes.
Interaction thesis: immediate keyboard actions, 100 ms pointer-only press acknowledgement, real backend progress; no continuously animated mascot or Earth on Clean home.

This is a planning artifact, not production implementation. Promote it to desktop/DESIGN.md during the approved desktop task. Preserve repository-root design.md, a historical architecture document with a case-insensitive name collision. CSS strategy: existing plain CSS/custom properties, React, and Base UI. The catalogue choice does not imply a shadcn, Tailwind, font, or icon dependency.

## Colors

Use the light/dark role sets above only as Clean-local aliases/overrides under the Clean workbench root. Do not replace shared root appearance tokens or change other modes/Settings palettes. Green means emphasis; amber attention; red failure/destructive confirmation. Every warning uses text plus an icon. Dividers are decorative; control-border is the visible input boundary; focus is a 3px ring.

Keep light/dark/system/Latte/Mocha/Codex/Claude choices and explicit font/scale preferences. Map Clean-local roles for each selected theme during implementation, inheriting saved preferences and leaving other mode tokens intact. Do not reset a user's installed-font preference. Check body pairs at 4.5:1 and focus/control pairs at 3:1. Compare unchanged modes' palette/geometry before and after, apart from the intentional shared logo.

## Typography

Default to the existing Windows-native sans/CJK stack. Titles: 24px/600; scope headings: 20px/600; body/control: 14px; metadata: 12px; measured capacity: at most 32px/600. Numbers use right-aligned tabular figures. Paths remain fully accessible/copyable. CJK tracking is zero and text wraps naturally. The supplied screenshot's saved font state is unknown.

## Layout

At >=1024px use existing compact mode navigation, a workspace capped at 1280px, 24px outer padding, and two sections in an approximately 2:3 global/project ratio. At <1024px stack sections, with 16px outer padding. At 390px use one-column rows with secondary metadata below; at 320px keep inputs/actions usable. Only path/table containers may horizontally scroll.

First-run global scanning works with zero project roots. The project section explains how to add a folder. Each section has its own action, report timestamp, scope, and error/partial state. One heavy operation runs at a time through the existing coordinator. Retain the other section's last report.

Concept, with no invented capacity values:

    [small animal] DevSweep   Clean | Software | Optimize | Analyze | Status | Settings
    Cleanup                                             Last scan / coverage
    Global caches                          Project artifacts
    Tool-owned developer caches            Selected folders [Manage]
    [Scan global caches]                   [Scan project artifacts]
    uv / npm / pip / ...                    Project > Node builds / dependencies
    capacity / capability / method                    > Cargo target / mbx outputs
    [Review selected global items]         [Review selected project items]

## Elevation & Depth

Use opaque surface steps and 1px row dividers. No glass, glow, decorative gradients, equal-size marketing card grid, or giant illustration. The small animal is the identity anchor. No sidebar or unrelated five-mode restyling.

## Shapes

8px controls, 12px group containers where a boundary helps, 4px badges. Preserve capsule navigation as shared shell chrome, allowing measured compactness fixes. The 16/20/24px brand mark requires a simplified silhouette; a large raster concept is not automatically a tray icon.

## Components

Scope section: title, purpose, controls, action, status/progress, rows. Scoped failure never erases the other report.
Scan button: 44px hit area, specific label, green primary, visible disabled reason, stable loading dimensions and aria-busy. Tokenized hover, 3px focus ring, pointer-only 1px press translation; reduced motion disables movement.
Folder input: explicit text type, 44px height, pasteable absolute paths, backend validation. Suggestions are never automatic selections.
Provider row: tool, footprint, capability, confidence and review. Distinguish missing/unsupported/failed/empty/available. Cache footprint is not guaranteed recoverable capacity.
Project row: verified owner, deduplicated artifact/dependency breakdown, external mbx/shared badge, confidence and warning. A warning never selects a target.
Warning: configurable user-approved 5 GiB per-project default; show threshold/reason. Unknown remains unknown; a partial value below threshold is not proof of being below threshold.
Semantic icons: retain one vector Glyph vocabulary for cache/tool, folder/build, shared cache, warning, inspect-only, preview, history. Image generation owns the original animal; action glyphs stay small and crisp.
Review/confirm: preserve plan, live digest, invalidation, irreversible-command disclosure, and second confirmation. Use modals only for actual confirmation.

## Do's and Don'ts

- Use independent scan actions and scope-specific results.
- Separate occupied footprint, selected estimate, and actual outcome.
- Use estimated-recoverable and moved-to-trash wording; never infer freed bytes.
- Count shared mbx storage once and never attribute it to every project.
- Keep full accessible paths and untruncated authority/action text.
- Never make a warning, remembered path, or folder suggestion grant authority.
- Do not copy Mole assets or invent progress percentages.
- Preserve user appearance settings and unrelated mode behavior.

## Responsive Behavior

Verify 390/800/1024/1350/1440 CSS widths and 320px controls; native minimum 900x600 and default 1080x720 plus minimum-width, minimum-height, combined-minimum and normal 1350x900. Cover English/Chinese, all themes, text scaling, long paths, keyboard, forced colors and reduced motion.

## Iteration Guide

Tokens, then controls, then rows, then composition. Use the same fixtures/viewports for before/after captures. Measure region/input bounds. Report checked/changed/unchanged consumers. Verify idle, scanning, cancel, partial/error, empty, oversized, review, confirm and outcome.

## Known Gaps

This planning turn has no rendered-product or native acceptance. Theme mappings are implementation work. The original animal concept needs alpha and small-size review before packaging. Default fonts must be diagnosed from actual preferences and computed styles before changing typography behavior.

## Agent Prompt Guide

Global section: 20px/600 heading, 14px/1.6 body, 16px gaps, 44px actions, 8px radii, selected theme surface/text/primary tokens. No project-root requirement.
Project row: 16px semantic icon, 14px title, complete accessible path, right-aligned capacity, text-plus-icon warning, 16px column gaps and 12px row padding. Separate shared storage from project-attributed bytes.


