# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This project is a safety-first cleanup tool. Backend quality checks must prove
that planning and dry-run behavior stay non-mutating until an execution task
explicitly adds cleanup support.

---

## Forbidden Patterns

- Do not add file deletion, trash movement, or external cleanup command
  execution from scanner or plan code.
- Do not shell-compose commands. Command-backed cleanup must keep program and
  argv separate.
- Do not run permanent delete in the current implementation. It is not exposed
  as a CLI flag.

---

## Required Patterns

### Scenario: Five-mode CLI, machine output, and localization foundation

#### 1. Scope / Trigger

- Trigger: changing the breaking five-mode parser, bilingual human output,
  locale-neutral JSON/NDJSON, command-module registration, output sinks, or the
  staged handoff used by later mode tasks.

#### 2. Signatures

- Local command entrypoints:
  - `just ci`
  - `just build`
  - `just test`
- CLI roots:
  - bare `devsweep [--language <en|zh-CN>]` opens the TUI;
  - `clean {scan,plan,preview,execute,protect,rules}`;
  - `software {inventory,plan,preview,uninstall}`;
  - `optimize {list,plan,preview,run}`;
  - `analyze scan`, `status {snapshot,live}`, and read-only
    `history {list,show}`.
- Mutating signatures:
  - `clean execute --plan <FILE> --preview-digest <DIGEST> --confirm`;
  - `software uninstall --plan <FILE> --preview-digest <DIGEST> --confirm`;
  - `optimize run --plan <FILE> --preview-digest <DIGEST> --confirm`.
- Stream signature:
  - `status live [--interval <1..60>] [--process-limit <1..100>]
    [--format <human|ndjson>] [--output <FILE>]`.
- The exhaustive generated signatures live in `docs/reference/cli.md` and
  `docs/zh/reference/cli.md`; both exact manifests must be generated from the
  same built Clap tree.

#### 3. Contracts

- `just ci` is the canonical local quality gate and must include formatting,
  type-checking, tests, and clippy.
- Old `tui`, `scan`, `inventory`, `protect`, and `rules` roots are unknown
  commands, not aliases. Bare invocation requires stdin and stdout TTYs.
- `--language` is global but valid only for human output. CLI precedence is an
  explicit flag, then a supported Simplified-Chinese Windows user locale, then
  English. JSON, NDJSON, plans, codes, identities, and digests never localize.
- Human messages use embedded, build-validated `resources/i18n/{en,zh-CN}.json`
  catalogues. Placeholder, plural, accelerator, group, and truncation metadata
  must match exactly. User data is plain text; terminal controls and bidi
  formatting controls are escaped before interpolation.
- Named outputs use atomic create-new behavior. Existing files are never
  truncated or replaced; `-` is not a file sentinel. Human output does not
  auto-switch formats under redirection.
- JSON uses one V1 envelope with
  `{schema_version,command,outcome,data,warnings,error}`. Status NDJSON emits
  closed V1 lifecycle events with monotonic sequence and one internal terminal
  state. Broken pipe cancels and joins its producer and exits 0; other sink
  errors produce `producer_error` when possible and exit 6.
- Exit classes are 0 success, 2 usage/TTY/conflict, 3 stale authority, 4
  unavailable/unsupported/permission, 5 truthful partial, 6 failed or unknown
  after dispatch, and 130 canceled before dispatch.
- Mutating commands require a saved domain plan, the matching live preview
  digest (`sha256:` plus 64 lowercase hexadecimal characters), and `--confirm`.
  Parser and output infrastructure do not create execution authority.
- `application/commands/mod.rs` and `application/presentation/mod.rs` declare
  the complete compiler-checked module tree. Foundation creates the empty
  module skeletons; later mode tasks fill their assigned files without editing
  either root module.
- During staged delivery, a parsed but unregistered mode returns truthful
  `mode_unavailable`/exit 4. It must not fall through to a legacy handler or
  fabricate success.

#### 4. Validation & Error Matrix

- Bare invocation with either stream redirected -> `tty_required`, exit 2, no
  TUI start.
- `status live --format human` with redirected stdout -> exit 2 and guidance to
  use NDJSON; NDJSON is the explicit non-interactive stream contract.
- `--language` combined with JSON/NDJSON -> `invalid_cli`, exit 2, with a
  locale-neutral machine error envelope when the command selects one.
- Missing plan, digest, or `--confirm` -> usage exit 2 before dispatch. A
  well-formed but mismatched digest -> stale-authority exit 3.
- Existing named output -> `output_exists`, exit 6, and the existing bytes are
  unchanged.
- Unsupported catalogue key, placeholder drift, duplicate visible mnemonic,
  unknown language tag, wrong Status field/type/unit/terminal, or documentation
  manifest drift -> test/build failure.
- Registered grammar with no owned handler yet -> `mode_unavailable`, exit 4,
  no collector, executor, audit, trash, or process side effect.

#### 5. Good/Base/Bad Cases

- Good: English/Chinese help, generated manifests, and the actual parser all
  derive from the same built Clap tree; value-less flags remain value-less.
- Good: the CLI and future Tauri Status fixtures decode through one closed V1
  test contract and are byte-identical before runtime adapters are registered.
- Base: a fully shaped staged command returns a locale-appropriate human error
  or locale-neutral machine error with `mode_unavailable`/exit 4.
- Bad: a custom help renderer models `ArgAction::SetTrue` as
  `--confirm <CONFIRM>` or omits inherited `--language`.
- Bad: module ownership is represented only by string constants; downstream
  modules would then require edits to a supposedly frozen root `mod.rs`.
- Bad: a parser/output task calls a collector, constructs a cleanup plan, or
  dispatches an action merely to make an end-to-end command appear complete.

#### 6. Tests Required

- Exhaustive parser/help tests for every root, nested path, option, default,
  range, conflict, removed root, bare TTY case, and both languages.
- Exact generated-manifest tests against both reference documents. Tests must
  compare actual parser behavior too, not only metadata produced by the same
  renderer.
- Catalogue parity, plural, accelerator, metadata, locale, unit, injection,
  truncation, and closed presentation-language-tag tests.
- Closed Status snapshot/live fixture tests, including negative primitive,
  unit, availability, capability, truncation, sequence, and terminal cases plus
  byte-identical CLI/Tauri fixtures.
- Output tests for create-new, stdout/stderr separation, cancel/join,
  `producer_error`, and a real closed OS pipe.
- Native Windows Terminal evidence for TTY versus redirected behavior,
  English/Chinese UTF-8, long paths, pipeline close, create-new refusal, and
  final process cleanup.
- Source/diff review proving no collector, executor, desktop page, preference
  store, compatibility alias, or dependency entered the foundation task.

#### 7. Wrong vs Correct

Wrong:

```rust
// A string list does not register or compiler-check downstream modules.
const FROZEN_MODULES: &[&str] = &["clean", "status"];
```

Correct:

```rust
// The foundation owns stable registration; later tasks fill these files.
mod clean;
mod status;
```

### Scenario: Execution engine and audit log

#### 1. Scope / Trigger

