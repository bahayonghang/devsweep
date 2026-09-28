# Settings Visual Proposal

Status: design proposal for planning review. Applied skills: frontend-design and su-architecture-first. No product code is changed.

## Direction

DevSweep is a desktop maintenance workbench with dense operational information. Settings should provide direct configuration, a visible current value, and a clear saved state. Keep the centered capsule, original DevSweep identity, flat surfaces, existing radius tokens, and five operational modes. Use color only for hierarchy, selection, and established status meanings.

The primary visual element is the theme preview group. Use quieter setting rows for language and performance. Avoid a sequence of equally heavy cards, repeated explanatory paragraphs, decorative gradients, glass, or added animation.

## Proposed layout

~~~text
 [DevSweep | Clean | Software | Optimize | Analyze | Status | Settings]

 Settings                                              Saved
 Appearance                                   Restore defaults
 Theme
 [DevSweep dark] [DevSweep light] [System]
 [Latte preview] [Mocha preview] [Codex preview] [Claude preview]

 Font                         [Search installed fonts          v] [Refresh]
                              Current font / fallback status
 Text size                    [100% v]
 [DevSweep   Clear, readable text   开发环境 清晰可读   0123456789 128 GiB]

 Language
 Display language             [简体中文 v]

 Performance                                  Restore defaults
 Motion                       [Follow system v]
 Planet frame cap             [30 FPS v]       Reason when disabled
 Status sampling interval     [2 seconds v]
 Status process rows          [15 rows v]
 HUD refresh interval         [2 seconds v]
~~~

Use one aligned container, maximum width 1040 CSS px, centered under the capsule. Align titles, section labels, and row labels to the same left edge. On wide screens each setting row has a label/description column and a control column. At widths below 800 px, stack each row. Below 480 px, theme tiles form one column; controls take the available width. The capsule retains horizontal scrolling. Keyboard focus must bring Settings into view at 390 px.

Settings has a visible page title and a tabpanel. Remove the old Back to Clean/last-mode button from Settings after promotion. Supporting pages retain their existing back behavior. Remove the duplicate Settings menu item; existing settings deep links still work.

## Controls

- Use a shared unstyled accessible Select primitive for fixed short lists, with DevSweep CSS. Approved package: @base-ui/react; see approval.md.
- Fonts use the same library's Combobox. Query text filters names and localized aliases. Typing does not save a font. Enter or pointer selection saves one option. Escape restores the committed display and closes the popup. Arrow movement highlights a candidate without writing preferences.
- Theme choices use native radio inputs with styled preview tiles and a check indicator. Selecting a theme is explicit. Hover and keyboard focus do not change the application theme.
- Proposed base control height is 40 px with 8 px radius, 12 px horizontal padding, a 16 px decorative chevron, and a visible focus outline. Use minimum sizes; allow text scale and wrapping to grow the control.
- The popup uses a portal to the document root, a border, a modest shadow, and shared surface/text/selection tokens. Align to the trigger with a 6 px offset; flip above when needed, constrain to the viewport, and scroll internally. Disable Base UI's overlapping Select positioning with alignItemWithTrigger=false.
- A selected option has a check plus text. A highlighted option has a distinct background. Disabled, loading, empty, and failed states include text. Keep the committed value visible after a failed save.
- Do not scale the entire page to the viewport. Keep the existing 100/110/125 percent choices, Chinese/English fallbacks, and separate HUD base size.
- Preview the selected font in one sample area. Keep catalogue rows in the UI font for stable list measurement; do not load and render every font as a preview at once.

## Palette proposals

The Catppuccin values below come from the official palette. Codex and Claude are DevSweep-authored inspired palettes. Their values are proposals, not claims about official product design tokens. No third-party logo, asset, or font is copied.

| Preset | Scheme | Canvas | Surface | Raised | Text | Muted | Accent |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Catppuccin Latte | light | #eff1f5 | #e6e9ef | #dce0e8 | #4c4f69 | #5c5f77 | #1e66f5 |
| Catppuccin Mocha | dark | #1e1e2e | #181825 | #313244 | #cdd6f4 | #bac2de | #cba6f7 |
| Codex | dark | #181818 | #212121 | #2b2b2b | #f2f2f2 | #b4b4b4 | #b8d7ff |
| Claude | light | #faf9f5 | #f0eee6 | #e8e5dc | #30302e | #62615b | #9d452d |

Keep the existing DevSweep dark and light palettes as compatibility choices. System continues to resolve those two palettes. The four named themes have explicit schemes. Default remains DevSweep dark; reset appearance returns to that default.

Theme tokens cover canvas, surface, raised/popup, text, muted text, border, focus, accent foreground/background, selection foreground/background, disabled text, danger, warning, success, overlay, shadow, capsule, titlebar, cards, tables, diagrams, dialogs, and HUD. The initial six-color proposals do not constitute a complete implementation palette.

Catppuccin surface tones alone can be too close for an input boundary. Define a separate stronger control-border token and check its contrast. Do not use a pastel accent as small body text without measuring contrast. Keep red/amber/green semantics for risk and result states. Preserve the existing procedural planet palettes; theme changes must not replace the renderer or its sampling behavior.

## Typography

The selected installed family becomes the UI family for body text, headings, labels, controls, and HUD. Retain explicit technical monospace where content requires it, and tabular number features where the chosen family supports them. The system choice retains the current stack. A missing selected font falls back to the existing Chinese/English system stack, while Settings displays the saved family and a missing-font notice.

Use a 24 px page title, 17-18 px section titles, the existing 14 px main base, and 12-13 px descriptions, all using the existing text-scale mechanism. Keep readable contrast in helper text. Keep prose to approximately 70 characters per line on wide screens. Do not add a remote or bundled display font.

## Design review against the request

The proposal makes Settings continuously reachable, replaces the reported native popup, removes the three-font ceiling, and makes all four named palettes visible. The proposal uses a theme preview group instead of another generic dropdown. Other rows share one restrained structure. The requested Claude palette justifies a warm light option; the app default and other themes retain their own identities.

See external-references.md for sources. Visual/native evidence is an implementation acceptance item, not a result of this planning turn.
