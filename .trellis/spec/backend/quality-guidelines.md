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

### Scenario: Foundation CLI and command entrypoints

#### 1. Scope / Trigger

- Trigger: the foundation task defines the project command surface and the
  local validation entrypoint used by later Trellis tasks.

#### 2. Signatures

- Local command entrypoints:
  - `just ci`
  - `just build`
  - `just dev`
  - `just test`
- CLI entrypoints:
  - `devsweep tui`
  - `devsweep scan [ROOT]... [--json] [--global] [--projects] [--rescan-target TARGET_ID]`
  - `devsweep inventory [ROOT] [--json]`
  - `devsweep clean [--plan PATH] [--execute] [--audit-log PATH]`
  - `devsweep rules`

#### 3. Contracts

- `just ci` is the canonical local quality gate and must include formatting,
  type-checking, tests, and clippy.
- `devsweep scan --json` emits a versioned `ScanReport` with an embedded
  declarative v2 `UntrustedPlan` and non-authoritative health observations. A
  saved target carries typed intent and observed facts, never executable argv,
  cwd, or an authoritative action path. An empty embedded plan is:
  ```json
  {
    "version": 2,
    "targets": []
  }
  ```
- `devsweep clean` defaults to dry-run behavior.
- `devsweep clean --execute` is owned by `src/executor.rs` after the execution
  engine task.

#### 4. Validation & Error Matrix

- `scan --json` succeeds -> valid JSON report with `version`, `plan`, and
  `health`; `plan` remains a valid v2 cleanup plan.
- `clean --plan PATH` without `--execute` succeeds -> reports dry-run and
  performs no action.
- `clean --execute` without `--plan PATH` -> returns an error before execution
  starts.
- Missing future scanner/provider implementations -> return placeholder output,
  not side effects.

#### 5. Good/Base/Bad Cases

- Good: `just ci` passes before a task is reported complete.
- Base: `cargo check --all-targets` passes when run directly.
- Bad: `scan` discovers or deletes directories before the scanner task exists.
- Bad: `clean --execute` invokes `std::process::Command` outside
  `src/executor.rs`.

#### 6. Tests Required

- CLI definition test for subcommand shape.
- Scan-report JSON serialization/round-trip tests for representative targets.
- TUI placeholder render test through ratatui `TestBackend`.
- Source scan or review confirming no deletion/trash/process execution code is
  present in foundation.

#### 7. Wrong vs Correct

Wrong:

```rust
// Scanner/foundation code must not execute cleanup commands.
std::process::Command::new("cargo").arg("clean").status()?;
```

Correct:

```rust
// Saved plans declare an intent; the private rules registry owns the argv template.
CleanupIntent::RunBuiltInAction {
    provider_id: "cargo".to_string(),
    action_id: "clean_manifest".to_string(),
}
```

### Scenario: Execution engine and audit log

#### 1. Scope / Trigger

- Trigger: `devsweep clean` consumes an existing cleanup plan and either
  dry-runs selected actions or executes command/trash-backed actions with an
  audit JSONL record for each attempted target.

#### 2. Signatures

- CLI entrypoint:
  - `devsweep clean [--plan PATH] [--execute] [--audit-log PATH]`
- Library entrypoint:
  - `Executor::default().run_plan(&ValidatedPlan, ExecutionRequest) -> anyhow::Result<ExecutionReport>`
  - `ExecutionRequest.selected: Vec<TargetId>` names the execution set
    explicitly; `ValidatedPlan::default_selected_ids()` provides the default
    (all `selected_by_default` targets, plan order).

#### 3. Contracts

- `clean` without `--execute` is always dry-run and must not call command or
  trash runners.
- `clean --execute` requires `--plan PATH`; execution must never discover new
  targets.
- The CLI/TUI validates a v2 `UntrustedPlan` once before the executor sees it.
  The executor runs exactly the intersection of `request.selected` with the
  validated plan's targets, in plan order; unknown ids are ignored. It does not read
  `selected_by_default` — that flag is a scan-time ranking hint, written only
  by the freshness guard, and callers translate it into an explicit selection
  via `default_selected_ids()`.
- Command actions use `CommandRequest { program, args, cwd }`; the private
  registry under `src/rules/`, not a plan file, reconstructs those values and
  they must never form a shell string.
- `MoveToTrash` actions use the rules-reconstructed path only after it
  matches the validated observed target path.
- `DeletePermanently` is disabled in this build and cannot be selected.
- Execution appends JSONL audit records to `--audit-log PATH` or
  `devsweep-audit.jsonl`.

#### 4. Validation & Error Matrix