- Trigger: `devsweep clean preview` or `devsweep clean execute` consumes an
  existing cleanup plan and either previews selected actions or executes
  command/trash-backed actions with an audit JSONL record for each attempted
  target.

#### 2. Signatures

- CLI entrypoint:
  - `devsweep clean preview --plan PATH [--format <human|json>] [--output FILE]`
  - `devsweep clean execute --plan PATH --preview-digest DIGEST --confirm
    [--format <human|json>] [--output FILE]`
- Internal execution boundary:
  - `Executor::default().run_plan(&ValidatedPlan, ExecutionRequest) -> anyhow::Result<ExecutionReport>`
  - `ExecutionRequest.selected: Vec<TargetId>` names the execution set
    explicitly; `ValidatedPlan::default_selected_ids()` provides the default
    (all `selected_by_default` targets, plan order).

#### 3. Contracts

- `clean preview` is always dry-run and must not call command or trash runners.
- `clean execute` requires `--plan`, the matching live `--preview-digest`, and
  `--confirm`; execution must never discover new targets.
- The CLI/TUI validates a v2 `UntrustedPlan` once before the executor sees it.
  The executor validates and deduplicates `request.selected` before opening an
  audit journal or dispatching a side effect. Unknown ids reject the whole
  request, duplicate ids produce one target outcome plus a normalization note,
  and selected targets still run in validated plan order. It does not read
  `selected_by_default` — that flag is a scan-time ranking hint, written only
  by the freshness guard, and callers translate it into an explicit selection
  via `default_selected_ids()`.
- `ExecutionReport` contains one terminal outcome for every deduplicated
  selected target, including dry-run and unprocessed targets after cancellation
  or fail-closed audit termination. `selected == attempted == outcomes.len()`
  and `succeeded + failed + skipped == attempted`; the retained `failures`
  compatibility field is derived only from failed outcomes.
- A dry run returns a `ConfirmationDigest` over the canonical validated
  manifest and sorted selected-id set. Execution requests that supply
  `expected_digest` fail with structured `ExecutionError::StaleConfirmation`
  before audit or side effects when it does not match. Current CLI/TUI callers
  may omit the compatibility field; GUI execution must supply it.
- Command actions use `CommandRequest { program, args, cwd }`; the private
  registry under `crates/devsweep-core/src/rules/`, not a plan file, reconstructs those values and
  they must never form a shell string.
- `MoveToTrash` actions use the rules-reconstructed path only after it
  matches the validated observed target path.
- `DeletePermanently` is disabled in this build and cannot be selected.
- Execution appends versioned JSONL audit records only to the fixed
  `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl` store under an exclusive lock.
  The removed `--audit-log` path and legacy `%APPDATA%\devsweep\audit.jsonl`
  file are never discovered, imported, converted, or modified.
- `ExecutionEvidence.estimated_bytes` is optional and additive
  (`serde(default, skip_serializing_if = "Option::is_none")`; the record keeps
  `deny_unknown_fields`). Only a `succeeded` `move_to_trash` transition sets it,
  from the verified or lower-bound target size. Old lines parse with `None`.
- `history::clean_moved_totals()` is read-only. It reads the fixed Clean V1
  store through the history reader and returns `CleanMovedTotalsV1` with
  `known_bytes`, `unknown_records`, and `lower_bound`. `lower_bound` is true
  when a succeeded trash move has no size or a partial size, or when the store
  reports a corrupt or unsupported line. Copy says "moved to Recycle Bin",
  never freed.
- Whole-list protection updates validate and normalize every requested path
  before replacing the persisted snapshot. Callers use
  `UserProtectionList::replace`; they must not emulate set semantics with a
  sequence of independently persisted `add` and `remove` calls.

#### 4. Validation & Error Matrix

- Preview with a valid plan -> returns selected target count and confirmation
  digest, with no side effects or audit append.
- Unknown selected target or selected inspect-only target -> structured request
  error before audit or cleanup side effects.
- Stale supplied confirmation digest -> structured request error before audit
  or cleanup side effects.
- Missing plan, preview digest, or `--confirm` -> error before executor runs.
- Fixed audit store cannot be resolved, opened, locked, or flushed -> error
  before any target action runs.
- Individual target failure -> record failed audit entry, continue remaining
  targets, return a report with failures.
- Failure to persist an authorization denial or safety skip -> report that
  target as failed and mark all remaining targets skipped without dispatch.
- Inspect-only target selected for cleanup -> error before audit or side effect.
- Target path contains the running `devsweep` executable -> skipped audit entry,
  no command/trash side effect. Use the shared path-safety helper rather than
  duplicating path prefix checks in callers.
- Permanent delete action -> error before audit or side effect.

#### 5. Good/Base/Bad Cases

- Good: command-backed Rust target records `["cargo", "clean",
"--manifest-path", "<Cargo.toml>"]`.
- Good: a selected target that contains `std::env::current_exe()` is skipped
  before invoking `CommandRunner` or `TrashRunner`.
- Good: a rules-resolved trash target moves exactly the validated path.
- Base: `devsweep clean preview --plan <empty-v2-plan>` reports zero selected
  targets and a stable digest without dispatching an action.
- Bad: executor reruns scanner logic to infer paths.
- Bad: executor uses `cmd /C`, `sh -c`, or string-form shell commands.
- Bad: a failed target aborts the job before later selected targets are audited.

#### 6. Tests Required

- Dry-run test proving command and trash runners are not called.
- Selection-boundary tests for atomic unknown-id rejection, duplicate
  deduplication notes, inspect-only rejection, and irreversible command
  projection.
- Confirmation tests proving stability, plan and selection sensitivity, and
  stale execute rejection before side effects.
- Report tests proving serde round-trip, capacity-confidence aggregation, and
  count/detail consistency across dry-run and early termination.
- Command-runner test asserting program and argv are separate.
- Self-clean guard test asserting the command runner is not called and the audit
  record status is `skipped`.
- Trash-runner test asserting the exact plan path is used.
- Audit JSONL test covering both success and failure records in one job.
- Permanent-delete test proving the action is rejected before runner or audit
  calls.
- Protection-list replacement test proving duplicate normalization and that an
  invalid requested path leaves the prior persisted snapshot unchanged.
- Scanner regression proving Rust target plans keep `--manifest-path`.

#### 7. Wrong vs Correct

Wrong:

```rust
std::process::Command::new("cmd")
    .args(["/C", &format!("cargo clean --manifest-path {}", manifest.display())])
    .status()?;
```

Correct:

```rust
CommandRequest {
    program: "cargo".to_string(),
    args: vec![
        "clean".to_string(),
        "--manifest-path".to_string(),
        manifest.display().to_string(),
    ],
    cwd: None,
}
```

### Scenario: Software execution and durable audit

#### 1. Scope / Trigger

- Trigger: `software preview` or `software uninstall` consumes a Software V1
  selection plan, reconstructs current-user MSIX authority, calls the WinRT
  removal API, or appends/replays the Software audit journal.

#### 2. Signatures

