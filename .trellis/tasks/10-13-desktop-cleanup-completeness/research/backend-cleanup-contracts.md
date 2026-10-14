# Curated backend contract excerpts: backend-cleanup-contracts

Source: [canonical backend quality guidelines](../../../spec/backend/quality-guidelines.md), captured 2026-10-13. Source SHA-256: `a3b16d9ad96671d448b5a8f7903af77168394200cbdf947bbbc159de74059bcc`.

These verbatim relevant sections avoid the context injector's 32768-byte per-file truncation. This task-local snapshot is not a new specification. Before implementation, read the current canonical sections and resolve changes against the child design; do not rewrite the canonical source from this snapshot. Prospective task requirements may intentionally amend current contracts through a targeted spec update. Other scenarios remain in the canonical source.

<!-- Canonical source lines 1-27 -->

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

<!-- Canonical source lines 165-312 -->

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

<!-- Canonical source lines 728-1050 -->

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