- Dry-run with or without plan -> returns selected target count, no side
  effects, no audit file.
- `--execute` without `--plan` -> error before executor runs.
- Audit file cannot be opened -> error before any target action runs.
- Individual target failure -> record failed audit entry, continue remaining
  targets, return a report with failures.
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
- Base: `devsweep clean` reports a dry-run with zero selected targets when no
  plan is provided.
- Bad: executor reruns scanner logic to infer paths.
- Bad: executor uses `cmd /C`, `sh -c`, or string-form shell commands.
- Bad: a failed target aborts the job before later selected targets are audited.

#### 6. Tests Required

- Dry-run test proving command and trash runners are not called.
- Command-runner test asserting program and argv are separate.
- Self-clean guard test asserting the command runner is not called and the audit
  record status is `skipped`.
- Trash-runner test asserting the exact plan path is used.
- Audit JSONL test covering both success and failure records in one job.
- Permanent-delete test proving the action is rejected before runner or audit
  calls.
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
  `plan format v1 is no longer accepted; re-run \`devsweep scan --json\``;
  every other version fails before execution.
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
- CLI dry-run and execute consume the same validated plan. TUI confirmation
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
  exact rescan wording.
- Validation tests cover relative/scope-mismatched paths, unknown IDs,
  inspect-only selection, risk/action drift, and duplicate canonical
  fingerprints, including Windows spelling variants and POSIX leading
  separator variants.
- Digest tests prove input target ordering is insensitive and a logical target
  change or reconstructed provider command changes the digest.
- Executor test injects a malformed test-only validated plan and proves the
  once ledger blocks the second runner call.
- CLI/TUI tests prove dry-run/execute share validation and a digest mismatch
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

- Trigger: `devsweep scan --global` discovers global package-manager cache
  providers and emits command-backed or inspect-only cleanup plan targets.

#### 2. Signatures

- CLI entrypoint:
  - `devsweep scan [ROOT]... [--json] [--global] [--projects]`
- Library entrypoint:
  - `GlobalProviderScanner::new().scan() -> CleanupPlan`
- Provider discovery boundary:
  - executable lookup by program name
  - command output probes for official inspect commands only

#### 3. Contracts

- `scan --global` may run read-only or provider-owned inspect commands such as
  `npm config get cache`, `pip cache dir`, `pnpm store path`, `yarn --version`,
  and Yarn cache-folder commands.
- `scan --global` must never execute cleanup commands. It produces typed
  in-memory scan facts; `scan --json` converts those facts to declarative
  `RunBuiltInAction` intents.
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
  `devsweep rules` / the TUI `Rules` tab. Every rule id, fact, table row, and
  procedural `RuleDoc` lives in `src/rules/definitions.rs`; scanner/provider
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
  the single row formatter shared by the CLI `rules` command and the TUI
  Rules tab (both group by scope with section headings).

#### 3. Contracts

