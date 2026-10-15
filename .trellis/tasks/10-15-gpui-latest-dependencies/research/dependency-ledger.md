# C16 dependency decision ledger

Checked 2026-10-15 against official registry metadata. Exact source URLs and query
timestamps are retained in `direct-dependency-inventory.json`, `registry/` and
`legacy-frontend-inventory.json`. Selected revisions are in Cargo.lock.

## Shared core/CLI and retained Rust shell

| Consumer / kind | Dependency | Baseline direct | Latest stable | Selected | Reason |
| --- | --- | --- | --- | --- | --- |
| core / dependencies | anyhow | 1.0.104 | 1.0.104 | 1.0.104 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | rayon | 1.12.0 | 1.12.0 | 1.12.0 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | serde | 1.0.229 | 1.0.229 | 1.0.229 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | serde_json | 1.0.151 | 1.0.154 | 1.0.154 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | sha2 | 0.10.9 | 0.11.1 | 0.11.1 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | trash | 5.2.6 | 5.2.9 | 5.2.9 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | tracing | 0.1.44 | 0.1.44 | 0.1.44 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dev-dependencies | tempfile | 3.27.0 | 3.27.0 | 3.27.0 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | windows | 0.56.0 | 0.62.2 | 0.62.2 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | windows-sys | 0.59.0 | 0.61.2 | 0.61.2 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| core / dependencies | libc | 0.2.189 | 0.2.190 | 0.2.190 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | anyhow | 1.0.104 | 1.0.104 | 1.0.104 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | clap | 4.6.4 | 4.6.7 | 4.6.7 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | crossterm | 0.29.0 | 0.29.0 | 0.29.0 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | devsweep-core | 0.3.0 | local | 0.3.0 | Local shared core; no registry version. |
| cli / dependencies | ratatui | 0.30.2 | 0.30.2 | 0.30.2 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | serde_json | 1.0.151 | 1.0.154 | 1.0.154 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | tracing-subscriber | 0.3.23 | 0.3.23 | 0.3.23 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dependencies | unicode-width | 0.2.2 | 0.2.2 | 0.2.2 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| cli / dev-dependencies | tempfile | 3.27.0 | 3.27.0 | 3.27.0 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| desktop / dependencies | anyhow | 1.0.104 | 1.0.104 | 1.0.104 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| desktop / dependencies | devsweep-core | 0.3.0 | local | 0.3.0 | Local shared core; no registry version. |
| desktop / dependencies | serde | 1.0.229 | 1.0.229 | 1.0.229 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| desktop / dependencies | serde_json | 1.0.151 | 1.0.154 | 1.0.154 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| desktop / dependencies | tauri | 2.11.5 | 2.12.4 | 2.11.5 | Legacy Tauri comparison baseline only; unchanged pinned resolution, not a new-app selection. |
| desktop / dependencies | tokio | 1.53.1 | 1.53.3 | 1.53.1 | Legacy Tauri comparison baseline only; unchanged pinned resolution, not a new-app selection. |
| desktop / dev-dependencies | tempfile | 3.27.0 | 3.27.0 | 3.27.0 | Latest stable compatible direct release; required minimum recorded and exact resolution locked. |
| desktop / build-dependencies | tauri-build | 2.6.3 | 2.7.3 | 2.6.3 | Legacy Tauri comparison baseline only; unchanged pinned resolution, not a new-app selection. |
| desktop / dependencies | windows | 0.61.3 | 0.62.2 | 0.61.3 | Legacy Tauri comparison baseline only; unchanged pinned resolution, not a new-app selection. |
| desktop / dev-dependencies | windows-sys | 0.59.0 | 0.61.2 | 0.59.0 | Legacy Tauri comparison baseline only; unchanged pinned resolution, not a new-app selection. |
| core / dependencies | windows-collections | new direct edge | 0.100.0 | 0.3.2 | windows 0.62.2 requires 0.3.2 with windows-core 0.62; latest 0.100 requires incompatible windows-core 0.100 and Rust 1.95. |
| core / dependencies | windows-future | new direct edge | 0.100.0 | 0.3.2 | windows 0.62.2 requires 0.3.2 with windows-core 0.62; latest 0.100 requires incompatible windows-core 0.100 and Rust 1.95. |

