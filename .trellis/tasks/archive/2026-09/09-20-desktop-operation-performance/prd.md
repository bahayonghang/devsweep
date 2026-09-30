# Measure and optimize desktop operation performance

## Goal

Produce reproducible release-build evidence for the five desktop modes and fix
only measured lifecycle, rendering, backpressure, or cancellation problems
that violate the existing operation contract.

## Dependencies and ownership

- Parent task: `09-20-desktop-mole-tauri-performance`.
- Depends on the accepted shared typed-service child; may consume the UX
  child's deterministic fixtures.
- Own resource/render measurement, operation lifecycle, coordinator,
  backpressure, and focused performance fixes. Do not change safety policy,
  add a second service path, or claim native evidence.

## Requirements

### R1. Reproducible protocol

Use the current release binary, current commit, host metadata, and pinned
fixtures. Capture raw 200 ms samples after warm-up and at least five repeats
for idle, Clean, Analyze, Software, Optimize, Status live, cancellation, and
post-operation quiescence. Preserve command lines, sample files, hashes, and
whether a result is local, simulated, fixture, or native.
The Clean comparison uses a fresh accepted baseline captured in the same
session on the same host: the preserved pre-change release CLI binary is
measured in alternating pairs with the candidate binary on the same absolute
scan root, recorded entry count, toolchain, power state, and fixture hashes
(TPR-01). The historical `39999.052` ms constant is not a baseline.
Desktop Status post-stop quiescence is measured on the same live desktop
process across the stop request, acknowledgement, join, and the 25 scheduled
post-stop samples; the CLI `status live` exit experiment is recorded as
separate CLI-exit evidence (TPR-02).

### R2. Evidence-led fixes

Inspect the operation coordinator, Tauri worker/channel lifecycle, event
decoding, reducers, selectors, and render benchmark before changing code. Fix
the smallest verified cause for stale updates, orphan workers, unbounded event
backlog, slow cancellation, unnecessary rerendering, or failure to quiesce.
Do not edit thresholds to make a failing run pass.

### R3. Existing quality gates

Re-run the archived five-mode thresholds against the current build and host;
report pass, fail, skipped, and unavailable evidence separately. A focused
optimization must preserve typed results, audit/provenance, cancellation/join,
and clean plan/digest authority.
Every Clean gate is a real comparison against the fresh baseline: median
elapsed ratio <= 1.20, peak private bytes <= baseline + 64 MiB, max threads
<= baseline + 2. A non-comparable pair fails the Clean gates (TPR-01).
Cancellation and completion are judged per operation by the design's
operation table: Analyze keeps cancel acknowledgement p95 <= 500 ms with
join, Status keeps the 5.0 s same-PID post-stop window, Clean dry-run and
execute are wait-only and must complete before replacement or close, and the
remaining operations must join with recorded request/ack/join latency and no
numeric bound (TPR-06). No universal cancel-to-join timeout exists, and no
operation may be force-killed to meet a gate.
A measured miss is `fail`. `UNVERIFIED` is reserved for evidence that this
child cannot capture, with a recorded reason and owner; it never replaces a
measured `fail` (TPR-07).

## Acceptance Criteria

- [ ] AC1 (R1): Raw measurement artifacts are reproducible on the same
      host/build/fixture and identify every workload and sample count. The
      Clean run records the baseline binary hash, candidate binary hash,
      pairing order, scan root, and entry count; the desktop Status run
      records the live app PID, stop request, acknowledgement, join, and 25
      post-stop samples on that PID.
- [ ] AC2 (R3): Current release evidence meets the applicable archived
      thresholds, including the three real Clean comparisons and the
      same-PID desktop Status quiescence predicate. A measured miss is
      reported as `fail` with its raw run and blocks handoff; only
      native-only evidence that this child cannot capture is `UNVERIFIED`
      with an explicit owner.
- [ ] AC3 (R2, R3): No stale/orphan completion, uncontrolled event backlog,
      semantic result change, authority bypass, or cancellation regression
      is introduced. Every heavy operation is judged by its row in the
      design's operation table: cooperative operations record request,
      acknowledgement, and join; wait-only Clean actions record completion
      before replacement; no operation is force-killed.
- [ ] AC4 (R3): Focused coordinator/mode tests and the canonical `just ci`
      gate pass.
- [ ] AC5 (R1, R3): Parent plan receives links to raw measurements, the
      operation table with observed values, and a concise
      cause/fix/verification record before native acceptance begins; each
      handoff row carries exactly one status of `pass`, `fail`, `skipped`
      (with reason), or `UNVERIFIED` (with reason and owner).

## Out of scope

- External CLI subprocesses, packaging redesign, new integrations, and native
  visual acceptance.
- Rewriting thresholds, removing scenarios, or replacing raw evidence with
  subjective UI inspection.
