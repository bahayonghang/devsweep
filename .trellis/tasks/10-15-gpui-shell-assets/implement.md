# C10 Implementation and verification

Status: 2026-10-19 `just gpui-test` exited 0 (63 passed) and `just gpui-build` exited 0 on rustc 1.99.0. This is not an MSRV 1.88.0 pass. C10-AC1, C10-AC2, and C10-AC4 stay checked. C10-AC3 stays unchecked. Commit `3eac220` waits 16_000 us on every queued frame and no longer calls `DwmFlush`. Release SHA-256 `6828BB1023285CD839EB575778044940B901D1193B734F2AA991B6BAEE9445CD`. A stimulated `Windows.Graphics.Capture` sample has 121 intervals, median 16.300500 ms, nearest-rank p95 19.474600 ms. The 16.7 ms budget is not met. Reduced motion was not switched. Narrator was not started. The round-3 utterance stays NOT_RUN. It is not PASS. See `evidence/c10-ac3-round4-2026-10-19.md`. Round 3 remains `evidence/c10-ac3-round3-2026-10-19.md`. An uncommitted remainder-wait build, SHA-256 `15FD29C93280A18BA709E7EC4ECE98E02859E1F3F55A86EDE0A1C3A5F811442D`, measured nearest-rank p95 25.582100 ms and was not kept. See `evidence/c10-ac3-round5-2026-10-19.md`. Commit `b59b28d` waits on a carried deadline before `RedrawWindow`. Release SHA-256 `6B84E1CEBBF6E042F602F2DEAF9B28BA8B74DF0E06C34EA8E0938C3C5A176E52`. A stimulated `Windows.Graphics.Capture` sample has 125 intervals, median 16.028600 ms, nearest-rank p95 18.469500 ms. The 16.7 ms budget is not met. Reduced motion was not switched. Narrator was not started. The round-3 utterance stays NOT_RUN. It is not PASS. See `evidence/c10-ac3-round6-2026-10-19.md`. An uncommitted spin-until-deadline build, SHA-256 `02A5ECBF5222D3F9BD8C6CD34EFB9344E82EC086839A3EFBB18A12B565AEA9C1`, measured 125 intervals, median 16.009200 ms, nearest-rank p95 20.269400 ms. That p95 is above 16.7 ms and above the committed round-6 p95. The diff was not kept. HEAD still has the carried deadline from `b59b28d`. See `evidence/c10-ac3-round7-2026-10-19.md`. The committed `b59b28d` release SHA-256 `6B84E1CEBBF6E042F602F2DEAF9B28BA8B74DF0E06C34EA8E0938C3C5A176E52` was measured with ETW `Microsoft-Windows-DXGI` EventID 42. 123 intervals, median 16.0296 ms, nearest-rank p95 32.1048 ms. The 16.7 ms budget is not met. One `SetFocus` on `note-input` left UI Automation focus on Narrator's `下一页`. Narrator utterance stays NOT_RUN. It is not PASS. See `evidence/c10-ac3-round8-2026-10-19.md`. An uncommitted synchronous-present build, SHA-256 `CA323B1E1C3F8AC5C4BF9D5C92FB0F16DBFD21351326BA0205E0095BA2CC827F`, length 14141440, measured ETW `Microsoft-Windows-DXGI` EventID 42. 122 intervals, nearest-rank median 16.3087 ms, nearest-rank p95 32.4708 ms. The 16.7 ms budget is not met. The frame process was killed after `WM_CLOSE` stayed unanswered for 20 s. The diff was not kept. One click on `note-input` left focus on `Browse Microsoft Store`. Narrator utterance stays NOT_RUN. It is not PASS. See `evidence/c10-ac3-round9-2026-10-19.md`. An uncommitted deferred-present build, SHA-256 `9A0BA394092FF8F6A37D0D4929761CE9F38DE8075C8A6ABCD316F4360AE0DC32`, length 14140928, measured ETW `Microsoft-Windows-DXGI` EventID 42. 126 intervals, nearest-rank median 16.0575 ms, nearest-rank p95 32.0082 ms. The 16.7 ms budget is not met. The frame process was killed after `WM_CLOSE` stayed unanswered for 20 s. The diff was not kept. `vsync.rs` was not changed. One click did not leave focus on `note-input`. Narrator utterance stays NOT_RUN. It is not PASS. See `evidence/c10-ac3-round10-2026-10-19.md`. The committed `b59b28d` release SHA-256 `6B84E1CEBBF6E042F602F2DEAF9B28BA8B74DF0E06C34EA8E0938C3C5A176E52` was traced with ETW `Microsoft-Windows-Dwm-Core` mask `0x20002`. logman create, stop, and tracerpt exited 0. EventsLost was 0. The events ran in dwm.exe PID 2408. DevSweep PID 189024 does not appear, so no frame interval was calculated. `FRAME_RESULT=NOT_RUN`. It is not PASS. `WM_CLOSE` did not exit the process. A second launch left focus on TermControl PID 180940. Narrator was not started. Narrator utterance stays NOT_RUN. It is not PASS. See `evidence/c10-ac3-round11-2026-10-19.md`. Dependencies C01 and C09 are archived. This child is not archived.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C10-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- GPUI state/action tests on the approved pinned version
- Native shell/settings/locale/accessibility matrix
- `just ci plus explicit GPUI native gate`

Use [the parent protocol](../10-15-gpui-desktop-rewrite/research/performance-protocol.md)
for applicable native measurements. Freeze approved budgets and exact scopes before
candidate work. A failed result never authorizes a relaxed threshold.

## Evidence and stop conditions

- Map every requirement to the acceptance checklist in prd.md.
- Use isolated roots, test registry state, disposable apps and owned processes.
- No real user-cache cleanup, unrelated app uninstall or host-wide repair for tests.
- Keep failures and NOT_RUN/UNVERIFIED platform/hardware results explicit.
- Failed required dependency evidence blocks downstream integration.
- Preserve current rollback behavior until C14 accepts migration.
