# Research: independent continuous read-only scan preview contract review

- Query: Trace the approved continuous read-only scan-preview direction across core scan emission, CLI/TUI behavior, Tauri projection/transport, React state/rendering, event correlation, cancellation, ordering, coalescing/backpressure, accessibility, and tests.
- Scope: mixed (repository code/specs plus official Tauri and W3C references)
- Date: 2026-08-29

## Findings

### Executive contract finding

This is a cross-layer scan-contract change, not a desktop-only styling change.
The current implementation has exactly one useful staged result boundary:
project targets become available after the entire project phase finishes, before
the global phase starts. A continuous target preview therefore requires all of
the following to change together:

1. core scanners must expose completed target observations before a whole phase
   returns;
2. the core/desktop boundary must project those observations into a DTO that
   cannot be submitted as a cleanup plan;
3. desktop streaming must correlate and order updates and bound their rate;
4. React must keep preview state separate from the completed `ScanReport` and
   from executable selection/dry-run state;
5. cancellation/error/completion must reconcile the same preview snapshot into
   distinct terminal UI states; and
6. the TUI must either make its staged state read-only or explicitly stop
   exposing newly continuous partials as immediately executable targets.

The smallest truthful progress visualization with the present backend contract
is an **indeterminate progress bar** plus backend phase/message and discovered
target count. There is no total-work denominator from which a percentage can be
derived. This is already the repository contract
(`.trellis/spec/desktop-frontend/component-guidelines.md:39-45` and
`.trellis/spec/desktop-frontend/index.md:53-61`). Adding a measured percentage is
a separate backend accounting feature and should not be inferred from the number
of targets found.

### Current end-to-end data flow

```text
ProjectScanner / GlobalProviderScanner
  -> Sweeper::full_scan_outcome
     -> ScanProgress { phase, message, partial: Option<CleanupPlan> }
        -> CLI: callback discarded
        -> TUI: trusted partial plan becomes visible/selectable App.targets
        -> Tauri: partial is forcibly replaced with None
           -> global scan://progress event
              -> strict TS decoder accepts only partial: null
                 -> reducer stores phase/message only
                    -> ReviewPage still renders completed-empty-state copy
```

Evidence:

- `ScanProgress.partial` is a cumulative `CleanupPlan`, whose targets contain
  trusted `CleanAction` values (`crates/devsweep-core/src/scan/mod.rs:118-128`,
  `crates/devsweep-core/src/model/plan.rs:145-176`,
  `crates/devsweep-core/src/model/plan.rs:240-270`).
- `Sweeper` emits a project start update, then only one project partial after
  `scan_roots_with_health` returns, followed by two global status updates and
  one global partial after all providers return
  (`crates/devsweep-core/src/scan/mod.rs:221-275`). Existing tests lock this to
  five status events and two partial snapshots
  (`crates/devsweep-core/src/scan/mod.rs:385-419`,
  `crates/devsweep-core/src/scan/mod.rs:453-469`).
- The plain CLI passes `&mut |_| {}` and emits results only after completion;
  JSON output must remain a single clean document
  (`crates/devsweep-cli/src/application/commands.rs:17-35`).
- The TUI worker forwards the trusted partial verbatim
  (`crates/devsweep-cli/src/tui/runtime/workers.rs:22-49`), and the reducer
  replaces its staged target snapshot and derives selection defaults from it
  while the job is still running
  (`crates/devsweep-cli/src/tui/app/worker.rs:262-305`,
  `crates/devsweep-cli/src/tui/app/worker.rs:338-364`).
- The Tauri adapter deliberately removes `partial` before emission because the
  plan contains program/argv/cwd authority
  (`desktop/src-tauri/src/scan.rs:12-19`). Its Rust regression proves those
  strings do not reach IPC (`desktop/src-tauri/src/scan.rs:238-277`).
- The TypeScript decoder rejects every non-null partial
  (`desktop/src/api/contract.ts:212-217`) and the generated type fixes the field
  to `null` (`desktop/src/api/types.gen.ts:236-240`).
- `App` registers one application-lifetime global listener
  (`desktop/src/App.tsx:22-32`) and the reducer only retains the latest
  phase/message while `phase === "scanning"`
  (`desktop/src/state/app-state.ts:53-60`).
