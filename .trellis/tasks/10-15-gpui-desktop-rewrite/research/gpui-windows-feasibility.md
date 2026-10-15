# Research: GPUI Windows feasibility

> Policy update after this research: the user explicitly requested latest library
> versions where possible. `dependency-policy.md` supersedes the original recommendation
> to prioritize the old Rust 1.88 baseline; the recommendations below are updated.
> Preserve the source observations,
> but let C16/C09 select and test current dependencies/toolchains and document any
> deliberate compatibility migration. No final dependency was selected here.

- Query: Assess a native GPUI desktop replacement while retaining a CLI-first Rust core, existing safety contracts, and reusable DevSweep assets.
- Scope: mixed; primary upstream sources and targeted local source/spec inspection only.
- Date: 2026-10-15. All external sources below were accessed on this date.
- Status: research complete; dependency selection, builds, native behavior, and performance remain **NOT_RUN / UNVERIFIED**.

## Findings

### Recommendation and architecture boundary

Proceed with a small Windows native feasibility child task before committing the complete desktop implementation to a GPUI dependency revision. Current upstream has concrete Win32, DirectWrite, Direct3D, IME, and accessibility implementations. This supports technical feasibility, but does not prove that a selected dependency set works in this workspace or improves DevSweep performance.

The new application should contain Rust application/domain/UI code and have **no runtime WebView, JavaScript, or Node requirement**. Native Windows libraries, MSVC/SDK build tools, GPU drivers, and audited Rust FFI dependencies are compatible with this architecture. “Pure Rust application” must not be interpreted as eliminating operating-system dependencies or guaranteeing an entirely Rust transitive supply chain.

Keep core/CLI functionality independently useful. GPUI should call frontend-neutral core services and own presentation/lifecycle only. Reuse image/icon assets and their notices; do not port React/WebGL plumbing, Tauri IPC, or Mole artwork as dependencies of the new shell.

### Published package versus moving upstream

| Evidence | Verified observation | Planning implication |
| --- | --- | --- |
| Published `gpui` | docs.rs identifies version **0.2.2**, published **2025-10-22**. Its README uses `Application::new()` and still describes macOS/Linux. Its normalized manifest contains Windows dependencies and edition 2024, but no `rust-version` or `accesskit` entry. [S1–S2] | Do not infer “no Windows code” from the old README. Equally, do not infer modern accessibility support from current main. Evaluate the actual published source if selecting this candidate. |
| Current `main` | `gpui/Cargo.toml` also says 0.2.2, while its README now uses `gpui_platform::application()`. `gpui_platform` selects a separate `gpui_windows` backend on Windows. Both platform package manifests currently say 0.1.0. [S3–S5] | An unchanged package version string does not establish API or source equivalence. Do not mix published GPUI with arbitrary main-only platform packages or examples. Registry availability of the new platform packages was not established in this research. |
| Maintenance policy | Upstream calls GPUI pre-1.0, warns about breaking changes, and requests latest stable Rust. [S3] | Never use wildcard, floating branch, or “latest” in the production dependency contract. |
| Published provenance | The 0.2.2 archive records Git SHA `69e2130295c2649963eb639fc70b4f2ee8ea1624` **and `dirty: true`**. [S6] | This metadata is not proof that checking out that SHA reproduces the published archive. Prefer the actual registry archive/checksum for a registry candidate. |

**Proposed decision rule:** prefer an exact published version only if it passes the same required Windows input/accessibility/native gate. Otherwise evaluate a full immutable Git revision containing the current platform/accessibility work; pin GPUI and platform crates to the same repository revision, retain Cargo.lock, and record compiler, resolved packages, feature flags, and license notices. No exact Git revision was selected or validated here. Main URLs below describe accessed source, not a release guarantee.

### Rust and workspace compatibility

