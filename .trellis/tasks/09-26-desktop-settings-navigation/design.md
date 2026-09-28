# Design: Settings Navigation

Read ../09-26-desktop-settings-experience/design.md section 1. This child owns shell navigation only.

Use a primary-navigation route type consisting of the available mode routes plus settings. Derive labels, DOM IDs, rendered order, tab stops, key movement, and focus refs from that list. Leave ModeRegistration and operational ModeId unchanged. Extend the navigation list instead of inserting a special unregistered button.

AppShell remains the route authority. Keep parseShellRoute and the settings hash. Route changes still call navigate and coordinator.cancelAndJoin. Generalize the existing success/failure focus restoration to primary destinations. Settings receives a matching tabpanel and selected tab association. Supporting pages retain their prior heading/back surface and a stable primary-navigation entry point.

Remove only the obsolete Settings menu/opener behavior replaced by the tab. Keep the brand menu keyboard rules and lifecycle rejection recovery. Retain full names at small widths and explicitly scroll a focused tab into the capsule viewport. Do not introduce a navigation library.

Expected files: desktop/src/app-shell/AppShell.tsx and tests, registry tests where assumptions change, desktop/src/styles.css, and the shell/component spec clauses. Reuse resources/i18n entries where labels already exist. Update catalogue sources only if an accessible label requires a new key.

Rollback: revert this shell change without modifying persisted settings or operational registries. See parent acceptance for integration with the new control layout.