- During scanning, `state.scan` is null because `scan_requested` resets to
  `initialState`; `ReviewPage` therefore renders `No scan results`
  (`desktop/src/state/app-state.ts:36-38`,
  `desktop/src/state/app-state.ts:53-67`,
  `desktop/src/pages/ReviewPage.tsx:6-13`). This exactly explains the supplied
  screenshot; the screenshot contains no instructions and was used only as
  visual evidence.

### Core emission granularity and cancellation checkpoints

#### Project scan

- Project discovery recursively accumulates all targets in a local vector and
  returns them only after all normalized roots are traversed and deduplicated
  (`crates/devsweep-core/src/scan/project/mod.rs:85-146`).
- Candidate construction is currently synchronous at each `targets.push` site:
  pycache at `crates/devsweep-core/src/scan/project/mod.rs:193-217`, Rust at
  `:325-363` / `:385-410`, Node at `:415-444`, and Python at `:447-470`.
  Those are the natural "target is fully observed" emission points.
- Traversal checks cancellation before each recursive directory call
  (`crates/devsweep-core/src/scan/project/mod.rs:149-166`) and forwards the
  cancellation token through recursive descent
  (`crates/devsweep-core/src/scan/project/mod.rs:247-281`).
- However, ordinary project-target sizing drops that token: the target builder
  calls `estimate_tree_with_budget_and_cancel_and_probe(..., None, ...)`
  (`crates/devsweep-core/src/scan/project/mod.rs:582-618`). The sizing engine is
  cancellation-aware when given a token
  (`crates/devsweep-core/src/filesystem/sizing.rs:96-125`,
  `crates/devsweep-core/src/filesystem/sizing.rs:224-258`), but ordinary project
  scan construction does not supply it. Continuous preview work should thread
  the scan token through target builders; otherwise one large target can freeze
  both preview updates and cancellation until its bounded size walk finishes.
- The existing project cancellation test requests cancellation before the scan
  starts, proving quick discovery exit but not cancellation during target sizing
  (`crates/devsweep-core/src/scan/project/mod.rs:1567-1596`). The sizing test
  separately proves the lower-level primitive (`crates/devsweep-core/src/filesystem/sizing.rs:383-410`),
  so an integration regression is missing.

#### Global scan

- Global scanning is a fixed sequential chain of provider helpers, but it also
  returns one complete vector only after every helper runs
  (`crates/devsweep-core/src/scan/global/mod.rs:45-59`). Each helper's push after
  its probe and size estimate is a natural preview boundary, e.g. npm
  (`:61-86`), pip (`:89-126`), pnpm (`:129-154`), Yarn (`:157-207`), Go
  (`:210-244`), Cargo home (`:247-284`), and known home caches (`:287-333`).
- Provider commands are bounded by a global phase deadline and observe
  cancellation (`crates/devsweep-core/src/scan/global/probe.rs:44-104`);
  provider size walks also receive the token (`:126-136`).
- `GlobalScan` returns only `CleanupPlan`, not health/diagnostics
  (`crates/devsweep-core/src/scan/mod.rs:61-65`). Cancellation during global
  probing can therefore lead to skipped probes without an explicit terminal
  canceled classification.

#### Recommended emission boundary

Emit only after a cleanup candidate has completed its initial bounded sizing and
has a stable `TargetId`; never emit each visited directory or size-walk entry.
The core owner should continue deduplication and ranking. A preview update should
be a cumulative replacement snapshot or a bounded batch whose receiver can
reconstruct exactly the same snapshot. Do not make OS `read_dir` order or
first-seen time the presentation order.

The existing shared ordering is size descending, then freshness/mtime/id
(`crates/devsweep-core/src/scan/ranking.rs:98-114`). The TUI spec explicitly
allows a later larger global target to move ahead of an earlier project target
(`.trellis/spec/frontend/hook-guidelines.md:72-84`). Therefore "stable order"
should mean deterministic shared rank with stable target keys and no duplicate
append, not append-only screen position.

### Preview authority boundary

Reusing either `CleanupPlan` or the complete `UntrustedPlan` as the desktop
preview payload does not satisfy the literal read-only-preview requirement:

