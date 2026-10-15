# Research: Fresh GPUI dependency checkpoint

- Query: Verify current GPUI registry releases and immutable upstream source for C16; identify the candidate C09 should evaluate without changing dependencies or running builds.
- Scope: mixed; fresh official HTTP metadata/source reads and the approved C16 dependency contract.
- Date: 2026-10-15; requests completed approximately 06:07–06:10 UTC (02:07–02:10 America/New_York).
- Result: **metadata/source verification complete; dependency compatibility and native acceptance NOT_RUN**.

## Findings

### 1. Exact current identities

| Item | Fresh verified result | Primary evidence |
| --- | --- | --- |
| Latest registry `gpui` | `0.2.2`; `max_version`, `max_stable_version` and `newest_version` all agree; not yanked; published `2025-10-22T03:56:29.8357Z`; `rust_version: null`; Apache-2.0 | [crates.io API](https://crates.io/api/v1/crates/gpui), HTTP 200, response Date `2026-10-15 06:07:37 GMT` |
| Registry package checksum | `979b45cfa6ec723b6f42330915a1b3769b930d02b2d505f9697f8ca602bee707` | The API's 0.2.2 `checksum` field; package archive itself was not downloaded or hashed here |
| Registry `gpui_platform` | HTTP **404 Not Found**; no publicly available crate record at the queried name | [crates.io API](https://crates.io/api/v1/crates/gpui_platform), requested in the same checkpoint |
| Current Zed main | **`3e145061d861ed32bb6a08dad7b03f2b5e455ad8`**; commit timestamp `2026-10-14T19:54:27Z` | [GitHub commit API](https://api.github.com/repos/zed-industries/zed/commits/main), HTTP 200, response Date `2026-10-15 06:07:39 GMT`; [immutable commit](https://github.com/zed-industries/zed/commit/3e145061d861ed32bb6a08dad7b03f2b5e455ad8) |
| Upstream compiler selection | `rust-toolchain.toml` selects **1.98.1** | [Immutable toolchain][toolchain], line 2 |
| Declared upstream MSRV | No `rust-version` key found in the inspected root, `gpui`, `gpui_platform` or `gpui_windows` manifests; registry GPUI reports null | [Root manifest][root], [GPUI manifest][gpui], [platform manifest][platform], [Windows manifest][windows] |

**1.98.1 is the selected upstream development toolchain, not a measured minimum supported Rust version.** The absence of MSRV metadata is not a promise of Rust 1.88 compatibility. C16 may revise the declared shared policy under the user's approved latest-compatible requirement; C09 must test the eventual native dependency/compiler combination.

### 2. Published 0.2.2 is not the current same-numbered source

| Surface | Exact published crate | Immutable current upstream |
| --- | --- | --- |
| Version | `gpui 0.2.2`, edition 2024 | `gpui` also says `0.2.2`; inherited edition 2024 |
| Startup API documented | `Application::new()` in [published README](https://docs.rs/crate/gpui/0.2.2/source/README.md) | `gpui_platform::application()` in [immutable README][readme], lines 15–21 |
| Platform packages | No `gpui_platform` dependency in the [published normalized manifest](https://docs.rs/crate/gpui/0.2.2/source/Cargo.toml) | Separate `gpui_platform` and `gpui_windows`, both source version `0.1.0`; Windows platform selection links `gpui_windows` |
| Publication status | Registry artifact exists and has the checksum above | `gpui` explicitly sets `publish = true`; the two platform manifests inherit `publish.workspace = true`, whose actual root value is **false** (`Cargo.toml:276`). Their source version is not a published-release guarantee |
| Accessibility dependency | No `accesskit` entry found in the published manifest | GPUI uses `accesskit`; Windows backend uses `accesskit_windows` |
| License | Apache-2.0 | GPUI, platform and Windows backend each declare Apache-2.0 |

The published README still describes macOS/Linux, while its package has Windows dependencies. Treat that README wording as incomplete evidence, not proof that the published crate has no Windows backend. Conversely, do not assume it exposes current AccessKit/platform APIs.

The published [VCS metadata](https://docs.rs/crate/gpui/0.2.2/source/.cargo_vcs_info.json) records SHA `69e2130295c2649963eb639fc70b4f2ee8ea1624` and **`dirty: true`**. Therefore that source SHA is not a substitute for the registry archive/checksum when reproducing the exact published crate.

### 3. Windows source at the verified immutable commit

| Capability | Fresh source evidence | C09 limitation |
| --- | --- | --- |
| Windowing/rendering/text | [README][readme]:43 states Win32 and DirectWrite, with no Windows-specific feature required on `gpui_platform`; [Windows manifest][windows] enables Direct3D11, DirectWrite and IME bindings | Verify release rendering, GPU/driver requirements, DPI and text on the actual supported Windows host; this does not promise universal GPU fallback |
| IME | [Window source][window]:1064 updates IME position; [event source][events]:693–755 contains context association, composition and result-string handling | Exercise actual Chinese input widgets, candidate positioning, focus transitions and commit/cancel; source handlers are not native input acceptance |
| UI Automation | [Window source][window]:1080–1100 initializes the AccessKit Windows adapter and updates the accessibility tree | Controls still need stable IDs, names, roles/state and accessible actions; prove native UIA/Narrator use, including virtualized content |
| Accessibility authoring | [Accessibility guide][a11y]:63–103 requires roles for exposed nodes; line 207 documents accessible-action handlers | Painted/custom controls are not automatically complete accessible controls |
| Native dialog seam | [Platform source][platform-source]:804 and 817 implement `prompt_for_paths` and `prompt_for_new_path` | Test owner focus, cancel/close, directory selection and Unicode paths in C09 or the owning native-integration task |

No new application should require a Tauri runtime, WebView, Node or JavaScript. Native Windows APIs/SDK/runtime libraries remain compatible with the approved Rust application architecture. Neither API inspection nor GPUI test contexts establish actual input, accessibility, shutdown, performance or installed-package behavior.

### 4. C09 candidate recommendation

**Evaluate immutable upstream `3e145061d861ed32bb6a08dad7b03f2b5e455ad8` as the current candidate**, using the GPUI/platform packages from that same source tree, because the required Windows accessibility/platform work is present there and the platform crate is not available as a registry release. Keep published `=0.2.2` recorded as the latest stable registry alternative, not as an API-equivalent substitute.

This is a candidate recommendation, **not a selected production dependency or successful build**. At C09 start, refresh official metadata again; if a newer candidate is evaluated, record its immutable revision and rerun the complete native gate. If choosing the registry crate instead, demonstrate the required capabilities from that exact crate. Never mix its older API with current README/platform instructions, use a floating `main`, or label an untested revision compatible.

C16 can finish its shared dependency/toolchain work with this candidate audit and the native result explicitly NOT_RUN. C09 owns the final coherent pin, resolved dependency graph, compiler matrix and native evidence. No GPUI entry was added to a manifest or lockfile in this research.

### 5. Integrity receipts and fetch method

All hashes below are SHA-256 of the fetched response's UTF-8 text bytes. They identify the observed text; they are **not crate package checksums**. Raw source was fetched from the immutable SHA URLs. Metadata endpoints are mutable snapshots.

| Response | SHA-256 |
| --- | --- |
| crates.io `gpui` metadata | `183a1c89373266dc4fc9acde88fcb659f9cee83a77649d6862128dbf05e6bcfe` |
| GitHub main commit metadata | `69d1a1fcd0f8115d0d6798051aec83875df867381b1dc21380b4bbe31f102d19` |
| `rust-toolchain.toml` | `3d9002e97d5c3de93d49a1a36649153da4f40c29b061d0f2e09ea84ee143ba3e` |
| Root `Cargo.toml` | `0561c52b248fbb96c572eba0623d3d90469d732dbcc36ac727fcc46797ead065` |
| GPUI `Cargo.toml` | `a50e210fa0b66b6a0e8e5c62f31023a132f30c0a496cb28da3f0a37051cdb6cd` |
| GPUI `README.md` | `989b10e5914dcc8a1f2568303849aeab2d6aa26f2c5dfead96de0491330ada79` |
| `gpui_platform/Cargo.toml` | `550b454202f6ec460f2989a65e4092b70c87f17ca0c079be6ac577570df0d2c3` |
| `gpui_windows/Cargo.toml` | `e85aaccd29acfb146a18b34ae2e57dc66050859d5663178eb80d29bea2a693e1` |
| Windows `window.rs` | `e0813871c6eb06180b702e3345e00331c3f4a7f062089b599f19580cf30b4cbb` |
| Windows `events.rs` | `712bb4de608a072895ed0228afc164c517a5490efa1bcc66d013672c782dbc71` |
| Windows `platform.rs` | `a1228e691876d83ce31a6c68ca39bac70572fa59ae9830c48b180618037c4178` |
| GPUI `_accessibility.rs` | `bcdeb209fa6309f5c507d9e8918235e75c18751c1985d0ebed632de643d7ea2e` |

Mechanism: PowerShell `Invoke-WebRequest`, read-only official HTTPS, `User-Agent: DevSweep-C16-dependency-research` and `Cache-Control: no-cache`, through an approved `exec_command` sandbox escalation. Normal sandbox DNS had failed, and the browsing API could not open the registry/GitHub API endpoints. No network configuration, credentials, dependencies or tools were changed. HTTP Date headers for immutable raw files were 06:08:40–06:09:43 UTC. The docs.rs README/manifest responses retained CDN Date values from October 13 despite this fresh request; they describe the fixed 0.2.2 release, and are not used to establish latest-version status.

## Files Found / Related Contracts

- `../prd.md:C16-R1–R5, C16-AC2–AC3`: fresh selection audit belongs to C16; final GPUI native pin/build remains C09.
- `../design.md`: latest means verified at implementation time; compatible compiler-policy changes are explicit and tested.
- `../../10-15-gpui-desktop-rewrite/research/dependency-policy.md`: latest compatible libraries supersede treating Rust 1.88 as an absolute constraint; pins and exceptions remain reproducible.
- `.trellis/spec/backend/quality-guidelines.md`: shared dependency changes retain canonical checks and executable safety contracts.
- Prior `gpui-windows-feasibility.md` was contextual only; this file's versions, SHA, toolchain and API observations were freshly fetched from the official sources above.

## Caveats / Not Found

- No dependency archive download, mutation, install, repository clone, native build or runtime validation occurred.
- Registry metadata did not declare GPUI's MSRV. A measured minimum compiler, compatible combined lockfile and actual Windows behavior remain NOT_RUN.
- This checkpoint is time-scoped. GitHub main may move after the recorded request; the identified SHA and source URLs remain immutable.
- The platform crate's 404 and inherited publication setting do not imply that unrelated similarly named forks are substitutes. Only the verified Zed source family is proposed here.

[toolchain]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/rust-toolchain.toml
[root]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/Cargo.toml
[gpui]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui/Cargo.toml
[readme]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui/README.md
[platform]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui_platform/Cargo.toml
[windows]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui_windows/Cargo.toml
[window]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui_windows/src/window.rs
[events]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui_windows/src/events.rs
[platform-source]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui_windows/src/platform.rs
[a11y]: https://raw.githubusercontent.com/zed-industries/zed/3e145061d861ed32bb6a08dad7b03f2b5e455ad8/crates/gpui/src/_accessibility.rs
