# C13 Native analysis status and tray surfaces

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C07](../10-15-gpui-analyze-capabilities/prd.md), [C08](../10-15-gpui-status-capabilities/prd.md), [C10](../10-15-gpui-shell-assets/prd.md).

## Goal

Build scalable disk visualization, live status/process views and a lifecycle-aware Windows tray.

## Requirements

C13-R1. Analyze offers drive/folder selection, treemap plus accessible list, breadcrumbs, large-item explanations and reviewed reveal/recycle actions.

C13-R2. Status shows supported health/CPU/GPU/memory/battery/space/network/thermal groups and a searchable sortable process list with local explanations.

C13-R3. Tray provides an optional compact status panel from shared sampling; no second independent sampler while the main panel is visible.

C13-R4. Present Keep Screen On controls over C08's accepted CLI/service lease contract. Add a presentation-only Clean Screen session after native lifecycle design: bounded ownership, visible active state, escape shortcut and guaranteed restoration on exit.

## Acceptance criteria

- [ ] C13-AC1: Treemap/list share a stable node identity and accessible selection; unreadable content is distinct from empty and recycle uses the same CLI authority.
- [ ] C13-AC2: Unavailable sensors remain honest; main/tray subscriptions start/stop predictably and hidden panels add no decorative work.
- [ ] C13-AC3: Owned Keep Screen On state releases on normal exit/crash as supported by OS; Clean Screen always has a tested emergency escape and never locks input permanently.
- [ ] C13-AC4: Large snapshot rendering, 20 open/close/minimize cycles, native UIA and CLI/service parity pass; no process kill based on displayed PID alone.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