- CLI entrypoints:
  - `devsweep software preview --plan PATH [--format <human|json>] [--output FILE]`
  - `devsweep software uninstall --plan PATH --preview-digest DIGEST --confirm
    [--format <human|json>] [--output FILE]`
- Core boundary:
  `SoftwareExecutor::execute(SoftwareExecutionRequest { plan,
  expected_preview_digest, confirmed, cancel })`.

#### 3. Contracts

- `SoftwareExecutionRequest` cannot carry a serialized or caller-constructed
  inventory. The executor acquires current-user MSIX inventory through its
  private live provider behind process single-flight and audit locking, after
  reconciling pending dispatches.
- `ValidatedSoftwareAction` is opaque and non-deserializable. Only a matching
  live eligible current-user MSIX identity and preview digest can construct it;
  explicit confirmation is also required at the core boundary.
- MSI, registry-only, machine/other-user, protected, stale, malformed, and
  unsupported identities fail before the removal adapter. No vendor command,
  shell text, elevation fallback, or serialized argv participates.
- The fixed versioned Software journal durably syncs `dispatch_started` before
  `RemovePackageAsync`, holds an exclusive sidecar lock, enforces monotonic
  transitions and exactly one terminal, and contains tagged identity plus
  stable codes without localized or executable text.
- Startup recovery re-queries pending exact identities and never redispatches.
  A terminal must match accumulated adapter and requery evidence from the same
  operation.
- The MSIX adapter sends the exact `PackageFullName` through the joined MTA
  helper, monitors for at most 120 seconds, requests post-dispatch OS
  cancellation at most once, waits five seconds, then re-queries at 0, 2, and
  10 seconds.
- Post-dispatch terminals are exactly `removed`, `reboot_required`,
  `still_present`, `failed`, or `unknown_after_dispatch`. Software uninstall is
  irreversible from DevSweep's perspective and has no execution `partial`.

#### 4. Validation & Error Matrix

- Missing confirmation, stale digest, invalid identity, or audit-lock failure
  -> no adapter call.
- Cancellation before durable dispatch -> `canceled_before_start`, no adapter
  call. Cancellation, timeout, crash, or unfinished adapter after dispatch ->
  authoritative requery or `unknown_after_dispatch`, never canceled.
- Exact identity absent -> `removed`, except documented reboot evidence wins as
  `reboot_required`. Present plus definitive failure -> `failed`; present plus
  success -> `still_present`; unavailable/conflicting evidence ->
  `unknown_after_dispatch` unless reboot evidence has higher precedence.
- Unknown audit version, corrupt transition order, repeated terminal, or a
  terminal inconsistent with accumulated evidence -> fail closed without a new
  dispatch.

#### 5. Good/Base/Bad Cases

- Good: preview exposes the exact tagged current-user MSIX identity and opaque
  digest; uninstall independently reacquires live authority after recovery.
- Base: adapter success plus exact identity still present is
  `still_present`, not removal success.
- Bad: a public request injects a forged `SoftwareInventoryV1`, recovery calls
  the adapter again, or a terminal claims an installed state that no requery
  observed.

#### 6. Tests Required

- Compile/runtime authority tests for opaque actions, private live inventory,
  explicit confirmation, every refusal class, digest failure, and lock failure
  before adapter invocation.
- Exhaustive valid adapter/installed-state terminal matrix, pre/post-dispatch
  cancellation, timeout, crash/recovery, evidence conflict, and no-second-call
  recovery tests.
- Journal durability, redaction, unknown-version, transition-order, accumulated
  evidence, exclusive-lock, and exactly-one-terminal tests.
- CLI hostile-plan, locale-neutral machine output, irreversible copy, and
  single-document failed-exit tests.
- Native non-uninstall evidence must verify medium integrity, no UAC, stale
  authority, and journal persistence. Real removal remains `UNVERIFIED` when no
  user-confirmed disposable current-user MSIX fixture exists.

#### 7. Wrong vs Correct

Wrong:

```rust
SoftwareExecutionRequest {
    plan: &plan,
    live_inventory: &deserialized_inventory,
    expected_preview_digest: &digest,
    cancel: None,
}
```

Correct:

```rust
SoftwareExecutionRequest {
    plan: &plan,
    expected_preview_digest: &digest,
    confirmed: true,
    cancel: None,
}
```

### Scenario: Software update check, startup toggle, and leftover move

#### 1. Scope / Trigger

- Trigger: code changes `software/updates.rs`, `software/startup.rs`,
  `software/leftovers.rs`, the Software support audit records, or the desktop
  commands `software_updates_check`, `software_startup_list`,
  `software_startup_set`, `software_leftovers_preview`, and
  `software_leftovers_execute`.

#### 2. Signatures

- `check_software_updates(inventory, cancel) -> SoftwareUpdatesV1`
  (`state: available { rows } | unavailable { reason_code }`).
- `list_startup_entries() -> SoftwareStartupListV1` and
  `set_startup_enabled(entry_id, enabled, confirmed)`.
- `discover_software_leftovers`, `plan_software_leftovers`, and
  `execute_software_leftovers(SoftwareLeftoverExecutionRequest { plan,
  expected_preview_digest, confirmed, cancel })`.
- Audit: `SoftwareSupportAuditRecordV1` tagged by `record_kind`
  (`startup_toggled`, `leftover_moved`) in the same fixed Software journal.

#### 3. Contracts

- The update probe runs program `winget` with argv
  `["upgrade", "--disable-interactivity"]` through `ProcessRunner`, neutral cwd,
  and the 30-second provider deadline. Never pass
  `--accept-source-agreements` or `--accept-package-agreements`, and never run
  an upgrade.
- The parser reads column starts from display-width offsets of the header
  tokens above the dash separator (East Asian Wide/Fullwidth count as 2),
  requires exactly five columns, and never interprets header words. Only the
  known English and zh-CN no-update messages give an empty success. Any other
  mismatch, non-zero exit, timeout, truncation, or missing winget gives
  `unavailable` with a stable reason code.
- The only registry write in DevSweep is the current-user
  `Explorer\StartupApproved` value. The write uses 12 bytes (`0x02` or `0x03`,
  three zero bytes, then a zero or current FILETIME), re-reads the value, and
  appends `startup_toggled`. HKLM rows are view-only
  (`requires_administrator`). No Run value or Startup-folder file is deleted.
- Leftover candidates are the ARP `InstallLocation` (certain, selected by
  default) and exact-name directories under `%APPDATA%`, `%LOCALAPPDATA%`, and
  `%PROGRAMDATA%` (uncertain, unselected by default). Protected paths, system
  roots, shared vendor folders, and the running executable are never
  candidates.
- Leftover removal needs a terminal `removed` or `reboot_required` uninstall
  audit for the same identity and operation, a live plan digest, and explicit
  confirmation. Execution rediscovers live, rechecks each path, moves it to the
  Recycle Bin, and appends one `leftover_moved` record per item. Sizes are
  "moved to the Recycle Bin", never "freed".
- Support records carry no path, command, argv, or localized text.

#### 4. Validation & Error Matrix

- Unrecognized winget output -> `unavailable: unrecognized_output`, never
  `available` with zero rows.
