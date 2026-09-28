# Implementation Plan: Settings Navigation

Prerequisite: approved parent planning summary; child status in_progress before code changes.

- [ ] Read AppShell route, drain, focus, brand-menu, and tab tests.
- [ ] Add the typed primary-navigation list and Settings tab/panel association.
- [ ] Extend roving focus, hash/history restoration, scroll-into-view, and failure recovery through the same list.
- [ ] Remove only the displaced Settings menu entry/opener/back-header code.
- [ ] Update affected component/state specs; preserve five-mode language for operational surfaces.
- [ ] Run focused AppShell and registry tests, including coordinator rejection/stale transition cases.
- [ ] Check 390/800/1024/1440 widths and both locales on the Settings/navigation surface.

Focused command from desktop: npm test -- src/app-shell/AppShell.test.tsx src/app-shell/registry.test.ts. Parent owns the final integrated desktop gate. Record N-AC1 through N-AC5 evidence in the child. No settings migration or product dependency belongs to this child.
