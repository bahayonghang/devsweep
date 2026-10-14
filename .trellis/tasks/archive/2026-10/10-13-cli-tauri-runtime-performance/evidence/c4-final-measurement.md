# C4-final measurement

Frozen identity, unchanged during these runs:

| Artifact | SHA-256 |
| --- | --- |
| Source archive | `899df4cb2507c45b4e5881e4a40217f1ffcf7e44c460590ba34ed4b8b34b0bc7` |
| CLI | `26b428fb16d5180a61bc8f86ec397428023f78fc937e119717a5df30aaab6c6a` |
| Native | `7686605676f16138d5cd6b9c2b83b9cfab0f1dc293ee81e283832ee451aec2dd` |

`CARGO_TARGET_DIR` was unset. Output is
`dist/performance/cleanup-earth/final-C4-final-01/`. No DevSweep process
remained afterward.

The Clean runner reads `fixture-manifest.json` as UTF-8. The first PowerShell
launch corrupted the Unicode workload name, and a locale `read_text()` then
disagreed with the live tree even though the UTF-8 content identity matched.
That empty `cli-B3` directory was removed before the successful Python runner.
The 7205-file manifest identity stayed
`4067adeef4c76e4d6abc7bf8b3c09142b7219f315d8d9c199a9b393836898212`
before and after both CLI series.

## Provider parity

Six B3/C4-final cases passed complete envelope and exit parity: go-native,
known-configured, npm-configured, nested-python, partial-roots, and
yarn-delegation. No cleanup ran.

## Clean pairs

Each series is first use, one discarded warmup, and five alternating pairs.
Full-envelope parity held for every retained scan. Median gate is candidate
median ≤ reference × 1.20. Private and thread gates apply only when both
builds have two or more 200 ms root samples.

| Series | Workload | Reference median / p95 ms | C4 median / p95 ms | Median | Private | Threads |
| --- | --- | ---: | ---: | --- | --- | --- |
| B3 | many-100 | 35354 / 53230 | 1658 / 1792 | pass | pass | pass |
| B3 | overlap | 33655 / 36978 | 1794 / 2398 | pass | pass | pass |
| B3 | wide | 190 / 224 | 193 / 224 | pass | unavailable | unavailable |
| B3 | deep | 68 / 94 | 71 / 74 | pass | unavailable | unavailable |
| B3 | Unicode path 测试 | 63 / 68 | 65 / 77 | pass | unavailable | unavailable |
| A | many-100 | 44134 / 48197 | 1745 / 2343 | pass | pass | pass |
| A | overlap | 39783 / 42990 | 1620 / 1772 | pass | pass | pass |
| A | wide | 173 / 272 | 170 / 211 | pass | unavailable | unavailable |
| A | deep | 65 / 65 | 72 / 74 | pass | unavailable | unavailable |
| A | Unicode path 测试 | 70 / 75 | 72 / 100 | pass | unavailable | unavailable |

Unavailable resource gates are not passes. The short scans finished before
two 200 ms root samples.

## Earth A/C4

Five alternating launches, still then animated or the reverse, 5 s settle and
15 s retain. Viewport stayed 1080×720 at DPR 1.25. Still frames recorded 0
RAF callbacks; animated frames recorded about 2157–2162. Every launch passed
the harness lifecycle assert, and normal close left no DevSweep process.
There is no absolute CPU, GPU, or whole-app gate. CPU is percent of one
logical core.

| Build | State | Root CPU median / p95 | Tree CPU median / p95 |
| --- | --- | ---: | ---: |
| A | still | 0 / 0.211 | 0.422 / 0.528 |
| A | animated | 0 / 0.627 | 7.995 / 8.646 |
| C4 | still | 0 / 0 | 0.106 / 0.317 |
| C4 | animated | 0 / 0 | 2.024 / 27.163 |

Host and owned-tree figures are separate. The animated C4 tree p95 is one
high window, not a GPU-completion result.

## Not measured

The formal native Software inventory series was not repeated. C3 p95
5211.467 ms remains the last formal result against the 5000 ms limit. The
combined GUI acceptance was not launched.
