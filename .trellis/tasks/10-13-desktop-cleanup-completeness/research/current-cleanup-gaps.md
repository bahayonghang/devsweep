# Research: Current cleanup coverage, action reachability, and CLI/Tauri parity

- Query: Why does desktop Clean fail to discover and clean all expected folders? Trace scope selection, discovery, provider/rule coverage, plan conversion, preview, execution, and results; identify concrete gaps and propose shared CLI/Tauri acceptance criteria.
- Scope: internal. The parent researches `ref/Mole` and mole.fit separately. User scope was expanded during research to include both CLI and Tauri. Performance measurements are owned by the separate performance researcher; this document records relevant functional limits only.
- Date: 2026-10-13
- Method: read-only inspection of the current checkout, relevant specs, focused test bodies, and archived task PRDs. No machine cleanup, broad machine scan, Cargo invocation, or Git operation was performed. Source-proven failure conditions below are not claims that those conditions exist on the user's machine.

## Findings

### 1. Main conclusions

1. **Desktop project discovery uses the application's current working directory, not user project roots.** `CleanWorkbench` initializes `roots: ["."]`; its screen offers only Projects/Global checkboxes. There is no root picker or visible effective-root explanation. This alone makes a packaged GUI fundamentally different from a CLI launched in a chosen project directory.
2. **The shared scanner and trusted rule registry have concrete integration breaks.** A Go command candidate cannot be converted into a report; nondefault Gradle/NuGet paths are discovered but rejected by plan validation; inherited Python contexts can emit nested cache paths that validation rejects.
3. **Global discovery can silently lose coverage.** Paths containing spaces are discarded, only one Python interpreter is tried, provider process failures become `None`, and the global interface cannot return health diagnostics. Missing, failed, unsupported, and genuinely absent providers are not distinguishable in the result.
4. **Cleanup already exists.** Desktop and CLI reach the same real executor, official-command adapter, and OS trash adapter. The task should repair coverage and rule roundtrips and improve outcome explanations, not implement a second executor or bypass existing confirmation.
5. **Existing scope is developer artifacts and a small cache catalogue.** Generic OS temporary files, browser caches, application logs, installers/downloads, and many other ecosystem caches do not have Clean rules. This is a product coverage gap, not evidence that an arbitrary directory can safely be deleted.
6. **CLI human output and desktop empty/partial states hide important facts.** CLI scan prints only count/empty copy, preview only count/digest, execution only a generic status; partial scan JSON is still labeled `succeeded`. Desktop empty reports suppress the review diagnostics surface.

### 2. Files found and their responsibilities

