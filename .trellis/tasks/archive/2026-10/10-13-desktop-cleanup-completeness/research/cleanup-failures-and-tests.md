# Research: Cleanup failures, safety boundaries, and proposed regression tests

Companion to [current-cleanup-gaps.md](current-cleanup-gaps.md); same 2026-10-13 source investigation and evidence classification. Split for complete context injection; findings have not been remeasured. The child PRD/design is authoritative where later planning review refined the proposal.

### 5. Confirmed defects and missing capabilities

#### F1 — Hidden current-directory desktop scope (P0)

Evidence: desktop defaults/controls and forwarding in section 3. No discovery of user profiles, named project libraries, other volumes, or recent workspaces is performed by this Clean entry point. `ScanOptions` contains only two booleans and roots (`scan/mod.rs:159`). Empty root lists are accepted by the project scan loop and can yield complete/empty output (`scan/project/mod.rs:111`, `:153`). Missing/unreadable roots fail the whole project phase before global scanning (`scan/project/mod.rs:518`, `scan/mod.rs:335`).

Acceptance: show and allow editing effective roots in desktop; persist or explicitly reuse chosen roots using a narrowly scoped settings contract; never use process CWD as an invisible GUI default. CLI keeps explicit repeated roots and reports their resolved scope. Test startup from an unrelated directory, multiple volumes, duplicate/overlapping roots, Unicode/spaces, absent/inaccessible roots, and global-only scan. Define whether a failed root makes only that root partial instead of aborting otherwise valid global work.

#### F2 — Go command discovery cannot produce a final report (P0)

Evidence chain:

1. `scan/global/mod.rs:271` constructs a `go.mod_cache.clean` command target when Go and a usable module cache are present.
2. Final report conversion calls `untrusted_plan_from_scan` (`scan/mod.rs:286`; CLI counterpart `:268`).
3. Conversion collects all target results with `?` (`plan/mod.rs:266`) and sends every command to `intent_from_scan_action` (`plan/mod.rs:310`).
4. The conversion only knows Cargo/npm/pip/pnpm/Yarn; otherwise it errors (`rules/registry.rs:54`). Trusted built-in reconstruction also has no Go case (`registry.rs:229`), and the authorized-command docs list omits Go (`rules/definitions.rs:128`).

Result: one Go target causes the entire desktop/CLI scan report to fail, even if the user intended to select only another ecosystem. Existing provider test `command_providers_emit_official_actions_when_tools_are_available` covers npm/pip/pnpm/Yarn, not Go, and asserts raw scanner actions without roundtripping (`scan/global/mod.rs:605`).

Acceptance: add a registered typed Go intent with independently reconstructed scope, or explicitly downgrade unsupported Go discovery to a diagnostic/inspect state until supported. A fixture with Go plus an unrelated valid target must finish report conversion, validate, preview, and dispatch only approved selected actions through fake runners. Unknown external rule IDs must still fail closed.

#### F3 — Environment-aware discovery disagrees with trusted path resolution (P0)

Evidence: `resolve_known_cache_path` honors `GRADLE_USER_HOME`, `NUGET_PACKAGES`, and `GOMODCACHE` (`scan/global/mod.rs:532`); `resolve_trash` requires `current_home_dir().join(rule.relative)` (`rules/registry.rs:159`), and `resolve_inspect_only` repeats the default-only requirement (`registry.rs:353`). The safety helper also encodes default known paths (`execution/safety/mod.rs:780`), so all layers need a consistent reviewed contract, not just relaxed equality.

Result: observation succeeds for legitimate custom Gradle/NuGet cache paths, but desktop preview validates the full plan and rejects it. CLI fails when an affected target is selected into its saved plan. A custom inspect-only Go directory can likewise poison the full desktop plan despite being unselectable.

Acceptance: resolve permitted configured roots from trusted live context through a shared core resolver used in discovery, reconstruction, and live authorization. Bind the observed configuration/footprint to preview and revalidate drift before dispatch. Test default roots, custom roots inside/outside home, changed env/config after preview, invalid/relative roots, protected overlap, reparse paths, and malicious caller-supplied paths. Do not accept arbitrary paths just because their rule ID is known.

#### F4 — Legitimate paths are lost by provider output parsing (P0/P1)

