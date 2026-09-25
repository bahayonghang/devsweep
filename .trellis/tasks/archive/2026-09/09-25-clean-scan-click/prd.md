# Restore the desktop Clean scan button

## Goal

Clicking 扫描 on the desktop Clean stage starts a scan. The stage leaves
就绪. The same click reaches `scanStart` in the Tauri development window
that `just tdev` opens.

## Background

The reported window is the Clean stage: headline 「先逐项审核，再移动任何文件。」,
scopes 项目 and 全局缓存 checked, primary button 扫描, secondary text 就绪.
That is the idle stage rendered by `ScanPage` (`desktop/src/pages/ScanPage.tsx:123-141`).
The button calls `onScan` (`desktop/src/pages/ScanPage.tsx:100-102`).
`CleanWorkbench` passes `onScan={() => void scan()}`
(`desktop/src/modes/clean/CleanWorkbench.tsx:238`).
Default options include both scopes
(`desktop/src/modes/clean/CleanWorkbench.tsx:49`), and the initial reducer
phase is `idle` with `pending: null`
(`desktop/src/state/app-state.ts:69-70`), so the button is enabled.

`desktop/src/main.tsx:24-25` mounts `App` inside `React.StrictMode`.
Desktop React is 19.1.1 (`desktop/package.json`).
`App` registers one lifecycle effect whose cleanup calls
`lifecycleController.drain()` (`desktop/src/App.tsx:119-137`).
`drain()` calls `OperationCoordinator.close()` once and keeps that promise
(`desktop/src/lifecycle.ts:48-55`).
`close()` makes every later `start()` return `null` before `operation.start`
(`desktop/src/state/operation-coordinator.ts:32-35`).
`scan()` returns at `if (!lease) return` before `scan_requested`
(`desktop/src/modes/clean/CleanWorkbench.tsx:67-92`).
The `catch` stores an error only after `started` is set
(`desktop/src/modes/clean/CleanWorkbench.tsx:102-105`).
`command_failed` can store an error while phase stays `idle`
(`desktop/src/state/app-state.ts:274-282`), and `ErrorBanner` exposes it as
`role="alert"` (`desktop/src/components/ErrorBanner.tsx:40-41`).
The current closed-coordinator test expects no `scanStart` and does not
require an alert (`desktop/src/App.test.tsx:687-698`).

Repro, run from `desktop/` with Node 22, then deleted
(`research/strict-mode-scan.md`):

- `App` without `StrictMode`: click 扫描, `scanStart` is called once.
- `App` inside `StrictMode`, after the drain microtask: 就绪 remains, no
  alert, `scanStart` is called 0 times.

The planet canvas does not cover the button (`desktop/src/stage/styles.css:15-20`
sets `pointer-events: none`). The shell has no `data-tauri-drag-region`.

`just tdev` starts `npm run tauri -- dev` (`justfile:175-176`), which loads
this development mount. A Vite production bundle runs the effect setup once,
so an installed release window does not take this cleanup path.

## Requirements

### R1. Development mount starts Clean scan

Mount `App` the way `desktop/src/main.tsx` mounts it, including
`StrictMode`, zh-CN presentation settings, and route `#/clean`.
After the lifecycle effect has settled, clicking 扫描 calls the bridge
`scanStart` once and removes 就绪.

### R2. Real close still drains

A real React unmount still closes the shared coordinator and joins an
active Clean scan. The native close listener still awaits that same drain
before the test records window destruction. Route changes still cancel and
join through the existing shell adapter.

### R3. A closed coordinator is visible on the mounted Clean stage

When Clean stays mounted and `coordinator.start()` returns `null`, the
click does not call `scanStart`, `planDryRun`, or `planExecute`. The
existing `ErrorBanner` shows
`Desktop operation failed: The desktop operation coordinator is closed.`
The stage does not remain visually identical to idle 就绪.

### R4. The shared coordinator stays usable for another mode

On the same StrictMode mount, after the lifecycle effect has settled,
opening Status and activating its live control calls the status bridge.
The Clean fix must not be a Clean-only bypass around `OperationCoordinator`.

## Acceptance Criteria

- [ ] AC1 (R1): The StrictMode zh-CN test clicks 扫描, observes one
      `scanStart` call, and does not find 就绪.
- [ ] AC2 (R2): Existing App unmount, duplicate native-close, and late
      unlisten tests still prove one `close()` and joined cancellation.
- [ ] AC3 (R3): The pre-closed coordinator test observes no `scanStart`
      and finds the alert text named in R3. The same alert appears for a
      null lease from Clean dry-run and Clean execute while those controls
      are mounted.
- [ ] AC4 (R4): A StrictMode App test reaches Status and observes one live
      status start call.
- [ ] AC5: `desktop` lint, typecheck, and test pass with Node 22. No Rust
      command, generated IPC type, or catalogue file changes.

## Out of Scope

- Scan discovery, `roots: ["."]`, progress events, and `scan_start` IPC.
- Removing `StrictMode` from `main.tsx`.
- A new Rust `CommandError` variant or a change to `types.gen.ts`.
- Localizing `ErrorBanner`. Its current strings are English for every code.
- Software, Optimize, Analyze, and Status reducers. Their failure actions
  ignore an error unless `operationId` is already installed, and a null
  lease never installs one. Those silent leases stay as they are.
- A release-binary click failure. That window does not run this cleanup
  twice. Reproduce it separately if it remains after R1.
- Hit-testing, planet rendering, and window drag regions.

## Constraints

- Cleanup stays dry-run until an explicit saved plan, live digest, and
  confirmation. This task does not add a cleanup action.
- The coordinator still accepts only kind, id, cancel, and start. It does
  not receive plans, digests, or paths.
- Use the existing `{ code: "io", message }` value. `commandError()`
  already maps an `Error` to that shape.