- `CleanupPlan` contains trusted `CleanAction::Command { program, args, cwd }`
  and exact trash paths (`crates/devsweep-core/src/model/plan.rs:240-270`). It
  must never cross into the webview.
- `UntrustedPlan` removes direct action authority, but each target still has
  `selected_by_default` and a valid cleanup `intent`
  (`crates/devsweep-core/src/model/plan.rs:34-70`). `plan_dry_run` accepts any
  submitted `UntrustedPlan` and selection and returns a confirmation digest
  after validation (`desktop/src-tauri/src/commands.rs:74-105`), while
  `plan_execute` accepts the same plan, selection, and digest
  (`desktop/src-tauri/src/commands.rs:107-145`). A preview payload shaped as a
  complete valid plan would therefore be needlessly reusable as workflow input.
- The strict scan-to-untrusted conversion is currently crate-private and owns
  the action-to-intent validation (`crates/devsweep-core/src/plan/mod.rs:209-281`).
  The desktop crate cannot safely reproduce it.

Recommended contract: add a **core-owned observation DTO** such as
`ScanPreviewSnapshot` / `ScanPreviewTarget`, serialized by Rust and decoded once
in `desktop/src/api/contract.ts`. It should not use the `UntrustedPlan` envelope
and should omit at least executable action, cleanup intent, and
`selected_by_default`. It may carry observation-only identity and presentation
facts already established by scanning: target id, scope, ecosystem, kind, path,
capacity/confidence/warnings, mtime, risk, and evidence if evidence disclosure is
explicitly desired. The completed `ScanReport` remains the sole source of
selection defaults, dry-run input, and cleanup workflow authority.

React state should enforce the same separation structurally:

```text
activeScan.preview.targets  -- observation-only, no selectedIds/dryRun
completedScan.plan          -- selection/dry-run source only after success
```

Do not put preview targets into `state.scan`, do not run `isExecutable` on them,
and do not reuse the selectable `TargetTable` API without an explicit
non-interactive preview mode. Completion should atomically replace/discard the
preview with the decoded final report and only then derive default selection.

### CLI/TUI compatibility

#### Plain CLI

The non-interactive CLI currently ignores progress and then writes either a
human summary or one JSON document (`crates/devsweep-cli/src/application/commands.rs:17-68`).
That is appropriate. If the shared progress type changes, adapt this no-op
consumer but do not put progress on JSON stdout. If human streaming is desired
later, it belongs on stderr/non-JSON only and requires its own output contract.

#### TUI

The TUI already implements job correlation correctly: `WorkerEvent::ScanProgress`
contains `job_id` (`crates/devsweep-cli/src/tui/app/events.rs:38-60`), and
`should_apply_scan_update` accepts only the latest active scan job
(`crates/devsweep-cli/src/tui/app/worker.rs:308-321`). Tests cover early project
visibility, cumulative replacement/no duplicates, and stale-job rejection
(`crates/devsweep-cli/src/tui/app/tests.rs:568-645`,
`crates/devsweep-cli/src/tui/app/tests.rs:647-687`).

The current TUI partial is **not read-only**:

- staged targets become the main `App.targets` and their default selections are
  projected immediately (`crates/devsweep-cli/src/tui/app/worker.rs:338-364`);
- pressing `c` is blocked only by an active cleanup job, not an active scan, and
  may open confirmation for current staged targets
  (`crates/devsweep-cli/src/tui/app/input.rs:63-95`);
- pressing `d` opens the dry-run overlay without checking for an active scan
  (`crates/devsweep-cli/src/tui/app/input.rs:97-103`).

Increasing partial frequency without addressing this would expand the window in
which an incomplete scan snapshot can be selected and confirmed. The shared
semantics should be made explicit: while a scan is active, staged targets are
preview-only and selection/dry-run/cleanup are disabled; only `ScanFinished`
promotes the final plan. If that TUI behavior is intentionally retained instead,
the desktop must still not inherit it because the task's preview contract is
explicitly read-only, and the difference must be documented as a deliberate
interaction-policy exception.

### Event correlation, ordering, and bounded delivery

