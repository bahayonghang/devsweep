# C09 GPUI Windows feasibility and dependency pin

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C16](../10-15-gpui-latest-dependencies/prd.md), [C01](../10-15-gpui-cli-contracts/prd.md).

## Goal

Prove the selected GPUI revision on native Windows before building the replacement UI.

## Requirements

C09-R1. Select the latest compatible stable GPUI/platform packages, or latest validated immutable upstream revision where required, and record licenses, prerequisites, binary dependencies and toolchain constraints.

C09-R2. Build a minimal Rust native window with real text input, Chinese IME, focus, keyboard actions, scalable fonts, list virtualization and a local icon.

C09-R3. Verify Windows accessibility/UI Automation, high contrast/reduced motion, DPI/multiple displays and supported graphics paths; unresolved blockers prevent UI rollout.

C09-R4. Measure empty-shell startup, idle resources and normal close; no WebView/Node frontend runtime or Tauri application dependency.

## Acceptance criteria

- [ ] C09-AC1: A release binary launches from an unrelated directory and without the old desktop frontend/runtime; packaged icons and text work offline.
- [ ] C09-AC2: Native UIA exposes labels/roles/focus, IME composition works, and keyboard-only operation reaches all spike controls.
- [ ] C09-AC3: Pinned build is reproducible; the explicitly documented current dependency/toolchain policy passes core/CLI and native CI without silently claiming the previous Rust 1.88 baseline still works.
- [ ] C09-AC4: Native screenshots, binary hash, PID/creation identity and shutdown/resource samples exist; test context alone does not satisfy this gate.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
