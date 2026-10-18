# C09 GPUI pin

Refresh date: 2026-10-17. One authenticated GitHub GraphQL request. Default branch name and commit SHA came from that response.

| Item | Value |
| --- | --- |
| Repository | `zed-industries/zed` |
| Default branch | `main` |
| Selected revision | `322cd509dcccaee1f6bc44725d270598ae8171ad` |
| C16 candidate | `3e145061d861ed32bb6a08dad7b03f2b5e455ad8` |
| Refresh | Returned a full 40-hex SHA. The selected revision is that SHA. |
| Manifest | `rev = "322cd509dcccaee1f6bc44725d270598ae8171ad"` on `gpui`, `gpui_platform`, and `gpui_windows` |
| Rejected | Floating `main`. A `*` version. Mixing crates.io `gpui` 0.2.2 with upstream `gpui_platform` or `gpui_windows`. |

`gpui`, `gpui_platform`, and `gpui_windows` declare Apache-2.0 at this revision. DevSweep's own spike package stays MIT. The published registry crate `gpui` 0.2.2 remains the registry alternative and is not the pin.

Upstream `rust-toolchain.toml` at this revision selects Rust 1.98.1. That file is not copied into `desktop-gpui/`. This spike builds with the already installed default host toolchain. Rust 1.88.0 was not installed and was not downloaded. A pass on a newer compiler is not a pass of the declared workspace MSRV.

Windows prerequisites recorded from the pinned manifests and the linked release binary: MSVC host, Windows SDK, Win32, Direct3D 11, DirectComposition, DXGI, DirectWrite, UI Automation, and the system IME. No Tauri, WebView, or Node dependency is declared. The release import-name scan found `d3d11.dll`, `dcomp.dll`, `dxgi.dll`, `dwrite.dll`, `uiautomationcore.dll`, and `imm32.dll`. It found no WebView, Node, or Tauri DLL name. Runtime module lists from the two launches agree. Rust 1.88.0 was not run.

The spike is one window. It is not the five-mode shell.
