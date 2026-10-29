# C11 release evidence, 2026-10-26

HEAD at the start of the session was `7ef934b`. The measured GPUI release is an
uncommitted build. rustc 1.99.0 via the mbx `cargo` is not an MSRV 1.88.0 pass.
No commit, push, archive, install, or Tauri removal was performed.

Binary SHA-256 `9F1F6D1CD113AC21E7C743FC4CF95DA62E9DC60A3159938F57151E5A2EC93355`,
16704512 bytes. That hash includes the bucketed dedupe and the discovery
cooldown. It does not include the later Optimize/Software display lines.

## History keyboard

`foreground_ok=True`. F10, Down, Down, Down, Enter. The page read
`clean | op-c11-r10-history | succeeded` from an isolated store. The empty-store
sentence and the unreadable-store sentence were both absent. `WM_CLOSE` exited 0.
FocusPattern exposed count was 0. `FOCUS_PATTERN_RESULT=NOT_PASS`.

## Native AI review

`DEVSWEEP_AI_REVIEW_HOME` was
`C:\Users\lyh\AppData\Local\Temp\devsweep-gpui-c11-r10-ai\profile\.codex`.
`USERPROFILE` and `HOME` for that process were the isolated profile parent, not
the real user profile. F8 opened the review with no home line. Enter scanned.
The UI showed that exact home, 14 observed rows, 7 sensitive, and the cache row
ids that CLI `clean ai scan --provider codex --home` returned for the same
directory. Real `.codex`, `.claude`, and Cursor paths were absent. No execute
and no restore.

## 10k rows

An isolated tree of 10000 `.pytest_cache` directories presented
`Scan finished. 10000 targets found.` and 48 visible rows. Nearest-rank p95
upper bounds, including one UIA read after a 20 ms gap:

| Action | n | p95 | Budget | Result |
| --- | ---: | ---: | --- | --- |
| Search | 100 | 52.764 ms | 100 ms | within, as an upper bound |
| Select (Space) | 100 | 50.420 ms | 100 ms | within, as an upper bound |
| Scroll (PageDown) | 97 | 59.410 ms | 100 ms | within, as an upper bound |
| Cancel acknowledgement | 6 | 36.226 ms | 100 ms and n>=100 | sample count unmet |
| PageDown Present intervals, no UIA polling | 119 | 45960 us | 16.7 ms | NOT_PASS |

The 16.7 ms budget was not relaxed. An earlier scroll trace that polled UIA
between keys had p95 222423 us and is not the formal frame sample.

C11-AC1 through C11-AC4 stay unchecked.
