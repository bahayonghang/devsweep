# Recommendation report

Inspect command: `C:\Users\lyh\.cargo\bin\devsweep.exe clean scan --root D:\Documents\Code --scope all --format json --output inspect-observation.json`

Program: globally installed `devsweep.exe` on PATH. Not `cargo run` of this repository.

Roots: `D:\Documents\Code`

Scan Report: `inspect-observation.json` (completed observation; not an executable plan).

## Capacity summary

Estimated Recoverable: verified 1.2 GiB; partial-lower-bound 0.4 GiB; unknown 1 row.

## Cleanup Target table

| id | path | evidence | risk | Estimated Recoverable class | advice |
|---|---|---|---|---|---|
| tgt-1 | `D:\Documents\Code\Other\app\target` | Rust build output | low | verified | recommend |
| tgt-2 | `%USERPROFILE%\.cargo` | Cargo home | high | unknown | inspect-only |

## Inspect Only and excluded rows

Cargo home is Inspect Only. Docker is exclude. This repository is exclude unless the user names those ids. No Cleanup Plan was saved.

## Next step

This inspect request stops at advice. Cleanup needs a displayed list, explicit confirmation of named ids, `clean preview`, the live digest, and `--confirm`.