| File | Responsibility |
|---|---|
| `desktop/src/modes/clean/CleanWorkbench.tsx` | Default scan options, operation leases, scan/dry-run/execute effects, first-screen/result composition. |
| `desktop/src/pages/ScanPage.tsx` | Only two scope controls; read-only progress/preview UI. |
| `desktop/src/pages/ReviewPage.tsx` | Selection, per-group totals, diagnostics details, preview entry point. |
| `desktop/src/state/app-state.ts` | Scan identity/sequence checks, selection validity, stale digest invalidation, execution gating. |
| `desktop/src/state/selectors.ts` | Selected target/capacity projection. |
| `desktop/src/modes/clean/result.ts` | Counts only successful trash-move estimates in the result's byte value. |
| `desktop/src/api/bridge.ts` | Typed IPC calls and response correlation. |
| `desktop/src-tauri/src/commands.rs` | Forwards `ScanOptions` and command-scoped progress channel to the Rust scan runner. |
| `desktop/src-tauri/src/scan.rs` | Scan single-flight/cancellation and safe preview projection. |
| `desktop/src-tauri/src/clean.rs` | Validates the complete received plan, then invokes shared preview/execute. |
| `crates/devsweep-cli/src/application/cli.rs` | CLI roots, scope, reviewed-size rescan, saved-plan and confirmation grammar. |
| `crates/devsweep-cli/src/application/commands/clean.rs` | Scan/report export, explicit selection into a saved plan, preview/execute, human/JSON result rendering. |
| `crates/devsweep-core/src/scan/mod.rs` | Shared project/global scan orchestration, cumulative ranking, final v2 report conversion. |
| `crates/devsweep-core/src/scan/project/mod.rs` | Marker-first traversal, target construction, root normalization, exact-target size rescan. |
| `crates/devsweep-core/src/scan/project/cargo.rs` | Scan-lifetime Cargo workspace/manifest probe cache. |
| `crates/devsweep-core/src/scan/global/mod.rs` | Global provider probes, known paths, targets, and output-path parsing. |
| `crates/devsweep-core/src/scan/global/probe.rs` | PATH/environment resolution and bounded read-only process execution. |
| `crates/devsweep-core/src/rules/definitions.rs` | Rule facts and OS-specific directory catalogue. |
| `crates/devsweep-core/src/rules/registry.rs` | Scanner action to persisted intent conversion; trusted action reconstruction. |
| `crates/devsweep-core/src/plan/mod.rs` | Whole-plan validation and scanner-to-v2 conversion. |
| `crates/devsweep-core/src/model/scan.rs` | Health, diagnostics, preview observations, and capacity-confidence DTOs. |
| `crates/devsweep-core/src/filesystem/sizing.rs` | No-follow, bounded sizing; verified/partial/unknown estimates. |
| `crates/devsweep-core/src/execution/mod.rs` | Selection/digest gate, audit, live authorization, per-target dispatch/results. |
| `crates/devsweep-core/src/execution/safety/mod.rs` | Live footprint, marker, identity, reparse, protected-path checks. |
| `crates/devsweep-core/src/execution/command.rs` | Program/argv process adapter and `trash::delete` adapter. |
| `crates/devsweep-core/src/inventory/mod.rs` | Read-only capacity inventory, distinct from a cleanup plan. |

### 3. End-to-end paths and front-end differences

#### Desktop

`CleanWorkbench` initializes `{ include_projects: true, include_global: true, roots: ["."] }` at `desktop/src/modes/clean/CleanWorkbench.tsx:49`. The two checkboxes at `desktop/src/pages/ScanPage.tsx:96` only change booleans. The unchanged options flow through `CleanWorkbench.tsx:78`, `desktop/src/api/bridge.ts:205`, and `desktop/src-tauri/src/commands.rs:92` into `CoreScanRunner` (`desktop/src-tauri/src/scan.rs:50`), then `Sweeper::full_scan_report_run_with_cancel`.

Project discovery receives exactly `options.roots` (`crates/devsweep-core/src/scan/mod.rs:335`). A completed report can become selectable; active/canceled/failed observations cannot. Selected IDs and the current full report plan go to dry run (`CleanWorkbench.tsx:122`) and then confirmed execution (`CleanWorkbench.tsx:154`). The current first screen already has a Clean-selected action when defaults are nonempty (`CleanWorkbench.tsx:286`), so earlier findings that Clean requires entering review first are stale.

Tauri validates **every target** in the report plan before considering selection (`desktop/src-tauri/src/clean.rs:34`, `:74`; `crates/devsweep-core/src/plan/mod.rs:170`). Therefore one invalid unselected discovered target can block dry run for unrelated valid targets. Do not change the trust boundary to accept invalid input; make scanner-produced candidates satisfy it and deliberately define partial coverage behavior.

#### CLI

`clean scan` exposes repeated `--root` (default `.`), `--scope projects|global|all` (default all), and `--rescan-target TARGET_ID` (`crates/devsweep-cli/src/application/cli.rs:78`). CLI root selection already works as an explicit user input, unlike desktop. `--rescan-target` currently requires exactly one root and a scope containing projects (`cli.rs:354`), although the core helper locates any path-backed target in the current scan.

