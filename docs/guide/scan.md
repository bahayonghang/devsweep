# Scan

`clean scan` discovers supported cleanup candidates and emits either a concise
text summary or a structured observation. The observation is not an executable
plan.

```powershell
devsweep clean scan --root PATH --scope <projects|global|all>
```

`--root` may be repeated. The default root is `.` and the default scope is
`all`.

## Choose the scope

```powershell
# Scan project artifacts below a repository root.
devsweep clean scan --root C:\code\my-project --scope projects

# Scan supported global providers and caches.
devsweep clean scan --root . --scope global

# Scan both.
devsweep clean scan --root C:\code\my-project --scope all
```

`--scope projects` considers project-level artifacts such as `target/`,
`node_modules`, virtual environments, and supported tool caches.
`--scope global` considers supported package-manager providers and inspect-only
locations. The [rules reference](/reference/rules) shows the current catalog.

## Save JSON for review

Use `--format json` and a create-new `--output` file. Do not overwrite an
existing path; `-` is not a file sentinel.

```powershell
devsweep clean scan --root . --scope all --format json --output observation.json
```

The machine document is a V1 envelope `{schema_version,command,outcome,data,warnings,error}`.
`data` is the scan report: a versioned observation with an embedded untrusted
plan, health completeness, diagnostics, and separate verified, partial-lower-bound,
and unknown-size totals.

That file is suitable as `--observation` for [`clean plan`](/guide/clean). It is
**not** a runnable plan. Do not pass it to `clean preview` or `clean execute`.

## Re-estimate one target

If a scan reports an incomplete size estimate, request a higher-budget
re-estimation for a target from that live scan. Rescan requires exactly one
`--root` and a scope that includes projects:

```powershell
devsweep clean scan --root C:\code --scope projects --rescan-target "python.__pycache__:C:/code/app/__pycache__"
```

The target ID must be present in the current scan. This option is not a way to
ask DevSweep to inspect an arbitrary path.

## Read the result conservatively

A partial lower bound is not an exact size. Diagnostics explain skipped,
canceled, or incomplete work. Resolve uncertainty before selecting a target
into a new plan.

## Desktop live preview

The desktop workbench shows an indeterminate phase progress bar and cumulative
targets while a scan is running. These rows are incomplete, read-only
observations grouped by project and global scope. They cannot be selected or
sent to preview. Only the completed scan report enables selection and cleanup
review. If a scan is canceled or fails, the latest preview remains visible as
partial evidence without becoming cleanup authority.
After a stopped rescan, use **Return to previous report** to leave the read-only
preview and restore the last completed report's default review selection.