The current desktop event has no scan id or sequence number. `App` subscribes to
one global `scan://progress` event for its lifetime
(`desktop/src/App.tsx:22-32`; `desktop/src/api/bridge.ts:39-42`), and the reducer
accepts any well-formed event whenever the phase happens to be `scanning`
(`desktop/src/state/app-state.ts:57-60`). Single-flight rejects a concurrent
backend scan (`desktop/src-tauri/src/scan.rs:44-57`), but it does not prove that
an asynchronously delivered old global event cannot arrive after a new frontend
scan action. It also broadcasts to every listener/window because `AppHandle.emit`
is global (`desktop/src-tauri/src/commands.rs:46-66`).

The desktop stream needs both correlation and monotonic ordering:

- a `scan_id` known before the first update and echoed by every update;
- a monotonically increasing `seq` per scan;
- reducer state `{ activeScanId, lastSeq }` that rejects a different id and
  rejects `seq <= lastSeq`; and
- final success/cancel/error correlated to the same scan id.

For this repository's Tauri 2 stack (the manifest declares `tauri = "2.11.3"`
at `desktop/src-tauri/Cargo.toml:20-24` and `Cargo.lock` resolves Tauri 2.11.5;
`@tauri-apps/api ^2.8.0` at
`desktop/package.json:20-23`), a per-command Tauri `Channel` is a better match
than the global event bus for continuous snapshots. Tauri's official docs say
events are not designed for high-throughput/low-latency streaming and channels
are fast and ordered. A command-scoped channel also supplies natural invocation
correlation and avoids cross-window broadcast. If the existing global event is
retained, explicit `scan_id`/`seq` remains mandatory.

Ordered transport alone does not bound rendering cost. Emitting a full
cumulative plan for every target has quadratic serialization/copy cost and can
queue more React updates than it can paint. The emission policy should be part
of the contract, not an incidental debounce:

- publish the first completed target immediately;
- coalesce later discoveries to a bounded cadence or bounded target batch;
- always publish phase transitions and the final pre-terminal snapshot;
- replace a cumulative snapshot by `TargetId`; never append blindly;
- keep only the latest pending snapshot when the transport/UI is behind; and
- keep spoken live-region updates coarser than visual row updates.

The coalescer needs deterministic tests with an injectable clock or explicit
batch threshold; sleep-based tests will be flaky. A channel improves ordering
but is not itself proof of a bounded pending-update policy.

### Cancellation, failure, and terminal reconciliation

Current production cancellation semantics do not match the fixture-driven
desktop test:

- `ScanCoordinator.cancel` only flips a cooperative flag and is idempotent
  (`desktop/src-tauri/src/scan.rs:49-75`).
- If cancellation is observed after project scan, `Sweeper` marks health partial
  and returns `Ok` with the earned plan (`crates/devsweep-core/src/scan/mod.rs:230-253`).
- The Tauri runner returns that `ScanReport` as success; it does not translate a
  requested cancellation into a typed canceled outcome
  (`desktop/src-tauri/src/scan.rs:33-41`,
  `desktop/src-tauri/src/commands.rs:47-67`).
- `CommandError` has no `scan_canceled` variant
  (`desktop/src-tauri/src/error.rs:6-29`).
- The Rust cancellation test proves only that a fake runner stops within five
  seconds, not how the terminal result is classified
  (`desktop/src-tauri/src/scan.rs:191-217`).
- The React test instead manually rejects the command with
  `{ code: "scan_failed", message: "Scan canceled" }`
  (`desktop/src/App.test.tsx:33-55`). That is controlled-fixture behavior, not a
  production contract proof.

The plan needs a typed terminal decision. For the requested read-only preview,
the safest coherent behavior is:

- `success`: final `ScanReport` replaces preview and enables selection/dry-run;
- `canceled`: retain the last observation preview with canceled copy, do not
  promote it to `state.scan`, do not enable selection/dry-run;
- `failed`: retain the last observation preview plus structured error, do not
  promote it; and
- `partial success`: a completed `ScanReport` with partial health is still a
  completed review state and keeps its verified/lower-bound/unknown semantics.

Do not infer cancellation from `ScanHealth.diagnostics`: global cancellation
does not currently contribute global health, and a partial scan can result from
non-cancel diagnostics. Carry an explicit terminal kind.

