# Plans and Reports

DevSweep uses versioned JSON documents so the result of a scan can be reviewed
and later supplied to `clean`.

## Scan report

`scan --json` emits a scan report with:

- `version`, the scan-report version;
- `plan`, an embedded cleanup-plan document;
- `health`, including completeness, diagnostics, and totals.

Health totals distinguish verified bytes, partial lower bounds, and unknown
target count. Incomplete sizing is evidence of uncertainty, not a reason to
assume an exact size.

## Cleanup plan

The embedded plan has version `2` and a list of targets. A target records an
ID, scope, ecosystem, kind, path, estimated size, size completeness, last
modified time, risk, reversibility, default selection, evidence, and cleanup
intent.

Evidence can identify a marker file, known cache directory, rule match, or
other observation that explains why DevSweep considered the target. The intent
states the rule-defined category of work the document claims, such as a
trash-backed project artifact or a registry-owned provider command. Validation
checks that claim against the built-in rule registry.

## Validation before execution

The JSON document is not a shell script. `clean` decodes it as untrusted input,
enforces the supported version and strict fields, then validates each target and
reconstructs the executable action from its registered rule. The program
performs live authorization checks again before execution.

This design prevents a modified plan file from changing DevSweep into an
arbitrary command runner.

## Inventory reports are different

`inventory --json` emits a separate read-only report with `observations`.
Because that document has no cleanup plan, `clean` rejects it. Use inventory to
understand capacity; use scan to create a reviewable cleanup plan.

## Keep documents current

Do not treat an old JSON file as a permanent approval. Re-scan when the target
set, project identity, diagnostic health, or size estimate has changed. A v1
plan must be replaced by a fresh `devsweep scan --json` result.
