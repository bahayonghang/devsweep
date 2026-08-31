# Root programme acceptance review

- Date: 2026-09-01
- Reviewer role: coordination closeout for `08-29-mole-inspired-cli-windows-desktop-redesign`
- Review type: planning/integration gate; no product-code ownership; no `task.py start`
- Tree: root `task.json.children` recursive closure is exactly 22; `MISSING=[]`
- HEAD at review start of this file: `e44821c` (`chore(task): archive 08-29-status-mode`)
- Protected out-of-scope dirt retained uncommitted: `justfile` (and any pre-start `README.md` hunks). `.trellis/.gitignore` was not modified by this programme closeout.

This review traces every root PRD requirement and acceptance criterion to child
archives, parent `acceptance.md` overall `PASS` records, and the five-mode
integration independent check. It does not reopen archived decisions that later
integration evidence did not overturn.

## Tree and archive ledger

| Dir | Kind | Archive path | Archive commit |
| --- | --- | --- | --- |
| `08-29-cli-contract-localization` | leaf | `.trellis/tasks/archive/2026-08/08-29-cli-contract-localization/` | `f562a73` |
| `08-29-bound-sizing-concurrency` | leaf | `.trellis/tasks/archive/2026-08/08-29-bound-sizing-concurrency/` | `259b57b` |
| `08-29-generated-app-icon-integration` | leaf | `.trellis/tasks/archive/2026-08/08-29-generated-app-icon-integration/` | `d79041f` |
| `08-29-scan-resource-bounds-app-icon` | parent | `.trellis/tasks/archive/2026-08/08-29-scan-resource-bounds-app-icon/` | `15930b0` |
| `08-29-desktop-shell-navigation-brand` | leaf | `.trellis/tasks/archive/2026-08/08-29-desktop-shell-navigation-brand/` | `5fa2264` |
| `08-29-clean-mode-workbench` | leaf | `.trellis/tasks/archive/2026-08/08-29-clean-mode-workbench/` | `3b2ffb6` |
| `08-29-analyze-core-ipc` | leaf | `.trellis/tasks/archive/2026-08/08-29-analyze-core-ipc/` | `392909d` (re-archive after repair) |
| `08-29-analyze-tui-desktop-treemap` | leaf | `.trellis/tasks/archive/2026-08/08-29-analyze-tui-desktop-treemap/` | `5d10394` |
| `08-29-analyze-mode` | parent | `.trellis/tasks/archive/2026-08/08-29-analyze-mode/` | `2f10512` |
| `08-29-protect-rules-history-surfaces` | leaf | `.trellis/tasks/archive/2026-08/08-29-protect-rules-history-surfaces/` | `dfdf5bb` |
| `08-29-software-inventory-plan` | leaf | `.trellis/tasks/archive/2026-08/08-29-software-inventory-plan/` | `b650128` |
| `08-29-software-execution-audit` | leaf | `.trellis/tasks/archive/2026-08/08-29-software-execution-audit/` | `ed0803a` |
| `08-29-software-cli-tui-desktop-native` | leaf | `.trellis/tasks/archive/2026-08/08-29-software-cli-tui-desktop-native/` | `a8fd585` |
| `08-29-software-mode` | parent | `.trellis/tasks/archive/2026-09/08-29-software-mode/` | `8634617` |
| `08-29-optimize-catalog-execution` | leaf | `.trellis/tasks/archive/2026-08/08-29-optimize-catalog-execution/` | `6861a65` |
| `08-29-optimize-cli-tui-desktop-native` | leaf | `.trellis/tasks/archive/2026-08/08-29-optimize-cli-tui-desktop-native/` | `7f70f16` |
| `08-29-optimize-mode` | parent | `.trellis/tasks/archive/2026-09/08-29-optimize-mode/` | `4b5dd6d` |
| `08-29-status-collector-cli` | leaf | `.trellis/tasks/archive/2026-08/08-29-status-collector-cli/` | `edb42f4` |
| `08-29-status-tui-desktop-native` | leaf | `.trellis/tasks/archive/2026-08/08-29-status-tui-desktop-native/` | `25b83d8` |
| `08-29-status-mode` | parent | `.trellis/tasks/archive/2026-09/08-29-status-mode/` | `e44821c` |
| `08-29-five-mode-native-integration` | leaf | `.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/` | `bd53bd6` |
| this root | planning parent | still active until after this review is committed | n/a |

Required parent `acceptance.md` overall records while those parents were active,
now archived:

| Parent | Overall |
| --- | --- |
| `08-29-scan-resource-bounds-app-icon` | **PASS** |
| `08-29-analyze-mode` | **PASS** (round 2 supersedes round-1 FAIL) |
| `08-29-software-mode` | **PASS** |
| `08-29-optimize-mode` | **PASS** |
| `08-29-status-mode` | **PASS** |

Integration product `760707d`, planning `552a139`, archive `bd53bd6`. Independent
check **PASS** at
`.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/evidence/independent-check-report.md`.
Returned domain defects were repaired in owning product commits `fac3e8b`
(Software overlap) and `1cda3f2` (Analyze walker yield) without reopening the
three baseline archives (`f562a73` / `259b57b` / `d79041f`).