The handler builds the same `ScanOptions` and uses the same Sweeper (`crates/devsweep-cli/src/application/commands/clean.rs:47`). `clean plan --observation ... --select ... --output ...` copies only explicitly selected targets into a saved v2 plan and validates that subset (`clean.rs:70`). Preview loads/validates the saved plan (`:105`, `:209`); execute requires the saved plan, `sha256:` digest, and `--confirm` (`cli.rs:122`, `clean.rs:127`). Consequently custom-path/Python validation defects can be avoided in CLI only by not selecting the affected candidates; desktop passes the full report and is more easily blocked. Go report-conversion failure affects both before selection.

#### Actual execution

The shared executor validates selection and digest before side effects (`crates/devsweep-core/src/execution/mod.rs:554`). Dry run returns before opening audit or running safety authorization (`:567`). Execute opens the fixed audit journal and live-authorizes each target (`:600`, `:666`), writes durable transitions, and dispatches (`:802`). Commands retain separate program/argv (`:936`; `execution/command.rs:88`); trash uses the authorized exact path and `trash::delete` (`execution/mod.rs:953`; `execution/command.rs:132`). Permanent deletion is disabled (`execution/mod.rs:962`).

This explains an important UX distinction: a successful dry run establishes a valid action/selection digest, but does not prove that later live safety checks, permissions, tools, or trash operations will succeed. A read-only eligibility/preflight projection could expose likely refusals earlier, while execution must still revalidate.

### 4. Current supported rule/provider matrix

These are current code facts, not recommendations to run cleanup on the machine. All default selections remain subject to size completeness and the seven-day freshness guard (`scan/ranking.rs:8`, `:36`).

| Family / rule | Discovery evidence / location | Current action | Important limits |
|---|---|---|---|
| Rust `rust.target` | `Cargo.toml`; Cargo metadata resolves workspace and target directory | Official `cargo clean --manifest-path ... --target-dir ...`; metadata-failure fallback is local `target/` trash, medium risk, unselected | Custom target directories are supported by Cargo metadata and live scope revalidation. See `scan/project/mod.rs:307`, `:389`. |
| Node `node.node_modules` | `package.json`, `package-lock.json`, `pnpm-lock.yaml`, or `yarn.lock`; direct `node_modules` | Trash, medium risk, unselected | Does not treat a markerless matching name as authority. |
| Node `node.next_cache`, `node.turbo`, `node.parcel_cache` | Same markers; `.next/cache`, `.turbo`, `.parcel-cache` | Trash, low risk, initially selected | Next support is cache subtree only, not all `.next` output. |
| Python `python.venv_dot`, `python.venv`, `python.tox` | `pyproject.toml`, `requirements.txt`, `setup.py`, `setup.cfg`, `tox.ini`; `.venv`, `venv`, `.tox` | Trash, medium risk, unselected | `pyvenv.cfg` alone is not included in marker detection. Inherited nested paths have the validation mismatch below. |
| Python `python.pytest_cache`, `python.mypy_cache`, `python.ruff_cache` | Same Python project context; corresponding directory names | Trash, low risk, initially selected | Same inherited-context mismatch for nested cache directories. |
| Python `python.__pycache__` | Python context inherited down source directories | Trash, low risk, initially selected | Registry deliberately permits nested `__pycache__` under the project. |
| `npm.cache.clean` | `npm` on PATH; `npm config get cache` | `npm cache clean --force`, irreversible, unselected | Missing path leaves an unknown-size command candidate. No tool means no candidate. |
| `pip.cache.purge` | First available `py`/`python`/`python3`; `-m pip cache dir` | `-m pip cache purge`, irreversible, unselected | Only one interpreter is tested; installed interpreter without pip prevents fallback to a later working interpreter. |
| `pnpm.store.prune` | `pnpm` on PATH; `pnpm store path` | `pnpm store prune`, irreversible, unselected | Full observed store size is not evidence that all those bytes are prunable. |
| `yarn.cache.clean.classic` | Yarn version <= 1; `yarn cache dir` | `yarn cache clean`, irreversible, unselected | No parseable version means provider omitted. Base `yarn.cache.clean` is catalogue description, not emitted executable identity. |
| `yarn.cache.clean.modern` | Yarn major > 1; `yarn config get cacheFolder` | `yarn cache clean --mirror`, irreversible, unselected | CacheFolder sizing versus mirror cleanup footprint needs verification. Future major versions are treated as modern without capability detection. |
| `go.mod_cache.clean` | `go` on PATH, `GOMODCACHE` or `go env GOMODCACHE`; directory exists | Scanner constructs `go clean -modcache` | **Broken end to end:** missing intent/registry handling prevents report conversion. |
| `cargo.home.inspect` | `CARGO_HOME`, else home `.cargo` | Inspect only | Credentials, binaries, registry/git internals intentionally have no cleanup action. |
| `gradle.caches`, `gradle.wrapper_dists` | `.gradle/caches`, `.gradle/wrapper/dists`; discovery honors `GRADLE_USER_HOME` | Trash, medium risk, unselected | **Nondefault locations fail trusted path validation.** |
| `nuget.packages` | `.nuget/packages`; discovery honors `NUGET_PACKAGES` | Trash, medium risk, unselected | **Nondefault locations fail trusted path validation.** |
| `ivy.cache` | `.ivy2/cache` | Trash, medium risk, unselected | Fixed home-relative path only. |
| `maven.repository` | `.m2/repository` | Inspect only, high risk | Intentional protection of repositories that can contain locally published artifacts. |
| `go.mod_cache` | `go/pkg/mod`, discovery honors `GOMODCACHE`; skipped whenever `go` resolves | Inspect only, high risk | If Go resolves but its probe fails, fallback is still suppressed. Nondefault inspect paths also fail registry validation. |
| `jetbrains.caches` | Platform-specific vendor root | Inspect only, high risk | Vendor root may contain important data; enabling whole-root trash is not an acceptable coverage fix. |
| `huggingface.hub` | Platform-specific fixed home-relative directory | Inspect only, high risk | No `HF_HOME`/`HF_HUB_CACHE` resolution in this provider path. Actual installation coverage needs separate provider research. |
| `docker` | Catalogue has a deferred doc entry | No scan or execute support | Explicitly outside the current MVP contract. |

