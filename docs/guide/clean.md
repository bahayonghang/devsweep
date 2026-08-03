# Clean a Saved Plan

`clean` consumes a saved cleanup plan or a saved scan report. It validates the
document before doing anything and uses the plan's default selected target IDs.

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json
```

## Dry-run first

The command above is a dry-run. It reports how many targets are selected and
does not invoke a cleanup command or move a path to trash.

Run a dry-run again when a plan is old, a project has changed, or an earlier
scan reported incomplete health. A plan is a review artifact, not a permanent
authorization token.

## Execute only after review

Execution requires an explicit flag and a plan path:

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json --execute --audit-log devsweep-audit.jsonl
```

DevSweep writes JSONL audit records to the supplied path. If `--audit-log` is
omitted during execution, it uses `devsweep-audit.jsonl`.

## Validation boundaries

- A plan format v1 document is rejected; rerun `devsweep scan --json`.
- An inventory report is rejected because it has observations, not cleanup
  authority.
- Unsupported report versions and unknown fields are rejected.
- Serialized executable command fields are not trusted. Validated actions are
  reconstructed from the built-in rule registry.
- Targets that are inspect-only, no longer authorized, or no longer match their
  expected identity are not executable.

## No permanent-delete escape hatch

There is no command-line option that re-enables permanent deletion. Docker and
Cargo home remain outside executable cleanup behavior.
