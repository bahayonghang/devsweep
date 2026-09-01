# Confirmed clean

Inspect permission is not execute permission. A request such as "清理 low 和
medium" is selection criteria for a list. It is not execute authority.

## Program

Use only the PATH install printed by `scripts/resolve_devsweep.py`. That
binary must not live under this repository's `target/` tree.

Do not select Cleanup Targets whose path is inside the current repository
unless the user names those ids after seeing them.

## List and wait

1. After a Scan Report exists, run:

```text
python scripts/list_selectable.py <observation.json> --risk low,medium --exclude-root <repo-root> --ids-out <ids.json>
```

2. Show the markdown table in the user reply. Include id, path, evidence,
   risk, Estimated Recoverable class, and advice.
3. Stop. Ask the user to confirm the listed ids, or to name a subset.
4. Do not run `clean plan`, `clean preview`, or `clean execute` in that turn.

Ranking, default selection, and risk filters are not confirmation.

## After confirmation

1. Build untrusted Cleanup Plans with program and argv separate:

```text
python scripts/plan_selected.py --devsweep <global-exe> --observation <observation.json> --ids <ids.json> --output-prefix <new-prefix>
```

Skip ids that `validate_plan` rejects. Report them. Do not invent replacements.

2. For each saved plan, run `clean preview --plan <FILE> --format json --output <new-file>`.
3. Show the preview digest and the selected list again.
4. Run `clean execute --plan <FILE> --preview-digest sha256:<digest> --confirm --format json --output <new-file>` only for plans the user confirmed.
5. Report succeeded, failed, and skipped ids. Trash-backed cleanup does not
   make capacity available until the user empties trash. Do not write
   "space freed".

Batch `--select` when Windows argv length requires it. Keep each `--output`
path new; never overwrite.

## Still forbidden

- `optimize run`, `software uninstall`, protection mutations
- `rm`, `Remove-Item`, `Clear-RecycleBin`
- Cargo home, Docker, unresolved provider paths
- Treating a preview digest as user confirmation