## Retained legacy frontend / documentation

Every npm direct edge is baseline-only. This audit does not update React, Tauri
JavaScript tooling, or the documentation stack. package-lock.json is retained.

| Manifest | Dependency | Selected baseline | Latest stable/tag | License |
| --- | --- | --- | --- | --- |
| desktop/package.json | @base-ui/react | 1.8.0 | 1.9.0 | MIT |
| desktop/package.json | @tauri-apps/api | 2.11.1 | 2.12.4 | Apache-2.0 OR MIT |
| desktop/package.json | react | 19.2.8 | 19.3.1 | MIT |
| desktop/package.json | react-dom | 19.2.8 | 19.3.1 | MIT |
| desktop/package.json | @tauri-apps/cli | 2.11.4 | 2.12.3 | Apache-2.0 OR MIT |
| desktop/package.json | @testing-library/jest-dom | 6.9.1 | 7.0.1 | MIT |
| desktop/package.json | @testing-library/react | 16.3.2 | 16.3.3 | MIT |
| desktop/package.json | @testing-library/user-event | 14.6.1 | 14.6.8 | MIT |
| desktop/package.json | @types/node | 22.19.11 | 26.6.5 | MIT |
| desktop/package.json | @types/react | 19.2.18 | 19.3.0 | MIT |
| desktop/package.json | @types/react-dom | 19.2.4 | 19.3.0 | MIT |
| desktop/package.json | @typescript-eslint/eslint-plugin | 8.65.0 | 8.71.2 | MIT |
| desktop/package.json | @typescript-eslint/parser | 8.65.0 | 8.71.2 | MIT |
| desktop/package.json | @vitejs/plugin-react | 5.2.0 | 6.1.2 | MIT |
| desktop/package.json | eslint | 9.39.5 | 10.12.0 | MIT |
| desktop/package.json | eslint-plugin-react-hooks | 7.1.1 | 7.1.1 | MIT |
| desktop/package.json | globals | 16.5.0 | 17.13.0 | MIT |
| desktop/package.json | jsdom | 27.4.0 | 30.1.2 | MIT |
| desktop/package.json | quicktype-core | 23.2.6 | 26.0.0 | Apache-2.0 |
| desktop/package.json | typescript | 5.9.3 | 7.0.2 | Apache-2.0 |
| desktop/package.json | typescript-eslint | 8.65.0 | 8.71.2 | MIT |
| desktop/package.json | vite | 7.3.6 | 8.3.4 | MIT |
| desktop/package.json | vitest | 3.2.7 | 5.0.4 | MIT |
| package.json | vitepress | 1.6.4 | 1.6.4 | MIT |

## Transitive constraints and advisory disposition

`resolved-dependency-constraints.json` records every resolved package, license,
declared compiler, source and all dependency requirements. Related families are
not force-upgraded across upstream semver or ABI requirements.

- `lru`: 0.18.1 -> latest 0.18.5 under Ratatui's compatible range; fixes
  RUSTSEC-2026-0253 (patched >=0.18.2). This is a reviewed security update.
- `windows`0.61.3 / `windows-sys`0.59.0 remain in the legacy Tauri graph;
  windows-sys0.45.0 is retained transitively. The shared core uses0.62.2 / 0.61.2.
  Native handles never cross public core APIs as version-specific binding types.
- `sha2`0.10.9 remains a Tauri build dependency; core uses0.11.1. Separate
  digest/block-buffer/crypto-common generations are required by their parent ranges.
- `glib`0.18.5 has RUSTSEC-2024-0429 (fixed >=0.20); it is constrained by
  Tauri's GTK0.18 Linux dependency graph, not linked into the Windows-first app or
  cross-platform CLI. Updating it independently would break GTK type compatibility.
  This retained legacy warning is not a clean-slate security claim or waiver for
  any future Linux desktop distribution.
