# Rust Source Architecture Audit

## Scope

This audit covers the current production code under `src/`, its colocated
tests, crate exports, maintained Trellis specs, `code_map.md`, historical
`design.md`, and recent source history. It records evidence for planning; it is
not the final target design.

## Baseline

- The working tree was clean before task creation.
- `just ci` passed on 2026-08-02: `cargo fmt --check`, locked all-target check,
  221 tests (216 library and 5 binary), and Clippy with warnings denied.
- The problem is therefore maintainability and ownership, not a known compiler,
  test, formatting, or lint failure.

## Production Hotspots

The production count stops before each module's `mod tests` block.

| Module | Production lines | Test lines | Primary responsibilities observed |
| --- | ---: | ---: | --- |
| `src/tui/app.rs` | 2,012 | 1,128 | state, reducer, input, selection, jobs, logs, confirmation, scan snapshots |
| `src/tui/render.rs` | 1,928 | 696 | all views, modals, styles, layout, formatting, sanitization |
| `src/safety.rs` | 1,225 | 487 | authorization, live checks, user protections, Cargo metadata probing |
| `src/scanner.rs` | 1,043 | 1,079 | traversal, ecosystem discovery, Cargo workspace cache, dedupe, rescan |
| `src/executor.rs` | 1,015 | 1,188 | execution orchestration, adapters, progress, audit journal, replay |
| `src/providers.rs` | 764 | 511 | executable probing, provider commands, known cache discovery, sizing |
| `src/process_runner.rs` | 759 | 337 | cancellation, process policy, capture, timeout, platform tree termination |

Large colocated test blocks are not themselves a defect. The production
responsibility lists show that these files have multiple natural ownership
clusters even before their tests are considered.

## Coupling Findings

### Rule ownership forms conceptual cycles

- `rules` imports implementation modules to aggregate their documentation
  (`src/rules.rs:22`, `src/rules.rs:346`).
- `providers` and `scanner` both import rule tables and rule types
  (`src/providers.rs:20`, `src/scanner.rs:20`).
- `registry` also imports rule documentation constants from both implementation
  modules (`src/registry.rs:8`).

Rule identity and trusted action metadata therefore do not have one owner. A
new rule can require edits across declaration, discovery, catalogue, registry,
and tests, and the dependency direction is bidirectional.

### Discovery infrastructure is owned by execution safety

- Project scanning imports Cargo metadata probe types from `safety`
  (`src/scanner.rs:21`).
- Cargo metadata scope, probe adapters, process classification, and JSON parsing
  live near the bottom of `safety` (`src/safety.rs:1065`).
- `safety` itself imports path, validation, process, and rule modules
  (`src/safety.rs:13`).

Cargo metadata is used for both scan discovery and live authorization, but its
probe implementation is not itself an authorization policy. It needs a neutral
owner that both callers can use without making scanner and safety depend on one
another's implementation modules.

### Contract code depends on runtime infrastructure

- `model` imports `ProcessResult` and `ProcessStatus` (`src/model.rs:5`) to
  implement scan-diagnostic conversions (`src/model.rs:170`,
  `src/model.rs:199`).

Versioned data and domain types should not need the process runner's runtime
types. Mapping runtime observations into report DTOs belongs at the caller seam
or in a neutral conversion module.

### Entrypoint and crate interface are broad

- `main` owns command dispatch plus scan, inventory, clean, protect, and rules
  handlers, plan-file decoding, and display formatting (`src/main.rs:19` through
  `src/main.rs:274`).
- `lib.rs` publicly exports almost every internal module (`src/lib.rs:1`
  through `src/lib.rs:18`).
- Repository-wide search finds `devsweep::...` paths only in `src/main.rs`; no
  in-repository downstream library consumer exists.

Repository evidence cannot determine whether an external Rust caller relies on
the current paths. That compatibility decision must be made before finalizing
the target interface.

### TUI files are split by layer but not by cohesive behavior

- `App` contains cleanup targets, inventory state, selection projections,
  filters, overlays, jobs, logs, progress, scan snapshots, and quit state
  (`src/tui/app.rs:33`).
- `App::update` delegates to large key and worker-event handlers
  (`src/tui/app.rs:110`, `src/tui/app.rs:119`, `src/tui/app.rs:531`).
- `render_app` is a useful small external interface, but its implementation
  contains more than eighty top-level render and formatting helpers spanning
  every tab and modal (`src/tui/render.rs:63`).

The external TUI seam can remain `tui::run()`. Internal state transitions and
view modules can be reorganized without exposing additional interfaces.

## Contracts That Must Survive

- Cleanup remains dry-run by default and executes only an explicitly saved,
  validated plan.
- Permanent delete remains disabled, including when an obsolete flag is
  supplied.
- Scanners, inventory, models, and rules do not perform cleanup side effects.
- Trusted program and argv reconstruction stays separate and never becomes a
  shell command string.
- `SafetyPolicy::authorize` remains the sole pre-side-effect authorization
  funnel, including live path identity and protected-path checks.
- Cargo home stays inspect-only; Docker cleanup is not introduced.
- CLI flags, TUI flows, plan/report JSON versions and fields, audit behavior,
  cancellation, and Windows/Unix process-tree cleanup remain compatible.

## Candidate Deep-Module Seams

These are planning candidates, not approved names or final paths.

1. A rule/trust module should own rule declarations, catalogue metadata, and
   trusted intent-to-action reconstruction behind a small lookup/resolve
   interface. Scanner and provider implementations should consume it, not own
   fragments that it imports back.
2. A neutral filesystem/platform module should own lexical identity, live file
   identity, no-follow reparse probing, containment, and bounded size walking.
   Platform-specific unsafe code should remain private to focused submodules.
3. A discovery module should hide project traversal, Cargo metadata probing,
   target dedupe/rescan, and global-provider scanning behind the existing
   project/global scan interfaces. Inventory remains read-only and should keep
   a separate external interface even if it reuses filesystem internals.
4. An execution module should keep `Executor` as the small interface while
   hiding command/trash adapters, authorization coordination, durable audit
   journal, and replay in internal submodules.
5. The TUI should keep `tui::run()` as its only external interface while
   splitting reducer concerns and render views into private submodules. File
   splits must preserve one state owner and one effect/worker protocol rather
   than creating pass-through modules.
6. The binary should become composition only. Command handlers and plan input
   decoding need a cohesive application/command owner so `main` does not force
   every internal module to be public.

The deletion test applies to every proposed module: removing it should make its
hidden complexity reappear in callers. One-adapter traits and forwarding-only
files do not qualify as architecture improvements.

## Recommended Task Shape

Use this task as a parent requirement and integration task, with sequential,
independently gated children:

1. Centralize rule/trust ownership and remove the model/process plus
   scanner/safety dependency reversals.
2. Modularize backend implementation clusters (filesystem/process, scanning,
   inventory, execution/audit) behind stable interfaces.
3. Decompose TUI state/update and render implementation while retaining the
   `tui::run()` seam and existing behavior tests.
4. Thin the binary and crate interface, then update `code_map.md`, affected
   Trellis architecture specs, and run the final cross-child `just ci` gate.

The Rust public-interface compatibility decision may merge or reshape child 4,
but does not change the behavioral safety contracts above.
