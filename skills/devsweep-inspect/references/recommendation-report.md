# Recommendation report

Write the report after inspect commands finish. Use terms from the repository
`CONTEXT.md`: Cleanup Target, Scan Report, Cleanup Plan, Estimated
Recoverable, and Inspect Only. Do not write "junk file", "space freed",
"partial plan", or "executable plan" for a Scan Preview.

## Required sections

1. Inspect command, global binary, roots, and the Scan Report file that
   supplied evidence (or state that no Scan Report was produced)
2. Capacity summary
3. Cleanup Target table
4. Inspect Only and excluded rows
5. Optimize or Status facts when those commands ran
6. Next step

## Capacity summary

Separate Estimated Recoverable into verified, partial-lower-bound, and
unknown. Incomplete measurements stay in partial-lower-bound or unknown.

## Cleanup Target table

| id | path | evidence | risk | Estimated Recoverable class | advice |
|---|---|---|---|---|---|
| `<target-id>` | `<path>` | short why-stale or why-cleanable phrase | as reported | verified / partial-lower-bound / unknown | recommend / inspect-only / exclude |

Advice values:

- `recommend` — supported Cleanup Target with evidence, not Inspect Only
- `inspect-only` — reviewable, not selectable for cleanup
- `exclude` — out of product scope, this repository, or blocked by the safety contract

DevSweep ranking or default selection is not user approval. Leave rows
unselected until the user names ids.

Hide rows whose verified size is below 512 MiB unless the user asked for a
full list or asked to clean. Keep the full set in the JSON observation file.
Cleanup confirmation lists must show every selectable id.

## Optimize and Status

Optimize `list` rows are catalogue facts. A Settings launch is not completed
maintenance. Guidance-only operations have no run action.

Status snapshot rows are machine facts. They never create a Cleanup Plan.

## Next step

If the user asked only for advice: stop. Later cleanup needs a displayed
list, explicit confirmation of named ids, `clean preview`, the live
`sha256:` digest, and `--confirm`. Omit execute argv from that report.

If the user asked to clean: show the selectable list from
`scripts/list_selectable.py` and wait. Do not execute in that turn.
