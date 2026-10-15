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

C12-R4. Optimize presents applicable tasks, impact, privilege, skip reason and last result; batch action opens review and preserves per-action outcomes.

## Acceptance criteria

- [ ] C12-AC1: All actions map to accepted CLI/core operations; unavailable actions explain capability and never act as success placeholders.
- [ ] C12-AC2: Provider UI pending, privilege decline, reboot, failed uninstall and changed selection are rendered correctly.
- [ ] C12-AC3: List virtualization and shared single-flight coordination prevent freezes; route changes cannot cancel a dispatched mutating operation implicitly.
- [ ] C12-AC4: Native EN/zh-CN screenshots and UIA/keyboard review flows cover normal/error/partial/confirmation states.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
