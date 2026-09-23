# Design — Mole-informed workbench with a shared typed service path

## 1. Boundary and ownership

The product boundary is:

```text
React mode -> DesktopBridge -> Tauri command adapter -> devsweep-core service
                                      |                  -> typed error/DTO
                                      +-> Channel for bounded progress/events
CLI command -> the same devsweep-core service -> CLI presentation/output
```

The selected “Tauri 调用 CLI” meaning is contract/service parity, not a
second executable. Tauri must not import the private `devsweep-cli` crate or
launch a packaged `devsweep` child process. Shared domain behavior stays in
`devsweep-core`; CLI human/JSON rendering stays in the CLI crate; Tauri owns
only command state, channel forwarding, app-data paths, and boundary errors.
If an orchestration helper is duplicated between CLI and Tauri, move the
smallest reusable typed operation into core rather than adding a new runtime
dependency or JSON loopback protocol.

The parent owns cross-child contracts and integration. Children own exclusive
file surfaces:

| Child                           | Primary ownership                                                                                    | Must not own                                                   |
| ------------------------------- | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `desktop-mole-workbench-ux`     | `desktop/src/` presentation, mode composition, CSS, visual fixtures                                  | Core safety, command grammar, new providers                    |
| `desktop-tauri-cli-service`     | `desktop/src/api/`, `desktop/src-tauri/`, shared core adapters and contract fixtures                 | Visual redesign or benchmark thresholds                        |
| `desktop-operation-performance` | coordinator/backpressure seams, benchmark drivers, measurement artifacts                             | New product capabilities or safety-policy changes              |
| `desktop-native-acceptance`     | native harness/docs/evidence, local package/executable identity checks, and final integration checks | Feature implementation, installer execution, or unrelated dirt |

## 2. Data and lifecycle contracts

### Query and read-only modes

Clean scan, Analyze, Software inventory, Optimize list/preview, and Status
snapshot/live each have one typed request, an opaque operation id, and a
closed result union. The Tauri adapter runs blocking core work off the async
runtime, sends cumulative progress/events through a per-operation `Channel`,
and finishes the coordinator slot only after the worker and event producer
join. React reducers accept only the active operation id and increasing event
sequence.

### Cleanup authority

Clean remains observation-only until a completed scan report becomes a saved
untrusted plan. Selection is projected from executable target ids, dry-run
returns the core digest, and execute accepts only that current digest plus the
second confirmation. Tauri never computes, edits, or reconstructs a digest.
Selection changes and rescans clear dry-run state synchronously.

### Software and Optimize authority

Software inventory and Optimize catalogue items remain untrusted observations
until their existing plan builders and live preview functions reconstruct
opaque authority. Current-user MSIX-only execution and the closed Optimize
catalogue remain core-owned. Manual-only MSI/unsupported entries cannot become
commands through UI state.

### Status and Analyze

Analyze stays read-only and bounded; treemap selection changes only the view
root. Status snapshot/live retains availability unions, interval/process caps,
monotonic event sequence, single-flight sampling, and terminal join behavior.
Missing metrics remain unavailable/unsupported rather than zero.

## 3. Workbench design

Use one original Windows workbench grammar informed by Mole's hierarchy:

- persistent sidebar with five primary tabs and named supporting destinations;
- visible page header and mode status chip;
- dark mode canvas with mineral/forest mode accents;
- raised cards, dense but readable grouped rows, progressive evidence details,
  and a stable action/status boundary;
- Clean review and result stages, Software inventory/preview, Optimize
  checklist, Analyze list/treemap breadcrumbs, and Status metric/process cards;
- visible keyboard focus, arrow/Home/End tab movement, route/back focus restore,
  reduced-motion and forced-colors variants, and complete accessible/copyable
  user data.

The visual child may replace generic fallback colors and spacing in
`desktop/src/styles.css`, but it must not alter authority reducers, invent
counts/sizes/status, or reintroduce light panes, glass, gradients, planet
metaphors, fake macOS chrome, or “space freed” copy.

## 4. Performance design

The performance child extends the existing release measurement protocol rather
than inventing a subjective score:

1. Record commit, binary hashes, host/OS/toolchain, locale, fixture hashes,
   power state, and Defender/environment notes.
2. Use one warm-up and five measured repetitions per workload, 200 ms process
   samples, named nearest-rank p95/median/max statistics, and raw JSON/CSV.
3. Cover idle desktop, Clean scan, Analyze traversal and UI drill-ins,
   Software inventory, Optimize list/preview, Status snapshot/live, route
   cancellation, and post-stop quiescence.
4. Reuse thresholds in
   `.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/design.md`.
   A changed host, fixture, or build invalidates comparison and requires a
   fresh baseline; old reports remain historical evidence.
5. Clean baseline (TPR-01): the accepted baseline is a fresh same-session
   measurement of the baseline release CLI binary, built from the parent
   base commit (the HEAD recorded when the first child starts) in a separate
   git worktree with the same toolchain, alternated in pairs with the
   candidate binary on the same host, absolute scan root, recorded entry
   count, toolchain, and power state. All three frozen comparators use real
   statistics: candidate median elapsed / baseline median elapsed <= 1.20;
   candidate peak private bytes <= baseline peak + 64 MiB (67108864 B);
   candidate max threads <= baseline max + 2. The historical constant
   `39999.052` ms at `tools/measure-resources.ps1:1382` and the
   recorded-only `$true` gates at `:1395-1396,1399-1403` are not
   comparisons; the performance child replaces them. A non-comparable pair
   makes the Clean gates `fail`, never pass.
