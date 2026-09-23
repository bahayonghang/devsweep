# Implement — staged child execution plan

## Preconditions

- Keep the parent and every child in `planning` until the user approves the
  latest final planning summary.
- Run `task.py validate` recursively and complete the independent plan review
  before any `task.py start`.
- Curate real `implement.jsonl` and `check.jsonl` entries for every dispatched
  child; placeholder manifest rows do not count.
- Read the desktop/backend spec indexes in the child context before editing
  product files. No child may add a production dependency, publish, push,
  archive, or alter unrelated dirt.

## Ordered execution

### 1. `desktop-tauri-cli-service`

1. Freeze the shared typed-service boundary in the backend and desktop specs.
2. Trace each existing CLI operation to its core owner and record any small
   extraction needed for parity; keep the CLI crate private to the binary.
3. Align Tauri request/result/error envelopes, progress/event sequence, and
   cancellation joins with the generated frontend contract.
4. Add cross-surface fixtures for Clean, Analyze, Software, Optimize, Status,
   locale invariance, stale ids, structured errors, and inspect-only refusal.
5. Run focused Rust and desktop contract tests, then stop for review if a
   command/schema change is required.

### 2. `desktop-mole-workbench-ux`

1. Freeze the original visual tokens and copy/provenance rules before JSX/CSS.
2. Refine AppShell and the five mode pages within the existing registry,
   reducers, bridge, and coordinator seams.
3. Replace generic/light fallback surfaces with consistent dark cards, grouped
   rows, stable action bars, evidence disclosure, and clear empty/loading/error
   states. Keep authority copy and selection semantics intact.
4. Verify keyboard focus, route/back restoration, accessible long data,
   reduced motion, forced colors, and EN/zh-CN at 390/800/1024/1440 CSS px.
5. Run frontend gates and record visual evidence; do not claim native scaling
   until the native child captures it.

### 3. `desktop-operation-performance`

1. Reproduce current operation lifecycle and render/resource baselines before
   mutation.
2. Fix only measured backpressure, duplicate serialization, stale-event, or
   cancellation/join defects; preserve the coordinator's single-flight seam.
3. Extend the production benchmark harness and resource sampler with raw
   records, manifests, p95/median/max calculations, and quiescence checks.
   Replace the Clean historical constant and recorded-only gates in
   `tools/measure-resources.ps1:1381-1404` with the paired fresh baseline and
   the three real comparators (parent design §4.5, TPR-01). Add the
   same-live-PID desktop Status stop experiment beside the CLI exit
   experiment (parent design §4.6, TPR-02) and the per-operation
   cancel/completion table (parent design §4.7, TPR-06).
4. Run fixture/offline benchmarks first, then release-build same-host runs.
   Keep CDP/debug and native evidence in separate files.
5. Stop on any threshold miss and return the owning child a concrete trace;
   do not loosen thresholds or hide work behind UI state. A measured miss
   stays `fail` in the handoff; only uncaptured evidence is `UNVERIFIED`
   (parent design §4.9, TPR-07).

### 4. `desktop-native-acceptance`

1. Build the current release binary and capture host/build/fixture hashes.
2. Run native Windows keyboard, focus, bilingual, WebView device scale
   100/125/150/200% plus unchanged current OS scale, restart, cancellation,
   route-change, and no-UAC scenarios with isolated `LOCALAPPDATA`. Do not
   change the user's display configuration (TPR-05).
3. Re-run `just ci`, the desktop web gates through `just desktop-web-check`
   (read-only `types:generate -- --check`, TPR-08), `git diff --check`, and
   the local package/executable identity checks from parent design §4
   (`just desktop-build` outputs, hashes, version resource, unsigned status,
   running window/process identity; no installer execution, TPR-03).
4. Write the parent evidence matrix with PASS/FAIL/UNVERIFIED per mode,
   locale, authority state, scale, and evidence type. Consume the performance
   child's same-live-PID desktop Status window and operation table; a
   performance `fail` stays `fail` in the matrix (TPR-02/06/07).
5. Return unresolved product or contract failures to the owning child; do not
   patch around them in the acceptance task.

## Validation commands

From `desktop/` during implementation (regeneration is allowed while the wire
contract changes):

```powershell
mise exec node@22 -- npm run types:generate
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Final verification uses the read-only check instead of regeneration
(TPR-08). From the repository root:

```powershell
just desktop-web-check
just ci
git diff --check
python ./.trellis/scripts/task.py validate desktop-mole-tauri-performance
```

The performance child additionally runs the repository's frozen resource
protocol and the Analyze production harness. The native child records commands,
exit codes, logs, raw samples, and evidence paths rather than reporting a
screen-only impression.

## Review gates

- Gate A: service boundary and specs are coherent; no Tauri→CLI subprocess,
  shell string, or duplicated authority path.
- Gate B: all five modes render through the existing registry and preserve
  reducer/bridge invariants in both locales.
- Gate C: performance traces show the operation table's cancellation and
  completion predicates, no overlap, no stale completion, a comparable fresh
  Clean baseline with three real comparisons, same-live-PID Status
  quiescence, and passing resource/render thresholds. A measured `fail` or
  a missing required row fails Gate C; `UNVERIFIED` never converts a
  measured `fail`.
- Gate D: native acceptance and independent plan review pass; only then may the
  parent be considered ready for user-authorized implementation closeout.

## Risky files and rollback points

| Area                                                                       | Risk                                                                              | Rollback                                                                                                                                    |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `.trellis/spec/{backend,desktop-frontend}`                                 | Contract drift can authorize the wrong boundary.                                  | Revert spec and dependent task docs together.                                                                                               |
| `desktop/src/api/*`, generated types, `desktop/src-tauri/src/*`            | Wire/authority mismatch.                                                          | Revert adapter, generated output, and fixtures as one unit.                                                                                 |
| `desktop/src/app-shell/*`, `desktop/src/styles.css`, `desktop/src/modes/*` | Visual or focus regression.                                                       | Revert child-owned UI files only; preserve reducers/contracts.                                                                              |
| `desktop/src/state/operation-coordinator.ts`, benchmark tools              | Lifecycle/resource regression.                                                    | Restore prior coordinator and discard new measurements; never edit thresholds to pass.                                                      |
| `tools/measure-resources.ps1` Clean/Status gates                           | A retained historical constant or recorded-only gate passes without a comparison. | Keep the historical run as archived evidence; do not accept a summary whose Clean or desktop Status gates lack a fresh comparable baseline. |
| native evidence/package files                                              | False confidence from stale binaries, CDP, or an installed copy.                  | Mark `UNVERIFIED`, discard stale artifacts, rebuild and recapture from the local build only.                                                |

## Closeout boundary

This file does not authorize implementation, commit, push, archive, or release.
After the user's explicit approval of the final planning summary, start only
the first child and keep the parent as the coordination task.
