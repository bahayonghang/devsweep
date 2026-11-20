# Status 契约版本结论

来源：`11-12-gpui-status-dashboard/research/collectors.md` 第 3 节（2026-11-13）。


### Spec text

- `quality-guidelines.md:76-78`: "JSON uses one V1 envelope with `{schema_version,command,outcome,data,warnings,error}`. Status NDJSON emits closed V1 lifecycle events …"
- `quality-guidelines.md:108`: "wrong Status field/type/unit/terminal … -> test/build failure."
- `quality-guidelines.md:117-118`: "the CLI and future Tauri Status fixtures decode through one closed V1 test contract and are byte-identical"
- `quality-guidelines.md:137-139`: "Closed Status snapshot/live fixture tests … plus byte-identical CLI/Tauri fixtures."
- `bounded-delivery.md` §3: "JSON remains one V1 envelope, and NDJSON retains its closed record schema."
- Additive-in-V1 precedents: `quality-guidelines.md:219-222`: "`ExecutionEvidence.estimated_bytes` is optional and additive (`serde(default, skip_serializing_if …)`; the record keeps `deny_unknown_fields`) … Old lines parse with `None`." `scan-coverage.md:17-33, 55-57`: `coverage` is added with `serde(default)`, "ScanReport remains version 1", and missing legacy coverage decodes as empty.
- V2 precedent: `database-guidelines.md` "Migrations" and `desktop-preferences.md` (closed V2 with read-only V1 migration). This applies to **persisted files**. Status snapshots are never persisted (`status/mod.rs:1-3`: "never persists samples").
- Status-specific precedent: commit `9755a22` added the **required** `gpu` and `thermal` groups to `StatusSnapshotV1`. It kept `schema_version: 1`, updated both closed decoders, the fixtures and the generated types in the same change, and recorded the contract as the "Status PDH probes" scenario (`quality-guidelines.md:635-720`).

### Conclusion: stay on V1, extend additively in one lockstep change

The new fields can stay in `StatusSnapshotV1`, `CpuV1`, `MemoryV1`, `GpuAdapterV1`, `PowerV1` and `NetworkInterfaceV1`, with `schema_version` remaining 1. Reasons:

1. "Closed V1" in this repo means every consumer-side decoder enumerates the exact key set. It does not mean the schema is frozen forever. The gpu/thermal extension did exactly this and stayed V1.
2. Status JSON is a command/IPC boundary, not a persisted store. No old file needs migrating, and the only closed consumers are in this repo and change in the same commit.
3. A V2 bump would break `tools/status-sink-smoke.py:159` and `:174`, which assert `schema_version == 1`. It would also force a dual-version decoder in both the CLI tests and Tauri for no reader that actually needs V1.

Conditions for staying V1:
- **Rust read compatibility** (design: "old snapshot JSON is still readable"). New `Option<T>` struct fields already deserialize missing keys as `None` with serde derive. The new **group** fields `host` and `disk_activity` are `AvailabilityV1<…>`, not `Option`, so they need `#[serde(default = …)]` returning, for example, `unavailable("not_sampled")`. Otherwise an old JSON without `host` fails to deserialize. No Status struct uses `deny_unknown_fields`, so older Rust readers already ignore the new keys.
- **Serialization shape**: existing Status `Option` fields serialize as explicit `null` (for example `PowerV1.charge_basis_points`), and both closed decoders expect the key to be present. To match, new optional fields should be emitted as `null`, without `skip_serializing_if`. The audit-record precedent uses `skip_serializing_if`, so this differs from it.
- Parent design.md:33 asks for this conclusion to be copied into `11-12-gpui-mole-parity-airy-glass/research/`. That is outside this agent's write scope; the main agent should copy it.

### What breaks in the Tauri closed decoder (`desktop/src/api/contract.ts`) and what must change

