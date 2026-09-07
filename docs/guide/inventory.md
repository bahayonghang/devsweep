# Inventory

Disk-capacity observation is `analyze scan`. The old root `inventory` is
rejected (exit 2). This command never creates cleanup targets, and its output
cannot be passed to `clean` as a plan.

```powershell
devsweep analyze scan --root PATH
```

Use `--format json` when another tool needs the report, and optional create-new
`--output`:

```powershell
devsweep analyze scan --root C:\code --format json --output analyze.json
```

Installed-software listing is a different mode: [`software inventory`](/guide/software).

## What the report means

Analyze walks one root and emits a versioned snapshot of nodes, accounted size,
and warnings. A size may be verified, a partial lower bound, or unknown; do not
treat a lower bound as an exact capacity figure. The document has no cleanup
intent and no executable action.

Use [scan](/guide/scan) (`clean scan`) when you want to discover candidates
according to DevSweep's cleanup rules. Use Analyze when the goal is observation
only.
