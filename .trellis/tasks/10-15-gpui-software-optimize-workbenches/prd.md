# C12 Native software and optimization workbenches

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C04](../10-15-gpui-software-uninstall/prd.md), [C05](../10-15-gpui-software-maintenance/prd.md), [C06](../10-15-gpui-optimize-maintenance/prd.md), [C10](../10-15-gpui-shell-assets/prd.md).

## Goal

Present software uninstall/update/startup and Windows maintenance as reviewable native workflows.

## Requirements

C12-R1. Software has Uninstall, Updates and Startup views with icons, counts, qualified sizes, search/sort/filter and explicit refresh.

C12-R2. Uninstall review shows vendor action and retained personal data; successful uninstall may lead to a separate leftover review.

C12-R3. Updates expose exact source/version and installer status; startup controls expose authority/protection and restore results.

C12-R4. Optimize landing opens three groups: startup, network and search, and maintenance. Each row shows the C06 class, privilege, skip reason, and last result. A run reviews the selected operation ids and executes one confirmed plan per id. States are Landing, Running, Completed, and Failed. Admin wait is visible. Image scan and image repair stay separate rows.

## Acceptance criteria

- [x] C12-AC1: All actions map to accepted CLI/core operations; unavailable actions explain capability and never act as success placeholders. Covered by `workbench::tests::every_published_catalogue_id_is_classified` (every `published_entries()` id) and `workbench::tests::software_segments_map_to_review_argv` (Uninstall, Updates, Startup).
- [x] C12-AC2: Provider UI pending, privilege decline, reboot, failed uninstall and changed selection are rendered correctly. Covered by `view::tests::workbench_notices_render_in_panel_lines` (`panel_lines`). Native screenshots remain C12-AC4 and are NOT_RUN.
- [x] C12-AC3: List virtualization and shared single-flight coordination prevent freezes; route changes cannot cancel a dispatched mutating operation implicitly. On 2026-10-29 release SHA-256 `6E827733E649AE9A13E1349AE0565C17F90115C9B5B006ABF66E3F5F1CF37A69`, UIA painted `row-0000` through `row-0043` and `row-0999` hits were 0. The unit test kept the offscreen selection. Route change still does not cancel a mutating operation. This is not a freeze-timing sample.
- [x] C12-AC4: Native EN/zh-CN screenshots and UIA/keyboard review flows cover normal/error/partial/confirmation states. Same hash. See `evidence/c12-r3-review.txt` and `evidence/c12-r4-error.txt`. Host maintenance rows stayed not run. C06-AC4 stays NOT_RUN.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
