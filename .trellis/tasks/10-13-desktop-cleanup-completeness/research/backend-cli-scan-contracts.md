# Curated backend contract excerpts: backend-cli-scan-contracts

Source: [canonical backend quality guidelines](../../../spec/backend/quality-guidelines.md), captured 2026-10-13. Source SHA-256: `a3b16d9ad96671d448b5a8f7903af77168394200cbdf947bbbc159de74059bcc`.

These verbatim relevant sections avoid the context injector's 32768-byte per-file truncation. This task-local snapshot is not a new specification. Before implementation, read the current canonical sections and resolve changes against the child design; do not rewrite the canonical source from this snapshot. Prospective task requirements may intentionally amend current contracts through a targeted spec update. Other scenarios remain in the canonical source.

<!-- Canonical source lines 28-164 -->

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

<!-- Canonical source lines 1172-1514 -->

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