Evidence: `output_path` rejects every line containing a space and any substring `warning` or `error` (`scan/global/mod.rs:502`), then takes only the first surviving line. `C:\Users\Jane Doe\...` and otherwise legitimate directory names with those substrings are discarded. npm/pip/pnpm/Yarn retain unknown-size command candidates (`:559`); Go emits no command target without a usable directory (`:283`).

Acceptance: provider-specific structured output parsing, or strict documented single-path parsing that preserves spaces/Unicode and validates absolute path shape. Cover CRLF, one path with spaces, Unicode, benign stderr warnings, noisy/ambiguous stdout, quoted output only where the provider documents it, malformed/relative values, and nonzero/truncated process output. Do not turn arbitrary textual warnings into path authority.

#### F5 — Python inherited discovery emits paths its registry refuses (P1)

Evidence: Python context flows from the nearest ancestor marker (`scan/project/mod.rs:252`). Every descendant calls `scan_python_project_dir`, which constructs `dir.join(rule.relative)` while recording `scope.root = context.project_root` (`:480`). The trusted registry requires `scope.root.join(rule.relative)` for all fixed Python rules (`rules/registry.rs:130`). Only `python.__pycache__` has explicit nested containment semantics (`registry.rs:139`).

Example: `project/pyproject.toml` plus `project/subpackage/.pytest_cache` produces a target with root `project` and path `project/subpackage/.pytest_cache`, which fails exact equality during preview. The main discovery fixture includes nested `__pycache__`, but only root-level `.pytest_cache`/`.venv`/`.mypy_cache` etc. (`scan/project/mod.rs:931`).

Acceptance: decide per rule whether nested caches are supported, then encode the exact ownership/evidence in both discovery and registry. Test inherited context, nested independently marked projects, nested cache positives, markerless negatives, source directories with cache-like names, and tampered path escapes. Do not globally loosen every Python rule to arbitrary descendant trash.

#### F6 — Provider coverage/error reporting is lossy (P1)

- `ProviderProbe::command_output` returns `Option<String>` (`scan/global/probe.rs:19`). Timeout/cancel/exit errors become warnings to tracing and `None` (`:49`). Successful output is returned without checking truncation flags (`:88`); the process runner tracks truncation separately (`process/mod.rs:94`, `:246`).
- `GlobalScan` returns only `CleanupPlan`, no diagnostics (`scan/mod.rs:88`), and global orchestration never merges provider health (`:394`). `ScanDiagnosticStage::Provider` already exists (`model/scan.rs:143`) but this adapter does not populate it.
- npm/pip/pnpm can still appear with unresolved paths and partial sizing; absent executables and Yarn version failures disappear entirely (`scan/global/mod.rs:122`, `:156`, `:190`, `:218`). Therefore some failures produce a partial size warning, while others can leave a supposedly complete report with no indication of omitted work.
- First existing Python executable wins even if pip is unusable (`scan/global/mod.rs:156`). Go inspect fallback is suppressed merely by finding `go`, independent of successful module-cache discovery (`:364`).

Acceptance: add a core-owned provider outcome/coverage record (available, absent, unsupported, failed, timeout, permission denied, canceled, or budget-limited as appropriate); merge into final health; show the same semantics in CLI human/JSON and desktop. A failed probe must not silently mean zero capacity. Healthy providers should remain usable after another provider fails. Provider fallback and partial execution eligibility must be explicit, not inferred from a display string.

#### F7 — CLI review output is too sparse and partial scan status is misleading (P1)

Evidence: human scan emits only a localized empty/count sentence (`application/commands/clean.rs:232`); preview emits digest/count (`:256`); execution emits generic completed/partial/failed text (`:293`). None renders target IDs/paths/rules, estimated capacity, risk, action, irreversibility, per-provider coverage, or refusal details. The JSON scan envelope is hardcoded `outcome: succeeded` (`:247`), with `warnings: []` (`:334`), regardless of `ScanHealth.completeness`. Execute classification considers failed/succeeded counts but not skipped-only results (`:163`, `:298`), so all-skipped can be generic success without explaining why nothing happened.

Acceptance: useful bilingual human tables/details and actionable next-step commands; JSON retains typed machine-neutral fields and truthful partial/unavailable outcomes; reflect skip-only and unknown outcomes without claiming cleaned capacity. Test healthy, empty, partial, absent-provider, all-skipped, mixed command/trash, and failures. Preserve saved-plan and digest requirements and create-new output behavior. Verify CLI/Tauri parity by comparing normalized core facts, not localized text or wrapper envelopes.

