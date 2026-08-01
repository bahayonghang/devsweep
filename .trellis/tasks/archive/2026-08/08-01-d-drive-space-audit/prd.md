# Audit D drive space and cleanup candidates

## Goal

Identify what consumes space on the current system's `D:` volume and provide an evidence-backed table of cleanup candidates. The audit must use devsweep's scanner where it applies and a read-only filesystem inventory for the complete volume.

## Confirmed Facts

- The live `D:` volume is 1,907.73 GiB total, with 1,450.85 GiB used (76.05%) and 456.88 GiB free. This is consistent with the supplied Explorer screenshot, which rounds the values to 1.86 TB total and 458 GB free.
- `devsweep scan` emits a cleanup plan; it does not execute cleanup. Cleanup execution is only reachable through `devsweep clean --plan <path> --execute`.
- The scanner recognizes developer-project artifacts and caches; it is not a general-purpose large-file classifier. A filesystem inventory is therefore required to explain all occupied D: space.
- The existing repository safety policy prohibits permanent deletion in this build. Cargo home is inspect-only.
- The complete D: project-rule scan, `cargo run --quiet --bin devsweep -- scan --projects --json D:\`, returned 657 targets with 688.777 GiB of logical estimates. Only 643 targets / 264.658 GiB have complete measurements; 14 targets / 424.119 GiB are lower-bound-only estimates and must not be presented as a reclaim promise.
- The full scan emitted 200 Cargo metadata warnings (186 invalid JSON parses and 14 nonzero exits) to stderr. The JSON plan contains target-level `size_complete`, but does not preserve the scan-level diagnostics or completeness state.
- `D:\Documents\LYH` is a Cloud Files reparse point (tag `0x9000701a`). The full project scan entered it and emitted 34 targets / 8.026 GiB, despite the documented reparse-point skip contract. These paths are excluded from cleanup recommendations pending a fail-closed fix.

## Audit Evidence

- A read-only root inventory found `D:\Documents` at 1,308.97 GiB logical size, followed by `D:\SteamLibrary` at 108.81 GiB, `D:\BaiduNetdiskDownload` at 59.21 GiB, `D:\Music` at 35.27 GiB, `D:\$RECYCLE.BIN` at 27.52 GiB, and `D:\.pnpm-store` at 8.77 GiB. Logical tree sizes are not summed into the volume total because sparse files, hard links, and skipped reparse points can differ from allocated disk use.
- `D:\Documents\LYH` is a reparse point and was deliberately not followed. The inventory recorded no access errors except the protected `D:\System Volume Information` root.
- `cargo run --quiet --bin devsweep -- scan --projects --json D:\Documents\Code` completed without invoking `clean`. Its valid JSON evidence is `research/devsweep-code-scan-20260801-1516.json`.
- devsweep found 428 project cleanup candidates with 665.34 GiB estimated logical size. Of those, 415 candidates have complete sizing (242.58 GiB); 13 candidates are incomplete (422.76 GiB) and are not suitable for an automatic recommendation.
- `cargo run --quiet --bin devsweep -- scan --projects --json D:\` completed without invoking `clean`. Its complete per-target plan is `research/devsweep-d-drive-project-scan-20260801.json` (SHA-256 `C1A32CD95DD1EF6AE59C3801ADFCA78F710C664ECAEA37E8915785E69751D1E2`); its stderr warnings are retained as `research/devsweep-d-drive-project-scan-20260801.stderr.log`.
- `research/devsweep-full-d-drive-comparison.md` compares the two scans with the read-only inventory and records prioritized rule/safety improvements. It does not authorize source changes or cleanup execution.
- The report `research/d-drive-audit-findings.md` records the candidate tiers, evidence limits, and approval contract. A zero-byte file from a failed ambiguous `cargo run` invocation is intentionally retained as `research/devsweep-code-scan.json` under the no-deletion constraint.

## Requirements

- R1. Inspect only the current `D:` volume. Do not inspect, clean, or change other volumes unless required to resolve a reparse point; reparse points are skipped rather than followed.
- R2. Run a read-only size inventory that reports the largest top-level roots and drills into the largest roots as needed. Record inaccessible locations and do not treat incomplete measurements as exact.
- R3. Use `devsweep scan --projects --json` against identified developer-workspace roots to discover supported cleanup targets. Do not run `devsweep clean`, do not use `--execute`, and do not alter the protection list.
- R4. Produce a candidate table with path, observed size, category, evidence, reclaim estimate, reversibility/risk, and a recommendation. Clearly distinguish safe supported candidates from items that need manual review or must be retained.
- R5. Preserve all existing working-tree changes and do not modify product source code.
- R6. Do not delete, move, trash, compress, uninstall, empty a recycle bin, or execute any cleanup command. A later cleanup action requires the user's explicit approval of the selected row or rows.
- R7. Compare the D: project-rule scan with the filesystem inventory and identify rule, safety, diagnostic, and presentation improvements without broadening automatic deletion scope.

## Acceptance Criteria

- [x] The report states the live `D:` total, used, and free capacity and identifies the largest observed consumers.
- [x] The report separates general storage consumers from devsweep-recognized cleanup candidates and reports any inaccessible/skipped paths.
- [x] Every proposed cleanup row has a path, measured or estimated reclaimable size, rationale, risk/reversibility, and a no-action default.
- [x] No deletion, move, recycle-bin operation, uninstall, or cleanup execution occurs during the task.
- [x] The task remains in planning until the user reviews the findings; no `task.py start` is run as part of this audit.
- [x] The full D: project-rule scan is retained as a complete raw JSON artifact and compared with the earlier Code-only scan and volume inventory.
- [x] The rule assessment distinguishes required safety fixes from optional coverage and presentation improvements.

## Out Of Scope

- Executing a cleanup plan or selecting targets on the user's behalf.
- Deleting user documents, media, archives, source repositories, installed applications, Cargo home contents, or system-managed data.
- Changing devsweep source code, configuration, or safety policy.
- Inventorying other local, removable, network, or cloud volumes.

## Notes

- This is a lightweight PRD-only read-only audit. The separate approval gate applies before any eventual cleanup task, and the discovered Cloud Files reparse-point gap blocks recommendations for paths below that root.
