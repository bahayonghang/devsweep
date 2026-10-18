# Acceptance ledger

2026-10-15: **implementation approved; C16 active**. Document validation and user
approval do not prove product behavior. Evidence will be recorded per child; no
real user cleanup, uninstall or maintenance is authorized as a validation shortcut.

| Item | Criteria | State | Evidence |
| --- | --- | --- | --- |
| Parent | P-AC01 through P-AC12 | NOT_RUN | Mandatory gates remain open: hosted matrix, the performance protocol's relative baselines, UIA focus/IME, real Recycle Bin, and native defrag/dism/w32tm. Rust 1.88.0 is now actually executed on Linux (see C16) but not on Windows. No aggregate completion claim. |
| C01 | C01-AC1, C01-AC2, C01-AC3 | CHECKED_NOT_ACCEPTED | All three boxes checked in the child prd. Not archived. C16-AC3 is still open. The file's Windows-only tests are now `#[cfg(windows)]`-gated so the hosted Linux jobs can run. |
| C02 | C02-AC1, C02-AC2, C02-AC3 | CHECKED_NOT_ACCEPTED | All three boxes checked. Real Windows UAC prompt remains NOT_RUN inside C02-AC2. Not archived. |
| C03 | C03-AC1 through C03-AC4 | CHECKED_NOT_ACCEPTED | All four boxes checked. The Recycle Bin clause is now measured for real: the production scan -> plan -> preview -> execute flow exited 0, the source path was gone, and the shell Recycle Bin count rose 201 -> 202. A real cloud-files reparse tag remains NOT_RUN. Not archived. |
| C04 | C04-AC1, C04-AC2, C04-AC3 | CHECKED_NOT_ACCEPTED | All three boxes checked. Real MSIX removal remains NOT_RUN inside C04-AC3. Not archived. |
| C05 | C05-AC1 through C05-AC4 | CHECKED_NOT_ACCEPTED | All four boxes checked. GPUI parity stays with C12. Not archived. |
| C06 | C06-AC1, C06-AC2, C06-AC3 checked; C06-AC4 open | NOT_ACCEPTED | Native defrag/dism/w32tm evidence remains NOT_RUN. The `dead_startup` `.lnk` path now moves the shortcut to the Recycle Bin through `SystemTrashRunner` instead of `fs::remove_file`, and the `.reg` backup records each moved link as a `;` comment. |
| C07 | C07-AC1 through C07-AC4 | CHECKED_NOT_ACCEPTED | All four boxes checked. Not archived. |
| C08 | C08-AC1 through C08-AC5 | CHECKED_NOT_ACCEPTED | All five boxes checked. Not archived. |
| C09 | C09-AC1, C09-AC4 checked; C09-AC2, C09-AC3 open | NOT_ACCEPTED | A fresh probe of release SHA-256 `D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061`, 14179328 bytes, exposed a 13-node UIA tree with Window/Pane/Button/Tab/5x TabItem/Edit/2x ListItem roles and Chinese labels. Keyboard focus, keyboard-only operation and IME composition remain NOT_RUN: `HasKeyboardFocus` was False on every node while `GetGUIThreadInfo.hwndFocus` equalled the window, `SetForegroundWindow` returned False even after `AttachThreadInput`, and `ImmGetContext` returned NULL. Reproducibility was re-tested and initially FAILED: four clean builds produced four different SHA-256 values, differing in 24 bytes inside the MSVC Rich header and PE timestamp region. `RUSTFLAGS='-C link-arg=/Brepro'` fixed it; two clean builds now both give `DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34` at 14179328 bytes, stable across four builds. `justfile` now has `gpui-check`/`gpui-test`/`gpui-build`/`gpui`; `just gpui` exits 0. Hosted native CI remains NOT_RUN. |
| C10 | C10-AC1, C10-AC2, C10-AC4 checked; C10-AC3 open | NOT_ACCEPTED | Text scale, overflow tray, UI Automation names, zh-CN copy, and a Settings page showing the word `reduced` were sampled. `SPI_GETHIGHCONTRAST` returned `dwFlags` `0x0000007E` with high contrast off, so no high-contrast window was launched. Narrator was not run. No frame interval was measured. |
| C11 | C11-AC1 through C11-AC4 | NOT_ACCEPTED | Isolated digest, recovery, search, and first-launch evidence exists. Keyboard and ListItem names show the `>` marker on the isolated mypy row after two Alt+Down chords. A preview in that review flow remains NOT_RUN. Real Recycle Bin and the performance protocol remain NOT_RUN. |
| C12 | C12-AC1, C12-AC2 checked; C12-AC3, C12-AC4 open | NOT_ACCEPTED | Native list virtualization, screenshots, and UIA remain NOT_RUN. |
| C13 | C13-AC2 checked; C13-AC1, C13-AC3, C13-AC4 open | NOT_ACCEPTED | UIA selection, OS crash release of the real execution state, and the 20-cycle native measurement remain NOT_RUN. |
| C14 | C14-AC4 checked; C14-AC1, C14-AC2, C14-AC3 open | NOT_ACCEPTED | Tauri was not removed. Startup and memory absolute limits now have real samples on the single-window spike (p95 616 ms of a 1500 ms budget; 113 MiB private of a 200 MiB budget; 20/20 clean exits with 0 orphans). **Idle CPU FAILS the frozen budget: median 0 % passes, p95 7.44 % exceeds the 1.0 % limit** over 150 retained 200 ms samples. Every relative clause against a fresh Tauri baseline remains NOT_RUN because no fresh Tauri baseline was built. No packaged installer. See evidence/c14-perf-round3.md and c14-idle-cpu-round3.md. |
| C15 | C15-AC1 through C15-AC5 | CHECKED_NOT_ACCEPTED | All five boxes checked. GPUI parity stays with C11. Not archived. |
| C16 | C16-AC1, C16-AC2, C16-AC4 checked; C16-AC3 open | NOT_ACCEPTED | `just ci` exit 0, `cargo deny check all` exit 0, and a non-incremental GPUI release pair match on rustc 1.99.0. Declared MSRV 1.88.0 **is now actually executed on Linux**: with `RUSTC` pinned to the installed `1.88.0-x86_64-unknown-linux-gnu` and cargo 1.98.0, `cargo check` and `cargo test -p devsweep-core -p devsweep-cli --locked --all-targets` both exit 0 (770 passed, 0 failed). Cargo was verified to invoke the 1.88.0 `rustc` binary directly. Windows/macOS 1.88.0 and the hosted matrix remain NOT_RUN. No child is archived. |

Historic C3 Software failure remains in the archived evidence described in
research/repository-audit.md. No GPUI measurement is inferred from it.
Planning validation is recorded separately in planning-validation.md.


