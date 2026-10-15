# Research: Independent global cleanup and project artifact inspection

- Query: What do current Core, CLI and Tauri contracts actually scan and clean; what is missing for independent global-cache cleanup and on-demand project artifact cleanup with capacity warnings, including uv and mbx?
- Scope: mixed; source inspection plus installed mbx help and upstream primary documentation. Research only; no product changes, builds, cleanup, broad cache scans or Git commands.
- Date: 2026-10-14
- Active task: `.trellis/tasks/10-14-cleanup-home-redesign`
- Confirmed product decision relayed by the coordinating session: warn when one project's artifact total is **>= 5 GiB**, make the threshold configurable, and evaluate only after a user-requested scan. No background scan and no automatic cleanup.

## Findings

### 1. The two scan scopes already exist; the initial desktop interaction couples them

`ScanOptions` already contains independent `include_projects`, `include_global`, and `roots` fields (`crates/devsweep-core/src/scan/mod.rs:203`). The pipeline scans projects first when requested, merges their observations, then independently scans global providers (`scan/mod.rs:455`, `scan/mod.rs:503`). A global-only request does not need a project root. A project scan requires at least one root (`scan/project/mod.rs:116`).

CLI already exposes `clean scan --scope projects|global|all`, defaulting to `all` and root `.` (`crates/devsweep-cli/src/application/cli.rs:79`). Its adapter translates the scope directly into the two core flags (`application/commands/clean.rs:47`). This CLI default is deliberately different from the desktop's explicit-root contract; do not accidentally change it in a homepage redesign.

Desktop initializes both flags to true with an empty root list (`desktop/src/modes/clean/useScanScope.ts:8`). The two checkboxes control one Scan button, which is disabled whenever projects are enabled but roots are empty (`desktop/src/pages/ScanPage.tsx:98`). Consequently a first-time user must discover that disabling Projects unlocks global inspection. This matches the screenshot's disabled Scan state; it is an interaction-model problem even though the underlying scope mechanism exists.

Tauri passes `ScanOptions` to the shared `Sweeper`, and enforces explicit absolute project roots (`desktop/src-tauri/src/commands.rs:102`, `commands.rs:185`; `desktop/src-tauri/src/scan.rs:51`). It does not run a second scanner or invoke a CLI subprocess. Preserve that shared-core architecture; the user's requirement that CLI can clean global caches is already compatible with it.

Recommended first-screen model: two independent entry points with scope-local results and status: **Global caches** and **Project artifacts**. Global action sends `{include_projects:false, include_global:true, roots:[]}`. Project action sends the explicit validated roots with `{include_projects:true, include_global:false}`. Do not silently request both scopes, automatically traverse suggested folders, or imply whole-machine coverage.

### 2. Global provider catalogue and uv are already implemented

The built-in rule catalogue is `crates/devsweep-core/src/rules/definitions.rs`, not UI text. Global scanning currently attempts npm, pip, pnpm, Yarn, Cargo home, Go modules, uv, Bun, Go build cache, known Gradle/NuGet paths and Windows cache families. Provider construction is in `scan/global/mod.rs:216`; fixed Windows families are listed at `rules/definitions.rs:5`; known global paths start at `definitions.rs:401`.

| Finding | Existing behavior | Evidence |
| --- | --- | --- |
| uv | Query `uv cache dir`; owner command `uv cache clean`; medium risk; not selected by default | `scan/global/mod.rs:284`, `rules/definitions.rs:242`, `rules/registry.rs:273` |
| npm | `npm cache clean --force` | `rules/definitions.rs:160` |
| pip | Resolve Python launcher and use `-m pip cache dir` / `-m pip cache purge` | `scan/global/mod.rs:538` |
| pnpm | Query store path, use `pnpm store prune`; store occupancy is not pruning yield | `scan/global/mod.rs:579` |
| Yarn | Separate Classic and modern global/mirror commands | `scan/global/mod.rs:607` |
| Go | Separate module cache and build cache owner commands | `rules/definitions.rs:224`, `definitions.rs:258` |
| Cargo home | High-risk inspect-only finding; never clean credentials, installed binaries or registry/Git internals | `rules/definitions.rs:215` |
| Docker | Deferred catalogue entry, not scanned | `rules/definitions.rs:233` |
| mbx | No implementation or references found in the searched product Core/CLI/Tauri/frontend code | Scoped `rg -n mbx` search on those trees and docs returned no matches |

