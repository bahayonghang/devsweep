# Recommendation report

Inspect command: `cargo run --locked -p devsweep-cli --bin devsweep -- clean scan --root . --scope all --format json --output inspect-observation.json`

Roots: `.`

Scan Report: `inspect-observation.json` (completed observation; not an executable plan).

## Capacity summary

Estimated Recoverable: verified 1.2 GiB; partial-lower-bound 0.4 GiB; unknown 1 row.

## Cleanup Target table

| id | path | evidence | risk | Estimated Recoverable class | advice |
|---|---|---|---|---|---|
| tgt-1 | `crates/devsweep-cli/target` | Rust build output for this crate | low | verified | recommend |
| tgt-2 | `%USERPROFILE%\.cargo` | Cargo home | high | unknown | inspect-only |

## Inspect Only and excluded rows

Cargo home is Inspect Only. Docker is exclude. No Cleanup Plan was saved.

## Next step

This skill does not execute. A later explicit request can use `clean preview`, the live digest, and `--confirm`. Do not treat ranking as approval.
