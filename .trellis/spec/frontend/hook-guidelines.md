# Hook Guidelines

> Stateful UI logic conventions for this project.

---

## Overview

This Rust TUI project does not use React hooks. The equivalent boundary is the
event/update layer in `src/tui/app.rs`: keyboard events and worker messages enter
`App::update`, which mutates state and returns side-effect requests.

---

## Event Logic Patterns

Interactive code follows a simple state/update/render split:

- input events are converted into actions
- update code mutates app state and returns worker/effect commands
- render code reads state only
- modal validation failures must update visible modal state, not only logs
- worker progress that is meant to be visible must survive fast final events;
  keep a final state or require explicit dismissal instead of clearing it before
  the next render
- long-running workers should emit staged progress before their slowest phase
  completes when an earlier phase can produce useful UI state
- staged worker updates must include a job id and the app must ignore stale
  updates from older jobs after a newer job starts

The terminal event loop is responsible for interpreting effects such as start
scan, start clean, cancel job, and quit.

---

### Scenario: Staged scan worker progress

#### 1. Scope / Trigger

- Trigger: a scan or other worker has a fast phase that can produce useful
  `App` state before a slower phase such as global provider discovery or size
  estimation completes.

#### 2. Signatures

- Worker event shape:
  - `WorkerEvent::ScanProgress { job_id, phase, message, plan }`
- App-owned staging shape:
  - current scan job id
  - latest project-phase targets
  - latest global-phase targets

#### 3. Contracts

- `job_id` identifies the worker that produced the update.
- `phase` identifies which staged target slice the update owns.
- `message` is user-visible job/log progress text.
- `plan` is optional; progress-only updates may carry no targets.
- Render functions must consume staged state after `App::update` applies it;
  render functions must not call scanner/provider APIs.
- Project/global target slices must merge through the shared cleanup-plan
  ranking helper so staged TUI output matches `scan --json` ordering and
  freshness default-selection behavior.
- A newer scan job invalidates older staged scan updates for visible target
  replacement.

#### 4. Validation & Error Matrix

- Project phase succeeds, global phase still running -> visible targets may show
  project results and job remains running.
- Global phase succeeds -> project and global targets merge in stable order.
- Global phase includes larger targets than project phase -> larger global
  targets may move ahead of smaller project targets after the shared ranking
  pass.
- Same phase reports again -> replace that phase slice, do not append duplicate
  rows.
- Older job reports after a newer scan starts -> do not overwrite the newer
  visible target snapshot.

#### 5. Good/Base/Bad Cases

- Good: startup scan shows project targets before global cache size estimation
  finishes.
- Base: final scan completion still marks the scan job succeeded and leaves the
  final target snapshot visible.
- Bad: one final `ScanFinished` event gates all visible targets on the slowest
  global provider.
- Bad: stale worker results from an older manual scan replace newer visible
  targets.

#### 6. Tests Required

- State-level test proving project-phase results populate `App.targets` before
  global completion.
- State-level test proving project/global phase updates merge without
  duplicates.
- State-level test proving stale scan job updates do not overwrite a newer scan.

#### 7. Wrong vs Correct

Wrong:

```rust
// Keeps the UI empty until every scan phase completes.
let mut plan = project_scan()?;
plan.targets.extend(global_scan().targets);
send(WorkerEvent::ScanFinished { job_id, plan });
```

Correct:

```rust
// Lets the UI render useful partial state while slow global work continues.
let project_plan = project_scan()?;
send(WorkerEvent::ScanProgress {
    job_id,
    phase: ScanPhase::Projects,
    message,
    plan: Some(project_plan),
});
```

### Scenario: Confirmation snapshots and cleanup worker ownership

#### 1. Scope / Trigger

- Trigger: a TUI change can replace scan results while a cleanup confirmation is
  open, start cleanup work, or receive delayed worker events.

#### 2. Signatures

- Reducer boundary: `App::update(UiEvent) -> Vec<Effect>`.
- Cleanup request: `Effect::StartClean { job_id, plan, selected, plan_digest }`.
- Worker boundaries: `WorkerEvent::ScanProgress`, `WorkerEvent::ScanFinished`,
  `WorkerEvent::CleanProgress`, `WorkerEvent::CleanFinished`, and
  `WorkerEvent::JobFailed`.

#### 3. Contracts

- Opening confirmation creates a private immutable execution manifest from the
  selected typed targets and validates it before showing its canonical digest
  prefix. Enter must emit `StartClean` with that exact manifest and full
  `plan_digest`, not reconstruct a plan from the current mutable target list
  or selection state.
