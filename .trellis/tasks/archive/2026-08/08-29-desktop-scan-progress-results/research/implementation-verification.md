# Implementation verification

## Baseline

- `cargo test -p devsweep-core scan::`: 58 passed before implementation.
- The planned command `cargo test -p devsweep --lib tui::` did not resolve a
  package because the manifest package is `devsweep-cli` (the binary remains
  `devsweep`). The corrected baseline command
  `cargo test -p devsweep-cli --lib tui::` passed 62 tests.
- `cargo test -p devsweep-desktop --lib`: 15 passed and 2 process-fixture
  entrypoints were ignored before implementation.
- `mise exec node@22 -- npm --prefix desktop run test`: 31 passed before
  implementation.

## Lock-resolved Tauri API

- `desktop/src-tauri/Cargo.toml` declares `tauri = "2.11.3"`. This is a Cargo
  caret requirement, not an exact pin.
- `Cargo.lock` resolves `tauri 2.11.5`.
- The lock-resolved local crate source exposes `tauri::ipc::Channel<T>` as a
  command argument and `Channel::send(T)`. The installed frontend API serializes
  a `Channel` command argument as `__CHANNEL__:<id>` and orders callback messages
  by their message index.
- The implementation uses one command-scoped channel per scan invocation,
  sends the latest pending preview before returning the explicit terminal result,
  and ignores stale ids/sequences again in the reducer.

## Focused implementation evidence

- `cargo test -p devsweep-core`: 167 unit tests and 1 public-API test passed.
- `cargo test -p devsweep-cli --lib`: 72 passed.
- `cargo test -p devsweep-desktop --lib`: 18 passed and 2 process-fixture
  entrypoints ignored.
- `npm run lint`, `npm run typecheck`, and `npm run build` from `desktop/`:
  passed.
- `npm test` from `desktop/`: 45 passed across 8 files.
- `npm run types:generate` from `desktop/`: passed and regenerated the checked-in
  IPC types from the seven named contract fixtures.
- `npm run docs:build`: passed for the English and Chinese scan guides.
- `just ci`: passed format, offline lock synchronization, workspace check,
  258 Rust tests (72 CLI + 167 core + 1 public API + 18 desktop),
  and workspace Clippy with warnings denied. Two desktop process-fixture
  entrypoints remained intentionally ignored.

## Independent review corrections

- Known global-cache rules now publish and recheck cancellation after each
  completely constructed target instead of waiting for the provider helper to
  finish its entire loop.
- A canceled or failed TUI scan keeps its snapshot preview-only; selection,
  dry-run, and cleanup actions remain gated until a later `ScanFinished` event
  promotes a report.
- A stopped desktop rescan with an older completed report offers an explicit
  `Return to previous report` transition. The stopped preview never inherits
  plan authority or selection state.
- A progress-channel decode failure requests cancellation and waits for the
  backend terminal result before releasing the frontend run, preventing a
  second scan from racing the still-owned coordinator.
- The visual preview still refreshes at the bounded delivery rate, while the
  polite live region coalesces same-phase announcements to at most once per
  second. Phase changes and cancellation remain immediate announcements.
- A global-only scan canceled before its first provider now reports the Global
  phase instead of the unrequested Projects phase.

## Fixture visual evidence

- The Impeccable detector reported one pre-existing thick side-border warning;
  the dialog callout now uses a quiet full border. No detector rerun was used.
- The fixture workflow was inspected at 800x600 and 390x844 with `scrollY = 0`.
  At 800px the document and body were 800px wide. At 390px the document and body
  content width was 375px inside the 390px viewport, so the page had no
  horizontal overflow; the result table scrolled inside its own frame
  (`clientWidth = 345`, `scrollWidth = 860`).
- Refreshed captures under `research/visual/` show the full brand header and top
  controls, correct singular `1 target` copy, the first discovered result, the
  stable cancel control, stopped preview, and the completed review action. The
  fixture console had no application errors.

Native Windows WebView appearance and installed-app accessibility remain
`UNVERIFIED` until the manual gate is run; fixture-browser evidence does not
substitute for that native check.