#### F8 — Desktop partial/empty states and command results need truthful explanations (P1)

Evidence: `emptyReport` is solely target-count-based and bypasses `ReviewPage` (`CleanWorkbench.tsx:217`); the empty ScanPage receives no health object (`:254`). The review page only exposes diagnostics in a collapsed numeric summary (`ReviewPage.tsx:88`), and always uses scan-complete count copy (`:82`). Size-only partial health can lack diagnostic rows. The result stage always displays successful trash-move bytes (`CleanWorkbench.tsx:258`), with `movedTotals` deliberately excluding command outcomes (`modes/clean/result.ts:10`). Thus a successful Cargo/npm-only run can truthfully show 0 B moved to Recycle Bin while still looking like no cleanup happened.

Acceptance: preserve diagnostics even for zero targets; show scope, inspected/skipped/unavailable providers and partial reason, plus recovery. Split results into command outcomes and trash outcomes with per-target status/reasons and optional rescan. Do not relabel command estimates as actual freed bytes or trash moves as released disk space.

### 6. Discovery limits and intentional protections

- **No generic hidden-folder or `.gitignore` filtering was found.** Traversal uses `std::fs::read_dir`; recognized dot directories are actively matched (`scan/project/mod.rs:250`, `:262`, `:451`). Avoid diagnosing the issue as a blanket hidden-file exclusion.
- Descent stops by basename for `.git`, `.hg`, `.svn`, `target`, `node_modules`, `.turbo`, `.parcel-cache`, `.next`, `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox` (`scan/project/mod.rs:802`). This prevents scanning dependency/VCS internals. A user-selected project directory itself named `target`/`venv`, or an unrelated container with those names, is consequently skipped before marker discovery (`:212`). Treat this as a coverage edge case needing a defined root policy, not permission to descend into every dependency tree.
- Markerless `target`, `node_modules`, `build`, and `dist` are intentionally not authority. Fixed rules only cover listed Rust/Node/Python artifacts; no generic `build`/`dist`, Java project output, .NET `bin/obj`, or similar project families exist in `PROJECT_DIR_RULES` (`rules/definitions.rs:137`).
- Reparse/symlink/junction targets, markers, and directories are not followed; unverifiable safety fails closed. Permission-denied nested entries produce partial diagnostics (`scan/project/mod.rs:186`, `:262`). Target/marker helper failures often return `false` without their own diagnostic (`:763`), so a coverage ledger can improve explanation while preserving refusal.
- Default target sizing is 50,000 entries and depth 64 (`filesystem/sizing.rs:63`, `:115`); reviewed rescan raises entry budget to 500,000 (`scan/project/mod.rs:668`) but not depth. Partial sizing unselects defaults (`:640`). Desktop has no reviewed-rescan bridge method (`desktop/src/api/bridge.ts:49`, `:205`); CLI exposes it with restrictions noted above.
- Project discovery itself has recursive traversal and cancellation checks, without a root-wide entry/depth/time budget parameter (`scan/project/mod.rs:169`, `:275`). Performance/bounded traversal work should not misreport capped traversal as complete.
- Provider process defaults are 5 seconds per command and a 30-second phase deadline (`process/mod.rs:60`); sizing shares the scanner lifetime but is not bounded by that process deadline. Missing coverage after the deadline needs an explicit reason.
- All global targets are unselected by default. Recent targets (under seven days), unknown mtime, and incomplete size lose default selection (`scan/ranking.rs:36`). An empty selection is not proof that nothing is cleanable.
- Cargo home, Maven repository, JetBrains vendor roots, model stores, and unowned Go caches are intentionally inspect-only. Keep these protections until a new narrowly scoped rule has its own evidence, trusted reconstruction, and tests.
- Execute protects sensitive credential/config subtrees, home/Desktop/Documents/Downloads exact roots, volume roots, user protections, running executable, and audit path (`execution/safety/mod.rs:199`, `:413`, `:638`). Runtime refusal can remain correct after a successful observation.

### 7. Capacity versus action semantics

`ScanTotals` sums all discovered target sizes, including inspect-only entries (`model/scan.rs:339`). Preview's found-so-far total is the same observation aggregate (`:327`), while the completed desktop stage correctly switched to selected estimates (`CleanWorkbench.tsx:286`) and uses all-found only as secondary context. Preserve that 09-29 fix.

