# C14 Native acceptance performance and migration

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md), [C03](../10-15-gpui-clean-coverage/prd.md), [C04](../10-15-gpui-software-uninstall/prd.md), [C05](../10-15-gpui-software-maintenance/prd.md), [C06](../10-15-gpui-optimize-maintenance/prd.md), [C07](../10-15-gpui-analyze-capabilities/prd.md), [C08](../10-15-gpui-status-capabilities/prd.md), [C09](../10-15-gpui-native-feasibility/prd.md), [C10](../10-15-gpui-shell-assets/prd.md), [C11](../10-15-gpui-clean-workbench/prd.md), [C12](../10-15-gpui-software-optimize-workbenches/prd.md), [C13](../10-15-gpui-analyze-status-tray/prd.md), [C15](../10-15-gpui-ai-tool-cleanup/prd.md), [C16](../10-15-gpui-latest-dependencies/prd.md).

## Goal

Verify feature parity and measured improvement, package GPUI and make the Tauri retirement decision reviewable.

## Requirements

C14-R1. Maintain a requirement-to-evidence ledger for all five modes, CLI reachability, safe actions, resources, accessibility and installed-package behavior.

C14-R2. Freeze fresh current Tauri/core and candidate identities with equal fixtures/scopes; retain historic failures without reclassifying them.

C14-R3. Run native release startup/interaction/scan/idle/active/minimize/close measurements and source safety checks using the parent protocol.

C14-R4. Prepare isolated Windows packaging and compatible settings migration; switch defaults only after acceptance, with an explicit rollback path and later retirement checklist.

## Acceptance criteria

- [ ] C14-AC1: Every parent criterion is PASS or explicitly FAIL/NOT_RUN/UNVERIFIED with evidence; no aggregate complete claim while a mandatory gate remains unmet.
- [ ] C14-AC2: Canonical Rust checks, CLI contract fixtures and separate GPUI build/test/native gates pass; unchanged legacy baseline remains reproducible.
- [ ] C14-AC3: Packaged GPUI launches offline/from unrelated CWD with correct assets, native actions and normal owned-process shutdown without WebView/Node runtime.
- [ ] C14-AC4: Tauri removal/publishing/install/signing/commit are separate approval actions; planning or build completion does not perform them.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
