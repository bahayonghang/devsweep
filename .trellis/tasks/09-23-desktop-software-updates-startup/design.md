# Design — Software parity

## Core modules (`crates/devsweep-core/src/software/`)

- `updates.rs`
  - `check_updates(runner, cancel) -> SoftwareUpdatesV1` with
    `state: available { rows } | unavailable { reason_code }`.
  - Request: program `winget`, argv
    `["upgrade", "--disable-interactivity"]`, `ProcessPolicy` with the
    provider phase deadline (30 s) and the default stream cap. Do not pass
    `--locale`; that flag selects the manifest locale, not the table text.
    Parsing is column-position based and does not depend on header words.
  - Parser: find the separator line of `-` characters; the line above is the
    header. Column starts are the display-width offsets of header tokens
    (East Asian Wide/Fullwidth count as 2). Require exactly 5 columns;
    otherwise `unavailable: unrecognized_output`. Slice each row by display
    width. A trailing summary line (count of upgrades) is ignored by requiring
    rows to have non-empty Id and Available columns. Strip the "…" suffix
    and mark `name_truncated`.
  - Match to inventory by exact case-insensitive display name; unmatched rows
    are still listed.
- `startup.rs`
  - Sources: `HKCU` and `HKLM` `Software\Microsoft\Windows\CurrentVersion\Run`
    (64 and 32 views for HKLM), `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`.
  - State: `Explorer\StartupApproved\Run`, `Run32`, `StartupFolder`. First
    byte even (`0x02`, `0x06`) → enabled; odd (`0x03`, `0x07`) → disabled;
    missing value → enabled.
  - `set_startup_enabled(entry_id, enabled, confirm)`: only for
    `scope = current_user`. Writes a 12-byte value: first byte `0x02`/`0x03`,
    remaining 11 bytes zero on enable, FILETIME of now on disable (same layout
    Task Manager writes). Re-reads to verify, then appends a Software audit
    record `startup_toggled`.
  - Registry write access reuses the existing registry helper in `arp.rs`
    (extend it with a set-value function); add the needed `windows-sys`
    feature flag only if the helper does not already cover it. No new crate.
- `leftovers.rs`
  - `discover_leftovers(entry) -> Vec<LeftoverCandidateV1>` with
    `certainty: certain | uncertain`, `path`, `size: SoftwareSizeEvidence`
    (bounded measurement via `filesystem` sizing with a time budget).
  - Exclusions: protection list, `%WINDIR%`, `%PROGRAMFILES%` roots
    themselves, user profile root, and any path above the candidate roots.
  - Removal plan: `SoftwareLeftoverPlanV1 { app_id, uninstall_operation_id, items }`,
    digest via `plan/digest.rs`, execution via the Clean `MoveToTrash`
    executor path. Precondition: the Software audit contains a succeeded
    uninstall for `app_id` under `uninstall_operation_id`.

## Tauri and wire

Commands: `software_updates_check`, `software_startup_list`,
`software_startup_set`, `software_leftovers_preview`,
`software_leftovers_execute`. Heavy ones (`updates_check`, `leftovers_*`)
go through `OperationCoordinator` with cancel support; startup list/set are
light. Regenerate `types.gen.ts`, add decoders and fixture bridge values.

## Desktop

`SoftwareWorkbench` gets a stage (counts) and three `DetailView`s: Inventory
(existing list + leftover preview inside the uninstall flow), Updates
(read-only table), Startup (table with a switch per current-user row; machine
rows disabled with the administrator label).

## Spec updates

- `backend/directory-structure.md` / `quality-guidelines.md`: record the
  winget read-only probe and the `StartupApproved` write as the only registry
  mutation, current-user only.
- `docs/safety-capability-matrix.md`: add rows for update check, startup
  toggle, leftover move-to-trash.

## Rollback

Each of R1, R2, R3/R4 is an independent module and command set; revert one
without the others.
