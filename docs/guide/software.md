# Software inventory and uninstall

Software mode inventories installed-software evidence without turning display names, publisher text, registry uninstall strings, or installed paths into cleanup authority. Run it as a standard user. DevSweep does not request elevation for this flow.

Only an exact, healthy current-user MSIX identity can be selected in Software V1. MSI and ARP entries remain visible but manual, including machine and per-user MSI contexts. Each row labels reported estimates, measured installed-location evidence, partial lower bounds, or unknown size. Last-used information is always shown as unknown because V1 has no supported exact source.

An uninstall requires two explicit steps: first review a freshly revalidated preview and its digest, then confirm the irreversible action. DevSweep cannot restore or reinstall removed software. A size value is inventory evidence, not a promise that the same amount of disk space will be freed.

Execution records one closed outcome per exact identity: removed, reboot required, still present, failed, or unknown after dispatch. Cancellation before dispatch is recorded separately. On restart, DevSweep re-queries an interrupted dispatch from the Software audit journal and does not dispatch the removal again. The journal is stored under the current user's local application-data directory at `DevSweep/audit/software-v1.jsonl`.

Before exercising uninstall in a native build, use a disposable, user-confirmed MSIX target. Inventory and preview are read-only; never substitute an arbitrary installed package as an uninstall test target.