- `proc-macro-error`1.0.4 and `unic-*`0.9.0 unmaintained notices remain in
  upstream-constrained legacy/TUI backend dependency families. Record them; do not
  silently ignore or force unreviewed forks.
- Baseline Tauri family versions and frontend assets can be reproduced using the
  clean baseline Git commit and copied lock/manifests. Shared dependency updates in
  the candidate do not overwrite the historical baseline evidence.

## Official transitive-family freshness checkpoint

Parent requirements and exact query timestamps are recorded in
`transitive-family-checkpoint.json`. Multiple selected generations are required
by the listed upstream semver constraints or the retained legacy baseline.

| Family | Selected versions | Latest stable | Official source |
| --- | --- | --- | --- |
| windows-core | 0.61.2, 0.62.2 | 0.100.0 | [crates.io](https://crates.io/api/v1/crates/windows-core) |
| windows-sys | 0.45.0, 0.59.0, 0.61.2 | 0.61.2 | [crates.io](https://crates.io/api/v1/crates/windows-sys) |
| sha2 | 0.10.9, 0.11.1 | 0.11.1 | [crates.io](https://crates.io/api/v1/crates/sha2) |
| digest | 0.10.7, 0.11.3 | 0.11.3 | [crates.io](https://crates.io/api/v1/crates/digest) |
| block-buffer | 0.10.4, 0.12.1 | 0.12.1 | [crates.io](https://crates.io/api/v1/crates/block-buffer) |
| crypto-common | 0.1.7, 0.2.2 | 0.2.2 | [crates.io](https://crates.io/api/v1/crates/crypto-common) |
| glib | 0.18.5 | 0.22.10 | [crates.io](https://crates.io/api/v1/crates/glib) |
| gtk | 0.18.2 | 0.19.0 | [crates.io](https://crates.io/api/v1/crates/gtk) |
| proc-macro-error | 1.0.4 | 1.0.4 | [crates.io](https://crates.io/api/v1/crates/proc-macro-error) |
| unic-char-property | 0.9.0 | 0.9.0 | [crates.io](https://crates.io/api/v1/crates/unic-char-property) |
| unic-char-range | 0.9.0 | 0.9.0 | [crates.io](https://crates.io/api/v1/crates/unic-char-range) |
| unic-common | 0.9.0 | 0.9.0 | [crates.io](https://crates.io/api/v1/crates/unic-common) |
| unic-ucd-ident | 0.9.0 | 0.9.0 | [crates.io](https://crates.io/api/v1/crates/unic-ucd-ident) |
| unic-ucd-version | 0.9.0 | 0.9.0 | [crates.io](https://crates.io/api/v1/crates/unic-ucd-version) |
| lru | 0.18.5 | 0.18.5 | [crates.io](https://crates.io/api/v1/crates/lru) |

## Compiler, license and verification boundaries

Core/CLI keep the declared Rust 1.88.0 MSRV. Current Windows validation uses
Rust 1.99.0; exact/mixed MSRV receipts are documented in acceptance-status.md.
MSRV and non-Windows CI now explicitly name core/CLI packages. The newer GPUI
toolchain is not implicitly imposed on CLI.

All package license expressions are retained in the resolved ledger.
`cargo deny --all-features check` evaluates the existing allowlist and source rules
without adding exceptions; `cargo audit` retains all advisory notices in its JSON.
See final verification receipts for exact database revision and gate results.

## GPUI handoff

See [fresh GPUI checkpoint](gpui-latest-checkpoint.md): published 0.2.2 versus
upstream commit `3e145061d861ed32bb6a08dad7b03f2b5e455ad8`. The latter uses
unpublished platform crates and toolchain 1.98.1. C09 owns final native selection,
coherent revision pin, native build and current compiler acceptance; C16 makes
no GPUI compile/performance claim.
