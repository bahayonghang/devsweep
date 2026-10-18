# Acceptance ledger

2026-10-15: **implementation approved; C16 active**. Document validation and user
approval do not prove product behavior. Evidence will be recorded per child; no
real user cleanup, uninstall or maintenance is authorized as a validation shortcut.

| Item | Criteria | State | Evidence |
| --- | --- | --- | --- |
| Parent | P-AC01 through P-AC12 | NOT_RUN | Mandatory gates remain open: Rust 1.88.0 MSRV, hosted matrix, performance protocol, UIA/IME, real Recycle Bin, and native defrag/dism/w32tm. No aggregate completion claim. |
| C01 | C01-AC1, C01-AC2, C01-AC3 | CHECKED_NOT_ACCEPTED | All three boxes checked in the child prd. Not archived. C16-AC3 is still open. |
| C02 | C02-AC1, C02-AC2, C02-AC3 | CHECKED_NOT_ACCEPTED | All three boxes checked. Real Windows UAC prompt remains NOT_RUN inside C02-AC2. Not archived. |
| C03 | C03-AC1, C03-AC2, C03-AC4 checked; C03-AC3 open | NOT_ACCEPTED | `user_temp_foreign_sid_is_ownership_unknown` exited 0. A real cloud-files reparse tag and the Recycle Bin remain NOT_RUN. |
| C04 | C04-AC1, C04-AC2, C04-AC3 | CHECKED_NOT_ACCEPTED | All three boxes checked. Real MSIX removal remains NOT_RUN inside C04-AC3. Not archived. |
| C05 | C05-AC1 through C05-AC4 | CHECKED_NOT_ACCEPTED | All four boxes checked. GPUI parity stays with C12. Not archived. |
| C06 | C06-AC1, C06-AC2, C06-AC3 checked; C06-AC4 open | NOT_ACCEPTED | Native defrag/dism/w32tm evidence remains NOT_RUN. |
| C07 | C07-AC1 through C07-AC4 | CHECKED_NOT_ACCEPTED | All four boxes checked. Not archived. |
| C08 | C08-AC1 through C08-AC5 | CHECKED_NOT_ACCEPTED | All five boxes checked. Not archived. |
| C09 | C09-AC1, C09-AC4 checked; C09-AC2, C09-AC3 open | NOT_ACCEPTED | UIA focus and IME remain NOT_RUN. Current non-incremental release pair matches SHA-256 `095f51d77da5da5729c2341d24c726d460ca1a44ede2204789c13227e521fe62`, 13437952 bytes. Hosted native CI and Rust 1.88.0 remain NOT_RUN. |
| C10 | C10-AC1, C10-AC2, C10-AC4 checked; C10-AC3 open | NOT_ACCEPTED | Text scale, overflow tray, UI Automation names, zh-CN copy, and a Settings page showing the word `reduced` were sampled. `SPI_GETHIGHCONTRAST` returned `dwFlags` `0x0000007E` with high contrast off, so no high-contrast window was launched. Narrator was not run. No frame interval was measured. |
| C11 | C11-AC1 through C11-AC4 | NOT_ACCEPTED | Isolated digest, recovery, search, and first-launch evidence exists. One Down key changed Clean to Software. PID 87176 showed the `>` marker move from an isolated pytest row to a mypy row while Clean stayed selected. UI parity, real Recycle Bin, performance protocol, and UIA remain NOT_RUN. |
| C12 | C12-AC1, C12-AC2 checked; C12-AC3, C12-AC4 open | NOT_ACCEPTED | Native list virtualization, screenshots, and UIA remain NOT_RUN. |
| C13 | C13-AC2 checked; C13-AC1, C13-AC3, C13-AC4 open | NOT_ACCEPTED | UIA selection, OS crash release of the real execution state, and the 20-cycle native measurement remain NOT_RUN. |
| C14 | C14-AC4 checked; C14-AC1, C14-AC2, C14-AC3 open | NOT_ACCEPTED | Tauri was not removed. Performance migration and packaged current-binary acceptance remain open. |
| C15 | C15-AC1 through C15-AC5 | CHECKED_NOT_ACCEPTED | All five boxes checked. GPUI parity stays with C11. Not archived. |
| C16 | C16-AC1, C16-AC2, C16-AC4 checked; C16-AC3 open | NOT_ACCEPTED | `just ci` exit 0, `cargo deny check all` exit 0, and a non-incremental GPUI release pair match on rustc 1.99.0. Declared MSRV 1.88.0 and the hosted matrix remain NOT_RUN. No child is archived. |

Historic C3 Software failure remains in the archived evidence described in
research/repository-audit.md. No GPUI measurement is inferred from it.
Planning validation is recorded separately in planning-validation.md.