- Unconfirmed toggle, unknown entry id, or machine entry -> no registry write.
- Missing succeeded uninstall, stale candidate, digest mismatch, or protection
  load failure -> no Recycle Bin move.

#### 5. Good/Base/Bad Cases

- Good: a disabled current-user Run entry is enabled by writing `0x02` plus
  eleven zero bytes, and the re-read state is `enabled`.
- Base: a truncated winget name keeps `name_truncated: true` and no inventory
  match.
- Bad: an update check that treats a changed table layout as "no updates".

#### 6. Tests Required

- Parser tests on the captured EN fixture and the zh-CN fixture, CJK width,
  truncation, the no-update message, non-zero exit, and unknown output.
- An argv test that proves no agreement flag and a `ProcessRunner` request.
- Startup decoding tests and one registry test under a temporary test key,
  never the real Run key.
- Leftover certainty, exclusion, size evidence, and removal precondition
  tests with a fake trash runner.

#### 7. Wrong vs Correct

Wrong:

```rust
let args = ["upgrade", "--accept-source-agreements", "--disable-interactivity"];
```

Correct:

```rust
const WINGET_UPGRADE_ARGS: [&str; 2] = ["upgrade", "--disable-interactivity"];
```

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

### Scenario: Declarative plan trust boundary

#### 1. Scope / Trigger

- Trigger: changing the JSON plan schema, loading a saved plan, resolving a
  cleanup rule, calculating a manifest identity, or passing a plan from the
  TUI into execution.

#### 2. Signatures

- Persisted input: `UntrustedPlan { version, targets: Vec<UntrustedTarget> }`.
- Trust conversion: `validate_plan(&UntrustedPlan) -> Result<ValidatedPlan>`.
- Scan serialization: `untrusted_plan_from_scan(&CleanupPlan) -> Result<UntrustedPlan>`.
- Execution boundary:
  `Executor::run_plan(&ValidatedPlan, ExecutionRequest) -> Result<ExecutionReport>`.
- Identity: `ActionFingerprint` and lowercase SHA-256 `ValidatedPlan::digest()`.

#### 3. Contracts

- Persisted plans are exact schema v2. v1 fails with
  guidance to re-run `devsweep clean scan` and `devsweep clean plan`; every
  other version fails before execution.
- `UntrustedPlan`/nested DTOs are closed-world serde types. They serialize
  facts, `rule_id`, and `CleanupIntent`, never `CleanAction`, `program`,
  `args`, `cwd`, or permanent-delete authority.
- `rules::resolve_action` is the only plan-validation action factory. Its
  private registry reconstructs trusted argv and trash paths from rule/intent
  IDs, then validates scope, path, ecosystem, kind, risk, and reversibility
  against the observed target.
- Canonical identity sorts normalized targets, normalizes lexical absolute
  paths, sorts evidence, includes the reconstructed trusted `CleanAction`, and
  hashes the domain-separated v2 representation. `ActionFingerprint` combines
  rules-owned action identity with a canonical path or a rules-owned logical
  provider footprint. Duplicate fingerprints fail validation; the executor
  additionally keeps a once ledger.
- CLI preview and execute consume the same validated plan. TUI confirmation
  displays the digest prefix, carries the full digest with its frozen snapshot,
  and revalidates both before creating an executor request.

#### 4. Validation & Error Matrix

- v1 or unsupported version -> error before runner, trash, audit, or worker
  dispatch.
- Unknown serde field, arbitrary command field, relative path, scope escape,
  unknown rule/action, or inconsistent risk/reversibility -> validation error.
- Selected inspect-only target -> error before audit or cleanup side effect.
- Equivalent Windows case/separator/trailing-slash paths, equivalent POSIX
  leading separators, or two provider targets sharing a logical command
  footprint -> duplicate fingerprint error.
- A child under project root `/` -> valid containment; a path merely sharing a
  non-root textual prefix -> scope-escape error.
- TUI digest differs after runtime revalidation -> failed job before executor.

#### 5. Good/Base/Bad Cases

- Good: a saved Rust target declares `cargo/clean_manifest`; the rules registry
  reconstructs `cargo clean --manifest-path <Cargo.toml>`.
- Good: a provider-supplied cache path is evidence only and cannot create a
  second command footprint or change its argv.
- Good: `py` and `python` are one logical pip-purge footprint for duplicate
  prevention, while their different trusted commands produce different
  confirmation digests when validated separately.
- Base: an empty v2 plan validates and dry-runs with zero selected targets.
- Bad: deserializing `cmd.exe`, PowerShell, `/bin/sh`, or extra argv from a
  plan file.
- Bad: accepting a v1 plan or deriving execution authority directly from a
  target path/action stored in JSON.

#### 6. Tests Required

- Serde tests reject executable and unknown nested fields; v1 tests assert the
  exact `clean scan` plus `clean plan` migration wording.
- Validation tests cover relative/scope-mismatched paths, unknown IDs,
  inspect-only selection, risk/action drift, and duplicate canonical
  fingerprints, including Windows spelling variants and POSIX leading
  separator variants.
- Digest tests prove input target ordering is insensitive and a logical target
  change or reconstructed provider command changes the digest.
- Executor test injects a malformed test-only validated plan and proves the
  once ledger blocks the second runner call.
- CLI/TUI tests prove preview/execute share validation and a digest mismatch
  reaches no executor action.

#### 7. Wrong vs Correct

Wrong:

```rust
let plan: CleanupPlan = serde_json::from_reader(file)?;
Executor::default().run_plan(&plan, request)?;
```

Correct:

```rust
let plan: UntrustedPlan = serde_json::from_reader(file)?;
let validated = validate_plan(&plan)?;
Executor::default().run_plan(&validated, request)?;
```

### Scenario: Global cache providers

#### 1. Scope / Trigger

- Trigger: `devsweep clean scan --scope global|all` discovers global
  package-manager cache providers and emits command-backed or inspect-only
  cleanup observations.

#### 2. Signatures

- CLI entrypoint:
  - `devsweep clean scan [--root PATH]... [--scope <projects|global|all>]
    [--format <human|json>] [--output FILE]`
- Internal provider boundary:
  - `GlobalProviderScanner::new().scan_with_cancel(None) -> CleanupPlan`
- Provider discovery boundary:
  - executable lookup by program name
  - command output probes for official inspect commands only

#### 3. Contracts

- `clean scan --scope global|all` may run read-only or provider-owned inspect
  commands such as
  `npm config get cache`, `pip cache dir`, `pnpm store path`, `yarn --version`,
  and Yarn cache-folder commands.
- `clean scan --scope global|all` must never execute cleanup commands. It
  produces typed in-memory scan facts; `--format json` serializes the
  observation without creating execution authority.
- npm, pip, pnpm, and Yarn cleanup argv are reconstructed by the trusted
  registry from provider/action IDs. JSON must not carry program, args, or cwd.
- A provider must not emit multiple user-facing cleanup targets for the same
  cache path just because the tool has alternative commands. Keep one cleanup
  target per physical cache footprint and record related official commands in
  evidence unless the model grows an explicit action-choice contract.
