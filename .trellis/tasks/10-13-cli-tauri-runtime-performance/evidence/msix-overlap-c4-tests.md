# C4 MSIX overlap tests

The three C4 sources still match the static record
`dist/performance/cleanup-earth/msix-overlap-static-check.json`:

| File | SHA-256 |
| --- | --- |
| `crates/devsweep-core/src/software/msix.rs` | `9f26cb84ac66dc95774ab36506cbce269b172b5794b01077a7a687db16eada1d` |
| `crates/devsweep-core/src/filesystem/sizing.rs` | `913c106f7b600a50df819f3b6a046efc6744933106c9a7b06c4fa27db8fefa9c` |
| `crates/devsweep-core/src/filesystem/mod.rs` | `d2eabd44fd11ff73207f822eb48dcf69c0e13db82d1e3876b4125c462d7a7c71` |

Commands used `--target-dir dist` and did not set `CARGO_TARGET_DIR`.
The unrelated `mbx` cargo shim PID 116744 was left running.

| Group | Log | SHA-256 | Result |
| --- | --- | --- | --- |
| `software::msix::overlap_tests` `--test-threads=1` | `dist/performance/cleanup-earth/msix-overlap-overlap-tests-02.txt` | `5e4ac87333ef1642e39257f6fe7e81d872f9c716cc3083c16b2ed2138b291011` | 4 passed, 0 failed, 0.01 s |
| `filesystem::sizing::tests` | `dist/performance/cleanup-earth/msix-overlap-sizing-tests-02.txt` | `4f828f329c446207621cb6bd1c0b8242b13c2b3511c600fd4576df820da37311` | 28 passed, 0 failed, 0.16 s |
| `software::` | `dist/performance/cleanup-earth/msix-overlap-software-tests-02.txt` | `33de9c021d1cd20d700a3c486ebcf5252c0efc899fa6aa2e6277928e3937e83f` | 58 passed, 0 failed, 2 ignored, 6.84 s |

The earlier streaming log remains
`msix-overlap-streaming-01.txt`
(`e0de3d9b8cda9f2ee087b3650508aa8c992a8482e20d67b5c60a1a084275c941`),
4 passed. Those four tests also passed again inside the sizing group.
The two ignored software tests are the opt-in native MSIX diagnostics and
were not executed.

## Timeout gates

Streaming `Gate::wait` records `timed_out` and does not open the gate.
`full_pipeline_cancel_and_worker_panic_release_producer_before_worker_join`
and `producer_panic_waits_for_every_submitted_root` require the started,
blocked, and resumed observations and assert `!timed_out`. A wait expiry
fails those assertions. The older worker and cancel probes call
`assert!(!timeout.timed_out(), ...)` before continuing. None of these gates
turn a timeout into a pass.

## Not yet claimed

This file does not claim the full `just ci` gate, a frozen C4 identity, or a
Software performance pass. The independent review was still outstanding when
these commands finished.
