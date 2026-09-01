# AppShell focus restoration verification

Date: 2026-08-30

## Independent finding

The prior shell focused the hidden content heading after every route change. It did not restore the actual primary/supporting navigation control that initiated the transition, and Back from Settings did not restore the Settings opener.

## Deterministic repair

- Each typed navigation request now carries either the actual initiating `HTMLElement` or a stable typed destination-control intent.
- The winning request records its focus intent only after `cancelAndJoin` completes and its request sequence is still current.
- A layout effect consumes only an intent whose request and route match the committed composition.
- Primary and supporting pointer/keyboard clicks restore the actual initiating button. Locale accelerators resolve to the registered primary control.
- Back from Settings restores the captured Settings opener. A deep-linked Settings route with no opener falls back to the registered destination control.
- A disconnected, disabled, hidden, or aria-hidden activator is rejected and falls back to the route heading. Unavailable routes produce no focus request, and stale requests cannot install a focus intent.
- Existing route announcements, browser history, coordinator ordering, locale behavior, and feature availability semantics are unchanged.

## Changed product and test files

- `desktop/src/app-shell/AppShell.tsx`
- `desktop/src/app-shell/AppShell.test.tsx`

The focused AppShell suite contains 10 tests. New/updated assertions cover primary activator restoration, History/supporting restoration, Alt-key activation, Settings Back opener restoration, deep-link/no-opener fallback, and stale/unavailable transition isolation.

## Commands and results

| Command | Working directory | Exit | Result |
| --- | --- | ---: | --- |
| `rtk npm test -- src/app-shell/AppShell.test.tsx` | `desktop/` | 0 | 10/10 passed |
| `rtk npm run lint` | `desktop/` | 0 | passed |
| `rtk npm run typecheck` | `desktop/` | 0 | passed |
| `rtk just desktop-web-check` | repository root | 0 | 11 files, 78/78 tests, Vite build passed |
| `rtk git diff --check` | repository root | 0 | no whitespace errors before evidence-only additions |
| `rtk just ci` | repository root | 0 | fmt, offline dependency check, workspace check/tests, clippy passed |

Complete logs:

- `evidence/logs/focus-restoration-app-shell-focused.log`
- `evidence/logs/focus-restoration-desktop-web-check.log`
- `evidence/logs/focus-restoration-just-ci-full.log`
- `evidence/logs/focus-restoration-final-audit.log`

## Safety and evidence boundary

No GUI, store, cleanup, dependency, system setting, stage, commit, archive, push, signing, or release operation was invoked. The final read-only audit found zero DevSweep processes, zero desktop dev servers, and zero listeners owned by either. This deterministic browser-component repair adds no native evidence claim. The task's existing scaling, native High Contrast, and native Reduced Motion items remain `WAIVED/UNVERIFIED` and are not represented as PASS.