6. Status stop (TPR-02): desktop post-stop quiescence is observed on the same
   live desktop PID. The stop is the UI stop-live action, which reaches
   `status_cancel` (`desktop/src-tauri/src/status.rs:219-225`);
   acknowledgement is the Status surface entering `canceling`. Join is
   layered because a release binary exposes no Tauri promise to the driver:
   the surface leaving `canceling` (terminal event delivered), a focused
   Rust test that `status_live_start` returns only after `control.join()`
   (`status.rs:143-168`), and the thread-count part of the final-five hold,
   which fails if the producer or blocking worker thread survives.
   The 25 scheduled 200 ms samples start at the observed acknowledgement
   and the app process must be present in every sample. The existing CLI
   `status live` exit experiment (`tools/measure-resources.ps1:1098-1144`)
   remains CLI-exit evidence under separate gate names; a killed process or
   an absent PID is never desktop quiescence evidence.
7. Cancellation and completion predicates (TPR-06): the performance child's
   design holds one operation table with the cancel semantics, request,
   acknowledgement, and join events, statistic, bound, and evidence source
   for each heavy operation. Analyze keeps the frozen cancel acknowledgement
   p95 <= 500 ms with join; Status keeps the 5.0 s post-stop window; Clean
   dry-run/execute are wait-only and complete before replacement or close.
   Operations without a frozen numeric bound must join and record
   request/ack/join latency; no universal timeout is added.
8. Treat a threshold miss as a stop/go failure. Do not hide a slow path with a
   loading animation or count a CDP/debug process as native product evidence.
9. Evidence status (TPR-07): `fail` is a measured miss; it blocks the child,
   Gate C, and parent AC4 until the owning child fixes it. `UNVERIFIED` is
   only for evidence that could not be captured in that child, with a reason
   and owner; it may be handed to native acceptance but never replaces a
   measured `fail`. `skipped` requires a recorded reason and is not allowed
   for a required row.

Frontend render evidence uses the real production component harness and the
checked-in CDP driver where appropriate. Native evidence launches the real
binary with isolated `LOCALAPPDATA`; it records process tree, PID, hashes,
accessibility tree, screenshots, and scale evidence. Required scale evidence
(TPR-05) is WebView device scale 100/125/150/200% through the WebView2
`--force-device-scale-factor` launch argument (the driver launch in
`tools/measure-resources.ps1:552-554`) plus the unchanged current actual
Windows display scale. Neither the agent nor the operator changes the user's
display configuration; other actual Windows scales are non-gating
`UNVERIFIED`. Each row labels its evidence as actual OS scale, WebView device
scale, or CSS viewport emulation.

Package and executable identity (TPR-03) is limited to the local build.
`just desktop-build` produces `target/release/devsweep-desktop.exe` and the
unsigned NSIS `target/release/bundle/nsis/*-setup.exe`. The native child
records both SHA-256 hashes, checks that each file's version-resource product
name and version match `desktop/src-tauri/tauri.conf.json:3-4`, records the
Authenticode status as unsigned, and verifies the running executable's main
window title `DevSweep` (`tauri.conf.json:15`), process name, and executable
path. The installer is never executed; install mode, shortcuts, and
uninstall entries are non-gating `UNVERIFIED`.

## 5. Compatibility and migration

- Keep existing Tauri command names, closed Serde unions, CLI machine fields,
  plan schema, digest semantics, and audit records unless a child documents a
  versioned change and updates every decoder/fixture.
- Keep `desktop/src/api/bridge.ts` as the only domain IPC import. The typed
  presentation-settings adapter (`desktop/src/i18n/index.ts:165-167`) and the
  window-lifecycle/DEV fault adapter (`desktop/src/lifecycle.ts:21-35`) remain
  the two explicit non-domain exceptions (TPR-04); components never import
  `@tauri-apps/api`. Preserve generated types and exact decoders.
- Keep the CLI/TUI catalogue and locale precedence unchanged; additive desktop
  copy must remain EN/zh-CN parallel and machine output locale-neutral.
- Do not change `.trellis/spec/frontend/` TUI guidance or resurrect archived
  desktop shell tasks.

## 6. Rollout and rollback

1. Land or stage service/contract decisions and spec updates before UI or
   benchmark code.
2. Implement the workbench child against existing contracts.
3. Add performance instrumentation and run focused fixture tests before native
   evidence.
4. Run final native/resource gates and parent integration review.

Each child can roll back its owned files independently. If service contracts
change, roll back bridge, Tauri adapter, generated types, and fixtures as one
unit. If the visual contract fails, revert the mode CSS/composition without
touching core authority. Never leave a new shell or command path enabled with
an old or partial safety contract.

## 7. Risks and deferred items

- Shared-service extraction can accidentally move presentation or authority
  into core; keep the transitive type closure minimal and review every new
  public symbol.
- Analyze and Clean are filesystem-heavy; benchmark cancellation and memory
  on the exact fixture before adjusting worker counts or backpressure.
- Windows native evidence, scaling, WebView2 accessibility, and real provider
  availability remain `UNVERIFIED` until direct evidence is captured.
- External CLI process isolation is deliberately deferred to a separate task;
  it is not a hidden fallback.