- Four rule shapes exist; two are data, two are procedural:
  - A. Project marker -> relative dir (Node/Python): tabled in
    `PROJECT_DIR_RULES`, consumed by `ProjectScanner::scan_*`.
  - C. Known home-relative global cache (gradle/maven/go/...): tabled in
    `GLOBAL_CACHE_RULES(_OS)`, consumed by `providers::add_known_cache_targets`.
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
  - `cargo check --all-targets`
  - `cargo test --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

#### 3. Contracts

- `just ci` remains the canonical local quality gate.
- CI must run the same four validation classes as `just ci`: format, check,
  tests, and clippy.
- `just release-archive` builds the release binary and writes a Windows archive
  to `dist/devsweep-x86_64-pc-windows-msvc.zip`.
- `dist/` is a generated artifact directory and must stay ignored by git.
- Release packaging must not imply package-manager distribution such as Scoop,
  Winget, Homebrew, or cargo publish.

#### 4. Validation & Error Matrix

- Format/check/test/clippy failure -> CI fails and local `just ci` fails.
- Release build failure -> `just release-archive` fails before creating or
  replacing the archive.
- Missing `target/release/devsweep.exe` after build -> archive command fails.
- Generated `dist/` output -> ignored by git status.

#### 5. Good/Base/Bad Cases

- Good: CI includes Windows and a non-Windows runner for the Rust validation
  matrix.
- Good: release recipe uses the already built single binary.
- Base: local `cargo build --release` succeeds without packaging.
- Bad: committing generated `dist/*.zip` archives.
- Bad: documenting Docker cleanup, permanent delete, or package-manager
  distribution as released MVP behavior.

#### 6. Tests Required

- Run `just ci` after workflow or validation command changes.
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

### Scenario: Cleanup plan ranking and freshness guard

#### 1. Scope / Trigger

- Trigger: scanner, provider, CLI, or TUI code changes the order or default
  selection state of `CleanupPlan.targets`.

#### 2. Signatures

- Library entrypoint:
  - `ranking::rank_cleanup_plan(&mut CleanupPlan)`
- Score helper:
  - `ranking::target_score(&CleanTarget, SystemTime) -> f64`
- Pipeline owner:
  - `sweep::Sweeper::default().full_scan(&ScanOptions, &mut dyn FnMut(ScanProgress)) -> anyhow::Result<CleanupPlan>`
- Assembly boundaries:
  - `ProjectScanner::new().scan_roots(&[PathBuf]) -> anyhow::Result<CleanupPlan>` (returns unranked)
  - `GlobalProviderScanner::new().scan() -> CleanupPlan` (returns unranked)
  - `devsweep scan [ROOT]... [--json] [--global] [--projects]`
  - TUI scan worker driving `sweep::full_scan`

#### 3. Contracts

- Ranking ownership lives in `sweep::full_scan`: it merges scanner and provider
  results and calls the ranking helper exactly once per cumulative set. Scanner
  and provider modules return unranked plans and must not call ranking
  themselves; callers of `full_scan` receive a ranked plan and must not
  re-rank.
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

- Good: CLI and TUI both obtain plans through `sweep::full_scan`, so ranking
  runs in exactly one production call site instead of each entry point
  implementing local ordering.
- Good: a 2 GiB global target appears before a 1 GiB project target in both
  `scan --json` and TUI results.
- Base: calling the ranking helper twice is idempotent and does not duplicate
  freshness evidence.
- Bad: sorting only in render code while `scan --json` uses discovery order.
- Bad: selecting a large target by default only because it is large.
- Bad: scanning the filesystem again from ranking to calculate missing data.
- Bad: a caller of `full_scan` defensively re-ranking the returned plan.

#### 6. Tests Required

- Unit tests for size ranking, tie-breakers, score calculation, missing mtime,
  future mtime, and freshness evidence idempotence.
- Sweep tests with fake scanners proving merge order, single ranking pass with
  freshness guard, staged progress event sequence, and cumulative ranked
  partial plans.
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
let plan = Sweeper::default().full_scan(&options, &mut |_| {})?;
// plan is merged and ranked by contract; do not re-rank.
```

### Scenario: Project scanner and JSON cleanup report

#### 1. Scope / Trigger

- Trigger: scanner code discovers project cleanup candidates or changes the
  `devsweep scan --json` report projection from an empty placeholder to real
  `CleanTarget` values and scan-health observations.

#### 2. Signatures

- Library entrypoint:
  - `ProjectScanner::new().scan_roots(&[PathBuf]) -> anyhow::Result<CleanupPlan>`
- CLI entrypoint:
  - `devsweep scan [ROOT]... [--json] [--projects] [--rescan-target TARGET_ID]`

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
- Base: `devsweep scan . --json` emits a valid report whose embedded plan
  validates through the normal cleanup-plan boundary.
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
  - `devsweep scan [ROOT]... --json [--rescan-target TARGET_ID]`
  - `devsweep inventory [ROOT] [--json]`
  - `devsweep clean --plan PATH [--execute]`
- TUI inventory boundary:
  `inventory_root_with_cancel(&Path, Option<&Arc<FlagCancelObserver>>) -> Result<InventoryReport>`.
- Review boundary:
  `rescan_target_size(&mut CleanupPlan, &TargetId, cancel) -> Result<()>`.

#### 3. Contracts

- Scan JSON is a report document. Only its embedded v2 `UntrustedPlan` can
  cross into `validate_plan`; diagnostics, totals, and sizing warnings are
  observations and cannot reconstruct actions or affect a validated digest.
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
- `inventory` returns read-only `InventoryReport` observations and optional
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
- Inventory report supplied to `clean --plan` -> reject before validation,
  audit, or cleanup side effect.
- Inventory cancellation before or during traversal -> no later top-level
  observation or pnpm-reference probe runs after the cancellation check.

#### 5. Good/Base/Bad Cases

- Good: a partial size appears as a lower bound with typed warnings, while a
  successful reviewed rescan replaces those warnings with a complete estimate.
- Good: `clean --plan` accepts either a plan-only v2 document or the
  embedded plan from a supported scan report.
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
- CLI tests assert report decoding, inventory rejection by `clean --plan`, and
  valid `--rescan-target` parsing.
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
