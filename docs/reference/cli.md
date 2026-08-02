# CLI Reference

Run the main program explicitly from this repository:

```powershell
cargo run --locked --bin devsweep -- <COMMAND>
```

## Commands

| Command | Purpose | Key options |
| --- | --- | --- |
| `tui` | Open the interactive terminal UI. | None. |
| `scan [ROOT]...` | Discover cleanup candidates and emit a report. | `--json`, `--global`, `--projects`, `--rescan-target <TARGET_ID>`. |
| `inventory [ROOT]` | Inspect capacity without cleanup targets. | `--json`. |
| `clean` | Dry-run or explicitly execute a saved plan/report. | `--plan <PATH>`, `--execute`, `--audit-log <PATH>`. |
| `protect add <PATH>` | Add an existing path to the persistent protection list. | None. |
| `protect remove <PATH>` | Remove a path from that list. | None. |
| `protect list` | List protected paths. | None. |
| `rules` | Print the current project and global rule catalog. | None. |

## Global options

`--help` prints command help. `--version` prints the DevSweep version.

## Scope behavior for `scan`

When neither `--global` nor `--projects` is supplied, `scan` includes both.
Supplying only `--global` excludes project scanning; supplying only `--projects`
excludes global-provider scanning.

## Examples

```powershell
# Inspect known cleanup rules before scanning.
cargo run --locked --bin devsweep -- rules

# Save a global-only scan as JSON.
cargo run --locked --bin devsweep -- scan --global --json > global-scan.json

# Dry-run a previously reviewed plan.
cargo run --locked --bin devsweep -- clean --plan global-scan.json
```

See [plans and reports](/reference/plan-and-report) for the format accepted by
`clean`.
