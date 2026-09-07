# Software inventory and uninstall

Software mode inventories installed-software evidence without turning display names, publisher text, registry uninstall strings, or installed paths into cleanup authority. Run it as a standard user. DevSweep does not request elevation for this flow.

Only an exact, healthy current-user MSIX identity can be selected in Software V1. MSI and ARP entries remain visible but manual, including machine and per-user MSI contexts. Each row labels reported estimates, measured installed-location evidence, partial lower bounds, or unknown size. Last-used information is always shown as unknown because V1 has no supported exact source.

Software follows the same authority chain as Clean:

save inventory → exact select → save **new** plan → live preview digest → `--confirm`.

Inventory JSON is not a runnable plan. `software preview` and `software uninstall` require the new plan file.

```powershell
devsweep software inventory --source all --format json --output inventory.json
devsweep software plan --inventory inventory.json --select SOFTWARE_ID --output software-plan.json
devsweep software preview --plan software-plan.json
```

Read `SOFTWARE_ID` values from `data.entries[].id` in the V1 inventory envelope. Only an eligible current-user MSIX identity can be planned.

Uninstall is irreversible. DevSweep cannot restore or reinstall removed software. A size value is inventory evidence, not a promise that the same amount of disk space will be freed. Confirm the grammar with `--help`; do not run uninstall as a first-run or documentation check:

```powershell
devsweep software uninstall --help
devsweep software uninstall --plan software-plan.json --preview-digest sha256:DIGEST --confirm
```

Execution records one closed outcome per exact identity: removed, reboot required, still present, failed, or unknown after dispatch. Cancellation before dispatch is recorded separately. On restart, DevSweep re-queries an interrupted dispatch from the Software audit journal and does not dispatch the removal again. The journal is stored only at `%LOCALAPPDATA%\DevSweep\audit\v1\software.jsonl`. Legacy `%APPDATA%\devsweep\audit.jsonl` is not used or imported.

Before exercising uninstall in a native build, use a disposable, user-confirmed MSIX target. Inventory and preview are read-only; never substitute an arbitrary installed package as an uninstall test target.
