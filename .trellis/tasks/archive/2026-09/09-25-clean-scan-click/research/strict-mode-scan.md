# StrictMode Clean scan repro

Date: 2026-09-25. Temporary test file `desktop/src/strict-scan-repro.test.tsx`
was removed after this run. Recreate the assertion as the permanent R1 test.

Command, from `desktop/`:

```powershell
mise exec node@22 -- npm exec -- vitest run src/strict-scan-repro.test.tsx
```

Vitest 3.2.7. Two cases, same zh-CN `App`, same `scanStart` spy, click on
the button named 扫描.

## Without StrictMode

Passed in 166ms. `scanStart` was called once.

## With StrictMode

Failed. After `findByRole` for 扫描, the test flushed two microtasks, then
clicked.

Observed, in order:

- `getByText("就绪")` passed, so the idle status was still on screen.
- `queryByRole("alert")` found nothing.
- `scanStart` was called 0 times.

```text
FAIL  src/strict-scan-repro.test.tsx > StrictMode dev mount keeps Clean scan alive
AssertionError: expected "spy" to be called once, but got 0 times
 ❯ src/strict-scan-repro.test.tsx:81:21
     79|   expect(screen.getByText("就绪")).toBeInTheDocument();
     80|   expect(screen.queryByRole("alert")).not.toBeInTheDocument();
     81|   expect(scanStart).toHaveBeenCalledOnce();
```

The first case passing and the second case stopping before `scanStart`
is the reported symptom: the Clean stage stays on 就绪 and no alert
appears.
