# Implementation plan

## Preconditions

1. Confirm UX, typed-service, and operation-performance child reviews passed.
2. Freeze the release artifact, commit, fixtures, host, locale, and test data.
3. Read the parent implementation plan, archived resource protocol, and
   desktop acceptance/spec documents.

## Ordered work

1. Run automated format, lint, typecheck, tests, build, and Rust gates.
2. Prepare isolated native data and record host/build/fixture metadata.
3. Build the local package with `just desktop-build`; record the executable
   and NSIS installer hashes, version resources, unsigned status, and the
   running window/process/executable identity (design, package section).
   Do not run the installer.
4. Execute the mode, locale, width, keyboard/focus, cancellation, restart,
   unavailable, and error-recovery matrix. Capture the four WebView device
   scales through the launch argument and record the unchanged current
   Windows scale; do not change display settings.
5. Repeat resource/quiescence checks after normal completion and cancellation
   using the performance child's operation table and the same-live-PID
   desktop Status stop window.
6. Publish the matrix and route failures to the owning child; update the parent
   evidence links without changing product code here. Keep measured `fail`
   rows as `fail`.

## Validation

- `just ci`
- `just desktop-web-check` (runs the read-only
  `npm run types:generate -- --check`, lint, typecheck, test, and build)
- `just desktop-build`
- `git diff --check`
- Native manual matrix with screenshots, logs, process tree, and raw resource
  samples retained for every unavailable or failing row.
- Identity record: SHA-256, `VersionInfo`, `Get-AuthenticodeSignature`, and
  running window/process/path for the local build outputs.

## Risk and rollback

The main risk is confusing visual success with native lifecycle or safety
success. Keep evidence channels separate and stop at the first failed safety
row. There is no code rollback in this child; route implementation regressions
to the owning child and retain the frozen failing artifact.
