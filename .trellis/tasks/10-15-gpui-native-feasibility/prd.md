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

- [x] C09-AC1: A release binary launches from an unrelated directory and without the old desktop frontend/runtime; packaged icons and text work offline.
- [ ] C09-AC2: Native UIA exposes labels/roles/focus, IME composition works, and keyboard-only operation reaches all spike controls.
- [ ] C09-AC3: Pinned build is reproducible; the explicitly documented current dependency/toolchain policy passes core/CLI and native CI without silently claiming the previous Rust 1.88 baseline still works. Local `just ci` exited 0 and a locked release build exited 0 on rustc 1.99.0. `desktop-gpui` is not a workspace member, so that `just ci` did not build it. A second locked release into an outside target directory matched SHA-256 `7C1A82CD42F6E071624DFAAD8B1E70AD21C822FCBE833FC6764D54B73230669D` and 13431296 bytes. On 2026-10-17 an earlier tree was built twice into fresh outside target directories. Both `devsweep-gpui-spike.exe` files were SHA-256 `bf592d32b8b6e612d63a5d94c1bf4071ca36d0f45c57f496b64cb335553fc14a` and 13437440 bytes. That pair does not identify the later warning-free source. A warning-free locked release into `devsweep-gpui-release-warn` is SHA-256 `8d07070bcfa935ec93fa231fa0b5c5ed60f5461aaa3785749982c1beebed1f22` and 13348864 bytes. A second fresh target, `devsweep-gpui-release-c`, exited 0 with no `warning:` lines but produced SHA-256 `095f51d77da5da5729c2341d24c726d460ca1a44ede2204789c13227e521fe62` and 13437952 bytes. The desktop-gpui sources were not newer than the first of those two binaries. `hashes_equal=false`. The first log records mbx `1 incremental`; the second log does not. Both report 348 cache hits and 6 misses. A later pair with no `incremental` line in either log matches: `devsweep-gpui-release-d` and `devsweep-gpui-release-e` are SHA-256 `095f51d77da5da5729c2341d24c726d460ca1a44ede2204789c13227e521fe62` and 13437952 bytes. Both of those builds exited 0 and neither log contains `warning:`. Hosted native CI was not run: NOT_RUN. Rust 1.88.0 was not executed.
- [x] C09-AC4: Native screenshots, binary hash, PID/creation identity and shutdown/resource samples exist; test context alone does not satisfy this gate. 2026-10-17 launch of `devsweep-gpui-spike.exe` SHA-256 `7C1A82CD42F6E071624DFAAD8B1E70AD21C822FCBE833FC6764D54B73230669D`, 13431296 bytes, PID 61896, creation `2026-10-17T13:25:46.9965287-04:00`, working set 89903104, private bytes 125505536, `WM_CLOSE` exit 0, from an empty unrelated directory. Screenshot: `evidence/c09-ac4-launch.png`. The current non-incremental binary SHA-256 `095f51d77da5da5729c2341d24c726d460ca1a44ede2204789c13227e521fe62`, 13437952 bytes, PID 89764, creation `2026-10-17T17:58:27.4045295-04:00`, also launched from an empty unrelated directory, showed a DevSweep window, and exited 0 after `WM_CLOSE`. Screenshot: `evidence/c09-current-launch.png`. Loaded modules did not include WebView, Node, or Tauri. UIA focus and IME composition stay on C09-AC2. Rust 1.88.0 was not run.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
