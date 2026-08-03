# Tauri Desktop Parent Integration Review

Date: 2026-08-03
Baseline commit: `4917126e822084d7174708fc5776aef6447709bb`

## Task Tree

All four planned children and the integration-discovered repair child are
archived with `status: completed`:

| Child | Work commit | Primary evidence |
| --- | --- | --- |
| `08-03-core-api-extraction` | `e4b8386` | `research/verification.md`, normalized scan snapshots |
| `08-03-core-contract-extension` | `614a79e` | `research/verification.md`, normalized scan snapshots |
| `08-03-tauri-shell-backend` | `54b2ca6` | `research/verification.md`, native launch logs |
| `08-03-desktop-frontend-ui` | `6408f65` | `research/fixture-e2e.md`, `research/type-generation.md` |
| `08-03-desktop-ci-frontend-gate-fix` | `712884a` | `research/verification.md`, `research/self-review.md` |

The child decisions and deviations are reflected in the parent design: progress
events redact `ScanProgress.partial`, the webview receives no cleanup authority,
desktop CI is Windows-only with Node 22, and core/CLI remain in the three-platform
Rust and two-platform MSRV matrices.

## 1. Full Quality And Platform Gates

| Gate | Result |
| --- | --- |
| Windows `just ci` | PASS: fmt, lock sync, workspace check, 70 CLI/TUI tests, 161 core tests, 1 public API test, 15 desktop tests with 2 ignored fixture entrypoints, and Clippy with warnings denied |
| Node 22 `just desktop-web-check` | PASS: fixture-derived type generation, ESLint, TypeScript, 31 Vitest tests, and Vite production build |
| Desktop CI command contract | PASS after replacing the removed `npm run check` script with the current type-generation, lint, type-check, and test scripts; the following Tauri build step runs the production frontend build |
| Ubuntu 24.04 stable check/test/Clippy | PASS for `--workspace --exclude devsweep-desktop --locked --all-targets`; 70 CLI/TUI, 161 core, and 1 public API test passed |
| Ubuntu Rust 1.88 compiler check/test | PASS with `rustc 1.88.0`; the repaired WSL toolchain lacked its Cargo component, so stable Cargo 1.95 invoked the exact 1.88 rustc/rustdoc binaries |
| Windows native Rust 1.88 check/test | PASS in the archived core extraction verification with the complete 1.88 toolchain |
| macOS target compile | PASS: `cargo check --workspace --exclude devsweep-desktop --locked --all-targets --target x86_64-apple-darwin` |

The first Ubuntu test command exported `CARGO_TARGET_DIR`, which intentionally
changed the Rust fixture's discovered cleanup target and invalidated that test.
Using Cargo's `--target-dir` flag kept the build isolated without leaking the
variable into tests. One subsequent parallel run returned a transient
`NotFound` for `process_fixture`; the exact test and the full workspace suite
both passed on immediate reproduction with the fixture present and executable.

Hosted GitHub Actions were not run because this goal forbids pushing. The local
evidence checks the current workflow layout and platform-sensitive compilation
and tests; it does not claim that remote CI jobs completed.

## 2. CLI Regression

The fixed-timestamp Rust/Node/Python fixture was compared against the clean
baseline reconstructed from the baseline commit. Sorted JSON is byte-equivalent:

- `EQUIVALENT=1`;
- baseline and current SHA-256:
  `57063518d0a2172a002aa8483acef4b073de19f6c2008c3fe9efb8afaebe7489`.

An empty v2 plan was also checked through CLI dry-run and execute paths. Baseline
and current output are equivalent:

- dry-run SHA-256:
  `7e8c9676627f4e9d46df764c4ff1ee12e54899964bb6cdad047e4e16b1dfe3bb`;
- execute SHA-256:
  `27a571e81de73cb1a6b6642b73804e332480b38522ca5e38d888a20788e06713`.

## 3. Desktop Workflow

The archived frontend replay covers the complete controlled-fixture sequence:
scan, cancel, rescan, conservative selection, dry-run, selection change and
immediate digest invalidation, a second dry-run, explicit confirmation, execute,
and per-target report. The one-target digest
`014349a17bf4a3887a794e595fbf29d1d4ecb5754c27c746c2333b52d0e63388`
changes to
`9ad10cc52be4f63b75dcfa34c837260a5118f931229893fec4c90dfac93d7e1a`
after the selection changes. The replay passed at 800 x 600 and 390 x 844 with
no page-level horizontal overflow and no browser warnings or errors.

This replay injects the production `DesktopBridge`; it is not real cleanup or
native IPC evidence. The complementary Tauri child evidence covers the native
window, command contracts, scan event/cancel behavior, digest enforcement,
Windows Job Object child-tree termination, NSIS build, user-confirmed install,
and installed-app launch. The installer is
`target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe`, SHA-256
`1a24db399ac1dfd5fbd9ef8f0d7bc7c31bb9c396a1922284c811b0cfc9a01a2e`.

## 4. Safety Review

- Tauri registers only `scan_start`, `scan_cancel`, `plan_dry_run`,
  `plan_execute`, and protection-list commands. There is no alternate GUI
  execute command.
- `plan_execute` validates the untrusted plan, builds an executing
  `ExecutionRequest`, and always supplies the caller's digest as
  `expected_digest: Some(digest)`. Core rejects stale digests before audit or
  side effects.
- Scan progress is copied with `partial: None` before webview emission, so
  trusted program, argv, cwd, and path authority do not cross the IPC boundary.
- The default capability contains only event listen/unlisten permissions. No
  filesystem or shell plugin is enabled, and the CSP remains restrictive.
- Permanent delete remains disabled in core. Command programs and argv remain
  separate, and cleanup runs only through core validation and execution.
- A scoped copy search over `desktop/src`, `desktop/src-tauri`, and `README.md`
  found no misleading Chinese phrase `已释放`. UI reports use recycle-bin and
  estimated-recoverable wording.

## 5. Documentation And Specs

`README.md` documents Node 22 desktop setup, native development, frontend/Rust
checks, unsigned NSIS build, shared core safety, and recycle-bin capacity
semantics. `.trellis/spec/backend/` records the public core/digest/report
contracts, while `.trellis/spec/desktop-frontend/` records component, state,
type-safety, and quality rules for the React/Tauri surface.

The final integration check also corrected the Windows desktop workflow to call
the scripts that actually exist in `desktop/package.json`; this keeps the
hosted job aligned with `just desktop-web-check` instead of failing before the
Tauri compile step.

## Verdict

The parent acceptance criteria are satisfied by the combined child evidence and
this final integration review. No unresolved cross-layer defect was found. The
remaining product ideas (tray integration, scheduling, localization,
macOS/Linux desktop builds, and audit-history UI) remain explicitly outside the
MVP scope.
