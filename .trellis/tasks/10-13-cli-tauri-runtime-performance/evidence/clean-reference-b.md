# Corrected frontend reference B

## Preservation

After the functional Clean scope/coverage/rescan work, search-authority
correction and Earth initialization guard, the coordinator froze two production
builds under the independent ignored `dist/performance/cleanup-earth/B/`:

- `frontend-native/`: normal Vite main/HUD build, 11 files, no benchmark entry.
- `frontend-benchmark/`: real ReviewPage/TargetTable, core wire decoders and
  reducer, with fixed 100/1000/10000-result fixtures.

Sibling `frontend-native-manifest.json` and
`frontend-benchmark-manifest.json` record every file's size and SHA-256.
The production native reference must embed these frozen frontend assets when
the corrected Rust reference is ready, rather than a later optimized UI.

Build commands use Node 22.23.2 and bundled npm 10.9.8. The benchmark build was
`vite build --config vite.clean-benchmark.config.ts --mode B-corrected`;
normal production build was `npm run build`. Both passed.

## Measurement

Exact driver invocation:

```text
node tools/clean-render-benchmark.mjs dist/performance/cleanup-earth/B-ui-measure-01 B-corrected dist/performance/cleanup-earth/B/frontend-benchmark
```

The fresh task-owned Chrome profile used a loopback-only fixture server and
ephemeral debugging port verified against its own DevToolsActivePort identity.
The browser closed normally; forced teardown was false and exit was zero.
No user browser, native cleaner or mutation command participated.

Five warmups precede 30 measured iterations per size/surface. Bulk selection
includes both select-all and deselect-all per iteration. Preview replay parses
and decodes fresh objects for each event; it does not reuse target-array identity.
Full target retention and selection/exclusion counts are asserted. The primary
interaction clock spans the real checkbox handler, React layout commit and two
RAF boundaries allowing paint. It measures handler-to-visible, not OS input
scheduling. Parse, closed decoding and reducer work are recorded separately.

| Targets | Review row p95 | Bulk selection p95 | Preview replay p95 | Review DOM elements |
| --- | --- | --- | --- | --- |
| 100 | 14.1 ms | 14.1 ms | 14.0 ms | 2,453 |
| 1,000 | 48.9 ms | 48.8 ms | 15.3 ms | 24,106 |
| 10,000 | 497.8 ms | 459.6 ms | 128.5 ms | 240,636 |

At 10,000 targets, report/progress closed decoding p95 was 11.5/13.4 ms and
report reducer p95 was 0.3 ms. The 100 ms interaction criterion therefore fails
for both large-result surfaces before optimization. This evidence supports
reducing rendered elements and repeated presentation work; it does not justify
dropping targets, decoder checks or plan authority.

Raw samples, environment, fixture hashes and individual case records remain in
`dist/performance/cleanup-earth/B-ui-measure-01/`. The checked-in compact
[record](clean-reference-b.json) retains build/fixture identity, method, p95,
DOM/target counts and teardown. Final candidate measurements must use the same
fixture hashes and driver protocol. This is frontend B, not a claim that the
native corrected reference or entire runtime acceptance is complete.
