# C10 GPUI shell assets preferences and lifecycle

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C09](../10-15-gpui-native-feasibility/prd.md).

## Goal

Create the native five-mode shell with reusable assets, settings and bounded operation ownership.

## Requirements

C10-R1. Implement five clear mode entries, persistent navigation, local history/protection/rules/settings surfaces, EN/zh-CN copy and keyboard focus.

C10-R2. Reuse original DevSweep beaver master/ICO/PNG, licensed Earth still/texture and existing semantic icons/colors where suitable; document any asset conversion.

C10-R3. Preserve preference formats and separate scan roots from protections. Read existing stores without writes and migrate only on explicit save.

C10-R4. Run blocking domain work off the render thread with bounded mailboxes, operation IDs, stale-result fences and cancel/join or wait-only close policies.

## Acceptance criteria

- [ ] C10-AC1: Route/settings/back preserves reports, selection and appropriate invalidation; no old completion revives canceled/replaced state.
- [ ] C10-AC2: Old Tauri and new GPUI can read compatible preferences without corruption; concurrent writers and downgrade cases are tested.
- [ ] C10-AC3: Native titlebar/tray/icon, screen reader, text scaling, high contrast, reduced motion and Chinese text pass actual UI checks.
- [ ] C10-AC4: Decorative visuals are optional and idle/minimized/hidden states have no recurring decorative work; icons are offline and correctly attributed.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
