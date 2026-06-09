# Design.md split analysis design

## Architecture Of The Task Tree

This is a parent planning task. It should not be the implementation target unless the only remaining work is to update the planning artifacts themselves.

The parent owns:

- source requirements extracted from root `design.md`
- global safety invariants
- child task map
- cross-child acceptance criteria
- final integration review

Child tasks own concrete implementation deliverables. Dependencies must be written in each child artifact instead of being implied by tree position.

## Proposed Task Tree

| Order | Child slug | Purpose | Main dependency | Verification signal |
| --- | --- | --- | --- | --- |
| 1 | `foundation-cli-domain-model` | Cargo app skeleton, root `justfile`, CLI entrypoints, config/logging, empty TUI entrypoint, cleanup plan/domain model | none | `just ci`; CLI commands exist; empty JSON plan schema serializes |
| 2 | `project-scanner-json-plan` | Marker-first Rust/Node/Python scanner, size estimator, dedupe, JSON plan output | child 1 plan model | fixture tests prove expected targets and no markerless cleanup |
| 3 | `execution-engine-audit` | Dry-run, command-backed and trash-backed execution boundaries, audit JSONL, failure reports | child 1 model; child 2 targets | dry-run cannot mutate; audit emitted for success/failure paths |
| 4 | `global-cache-providers` | npm, pip, pnpm, Yarn, Cargo home inspect provider; Docker deferred | child 1 model; child 3 command runner | providers expose official commands and do not hard-delete cache internals |
| 5 | `ratatui-app-experience` | Dashboard, tabs, details, confirm modal, jobs/logs, filter/search, keyboard help | child 1 app model; scanner/executor APIs | TestBackend/smoke checks render state; worker events keep UI responsive |
| 6 | `safety-hardening-release` | Windows locked-file handling, symlink/reparse tests, snapshot tests, CI matrix, release zip, README, optional Docker follow-up plan | MVP functionality complete | platform/safety tests and release checklist pass |

## Dependency Boundaries

- Scanner code may produce `CleanTarget` values only; it must not delete, move, or execute cleanup commands.
- Execution code consumes `CleanTarget.action`; it must not rediscover cleanup candidates.
- Global providers should use official commands or inspect-only targets; they must not parse or delete opaque cache internals by default.
- The TUI reads app state and dispatches actions/jobs; render code must not perform scanning, cleanup, or large size calculations.
- Audit/plan serialization is a domain contract, not a UI detail.

## Safety Invariants

- Default behavior is scan/plan/dry-run, not cleanup.
- Every candidate must include evidence.
- Marker-first matching is required for project artifacts.
- `Cargo.toml` plus `cargo metadata` is the preferred proof for Rust `target`.
- `cargo clean` is preferred over deleting Rust `target` directly.
- Project directories use trash-backed cleanup by default.
- Permanent delete remains disabled unless a future explicit scope enables it.
- Cargo home is inspect-only in MVP unless the user later approves advanced component cleanup.
- Symlink, junction, and reparse point traversal is disabled by default.

## MVP Boundary

The root design names a broad MVP, but its later phase plan and minimum first-version recommendation narrow the initial implementation. The practical MVP should be:

- CLI/TUI entrypoints and root `justfile` command shortcuts exist.
- Project scanner finds Rust `target`, Node `node_modules` and selected Node caches, Python caches and virtualenvs under configured roots.
- JSON cleanup plan includes size, risk, evidence, selected-by-default, and action.
- Execution supports dry-run, `cargo clean`, trash-backed project cleanup, and audit JSONL.
- Global providers support npm, pip, pnpm, Yarn, and Cargo home inspect through official command discovery.
- TUI can display targets, details, selected totals, confirmation, jobs/logs, and cancellation without doing side effects in render code.

Docker builder cache is deferred out of MVP by user decision. It may be revisited in `safety-hardening-release` or a later child task after the language-cache MVP is working.

## Compatibility And Rollback Notes

- This repo is currently planning-only, so implementation tasks should expect to create the Rust project structure from scratch.
- Each child should preserve unrelated Trellis/bootstrap files.
- If a later child reveals a domain model flaw, return to the foundation child's artifacts and update dependent children before implementation continues.
- Any cleanup-capable child must include tests proving default non-mutating behavior.

## Trade-Offs

- Splitting into six children is slower than one large task, but it isolates dangerous cleanup behavior and keeps verification meaningful.
- Putting the domain model in the first child front-loads design effort, but prevents scanner, executor, CLI, and TUI from inventing incompatible contracts.
- Deferring Docker reduces MVP risk because Docker prune commands can reclaim shared cache across projects and are less predictable than language package caches.
