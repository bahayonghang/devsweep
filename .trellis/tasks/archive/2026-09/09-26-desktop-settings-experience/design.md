# Design: Desktop Settings Experience

## Status and authority

The user approved the documented plan and both production dependencies on 2026-09-26, then explicitly requested sequential implementation. See research/approval.md. The four user requirements are authoritative. research/repository-findings.md owns current-code evidence; research/design-proposal.md owns the visual proposal.

## Task ownership and order

| Child | Owns | Dependency |
| --- | --- | --- |
| 09-26-desktop-settings-navigation | Primary Settings tab, route/focus/panel behavior, relevant shell specs | None |
| 09-26-desktop-settings-controls | Settings composition, Select/Combobox presentation and interaction | Navigation first, because both touch Settings shell composition; Q1 |
| 09-26-desktop-settings-host-fonts | Native font catalogue, searchable font selection, preference V2 migration | Controls; Q1 |
| 09-26-desktop-settings-palettes | Final theme IDs, complete theme tokens, preview tiles, main/HUD coverage | Controls and fonts/V2 |

Use this sequence because files are shared. Do not treat the task tree as a dependency scheduler. Each child has focused checks and can be reviewed separately. Do not ship an intermediate preference schema; the parent is the release/integration acceptance boundary.

## 1. Navigation boundary

Retain ModeId with Clean, Software, Optimize, Analyze, and Status. Define a separate primary-navigation identity from mode routes plus settings. Build rendered order, key movement, control refs, labels, and selected state from the same list. Keep #/settings canonical. Supporting routes remain outside the primary tablist.

Settings receives the last tab and a corresponding tabpanel. Arrow/Home/End move focus within all six visible tabs; Enter/Space commits a route through the existing coordinator. The currently selected tab owns the tab stop. On supporting pages, retain one usable primary tab stop based on the last mode. Scroll a focused tab into the capsule viewport. Do not use focus movement to start or cancel operations.

All route entry points use the existing cancelAndJoin transition. A failed drain preserves the route, hash, state, and recoverable focus. Browser back/forward and direct deep links restore focus to the Settings tab when appropriate. Remove the Settings menu item and its obsolete opener state only when the new tab owns that behavior. Supporting page back controls remain intact.

## 2. Control boundary

Proposed production dependency: @base-ui/react, narrowly imported through Select and Combobox. Use local typed wrappers only where Settings shares behavior and tokens. Avoid a new general design system or a rewrite of other application controls. The font wrapper must accept typed options and separate highlighted/query state from the committed preference.

Select opens in a portal outside scrolling/overflow containers, with shared root tokens. Themes use native radio controls with visual tiles. Keyboard highlight, text filtering, pointer hover, and dismissal never save. Explicit option selection saves one existing typed field patch. Preserve the independent language store and its saving state. Preserve global preference saving/unavailable behavior and field-specific disabled reasons.

For a large font catalogue, begin with filtered and scrollable Combobox items, with no result truncation. Verify a 2,000-family synthetic list before release. If a measured result requires virtualization, use the selected library's supported integration and seek approval for any additional production dependency. Do not implement an unrelated performance framework or make an unmeasured latency claim.

## 3. Native font catalogue

Add a desktop-owned native module, proposed desktop/src-tauri/src/fonts.rs. Expose one read-only command desktop_fonts_list. The main window can call the command; HUD remains unable to enumerate or modify settings. Frontend calls remain inside the existing api/bridge.ts adapter. Core stores and validates the chosen value but does not discover fonts.

On Windows, use DirectWrite through a target-specific direct dependency on the windows crate with the required DirectWrite features. Select a release compatible with Rust 1.88 and the locked workspace. Create and consume the COM/DirectWrite objects within the enumeration worker; return only owned strings across the worker boundary. Invoke GetSystemFontCollection with cloud inclusion disabled. Explicit Refresh requests checkForUpdates=true. Use no shell commands, registry-only guessing, font-folder recursion, font installation, or font downloads.

Return all locally installed families exposed by the OS collection, including per-user fonts where the OS exposes them. Group faces by family; do not repeat bold/italic files as separate families. Each entry carries a canonical CSS family and localized name aliases. Prefer an en-US family name as canonical when available; otherwise use the first nonempty native family name. Preserve original spelling. Deduplicate by a stable case-insensitive family key, retain aliases, and sort for display without changing the persisted key.

Proposed response:

~~~json
{"status":"available","families":[{"family":"Segoe UI","names":[{"locale":"en-US","name":"Segoe UI"}]}]}
~~~

An unavailable response has a closed reason: unsupported_platform or enumeration_failed. An empty successful catalogue is a separate state. Do not return file paths or font bytes. Unsupported platforms remain buildable and return an explicit unavailable state; adding supported macOS/Linux desktop font discovery is outside the Windows MVP task.

Run discovery off the UI thread. Cache the last successful catalogue in the main renderer for the app session. Open Settings loads it once when absent; Refresh replaces it. Coalesce concurrent refresh requests. Ignore results after disposal or from older requests. Keep the old catalogue visible if refresh fails, and show the failure. No polling or background scan runs while Settings is unused.

The system-font choice always remains available. Search family names and aliases case-insensitively, including Chinese text. Selecting a family saves its canonical name. A family removed after selection retains its stored name; CSS fallbacks keep the UI readable and Settings reports the missing family after catalogue refresh. Enumeration failure does not mark the entire preference store unavailable.

