# Curated backend contract excerpts: backend-runtime-contracts

Source: [canonical backend quality guidelines](../../../spec/backend/quality-guidelines.md), captured 2026-10-13. Source SHA-256: `a3b16d9ad96671d448b5a8f7903af77168394200cbdf947bbbc159de74059bcc`.

These verbatim relevant sections avoid the context injector's 32768-byte per-file truncation. This task-local snapshot is not a new specification. Before implementation, read the current canonical sections and resolve changes against the child design; do not rewrite the canonical source from this snapshot. Prospective task requirements may intentionally amend current contracts through a targeted spec update. Other scenarios remain in the canonical source.

<!-- Canonical source lines 515-727 -->

### Scenario: Analyze reveal and Recycle Bin move

#### 1. Scope / Trigger

- Trigger: code changes `analysis/actions.rs`, the Analyze snapshot retention
  in `desktop/src-tauri/src/analyze.rs`, or the desktop commands
  `analyze_default_root`, `analyze_reveal`, `analyze_trash_preview`, and
  `analyze_trash_execute`.

#### 2. Signatures

- `analyze_node_path(snapshot, node_id) -> Result<PathBuf, AnalyzeActionError>`.
- `reveal_analyze_node(snapshot, node_id) -> Result<(), AnalyzeActionError>`.
- `preview_analyze_trash(operation_id, snapshot, node_ids) ->
  AnalyzeTrashPreviewV1 { version, operation_id, items, refused, digest }`.
- `execute_analyze_trash(AnalyzeTrashExecutionRequest { operation_id,
  snapshot, node_ids, expected_digest, confirmed, cancel }) ->
  AnalyzeTrashReportV1 { version, operation_id, moved_node_ids, report }`.
- Refusal codes: `protected`, `system_location`, `volume_root`,
  `profile_root`, `analysis_root`, `reparse_point`,
  `changed_since_snapshot`, `not_found`.

#### 3. Contracts

- Analyze is read-only by default. The only mutation is a Recycle Bin move
  through a core-built plan, a live confirmation digest, and an explicit
  second confirmation. The walker, snapshot, and progress types still carry
  no cleanup field.
- The desktop sends only `(operation_id, node_id)` or
  `(operation_id, node_ids)`. The Tauri `AnalyzeCoordinator` retains the last
  terminal snapshot for one operation id; any other id is
  `analyze_stale_operation`. Core rebuilds each path from `root.input` (or
  `root.normalized`) plus node names along `parent_id` links and rejects
  unknown ids and cycles.
- Reveal runs program `explorer.exe` with argv `["/select,", <path>]`
  through `ProcessRunner`, neutral cwd, and a 10-second deadline. Only a
  launch failure (`NotFound` or `InvalidOutput`) is an error, because
  Explorer exits with code 1 on success. Reveal is allowed for every node.
- Trash preview refuses: a path on or overlapping the user protection list
  or a Clean protected subtree (`.ssh`, `.aws`, `.gnupg`, `.docker`, Cargo
  credentials, `.npmrc`, `.config/gh`) or containing the running executable
  (`protected`); a path overlapping `%WINDIR%`, `%SystemRoot%`,
  `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%ProgramW6432%`, or
  `%ProgramData%`, or a path under a volume-root `$Recycle.Bin` or
  `System Volume Information` folder (`system_location`); a volume root
  (`volume_root`); the user profile, a folder that contains it, its
  top-level Desktop, Documents, or Downloads folder, or any other folder
  under the parent of the user profile, such as another account's profile
  (`profile_root`); the analysis root
  (`analysis_root`); a snapshot or live reparse point (`reparse_point`); a
  live type or modification-time mismatch with the snapshot
  (`changed_since_snapshot`); and a missing path or unknown id
  (`not_found`). When one selected node is an ancestor of another accepted
  node, only the ancestor stays.
- Accepted nodes become in-memory Clean targets with
  `CleanAction::MoveToTrash`, rule id `analyze.trash`, `Scope::Global`, and
  target id `analyze.trash:<operation_id>:<node_id>`. A crate-private
  constructor builds the `ValidatedPlan` from these trusted targets; the
  rules registry has no `analyze.trash` rule, so `validate_plan` and
  `plan_execute` still reject such a target from a plan file or IPC payload.
- The preview digest is the Clean `ConfirmationDigest` of that plan. It binds
  the operation id (through the target ids), the exact normalized paths, the
  snapshot sizes, and modification times. Execution rebuilds the plan from the
  retained snapshot, re-applies every refusal, and runs the Clean `Executor`
  with the expected digest, so live authorization, the self-clean guard, and
  the fixed Clean V1 audit journal apply unchanged. Moved bytes count in
  `history::clean_moved_totals()`.
- The audit record keeps the closed Clean V1 shape. It carries no origin
  field; the `analyze.trash` rule id stays in the in-memory plan and digest.
  Adding an origin field would make older builds classify the journal as
  corrupt.
- Permanent delete, Recycle Bin emptying, and moves to other locations are
  not available.

#### 4. Validation & Error Matrix

- Unknown operation id -> `analyze_stale_operation`, no path rebuilt.
- `confirmed: false` or no accepted node -> error before the executor runs.
- Stale digest (any live change, a different operation id, or a changed
  selection) -> `stale_confirmation`, no audit record, no trash move.
- Protection list cannot load -> `analyze_failed`, no preview and no move.
- Reveal launch failure -> `analyze_failed`.

#### 5. Good/Base/Bad Cases

