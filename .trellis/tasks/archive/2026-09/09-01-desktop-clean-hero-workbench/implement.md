# Implement - Clean hero workbench

1. Confirm spec-shell canvas tokens exist.
2. Map idle/scanning/reviewed/reported to new views without changing
   reducer authority.
3. Group review rows by `kind` then `scope`. Keep TargetTable selection
   rules.
4. Restyle ScanPreviewPage and ExecutePage onto the dark canvas.
5. Update tests and 390/800/1024/1440 CSS checks for Clean.
6. Desktop lint/typecheck/test/build.

Rollback: restore Clean pages/CSS/tests only.