## 4. Preference V2 and migration

Use DesktopPreferencesV2 with schema_version=2, existing performance/text fields, a closed theme ID, and a tagged font value. Proposed font field:

~~~json
{"font":{"kind":"system"}}
{"font":{"kind":"installed","family":"Microsoft YaHei UI"}}
~~~

Validate installed-family values as trimmed, nonempty strings of at most 256 Unicode scalar values with no control characters. Preserve non-Latin text, quotes, punctuation, and backslashes when valid. Escape the family as a CSS string through one tested serializer and set only the font custom property. Never interpolate a family into stylesheet source, a selector, HTML, or a URL. Search input is not a free-form persisted CSS stack.

Keep V1 decoding strict. V2 is stored at the fixed sibling path DevSweep/settings/desktop-preferences-v2.json. Reads first inspect V2. A valid V2 wins. Malformed, unreadable, or future V2 fails closed and must not fall back to V1. When V2 is absent, read and validate V1, convert it in memory, and leave V1 bytes unchanged. Missing V1 and V2 produce V2 defaults. Invalid V1 fails closed when V2 is absent.

Map V1 system to font.kind=system, segoe_ui to installed Segoe UI, and microsoft_yahei_ui to installed Microsoft YaHei UI. Preserve theme, scale, motion, Planet FPS, Status interval/rows, and HUD interval exactly. Language stays in presentation-v1.json.

The first successful explicit patch/reset creates V2 through the existing locked, reread-and-atomic-replace transaction pattern. Use the existing directory lock to serialize migration and updates. Recheck V2 under the lock before deriving V1 migration, so concurrent V2 creation cannot lose fields. Failed migration/write preserves original bytes, emits no committed event, and retains the committed view. Reads and opening Settings do not create files. Keep V1 as the rollback record; never silently down-convert new themes/fonts into V1.

A previous app build reads V1 and therefore sees the last V1 preferences. After V2 exists, new builds use V2; they do not resynchronize edits made by an older build. Record this downgrade boundary. Reset cannot overwrite corrupt or future documents. Appearance reset restores legacy dark, system font, and 100 percent; performance and language retain their existing independent boundaries.

Retain the positive safe-integer snapshot sequence, subscribe-before-read ordering, no rollback from stale events, same-bridge reload behavior, command-response application, and read-only HUD rules. Update Rust types, response/patch fixtures, generated TypeScript, closed decoders, fixture adapters, wire parity, and tests as one contract change. Do not hand-edit generated types.

## 5. Theme boundary

Final IDs are dark, light, system, catppuccin_latte, catppuccin_mocha, codex, and claude. Keep a closed catalogue with label key, light/dark scheme, and preview metadata. Store the chosen ID, resolve system to legacy light/dark, assign the resolved palette ID to data-theme, and assign only light or dark to CSS color-scheme. Listen for OS scheme changes only when system is chosen.

Use appearance.css as the shared palette-token owner for main and HUD. Use root-level semantic tokens so portal content inherits the same appearance. Add tokens for popup, highlighted option, selected option, control border, and disabled content. Every palette supplies every required token directly or through an explicit documented base. Do not let new light themes inherit dark danger fills or capsule colors accidentally.

Preserve risk meanings and the original five planet palettes. Use neutral settings emphasis rather than inventing a sixth operational mode accent. Audit titlebar, capsule, five modes, support pages, tables/treemap labels, errors, confirmations, and HUD. Theme changes affect colors, not routing, scans, sampling, or cleanup authority.

Forced colors override every palette after theme definitions, including portal/selected states. Reduced motion remains an accessibility override. Ordinary text must reach 4.5:1 contrast; large text and essential control/focus boundaries must reach 3:1 in their actual adjacent contexts. Do not rely on color alone for selection or status. Publish the measured token-pair report during implementation.

## 6. Scope and compatibility

Current-machine means the host running DevSweep. No remote font catalogue, sync, arbitrary theme editor, downloaded font, font-file import, new OS platform, or performance-setting expansion is planned. Other selection controls may adopt the new primitive later; this task changes Settings only. All existing languages, text scales, sampling bounds, save timing, and domain safety remain.

Change affected specs only during implementation. Required updates include the Settings navigation category, primary tab focus rules, font contract and discovery owner, V2 migration rules, palette IDs, token completeness, and allowed dependency choices.

## 7. Risk and evidence

| Risk | Required evidence |
| --- | --- |
| New tab bypasses drain/focus | Existing coordinator fake: success, rejection, stale requests, history, and Settings transitions |
| Popup clipped or off-screen | Open each control near viewport edges at the existing four widths and 125 percent text scale |
| Fonts are only apparent presets | Native catalogue count and successful search/selection across first, middle, last and available user-installed families |
| Font removal or invalid CSS breaks UI | Missing-family fixture; quoted/backslash/non-Latin family serializer tests; main/HUD fallback |
| Migration overwrites settings | V1/V2 precedence, invalid/future bytes, concurrent first saves, replacement failure, and restart tests |
| Named theme leaks old colors | Exhaustive token checks, contrast report, native main/HUD screenshots, and forced colors |
| New dependencies increase cost | Q1 approval, compatible versions, scoped imports/features, lockfile review, and recorded build-size difference |

Native evidence uses the real Windows build with isolated preference storage. Do not change the user's installed fonts, display scale, or saved settings for automated validation. Existing native-acceptance and operation-performance tasks retain their scope.
