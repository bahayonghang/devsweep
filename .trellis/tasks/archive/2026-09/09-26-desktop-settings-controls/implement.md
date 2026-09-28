# Implementation Plan: Settings Controls

Prerequisites: navigation child complete, Q1 resolved, parent plan approved, child activated.

- [ ] Pin the approved Base UI release; verify React 19 support and license; update the lockfile through the normal package workflow.
- [ ] Implement typed Select/Combobox wrappers and shared popup/control tokens.
- [ ] Apply the documented Settings row layout and existing-theme radio previews.
- [ ] Preserve save/disabled/reset behavior and independent language persistence.
- [ ] Add focused interaction tests: keyboard, Escape, outside click, IME, long labels, saving, failure, and focus restoration.
- [ ] Visually inspect open popups at viewport edges in both existing schemes, both locales, forced colors, and text-scale extremes.
- [ ] Update component guidance for the chosen accessible primitive and approval record.

Focused commands from desktop: npm test -- src/preferences/preferences.test.tsx plus the new selection component test path, then npm run typecheck and npm run lint for changed code. Parent owns the final integrated desktop gate. Record C-AC1 through C-AC5 evidence; do not use screenshot-only evidence for keyboard behavior.
