# Design: Settings Controls

Follow ../09-26-desktop-settings-experience/research/design-proposal.md and parent design section 2. The proposal uses @base-ui/react Select/Combobox. The user approved both named dependencies on 2026-09-26; installation belongs to implementation.

Local typed wrappers own visual anatomy, label/description associations, popup positioning, selected/highlight markers, and disabled states. Keep application preference updates in SettingsPage and the existing store. Use controlled committed values; keep input query and highlighted candidate state internal to the picker. Do not commit from a highlight callback or React effect.

Render popups through the library portal into the document root. Disable overlapping Select alignment; use the proposed offset, viewport collision handling, and scroll limits. Root appearance tokens must reach the portal. Style keyboard focus and selection separately. Native radio inputs own theme-tile semantics; only actual radio selection changes the theme.

Use the existing radius, text-scale, save-state, reset, and effective-motion policies. Font rows use the UI font; the dedicated preview uses the selected family. Async font loading/unavailable props must not disable unrelated settings. The later fonts child owns catalogue data and native bridge calls.

Review Base UI's approved release for React 19 compatibility, production size, licensing, IME support, and WebView2 operation. Use one library for Select/Combobox, not separate menu libraries. No additional virtualizer is proposed without measured need and explicit approval.

Expected files: desktop/src/preferences/SettingsPage.tsx; a small focused selection component module under desktop/src/components; desktop/src/styles.css; preferences tests; package.json/package-lock.json after Q1; resources/i18n/en.json and zh-CN.json only for new user-facing labels. Update the affected component/type-safety guidance. Do not hand-edit generated catalogue output.

Rollback returns the component dependency and wrappers together. Existing committed preferences and values remain valid.
