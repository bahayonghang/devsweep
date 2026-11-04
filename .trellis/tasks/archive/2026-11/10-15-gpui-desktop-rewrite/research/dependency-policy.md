# Dependency freshness policy

User requirement, 2026-10-15: use the latest versions where possible, including GPUI.
This replaces the draft assumption that preserving Rust 1.88 must dominate every
new dependency choice.

## Selection contract

- Query official registry/upstream release metadata at implementation start and
  again before final integration for the libraries in the affected dependency graph.
- Prefer latest stable compatible releases for all direct libraries; resolve
  transitive versions under upstream constraints and record older constrained families.
- GPUI is pre-1.0. Evaluate the latest published package and latest upstream Windows
  work; if a required native capability needs unreleased upstream code, pin one
  immutable evaluated repository revision consistently across GPUI/platform crates.
- Latest selection and reproducibility are compatible: commit the reviewed lockfile
  and exact source revision. Do not leave wildcard versions, floating main, or
  uncontrolled updates in the build contract.
- Record current/latest/selected/date/source/reason, Rust/SDK needs, license and
  compatibility evidence in the dependency ledger. Never label a library latest
  from an old note or from a package version string on main alone.
- Rust 1.88 is the current baseline. Prefer a documented current stable toolchain
  and deliberate compatibility migration when required by current libraries;
  update declared MSRV and CI together. Do not silently drop a promised compiler.
- C16 owns shared dependency/toolchain upgrades and the initial GPUI candidate audit.
  C09 owns final GPUI pin/native build. This avoids a dependency cycle.
- Keep the old Tauri comparison build reproducible. Inventory it, but record
  baseline-only pins as scoped exceptions rather than modernizing a retiring
  frontend without functional need.
- No dependency updates, installs or toolchain changes occur during planning.

## Current verified GPUI caution

[Upstream README](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/README.md)
warns of breaking changes and current stable Rust. The detailed primary-source
research in gpui-windows-feasibility.md distinguishes published 0.2.2 from newer
main source that still reports that version, including platform and accessibility
changes. That report's original suggestion to prioritize Rust 1.88 is superseded
by the user requirement above; its source observations remain useful.

A concrete final version matrix is deliberately owned by C16/C09, because these
are implementation tasks and no dependency graph is changed in this turn.

