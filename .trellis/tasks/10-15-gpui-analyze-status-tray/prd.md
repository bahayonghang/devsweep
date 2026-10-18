# C13 Native analysis status and tray surfaces

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C07](../10-15-gpui-analyze-capabilities/prd.md), [C08](../10-15-gpui-status-capabilities/prd.md), [C10](../10-15-gpui-shell-assets/prd.md).

## Goal

Build scalable disk visualization, live status/process views and a lifecycle-aware Windows tray.

## Requirements

C13-R1. Analyze states are Landing, Scanning, Detail, and Failed. Landing chooses a drive or folder. Detail shows treemap plus accessible list, breadcrumbs, large-item explanations and reviewed reveal/recycle actions.

C13-R2. Status states are Landing and Live. Live shows supported health/CPU/GPU/memory/battery/space/network/thermal groups and a searchable sortable process list with local explanations. Missing sensors stay unavailable.

C13-R3. Tray provides an optional compact status panel from shared sampling; no second independent sampler while the main panel is visible.

C13-R4. Present Keep Screen On controls over C08's accepted CLI/service lease contract. Add a presentation-only Clean Screen session after native lifecycle design: bounded ownership, visible active state, escape shortcut and guaranteed restoration on exit.

## Acceptance criteria

- [ ] C13-AC1: Treemap/list share a stable node identity and accessible selection; unreadable content is distinct from empty and recycle uses the same CLI authority.
- [x] C13-AC2: Unavailable sensors remain honest; main/tray subscriptions start/stop predictably and hidden panels add no decorative work.
- [ ] C13-AC3: Owned Keep Screen On state releases on normal exit/crash as supported by OS; Clean Screen always has a tested emergency escape and never locks input permanently.
- [ ] C13-AC4: Large snapshot rendering, 20 open/close/minimize cycles, native UIA and CLI/service parity pass; no process kill based on displayed PID alone.

`clean_screen_escape_restores_view_and_releases_lease` covers normal stop/Drop release and the Escape shortcut with `RecordingBackend`. An OS crash release of the real execution state remains NOT_RUN.

`analyze_list_and_treemap_share_ids` covers in-memory shared list/treemap node identity and empty vs unreadable vs present classification. Accessible UIA selection remains NOT_RUN.

`analyze_recycle_ids_match_cli_trash_plan` covers recycle selection using the same node id the CLI trash plan records. UIA selection remains NOT_RUN.

`analyze_trash_preview_digest_binds_recycle_ids` covers recycle_ids binding to a repeated CLI `analyze trash-preview` `data.digest`. UIA selection remains NOT_RUN.

`status_tray_shares_one_subscription` covers honest unavailable/permission-denied/unsupported sensor lines, one shared main/tray subscription start/stop, and Hidden presence with no decorative frames. This does not start a live PDH sampler.

`displayed_pid_does_not_kill` covers a displayed PID that is not termination authority.

**20-cycle native lifecycle measurement is now recorded (2026-10-18).** Twenty consecutive cycles of create -> window -> `SW_MINIMIZE` -> confirm iconic -> `SW_RESTORE` -> confirm restored -> close, on release SHA-256 `DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34` from an empty temp directory:

| Metric | Value |
| --- | --- |
| Cycles completed | 20 / 20 |
| Median open-to-window | 453 ms |
| Max open-to-window | 476 ms |
| Minimized / restored every cycle | yes |
| Exited on close every cycle | yes |
| Leftover processes every cycle | **0** |

No orphan accumulation across cycles. Evidence: `evidence/c13-cycles-round4.md`.

An earlier run of the same harness reported `leftover=1` for all 20 cycles. That was a defect in the measurement, not in the app: the owned-tree helper counted the root PID without checking whether it was still alive. The corrected helper counts only live processes, and the re-run reports 0 everywhere. Both runs are retained in the evidence rather than the failing one being erased.

UIA selection and CLI/service parity remain **NOT_RUN**. UIA selection is blocked by the same focus defect recorded under C09-AC2: `view.rs:753` pins keyboard focus to the Note input, so the shell's key context never activates and no assistive-technology user can move a selection.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.

