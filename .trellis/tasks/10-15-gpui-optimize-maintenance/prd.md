# C06 Bounded Windows maintenance through the CLI

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md).

## Goal

Extend the closed maintenance catalogue with preflighted native repairs and honest per-action outcomes.

## Requirements

C06-R1. Keep the eight catalogue v1 ids and their classes. Add catalogue v2 rows
below. `optimize list` prints every row with class `execute`, `settings_handoff`,
`guidance`, or `observe`. A v1 plan still resolves only its original id.

C06-R2. Implement these execute rows. Program and argv stay separate. No shell
command string.

| Id | Program and argv | Precheck refuse |
| --- | --- | --- |
| `maintenance.retrim` | `defrag.exe` `/L` `<volume>` once per ready fixed SSD | low battery, not SSD, disk problem, reboot pending, defrag task ran inside the cooldown |
| `maintenance.clock_probe` | `w32tm.exe` `/stripchart` `/computer:time.windows.com` `/samples:3` `/period:1` `/dataonly` | `w32tm.exe` missing |
| `maintenance.clock_sync` | `w32tm.exe` `/resync` | reboot pending, `w32tm.exe` missing |
| `maintenance.image_scan` | `dism.exe` `/Online` `/Cleanup-Image` `/ScanHealth` | reboot pending, `dism.exe` missing |
| `maintenance.image_repair` | `dism.exe` `/Online` `/Cleanup-Image` `/RestoreHealth` | preview health check is not repairable, reboot pending, `dism.exe` missing |
| `maintenance.dead_startup` | startup core from C05: delete only Run values and Startup `.lnk` files whose targets are gone, after writing a `.reg` backup | backup write failed. An entry whose target still exists is skipped, not deleted |

C06-R3. Add catalogue row `maintenance.disk_health` with class `observe`, and
`devsweep optimize health` as its read-only command. It prints device id and
health status. `optimize plan` refuses this id.

C06-R4. These rows are guidance and have no executor: `maintenance.update_cache`,
`maintenance.icon_cache`, `maintenance.search`, `maintenance.winget_repair`.
Update-cache deletion would be permanent delete. Icon-cache recovery kills
shell processes. Search repair restarts `WSearch` without a recovered safe
argv. Winget repair named `winget.exe` but no argv was recovered. `optimize plan`
on a guidance id returns the guidance class and does not execute.

C06-R5. Preflight privilege, AC/battery, update/reboot state, volume capability
and operation cooldown. Report the skip reason on that row only.

C06-R6. One saved plan contains one operation id. The GPUI button reviews the
selected ids and then runs one confirmed plan per id. Image scan does not start
image repair. Clock probe does not start clock sync.

C06-R7. Record cooldown in the existing optimize audit. Do not add `optimize.json`.

## Acceptance criteria

- [x] C06-AC1: `optimize list --format json` returns the eight v1 rows plus the v2 rows in this PRD, each with its class. Guidance and observe rows have no mutation executor. A v1 plan for `dns.flush` still previews.
- [x] C06-AC2: UAC refusal, unsupported hardware, low battery, ongoing update, cooldown and changed preconditions prevent only the affected action and report the reason.
- [x] C06-AC3: Cancel/close does not force-kill a system repair; already-dispatched work remains waited on or reported unknown/reboot-needed.
- [ ] C06-AC4: Fake runners plus disposable native Windows evidence prove side effects and requery; no productivity-machine repairs are used as tests. `fake_runner_requery_omits_deleted_dead_startup_name` records a MemStore deletion and a later requery; disposable native Windows evidence of defrag/dism/w32tm remains NOT_RUN.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
