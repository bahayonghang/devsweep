# Plans and Reports

DevSweep uses versioned JSON documents so a scan can be reviewed, then turned
into a **new** saved plan. Observation JSON is not executable authority.

## Scan observation

`clean scan --format json` emits a V1 machine envelope:

- `schema_version`, `command` (`clean.scan`), `outcome`, `warnings`, `error`;
- `data`, the scan report.

The scan report has:

- `version`, the scan-report version;
- `plan`, an embedded untrusted cleanup-plan document;
- `health`, including completeness, diagnostics, and totals.

Health totals distinguish verified bytes, partial lower bounds, and unknown
target count. Incomplete sizing is evidence of uncertainty, not a reason to
assume an exact size.

Pass this file to `clean plan --observation`. Do not pass it to
`clean preview` or `clean execute`.

## Cleanup plan

`clean plan --observation FILE --select TARGET_ID --output FILE` writes a new
plan file (not an envelope). The plan has version `2` and a list of selected
targets. A target records an ID, scope, ecosystem, kind, path, estimated size,
size completeness, last modified time, risk, reversibility, default selection,
evidence, and cleanup intent.

Evidence can identify a marker file, known cache directory, rule match, or
other observation that explains why DevSweep considered the target. The intent
states the rule-defined category of work the document claims, such as a
trash-backed project artifact or a registry-owned provider command. Validation
checks that claim against the built-in rule registry.

## Validation before execution

The JSON document is not a shell script. `clean preview` and `clean execute`
decode the saved plan as untrusted input, enforce the supported version and
strict fields, then validate each target and reconstruct the executable action
from its registered rule. Execution additionally requires the matching live
preview digest and `--confirm`. The program performs live authorization checks
again before side effects.

This design prevents a modified plan file from changing DevSweep into an
arbitrary command runner.

## Analyze reports are different

`analyze scan --format json` emits a read-only disk snapshot. Because that
document has no cleanup plan, `clean` rejects it as an observation or plan.
Use Analyze to understand capacity; use `clean scan` to create a reviewable
cleanup observation.

`software inventory --format json` is likewise not a Clean plan. Software
selection uses `software plan --inventory FILE --select SOFTWARE_ID --output FILE`.

## Keep documents current

Do not treat an old JSON file as a permanent approval. Re-scan when the target
set, project identity, diagnostic health, or size estimate has changed. A v1
plan or a leftover observation-as-plan file must be replaced by a fresh
`clean scan` observation and a new `clean plan` output.