- Local `Cargo.toml:2–10` defines one workspace, resolver 3, edition 2024, and Rust **1.88.0**; core and CLI inherit that Rust version. Current upstream `rust-toolchain.toml` selects **1.98.1**. This is an upstream toolchain observation, **not a measured minimum GPUI compiler version**. [S7]
- Matching edition 2024 does not prove Rust 1.88 compatibility. Neither absent `rust-version` metadata nor successful dependency resolution proves it. No compilation was performed.
- Updated recommendation after the user's latest-library requirement: C16 chooses and documents a current compatible toolchain, including an explicit core/CLI MSRV migration if required; C09 tests the desktop compiler requirement. Cargo supports differing Rust policies in a workspace, but shared dependency resolution and manifest loading can couple them. Validate the combined lockfile; isolate the desktop workspace only if a concrete incompatibility requires it. [S8]
- `.github/workflows/ci.yml:123–154` currently tests MSRV using `--workspace --exclude devsweep-desktop`. A new GPUI member would enter that selection accidentally unless the matrix is deliberately updated. Prefer explicit core/CLI package selections for MSRV, and separate native desktop jobs. `justfile:15–21` likewise includes all workspace members; preserve the canonical gate deliberately rather than silently excluding failing work.
- Core uses `windows = 0.56.0`, while existing Tauri code already uses a different Windows bindings version (`desktop/src-tauri/Cargo.toml`). Keep Windows/GPUI-specific handles and bindings out of core-facing public service types; use owned Rust values at that boundary.

### Native support and required verification

| Area | Upstream evidence | Remaining product evidence |
| --- | --- | --- |
| Rendering and text | Main README specifies Win32 and DirectWrite. `gpui_windows` declares Direct3D11/DirectComposition/DirectWrite dependencies; its device implementation creates D3D11 devices and checks supported GPU features. [S3, S5, S9] | Actual GPU/driver identity; first visible and interactive frame; device failure behavior; DPI transitions and text legibility. Do not promise universal GPU/software fallback from these sources. |
| IME | `gpui_windows/src/events.rs` implements IMM composition/result strings and IME enablement. `window.rs` updates the candidate/caret position using the current scale factor. [S10–S11] | Microsoft Pinyin composition, commit/cancel, candidate positioning, selection, paste, focus changes, Chinese paths, and scaling in the application's actual input widgets. Backend handlers alone are insufficient. |
| Accessibility | Main integrates AccessKit. Windows `a11y_init` constructs `accesskit_windows::Adapter`; tree updates raise UIA events. The GPUI guide requires explicit roles and stable IDs, and describes accessible action handling. [S11–S12] | Native UIA tree and Narrator operation for tabs, list rows, search, checkboxes, dialogs, progress and confirmation. Virtualized rows must keep stable identity and meaningful names/states. Painted text is not automatically a complete accessible interface. |
| Native dialogs | Windows platform has `prompt_for_paths` and `prompt_for_new_path`, with dialog ownership and asynchronous result receivers. [S13] | Directory-only picker behavior, cancellation, owner/modal focus, Unicode/long paths, close during dialog, and validation of returned selections in core. Prefer these existing APIs before adding another dialog library. |
| Tray and HUD | Standalone `tray-icon` supports Windows; its documented Windows requirement is an event loop on the creating thread. It provides event handlers/receivers. [S14] | Integration with GPUI's event loop remains a spike, not a verified drop-in. Own creation/destruction on a compatible thread, wake UI on events without busy polling, and verify reopen/menu/exit behavior. A Rust crate under the Tauri organization does not imply a WebView runtime dependency. |
| Native test support | GPUI documents simulated input through `TestAppContext` and its test macro. [S3] | Tests can prove state, focus transitions and event handling; they cannot alone prove Windows IME, screen-reader behavior, real pixels, idle GPU cost, or installed-app behavior. |