There is a second state-loss issue: `scan_requested` resets to `initialState`,
so a failed rescan loses the previous safe completed report even though the
desktop state spec says failures retain the last safe report where possible
(`desktop/src/state/app-state.ts:36-38`, `:53-67`, `:97-101` versus
`.trellis/spec/desktop-frontend/state-management.md:17-19`). Separate
`activeScan` from `completedScan` so previous authority is not silently mixed
with, or destroyed by, the new preview.

### Categorization and stable presentation

Use existing typed scan facts; do not parse path text or `rule_id` strings.
`Scope`, `Ecosystem`, and the six `TargetKind` variants already exist in the
core model (`crates/devsweep-core/src/model/plan.rs:179-224`). The current table
already presents kind as Category and ecosystem/scope as secondary identity
(`desktop/src/components/TargetTable.tsx:25-40`).

A conservative preview hierarchy is:

1. top-level group by `scope.type`: Project targets, Global caches;
2. keep `kind` as the visible category label/badge and ecosystem as secondary
   context; and
3. preserve core-ranked order within each group, keyed by `target.id`.

This matches the two scan toggles/phases and avoids inventing a second domain
taxonomy. Group projections belong in `desktop/src/state/selectors.ts`, not in
components. Do not calculate preview-selected totals; show observation totals
as "found" / estimated capacity with verified, at-least, and unknown labels.

The completed review table should remain unchanged until success. Preview rows
should omit selection controls entirely or render a dedicated non-interactive
table/list; disabling the existing selection column still communicates a
selection affordance that the task explicitly forbids.

### React rendering and accessibility

The screenshot's visual failure follows directly from routing: the toolbar is
aware of scanning, but main content renders the scan-null empty state
(`desktop/src/App.tsx:63-73`, `desktop/src/pages/ReviewPage.tsx:6-13`). Add an
explicit scanning surface or scanning branch before the completed review branch.

Accessibility requirements for that surface:

- Use native `<progress>` without a `value`, or `role="progressbar"` without
  `aria-valuenow`, while work is indeterminate. Give it a visible accessible
  name tied to the phase label. Never set a synthetic percent.
- Mark the preview results region `aria-busy="true"` while scanning and remove
  it on terminal state.
- Keep a pre-existing `role="status"` / `aria-live="polite"` node for phase,
  discovered-count, cancel-requested, and terminal copy. The current status node
  is already polite (`desktop/src/pages/ScanPage.tsx:8-17`), but per-target
  announcements must be coalesced so screen-reader users do not receive a rapid
  stream. Do not put `aria-live` on the whole changing results table.
- Cancellation remains a real button and must keep keyboard focus/width when its
  label changes (`Cancel scan` -> `Canceling…`).
- Do not animate row insertion/reordering. The existing stylesheet already
  disables spinner motion under `prefers-reduced-motion`
  (`desktop/src/styles.css:92-106`); any new progress animation needs the same
  static meaningful state.
- Preserve a real table for comparable data and a scrollable table frame at
  desktop sizes (`desktop/src/styles.css:44-53`), but verify narrow reflow at the
  existing 760 px breakpoint (`:92-105`). Category headings, long paths,
  diagnostics, cancel state, empty-active state, and terminal states need manual
  keyboard and narrow-window inspection per
  `.trellis/spec/desktop-frontend/index.md:45-51`.

### Required contract and regression coverage

#### Core Rust

- Target-level/continuous update appears before its phase finishes and only
  after target sizing completes.
- Updates are cumulative or reconstruct exactly the cumulative snapshot, remain
  deduplicated by target id, and final preview equals the final scan target set.
- Shared ranking remains deterministic when later larger global targets arrive.
- Safe preview serialization contains no `action`, `program`, `args`, `cwd`,
  plan `intent`, or default-selection authority.
- Cancellation during an ordinary project target size walk returns promptly and
  preserves the last emitted observation.
- Cancel, hard project error, partial-success diagnostic, and complete-empty are
  distinct.

Likely test owners: inline tests in
`crates/devsweep-core/src/scan/mod.rs`,
`crates/devsweep-core/src/scan/project/mod.rs`,
`crates/devsweep-core/src/scan/global/mod.rs`, and the safe DTO owner under
`crates/devsweep-core/src/model/`.

