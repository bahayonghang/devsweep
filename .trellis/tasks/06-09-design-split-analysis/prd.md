# Design.md split analysis

## Goal

Turn the root `design.md` product/architecture proposal into a Trellis-ready implementation map for `devsweep`: a parent planning task plus independently verifiable child scopes, with clear MVP boundaries, safety constraints, and acceptance criteria.

## User Value

- Preserve the safety-first design intent before implementation starts.
- Make the large cleanup TUI proposal executable in small, reviewable phases.
- Prevent accidental scope creep around destructive cleanup, global caches, Docker, and Cargo home.

## Confirmed Facts

- The repository currently contains only planning/bootstrap files: `design.md`, `AGENTS.md`, `.gitignore`, and `.trellis/`.
- The root `design.md` describes a Rust + ratatui developer disk cleanup tool.
- The product should distinguish global cleanup providers from project-level cleanup targets.
- The first principle is command-backed cleanup for global caches, trash-backed cleanup for project artifacts, dry-run by default, and permanent delete disabled by default.
- The core domain model is `CleanTarget = path/command + size + risk + evidence + reversible + action`.
- The proposal's implementation route is phased: skeleton, project scanner, execution engine, global providers, TUI, then hardening/release.
- The proposal contains a scope tension: Docker builder cache appears in the broad MVP table, but later sections mark Docker as optional or second-version work.
- User decision: Docker builder cache is deferred out of MVP.

## Requirements

- Split the proposal into a parent task that owns source requirements, cross-child acceptance, and final integration review.
- Define child tasks only where each deliverable can be implemented and verified independently.
- Keep safety decisions explicit in every child that can execute cleanup:
  - default dry-run
  - no scanner-side deletion
  - evidence required for every candidate
  - global cache cleanup via official commands
  - project artifacts via trash or command-backed cleanup
  - Cargo home inspect-only by default
  - permanent delete disabled unless separately enabled by an explicit future scope
- Preserve the design's recommended implementation dependency order.
- Do not start implementation until the user reviews the planning artifacts and approves `task.py start`.

## Proposed Child Scopes

1. Foundation CLI and domain model
   - Cargo project, root `justfile` for `ci`/`build`/`dev`/`test` commands, CLI skeleton, config/logging bootstrap, empty ratatui entrypoint, `CleanTarget`/plan schema.
2. Project scanner and JSON plan
   - Marker-first scanning for Rust, Node, and Python artifacts; size estimation; dedupe; JSON plan output.
3. Execution engine and audit log
   - Dry-run execution, `cargo clean`, trash-backed project cleanup, command runner boundary, failure report, audit JSONL.
4. Global providers
   - npm, pip, pnpm, Yarn, Cargo home inspect provider. Docker builder cache is deferred.
5. TUI application experience
   - Dashboard, project/global tabs, details, confirm modal, jobs/logs, filters, keyboard help, async/cancel behavior.
6. Safety hardening and release
   - Windows locked-file behavior, symlink/reparse point tests, snapshot tests, CI matrix, release archive, README, and optional Docker follow-up planning.

## Acceptance Criteria

- [x] Parent `prd.md` records confirmed facts, requirements, proposed child scopes, and open product decisions.
- [x] Parent `design.md` records the task tree, dependency boundaries, safety invariants, and scope trade-offs.
- [x] Parent `implement.md` records an ordered planning/execution checklist and validation expectations.
- [x] Child task boundaries are independently verifiable and do not rely on implicit tree ordering.
- [x] Remaining questions are limited to product intent or scope choices that cannot be answered from the repo.
- [ ] Implementation remains blocked until the user approves the planning artifacts and a specific child task is started.

## Out of Scope For This Planning Task

- Writing Rust implementation code.
- Starting any task with `task.py start`.
- Adding or changing third-party dependency versions beyond what is already proposed in `design.md`.
- Verifying live upstream documentation versions.
- Creating release workflows or CI implementation.

## Decisions

- Docker builder cache is deferred out of MVP and should not be part of the `global-cache-providers` child acceptance criteria.

## Open Questions

- None.