Matrix evidence: `crates/devsweep-core/src/rules/definitions.rs:12`, `:137`, `:240`; project marker functions at `scan/project/mod.rs:738`; global construction at `scan/global/mod.rs:122`, `:150`, `:190`, `:218`, `:271`, `:308`, `:351`.

#### Platform-specific locations and behavior

- Windows: JetBrains `home/AppData/Local/JetBrains`, HuggingFace `home/AppData/Local/huggingface`; macOS: `home/Library/Caches/JetBrains` and `.../huggingface`; other platforms: `home/.cache/JetBrains` and `.../huggingface` (`rules/definitions.rs:297`). These are fixed strings, not Windows Known Folder or XDG/HF environment resolution.
- Windows Python priority is `py`, `python`, `python3`; non-Windows reverses to `python3`, `python`, `py` (`scan/global/mod.rs:150`). Provider discovery does not enumerate all Python installations.
- PATH resolution uses PATHEXT on Windows (`scan/global/probe.rs:139`). Packaged desktop and an interactive terminal may inherit different PATH/environment values. This is a plausible machine-specific cause to test, not a proven cause from the screenshot.
- Home resolution prefers USERPROFILE, then HOMEDRIVE+HOMEPATH, then HOME (`scan/global/probe.rs:111`). Known-directory providers need a home value even for some environment overrides (`scan/global/mod.rs:357`).
- Core scanner and CLI operate across platforms; do not assume Mole's macOS cache/system paths map safely to Windows. Define platform applicability per rule and show unsupported/unavailable states.

## Continued evidence

See [Cleanup failures, safety boundaries, and proposed regression tests](cleanup-failures-and-tests.md) for the remainder of this investigation, including boundaries, acceptance proposals and references. Load both files for implementation/check context.
