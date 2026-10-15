# Frozen Clean review/preview benchmark

Current result: **PASS** for the inherited 100 ms interaction p95 budget at
100, 1,000 and 10,000 targets. This is production component/browser evidence,
not native filesystem throughput or a claim that every operation is faster.

The unchanged driver and frozen fixture hashes match the actual pre-edit
baseline. Both use 5 warmups, 30 retained samples, nearest-rank p95, real
production components and an isolated owned Chrome profile. Current measurement
ran after compilation stopped and while native work was paused. Exact host,
browser, source/fixture hashes and all six paired records are in
`render-performance.json`; raw current samples are under
`dist/cleanup-home-final/render-performance/`.

| 10,000-target metric | Before | Current |
| --- | ---: | ---: |
| Row update visible p95 | 21.0 ms | 14.1 ms |
| Select all visible p95 | 14.1 ms | 14.3 ms |
| Preview replay visible p95 | 13.6 ms | 13.6 ms |
| Report parse p95 (review) | 12.6 ms | 12.8 ms |
| Closed decode p95 (review) | 12.9 ms | 14.9 ms |
| Review DOM elements | 1,962 | 4,240 |
| Preview DOM elements | 1,729 | 1,729 |
| Mounted target rows | 100 | 100 |
| Retained targets | 10,000 | 10,000 |

Owner grouping adds bounded review markup; the DOM increase is explicit, not
presented as a reduction. Parse/decode timings are reported separately and are
not assigned an invented budget. Full report retention and the 100-row mount
cap passed for every fixture size. The new 10,000-owner home benchmark is a
separate workload documented in `home-performance.md`.

The browser exited normally with code 0, no forced teardown. Production test
pages were preserved outside the packaged frontend directory. Reproduction:

```powershell
& 'C:/Users/lyh/AppData/Local/mise/installs/node/22.23.2/node.exe' tools/clean-render-benchmark.mjs dist/cleanup-home-final/render-performance-new home-final dist/cleanup-home-final/benchmark-builds/clean-review measure
```

This does not rerun or clear the historical Software C3 failure or unrelated
Analyze/Optimize native latency gates. Their old numbers are not current
acceptance for this checkout.
