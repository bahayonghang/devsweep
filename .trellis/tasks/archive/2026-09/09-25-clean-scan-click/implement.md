# Implement: Clean scan click

## Checklist

1. In `desktop/src/App.tsx`, add the generation ref from `design.md` and
   move `lifecycleController.drain()` into the microtask guard. Leave
   `requestClose` on the native listener immediate.
2. In `desktop/src/modes/clean/CleanWorkbench.tsx`, at the scan, dry-run,
   and execute `if (!lease)` branches, dispatch `command_failed` when
   `mounted.current` is true, then return. Use the message in R3. Do not
   call the bridge.
3. Add the R1 test beside the existing desktop workflow tests in
   `desktop/src/App.test.tsx`. Mount `<StrictMode><App .../></StrictMode>`
   with zh-CN settings, route `#/clean`, and a `scanStart` spy. After the
   button named 扫描 appears, flush microtasks, click it, and expect one
   `scanStart` call and no 就绪.
4. Lock R3 in two seams. In `desktop/src/App.test.tsx` at the pre-closed
   coordinator test, click 扫描, expect no `scanStart`, and expect the R3
   alert. In `desktop/src/modes/clean/CleanWorkbench.test.tsx`, finish a
   scan with an open coordinator, call `coordinator.close()`, then click
   `Review dry run` and expect the alert with no `planDryRun`. Repeat
   from a completed dry run: close, confirm execute, expect the alert
   with no `planExecute`. The execute control is the confirm action in
   `ConfirmDialog`, reached only after a live dry-run digest exists.
5. Add the R4 test on a StrictMode `App`: open Status, click the live
   control, expect one status live start call. Override that bridge
   method with a spy so the fixture does not have to finish the stream.
6. Run the existing unmount and native-close tests. If an unmount
   assertion races the new microtask, wait until `close()` is observed.
   Do not put the drain back on the synchronous cleanup.
7. Run the validation commands below. Then update
   `.trellis/spec/desktop-frontend/state-management.md` under Operation
   Coordinator: effect replay and effect dependency changes do not close
   the coordinator still held by `App`; a Clean null lease while mounted
   is the R3 alert; native close still drains immediately.

## Validation

From `desktop/`, with Node 22:

```powershell
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
```

`npm run types:generate` and `just ci` stay unused. This change has no
Rust, catalogue, or generated IPC diff.

Manual check after the tests pass: `just tdev`, Clean, zh-CN, click 扫描.
The stage leaves 就绪 and shows the scanning stage. Stop the dev window
afterward.

## Rollback point

The safe stop is after step 2 if the tests in steps 3–5 are not written
yet: the button can work while the regression is still absent. Do not
archive there. AC1–AC4 are the stop that can be archived.

## Files

- `desktop/src/App.tsx`
- `desktop/src/modes/clean/CleanWorkbench.tsx`
- `desktop/src/App.test.tsx`
- `.trellis/spec/desktop-frontend/state-management.md`

Do not edit `desktop/src/main.tsx`, `desktop/src/lifecycle.ts` close
semantics, mode reducers, or `desktop/src/api/types.gen.ts`.
