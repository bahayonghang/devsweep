# Fix hidden default cleanup selection

## Goal

Prevent the TUI from executing cleanup actions the user did not visibly choose,
and prevent devsweep from attempting to clean the `target/` directory that
contains the currently running `devsweep.exe`.

## User Problem

When running `just dev`, the TUI can show the user on the `Global` tab with only
one visible selected target, while the header says `Selected 2`. Starting cleanup
then runs the hidden `Projects` target first:

- `D:\Documents\Code\Rust\Exp\devsweep\target`
- `cargo clean --manifest-path \\?\D:\Documents\Code\Rust\Exp\devsweep\Cargo.toml`

Because `just dev` runs `target\debug\devsweep.exe`, `cargo clean` tries to
remove the currently running executable and fails on Windows with:

`failed to remove file ... target\debug\devsweep.exe: Access is denied. (os error 5)`

The visible result feels like selecting pnpm store caused devsweep to clean
itself.

## Confirmed Facts

- `just dev` runs `cargo run -- tui`, so the process executable lives under this
  checkout's `target\debug`.
- TUI startup scans both the current project and global providers in
  `src/tui.rs`.
- `src/scanner.rs` marks Rust `target/` cleanup targets as
  `selected_by_default: true`.
- `src/tui.rs` initializes `selected_ids` from every `selected_by_default`
  target, regardless of the active tab.
- `Global` and `Projects` tabs filter visible rows separately, so a selected
  project target can be hidden while the user is viewing global targets.
- Global provider command targets such as `pnpm.store.prune` are not selected by
  default.
- The executor runs command-backed cleanups with program and argv separated,
  which must remain unchanged.

## Requirements

- Do not execute hidden project targets merely because they were selected by
  default at startup.
- Do not let devsweep execute a cleanup target whose path contains the currently
  running devsweep executable.
- Keep cleanup execution explicit: selected target count and confirmation
  details must make cross-tab selections visible before the user types
  `confirm`.
- Preserve safety contracts:
  - cleanup remains dry-run by default outside explicit TUI execution;
  - permanent delete remains disabled;
  - command-backed cleanup continues to use program plus argv, not shell strings.
- Keep the fix small and local to scanner/TUI/executor behavior needed for this
  issue.

## Acceptance Criteria

- [x] Running the TUI from this checkout with `just dev` does not default-select
      this checkout's Rust `target/` cleanup in a way that can be executed
      invisibly.
- [x] If a selected target path contains the current process executable,
      execution skips that target with a clear audit/progress message instead of
      invoking `cargo clean`.
- [x] The confirmation or dry-run review surface shows selected targets across
      scopes clearly enough that a hidden `Projects` target cannot be missed
      while the user is on `Global`.
- [x] Existing global provider targets such as `pnpm.store.prune` remain
      available and manually selectable.
- [x] Existing command execution tests still prove program and argv stay
      separate.
- [x] Focused tests cover:
      - current executable under selected target path is skipped;
      - startup/default selection no longer makes the self-clean target
        executable invisibly;
      - confirmation/dry-run output includes scope or path context for selected
        targets.
- [x] `just ci` passes before the task is reported complete.

## Out Of Scope

- Redesigning all default-selection policy for every ecosystem.
- Adding Docker cleanup behavior.
- Changing `cargo clean` to direct filesystem deletion.
- Changing permanent-delete behavior.
- Building a new multi-step wizard for cleanup execution.

## Open Decision

Should Rust `target/` stop being selected by default for all projects, or only
when the target contains the currently running executable?

Recommended answer: stop selecting Rust `target/` by default in the interactive
TUI, while keeping it selected in generated scan plans only if the existing CLI
contract requires that. This is the safer UX because `cargo clean` is
irreversible from devsweep's perspective and can be hidden behind another tab.
