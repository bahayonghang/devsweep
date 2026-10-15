# Existing contract loading map

Research snapshot 2026-10-15. This is an index and compact checklist, not a new spec.
The authoritative .trellis/spec/backend/quality-guidelines.md is 72,317 bytes,
larger than the 32,768-byte per-file context-injection limit. Inject this index and
read the applicable complete named sections from the original before editing.
Do not rely on an automatically truncated copy or treat omissions here as waivers.

## Applicable sections in the authoritative quality spec

| Section | Start line at inspected commit | Consumers |
| --- | ---: | --- |
| Forbidden Patterns | 15 | All |
| Five-mode CLI, machine output, and localization foundation | 28 | C01,C15,C16 |
| Execution engine and audit log | 165 | C02,C03,C15 |
| Software execution and durable audit | 313 | C04 |
| Software update check, startup toggle, and leftover move | 423 | C01,C05 |
| Analyze reveal and Recycle Bin move | 515 | C01,C07 |
| Status PDH probes and tray HUD sampler | 635 | C08,C13 |
| Declarative plan trust boundary | 728 | All mutating capabilities |
| Global cache providers | 828 | C03,C15 |
| Declarative rule catalogue | 938 | C03,C15 |
| CI and release archive | 1054 | C14,C16 |
| Cleanup plan ranking and freshness guard | 1175 | C03,C11,C15 |
| Project scanner and JSON cleanup report | 1299 | C03,C07 |
| Scan safety, review reports, and capacity inventory | 1413 | C03,C07,C15 |
| Testing Requirements and Code Review Checklist | 1522,1531 | All checks |

Line anchors are navigation aids. Read by current heading if source changes.

## Compact invariants

- Scan/model/render layers only observe and plan; execution has a distinct owner.
- Saved clean intents are untrusted until validated into the opaque plan boundary.
  Programs and argv are reconstructed from trusted policy, never user shell strings.
- Actual mutations use required review/confirmation, live identity/digest validation
  and durable audit. An audit failure refuses dispatch. No implicit retry of unknown
  outcomes or automatic authority from an observation file.
- Permanent file deletion, Cargo-home cleanup and Docker cleanup remain unavailable.
- Bilingual human output does not localize codes/IDs/digests. Preserve envelope/exit
  contracts, create-new files, stderr diagnostics and real pipe cancellation behavior.
- Software and maintenance have their own identity, audit, irreversible/reboot and
  external-confirmation semantics. Unsupported/guidance never equals execution.
- Analyze scan authority and recycle authority are separate. Rebuild paths from
  retained snapshot identity and revalidate at execution, never from display strings.
- Status unavailable values are not zero. Preserve bounded lossless NDJSON and
  distinguish sampler lifecycle from UI decoration.
- Recycle Bin movement is not verified physically freed storage. Unknown/partial/
  failed/canceled must remain distinguishable.
- Canonical root gate is just ci; native evidence is an additional gate, never a
  synonym for a build or fixture test.

## Related smaller authoritative specs

Use the curated child manifests for database/persistence, provider authority,
bounded delivery, runtime publication, scan coverage, preferences and occupancy.
Read current layer indexes before implementation. React/Tauri implementation details
do not automatically become GPUI requirements; semantic safety contracts still apply.

