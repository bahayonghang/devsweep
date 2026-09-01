# Design - Immersive desktop visual language

## 1. Boundary

Mole.fit and Mole for Mac screens are reference only. Implementation starts
from current DevSweep contracts in `desktop/src/` and
`.trellis/spec/desktop-frontend/`. No Mole source, screenshot, planet
photograph, hamster, or copy enters `desktop/` or tests.

TUI stays on the current workbench chrome. Spec edits stay under
`.trellis/spec/desktop-frontend/`. Do not change `.trellis/spec/frontend/`.

## 2. Spec rewrite (child 1, first)

`component-guidelines.md` Visual System currently forbids heroes, page-wide
palettes, oversized display type, and planets (`:82-99`). Replace that
clause with:

- Dark-only canvas. No light `#eef1ef` workbench.
- Original mineral/forest token family (not Mole navy/maroon/sun copies).
  One canvas token per mode, one accent per mode, shared text/border/focus.
- Centered capsule navigation. Pill radius is allowed on the capsule and
  primary actions only. Tool surfaces stay at 8px or below.
- Display-size tabular numbers for Estimated Recoverable and live metrics.
- CSS-native sweep body: one shape family, five tints, non-interactive,
  non-informational, still under `prefers-reduced-motion`. Not a globe map.
- Supporting destinations may sit in a labelled disclosure. Visible names
  required when open. Icon-only remains forbidden.
- Keep: native Windows chrome, Segoe UI Variable, visible focus, forced
  colors, Inspect Only, no "space freed", indeterminate Scan Progress.

`index.md` completion checklist line about "deep gray-green tokens" and
"bounded non-hero motifs" must match the new contract.

`state-management.md` workflow and coordinator invariants stay.

## 3. Shell composition

Keep `AppShell` as the route, focus, and coordinator owner
(`desktop/src/app-shell/AppShell.tsx`). Change presentation only:

```text
[ Windows title bar ]
        (icon)  Clean  Software  Optimize  Analyze  Status     [More]
                         mode canvas (full remaining height)
```

- Capsule is `role="tablist"` as today. Arrow/Home/End and Alt accelerators
  stay.
- Brand lockup text "Cleanup plan workbench" (`shell.v1.workbench`) is not
  required on the immersive chrome. Do not change that catalogue string;
  TUI still uses it (`crates/devsweep-cli/src/tui/shell/mod.rs:102`).
- More disclosure lists Protection, Rules, History, Language, Help with
  existing keys plus at most one additive `shell.v1.more`. Existing 22
  `shell.v1.*` forms stay byte-for-byte (`desktop/src/i18n/index.ts:9-32`).
- `data-mode` on `.app-shell` selects the mode canvas tokens.
- Deep links `#/clean` … `#/history` and `#/settings` stay.

No new production dependency. Sweep body is CSS (and optional SVG in the
React tree), not WebGL, not `<img>` of Earth.

## 4. Clean flow (child 2)

Reducer and IPC stay in `desktop/src/state/app-state.ts` and
`desktop/src/modes/clean/`. Pages change composition:

| Phase | First screen |
| --- | --- |
| idle, no Scan Report | Sweep body, ready copy, visible Projects/Global checkboxes, Scan |
| scanning | Same body, backend phase/message, indeterminate progress, Cancel scan |
| reviewed | Grouped Cleanup Targets by existing `kind` then `scope`; selection; Estimated Recoverable footer; Preview |
| dry_run / confirming / executing | Existing ExecutePage + ConfirmDialog on the dark canvas |
| reported | Large outcome number using existing trash copy; return to review |
| empty completed report | Sweep body + existing empty copy, not a light heading |

Scan Progress remains indeterminate. Do not invent a percentage.
Inspect Only stays unselectable in groups and select-all.
Capacity groups must keep verified / partial lower bound / unknown
separate. Do not sum them into one fake precise total.

Reuse existing keys (`clean.v1.action.scan`, `clean.v1.scope.*`,
`clean.v1.preview.estimated`, `clean.v1.trash.moved`). Additive
`clean.v1.*` keys are allowed only when no existing key fits; EN and
zh-CN must stay parallel.

## 5. Other modes (child 3)

No new IPC. Restyle on the shared canvas:

- Software: keep eligibility, manual MSI, current-user MSIX-only execute.
  Denser rows, leftover/details disclosure from existing fields, sticky
  summary/action bar.
- Optimize: first screen can show the sweep body plus Load/run of the
  closed eight-id catalogue. Running state lists existing operations with
  existing outcome tags. No extra maintenance ids.
- Analyze: keep list + treemap + breadcrumbs. Read-only. No trash action.
- Status: bento from existing CPU/memory/volume/network/power cards and
  process table. Unavailable stays unavailable. No health score. No GPU
  zero.

## 6. Tokens (original, not Mole)

Working names for the spec. Child 1 freezes hex in CSS variables.

| Token | Role |
| --- | --- |
| `--canvas` | Mode world background |
| `--canvas-clean` | Deep pine ink |
| `--canvas-software` | Dark oxide |
| `--canvas-optimize` | Dark olive |
| `--canvas-analyze` | Dark umber |
| `--canvas-status` | Dark gold-green |
| `--text` / `--muted` | Shared light text |
| `--accent` | Mode accent for the capsule pill and primary button |
| `--focus` | Visible focus ring |
| `--danger` / `--warning` / `--ok` | Semantic only |

Do not copy Mole screenshot hex. Do not introduce a light theme.

## 7. Compatibility

- Coordinator, fixtures, and generated IPC types stay.
- TUI snapshots must not change unless a catalogue form changes, which
  this design forbids for existing keys.
- `just ci` after catalogue edits.

## 8. Rollback

Each child rolls back its files. If the spec rewrite is reverted, UI
children must not ship. No partial immersive chrome with the old light
pane.
