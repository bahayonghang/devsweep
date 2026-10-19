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
| C09 | C09-AC1, C09-AC4 checked; C09-AC2, C09-AC3 open | NOT_ACCEPTED | C09-AC2 is now four-of-five PASS: labels/roles PASS, keyboard-only operation PASS (alt-left/alt-right while the Note holds focus), UIA selection PASS (`IsSelected` tracked 清理→软件→优化), and UIA focus **PASS after a real foreground cycle** (`FocusedElement` = `Note`/Edit with `hasFocus=True`, 1 of 13 nodes) with a documented upstream first-activation gap in AccessKit seeding. IME stays NOT_RUN because this session gives no IME context even to Notepad. C09-AC3 still needs hosted native CI, which requires a push. |
| C10 | C10-AC1, C10-AC2, C10-AC4 checked; C10-AC3 open | NOT_ACCEPTED | Text scale, overflow tray, UI Automation names, zh-CN copy, and a Settings page showing the word `reduced` were sampled. `SPI_GETHIGHCONTRAST` returned `dwFlags` `0x0000007E` with high contrast off, so no high-contrast window was launched. Narrator was not run. No frame interval was measured. |
| C11 | C11-AC1 through C11-AC4 | NOT_ACCEPTED | Isolated digest, recovery, search, and first-launch evidence exists. Keyboard and ListItem names show the `>` marker on the isolated mypy row after two Alt+Down chords. A preview in that review flow remains NOT_RUN. Real Recycle Bin and the performance protocol remain NOT_RUN. |
| C12 | C12-AC1, C12-AC2 checked; C12-AC3, C12-AC4 open | NOT_ACCEPTED | Native list virtualization, screenshots, and UIA remain NOT_RUN. |
| C13 | C13-AC2 checked; C13-AC1, C13-AC3, C13-AC4 open | NOT_ACCEPTED | UIA selection, OS crash release of the real execution state, and the 20-cycle native measurement remain NOT_RUN. |
| C14 | C14-AC4 checked; C14-AC1, C14-AC2, C14-AC3 open | NOT_ACCEPTED | Tauri was not removed. Startup and memory absolute limits now have real samples on the single-window spike (p95 616 ms of a 1500 ms budget; 113 MiB private of a 200 MiB budget; 20/20 clean exits with 0 orphans). Idle CPU absolute budget on commit `0d691dd`: round 13 whole owned tree, 150 samples, nearest-rank median 0 and p95 0 (4 nonzero, max 15.1188%, one process, tree private 118923264 bytes). Rounds 11 and 12 are root-process samples with the same median and p95; round 13 saw one owned process. Historical p95 7.4405% and 7.7351% stay in the child evidence files `c14-idle-cpu-round3.md` and `c14-idle-cpu-round10.md`. AC1–AC3 stay open. Feature equivalence, the other protocol rows, a packaged installer, and a native action beyond launch remain NOT_RUN. See `10-15-gpui-acceptance-migration/evidence/c14-idle-cpu-round13.md` and `c14-gpui-launch-round13.md`. |
| C15 | C15-AC1 through C15-AC5 | CHECKED_NOT_ACCEPTED | All five boxes checked. GPUI parity stays with C11. Not archived. |
| C16 | C16-AC1, C16-AC2, C16-AC4 checked; C16-AC3 unchecked | WAIVED_NOT_RUN | On 2026-10-19 the user waived the hosted matrix, Windows rustc 1.88.0, macOS rustc 1.88.0, and clippy under rustc 1.88.0. Those four stay NOT_RUN and are not PASS. They do not block C16. Linux 1.88.0 check, test, and fmt exited 0. rustc 1.99.0 `just ci` exited 0 (916 passed, 0 failed, 5 ignored) on the workspace tree shared by `090249a` and `8b173e6`. `cargo audit` and `cargo deny check all` exited 0 on rustc 1.99.0. That is not an MSRV pass. Directory not archived yet. |

Historic C3 Software failure remains in the archived evidence described in
research/repository-audit.md. No GPUI measurement is inferred from it.
Planning validation is recorded separately in planning-validation.md.




