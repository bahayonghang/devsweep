# Scan

`scan` discovers supported cleanup candidates and emits either a concise text
summary or a structured scan report.

```powershell
cargo run --locked --bin devsweep -- scan [ROOT]...
```

## Choose the scope

With neither scope flag, DevSweep includes both project targets and global
providers. Use one flag when you need a narrower scan:

```powershell
# Scan project artifacts below a repository root.
cargo run --locked --bin devsweep -- scan --projects C:\code\my-project

# Scan supported global providers and caches.
cargo run --locked --bin devsweep -- scan --global
```

`--projects` considers project-level artifacts such as `target/`,
`node_modules`, virtual environments, and supported tool caches. `--global`
considers supported package-manager providers and inspect-only locations. The
[rules reference](/reference/rules) shows the current catalog.

## Save JSON for review

Use `--json` to write the full report:

```powershell
cargo run --locked --bin devsweep -- scan . --json > plan.json
```

The document includes a versioned plan, health completeness, diagnostics, and
separate verified, partial-lower-bound, and unknown-size totals. It is suitable
for a later [`clean --plan`](/guide/clean) invocation after review.

## Re-estimate one target

If a scan reports an incomplete size estimate, request a higher-budget
re-estimation for a target from that live scan:

```powershell
cargo run --locked --bin devsweep -- scan --projects --rescan-target "python.__pycache__:C:/code/app/__pycache__" C:\code
```

The target ID must be present in the current scan. This option is not a way to
ask DevSweep to inspect an arbitrary path.

## Read the result conservatively

A partial lower bound is not an exact size. Diagnostics explain skipped,
canceled, or incomplete work. Resolve uncertainty before deciding whether a
target belongs in a cleanup execution.