The observed size of a command's cache root is not automatically the amount the command will remove. For example pnpm sizes the entire store but dispatches prune (`scan/global/mod.rs:190`); Yarn modern sizes `cacheFolder` but dispatches `--mirror` (`:226`). `ExecutionReport.estimated_recoverable` is derived from target estimates, not measured post-action disk change (`execution/mod.rs:291`, `:317`). These are confirmed modeling limits; the precise external tool semantics/versions should be verified during provider work.

Recommended contract: keep observed disk occupancy, selected candidate estimate, action-eligible estimate when actually provable, and observed outcome separate. Unknown reclaim potential must remain unknown. Inventory/Analyze is useful for finding large unsupported folders but its observations cannot become Clean authority by changing a label (`inventory/mod.rs:28`; backend quality spec at line 1434). Offer inspect/reveal/analyze pathways for unsupported folders, with separate existing Analyze confirmation rules where applicable.

### 8. Prioritized implementation and acceptance proposal

| Priority / work package | Scope | Verifiable completion |
|---|---|---|
| P0: shared rule roundtrip repair | Go intent/reconstruction; trusted configured-cache resolver; Python nested ownership; legitimate output paths | Every scanner-emittable rule can pass scan -> report -> v2 validation -> dry run; fake execution receives the exact trusted selected actions. Invalid/tampered counterparts still fail before side effects. |
| P0: explicit scan scope in CLI/Tauri | Desktop roots and visible scope; CLI effective-root reporting; healthy-root/global continuation policy | Same explicit fixture roots and provider context yield the same candidates/health in both interfaces; GUI launched elsewhere never silently substitutes CWD. |
| P1: coverage ledger and truthful partial states | Typed provider outcomes and root diagnostics, process result parsing, empty/partial UI/CLI | Timeout/permission/truncation/absence/budget cancellation all have distinct tested outcomes; zero targets after failed checks never reads as a fully clean machine. |
| P1: actionable review and results | Human CLI tables/next steps; desktop diagnostics, eligibility hints, reviewed size rescan, command/trash results | Selection/digest invalidation holds across changed scope/selection/rescan/protection; results explain executed/skipped/failed targets and never claim freed bytes without measurement. |
| P1/P2: deliberate provider expansion | Pick a finite Windows-first target catalogue informed by Mole research, extend other platforms explicitly | Each proposed rule has platform applicability, configuration precedence, evidence, footprint, risk, regeneration/retention note, live resolver, cancellation/budgets, and positive/negative/fake-execute tests. |
| Shared performance track | Scanner bounds/progress/payloads and CLI/Tauri overhead | Use the separate performance research and measured budgets; preserve coverage/partial semantics under every cap. |

Suggested expansion candidates to evaluate, not an approved delete list: additional marker-backed project outputs (Java/Gradle/Maven/.NET and supported frontend caches), official-command developer caches (e.g. Go build cache and currently missing package tools), and narrowly identified Windows user/application temporary or cache directories. Browser profiles, logs, downloads/installers, model stores, package repositories, and OS-owned caches each need separate ownership/retention analysis. Do not treat whole `AppData`, `TEMP`, user-home, browser profile, or vendor root deletion as feature parity.

Define “complete” as **all configured roots and supported providers were attempted and their outcomes are visible**, not “all directories on the machine are known safe to delete.” Keep unsupported areas explicit and inspectable. No scanner implementation can infer universal deletion safety from names alone.

### 9. Focused tests inspected and missing cross-layer coverage

Existing useful tests:

