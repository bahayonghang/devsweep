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

  Round-3 measurements against `research/performance-protocol.md` on the single-window spike, release SHA-256 `D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061`, 14179328 bytes, on a quiet host (12 percent load, no concurrent builds). Evidence: `evidence/c14-perf-round3.md` and `evidence/c14-idle-cpu-round3.md`.

  | Protocol metric | Budget | Measured | Result |
  | --- | --- | --- | --- |
  | Process-cold first usable window p95 | <= 1500 ms | 616 ms (n=20, raw retained) | **PASS** on the absolute limit |
  | Idle private bytes | <= 200 MiB | 113 MiB settled | **PASS** on the absolute limit |
  | Idle CPU median | <= 0.5 % of one core | 0 % (150 samples @ 200 ms) | **PASS** |
  | Idle CPU p95 | <= 1.0 % of one core | 7.4405 % | **FAIL** |
  | Lifecycle: clean exits over 20 open/close cycles | no orphan / no growth | 20/20 clean, 0 orphans | **PASS** |

  The idle-CPU p95 failure is real and is not relaxed, retried away, or
  reclassified. The raw 150-sample array is retained in the evidence file and
  shows intermittent spikes of one or two 15 ms scheduling quanta per 200 ms
  sample against an otherwise idle median of zero. It is a defect to fix, not a
  threshold to lower.

  **Round 4: the relative clauses now have a real fresh-Tauri baseline.**
  A baseline was built this round with
  `cargo build --locked --release -p devsweep-desktop --target-dir dist/tauri-baseline-target`
  (exit 0), SHA-256 `5A64470D5C334F19AF07DB7F7C58CE56094231ABAF4C5611493C8B180D71FBC4`,
  13306880 bytes. Measured across the **whole owned process tree** as the
  protocol requires. A first same-round attempt summed only the root process and
  is retained, annotated as superseded, because it wrongly reported Tauri at
  5 MiB / 116 ms.

  | Budget | Threshold | Tauri baseline | GPUI spike | Result |
  | --- | --- | --- | --- | --- |
  | Startup to settled usable tree, relative | <= 80 % | 1140 ms | 814 ms (71.4 %) | **PASS** |
  | Idle private memory, relative | <= 70 % | 171 MiB (7 processes) | 112 MiB (1 process) | **PASS** |
  | Startup absolute | <= 1500 ms | — | 814 ms | PASS |
  | Idle private absolute | <= 200 MiB | — | 112 MiB | PASS |

  Both sides left **0 processes** alive 2 s after close. Evidence:
  `evidence/c14-tree-comparison-round4.md`.

  **The equivalence caveat is not waived.** The GPUI side is the single-window
  spike (12 UIA descendants, 1 process); the Tauri side is the full five-mode
  application (23 descendants, 7 processes including the WebView2 tree). The
  protocol requires equal scopes, and these are not feature-equivalent, so these
  ratios are real whole-tree measurements but they are **not** the
  equivalence-gated acceptance C14 ultimately needs. The idle-CPU p95 FAIL
  (7.4405 % against a 1.0 % budget) still stands.

  Still **NOT_RUN**: warm-window, navigation/preview response, 10k-row frame
  interval, hidden-decoration frames, software-inventory p95, core-scan
  non-regression, cooperative cancel/close, and any five-mode GPUI measurement.

  A first run in this round reported idle CPU as 3.542 % from a single
  undivided 30 s window. That figure was not the protocol's median/p95 over
  200 ms samples and is superseded by the table above; both are retained rather
  than the earlier one being deleted.
- [ ] C14-AC2: Canonical Rust checks, CLI contract fixtures and separate GPUI build/test/native gates pass; unchanged legacy baseline remains reproducible. On 2026-10-18 `just ci` exited 0 (918 passed, 0 failed, 5 ignored, `ci complete`) and the separate `desktop-gpui` gate exited 0 for fmt, check, clippy `-D warnings`, 59 tests and a locked release build. The unchanged legacy baseline is now reproducible: `cargo build --locked --release -p devsweep-desktop --target-dir dist/tauri-baseline-target` exited 0 this round, so the legacy Tauri app still builds from the current tree.
- [ ] C14-AC3: Packaged GPUI launches offline/from unrelated CWD with correct assets, native actions and normal owned-process shutdown without WebView/Node runtime. The current release binary SHA-256 `D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061`, 14179328 bytes, launched from an empty temp directory, showed the DevSweep window, exited 0 after `WM_CLOSE`, and did not load WebView, Node, or Tauri. A packaged installer and a native action beyond launch remain NOT_RUN.
- [x] C14-AC4: Tauri removal/publishing/install/signing/commit are separate approval actions; planning or build completion does not perform them. `desktop/` remains. `just tdev` is still `cd desktop; npm run tauri -- dev`. This session did not remove Tauri, publish, install, or sign a package, and did not commit a retirement.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