## Root requirement verdicts

| Clause | Verdict | Evidence |
| --- | --- | --- |
| R1 clean-room | PASS | `research/mole-reference-audit.md` pins Mole `f92133a4d6277574177e0b1284742072fb3b5bdc`, GPL/trademark boundary, screenshot provenance, and accept/adapt/reject. No Mole source shipped. |
| R2 CLI taxonomy | PASS | Archived `08-29-cli-contract-localization` plus integration glue in `crates/devsweep-cli/src/application/mod.rs` and `docs/guide/cli-migration.md`. Old roots removed; five-mode + history. |
| R3 desktop IA | PASS | Archived desktop-shell, Clean, Analyze, Software, Optimize, Status, Protect/Rules/History. Five modes are real contracts; History is support-only. |
| R4 safety | PASS | Child safety contracts plus integration independent check: dry-run default, no permanent delete, Cargo home inspect-only, no Docker cleanup, program/argv separate. |
| R5 Windows visual | PASS | Archived desktop-shell brand/spec-first palette; native bilingual widths and scaling in integration native matrix. |
| R6 foundation | PASS | Scan parent retained; sizing and icon archived at baseline `259b57b` / `d79041f`; React shell owned by desktop-shell (`5fa2264`). |
| R7 five-mode ownership | PASS | Distinct children for CLI, shell, Clean, Analyze core/UI, Software three-leaf, Optimize two-leaf, Status two-leaf, Protect, integration. |
| R8 Software trust | PASS | Software parent acceptance **PASS**. Five terminals, no `partial` execution terminal, MSI manual, current-user MSIX only, last-used `unknown/no_supported_exact_source`. Real uninstall remains sanctioned UNVERIFIED (not completion-required). |
| R9 Optimize catalogue | PASS | Optimize parent acceptance **PASS**. Closed eight-id catalogue; DNS `ipconfig` + `[/flushdns]`; Settings handoffs; no UAC. |
| R10 Status sampling | PASS | Status parent acceptance **PASS**. Frozen `StatusSnapshotV1` / `AvailabilityV1`; live cancel/join; sleep/resume sanctioned UNVERIFIED. |
| R11 breaking CLI | PASS | CLI child archive plus migration guide; `clean --audit-log` not converted. |
| R12 bilingual | PASS | CLI/TUI/desktop catalogues; native EN/ZH + 100/125/150/200% in integration screenshots. |
| R13 resource protocol | PASS | Independent `five-mode-v1` recapture exit 0: Analyze CPU p95 190.16% (≤200); Software p95 3718 ms (≤5 s); Optimize list before Analyze; Clean live-repo ratio vs archived two-worker median 39999.052 ms; react-commit p95 6 ms on Chrome free port 2052. Round-4 padded `p95cpu=0` is not used. |

## Root acceptance-criterion verdicts

| Clause | Verdict | Evidence |
| --- | --- | --- |
| AC1 (R1) | PASS | `research/mole-reference-audit.md` and sibling research notes. |
| AC2 (R2) | PASS | CLI child command matrix and `docs/guide/cli-migration.md`. |
| AC3 (R2, R3) | PASS | Design architecture plus implemented mode owners; Installer remains rejected. |
| AC4 (R4) | PASS | Separate plan/preview/digest/execute domains per mutating mode; Analyze/Status read-only. |
| AC5 (R5) | PASS | Desktop-shell spec-first update archived before mode UI. |
| AC6 (R6) | PASS | Foundation subtree archived; sizing paused/partial recorded in that parent then completed as bound-sizing; icon React ownership not in icon child. |
| AC7 (R7) | PASS | This 22-node graph; file/contract ownership in child `implement.md` files. |
| AC8 (R7) | PASS | Child `task.py validate` and independent plan reviews preceded each implementation leaf `task.py start`. Coordination parents were never started. |
| AC9 (R8) | PASS | Software parent AC1–AC3 **PASS** with sanctioned uninstall UNVERIFIED only. |
| AC10 (R9) | PASS | Optimize parent AC1–AC3 **PASS**. |
| AC11 (R10) | PASS | Status parent AC1–AC3 **PASS**. |
| AC12 (R11) | PASS | CLI child plus integration CLI contract tests `crates/devsweep-cli/tests/five_mode_contract.rs`. |
| AC13 (R12) | PASS | Localization child plus per-mode native bilingual/scaling captures in integration `evidence/native/`. |
| AC14 (R13) | PASS | Independent `gates.json` / `summary.json` / `host-manifest.json` under integration `evidence/resources/independent-check/`; protocol exit 0; no completion-required UNVERIFIED. |

## Closeout constraints

- No push, publish, sign, or GitHub release.
- Archive commits after this planning commit must contain only `task.py`
  status/move for this root.
- `git add -f .trellis/` remains forbidden.
- Software MSI remains inventory + manual; V1 last-used stays
  `unknown/no_supported_exact_source`.

## Overall verdict

**PASS**