- Missing npm, pip, pnpm, or Yarn executables are non-fatal unavailable
  providers.
- Cargo home is inspect-only by default and uses
  `CleanAction::NoopInspectOnly`. Do not mark cargo `bin`,
  `credentials.toml`, `.crates.toml`, or the whole cargo home as a cleanable
  delete/trash target.
- Docker builder cache is not part of the MVP global provider set.
- The "never trash cache internals" rule here applies to command-backed
  providers (npm/pip/pnpm/yarn), which own official cleanup commands. Known
  cache directories with NO official command (gradle/maven/go/...) are handled
  by the declarative rule catalogue and MAY trash the whole directory or be
  inspect-only. See "Scenario: Declarative rule catalogue".

#### 4. Validation & Error Matrix

- Tool missing -> no target for that provider, scan still succeeds.
- Cache path command fails -> command-backed target may still be emitted with
  no `path`, as long as official command evidence is present.
- Yarn version unknown -> skip Yarn provider rather than guessing classic vs
  modern cleanup behavior.
- Cargo home missing or not a directory -> no Cargo home target.

#### 5. Good/Base/Bad Cases

- Good: npm target plans `["cache", "verify"]` or
  `["cache", "clean", "--force"]` without running either command.
- Good: npm cache discovery emits one target for the cache directory and may
  list `npm cache verify` as evidence for the same target.
- Good: pip target plans `["-m", "pip", "cache", "purge"]` and uses
  `pip cache dir` only for discovery evidence.
- Good: pnpm target plans `["store", "prune"]`.
- Good: Yarn provider branches on `yarn --version` before choosing classic or
  modern cache clean arguments.
- Good: Cargo home target is high risk, not selected by default, and
  inspect-only.
- Bad: deleting `_cacache`, pip `http-v2`, pnpm store internals, Yarn cache
  folders, or Cargo home directories directly.
- Bad: emitting both `npm.cache.verify:<path>` and `npm.cache.clean:<path>` as
  separate targets with the same estimated size.
- Bad: adding Docker builder cache to this MVP provider set.

#### 6. Tests Required

- Missing-tool provider test proving scan succeeds with no targets.
- Available-tool provider test asserting official program/argv pairs.
- Duplicate-path provider test proving alternative commands do not create
  duplicate user-facing targets for the same cache directory.
- Yarn version-branch test for classic and modern command arguments.
- Cargo home test proving the action is inspect-only and no cargo `bin` or
  credential path is emitted as cleanable.

#### 7. Wrong vs Correct

Wrong:

```rust
// Provider discovery must not directly delete opaque cache internals.
CleanAction::MoveToTrash {
    path: npm_cache.join("_cacache"),
}
```

Correct:

```rust
CleanAction::Command {
    program: npm_program,
    args: vec!["cache".to_string(), "clean".to_string(), "--force".to_string()],
    cwd: None,
    irreversible: true,
}
```

### Scenario: Declarative rule catalogue

#### 1. Scope / Trigger

- Trigger: adding or changing a cleanup rule, or listing rules via
  `devsweep clean rules list` / the TUI `Rules` surface. Every rule id, fact,
  table row, and
  procedural `RuleDoc` lives in `crates/devsweep-core/src/rules/definitions.rs`; scanner/provider
  implementations consume those definitions and `rule_catalogue()` aggregates
  them.

#### 2. Signatures

- `rules::PROJECT_DIR_RULES: &[ProjectDirRule]` - crate-private marker-gated
  project cache dirs.
- `rules::GLOBAL_CACHE_RULES` + `rules::GLOBAL_CACHE_RULES_OS` (`#[cfg]`-split) -
  known home-relative global caches with no official cleanup command.
- `rules::project_dir_rules(marker) -> impl Iterator<Item = &'static ProjectDirRule>`
- `rules::global_cache_rules() -> impl Iterator<Item = &'static GlobalCacheRule>`
- `rules::rule_catalogue() -> Vec<RuleDoc>` - flat display list of every rule,
  aggregated from the tables and procedural docs under the same rules owner.
- `rules::rule_row(&RuleDoc) -> String` + `rules::risk_label(&RiskLevel)` -
  the single row formatter shared by the CLI `clean rules` commands and the
  TUI Rules surface (both group by scope with section headings).

#### 3. Contracts

- Four rule shapes exist; two are data, two are procedural:
  - A. Project marker -> relative dir (Node/Python): tabled in
    `PROJECT_DIR_RULES`, consumed by `ProjectScanner::scan_*`.
  - C. Known home-relative global cache (gradle/maven/go/...): tabled in
    `GLOBAL_CACHE_RULES(_OS)`, consumed by `scan::global` target construction.
  - B. `cargo clean` project rule and D. command providers (npm/pip/pnpm/yarn):
    stay procedurally implemented (B needs a `target/` check; D must run a tool
    and parse stdout, yarn branches on version). Their `RuleDoc` facts still
    live in `rules::definitions`; implementations reference those facts rather
    than declaring a second copy.
- A procedural rule's id string appears exactly once in production code (its
  doc const); implementations reference `DOC.id` for target rule ids and
  `Evidence::RuleMatched` strings.
- Adding a rule of shape A or C means adding a table row, not a new branch.
- `rule_catalogue()` ids must be unique and every field non-empty.
- `GlobalCacheRule` carries a `KnownCacheAction`:
  - `Trash` for re-downloadable package caches with no official command
    (gradle/ivy/nuget) -> `CleanAction::MoveToTrash` (reversible).
  - `InspectOnly` for coarse, high-risk, or expensive-to-refetch caches (e.g.
    Maven local installs, Go modules, and HuggingFace models) ->
    `CleanAction::NoopInspectOnly`.
- Known-cache targets are NEVER `selected_by_default = true`, require a
  resolvable home dir, and carry `KnownCacheDir` + `RuleMatched` evidence — never
  a fabricated `OfficialCommand`.
- New known caches map to `Ecosystem::Generic` unless a dedicated ecosystem
  variant is added deliberately (that also touches `model/plan.rs` and TUI category
  counts).
- Ordinary rule additions must use an existing `CleanupIntent` and the shared
  private rules-registry contract. Any persisted plan, canonical digest, or
  version change requires the `plan/` owner and a compatibility review.

#### 4. Validation & Error Matrix

- Home dir unresolved -> `add_known_cache_targets` emits nothing, scan succeeds.
- Known-cache dir absent -> that rule emits no target, scan succeeds.
- Duplicate rule id in any table -> catalogue uniqueness test fails.
- Known cache without an official command -> grade `Trash`/`InspectOnly` by risk;
  never fabricate an `OfficialCommand`.

#### 5. Good/Base/Bad Cases

- Good: gradle/ivy/nuget caches emit `MoveToTrash`, Medium risk, not selected,
  with `KnownCacheDir` + `RuleMatched` evidence; Maven and Go coarse roots are
  inspect-only.
