# Design: Palette Catalogue

Follow parent design section 5 and research/design-proposal.md. The visual proposal owns the initial color values. The complete semantic token mapping and measured contrast pairs are implementation deliverables.

Extend the V2 theme enum and all corresponding fixtures/patch variants/decoders. Define one typed catalogue for choice ID, localized label, light/dark scheme, and preview metadata. Resolve System through existing legacy light/dark palettes. Apply data-theme separately from style.colorScheme; never assign catppuccin_mocha, codex, or another palette ID to colorScheme.

Keep appearance.css as the shared token owner. Add complete rules for all four new themes, including control-border, popup, highlighted/selected option, focus, semantic status foreground/background, and capsule colors. Portal roots inherit document tokens. Keep forced-colors rules after theme definitions with sufficient matching specificity. Retain effective reduced motion independently of palette.

Radio preview tiles show a small representative background, text, accent, and selected state. Use an accessible label and native checked state. Candidate hover does not apply globally. Parent save semantics determine when a chosen tile becomes the committed choice.

Audit literal colors only in affected visual consumers. Do not globally refactor styles or replace the original planet color model. Update styles.test.ts to enumerate theme IDs/token completeness rather than split CSS around one light-theme selector. Tests must also prove color-scheme resolution, forced-color precedence, and main/HUD saved snapshot application.

Expected files: appearance.ts/appearance.css, SettingsPage.tsx and control styles, PreferencesProvider, preferences/style tests, core desktop_preferences theme enum/tests, api fixtures/contracts/generator output, resources/i18n sources, and affected desktop visual/backend preference specs.

Record Catppuccin upstream source/license and the actual imported revision. Codex and Claude values are app-authored inspired designs; no official-palette claim is permitted. No palette npm package is required. Reverting the theme UI/renderer and schema extension must leave V2 bytes intact and fail closed when an older build cannot read a newer value.