Official Zed Windows documentation currently requires a DirectX 11-compatible GPU [S15], while its development guide retains a Vulkan troubleshooting passage [S16]. Treat this as documentation drift. Use the selected revision's actual backend and native receipts when documenting DevSweep requirements; do not copy Zed's full application dependency list as a minimal GPUI dependency list.

### Packaging and asset reuse

Start the gate with an ordinary release executable and locally packaged assets. Production packaging must separately own executable metadata/icon, Windows manifest and DPI behavior, dependency/DLL inventory, notices, deterministic artifact identity, and upgrade/uninstall behavior. Existing `desktop/src-tauri/tauri.conf.json:29–35` selects NSIS, but Tauri's bundler configuration is not automatically a GPUI packaging implementation. Installer technology can be chosen after the executable gate; installer/signing/publication are separate delivery work.

Use the MSVC Windows target as the initial tested candidate. Upstream's Windows development guide documents MSVC Build Tools and Windows SDK prerequisites; some of its CMake/other requirements belong to the complete Zed application. Derive the actual requirements from the locked GPUI dependency build. Native OS libraries are permitted; launching the release artifact must not require Node, npm, a local web server, or WebView2. [S16]

GPUI declares Apache-2.0 [S2, S4–S5]. Retain framework and transitive notices independently of DevSweep's MIT license. Existing assets include provenance-bearing files under `desktop/public/earth/`; `desktop/public/THIRD_PARTY_NOTICES.txt:3–21` records Natural Earth source and hashes. Reuse only assets with established repository provenance. Prefer the still image until animation has a measured purpose and budget.

### Minimal native feasibility gate (proposed; all NOT_RUN)

1. **Dependency/toolchain:** choose one exact source candidate; record lockfile, target, features, compiler and SDK. Build the release GPUI executable and check core/CLI under its explicitly declared and reviewed MSRV with the resulting dependency strategy. Prove no unwanted GPUI dependency enters core/CLI. Document and test any compiler migration; no silent MSRV increase.
2. **Representative native slice:** implement five-mode navigation, one 10k-row virtualized fixture list with selection/search, one Chinese text input, an owned directory picker, keyboard focus, and a tray open/exit path. Connect one bounded read-only core request, including cancellation/join and stale-generation rejection. It must launch offline with existing local assets.
3. **Real Windows operation:** capture PID, creation time, executable hash, GPU/driver and actual native UI evidence. Exercise Chinese IME, keyboard and UIA/Narrator; test 100/125/150/200% scaling and mixed-DPI transitions where available, resize, minimize/restore, and close/reopen. Record unavailable hardware/scaling cases as UNVERIFIED rather than substituting screenshots from emulation.
4. **Lifecycle and performance:** record release startup-to-interactive, process-tree CPU, private bytes/working set, GPU activity, idle/minimized behavior, list/search/selection latency, and repeated open/close/cancel resource return. Freeze workloads, warm-up, samples, percentile calculation and budgets before running; compare the same core workload and fixture identities with the retained Tauri reference. A smaller shell cannot compensate for a slow scanner.
5. **Gate result:** proceed with the selected revision only when the required native slice is functional and measured. Unit tests and build success remain separate receipts; retain failures and missing evidence. Any required IME/accessibility or lifecycle failure returns to this spike before broad UI implementation. Do not execute real cleanup, uninstall, or optimization as part of this gate.

## Files Found and Code Patterns

| Local file / anchor | Relevance |
| --- | --- |
| `Cargo.toml:2–10` | Workspace membership, edition, resolver and shared Rust 1.88 requirement. |
| `crates/devsweep-core/src/services.rs:32–74` | Frontend-neutral scan/clean/inventory traits; cancellation and expected-digest parameters already belong below the UI. |
| `desktop/src-tauri/src/async_runtime.rs:1–4` | Existing bounded dispatch addresses idle overhead; migration still needs worker ownership and blocking isolation. |
| `desktop/src-tauri/src/tray.rs:55–92` | HUD show-generation invalidation; reusable lifecycle reasoning, not reusable Tauri window types. |
| `desktop/src-tauri/src/tray.rs:204–231` | Current tray/menu and close semantics to inventory before replacement. |
| `desktop/src-tauri/src/fonts.rs:1` | Existing DirectWrite font discovery boundary. |
| `.github/workflows/ci.yml:123–154` | MSRV matrix currently excludes only the existing desktop package. |
| `desktop/public/THIRD_PARTY_NOTICES.txt:3–21` | Existing reusable Earth asset provenance. |