- Good: huggingface hub graded `InspectOnly` (High risk) -> `NoopInspectOnly`.
- Good: a new Node/Python cache added as one `PROJECT_DIR_RULES` row.
- Bad: adding a new scanner/provider branch instead of a table row.
- Bad: trashing `~/.cargo` internals or marking any known cache selected by
  default.
- Bad: giving a known cache an `OfficialCommand` evidence it does not have.

#### 6. Tests Required

- Catalogue id uniqueness and non-empty fields (`rules/mod.rs`).
- Catalogue aggregation completeness: every procedural doc and both tables
  from `rules/definitions.rs` appear in `rule_catalogue()`.
- Global-cache table uniqueness and coverage of new ecosystems (gradle/maven/go).
- Provider test: present known-cache dir -> trash target with correct
  risk/evidence and `!selected_by_default`.
- Provider test: an `InspectOnly` rule -> `NoopInspectOnly` (platform-agnostic:
  iterate `global_cache_rules()` to find one).
- Provider test: no home dir -> no known-cache targets.

#### 7. Wrong vs Correct

Wrong:

```rust
// New cache added as a scattered branch, trashed with a spoofed command.
if home.join(".gradle/caches").is_dir() {
    targets.push(command_target(/* pretend gradle has an official clean */));
}
```

Correct:

```rust
// One table row; the provider grades it Trash (no official command exists).
GlobalCacheRule {
    id: "gradle.caches",
    label: "gradle caches",
    ecosystem: Ecosystem::Generic,
    relative: ".gradle/caches",
    kind: TargetKind::PackageCache,
    risk: RiskLevel::Medium,
    action: KnownCacheAction::Trash,
}
```

### Scenario: CI and release archive

#### 1. Scope / Trigger

- Trigger: repository-level validation or release packaging changes add or
  change CI workflow commands, `justfile` recipes, or generated release
  artifacts.

#### 2. Signatures

- Local validation:
  - `just ci`
- Local release archive:
  - `just release-archive`
