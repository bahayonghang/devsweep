# Native Earth acceptance

## C2 candidate: native lifecycle and corrected visual matrix passed

The unchanged C2 executable SHA-256 is
`5ffaefe242808e9a2074880667209a0ece6facef1b15ab5f782adf36cc2f2a59`.
Its source archive is
`5420ec3087c8b4b61514cc3289484dccd5c0ee149a07661acbf45add8a5ec566`.
The isolated launch `dist/native-clean-c2-final-01/` used PID 115364,
FILETIME `134363836577642399`, main HWND 49549940 and port 52173. Actual
Windows DPI was 120 (125%), WebView DPR 1.25 and viewport 1080 by 720 CSS pixels.
The engine identified itself as `Edg/155.0.4283.45` with V8 `15.5.9.4`.

`dist/native-earth-c2-final-01/cycles.json` records twenty completed actual
route/unmount and minimize/restore cycles. Every unmount released all owned
textures, buffers, programs and shaders; pending RAF and later draws were zero.
Every exact HWND became minimized, the native visibility command returned false,
and the paused interval had zero draw/callback changes and zero pending RAF.
The document remained `visible` in all twenty cases, demonstrating that the
native lifecycle bridge supplies the missing signal. Restore returned native
visibility true and resumed rendering with one texture, one buffer, one program
and two shaders. No fake visibility event or replaced bridge supplied results.

That first driver run subsequently failed a harness assumption: the 800px-wide
responsive layout uses a 168px CSS globe, so DPR 2 correctly gave 336 backing
pixels, not the assumed desktop 464. The failure is preserved. The subsequent
`-02` run reused only the already completed twenty-cycle record from the same
still-running PID/FILETIME/executable, verified by the driver. Its
`reused-cycles.json` records source and SHA-256
`fa469e28bd9427d67a4931a557ab1947e4d245d3e3889bedc3855df136e288f3`.
The corrected 1080px resize/DPR still-pose check passed, as did forced colors
without Earth resource requests or draw/RAF work. The machine's reduced-motion
setting remains untouched; animation was explicitly emulated in this owned
WebView, with original media and restoration recorded.

The second driver generated all 32 EN/zh-CN, width and DPR captures, with no
horizontal overflow, a ready renderer and bounded backing dimensions. However,
a later audit found stale backing dimensions on several sequential DPR-only
changes (for example 348 instead of 464 at width 1440/DPR 2). Its automated
matrix assertion checked only the cap and therefore does not establish all
32 sharpness cases. That original matrix remains superseded, not silently
relabeled. The earlier still-pose check changed viewport geometry too and is
not substituted for DPR-only evidence.

The subsequent same-component probe fixed the viewport at 1080 by 900 and
changed only emulated DPR through 1, 1.25, 1.5, 2, 1.25 and 2, observing after
50 ms and one second per transition. `devicePixelRatio` and resolution query
`matches` changed correctly, but this native CDP path emitted **zero** resolution
media change events and **zero** window resize events. The application query
remained registered and its callback was never invoked; exact and rounded
resolution queries both matched the current DPR. Thus this probe did not show
a failed observer rearm. It demonstrated absent emulation notifications and
does not claim a real Windows monitor/scale transition was tested. The full
query-creation/event ordering trace is
`dist/native-earth-c2-dpr-01-probe/dpr-only-probe.json`. No production polling,
synthetic resize event, or source fix was added to conceal this distinction.

`dist/native-earth-c2-final-matrix-03/` then captured all 32 cases using an
independent fresh mount per width/DPR and asserted exact backing dimensions,
not just the 512px cap. EN and zh-CN both passed at 390/800/1024/1440 CSS widths
and DPR 1/1.25/1.5/2. The respective CSS globe sizes are 136/168/232/232px;
their DPR-2 backings are 272/336/464/464px. All cases have a ready renderer,
zero horizontal overflow, zero pending RAF in reduced motion, and preserved
Segoe UI Variable font selection. The actual host remains DPI 120/DPR 1.25;
other matrix scales are explicitly emulated. No OS display setting was changed.
Recognizable geography and readable fixture-root controls were visually
inspected in the narrow English and wide Chinese captures. Neutral-pointer
captures keep the separately recorded hover issue visible in its own evidence.

This second isolated app used PID 128400, FILETIME `134363847810008726`,
HWND 29235668 and port 52391, with the identical C2 executable. Only task fixture
root preferences were seeded from the first isolated run. One normal WM_CLOSE
ended it with exit 0, no timeout or forced teardown and no owned leftovers;
helpers settled in 1437.577 ms. Its `--matrix-only` result correctly records
zero new lifecycle cycles; twenty-cycle evidence remains bound to the first
launch rather than being invented a second time.

[native-earth-c2.json](native-earth-c2.json) retains both launch identities,
qualified acceptance results, the memory windows and hashes of raw evidence.
Production timing, scan responsiveness and no-debug process budgets remain
separate runtime/renderer measurements.

### Twenty-cycle resource assessment

`memory-windows.json` records actual owned process-tree totals while Planet is
unmounted, five-cycle windows and browser-reported JS heap, without forced GC.
These are instrumented lifecycle observations, not the separate idle CPU or
working-set acceptance gate. The observed memory settles into a bounded band
over this run: five-cycle medians are not monotonically increasing, late private
memory stays below the first-window maximum, JS heap repeatedly falls after
growth, and handles/threads finish below their initial window. There is no
observed sustained per-cycle accumulation; this is a bounded twenty-cycle
observation, not a claim about indefinite runtime or precise GPU memory bytes.