#### CLI/TUI Rust

- Plain `scan --json` remains one parseable document with no progress mixed into
  stdout; human CLI summary remains terminal-only unless separately designed.
- TUI keeps latest-job correlation and cumulative replacement.
- TUI selection, dry-run, and cleanup cannot start from active-scan preview
  targets if read-only semantics are adopted.
- Cancel requested ignores later progress and terminal canceled state retains
  preview without promoting it.

Likely owners:
`crates/devsweep-cli/src/application/commands.rs`,
`crates/devsweep-cli/src/tui/app/events.rs`,
`crates/devsweep-cli/src/tui/app/worker.rs`,
`crates/devsweep-cli/src/tui/app/input.rs`,
`crates/devsweep-cli/src/tui/runtime/workers.rs`,
`crates/devsweep-cli/src/tui/runtime/tests.rs`, and
`crates/devsweep-cli/src/tui/app/tests.rs`.

#### Tauri Rust

- Safe projection regression rejects/leaks none of program/argv/cwd/action and
  none of a submit-ready plan envelope.
- Per-command channel or event envelope preserves id/seq ordering.
- Coalescing is bounded, retains first/phase/final updates, and retains the
  latest snapshot under a burst.
- Single-flight, repeated cancellation, release-after-terminal, and a stale
  cancel id are covered.
- Production cancel classification is asserted, not only fixture latency.

Likely owners:
`desktop/src-tauri/src/scan.rs`,
`desktop/src-tauri/src/commands.rs`,
`desktop/src-tauri/src/error.rs`, and command registration in
`desktop/src-tauri/src/lib.rs` if signatures/event transport change.

#### TypeScript boundary/state/components

- Refresh `desktop/src/api/fixtures/scan-progress.json`; add multiple fixture
  variants for started, preview batch/snapshot, phase boundary, canceled/error,
  and final correlation as appropriate.
- Update `desktop/scripts/generate-types.mjs` references and regenerate
  `desktop/src/api/types.gen.ts`; it is generated and must not be edited by hand
  (`desktop/scripts/generate-types.mjs:17-42`,
  `.trellis/spec/desktop-frontend/type-safety.md:32-47`).
- Decoder tests reject unknown fields, unsafe integers, missing scan id/seq,
  non-monotonic/bad envelopes, and any authority-bearing preview shape.
- Bridge tests prove exact channel/event command arguments, ordered delivery,
  unlisten/channel cleanup where applicable, and decode failure routing.
- Reducer tests prove different-id and stale-seq rejection, snapshot
  replacement/no duplicates, preview isolation from `scan`/`selectedIds`,
  completion promotion, cancel/error retention, partial success, empty complete,
  and preservation of the last safe completed report on rescan failure.
- Component/App tests prove no completed-empty copy while scanning, a target is
  visible before command resolution, categories and deterministic order, no
  selection/dry-run affordance in preview, indeterminate progress semantics,
  polite status, cancel-requested copy, keyboard buttons, and distinct terminal
  states.
- Extend the controlled fixture bridge; current progress fixtures carry only
  phase/message/null and the first cancel is simulated as `scan_failed`
  (`desktop/src/api/fixture-bridge.ts:20-35`).

Likely owners:
`desktop/src/api/contract.ts`,
`desktop/src/api/bridge.ts`,
`desktop/src/api/fixtures/`,
`desktop/src/api/types.gen.ts` (generated),
`desktop/scripts/generate-types.mjs`,
`desktop/src/state/app-state.ts`,
`desktop/src/state/selectors.ts`,
`desktop/src/App.tsx`,
`desktop/src/pages/ScanPage.tsx`,
`desktop/src/pages/ReviewPage.tsx` or a dedicated preview component/page,
`desktop/src/components/TargetTable.tsx` only if it receives an explicit
non-interactive mode,
`desktop/src/styles.css`, and their existing Vitest suites.

### Files found