- The cleanup worker revalidates the frozen plan and compares its canonical
  digest with `plan_digest` before it constructs an executor request. A
  mismatch fails the job before audit or cleanup side effects.
- A current scan update immediately invalidates an open confirmation. Keep the
  modal visible with re-confirmation feedback, apply the scan update normally,
  and emit no cleanup effect until the user closes and reopens confirmation.
- Selection state is keyed by `TargetId`. Preserve explicit select/deselect
  overrides for matching staged-scan targets; use `selected_by_default` only
  for targets without an override.
- The app rejects scan and cleanup requests while a cleanup job is active. The
  runtime independently permits only one clean worker, so an accidental second
  `StartClean` cannot spawn a second mutation worker.
- Legal job transitions are `Running -> Cancelling`, `Running ->
  Succeeded|Failed`, and `Cancelling -> Canceled|Succeeded|Failed`. Terminal
  jobs ignore delayed progress, finish, and cancel events after logging them.
- Before real cancellation is wired, `Effect::CancelJob` must not fabricate a
  `JobCanceled` event. Show that the request waits for an action boundary and
  let the worker report its actual terminal result.

#### 4. Validation & Error Matrix

- Scan update while confirmation is open -> confirmation becomes invalid, the
  new snapshot renders, and Enter returns no `StartClean` effect.
- Second app-level cleanup or scan request during cleanup -> no effect and a
  visible log message.
- Second runtime `StartClean` while the permit is held -> no worker is spawned;
  the rejected job receives `JobFailed`.
- Delayed progress after `Cancelling` is ignored; a real worker finish may still
  land as `Succeeded` or `Failed` until true cancellation is wired. Terminal
  states ignore delayed progress, finish, and cancel events.

#### 5. Good/Base/Bad Cases

- Good: a confirmation runs exactly the targets displayed when it was opened.
- Good: a user deselects an existing target, then receives a staged scan update;
  that target remains deselected while a new default-selected target is added.
- Base: a cancellation request may still end as succeeded or failed until the
  real cancellation token reaches the worker.
- Bad: rebuilding a cleanup plan from `App.targets` when Enter is pressed.
- Bad: treating a UI cancellation request as proof that a worker stopped.

#### 6. Tests Required

- Reducer test: open confirmation, inject scan progress or finish, type the
  phrase, and assert no clean effect is emitted.
- Reducer test: mutate current selection after opening confirmation and assert
  the emitted plan still equals the frozen manifest.
- Reducer test: preserve an explicit selection override across a staged scan.
- Runtime test with a blocking clean service: dispatch two clean effects and
  assert exactly one service invocation.
- Runtime test: a revalidated snapshot with a different digest is rejected
  before the executor is called.
- State-machine test: inject delayed worker events after `Cancelling`,
  `Succeeded`, `Failed`, and `Canceled` and assert no terminal revival.

#### 7. Wrong vs Correct

Wrong:

```rust
// The selected set can drift while the modal is open.
let plan = self.selected_cleanup_plan();
return vec![Effect::StartClean { job_id, plan, selected }];
```

Correct:

```rust
// Confirmation owns the exact typed snapshot and identity that will execute.
let ExecutionManifest { plan, selected, digest } = *confirm.manifest.clone();
return vec![Effect::StartClean {
    job_id,
    plan,
    selected,
    plan_digest: digest,
}];
```

---

## Data Fetching

There is no server state or web data fetching. Scanner calls are local
filesystem operations and must run outside render code through the event loop /
worker path so the UI can stay responsive.

---

## Naming Conventions

- Do not use `use_*` naming; this is not a React codebase.
- Prefer `handle_*` for input handlers and `update_*` for state transitions
  when those functions are added.
- Worker event enums should use explicit names such as `WorkerEvent` and
  `ScanProgress`.
- Side-effect requests should use an explicit enum such as `Effect`, not
  stringly typed action names.

---

## Common Mistakes

- Do not introduce a generic hook framework for a single TUI interaction.
- Do not make render functions perform side effects because they are convenient
  places to access UI state.
- Do not scatter event-to-state transitions across individual widgets once the
  TUI has a central app state.
- Do not call scanner/provider/executor APIs directly from key handlers; return
  an effect and let the event loop run it.
- Do not trigger the initial scan from render code or from `App::new()`. Queue
  it through the event/update boundary so the startup job is visible and the
  resulting targets still flow through the normal worker path.