- CI validation:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace --locked --all-targets`
  - `cargo test --workspace --locked --all-targets`
  - `cargo clippy --workspace --locked --all-targets -- -D warnings`

#### 3. Contracts

- `just ci` remains the canonical local quality gate.
- Hosted `.github/workflows/ci.yml` runs on `pull_request` and
  `workflow_dispatch` only. It must not use an unbounded `on.push` that
  starts a full matrix on every push to `dev`.
- CI must run the same four validation classes as `just ci`: format, check,
  tests, and clippy.
- Process-runner tests in `devsweep-core` use the CLI-owned `process_fixture`
  binary. The shared test helper must build a missing fixture into the target
  root derived from the current test executable, including when Cargo uses a
  custom `--target-dir`; timeout measurements start only after fixture setup.
- Isolate validation build output with Cargo's `--target-dir` argument, not an
  exported `CARGO_TARGET_DIR`. The project scanner intentionally reads that
  environment variable as Cargo cleanup authority; exporting it into a test
  process changes the target discovered for Rust fixtures.
- `just release-archive` builds the release binary and writes a Windows archive
  to `dist/devsweep-x86_64-pc-windows-msvc.zip`.
- `dist/` is a generated artifact directory and must stay ignored by git.
- Release packaging must not imply package-manager distribution such as Scoop,
  Winget, Homebrew, or cargo publish.

#### 4. Validation & Error Matrix

- Format/check/test/clippy failure -> CI fails and local `just ci` fails.
- Clean or custom Cargo target without `process_fixture` -> the core test helper
  builds that exact target before running process assertions; it must not rely
  on a stale binary under the default `target/` directory.
- Exported `CARGO_TARGET_DIR` during scanner tests -> Rust fixtures correctly
  discover the override instead of their local `target/`; a harness that expects
  the local path is invalid, not evidence of a scanner regression.
- Release build failure -> `just release-archive` fails before creating or
  replacing the archive.
- Missing `target/release/devsweep.exe` after build -> archive command fails.
- Generated `dist/` output -> ignored by git status.

#### 5. Good/Base/Bad Cases

- Good: CI includes Windows and a non-Windows runner for the Rust validation
  matrix.
- Good: hosted CI starts from `pull_request` or `workflow_dispatch`, not from
  every push to `dev`.
- Good: a clean `cargo test --target-dir <isolated> --workspace --locked
  --all-targets` passes without a pre-existing fixture binary.
- Good: cross-platform validation passes `--target-dir /tmp/devsweep-check`
  directly to Cargo and leaves `CARGO_TARGET_DIR` unset for test processes.
- Good: release recipe uses the already built single binary.
- Base: local `cargo build --release` succeeds without packaging.
- Bad: committing generated `dist/*.zip` archives.
- Bad: locating `process_fixture` in the default target directory when the
  current test executable came from a custom target directory.
- Bad: starting a process timeout clock before an on-demand test fixture build.
- Bad: exporting `CARGO_TARGET_DIR=/tmp/devsweep-check` and then asserting that
  a scanner fixture resolves `<fixture>/target`.
- Bad: documenting Docker cleanup, permanent delete, or package-manager
  distribution as released MVP behavior.
- Bad: an unbounded `on: push:` in `.github/workflows/ci.yml` that reruns the
  full matrix on every `dev` commit.

#### 6. Tests Required

- Run `just ci` after workflow or validation command changes.
- Run the workspace tests once with a clean/custom target directory so cached
  `process_fixture` output cannot mask a broken cross-crate test dependency.
- When using a custom target for scanner tests, pass Cargo `--target-dir` and
  confirm `CARGO_TARGET_DIR` is not inherited by the test process unless the
  test explicitly covers Cargo's override behavior.
- Run the process-runner tests on Windows and one Unix host; the Unix run must
  exercise the process-group tree-termination test dynamically.
- Run `just release-archive` after release recipe changes on Windows.
- Check `git status --short --ignored` to confirm `dist/` is ignored.

#### 7. Wrong vs Correct

Wrong:

```text
Commit dist/devsweep-x86_64-pc-windows-msvc.zip as a release artifact.
```

Correct:

```text
Generate dist/devsweep-x86_64-pc-windows-msvc.zip locally and keep dist/
ignored by git.
```

Wrong:

```bash
export CARGO_TARGET_DIR=/tmp/devsweep-check
cargo test --workspace --locked --all-targets
```

Correct:

```bash
unset CARGO_TARGET_DIR
cargo test --workspace --locked --all-targets --target-dir /tmp/devsweep-check
```

### Scenario: Cleanup plan ranking and freshness guard

#### 1. Scope / Trigger

- Trigger: scanner, provider, CLI, or TUI code changes the order or default
  selection state of `CleanupPlan.targets`.

#### 2. Signatures

- Private ranking owner:
  - `scan::ranking::rank_cleanup_plan(&mut CleanupPlan)`
- Score helper:
  - `scan::ranking::freshness_tiebreaker(&CleanTarget, SystemTime) -> f64`
- Production report boundaries:
  - `scan::Sweeper::default().full_scan_report(...) -> anyhow::Result<ScanReport>`
  - `scan::Sweeper::default().full_scan_report_with_cancel(...) -> anyhow::Result<ScanReport>`
- Private merge/ranking owner:
  - `scan::Sweeper::full_scan_outcome(...) -> anyhow::Result<ScanPipelineOutcome>`
- Assembly boundaries:
  - `ProjectScanner::scan_roots_with_diagnostics_and_cancel(...)` returns
    unranked project observations.
  - `GlobalProviderScanner::scan_with_cancel(...)` returns an unranked global
    plan.
  - `devsweep clean scan [--root PATH]...
    [--scope <projects|global|all>] [--format <human|json>]`
  - TUI scan worker driving `scan::Sweeper::full_scan_report_with_cancel`

#### 3. Contracts

- Ranking ownership lives in private `scan::Sweeper::full_scan_outcome`: it
  merges project and global results and calls the ranking helper exactly once
  per cumulative set. Scanner and provider modules return unranked plans and
  must not call ranking themselves; report methods project the ranked result
  without re-ranking it.
- Plan targets are ordered by descending `estimated_bytes`; ties use the shared
  size/age score, older `last_modified`, then `TargetId` for deterministic
  output.
- The default freshness floor is seven days. A target modified within the floor
  can only move from `selected_by_default = true` to `false`.
- Missing `last_modified` leaves the existing default selection unchanged.
  Future mtimes are treated as age zero and therefore conservative.
- When the freshness guard deselects a target, add one
  `Evidence::RuleMatched { rule_id: "ranking.freshness_guard.7d" }`.
- Ranking must not add fields, enum variants, filesystem reads, process
  execution, trash moves, or delete operations.
- Manual TUI selection is not blocked by the freshness guard; the TUI passes
  explicitly selected target ids through `ExecutionRequest.selected` without
  rewriting `selected_by_default`.

#### 4. Validation & Error Matrix

- Multiple targets -> output is size-ranked with deterministic tie-breakers.
- Recently modified selected target -> deselected with freshness evidence.
- Stale selected target -> remains selected.
- Already unselected medium/high-risk target -> remains unselected.
- Missing mtime -> no panic and no more aggressive default selection.
- Future mtime -> no panic and conservative deselection if the target was
  selected by default.

#### 5. Good/Base/Bad Cases

- Good: CLI and TUI both obtain reports through `scan::Sweeper` report methods,
  so ranking runs in one private pipeline owner instead of each entry point
  implementing local ordering.
- Good: a 2 GiB global target appears before a 1 GiB project target in both
  `clean scan --format json` and TUI results.
- Base: calling the ranking helper twice is idempotent and does not duplicate
  freshness evidence.
- Bad: sorting only in render code while `clean scan --format json` uses
  discovery order.
- Bad: selecting a large target by default only because it is large.
- Bad: scanning the filesystem again from ranking to calculate missing data.
- Bad: a report or progress consumer defensively re-ranking the projected plan.

#### 6. Tests Required

- Unit tests for size ranking, tie-breakers, score calculation, missing mtime,
  future mtime, and freshness evidence idempotence.
- Sweep tests with fake scanners proving merge order, single ranking pass with
  freshness guard, staged progress event sequence, and cumulative ranked
  partial plans.
- Continuous progress snapshots must be cumulative, deduplicated, and ordered by
  `Sweeper`, not merged by TUI or desktop consumers. Ordinary target sizing must
  observe cooperative cancellation as well as traversal and provider probes.
  Provider helpers that can construct more than one target must publish and
  re-check cancellation after each completed target rather than batching the
  helper's entire loop behind one callback.
- Webview progress uses the core-owned `ScanPreviewSnapshot` projection. It is an
  observation DTO and must omit `CleanAction`, cleanup intent,
  `selected_by_default`, and the versioned plan envelope.
- TUI state tests proving staged scan partials apply as cumulative ranked
  snapshots with stale-scan protection.
- TUI test proving explicit manual selection can still execute a fresh target.

#### 7. Wrong vs Correct

Wrong:

```rust
// UI-only sorting makes JSON output and TUI output disagree.
targets.sort_by_key(|target| std::cmp::Reverse(target.estimated_bytes));
```

Correct:

```rust
let report = Sweeper::default().full_scan_report(&options, &mut |_| {})?;
// report.plan is projected from the merged, ranked result; do not re-rank.
```

### Scenario: Project scanner and JSON cleanup report

#### 1. Scope / Trigger

- Trigger: scanner code discovers project cleanup candidates or changes the
  `devsweep clean scan --format json` report projection from an empty
  placeholder to real `CleanTarget` values and scan-health observations.

#### 2. Signatures

- Internal project-discovery boundary:
  - `ProjectScanner::scan_roots_with_diagnostics_and_cancel(...) ->
    anyhow::Result<ScanOutcome>`
- CLI entrypoint:
  - `devsweep clean scan [--root PATH]... --scope projects
    [--rescan-target TARGET_ID] [--format <human|json>] [--output FILE]`

#### 3. Contracts

- Scanner code may only create `CleanTarget` values; it must not delete, move to
  trash, or execute cleanup commands.
- Project matching is marker-first:
  - Rust `target` requires `Cargo.toml` evidence.
  - Node cleanup targets require `package.json` or lockfile evidence.
  - Python cleanup targets require project markers or local virtualenv evidence.
- `.gitignore` filtering must not hide cleanup candidates. The current scanner
  uses `std::fs` traversal and therefore does not apply ignore files.
- Symlinks, Windows junctions, and reparse points are not followed by default;
  Windows decisions use a no-follow path-level tag probe and fail closed when
  the probe cannot verify safety.
- JSON reports embed plan targets with `rule_id`, `risk`, `evidence`,
  `selected_by_default`, `intent`, `estimated_bytes`, and size-completeness
  observations; they must not expose an executable `action`.
- Normalize scan roots before traversal and reduce them to the smallest covering
  set. Parent/child cleanup paths are deduplicated by canonical footprint plus
  action identity so one scan does not double-count or emit duplicate actions.
  Exact duplicates merge unique evidence; nested paths collapse only when their
  semantic action is the same, while the same footprint with distinct actions
  remains visible. This is scan-output quality only: validated-plan/executor
  invariants own the once-only guarantee for externally supplied plans.

#### 4. Validation & Error Matrix

- Missing scan root -> return an error to the CLI.
- Markerless `target`, `build`, `dist`, or `node_modules` -> no target emitted.
- Inaccessible nested entry -> skip that entry, continue scanning the rest of
  the root, and mark report health partial.
- Reparse root, target, or marker -> skip it without descending or counting
  descendant capacity.
- Equivalent duplicate roots, including Windows case/separator variants -> scan
  once after canonicalization.
- Exact footprint with the same action -> keep one target and merge evidence.
- Nested cleanup target under a parent with the same action -> keep the parent
  target and drop the nested target.
- Same footprint with distinct actions -> keep both targets.

#### 5. Good/Base/Bad Cases

- Good: fixture with `Cargo.toml` plus `target/` emits a Rust target with
  command-shaped `cargo clean` action data but does not run Cargo.
- Good: fixture with `package.json` plus `node_modules/` emits a medium-risk,
  not-selected-by-default dependency-directory target.
- Good: fixture with `pyproject.toml` plus `__pycache__/` emits a low-risk test
  cache target.
- Good: repeated or parent/child scan roots produce one target per
  footprint/action pair.
- Good: two rules with the same footprint but distinct command/trash actions
  remain independently visible.
- Base: `devsweep clean scan --root . --scope projects --format json` emits a
  valid observation report that must pass through `clean plan --observation`
  before any saved plan can be previewed or executed.
- Bad: matching a markerless directory because its name is `target`, `build`,
  or `dist`.
- Bad: scanner code importing `std::process::Command`, `trash`, or file removal
  APIs.

#### 6. Tests Required

- Fixture tests for Rust, Node, and Python discovery.
- Fixture tests proving markerless cleanup names are ignored.
- Tests that serialized report targets contain rule, risk, evidence, selection,
  intent, and size fields, omit executable action fields, and preserve partial
  health diagnostics.
- Dedupe tests for duplicate roots, exact evidence merging, nested paths with
  the same action, and equal paths with distinct actions.
- On Windows, fixture coverage for case and separator-equivalent roots.
- Non-mutating test that scanned fixture files still exist after scan.

#### 7. Wrong vs Correct

Wrong:

```rust
// Name-only matching is unsafe.
if path.file_name() == Some("target".as_ref()) {
    targets.push(clean_target_for(path));
}
```

Correct:

```rust
// Marker-first: only a Rust project root can own target/.
let manifest = project_root.join("Cargo.toml");
if manifest.is_file() && project_root.join("target").is_dir() {
    targets.push(clean_target_with_marker(manifest));
}
```

### Scenario: Scan safety, review reports, and capacity inventory

#### 1. Scope / Trigger

- Trigger: changing scan diagnostics, size estimates, reparse-point checks,
  scan JSON, reviewed target rescan, or read-only capacity inventory.

#### 2. Signatures

- Report boundary: `ScanReport { version, plan, health }` and
  `ScanHealth { completeness, diagnostics, totals }`.
- CLI boundaries:
  - `devsweep clean scan [--root PATH]... [--scope <projects|global|all>]
    --format json [--rescan-target TARGET_ID]`
  - `devsweep analyze scan --root PATH [--format <human|json>] [--output FILE]`
  - `devsweep clean plan --observation FILE --select TARGET_ID... --output FILE`
  - `devsweep clean preview --plan FILE [--format <human|json>] [--output FILE]`
  - `devsweep clean execute --plan FILE --preview-digest DIGEST --confirm
    [--format <human|json>] [--output FILE]`
- TUI inventory boundary:
  `inventory_root_with_cancel(&Path, Option<&Arc<FlagCancelObserver>>) -> Result<InventoryReport>`.
- Review boundary:
  `rescan_target_size(&mut CleanupPlan, &TargetId, cancel) -> Result<()>`.

#### 3. Contracts

- Scan JSON is an observation document and cannot cross directly into
  `validate_plan`. `clean plan --observation` is the sole CLI conversion
  boundary: it applies an explicit selection and writes a saved v2 plan before
  preview. Diagnostics, totals, and sizing warnings remain observations and
  cannot reconstruct actions or affect a validated digest.
- Size totals keep verified bytes, partial lower bounds, and unknown-target
  counts distinct. Incomplete size observations are not default-selected.
- Traversal, marker validation, sizing, and execution-time ancestor checks use
  the same no-follow reparse probe. A reparse or an unverifiable probe never
  grants traversal or cleanup authority.
- Neutral `cargo_metadata.rs` owns Cargo scope/probe types, process-result
  classification, and JSON parsing. Scanner owns only scan-lifetime caching;
  safety owns only live execution revalidation policy, and neither imports the
  other for Cargo metadata behavior.
- `--rescan-target` accepts only an exact target ID discovered by the current
  scan. It replaces that target's size observation under a larger but bounded
  walk, reusing cancellation and reparse checks.
- Capacity inventory returns read-only `InventoryReport` observations and optional
  inspect-only pnpm evidence. It has no cleanup intent/action and the plan
  decoder must reject it explicitly.
- A canceled inventory walk returns a partial read-only report with a typed
  canceled diagnostic; the TUI may discard that unfinished snapshot, but it
  must not turn cancellation into an executable action or resume traversal.

#### 4. Validation & Error Matrix

- Reparse point at root, child, marker, target, or execution ancestor -> skip
  or deny, with no descendant candidate/capacity contribution.
- Reparse probe failure -> partial health or execution denial; never fall back
  to metadata-only approval.
- Truncated Cargo metadata -> `output_truncated` diagnostic before JSON parse.
- Unknown review target ID or pathless target -> rescan fails without walking
  an arbitrary caller path.
- Capacity `InventoryReport` supplied to `clean plan --observation` -> reject
  before validation, audit, or cleanup side effect.
- Inventory cancellation before or during traversal -> no later top-level
  observation or pnpm-reference probe runs after the cancellation check.

#### 5. Good/Base/Bad Cases

- Good: a partial size appears as a lower bound with typed warnings, while a
  successful reviewed rescan replaces those warnings with a complete estimate.
- Good: `clean plan --observation` accepts a supported Clean scan report and
  emits a saved plan; `clean preview --plan` accepts that plan without granting
  execution authority.
- Good: a canceled TUI inventory worker receives a partial report containing a
  `canceled` diagnostic and remains outside the cleanup-plan flow.
- Base: a complete report with no diagnostics has zero partial/unknown totals.
- Bad: serializing diagnostic process output as executable argv or using an
  inventory observation as a cleanup target.
- Bad: accepting a raw path for a high-budget rescan.

#### 6. Tests Required

- Probe-seam tests cover Cloud Files-style root, child, marker, target, and
  execution-ancestor tags plus an unverifiable probe.
- Report tests assert typed truncation process metadata, total separation, and
  that warnings do not change validation or digest authority.
- Rescan tests assert exact-ID replacement, cancellation, and reparse denial.
- CLI tests assert report decoding, capacity-inventory rejection by
  `clean plan --observation`, and valid `--rescan-target` parsing.
- Inventory tests assert observations are read-only and an orphan-pnpm finding
  requires complete non-candidate reference evidence; cancellation tests assert
  a typed partial report without observations.

#### 7. Wrong vs Correct

Wrong:

```rust
// An arbitrary caller path could make the high-budget walker inspect anything.
estimate_tree_with_budget(Path::new(user_path), REVIEW_SIZE_ENTRY_BUDGET);
```

Correct:

```rust
// Locate the reviewed object in the current scan before replacing its estimate.
rescan_target_size(&mut plan, &target_id, None)?;
```

---

## Testing Requirements

- Run `just ci` before reporting implementation complete.
- For command-surface changes, run the affected CLI help or command manually
  and record the result in the task summary.
- For JSON contract changes, include serialization or round-trip tests.

---

## Code Review Checklist

- Does the change keep scanner/plan/foundation code non-mutating?
- Does `just ci` pass?
- Are command signatures and JSON fields covered by tests?
- Is any new execution behavior owned by the correct child task?