| Path | Role / affected reason |
| --- | --- |
| `crates/devsweep-core/src/scan/mod.rs` | Public progress shape, phase orchestration, cumulative ranking, phase-level tests. |
| `crates/devsweep-core/src/scan/project/mod.rs` | Project discovery/target creation, cancellation threading, target-level emission points. |
| `crates/devsweep-core/src/scan/global/mod.rs` | Sequential provider target construction and target-level emission points. |
| `crates/devsweep-core/src/scan/global/probe.rs` | Provider deadline/cancellation and cancel-aware sizing. |
| `crates/devsweep-core/src/filesystem/sizing.rs` | Existing bounded/cancelable size walker; ordinary project caller currently drops token. |
| `crates/devsweep-core/src/model/plan.rs` | Trusted versus untrusted authority shapes and stable classification enums. |
| `crates/devsweep-core/src/model/scan.rs` | Candidate owner for safe preview/update/terminal observation DTOs. |
| `crates/devsweep-core/src/plan/mod.rs` | Sole strict scan action-to-intent projection; should not be duplicated in desktop. |
| `crates/devsweep-core/src/services.rs` | Shared interactive scan-service callback/outcome type used by TUI. |
| `crates/devsweep-cli/src/application/commands.rs` | Plain CLI ignores progress and owns JSON stdout cleanliness. |
| `crates/devsweep-cli/src/tui/runtime/workers.rs` | Translates shared scan progress into job-correlated TUI events. |
| `crates/devsweep-cli/src/tui/app/events.rs` | Typed job-correlated progress protocol. |
| `crates/devsweep-cli/src/tui/app/worker.rs` | Partial snapshot replacement, stale-job rejection, selection promotion. |
| `crates/devsweep-cli/src/tui/app/input.rs` | Currently permits dry-run/confirmation while scan is active. |
| `desktop/src-tauri/src/scan.rs` | Single-flight, cancellation, trusted-partial redaction, progress forwarding tests. |
| `desktop/src-tauri/src/commands.rs` | Command-scoped worker/stream, global event emit, dry-run/execute input boundary. |
| `desktop/src-tauri/src/error.rs` | No explicit scan-canceled terminal code. |
| `desktop/src/api/contract.ts` | Closed-world decoder currently requires `partial: null`. |
| `desktop/src/api/bridge.ts` | Global event subscription with no id/seq correlation. |
| `desktop/src/api/fixtures/scan-progress.json` | Single phase/message/null fixture; insufficient for continuous stream variants. |
| `desktop/scripts/generate-types.mjs` | Fixture-driven generated type graph. |
| `desktop/src/api/types.gen.ts` | Generated progress DTO currently fixes partial to null. |
| `desktop/src/state/app-state.ts` | Needs distinct active preview/completed authority and id/seq terminal transitions. |
| `desktop/src/state/selectors.ts` | Natural owner for typed category grouping and preview totals. |
| `desktop/src/App.tsx` | Async scan effect/stream correlation and page routing. |
| `desktop/src/pages/ScanPage.tsx` | Toolbar status, cancel state, accessible progress presentation. |
| `desktop/src/pages/ReviewPage.tsx` | Current completed-empty copy leaks into active scan. |
| `desktop/src/components/TargetTable.tsx` | Existing comparable target presentation is selectable; preview needs explicit read-only structure. |
| `desktop/src/styles.css` | Active-scan hierarchy, progress, categories, narrow layout, reduced motion. |
| `desktop/src/App.test.tsx` | Current cancel fixture and workflow component coverage. |
| `desktop/src/state/app-state.test.ts` | Reducer invariants; currently no active preview/correlation tests. |
| `desktop/src/api/contract.test.ts` | Current authority rejection; needs safe preview envelope tests. |
| `desktop/src/api/bridge.test.ts` | Current global listener/unlisten contract; needs channel or id/seq coverage. |
| `desktop/src/api/types-generation.test.ts` | Deterministic generated-contract drift gate. |
| `desktop/src/api/fixture-bridge.ts` | Controlled scan timing/preview/cancel behavior used for UI testing. |

### Related specs

- `.trellis/spec/backend/directory-structure.md:99-103` — `Sweeper` owns merge,
  cumulative progress, and sole ranking; project/global scanners return
  unranked observations.
- `.trellis/spec/backend/directory-structure.md:130-138` — Tauri remains a thin
  adapter and must not invent parallel execution authority.