- Good: a user file under the analysis root previews with its size and a
  digest, and execution writes `started` and `succeeded` Clean audit
  records with `action_kind: move_to_trash`.
- Base: selecting a folder and a file inside it previews one item, the folder.
- Bad: the desktop sends a path string, or `plan_execute` accepts an
  `analyze.trash` target from JSON.

#### 6. Tests Required

- Core tests for each refusal class, nested selection, unknown id and cycle
  rejection, digest stability and staleness, operation-id binding, and an
  execution through a fake trash runner that writes Clean audit records.
- Reveal argv test through a captured `ProcessRequest`, and a launch-failure
  test.
- Tauri tests for snapshot retention, stale operation refusal, and the
  command inventory; a bridge test that proves only ids cross the boundary.

#### 7. Wrong vs Correct

Wrong:

```rust
Command::new("cmd").args(["/C", &format!("explorer /select,{path}")]);
```

Correct:

```rust
ProcessRequest {
    program: OsString::from("explorer.exe"),
    args: vec![OsString::from("/select,"), path.into_os_string()],
    ..
}
```

### Scenario: Status PDH probes and tray HUD sampler

#### 1. Scope / Trigger

- Trigger: code changes `status/pdh.rs`, the `gpu`/`thermal` groups of
  `StatusSnapshotV1`, `unsupported_capabilities`, or the Tauri files
  `hud.rs`, `tray.rs`, and the invoke-handler gate in `lib.rs`.

#### 2. Signatures

- `unsupported_capabilities(&AvailabilityV1<GpuV1>, &AvailabilityV1<ThermalV1>)
  -> Vec<UnsupportedCapabilityV1>` (replaces the static list).
- `GpuV1 { adapters: [{ adapter_id, utilization_basis_points }] }` and
  `ThermalV1 { zones: [{ zone_id, temperature_tenths_celsius }] }`.
- `HudSampler::start(interval, sample, emit)`, `HudSampler::stop()`; event
  `hud-status` with closed payload `{type: "sampling"}` or
  `{type: "snapshot", snapshot}`.

#### 3. Contracts

- One PDH query per `StatusSampler` holds the English wildcard counters
  `\GPU Engine(*)\Utilization Percentage` and
  `\Thermal Zone Information(*)\Temperature`, added with
  `PdhAddEnglishCounterW`. The query is primed once and collected once per
  sample; `Drop` closes it on every path. Only the `windows-sys` feature
  `Win32_System_Performance` is added; no new crate.
- GPU: group instances by the `luid_..._phys_<n>` adapter key, sum per engine
  type, keep the busiest type per adapter, and clamp to 10000 basis points.
  Thermal: Kelvin to tenths of °C; drop 0 K and non-finite zones.
- A missing query, counter, instance set, or valid value gives
  `unavailable` with a reason code (`pdh_unavailable`, `counter_missing`,
  `collect_failed`, `counter_read_failed`, `no_valid_data`, `no_instances`).
  It never gives a zero. `gpu_utilization` and `thermal` stay in
  `unsupported_capabilities` exactly when their group has no value. VRAM, fan,
  SMART, and physical-disk activity are always listed.
- Both groups join warning aggregation but not the outcome check: a host
  without thermal zones or GPU counters still reports `success` when the
  other groups are complete, as before the probes existed.
- The HUD sampler owns one thread and one `FlagCancelObserver`. It samples
  at the validated desktop-preference interval (2/5/10 s, default 2 s) only
  while the HUD window is visible. The interval is captured on show. Hide, HUD destroy, main
  window destroy, `RunEvent::ExitRequested`/`Exit`, and `Drop` cancel and
  join it. A second start while running is refused. Tray tooltip updates are
  posted to the event loop, so a join on the event loop cannot deadlock.
- Closing the main window calls `app.exit(0)`; the tray icon goes with the
  process. There is no autostart.
- Without an app ACL manifest, Tauri lets every local window invoke every app
  command. `window_gated` wraps `generate_handler!`: `main` may invoke all
  commands, `hud` only `presentation_settings_get` and `desktop_preferences_get`,
  any other label nothing.
  The `hud` capability grants only `core:event:allow-listen` and
  `core:event:allow-unlisten`.

#### 4. Validation & Error Matrix

- PDH open or counter add fails -> both or one group `unavailable`; the code
  stays in `unsupported_capabilities`.
- All instances invalid on the first rate read -> `no_valid_data`, not zero.
- HUD window invokes a cleanup, uninstall, or trash command -> rejected
  before the command runs.

#### 5. Good/Base/Bad Cases

- Good: two engines of type 3D on one adapter at 70% and 55% report 10000.
- Base: a desktop with no ACPI zones reports `thermal: unavailable` and lists
  `thermal` as unsupported.
- Bad: `thermal: available` with an empty zone list, or a HUD sampler thread
  that survives hide.

#### 6. Tests Required

- PDH fixture tests for grouping, clamp, idle zero versus no instance, 0 K
  drop, and reason codes; capability-list tests for each probe state.
- HUD sampler tests for start on show, cancel and join on hide and drop, no
  sample after stop, the selected cadence/default 2 s, and a refused second start.
- A window-gate test over every shipped command for `main`, `hud`, and an
  unknown label.

#### 7. Wrong vs Correct

Wrong:

```rust
// A missing counter reported as an idle GPU.
Err(_) => AvailabilityV1::available(now, 0, GpuV1 { adapters: vec![] }),
```

Correct:

```rust
Err(reason) => AvailabilityV1::unavailable(reason),
```
