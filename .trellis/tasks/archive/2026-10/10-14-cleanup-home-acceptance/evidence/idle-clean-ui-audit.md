# Read-only Clean idle source audit

The parent reported that the final 548-second native run had low host-process CPU but a hot owned WebView child. The child appeared after roughly 120 seconds and later windows showed 70–82% of one core. This source audit does **not** turn that observation into an idle PASS, identify the hot process role, or prove a runtime cause. Native process attribution is being investigated separately.

Scope: initial Clean route with no configured roots, no scan, and no prior navigation. Only source and static asset bytes were read. No product changes, build, test, browser, native launch, or comparison measurement was performed for this audit.

| Surface | Source observation |
| --- | --- |
| `CleanSession.tsx:7` | Holds reducer state, scan-scope preferences and advisory preferences. Recreated callback/context objects do not appear in a self-triggering effect dependency. Reducer dispatch is stable. |
| `useScanScope.ts:19` | Initialization depends only on the stable bridge object. One preferences read settles roots/suggestions/loading/error. Root validation/persistence and invalidation run only from explicit edit handlers. No timer or retry loop. |
| `useCleanAdvisory.ts:15` | Threshold preferences are read once per bridge identity. Saves run only from the form handler. |
| `useCleanAdvisory.ts:40` | Projection immediately returns when the report is null. Otherwise it depends on bridge, the retained report object and numeric threshold; it does not modify those inputs. Initial no-scan idle invokes no projection. |
| `CleanWorkbench.tsx:45` | Layout effect only copies slots into a ref. The `[coordinator, dispatch]` effect registers unmount cleanup. `hidden` dispatch happens in cleanup, not on each render. Scan/preview/execute/protection work is initiated by handlers. |
| `CleanOverview.tsx`, `ScanRoots.tsx` | Render state and controls; no effects, timers or animation-frame scheduling. At initial idle there is no owner list or report traversal. |
| `ConfirmDialog.tsx:10`, `ProtectDialog.tsx:23` | Effects depend on the open boolean and conditionally open/close native dialogs. Initial dialogs are closed. New handler identities do not retrigger their effects. |
| `App.tsx:94`, `main.tsx` | Coordinator/controller identities are memoized; production entry calls its render function once and supplies one `userLocales` array instance. Presentation loading depends on stable bridge/locales. Lifecycle installs listeners once per controller identity. |
| `AppShell.tsx:474`, `registry.ts` | Only the active mode's render factory is called. Other mode components and Planet are imported but not mounted on initial Clean. Event-listener effects without dependency arrays reattach on existing renders; they do not themselves schedule another render while Clean remains a valid route. |
| `preferences/store.ts:23` | One initial subscription/read, later reads on visibility changes, and updates on explicit settings events. Appearance depends on stored preferences and media-query values and does not write the native store. |
| `WindowTitlebar.tsx`, `lifecycle.ts:88` | Stable controller and `useSyncExternalStore` snapshot (`getSnapshot` returns `this.state`). Connection registers native resize and reads maximized state, with a second read after registration. No polling loop or window resize command is issued from rendering. |
| `stage/Planet.tsx:40` | The production continuous `requestAnimationFrame` loop is inside the Planet mount effect. Clean does not mount it. Other imported renderer modules have no repeating module-level work. |

The production-source timer/animation search found no Clean/session timer, interval, idle animation, or RAF. The other matches are the unmounted Planet, event-driven media/visibility code, fixture delays, and non-Clean components. CSS for the initial Clean surface has no continuous animation. The logo is a static PNG: chunks `IHDR`, `IDAT`, `caBX`, `IEND`; no APNG `acTL` chunk.

## Remaining uncertainty

No self-sustaining React update loop was identified from this inspection. That is a source finding, not measured proof of idle behavior. A native `resize`, `desktop-preferences-changed`, media, or visibility event flood could cause updates; the frontend subscribes to those external events, and this audit did not count them. It also did not inspect WebView utility/GPU/renderer internals, physical composition, or Chromium scheduling.

The earlier home performance harness intentionally omitted CleanSession and the shared shell, so its 14 ms component measurements cannot resolve this native idle issue. The parent has paused any new Chrome/full-App comparison pending the hot child's command-line role and old-binary control. No speculative product change is justified by the present source evidence.
