# Software parity: update check, startup items, leftover size

## Goal

Extend Software from "inventory + uninstall" to the three Mole-style Software
functions that have a safe Windows equivalent: see which installed apps have
an update, manage current-user startup items on the same page, and see the
application size plus leftover size before removal, with uncertain leftovers
unselected by default.

## Dependency and boundary

- Parent: `09-20-desktop-mole-tauri-performance`.
- Starts after `09-23-desktop-mole-capsule-shell`.
- Changes `devsweep-core/src/software/`, new Tauri commands, generated wire
  types, bridge/decoders, `desktop/src/modes/software/`, and CLI presentation
  where the CLI already exposes Software. Requires backend spec updates for the
  new process use and the registry write.

## Requirements

- R1 Update check (read-only): core runs
  `winget upgrade --disable-interactivity` through the existing
  `ProcessRunner` with program and argv separate, a timeout, and an output
  cap. It parses the table into `{ id, name, installed_version,
available_version, source }` rows and matches rows to inventory entries by
  exact name. Missing winget → `unavailable: winget_missing`; pending source
  agreement or non-zero exit → `unavailable` with a stable reason code. Core
  never passes `--accept-source-agreements` or `--accept-package-agreements`.
  The desktop shows an "Updates" tab with the rows and a count on the stage.
  Running the upgrade is out of scope for this round.
- R2 Startup items: core lists startup entries from
  `HKCU\...\CurrentVersion\Run`, `HKLM\...\CurrentVersion\Run` (both views),
  and the current-user Startup folder, with each entry's enabled state from
  the matching `Explorer\StartupApproved` key. Current-user entries can be
  enabled or disabled by writing only the `StartupApproved` value (the
  reversible mechanism Windows Task Manager uses). Machine entries are
  view-only with a "requires administrator" label. No entry is deleted and no
  command line is executed. Each toggle writes a Software audit record.
- R3 Leftover review: the uninstall preview adds leftover candidates for each
  selected app: its `InstallLocation` if present, and directories under
  `%APPDATA%`, `%LOCALAPPDATA%`, and `%PROGRAMDATA%` whose name matches the
  exact display name or `publisher\display name` (case-insensitive exact
  match). Each candidate shows a bounded measured size with the existing
  size-evidence states. `InstallLocation` candidates are "certain" and
  selected by default; name-match candidates are "uncertain" and unselected by
  default. Protected paths and system roots are never candidates.
- R4 Leftover removal: selected leftovers are moved to the Recycle Bin only
  after the uninstall audit reports success for that app, through a plan with
  a preview digest and the existing second confirmation. The stage result
  shows app size plus leftover size moved, never "freed".
- R5 Stage: the Software first screen shows the planet, installed app count,
  update count, and startup item count, with actions to open Inventory,
  Updates, and Startup detail views.

## Acceptance Criteria

- [ ] AC1: Parser tests cover English and zh-CN winget table fixtures, CJK
      display width, truncated names, the "no updates" message, and
      non-zero exit; unknown output → `unavailable`, never an empty success.
- [ ] AC2: A test proves the update argv contains no agreement-accept flag and
      goes through `ProcessRunner` (no shell string).
- [ ] AC3: Startup tests cover enabled/disabled decoding of `StartupApproved`
      bytes, HKLM rows being view-only, and a toggle writing only the
      `StartupApproved` value with an audit record; the fixture registry test
      runs under a temporary test key, not the real Run key.
- [ ] AC4: Leftover tests cover certain vs uncertain default selection,
      protected-path and system-root exclusion, and size evidence states.
- [ ] AC5: Leftover removal cannot run without a successful uninstall audit
      for the same app, a live digest, and confirmation.
- [ ] AC6: New Tauri commands have closed decoders, fixtures, and are listed
      in `SHIPPED_INVOKE_COMMANDS`; desktop tests cover the three detail views
      in both locales.
- [ ] AC7: `just ci`, desktop gates, and `types:generate -- --check` pass.

## Out of scope

- Running `winget upgrade <id>`, Microsoft Store deep links, or any
  auto-update.
- Scheduled tasks, services, or HKLM startup writes (need administrator).
- Deleting startup entries or registry keys.
