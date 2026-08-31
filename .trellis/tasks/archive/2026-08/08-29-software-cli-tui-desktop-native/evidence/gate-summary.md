# Software implementation gate summary

Date: 2026-08-31. Commands below were run from the repository root unless a desktop working directory is stated.

## Authoritative Rust gates

- `cargo check -p devsweep-cli` — exit 0.
- `cargo check -p devsweep-desktop` — exit 0.
- `cargo test -p devsweep-cli software` — exit 0; 13 matching tests passed.
- `cargo test -p devsweep-cli tui::modes::software` — exit 0; 4 tests passed.
- `cargo test -p devsweep-desktop software` — exit 0; 2 tests passed.
- `cargo test -p devsweep-cli --lib` — exit 0; 140 tests passed in the final workspace gate.
- `cargo test -p devsweep-desktop --lib` — exit 0; 30 passed and 2 ignored.
- `just ci` — first run exit 1 because Clippy found two identical branches in the new presentation code; after repair, final run exit 0. The final run passed formatting, offline dependency resolution, checks, all workspace tests (CLI 140, CLI contract 12, core 235, public API 1, desktop 30 passed/2 ignored), and Clippy.

## Desktop web gates

- `npm run types:generate` (desktop) — exit 0.
- `npm run lint` (desktop) — final exit 0.
- `npm run typecheck` (desktop) — final exit 0.
- `npm test` / focused standard Vitest invocation (desktop) — exit 1 before test discovery. Exact host error:

  ```text
  Error: spawn EPERM
      at ChildProcess.spawn (node:internal/child_process:457:11)
  ```

- `just desktop-web-check` — exit 1. Type generation, lint, and type-check completed successfully, then standard Vitest and Vite build both encountered the same `spawn EPERM` child-process restriction.
- `just desktop-build` — exit 1 in Tauri's `beforeBuildCommand` when `npm run build` encountered the same `spawn EPERM` restriction.
- Evidence-only Software suite using `vite-sandbox-shim.cjs` plus `vitest.sandbox.config.mjs` and TypeScript's in-process transpiler — exit 0; 3 files and 8 tests passed. This is supplementary evidence, not a replacement for the standard project gate.

## Repository hygiene and native inventory

- `git diff --check` — exit 0; only configured LF-to-CRLF warnings were emitted.
- First `cargo run -p devsweep-cli -- software inventory ...` attempt — exit 1 because Cargo required a binary name: `error: cargo run could not determine which binary to run... available binaries: devsweep, process_fixture`.
- Corrected `cargo run -p devsweep-cli --bin devsweep -- software inventory --source all --format json --output ...` — exit 0.
- Hidden-process `target/debug/devsweep.exe software inventory --source msix --format json --output ...` — exit 0.
- `target/debug/devsweep.exe software plan ...` for one selected current-user VP9 Video Extensions MSIX identity — exit 0.
- `target/debug/devsweep.exe software preview ...` in JSON, English-human, and Simplified-Chinese-human output modes — exit 0 for all three invocations; digest and exact identity matched.

No uninstall command was run and no installed package was removed.
