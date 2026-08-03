# Inventory

`inventory` measures immediate contents of a root as read-only capacity
observations. It never creates cleanup targets and its output cannot be passed
to `clean` as a plan.

```powershell
cargo run --locked --bin devsweep -- inventory [ROOT]
```

Use `--json` when another tool needs the report:

```powershell
cargo run --locked --bin devsweep -- inventory C:\code --json > inventory.json
```

## What the report means

Each observation has a path, an estimated size, and a completeness indication.
The report also carries scan-style health diagnostics and totals. A size may be
verified, a partial lower bound, or unknown; do not treat a lower bound as an
exact capacity figure.

For pnpm, inventory can surface an inspect-only orphan-store finding. That is a
capacity observation, not permission to clean the store.

Use [scan](/guide/scan) when you want to discover candidates according to
DevSweep's cleanup rules. Use inventory when the goal is observation only.
