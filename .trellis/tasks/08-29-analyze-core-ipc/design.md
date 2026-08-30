# Design - Analyze Core and IPC

## Fixed resource contract

The immutable V1 snapshot stores at most 250,000 nodes and 10,000 warnings. It
accounts owned memory before every allocation and caps the sum at 268,435,456
bytes. Accounting includes `Vec` capacities, string capacities, node/warning
records, identity sets, pending work, and progress buffers; allocator overhead
is measured separately in the final process gate. No user flag raises a cap.

A dedicated two-worker pool handles one active job; construction failure uses
the identical serial walker. Traversal is iterative with maximum logical depth
1,024. Progress publishes at most 256 changed nodes or every 100 ms, whichever
comes first, through a queue of capacity four; a newer cumulative batch replaces
an older unsent batch. Cancellation is checked at each dequeued entry and before
metadata/child enumeration.

Before a node/warning/work item that would cross a cap is inserted, the walker
marks its nearest represented ancestor incomplete, records one stable
`partial_budget` summary if warning capacity permits, stops scheduling, drains/
joins workers, and returns all represented sizes as lower bounds. It never drops
unknown descendants while calling the parent complete.

## Fixtures and measurements

- `analysis-250k-v1`: deterministic fake adapter with exactly 250,000 nodes,
  one 10,000-child directory, depth 1,024, hard-link duplicates, churn, access
  denial, and a variant that attempts node 250,001 and byte 268,435,457.
- `analysis-native-50k-v1`: generated Windows tree with 50,000 zero/sparse files,
  1,000 directories, long Unicode names, a reparse leaf, deletion/growth churn,
  and an access-denied branch where the host permits the fixture.

Structural tests assert exact accounted bytes and partial cut points. Release
native runs use one warm-up plus five samples, 200 ms process sampling, p95
cancel-to-join <= 500 ms, and no more than two Analyze worker threads. Final
integration applies the full-process private-memory threshold. For this
five-sample child gate, nearest-rank p95 means sorting ascending and selecting
`ceil(0.95 * 5) = 5`, the maximum sample; no interpolation is permitted.

## DTO and adapter boundary

Nodes carry snapshot-local id, parent id, kind, name, lower-bound bytes,
immediate/recursive counts, evidence, warnings, and optional timestamps. A root
record carries normalized input/root/volume identity. Reparse points are leaves.
Hard links use file identity to count bytes once per snapshot and emit an
explicit duplicate-link warning. No node contains actions or implements a
cleanup conversion.

The CLI implements only frozen `analyze scan --root <PATH>
[--format human|json] [--output <FILE>]`. Tauri start/cancel/progress/complete
events include operation id and monotonic sequence; stale/out-of-order events are
discarded. Cancel joins before terminal completion is reported.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-core/src/analysis/mod.rs` | domain coordinator, bounds, public read-only service |
| `crates/devsweep-core/src/analysis/model.rs` | V1 node/warning/snapshot DTO and byte accounting |
| `crates/devsweep-core/src/analysis/walker.rs` | iterative two-worker/serial no-follow traversal |
| `crates/devsweep-core/src/analysis/tests.rs` | fake/native fixture contract and resource tests |
| `crates/devsweep-core/src/lib.rs` | export Analyze read-only API |
| `crates/devsweep-cli/src/application/commands/analyze.rs` | frozen CLI handler/serializer |
| `crates/devsweep-cli/src/application/presentation/analyze.rs` | bilingual Analyze human renderer over the V1 snapshot/outcomes |
| `desktop/src-tauri/src/analyze.rs` | coordinator and typed commands/events |
| `desktop/src-tauri/src/lib.rs` | register Analyze commands/state |
| `desktop/src/api/fixtures/analyze/` | V1 snapshot/progress/error fixtures |
| `desktop/scripts/generate-types.mjs` | generate Analyze DTO types |

The CLI contract task first creates `application/commands/mod.rs` and
`application/presentation/mod.rs`; this task fills only its frozen `analyze.rs`
handler/renderer submodules and does not own either module root.

Rollback removes Analyze registration/domain as one unit and leaves cleanup
sizing untouched.
