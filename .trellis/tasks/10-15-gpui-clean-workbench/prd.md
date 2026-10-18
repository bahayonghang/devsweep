# C11 Native cleanup workbench

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C03](../10-15-gpui-clean-coverage/prd.md), [C10](../10-15-gpui-shell-assets/prd.md), [C15](../10-15-gpui-ai-tool-cleanup/prd.md).

## Goal

Build the GPUI review-and-clean workflow over accepted CLI/core capabilities.

## Requirements

C11-R1. Provide independent global-cache/project-artifact scans, first-use root setup, retained reports, coverage diagnostics and actionable progress. Landing shows the Earth still and one primary button. The button chooses global or project scope before a scan starts. Later states are Scanning, Found, AllClean, Cleaning, Completed, and Failed.

C11-R2. Present provider groups/project ownership, occupancy/cost/confidence, search/sort, bounded rows and full-report selection.

C11-R3. Use a staged review: selection, live preview, explicit confirmation, execution, truthful result/history; invalidate on all relevant changes.

C11-R4. Expose protection/ignore actions and rebuild-cost explanation without hiding partial or inspect-only items.

C11-R5. Include AI-tool review from C15 with separate cache/session/worktree categories, explicit sensitive-data selection and recovery status.

## Acceptance criteria

- [ ] C11-AC1: The same isolated inputs and selection produce equal CLI/GPUI preview digests and outcome semantics, including C15 AI cleanup/recovery contracts and fixture identities.

  Named test: `pytest_cache_confirmation_digest_matches_core`. The CLI preview digest matches the GPUI helper on this fixture. Named test: `ai_preview_digest_matches_cli`. The C15 AI preview digest matches between CLI `data.digest` and the GPUI helper wrapping `preview_ai`. Named test: `ai_recovery_identity_matches_cli`. Recovery-list and restore-plan identities match between CLI and the GPUI helpers wrapping `recovery_list` and `restore_plan`. Named test: `ai_execute_gate_stays_inside_temp`. The isolated digest gate and recovery bundle passed. Named test: `clean_outcome_lines_render_in_panel_lines`. These panel lines are not UI outcome parity and the real Recycle Bin remains NOT_RUN. Named test: `ai_cli_stale_execute_matches_notice`. This is the stale refusal only; UI outcome parity and the real Recycle Bin remain NOT_RUN.
- [ ] C11-AC2: 10k-item search/select/scroll and cancel stay responsive with bounded rows; selections survive navigation and do not miss offscreen rows.

  Named test: `clean_row_window_keeps_offscreen_selection`. Named test: `clean_search_filters_without_dropping_selection`. Search responsiveness remains NOT_RUN. Performance-protocol responsiveness remains NOT_RUN.
- [ ] C11-AC3: Unavailable, empty, partial, failed, canceled, stale preview, mixed execution and Recycle Bin results have usable native UI states.

  Named test: `mixed_execute_report_is_not_trash_reported`. The real Recycle Bin UI remains NOT_RUN.
- [ ] C11-AC4: Keyboard and UIA can complete the review flow; no automatic broad machine scan runs on first launch.

  Named test: `first_launch_does_not_scan`. One Down key on foreground PID 93804, binary SHA-256 `793245A0E369011E362BB8641FE1A6D42CB8DC12B3B9A25267B6540A1B4A88B5`, changed the selected pill from Clean to Software and exited 0 after `WM_CLOSE`. Screenshot: `evidence/c11-down-mode.png`. `alt_down_moves_clean_selection_without_changing_mode` exited 0: two `RowDown` calls select the second of three injected Clean ids and leave the mode on Clean; `Down` then switches to Software. `isolated_project_rows_apply_after_drain` exited 0. PID 110520, SHA-256 `A664EDD107B7488047972416049180013DEB787D7A3410B827AF70DE74E10C8F`, 14099456 bytes, showed one `python.pytest_cache` row for an isolated `devsweep-gpui-c11-` project after one F9, stayed on Clean after Alt+Down, and exited 0. Screenshots: `evidence/c11-f9-rows.png` and `evidence/c11-f9-altdown.png`. That build log contains `incremental`. One row cannot show a move to a second id. A later binary SHA-256 `E96C7024DAC8E6E63234AB3CAD2BC80622C4B051A0E5CEFC61B981874C0ACD3B`, 14099968 bytes, build exit 0, also contains `incremental`. PID 87176 showed both pytest and mypy rows for one isolated `devsweep-gpui-c11-` project. The first Alt+Down painted `> python.pytest_cache`. The second moved the marker to `> python.mypy_cache` while Clean stayed selected, then `WM_CLOSE` exited 0. Screenshots: `evidence/c11-mark-1.png` and `evidence/c11-mark-2.png`. The F9 capture was covered by another window and is not evidence. UIA review remains NOT_RUN. PID 92836 on the unlabeled binary did not expose those names. PID 55284 on SHA-256 `8DBEF9254DB55809BE22C8383E0BBF382B2A311887B8803E26E3B87A3200A975`, 14100480 bytes, also received F9 and two Alt+Down chords and exited 0, and the same name search was false for both caches. Log: `evidence/c11-aria-window.txt`. That name search is not repeated. PID 54516 on that same binary painted both cache rows, with `> ` on the mypy line, and exited 0. Screenshot: `evidence/c11-aria-paint.png`. The painted text is not a UI Automation name. PID 94444, SHA-256 `6302E6365A050BDEEAC60175824323E74937387A42DBDD6829AE10BE1D71101C`, 14104576 bytes, build exit 0, contains `incremental`. After F9 and two Alt+Down chords it had 4 ListItem elements, and ListItem names contained both `python.pytest_cache` and `python.mypy_cache`. Exit after `WM_CLOSE` was 0. Log: `evidence/c11-listitem.txt`. The sample does not show a preview, so the review flow stays open. PID 111368 on the same binary had 4 ListItems after F9 and two Alt+Down chords. Exactly one name started with `> `, that name contained `python.mypy_cache`, and another ListItem contained `python.pytest_cache` without the prefix. Exit after `WM_CLOSE` was 0. Log: `evidence/c11-selected-item.txt`.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
