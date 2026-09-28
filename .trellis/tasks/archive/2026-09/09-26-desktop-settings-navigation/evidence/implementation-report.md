# Settings Navigation Implementation

Date: 2026-09-26. Child: desktop-settings-navigation. Status changes remain with
the parent session. No production dependencies were added.

## Result

Settings is the final primary tab after Status. The operational ModeId set
remains Clean, Software, Optimize, Analyze, and Status. A typed primary route
list owns labels, tab IDs, focus references, order, selected state, and keyboard
movement. Settings remains available when only one mode is registered.

The active primary panel is named by its tab. Settings retains its visible title
and subtitle. Settings has no duplicate brand-menu entry or last-mode back
button. Protection, Rules, History, Help, and supporting back controls retain
their behavior. The selected primary tab owns the tab stop. Supporting pages
retain a tab stop for the last available operational mode.

Arrow/Home/End movement brings the focused tab into the capsule viewport without
changing routes. Enter and Space use the existing cancel-and-join transition.
Settings and operational routes restore their destination tab on history
navigation; supporting routes restore the brand button. A newer transition to
the still-active route supersedes a pending Settings transition. Failed drains
retain the current route and expose the existing error.

## Changed files

- desktop/src/app-shell/AppShell.tsx: primary navigation, panel association,
  history/focus restoration, menu/back removal, and pending-transition guard.
- desktop/src/app-shell/AppShell.test.tsx: six-tab order, both locales, all
  keyboard paths, selected panel, scrolling, history, support tab stop, failed
  drain, and stale Settings requests.
- desktop/src/App.test.tsx: Settings tab test helper, rejection focus target,
  and a regression proving direct Settings entry starts no Status sampling.
- .trellis/spec/desktop-frontend/component-guidelines.md: primary Settings
  navigation, keyboard, focus, menu, and panel contract.
- .trellis/spec/desktop-frontend/state-management.md: route ownership and
  stale-transition contract.
- .trellis/spec/desktop-frontend/index.md: six navigation destinations while
  retaining five operational ModeIds.
- This report and browser-navigation.json: focused implementation evidence.

No CSS edit was needed. The existing nowrap tab labels and horizontal capsule
overflow support the extra destination at every required width.

## Checks

Commands ran from desktop with Node 22 through mise.

| Check | Result |
| --- | --- |
| mise exec node@22 -- npx vitest run src/app-shell/AppShell.test.tsx src/app-shell/registry.test.ts src/App.test.tsx | 3 files, 70 tests passed |
| mise exec node@22 -- npm run lint | Passed |
| mise exec node@22 -- npm run typecheck | Passed |
| mise exec node@22 -- npm run build | Passed; Vite 7.3.6, 148 modules |
| git diff --check | Passed; only local LF/CRLF conversion notices |

The planned npm test -- file arguments run the full Vitest suite because the
package test script chains a Node test runner. The initial attempt exposed the
displaced Settings-menu helper in App.test.tsx. That helper was updated, and all
38 App tests passed in the focused command above. Use the direct Vitest command
for this child. Parent integration retains ownership of the full final gate.

ESLint required declaring navigate before effects that call the function. The
unavailable-registration effect schedules its route correction in a guarded
microtask. Cleanup discards a superseded or unmounted correction.

## Browser evidence

browser-navigation.json records real Tabbit Chromium DOM measurements with
the Vite fixture bridge. Native browser keyboard events exercised Clean -> End
-> Settings focus -> Enter activation at 390, 800, 1024, and 1440 CSS pixels in
English and Simplified Chinese. All eight combinations passed:

- Six tabs appeared in the required order with one selected Settings tab.
- End moved focus to Settings while Clean remained selected.
- Settings stayed within the capsule viewport without label clipping.
- No document-level horizontal overflow occurred.
- Enter activated the labelled Settings tabpanel.

At 390 px, English Settings occupied x=299.875..375.700 within a
10..380.400 capsule; capsule scrollLeft was 204.800. Chinese Settings occupied
x=317.325..375.325; capsule scrollLeft was 113.600. At wider sizes capsule
scrollLeft was zero.

The first fixture-server port, 1437, was refused by Windows with EACCES. The
same server succeeded on 5173. The task-created browser tab and Vite process
were closed after evidence collection. A screenshot request timed out after
30000 ms after fonts loaded. No screenshot or pixel-level visual acceptance
is claimed. Native WebView2/scaling and final palette/control screenshots
remain parent integration evidence.

## Pre-Base-UI production asset baseline

These are generated desktop/dist outputs after navigation, before the approved
control dependency is installed. Generated outputs remain untracked/ignored.

| Asset | Bytes | Build-reported gzip kB |
| --- | ---: | ---: |
| main-Onnrsve7.js | 173938 | 43.00 |
| state-DwIN7l5n.js | 489798 | 109.44 |
| hud-DqT_Nx8e.js | 2686 | 1.18 |
| main-yPMWXmFg.css | 50092 | 9.17 |
| hud-gFW-Biqh.css | 3515 | 1.30 |
| devsweep-icon-master-Bn2Ejx_Y.png | 956579 | not reported |

## Acceptance and follow-up

- N-AC1: both-locale tab/panel tests and eight browser combinations passed.
- N-AC2: keyboard, tab stop, and scroll tests passed; real 390 px geometry passed.
- N-AC3: direct Settings, actual Back/Forward, rejection, canonical hash, and
  superseded Settings transition tests passed.
- N-AC4: existing App integration paths passed; direct Settings starts neither
  statusSnapshot nor statusLiveStart. Operational registries and reducers did
  not change.
- N-AC5: menu tests retain Protection, Rules, History, Help; supporting back
  behavior passed. Settings duplicates are absent.

Ready for the independent navigation check. Controls, host fonts, palettes,
native visual evidence, and the parent integration gate remain later work.
