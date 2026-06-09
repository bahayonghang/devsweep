# Project scanner and JSON plan

## Goal

Implement marker-first project scanning and JSON cleanup plan output for the MVP project ecosystems.

## Parent Context

- Parent task: `.trellis/tasks/06-09-design-split-analysis`
- Depends on: `foundation-cli-domain-model`
- Source design sections: project cleanup scope, marker-first scanning, dedupe, size estimation, Rust target strategy.

## Requirements

- Scan configured roots for project-level cleanup candidates.
- Detect Rust `target` only through `Cargo.toml` and preferably `cargo metadata`.
- Detect Node targets only under projects with `package.json` or equivalent marker evidence:
  - `node_modules`
  - `.next/cache`
  - `.turbo`
  - `.parcel-cache` if included by local rule shape
- Detect Python targets with Python root markers or local evidence:
  - `.venv`
  - `venv`
  - `__pycache__`
  - `.pytest_cache`
  - `.mypy_cache`
  - `.ruff_cache`
  - `.tox`
- Disable `.gitignore` filtering for cleanup target discovery so ignored build artifacts are still found.
- Do not follow symlinks, Windows junctions, or reparse points by default.
- Estimate size and last-modified metadata where practical.
- Deduplicate parent/child targets so a selected parent does not double-count nested cleanup candidates.
- Emit JSON cleanup plans using the domain model from the foundation child.

## Acceptance Criteria

- [x] Fixture tests find expected Rust, Node, and Python targets.
- [x] Fixture tests do not match markerless `target`, `build`, or `dist` directories.
- [x] Scanner output includes risk, evidence, selected-by-default, path/command action, and estimated size.
- [x] Parent/child dedupe prevents duplicate counting and duplicate cleanup actions.
- [x] Scanner code only creates `CleanTarget` values and never mutates the file system.

## Out Of Scope

- Cleanup execution.
- Audit logging.
- Global cache provider discovery.
- TUI rendering.
- Docker support.
