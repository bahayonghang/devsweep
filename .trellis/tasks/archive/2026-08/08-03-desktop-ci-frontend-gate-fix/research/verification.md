# Desktop CI Frontend Gate Verification

Date: 2026-08-03

## Finding And Fix

The Windows Desktop job invoked `npm run check`, but `desktop/package.json`
defines no `check` script. The job would fail before compiling the Tauri app.
The workflow now executes the current explicit frontend gate after `npm ci`:

1. `npm run types:generate`
2. `npm run lint`
3. `npm run typecheck`
4. `npm test`

The following `npm run tauri -- build --no-bundle` step is unchanged. Tauri's
`beforeBuildCommand` still runs `npm run build`, so the production frontend
build remains part of the desktop compile.

## Verification

- `mise exec node@22 -- just desktop-web-check`: PASS; deterministic generation
  from 7 fixtures, ESLint, TypeScript, 31 Vitest tests, and Vite build.
- Structured `desktop/package.json` script lookup: PASS for
  `types:generate`, `lint`, `typecheck`, `test`, and `tauri`.
- `python ./.trellis/scripts/task.py validate`: PASS with 2 implementation and
  2 check context entries.
- `git diff --check`: PASS.
- Independent `trellis-check`: PASS with no unresolved workflow finding.

Hosted GitHub Actions were not run because the goal forbids pushing, and
`actionlint` is not installed locally. No remote-CI completion is claimed.
