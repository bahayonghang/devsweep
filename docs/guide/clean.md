# Clean workbench

Clean follows a frozen authority chain:

save observation → exact select → save **new** plan → live preview digest → `--confirm` → execution → Clean V1 audit.

Observation JSON is not a runnable plan. `clean preview` and `clean execute`
require the new plan file.

```powershell
devsweep clean scan --root . --scope all --format json --output observation.json
devsweep clean plan --observation observation.json --select TARGET_ID --output plan.json
devsweep clean preview --plan plan.json
```

Read `TARGET_ID` values from `data.plan.targets[].id` in the observation
envelope. Execute only after the live digest. Confirm the grammar with
`--help` before using it:

```powershell
devsweep clean execute --help
devsweep clean execute --plan plan.json --preview-digest sha256:DIGEST --confirm
```

There is no `--execute` flag.

## Safety

- Scan results are not selectable until a completed report exists.
- Preview is always a dry run. It never runs a command or moves trash.
- Execute requires the saved plan, the live `sha256:` digest from that preview, and `--confirm`.
- Permanent delete is disabled. Docker is out of scope. Cargo home is inspect-only.
- Command cleanup keeps program and argv separate. There is no `--audit-log` flag.

## Capacity

Size labels show verified, partial lower-bound, or unknown evidence. Incomplete totals are never shown as exact. A trash success says the target was moved to trash; capacity becomes available after trash is emptied.

## Protect and rules

Protection and rules are nested under Clean:

```powershell
devsweep clean protect list
devsweep clean protect add --path PATH --confirm
devsweep clean protect remove --path PATH --confirm
devsweep clean rules list
```

## Audit

Execution appends redacted Clean V1 records only to `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl`. Records are `execution_transition` or `protection_mutation`. They do not contain argv, command strings, raw paths, plan payloads, or localized text. Unknown versions and corrupt bytes are preserved and fail closed. The removed `--audit-log` path and `%APPDATA%\devsweep\audit.jsonl` are never discovered or rewritten.