- Project marker/discovery fixtures and nonmutation: `scan/project/mod.rs:931`, `:1042`, `:1177`.
- Reparse root/child/target/marker and unreadable directory behavior: `scan/project/mod.rs:1080`, `:1102`, `:1118`, `:1134`, `:1637` (platform-specific counterpart at `:1679`).
- Cargo metadata, workspace reuse, truncation, duplicate roots, target rescan: `scan/project/mod.rs:1214`, `:1268`, `:1386`, `:1435`, `:1774`.
- Provider actions, known caches, inspect-only, timeout continuation: `scan/global/mod.rs:605`, `:773`, `:845`, `:890`. These use raw internal plans and do not cover Go or env-override-to-registry roundtrips.
- Plan rejects unknown rule/actions; provider logical action identity tests: `plan/mod.rs:512`, `:743`, `:765`.
- Executor dry-run/no-side-effect, stale digest, program/argv separation, trash path, inspect-only denial: `execution/mod.rs:1016`, `:1204`, `:1310`, `:1388`, `:1704`.
- CLI handler tests are empty-plan preview, inspect-only selection rejection, and stale execute (`application/commands/clean.rs:369`, `:390`, `:440`). Parser tests cover repeated root ordering (`application/cli.rs:1191`).
- Desktop full action-flow test calls a mocked bridge (`CleanWorkbench.test.tsx:232`); reducer rejects inspect-only/default mismatches and stale preview/digest (`app-state.test.ts:19`, `:92`, `:114`). These do not prove real scanner output can cross the Rust trust boundary.

Required new fixture matrix: all supported rule identities; default/custom roots; spaces/Unicode; a working/nonworking Python candidate sequence; missing/version-unknown/timed-out/truncated provider; nested Python cache ownership; Go plus valid unrelated targets; protected/reparse/permission paths; all-inspect/all-recent/all-partial/all-skipped cases; deterministic CLI/Tauri normalized parity. At least one integration test must use production report/plan conversion and validation, not `ValidatedPlan::from_cleanup_plan_for_test`, because that helper bypasses the real registry (`plan/mod.rs:97`). Actual side effects should remain fake-runner tests or isolated disposable fixture evidence, never user-machine cleanup.

## Related specs and prior tasks

- `.trellis/spec/backend/index.md` and `.trellis/spec/backend/quality-guidelines.md`: execution/audit contract at line 165; project marker/no-follow/deduplication contract at line 1282; scan-health/capacity/target-rescan boundary at line 1391; `just ci` remains required for implementation.
- `.trellis/spec/desktop-frontend/index.md`: generated closed IPC, both locales, accessibility, truthful capacity, no fake scan percentage, frontend checks plus shared Rust gate.
- `.trellis/spec/desktop-frontend/state-management.md:100`: completed-report-only review authority, selection/digest invalidation, second confirmation, inspect-only rejection, cancel-and-join ownership.
- `.trellis/tasks/archive/2026-09/09-23-desktop-clean-mole-parity/prd.md`: delivered impact ordering, skip/protect, trash/cumulative totals; explicitly excluded new cleanup providers. It did not establish broad filesystem coverage.
- `.trellis/tasks/archive/2026-09/09-29-desktop-clean-action-flow/prd.md`: delivered direct Clean-selected entry, selected estimate, readable preview/result navigation; explicitly excluded backend discovery/execution. Avoid redoing this already implemented work or assuming it fixed underlying scope/rules.

## External references

No external reference was required for these source findings. Mole/ref parity and precise third-party provider documentation/version contracts are researched separately by the parent. A future provider implementation must cite current official provider documentation before fixing argv, configuration precedence, or reclaim semantics. This report refers to the current checkout as inspected on 2026-10-13; no Git revision was queried due to researcher role restrictions.

## Caveats / Not Found

- No live CLI scan, Tauri launch, broad disk inspection, or cleanup was run. The user's actual installed providers, CWD, PATH, environment overrides, and affected cache folders remain unverified.
- No tests were executed; this is planning research with writes restricted to this research file. Test locations above identify existing evidence and required regression coverage, not a passing gate claim.
- No relevant memory registry entry was found in the quick pass; conclusions come from current source and repository task docs.
- No root picker, scope preset/persistence, per-provider coverage report, or desktop reviewed-target-size-rescan IPC was found in the inspected Clean path.
- No generic hidden-file exclusion was found. “All folders” must not become an unbounded recursive delete promise.
- Logical provider actions currently reconstruct executable names rather than preserving scan-time executable identity, and external provider configuration can change between scan and execute (`rules/registry.rs:238`, `:282`; `execution/command.rs:88`). Whether this can make observed paths and actual cleanup footprints diverge requires provider-specific fixture/live-context validation. Treat that as an explicit follow-up test risk, not a proven machine incident.
- Windows `.cmd`/PATH-shim differences, packaged environment differences, Yarn mirror-size mismatch, HF configured paths, and cross-platform trash behavior need targeted native/provider evidence; do not label them as measured failures from this static investigation.