| Cycles | Tree private median (MB) | Working set median (MB) | Handles median | JS heap median (MB) |
| --- | ---: | ---: | ---: | ---: |
| 1–5 | 288.428 | 542.314 | 3738 | 8.952 |
| 6–10 | 278.434 | 544.068 | 3715 | 9.388 |
| 11–15 | 278.057 | 541.614 | 3720 | 9.315 |
| 16–20 | 282.329 | 545.124 | 3717 | 9.369 |

The final five cycles span 277.815–288.674 MB private memory, 3713–3717 handles
and 8.153–10.098 MB JS heap (decimal MB). The root app threads fall from 27 on
cycle one to 24 for cycles seven through twenty; process-tree thread totals are
larger because they include WebView helpers. A separate debug-enabled startup
diagnostic after 66.6 quiet seconds found 24 root threads. Formal no-debug
resource acceptance belongs to the runtime child.

### Known shell limitation and normal closure

The native screenshot exposed an existing selected-tab hover contrast bug:
`Clean` text is present, but the hover selector makes foreground and background
both `rgb(238, 241, 247)`. Moving the synthetic pointer off the tab restores
foreground `rgb(18, 23, 42)`. Font remains Segoe UI Variable and forced colors
is false; this is not a media/font artifact. The diagnosis and neutral-pointer
capture are in `-01/active-tab-diagnostic.json`, `active-tab-hover-diagnostic.json`
and `active-tab-not-hovered.png`. The coordinator kept this pre-existing shell
issue outside the agreed source scope; no CSS was changed to hide it.

One normal WM_CLOSE ended the exact app with exit 0, no timeout or forced
teardown and no owned leftovers; helpers settled in 1688.696 ms. Raw closure
records are in `dist/native-clean-c2-final-01/`. No owned app remains running.

## Historical C1 candidate: minimize lifecycle failed

The packaged C1 executable was
`dist/performance/cleanup-earth/C1-runtime/devsweep-desktop.exe`, SHA-256
`8f917025fefc66e40bb7a04591cce1b092b4901ae4597c161778fe07381efc72`.
The driver attached only to the isolated launch recorded in
`dist/native-clean-c1-final-01/launch.json`: PID 96876, FILETIME
`134363822986252072`, main HWND 19208558, debugger port 51991. Native window
DPI was 120 (125%); actual WebView DPR was 1.25 at 1080 by 720 CSS pixels.

`tools/native-earth-smoke.mjs` instruments the actual packaged WebView's RAF
callbacks and WebGL object/draw calls. These observations are separate from
uninstrumented resource measurements. It preserves actual process ownership,
offline origin, screenshots, browser identity, and per-cycle raw observations.

The first run, `dist/native-earth-c1-final-01/`, failed its animated-rendering
prerequisite because this machine's WebView reports reduced motion enabled.
Its single still draw, zero RAF callbacks and zero pending callbacks are
expected reduced-motion behavior. This is a method prerequisite diagnostic,
not an animation regression. The revised driver records the original media
state, temporarily emulates `prefers-reduced-motion: no-preference` only in its
owned WebView, and restores media emulation afterward. It changes no OS or
persistent user preference.

The retry, `dist/native-earth-c1-final-02/`, established non-vacuous animation
and passed the first route-unmount resource-release check. Its first actual
minimize then failed after 30 seconds: `document.visibilityState` remained
`visible`. No complete lifecycle cycle or 32-case visual matrix is claimed.
`failure.json`, `failure-page.json`, `initial.json`, and screenshots retain the
raw observations, including one live RAF after minimization.

An independent exact-window check confirmed a product defect rather than a
failed minimize action. `dist/native-clean-c1-final-01/minimize-diagnostic.json`
records HWND 19208558 as not minimized before `ShowWindow(..., 6)` and minimized
afterward (`IsIconic`), with identical root PID and creation identity. In
`minimize-page-diagnostic.json`, two samples 1506 ms apart remained
`document.hidden=false`, `visibilityState=visible`; draw count increased
910 to 954 (+44) and RAF callback count 4541 to 4758 (+217). Native visibility
must therefore participate in the decorative-renderer lifecycle.

The owned application was restored and closed normally with one WM_CLOSE.
`dist/native-clean-c1-final-01/explicit-close.json` and `summary.json` record
exit 0, no timeout or forced teardown, no owned leftovers, and natural WebView
helper settlement in 1261.566 ms. No test process remains owned by this driver.

At this checkpoint, the native publication and frontend consumption fix was in
progress. The corrected packaged candidate still needed to prove all
20 route/minimize cycles, zero drawing/RAF while minimized, resume behavior,
resource release, reduced motion, forced colors, and the full visual matrix.
Earlier production-browser matrix results do not substitute for this gate.
[native-earth-c1-failure.json](native-earth-c1-failure.json) preserves the compact
window/draw proof, normal-close result, and raw artifact hashes. The prepared C2
driver checks the real HWND's minimized state and the shipped read-only
`main_presentation_visibility` command before observing zero pending RAF and
stable draw/callback counts; DOM visibility remains recorded without being a
prerequisite. At the C1 checkpoint this preparation had only a Node 22 syntax
pass; the subsequent C2 native results are recorded above.
