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

  Clause-by-clause state on 2026-10-19:

  | Clause | State | Evidence |
  | --- | --- | --- |
  | Native UIA exposes labels and roles | **PASS** | 13-node tree: Window/Pane/Button(`DevSweep 菜单`)/Tab(`模式`)/5x TabItem(`清理`,`软件`,`优化`,`分析`,`状态`)/Edit(`Note`)/2x ListItem carrying their own text. |
  | Keyboard-only operation reaches all spike controls | **PASS** | Window activated with real OS keystrokes; `alt-right` moved `清理`→`软件`→`优化` and `alt-left` back, with the Note holding focus and no TAB pressed first. Plain arrows still move the caret (`xy`+`LEFT`+`Z` → `xZy`). `down`/`F10`/`Escape` unchanged. |
  | UIA exposes focus | **FAIL** | Proven an app defect, not the environment: in the same session Notepad reports `FocusedElement` = `Text editor` with `hasFocus=True` and 5 of 37 focused nodes, while the GPUI app reports the root window with `hasFocus=False` and **0 of 13**. Ruled out stale tree (repaints, a real click on the Note, and real typing all leave it unchanged), missing role/id (the Note node is present as `ControlType.Edit`), and accessibility being off (13 nodes published). Code path located in GPUI `div.rs:2326-2336` → `a11y.rs:233-250` → `window.rs:3786-3810`: the tree update only sets accesskit focus when `focus_handle.is_focused(window)` is true at prepaint, so it stays at `ROOT_NODE_ID`. Two repair attempts did not land. Evidence: `evidence/c09-uia-focus-defect-round7.md`. |
  | IME composition works | **NOT_RUN** | Environment limitation, not an app defect: Notepad with a native edit control and a real focused child HWND also returns a NULL `ImmGetContext` in this session, while both installed layouts report `ImmIsIME=True`. See `evidence/c09-ime-environment-round5.md`. |

  The keyboard clause was previously a **FAIL**. Root cause was proven experimentally: `left`/`right` were bound both globally to `ModePrev`/`ModeNext` and to `input::Left`/`input::Right` under the `SpikeInput` predicate, and the focused Note's more specific context won. Fixed by adding `SpikeInput`-scoped `alt-left`/`alt-right` bindings that bubble to the existing shell handlers, with a bilingual visible hint. Evidence: `evidence/c09-keyboard-rootcause-round5.md` and `evidence/c09-alt-chord-round6.md`.
- [ ] C09-AC3: Pinned build is reproducible; the explicitly documented current dependency/toolchain policy passes core/CLI and native CI without silently claiming the previous Rust 1.88 baseline still works.

  **2026-10-18 reproducibility re-test: the earlier matching pairs were not reproducible.** Four clean builds of the same source with the same lock and rustc 1.99.0 produced four different SHA-256 values (`9C192A32…1AC7`, `FB060EFA…8D9`, `34CB6934…D8B6`, `147F68BE…3444`), all exactly 14179328 bytes, none containing an `incremental` line and none containing `warning:`. Diffing two of them located the variance precisely: 24 bytes, at offsets 248-249 inside the MSVC Rich header immediately after the `Rich` marker, and around offset ~12.3 MB in the PE timestamp/build-id region. That is MSVC linker non-determinism, not a source or dependency difference. Evidence: `evidence/c09-reproducibility-round3.md`.

  **Fixed and now reproducible.** `RUSTFLAGS='-C link-arg=/Brepro'` makes the link deterministic. Two clean builds through the new `just gpui-build` recipe, into separate fresh target directories, both produce SHA-256 `DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34` at 14179328 bytes, with 0 `incremental` and 0 `warning:` lines. A second independent pair reproduced the same hash, so it is stable across four builds. The recipe is committed in `justfile` as `gpui-check`, `gpui-test`, `gpui-build` and the aggregate `gpui`; `just gpui` exits 0. This also closes the gap that `desktop-gpui`, being a standalone workspace, had no first-class gate at all.

  Still NOT_RUN for this criterion: hosted native CI (not run) and Rust 1.88.0 on this native target. The declared 1.88.0 MSRV is now actually executed on Linux under C16-AC3.
- [x] C09-AC4: Native screenshots, binary hash, PID/creation identity and shutdown/resource samples exist; test context alone does not satisfy this gate. 2026-10-17 launch of `devsweep-gpui-spike.exe` SHA-256 `7C1A82CD42F6E071624DFAAD8B1E70AD21C822FCBE833FC6764D54B73230669D`, 13431296 bytes, PID 61896, creation `2026-10-17T13:25:46.9965287-04:00`, working set 89903104, private bytes 125505536, `WM_CLOSE` exit 0, from an empty unrelated directory. Screenshot: `evidence/c09-ac4-launch.png`. A later launch of SHA-256 `D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061`, 14179328 bytes, PID 63932, creation `2026-10-18T05:33:05.3206660-04:00`, from an empty unrelated directory, showed a 13-node DevSweep accessibility tree and exited 0 after `WM_CLOSE`. Loaded modules did not include WebView, Node, or Tauri. UIA focus and IME composition stay on C09-AC2. Rust 1.88.0 was not run natively.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.


