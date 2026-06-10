# Design

## Root Cause

The failure is caused by three behaviors composing badly:

1. The project scanner marks Rust `target/` cleanup targets as selected by
   default.
2. The TUI imports all default selections into `selected_ids`, even for targets
   hidden by the active tab.
3. The executor invokes `cargo clean` for a target whose path contains the
   currently running `devsweep.exe`, which Windows refuses to delete.

Any complete fix must address both the confusing selection visibility and the
runtime self-clean safety boundary.

## Boundaries

- Scanner/model:
  - May adjust default selection policy for Rust `target/` if the change is
    deliberate and tested.
  - Must not replace command-backed cleanup with direct deletion.
- TUI:
  - Owns interactive startup selection behavior and confirmation presentation.
  - May override default selection for interactive use without changing the
    serialized cleanup-plan contract.
- Executor:
  - Owns last-mile safety. It should skip self-contained targets even if a bad
    plan reaches execution.
  - Must keep `CommandRequest { program, args, cwd }` as the execution boundary.

## Recommended Shape

### 1. Add self-clean guard in executor

Before executing a selected target, compare `std::env::current_exe()` with the
target path when present. If the executable is under the target path, return
`ActionStatus::Skipped` with a message such as:

`skipped: target contains the running devsweep executable`

Use path normalization carefully:

- Prefer canonical paths when they exist.
- Fall back to existing path buffers if canonicalization fails.
- Preserve Windows extended-path behavior (`\\?\`) without string-only prefix
  hacks.

This is the defense-in-depth layer. It protects CLI `clean --execute --plan`
too, not only the TUI.

### 2. Make interactive defaults safer

For TUI startup, avoid selecting irreversible command-backed project targets
that are not visible to the user. The narrowest implementation is to build
`selected_ids` through a helper that filters out self-clean targets and, if
needed, command-backed project targets.

Preferred product policy:

- Global providers remain unselected by default.
- Rust `target/` should not be automatically selected when running the TUI from
  that same checkout.
- Users can still manually select the target from `Projects`.

If changing scanner defaults globally is too broad, keep scanner output stable
and apply this only in `App::with_plan`.

### 3. Improve selected-target review

The dry-run/confirmation review should include scope and path/title context for
selected targets across tabs. The existing dry-run lines already list up to 12
selected targets; make sure the same clarity exists before execution when the
confirmation dialog is shown.

The minimal fix is to add scope/path context to selected-target summaries and
command previews, without redesigning the entire modal.

## Data Flow

```text
scan_current_workspace()
  -> ProjectScanner targets + GlobalProviderScanner targets
  -> App::with_plan()
       -> choose interactive selected_ids safely
  -> selected_cleanup_plan()
       -> selected targets only
  -> Executor::run_plan_with_progress()
       -> self-clean guard before command runner
       -> success/skipped/failed progress + audit JSONL
```

## Compatibility

- Existing serialized cleanup plans remain readable.
- Existing plan targets remain command-shaped and argv-based.
- A plan that explicitly selects the current process target will now skip it
  instead of failing with an OS-level access-denied error.
- Manual selection remains possible for project targets that do not contain the
  current executable.

## Risks

- If scanner default policy changes globally, tests and user expectations for
  `scan --json` may need updates.
- Canonicalization can fail for deleted or permission-limited paths; the guard
  should fail open only when it cannot prove the target contains the current
  exe, while command execution still reports normal errors.
- Confirmation UI text can become crowded in smaller terminals; keep additions
  compact.
