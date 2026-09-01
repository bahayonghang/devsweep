# Cleanup confirmation list

Scan Report: `inspect-observation.json`

Selectable Cleanup Targets: 2

| id | path | evidence | risk | Estimated Recoverable class | advice |
|---|---|---|---|---|---|
| `rust.target:other-app` | `D:\Documents\Code\Other\app\target` | Cargo.toml marker; target/ | low | verified | recommend |
| `node.node_modules:other-app` | `D:\Documents\Code\Other\app\node_modules` | package.json marker; node_modules | medium | verified | recommend |

Inspect Only: Cargo home. Excluded: this repository.

等待确认. Do not run clean execute until the user confirms these ids.