## Related Specs

- `.trellis/spec/backend/index.md` and `quality-guidelines.md`: shared domain safety, canonical `just ci`, release evidence and generated-artifact boundaries.
- `.trellis/spec/backend/bounded-delivery.md:24–31,94–102`: bounded/lossless Status delivery and separate native lifecycle evidence. GPUI must preserve semantics without blocking its UI thread on producer joins.
- `.trellis/spec/backend/desktop-runtime.md:28–38,63–80`: native visibility controls decoration only; domain jobs retain their lifecycle, and passing unit tests does not prove native performance.
- `.trellis/spec/desktop-frontend/index.md`: current five-mode, localization, confirmation and accessibility requirements. React/Tauri-specific mechanics are not a GPUI implementation contract; the parent task must author the new native contract explicitly.
- `.trellis/workflow.md`: persist planning/research before implementation and independently verify each child deliverable.

## External References

Primary sources only; accessed **2026-10-15**. `main`/`dev` links are moving source observations.

- S1: [Published GPUI 0.2.2 metadata and README](https://docs.rs/crate/gpui/0.2.2).
- S2: [Published normalized manifest](https://docs.rs/crate/gpui/0.2.2/source/Cargo.toml).
- S3: [Current GPUI README](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/README.md).
- S4: [Current GPUI manifest](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/Cargo.toml) and [gpui_platform manifest](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_platform/Cargo.toml).
- S5: [Current Windows backend manifest](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_windows/Cargo.toml).
- S6: [Published VCS metadata](https://docs.rs/crate/gpui/0.2.2/source/.cargo_vcs_info.json).
- S7: [Upstream toolchain](https://raw.githubusercontent.com/zed-industries/zed/main/rust-toolchain.toml).
- S8: [Cargo Rust-version and multiple-workspace-policy documentation](https://doc.rust-lang.org/cargo/reference/rust-version.html).
- S9: [Windows Direct3D device implementation](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_windows/src/directx_devices.rs).
- S10: [Windows event and IME implementation](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_windows/src/events.rs).
- S11: [Windows window, IME position and AccessKit adapter](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_windows/src/window.rs).
- S12: [GPUI accessibility guide](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/src/_accessibility.rs).
- S13: [Windows platform dialogs](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_windows/src/platform.rs).
- S14: [Standalone tray-icon upstream README](https://raw.githubusercontent.com/tauri-apps/tray-icon/dev/README.md).
- S15: [Zed Windows runtime guidance](https://zed.dev/docs/windows).
- S16: [Zed Windows build guidance](https://zed.dev/docs/development/windows).

## Caveats / Not Found

- No product edits, dependency downloads/builds, installations, benchmark runs, or real mutation tests were performed. The proposed gate is not acceptance evidence.
- Exact GPUI Git revision, tested MSRV, platform-crate registry availability, standalone tray integration, application accessibility completeness, and installer strategy remain unresolved technical deliverables.
- Source and docs were read through the browsing tool. Shell HTTPS access failed DNS resolution; no repository clone or current-main commit identity was obtained. Pin/recheck immutable sources during the dependency gate.
- Previous DevSweep performance notes were used only to identify evidence pitfalls; current spec/source was checked. No historical performance receipt is reused as current GPUI or Tauri acceptance.
- This research does not decide product platform scope or claim full Windows feature parity with Mole.
