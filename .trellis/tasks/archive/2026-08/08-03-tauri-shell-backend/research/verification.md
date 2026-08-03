# Tauri Shell Verification

Date: 2026-08-03
Host: Windows x86_64, Node 22.23.2, npm 11.19.0, cargo-tauri 2.11.4

## Automated Gates

- `cargo test -p devsweep-desktop --lib`: 14 passed, 2 fixture entrypoints ignored.
  Coverage includes
  scan single-flight rejection, idempotent cancellation below five seconds,
  complete progress forwarding, correct and stale confirmation digests,
  selection-sensitive digests, unknown and inspect-only target errors,
  invalid plans, atomic protection replacement, and the Windows process fixture.
- `cargo test -p devsweep-desktop --lib
  commands::tests::process_command_runner_terminates_tree_inside_desktop_process --
  --exact --nocapture`: 1 passed, 0 failed. The test copies the desktop test
  executable to a temporary directory, starts a child and grandchild through
  the production core `ProcessCommandRunner`, requests cancellation, and proves
  that the Win32 Job Object terminates the grandchild. Program and argv remain
  separate and no user path is read or modified.
- `cargo clippy -p devsweep-desktop --all-targets -- -D warnings`: passed.
- Node 22 `npm ci`: 72 packages installed, 73 packages audited, 0 vulnerabilities.
- Node 22 `npm run check`: passed.
- Node 22 `npm run build`: passed; Vite 7.3.6 transformed 28 modules.
- `just ci`: passed for the full workspace (format, offline lock sync, check,
  tests, and clippy).
- `.trellis/scripts/task.py validate`: all context files passed.

## Development Window Smoke

The first launch on template port 1420 failed with
`listen EACCES: permission denied 127.0.0.1:1420`. Windows reported that port
inside an excluded range (1246-1745), so both Vite and Tauri configuration were
moved to fixed port 4180.

The Node 22 retry with `cargo tauri dev` succeeded:

- Vite served `http://127.0.0.1:4180/` with HTTP 200 and app content containing
  `devsweep`.
- `target/debug/devsweep-desktop.exe` was responsive with window title
  `devsweep` and nonzero `MainWindowHandle` 94109884.
- The verified launch process tree was terminated after the smoke. Final checks
  reported zero remaining scoped processes, zero port-4180 listeners, and zero
  `devsweep-desktop` processes.

Raw launch logs:

- `research/cargo-tauri-dev.stdout.log`
- `research/cargo-tauri-dev.stderr.log`

## Bundle

`mise exec node@22 -- cargo tauri build` passed and produced one NSIS bundle:

- Path: `D:\Documents\Code\Rust\Exp\devsweep\target\release\bundle\nsis\devsweep_0.2.0_x64-setup.exe`
- Size: 2,131,534 bytes
- SHA-256: `1a24db399ac1dfd5fbd9ef8f0d7bc7c31bb9c396a1922284c811b0cfc9a01a2e`
- Authenticode status: `NotSigned` (expected for this acceptance build)

## Installed Application Smoke

After action-time user confirmation, the main session ran the exact installer
identified above through the Windows UI. NSIS used its default per-user install
location, reported `Installation Complete`, and launched the installed app from:

`C:\Users\lyh\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\devsweep\devsweep-desktop.exe`

The native window title was `devsweep`; its accessible web content contained
the heading `devsweep` and text `Desktop backend ready.`. The window rendered
successfully and was closed after verification. A final window enumeration
reported no remaining `devsweep` window.

## CI Decision

- A dedicated Windows job pins Node 22 through an immutable `setup-node` SHA,
  runs `npm ci`, checks TypeScript, and compiles Tauri with `--no-bundle`.
- `desktop/package-lock.json` is the independent frontend lockfile; the package
  engines require Node `>=22 <23` and npm `>=10 <12`.
- Ubuntu and macOS Rust jobs exclude the Windows-MVP desktop crate. Ubuntu does
  not install WebKitGTK until Linux desktop support becomes an explicit target.
- Existing MSRV jobs continue to cover core/CLI; the desktop toolchain is
  guarded by stable Rust and Node 22 on Windows.