Global owner commands bind live executable identity/hash, resolved cache path, version, relevant allowlisted nonsecret configuration, and required interpreter/script support files before execution. For uv the path/version probes and configuration keys include `UV_CACHE_DIR` and `UV_CONFIG_FILE` (`crates/devsweep-core/src/execution/provider.rs:88`, `provider.rs:100`). The reviewed uv range is currently major 0, minor 4 through 11 inclusive (`rules/configuration.rs:251`). A discovered cache is therefore not proof that the installed tool passes the preview capability gate.

Upstream uv documentation confirms `cache clean` clears cache entries and `cache prune` removes unused entries; cache internals must not be manually edited. This task should retain the existing `uv.cache.clean` behavior unless a separately specified prune action is wanted. Reference: [uv caching](https://docs.astral.sh/uv/concepts/cache/). The current CLI reference also warns that symlink installation mode couples environments to cache contents, so copy must not promise that every cache operation has zero impact: [uv CLI reference](https://docs.astral.sh/uv/reference/cli/).

Global results should distinguish an absent tool, an empty cache, unsupported configuration, blocked cleanup, and partial sizing. They must not flatten all of those outcomes into “0 B cleanable.” Coverage is typed, and independent provider failure preserves healthy siblings (`.trellis/spec/backend/scan-coverage.md`; `crates/devsweep-core/src/model/scan.rs:153`).

### 3. Project ownership is marker- and tool-based, not directory-name-based

Each target already records `Scope::Project { root }`, ecosystem, kind, path, size confidence, risk, and a declarative intent (`crates/devsweep-core/src/model/plan.rs:37`, `plan.rs:181`). `TargetKind` distinguishes build artifacts, dependencies, virtual environments, test caches and tool caches (`plan.rs:211`). This is enough to group known targets by their real owner without parsing labels or deriving roots from path strings.

**Rust:** `Cargo.toml` triggers `cargo metadata`; the returned workspace root and target directory own the result (`scan/project/mod.rs:400`). The target can be outside the project root. The scanner currently constructs the irreversible command `cargo clean --manifest-path <workspace/Cargo.toml> --target-dir <resolved target>` (`scan/project/mod.rs:438`), and the trusted registry independently reconstructs it (`rules/registry.rs:226`). Live safety rechecks Cargo's current target directory and rejects drift (`execution/safety/mod.rs:681`). Workspace-member metadata is cached within the scan, avoiding redundant probes (`scan/project/cargo.rs:61`).

When Cargo metadata fails, an ordinary, real local `<project>/target` can become a medium-risk, unselected trash fallback with explicit fallback evidence; no external target is invented (`scan/project/mod.rs:482`). A missing or reparse target is not followed.

**Node baseline:** package/lockfile evidence permits `node_modules`, `.next/cache`, `.turbo`, and `.parcel-cache`. Dependencies are medium risk and unselected; the three cache rules are currently low risk and initially selected (`rules/definitions.rs:298`). This does not mean all Node outputs, `.next` contents or arbitrary `build` directories are automatically supported.

**Additional generated outputs:** Nuxt, SvelteKit, Angular, Astro, generic Node `dist`/`build`/`coverage`, Maven, Gradle, .NET and nox have explicit rules (`rules/definitions.rs:33`). In particular:

- `dist`: recognized Node build dependency plus `.vite/manifest.json` evidence.
- `build`: recognized Node build dependency plus `asset-manifest.json` evidence.
- `coverage`: recognized test dependency plus `coverage-final.json` evidence.

Recognized generic Node build owners are `vite`, `webpack`, `rollup`, and `react-scripts`; test owners include `vitest`, `jest`, `nyc`, and `c8` (`rules/generated.rs:80`). Generated-content verification checks no-follow paths, rejects secret/repository material, requires appropriate output evidence, verifies Git ignore status and no tracked content, and is bounded to 50,000 entries / two seconds (`rules/generated.rs:152`, `generated.rs:238`). Failed proof retains a sized inspect-only finding rather than granting trash authority (`scan/project/mod.rs:610`, `mod.rs:664`).

Important current limitation: the generated-output owner check is process-name based. On Windows any matching `node.exe`/`bun.exe` can mark Node outputs busy, even when it belongs to a different project; non-Windows owner checks return unknown (`rules/generated.rs:129`; `rules/windows_caches.rs:875`, `windows_caches.rs:913`). Treat this as an existing conservative limitation, not permission to relax ownership checks in a visual refactor. If the project-artifacts child improves it, it needs explicit bounded process-to-project evidence and tests.

### 4. mbx requires a dedicated ownership adapter; executable integration is not proven

Read-only local checks on 2026-10-14:

- `Get-Command mbx` resolves `C:\Users\lyh\.cargo\bin\mbx.exe`.
- `mbx --version` reports **1.24.0**.
- `mbx settings get cache_dir` prints `D:/mbx-cache`.
- `mbx cache dir` prints `D:/mbx-cache\actions`, which is the action-store directory, not the entire cache root.
- `mbx cache --help`, `clean --help`, `gc --help`, `cache projects --help`, `cache stats --help`, and `cache remove --help` were read. No cache listing/sizing, collection, cleanup or mutation was run.

The prior machine-memory record described mbx 1.23.0 and shared target migration. It was useful for finding the ownership question but is not current-version evidence. Version and cache-dir facts above were refreshed directly. Effective `cargo` dispatch was not tested in this task.

| mbx surface | What is established | Proposed DevSweep treatment |
| --- | --- | --- |
| Shared action store | Reusable objects across workspaces; `cache dir --json` and `cache stats --json` are machine-readable surfaces | Global, inspect-only initially; never add the store's full occupancy to every project |
| Managed target and learned incremental state | `mbx clean <WORKSPACE>` removes these plus the managed target link, preserves shared objects/claims and keeps state in use | Project association is conceptually appropriate, but remains inspect-only until a versioned ownership adapter and live authority contract are proven |
| `mbx cache projects` | Human-readable workspace attribution command; installed help advertises no JSON flag | Do not treat table text as stable authority or invent a `--json` option |
| `mbx cache remove <WORKSPACE>` | Also forgets workspace cache claims | Broader intent than ordinary project build cleanup; exclude from initial action design |
| `mbx gc` | Global policy collection covers private state, managed targets and shared store; low-disk behavior may remove items absent from its dry-run list | Do not expose it as “clean this project's cache” or assume a frozen per-item preview |

The installed help is consistent with [mbx CLI 1.24.0](https://mr-boxington.jdx.dev/cli/) and [cache projects](https://mr-boxington.jdx.dev/cli/cache/projects). [mbx stability](https://mr-boxington.jdx.dev/stability) lists versioned JSON outputs but explicitly excludes internal session streams as a public API. The supported JSON report shapes and ownership records were not sampled; no safe per-workspace machine interface has been demonstrated here.

Managed targets normally live outside the project and are exposed through a `target` symlink; Cargo continues reporting the linked project path. Plain `cargo clean` does not remove mbx's private incremental state. See [managed target directories](https://mr-boxington.jdx.dev/managed-targets). DevSweep's ordinary Rust path check intentionally rejects reparse/symlink directories (`scan/project/mod.rs:421`, `mod.rs:1039`), so the current generic scanner cannot be assumed to measure or clean a managed target. Preserve that default no-follow rule. A dedicated provider must prove the link, workspace, resolved managed location, configured target root, and current tool identity; arbitrary link-following is not the fix.

[mbx gc](https://mr-boxington.jdx.dev/cli/gc) documents that low-disk collection can differ from its dry run and reclaim beyond the action-store budget. This is a material mismatch with a per-project selection. Do not implement `gc` as an unreviewed shortcut to satisfy the user's “mbx cache” example.

Bounded implementation sequence for the mbx child:

1. Pin/review mbx 1.24.0 reporting and config contracts; prove whether public versioned reports can associate private bytes with one workspace without parsing display text or unstable session files.
2. Add explicit availability and inspect-only coverage for supported, missing, malformed, unsupported-version, inaccessible and ambiguous states. If attribution cannot be established, report global shared storage separately and project attribution as unavailable; never return a misleading zero.
3. Only add a new `mbx clean <workspace>` built-in intent after the adapter can revalidate the exact workspace-owned state, external target roots, lease/busy state, protections, tool/config changes and stale digest behavior. Keep argv separate. Do not use `--under`, manual deletion, `cache remove`, or `gc` in this project action.
4. If that evidence cannot be established inside the child scope, finish with truthful inspection/warnings and document executable mbx cleanup as NOT_IMPLEMENTED. Do not mark the user's entire cleanup expectation satisfied by an inspect-only row.

### 5. Deduplication and size confidence need to precede the new warning

Input project roots are normalized to a minimal covering set (`scan/project/mod.rs:680`, `mod.rs:761`). The preferences store intentionally preserves separate nested selections; removing a nested selection does not exclude it while a parent stays selected (`.trellis/spec/backend/scan-preferences.md`). Show that distinction clearly in the project picker.

Targets are deduplicated by canonical footprint plus action identity: exact matches merge evidence; nested same-action paths collapse; equal paths with different actions stay visible (`scan/project/dedupe.rs:5`, `dedupe.rs:44`). This is not physical-byte accounting and is not a universal guarantee against overlap across distinct cleanup methods. New Cargo/mbx overlap must be resolved deliberately so the same managed target is not counted once as Rust output and again as mbx private storage. Shared cache claims are not additive project-owned bytes.

Current `ScanTotals` separates verified bytes, partial lower-bound bytes and unknown count, but simply aggregates target estimates (`model/scan.rs:451`). `SizingWarningKind` only covers incomplete observations, cancellation and path safety; it contains no oversized-project warning (`model/scan.rs:487`). No threshold/oversized-project rule was found in the current scan/model/Clean UI paths searched.

Use a distinct capacity advisory model or pure shared projection rather than overloading `SizingWarningKind`, risk, coverage failure or cleanup intent. The confirmed threshold is **5,368,709,120 bytes** (5 GiB), inclusive. Group by backend project owner and add deduplicated project artifact bytes once. Emit:

- Complete estimate >= threshold: oversized advisory with measured logical bytes.
- Partial lower bound >= threshold: oversized advisory qualified as “at least”; the inequality is already established.
- Partial lower bound below threshold or unknown: incomplete/unknown assessment, not “within budget.”
- Shared mbx store attribution: excluded from per-project totals; separately displayed global occupancy.

The warning evaluates only a completed user-requested result, does not schedule work or change selection, and never executes cleanup. Keep threshold settings outside cleanup authority/digests. A settings change can re-evaluate a displayed completed report without a filesystem scan. Persist the setting through a small closed schema with validation and corrupt/future-file preservation; the implementation child should choose the existing store extension versus a dedicated cleanup preference only after checking compatibility requirements.

One policy detail still worth making explicit in the child PRD: whether “project artifact total” includes `DependencyDirectory`/virtual environments as well as build/test/tool output. The user's Node build/Cargo build examples strongly support build/test/tool artifacts as the default total; display dependency directories separately unless the coordinating session confirms broader aggregation. Do not silently mix all workspace source files into this total.

### 6. Existing initial selection is more aggressive than “on demand” implies

Rust `rust.target` is currently low risk and initially selected (`scan/project/mod.rs:449`), as are some Node/Python cache rules. Ranking removes defaults for incomplete sizes, unknown mtime, or modification within seven days (`scan/ranking.rs:36`). Therefore old fully measured Rust outputs can still arrive selected. Desktop consumes `selected_by_default` on completed reports (`desktop/src/state/app-state.ts:96`, `app-state.ts:180`).

For the new project-artifacts workflow, use explicit opt-in selection; exceeding a threshold never selects anything. If this changes core defaults, cover CLI/TUI/Desktop parity and update the ranking spec. If the policy is desktop-only, make that explicit so CLI/TUI semantics are not misrepresented. The requirements currently favor a consistent core project default of false.

Spec drift found during research: `.trellis/spec/backend/quality-guidelines.md` says missing `last_modified` preserves existing default selection in its ranking scenario; current `scan/ranking.rs:53` explicitly deselects unknown mtime. Prefer current safe behavior and repair the spec during implementation; do not regress code to match the stale sentence.

### 7. Plan/preview/execute contracts remain reusable

CLI sequence is already concrete:

```text
devsweep clean scan --scope global --format json --output <new-observation.json>
devsweep clean plan --observation <observation.json> --select <target-id>... --output <new-plan.json>
devsweep clean preview --plan <plan.json> --format json
devsweep clean execute --plan <plan.json> --preview-digest sha256:<live-digest> --confirm
```

For projects, use explicit `--root` arguments and `--scope projects`. These are documentation examples; no commands above were executed during research.

`clean plan` requires exact target IDs, rejects inspect-only targets, creates a selected plan and validates it before writing (`application/commands/clean.rs:79`). Preview and execute both load the saved selection and call the shared `Executor`; execute rejects stale confirmation (`clean.rs:114`, `clean.rs:136`). The CLI parser requires `--confirm` and the live digest (`application/cli.rs:123`). Named output files are create-new under the established output contract.

Tauri accepts the untrusted plan plus explicit selected IDs, independently validates it, and invokes the same executor for dry-run/execute (`desktop/src-tauri/src/clean.rs:19`, `clean.rs:52`). A new homepage should reuse that backend authority, preserve cancel-and-join coordination, invalidate stale preview on scope/selection/protection changes, and show irreversible owner commands before the second confirmation. No direct provider cleanup from React or scanner modules.

### 8. Bounded child-task recommendations and tests

The coordinating session's six-child split fits the findings:

| Child | Contract / acceptance focus |
| --- | --- |
| Scope contracts and global cleanup | Independent requests/results; global scan with zero project roots; catalogue availability and truthful coverage; preserve uv command/version binding and CLI parity |
| Project artifacts and mbx | Marker/owner proof, explicit project grouping, Cargo workspace/shared target deduplication, Node generated-output eligibility, mbx versioned read-only adapter and explicit executable gate |
| Capacity warnings | Inclusive configurable 5 GiB aggregate threshold; user scans only; lower-bound/unknown logic; no selection mutation; no shared-store double-counting |
| Desktop workbench | Two visible independent workflows; scope-local state and freshness; validated roots; per-project review and explicit selections; existing shared authority flow |
| Animal identity | UI/branding owner; no scanner changes required |
| Integration acceptance | Equal-input CLI/Tauri report parity; schema generation; isolated provider fixtures; native read-only UI evidence; preserved cleanup safeguards |

Required meaningful verification, using isolated fixtures:

- Global-only with empty roots never calls project discovery; project-only never probes global providers; all scope preserves independent failure accounting.
- Node recognized output vs markerless `build/dist`, tracked files, secrets, reparse paths, running/unknown owner, missing output evidence, monorepo nested ownership.
- Cargo root/member deduplication, custom external target, metadata failure fallback, shared target among roots, modified target configuration after preview, and a linked mbx-managed target that generic scanning must not follow.
- mbx absent/unsupported/malformed/truncated probes; configured cache and target roots differ; shared bytes excluded from per-project totals; duplicated Cargo/mbx footprint; no unsafe cleanup intent synthesized from human output; every new action needs real scan -> saved plan -> live preview -> simulated executor sentinel tests.
- Warning tests at threshold minus one byte, exact threshold, plus one; sum of multiple smaller artifacts crossing the threshold; overlapping targets counted once; partial above/below threshold; unknown; settings changes; default 5 GiB; invalid/future/corrupt stored setting; no background task or cleanup dispatch.
- Selection tests proving oversized project rows remain unselected, inspect-only rows cannot be selected by row/select-all/reducer, and changed scope/selection immediately invalidates digest and confirmation.
- Retain separate verified/partial/unknown capacity in CLI and desktop; do not label logical occupancy or store-prune estimates as disk space already freed.

Useful existing test homes: `crates/devsweep-core/src/scan/project/mod.rs` inline tests (root coverage, reparse, deduplication and reviewed rescans); `scan/global/mod.rs` inline provider fixtures; `scan/ranking.rs`; `crates/devsweep-cli/tests/provider_authority.rs:177` and `:249`; `desktop/src-tauri/src/commands.rs` tests; `desktop/src/modes/clean/useScanScope.test.ts`; `desktop/src/state/app-state.test.ts`; `desktop/src/modes/clean/coverage-results.test.tsx`. Existing provider parity tests protect actual plan validation and stale script/config identity, so extend them instead of bypassing their authority pipeline with fabricated validated plans.

## Files Found and Related Specs

| File | Role |
| --- | --- |
| `AGENTS.md`, `code_map.md`, `.trellis/workflow.md` | Repository safety, navigation and persisted research workflow |
| `.trellis/spec/backend/scan-coverage.md` | Root/provider observations and partial/legacy compatibility |
| `.trellis/spec/backend/scan-preferences.md` | Explicit desktop roots and atomic preference contract |
| `.trellis/spec/backend/provider-authority.md` | Live tool/cache/config binding and owner-command constraints |
| `.trellis/spec/backend/quality-guidelines.md` | Plans, scan safety, ranking, selection and canonical validation gate |
| `.trellis/spec/desktop-frontend/index.md` | Core-to-Serde-to-TypeScript parity, native scope assertions and truthful capacity presentation |
| `crates/devsweep-core/src/model/{plan,scan}.rs` | Closed authority DTOs versus non-authoritative scan/capacity observations |
| `crates/devsweep-core/src/scan/{mod,ranking}.rs` | Independent scopes, cumulative merge/ranking, default-selection guard |
| `crates/devsweep-core/src/scan/project/{mod,cargo,dedupe}.rs` | Project discovery, Cargo workspace resolution and footprint deduplication |
| `crates/devsweep-core/src/scan/global/mod.rs` | Provider availability, query and target construction |
| `crates/devsweep-core/src/rules/{definitions,registry,configuration,generated,windows_caches}.rs` | Canonical rule facts, authority reconstruction, version/config checks and ownership proof |
| `crates/devsweep-core/src/execution/{provider,safety/mod}.rs` | Live digest binding and pre-dispatch safety checks |
| `crates/devsweep-cli/src/application/{cli,commands/clean}.rs` | Explicit scan/plan/preview/execute CLI adapters |
| `desktop/src-tauri/src/{commands,scan,clean}.rs` | Native scope validation, scan lifecycle and shared executor |
| `desktop/src/modes/clean/useScanScope.ts`, `desktop/src/pages/ScanPage.tsx`, `desktop/src/state/{app-state,selectors}.ts` | Current initial scope coupling, default selection and capacity projections |

## Caveats / Not Found

- This is source/help research, not a PASS report. No implementation, cleanup, tests, benchmarks, production cache traversal, native UI acceptance or Cargo dispatch validation was performed.
- The requested live mbx version is confirmed as 1.24.0; an upstream stability page still displayed a 1.23.0 navigation badge. Use the installed help and versioned fixtures for capability gating rather than assuming every documentation page refreshed together.
- mbx currently has no DevSweep integration. Per-workspace stable ownership output, detailed byte schema, and a safe executable preview contract remain unproven; initial inspect-only support is the defensible baseline until the child establishes those facts.
- A mistyped exploratory `mbx status --help` returned “no such command” (mbx delegates unknown commands toward Cargo). There is no observed `status` subcommand; subsequent inspection used documented `cache`, `stats` and `settings` surfaces.
- No oversized-project threshold exists in the inspected current code. The 5 GiB default is a newly confirmed product requirement, not an existing backend constant.
- No code or configuration was changed, including the pre-existing `desktop/src-tauri/Cargo.toml` modification noted by the coordinator. Only this research file is owned by this agent.
