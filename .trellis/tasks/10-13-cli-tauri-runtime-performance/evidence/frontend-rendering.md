# Clean frontend rendering: corrected B, lazy C1 and paged C2

2026-10-13. Production component microbenchmarks; these results do not close
native IPC/lifecycle or the complete runtime task. Search correctness was fixed
before B, as recorded in [frontend-reference-correction.md](frontend-reference-correction.md).

## Cause, change and effect

The corrected reference builds every row and all evidence DOM behind closed
details. A selection also regrouped/sorted the report. B's 10k review contained
240,636 DOM elements and missed the 100 ms visible-update budget.

C1 mounted evidence and skipped rows only on native disclosure, skipped hidden
table-header counting, and retained group/order/subtotal derivations while
their inputs were unchanged. It reduced DOM, but the 10k review still failed.
That retained failure justified C2's dependency-free 100-row display pages.

C2 uses native first/previous/next/last buttons and an announced page/range.
The completed report and selection are never sliced. Existing review search
filters display across the full report; a preview-only query similarly reaches
all retained observations without selection. Search changes reset displayed
pages; shorter replacement lists clamp the displayed page. Evidence and full
paths remain accessible and copyable. Decoder and reducer validation are
unchanged; their baseline costs did not justify authority-path changes.

| Count / metric, p95 ms | Corrected B | Lazy C1 | Paged C2 |
| --- | ---: | ---: | ---: |
| 1k row click to visible | 48.9 | 41.9 | 14.0 |
| 1k select-all to visible | 48.8 | 41.7 | 14.0 |
| 1k fresh preview to visible | 15.3 | 14.0 | 14.0 |
| 10k row click to visible | **497.8 fail** | **429.6 fail** | 14.0 |
| 10k select-all to visible | **459.6 fail** | **417.3 fail** | 14.0 |
| 10k fresh preview to visible | **128.5 fail** | 100.0 | 13.5 |
| 10k review DOM elements | 240,636 | 190,636 | 1,962 |
| 10k preview DOM elements | 220,021 | 170,021 | 1,729 |

C2 retains all 10,000 targets in each mode. Every measured select-all operation
asserts exactly 9,412 eligible selected IDs, or zero on deselection. Inspect-only
rows remain excluded. Its review parse/decode/report-reducer p95 was
10.9/10.4/0.2 ms; preview was 10.0/10.8/0 ms. Decode is measured separately, not
silently excluded from an end-to-end native claim. C1 preview decode p95 rose
to 36.5 ms; that raw result remains available and is not attributed to a decoder
change (there was none).

## Method and provenance

- Intel Core Ultra 9 275HX, 24 logical processors; Windows 10.0.26300;
  Headless Chrome 155.0.0.0; 1440×900 CSS pixels; DPR 1.25; Node 22.23.2.
- Same production Vite harness and deterministic `clean-render-v1` fixture;
  5 warm-ups and 30 measured samples per operation, nearest-rank p95.
  Each fresh preview is JSON parsed and closed decoded into new objects before
  dispatch. Review performs real native checkbox `.click()` handlers. Visible
  timing covers the commit and two RAF boundaries; this is handler-to-paint
  opportunity, not native OS input scheduling or GPU completion.
- Full report JSON SHA-256: 100 =
  `dd529385368b6dea23ba77820a7ae79edc91c60afa694e0320ce87ca6c5e0fed`;
  1k = `5dcabf8066bea802b6b999d74e8906186a79989a7937f4769a6ea486ac3b095a`;
  10k = `220aca202a98ea0a198b3c6fd55c40470fe0cc04e838535b473977fa92d3014c`.
  B, C1 and C2 records match all three identities.
- B source identity:
  `1b2b66ed7b475cc47aa1e15c643510630cb696652173cd31cf36705a17d17aed`;
  C1: `735e70e72b470c66ba20b40cbe10bc23fda901ac14f52fec915f5dbaf23dee6c`;
  C2: `df85c3557da48579592cdd36789ba8975ab988801f1b732ce02d06f9af6a4750`.
  These harness hashes cover `desktop/src` text, not an entire repository or
  native binary. The separately preserved build manifest hashes actual files.
- Raw samples and teardown records remain at ignored
  `dist/performance/cleanup-earth/{B,C1,C2}-ui-measure-01/`.
  All three owned browsers closed normally, exit 0, no forced teardown.
  Native compilation/tests were paused during each timing run by explicit
  coordination. OS cache state and other machine activity were uncontrolled.
- C2's measured build is preserved at
  `dist/performance/cleanup-earth/C2-ui-build/` with a sibling manifest. The
  standard production build cleared the temporary `desktop/dist` benchmark
  directory; an identical rebuild was copied out, its HTML hash and embedded
  source identity verified against the measurement. The asset filename/hash
  identity is unchanged. B's independent preserved directory was not touched.

Commands, from root except Vite in `desktop/`:

```powershell
node node_modules/vite/bin/vite.js build --config vite.clean-benchmark.config.ts --mode C1-lazy
node tools/clean-render-benchmark.mjs dist/performance/cleanup-earth/C1-ui-measure-01 C1-lazy
node node_modules/vite/bin/vite.js build --config vite.clean-benchmark.config.ts --mode C2-final
node tools/clean-render-benchmark.mjs dist/performance/cleanup-earth/C2-ui-measure-01 C2-final
```

All invocations used the explicit mise Node 22.23.2 executable and a scoped
Node 22 PATH. This checkpoint has one 30-sample run per candidate/case, not five
alternating process-level B/C pairs. It proves the deterministic DOM reduction
and this microbenchmark's budget; extended paired/native evidence remains
with runtime integration.

## Regression and quality checks

- Full ESLint and `tsc --noEmit`: pass.
- Generator: 48 named fixtures; full Vitest includes deterministic generation
  and closed IPC contract checks. Rust Serde wire parity remains the native gate.
- Full Vitest: **58 files, 429 tests pass**, 61.02 s, `--maxWorkers=2`.
- Node development-server/argument tests: **5 pass**.
- Production Vite build: pass; the existing >500 kB chunk warning remains.
- New tests cover full 10k selection/totals, absent/inspect/protected/skipped
  exclusions, replacing twenty fresh cumulative previews without promotion,
  display-only search and the unchanged full dry-run request, lazy evidence and
  skipped DOM, first/middle/last pages, keyboard Enter paging, retained checked
  state, shorter-list clamp, complete last paths and full 10k preview search.
- No new production package, cleanup operation, native process termination,
  decoder relaxation, or transport schema was introduced.

## Visual evidence

The visual-only harness extension renders the actual ReviewPage/TargetTable
at en/zh-CN × 390/800/1024/1440 CSS pixels, DPR 1.25: **16 captures, zero
document horizontal overflow**, 100 rows shown and all 10k retained. Paging
controls wrap at narrow widths. Images at en/zh-CN 390 review, en 1440 review
and zh-CN 800 preview were inspected. Tables keep their existing contained
horizontal scrolling. The component harness excludes native shell and its
root controls; this is not actual Windows scaling or native WebView evidence.

Raw PNGs, their hashes, layout records and normal-close teardown are at
`dist/performance/cleanup-earth/C2-ui-visual-01/`. The visual-only harness edit
followed performance preservation; `tsc`, scoped lint and its Vite build pass.
It did not change production components or the saved C2 measurement artifact.
