# Strict presentation-store identity repair verification

Date: 2026-08-30

## Independent finding and repair

The independent check found that changing either the `presentationSettings` bridge identity or the `userLocales` array identity started a replacement load in an effect while the previous `ready` state remained renderable until the effect ran. That allowed an old `.app-shell` and old `[data-locale]` to remain visible during the replacement resource's first paint.

The repair makes the presentation state carry the exact bridge and locale-array identities that produced it. Render compares those identities with the current props before inspecting the load status. A mismatch renders the loading gate immediately, so the old shell cannot render while the replacement load is pending. Load and save settlements retain the generation guard; their resulting state is also tied to the originating resource identity. A rejected replacement load therefore remains on the bilingual unavailable surface, and a late old load or save cannot revive an old locale.

No synchronous state update was added to the loading effect. A stable default locale-array constant prevents the default prop from creating a replacement resource every render.

## Changed product and test files

- `desktop/src/App.tsx`
- `desktop/src/App.test.tsx`

The focused test suite now has 22 tests (previously 18). The four identity-lifecycle regressions prove:

- a ready shell disappears immediately when the bridge identity changes, and replacement rejection remains unavailable;
- a late settlement from the old load cannot revive the old locale;
- a late settlement from an old save cannot revive the old locale;
- a `userLocales` identity change is gated until its successful load, after which successful `null` store state may resolve the new supported OS locale.

## Commands and results

All commands ran from the repository root unless noted.

| Command | Working directory | Exit | Result |
| --- | --- | ---: | --- |
| `rtk npm test -- src/App.test.tsx` | `desktop/` | 0 | 1 file, 22/22 tests passed |
| `rtk npm run lint` | `desktop/` | 0 | ESLint passed |
| `rtk npm run typecheck` | `desktop/` | 0 | TypeScript check passed |
| `rtk just desktop-web-check` | repository root | 0 | lint, typecheck, 11 files/76 tests, and Vite production build passed |
| `rtk git diff --check` | repository root | 0 | no whitespace errors before the final evidence-only update |
| `rtk just ci` | repository root | 0 | format, offline dependency update check, workspace check/tests, and clippy with warnings denied passed |

Complete final logs are preserved at:

- `evidence/logs/strict-store-identity-app-focused.log`
- `evidence/logs/strict-store-identity-desktop-web-check.log`
- `evidence/logs/strict-store-identity-just-ci-full.log` (27,193 bytes)

## Safety and residue

- The focused React tests use injected bridges and do not invoke a real presentation store, create a lock, or write user/global state.
- Final read-only process/listener inspection found zero `devsweep` processes and zero listeners owned by a `devsweep` process.
- A broad read-only temp inventory saw pre-existing historical names containing `devsweep` or `presentation`; none was attributed to this mocked repair, modified, or removed.
- No GUI was started. No display/system setting, cleanup, dependency, stage, commit, archive, push, signing, or release operation was performed.

## Remaining evidence boundary

This repair adds no new `UNVERIFIED` item. Previously approved scaling, native High Contrast, and native Reduced Motion observations remain `WAIVED/UNVERIFIED` exactly as recorded by the task; they are outside this deterministic store-identity repair and are not represented as PASS.