| Location | Breaks because | Required change |
|---|---|---|
| `SNAPSHOT_KEYS` `:1474-1477` + `exact(input, SNAPSHOT_KEYS)` `:1663` | The new top-level `host` and `disk_activity` keys are unknown, so **every** snapshot is rejected. That affects the Status page, live `status_snapshot` events and the HUD `hud-status` snapshot payload. | Add both keys. Add `decodeHostValue` and `decodeDiskActivityValue` through `decodeAvailability`. |
| `decodeCpuValue` `:1517` | `exact(["system_utilization_basis_points"])` | Add `frequency_mhz` (nullable unsigned). |
| `decodeMemoryValue` `:1523` | exact key list | Add `commit_used_bytes` and `commit_limit_bytes` (nullable). Consider adding the invariant used ≤ limit. |
| `decodeNetworkValue` iface `:1556` | exact key list | Add `kind` (nullable `oneOf` of the closed enum). |
| `decodePowerValue` `:1569-1575` | exact key list, plus the invariant "missing battery must not report charge" | Add the 3 nullable fields. Extend the invariant so `battery_present:false` requires all of them to be null. |
| `decodeGpuValue` adapter `:1590` | exact key list | Add `name` (nullable string). |
| `UNSUPPORTED_CODES` / expected-list check `:1471-1473, :1667-1672` | Today `physical_disk_activity` is always expected. If core makes it conditional on `disk_activity` having a value (like gpu/thermal), the decoder must apply the same rule, or it throws "unsupported capabilities do not match". Any new `UnsupportedCode` must be added here. | Mirror whatever rule core adopts (see open question 2). |
| `desktop/src/api/types.gen.ts` | Generated by `desktop/scripts/generate-types.mjs:54-57` from fixtures `status/snapshot.json`, `status/live-snapshot.json`, `status/snapshot-completed.json` and `status/hud-snapshot.json`. Do not hand-edit it. | Regenerate after the fixtures are updated. Fixtures should include both a value and a `null` for each nullable field so quicktype infers `T \| null` (I). |
| Fixtures | `desktop/src/api/fixtures/status/{snapshot,snapshot-completed,event-snapshot,live-snapshot,hud-snapshot}.json`; `desktop/src/api/fixtures/contract-variants.json` (`DesktopStatusSnapshotResult` around `:236-262`, status events around `:298-343`) | Update all of them. `desktop/src/api/fixtures/status/snapshot.json` is **byte-identical** to `crates/devsweep-cli/tests/fixtures/status/snapshot.json` (V, `cmp`), and they must stay so. |

The same applies on the CLI side. `crates/devsweep-cli/tests/cli_contract.rs` contains its own closed decoder (`status_v1_contract`: `ClosedObject::finish` rejects unknown fields at `:739-746`, structs around `:470-600`, `UnsupportedCode` at `:574-581`, expected-list logic around `:837`, and `verify_closed_schema_rejections` at `:1561+`). It compares `fixtures/cli/status-snapshot.json` against `status-snapshot.tauri.json` and `status-live.ndjson` against `status-live.tauri.ndjson` (`:1361-1425`). All four fixtures plus `fixtures/status/snapshot.json` need the new fields.

Rust struct-literal sites that stop compiling when fields are added (they are fixed in the same change): devsweep-core `status/{health,mod,network,pdh,sampler,system,tests}.rs`; devsweep-cli `presentation/status.rs` (6), `tui/modes/status/tests.rs` (1), `tests/cli_contract.rs` (12); `desktop/src-tauri/src/{hud,tray}.rs` (1 each); `desktop-gpui/src/status_panel.rs` (4). `status/health.rs:386` maps `UnsupportedCode` to a string and must cover any new variant.

Tool scripts: `tools/status-sink-smoke.py` checks only `schema_version == 1` and the envelope shape, so it is unaffected by V1 additions (V by reading `:153-180`). `tools/summarize-native-status.py` reads resource-sampler output, not snapshot fields, so it is unaffected (V by reading). Both should still be run once per implement.md step 5.

---