- `.trellis/spec/backend/error-handling.md:54-67` — inaccessible nested entries
  preserve sibling candidates as partial; incomplete sizes are lower bounds and
  not selected by default.
- `.trellis/spec/frontend/hook-guidelines.md:25-31` — useful worker progress must
  survive fast final events, long work emits staged progress, and stale job ids
  are ignored.
- `.trellis/spec/frontend/hook-guidelines.md:50-84` — TUI cumulative ranked
  replacement/no-duplicate/current-job contract.
- `.trellis/spec/desktop-frontend/index.md:9-12` — React is untrusted
  presentation; Rust owns cleanup authority.
- `.trellis/spec/desktop-frontend/index.md:22-31` — trace every field through
  Serde, decoder, reducer, and rendering; do not infer from display strings.
- `.trellis/spec/desktop-frontend/component-guidelines.md:24-45` — native
  controls, focus, reduced motion, exact backend message, no percentage without
  total, and truthful capacity labels.
- `.trellis/spec/desktop-frontend/state-management.md:21-49` — completed report
  owns selection, trusted partial is redacted, rescans invalidate dry run,
  conflicting commands are disabled, and late progress is ignored.
- `.trellis/spec/desktop-frontend/type-safety.md:3-30` — Rust-owned DTOs, strict
  unknown decoding, PathBuf non-authority, trusted partial redaction, exhaustive
  tagged unions.
- `.trellis/spec/guides/cross-layer-thinking-guide.md` — define one event owner,
  decode once, and correlate derived state to a source event id/sequence.

### External references and repository versions

- Tauri 2 official frontend communication docs:
  <https://v2.tauri.app/develop/calling-frontend/>. The event system is not
  intended for large/high-throughput streams; channels are fast and ordered.
- Tauri 2 official command/frontend docs:
  <https://v2.tauri.app/develop/calling-rust/>. Channels are the recommended
  mechanism for streamed command data, and event listeners must be cleaned up.
- WAI-ARIA 1.2 Recommendation (2023-06-06):
  <https://www.w3.org/TR/wai-aria-1.2/>. An indeterminate progressbar omits
  `aria-valuenow`; a determinate value is allowed only when actually known.
- WAI ARIA APG range/progress guidance:
  <https://www.w3.org/WAI/ARIA/apg/practices/range-related-properties/>. Native
  `<progress>` is suitable; omit value for indeterminate progress.
- WCAG 2.2 SC 4.1.3 Status Messages:
  <https://www.w3.org/TR/WCAG22/#status-messages>. Progress/cancel/result changes
  must be programmatically available without moving focus.
- W3C reduced-motion guidance:
  <https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions>.
  Respect the operating-system motion preference and avoid nonessential row
  motion.
- Repository versions: React `^19.1.1`, `@tauri-apps/api ^2.8.0`, Vitest
  `^3.2.4`, and TypeScript `~5.9.3` (`desktop/package.json:20-44`); Rust Tauri
  manifest requirement `2.11.3`, lock-resolved `2.11.5`
  (`desktop/src-tauri/Cargo.toml:20-24`; `Cargo.lock`).

## Caveats / Not Found

- This was a read-only research pass. No product/spec/task-plan files were
  edited and no build/test commands were run; all behavioral claims are source-
  and existing-test-backed, not fresh runtime verification.
- No current numeric work denominator, scan sequence id, safe preview DTO,
  coalescer, or explicit scan-canceled terminal type was found.
- The supplied `grill-with-docs` entry only instructs callers to invoke separate
  `grilling` and `domain-modeling` skill tools. Those tools were not exposed to
  this research subagent, and this task explicitly forbids ADR/CONTEXT/domain
  document edits. This file applies its question-driven contract review but does
  not claim the unavailable skill-tool calls occurred.
- A per-command Tauri channel is recommended from the official streaming
  contract. Implementation subsequently verified the lock-resolved Tauri 2.11.5
  `Channel<T>` argument/`send(T)` behavior and generated frontend channel shape;
  the manifest's `2.11.3` declaration remains the compatible caret requirement.
- Native Windows webview screen-reader behavior, narrow-window appearance, and
  reduced-motion appearance remain `UNVERIFIED` until manual built-app checks.
