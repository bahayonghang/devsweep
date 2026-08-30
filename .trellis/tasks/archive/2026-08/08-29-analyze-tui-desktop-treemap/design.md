# Design - Analyze Presentation

## Fixed render contract

Snapshot data is immutable. A current-directory selector works by node id and
sorts at most that directory's represented children. Desktop squarified layout
returns at most 512 known-size child rectangles plus one `Other` rectangle that
records the exact remaining known bytes and count. Unknown/partial descendants
are listed separately and never receive invented area.

The accessible list is canonical and uses deterministic 200-row pages rather
than rendering 250,000 nodes or adding a virtualization dependency. Breadcrumb,
search, sort, previous/next page, and keyboard drill-down make every represented
node reachable. The linked treemap is supplemental. TUI renders only its viewport
with the same breadcrumbs, proportional lower-bound bars, and evidence labels.

`analysis-250k-v1` includes a 10,000-child directory. After five warm-ups, 30
release-build navigation samples must produce <=513 rectangles, <=900 rendered
DOM elements, p95 pure layout <=50 ms, and p95 React commit <=100 ms. Sampling
records host/build/browser-webview versions and raw durations. Each p95 uses
nearest-rank: sort the 30 values ascending and select
`ceil(0.95 * 30) = 29`; no interpolation is permitted. A miss is a stop, not
permission to raise the limit during implementation.

## State and lifecycle

Mode state stores current node, 200-row page, sort/filter, focus anchor, and
operation id only. It consumes the frozen Analyze DTO and performs no filesystem
access. Leaving the mode asks the shell coordinator to cancel/join before the
snapshot/view is released. Stale ids/sequences are ignored.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-cli/src/tui/modes/analyze/` | hierarchy, paging, breadcrumbs, bars, reducer/tests |
| `desktop/src/modes/analyze/state.ts` | mode-local immutable view/lifecycle state |
| `desktop/src/modes/analyze/selectors.ts` | paging/search/sort/current-directory selectors |
| `desktop/src/modes/analyze/treemap.ts` | pure bounded squarified layout/tests |
| `desktop/src/modes/analyze/AnalyzePage.tsx` | accessible list/treemap/breadcrumb presentation |
| `desktop/src/modes/analyze/AnalyzePage.test.tsx` | states, keyboard, linkage, lifecycle, bilingual tests |
| `desktop/src/modes/analyze/styles.css` | bounded responsive visualization styles |
| `desktop/src/api/contract.ts` | consume generated Analyze decoders only if generator requires registration |

This task does not edit filesystem/core/Tauri DTO ownership, the CLI parser,
shell/spec files, or any cleanup selection/execution module. Rollback unregisters
Analyze presentation without schema migration.
