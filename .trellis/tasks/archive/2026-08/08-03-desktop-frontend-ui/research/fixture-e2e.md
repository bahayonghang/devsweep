# Desktop fixture workflow

The UI workflow is exercised with an injected `DesktopBridge`; this is browser
fixture automation, not real Tauri E2E and not cleanup execution evidence. No
Tauri cleanup command or filesystem mutation occurs. Archived fixtures under
`desktop/src/api/fixtures/` are decoded through the production contract layer.

Automated sequence:

1. Start scan, receive indeterminate progress, request cancellation, and observe
   a structured cancellation failure.
2. Rescan and project conservative default selection while disabling the
   inspect-only target.
3. Run a dry run, return to review, change selection, and verify the preview and
   digest disappear immediately. The one-target preview reports 500 MB; the
   two-target preview has a different digest and reports 500 MB plus an at-least
   128 MB estimate.
4. Repeat dry run, open the native confirmation dialog, execute with the exact
   backend digest, and render per-target success/skipped details.
5. Separately inject `scan_already_running`, `stale_confirmation`, and
   `unknown_target` to verify recovery-oriented messages.

Run the automated checks with:

```powershell
mise exec node@22 -- npm test
```

For a manual browser replay, start the dev-only fixture bridge:

```powershell
mise exec node@22 -- npm run dev:fixture
```

Open `http://127.0.0.1:4180`. The first scan waits for `Cancel scan`; the second
scan returns the archived targets and enables the remainder of the flow. Normal
`npm run dev` still uses the real Tauri bridge.

## Final command results

- Type generation: 7 named fixture files generated deterministically through
  pinned `quicktype-core`.
- ESLint: passed with no findings.
- TypeScript: `tsc --noEmit` passed.
- Vitest: 6 files, 31 tests passed.
- Vite build: 43 modules transformed; production bundle built successfully.
- Fixture dev server: startup smoke returned HTTP 200 on `127.0.0.1:4181` and
  the test-owned server was terminated afterward.
- Desktop Rust tests: 15 passed, 2 fixture entrypoints ignored.
- Repository `just ci`: 70 CLI/TUI, 161 core, 1 public API, and 15 desktop
  tests passed; fmt, check, and clippy with warnings denied passed.

## Main-session browser replay

The main session replayed the full fixture flow at `127.0.0.1:4180` after the
automated gates:

- The first scan showed indeterminate phase/message progress and was canceled;
  the UI returned to ready state with a recovery-oriented error.
- The second scan produced three targets. The Cargo target was selected by
  default, the Cargo home target was disabled as inspect-only, and the command
  target was visibly irreversible.
- The first dry run selected one target, reported 500 MB, and returned digest
  `014349a17bf4a3887a794e595fbf29d1d4ecb5754c27c746c2333b52d0e63388`.
- Returning to review removed the preview immediately. Selecting the command
  target and repeating dry run selected two targets, reported
  `500 MB + at least 128 MB`, and returned a different digest,
  `9ad10cc52be4f63b75dcfa34c837260a5118f931229893fec4c90dfac93d7e1a`.
- The confirmation dialog displayed the second digest and irreversible warning.
  The fixture execution report retained both target outcomes and recycle-bin
  capacity wording.
- At 800 x 600 and 390 x 844, the page had no body/root horizontal overflow;
  the narrow viewport measured 390 px for client and scroll width. Browser logs
  contained no warnings or errors.

The browser tab, viewport override, and task-owned fixture server were cleaned
up afterward. Final checks reported zero scoped processes and zero listeners on
port 4180. This remains fixture replay evidence only; no real cleanup ran.
