# Research: mbx 1.24.0 workspace-clean authority qualification

- Query: Can DevSweep safely expose a saved-plan workspace cleanup backed by mbx 1.24.0, and which machine inputs support project attribution?
- Scope: mixed; pinned upstream source, official reporting/config contracts, existing task designs and the earlier scan-contracts.md research. No cache enumeration, cleanup, gc, builds, user-data mutations or conformance execution.
- Date: 2026-10-14
- Task: `.trellis/tasks/10-14-cleanup-home-redesign`; qualification for child B, not implementation.

## Findings

### Decision and scheduling handoff

**Executable mbx cleanup is NOT QUALIFIED.** The public interfaces do not expose a per-workspace ownership/footprint/expected-state manifest. Source audit establishes substantially more mutations than deleting a managed target, but does not establish a complete environment-independent operation that DevSweep can bind to its saved preview and enforce at dispatch. Do not add `mbx clean WORKSPACE` to the trusted execution registry on the strength of a mock, version check, record file or output table.

B can implement meaningful project observation, conservative attribution, alias deduplication, no-follow traversal and honest capability statuses now. Node/Cargo ownership work can proceed. The mbx executable acceptance remains open; inspect-only is partial delivery. A has no new cleanup schema requirement and sets project default selection false. C should consume a separate observational DTO using existing `Scope::Project { root }`, with verified ownership and size-completeness evidence; it must never turn that DTO into cleanup authority.

The concrete completion path is an owner-side, versioned machine contract that reports the complete footprint and enforces its expected state under the appropriate leases before mutating. This may be an upstream mbx enhancement or a separately approved, version-pinned companion implementation using audited owner code. It is additional implementation scope, not a hidden substitute for the current command. An isolated conformance run can test a proposed contract; it cannot supply missing runtime guarantees by itself.

### Evidence fidelity and version

The earlier local help inspection established `C:\Users\lyh\.cargo\bin\mbx.exe` reports **1.24.0**. No new local executable invocation occurred in this qualification. That version string is not binary provenance or a digest binding; a future adapter must capture and revalidate the effective executable identity/hash.

The official [v1.24.0 release](https://github.com/jdx/mr-boxington/releases/tag/v1.24.0) resolves to commit **`1b62dc2c795c93630d4ce28c4d9d2540d6e776ea`**. Version-matched source was retrieved from the official GitHub Contents API at `ref=v1.24.0`, then its base64 content decoded in memory. Raw-source markdown extraction was found to remove Rust angle-bracket content and was not used as exact-source evidence. Local Cargo registry copies were 1.13/1.16, so they were not used to substantiate 1.24 behavior.

All upstream line references below refer to that pinned commit. Source links are collected at the end. Product code navigation and current generic Cargo safety are already documented in `research/scan-contracts.md` and were not re-researched.

### Public machine inputs: supported and insufficient

| Interface | Verified contract | Limitation for this task |
| --- | --- | --- |
| `mbx cache dir --json` | JSON `{ "version": 1, "store": "..." }`; source `cli/cache.rs:125-134` | Names the shared **actions** store, not a workspace target or the effective target root. |
| `mbx cache stats --json` | Versioned aggregate report, `byte_accounting = logical`, action/target/incremental/generated totals; `cli/cache.rs:253-383` | Traverses global stores; no per-project paths, ownership manifest, busy state or action authority. Do not run during this bounded qualification. |
| `mbx cache projects` | Human table; `cli/cache.rs:433` | No advertised JSON switch. Columns/text are not a stable machine authority. |
| `mbx settings get cache_dir`, `mbx settings get target.root` | Resolved scalar setting printed as text; unset prints nothing; `cli/settings.rs:123-132` | Not a JSON settings report. Empty output does not itself mean the effective path is absent; defaults still apply. |
| `mbx clean WORKSPACE` | Explicit workspace argument; no preview/expected-state/JSON flag in `CleanArgs`; `cli/clean.rs:14-25` | Human progress/warnings; zero exit status can include kept/busy components. |

Official user references: [cache dir](https://mr-boxington.jdx.dev/cli/cache/dir), [cache stats](https://mr-boxington.jdx.dev/cli/cache/stats), [cache projects](https://mr-boxington.jdx.dev/cli/cache/projects), [managed targets](https://mr-boxington.jdx.dev/managed-targets), [stability](https://mr-boxington.jdx.dev/stability). These live docs provide context; pinned source is the behavioral evidence. There is no assumed `projects --json`, `clean --json`, `clean --dry-run` or `status` API.

### Identity and configuration inputs

Use the following notation for the audited implementation, not a newly promised public format:

- `W`: Cargo-reported workspace root **with its exact native path spelling**. `cli/cache.rs:649-663` explicitly warns that filesystem canonicalization, Windows extended prefixes and symlink spellings are not interchangeable with the key identity. A member manifest may resolve to the whole workspace; failed Cargo probing falls back to the requested path.
- `C`: effective `Config.cache_dir`, from environment/config/default. `Config::load_for_cli` is `config.rs:1175`; configuration location is platform config directory plus `mbx/config.toml` (`config.rs:1864`). Relevant overrides are `MBX_CACHE_DIR` and `MBX_TARGET_ROOT`.
- `T`: effective `target.root`; absolute setting stays absolute, relative setting joins C, unset defaults to `C/targets` (`config.rs:1263-1267`). Relative roots containing parent components are not normalized into a safe DevSweep boundary by this code. A conservative observer should reject unresolved/relative/escaping roots.
- `S = C/actions`: shared action store (`config.rs:1631`). The Cargo-root probe library independently resolves its cache root via environment/global config/default (`mbx-cache-cargo/src/lib.rs:317-329`); binding only the mbx runtime `Config` object is insufficient.
- `Kt = BLAKE3(W.to_string_lossy().as_bytes())`, full hex; target is lexical-absolute `T/v1/Kt`; record is `T/v1/Kt.json`; lock is `T/v1/Kt.lock` (`target.rs:1770-1803`, `target.rs:285`).
- `Ki = first 16 hex BLAKE3(W.as_os_str().as_encoded_bytes())`; private learned state is `C/incremental/Ki`, record `checkout.json` (`incremental.rs:221-228`, `incremental.rs:283-288`). This is **not the target key formula** and must be computed with native Rust path bytes for the pinned platform, not JavaScript UTF-16 or UTF-8 guesses.

Both ownership records are internal version-1 records carrying `workspace_root` and `updated_secs`. Their version guards are valuable for observation, but do not make them a supported public clean-plan API. `.mbx.toml` is read at W after root resolution (`cli/clean.rs:44-45`, `config.rs:1474-1478`); the allowed workspace policy controls build behavior, not cache/target roots. Invalid workspace policy can fail before cleanup. Bind its contents if designing a future executable adapter anyway.

### Source-confirmed mutation envelope for explicit clean

This table is the minimum confirmed set from the explicit clean path, including ancillary metadata. It is **not certified as the complete effect set for an arbitrary inherited process environment or Cargo wrapper**.

| Stage | Paths/effects | Exact source |
| --- | --- | --- |
| Workspace resolution | Executes effective `CARGO` or PATH `cargo`, with `metadata --no-deps --format-version 1 --manifest-path <requested/Cargo.toml>` when the probe cache misses. Current working directory remains relevant. There is no forced `--frozen`/`--offline` in the clean-generated arguments. Arbitrary wrapper effects and Cargo-side mutations are not constrained by the mbx clean interface. | `cli/clean.rs:33-42`; `cli/cache.rs:656-663`; `cli/cargo.rs:1246-1263`; `mbx-cache-cargo/src/lib.rs:801-818,841-870` |
| Probe metadata | May create `C/cargo-roots/v1`, write `.<digest>.json.<pid>`, then rename to `<digest>.json`. Version-2 record caches W/target/build roots, key inputs, executable/manifests/config length+mtime pins. Failed staging write may leave a partial staging file. | resolver `:559-575,579-616,645-780` |
| Incremental locks | May create `C/incremental/.locks` and global `registrar.lock`; holds its blocking fslock while checking per-workspace lease files. Opens `.locks/Ki/*.lease`, removes stale unlocked lease files when cleanup=true. **Even an Active return can already have removed stale lease files.** | `incremental.rs:295-408` |
| Private incremental state | Matching v1 `C/incremental/Ki/checkout.json` permits recursive removal of the whole `Ki` tree; cleanup also removes legacy `.locks/Ki/registrar.lock` and tries removing empty `.locks/Ki`. Global registrar remains. Sizing uses metadata that follows links, unlike a DevSweep no-follow walk. | `incremental.rs:221-244,340-435` |
| Managed target reservation | With matching record, creates/opens `T/v1/Kt.lock` and takes exclusive nonwaiting file lock. The lock file remains. No matching record usually means Missing. | `target.rs:285-324,369-392` |
| Managed target/record/link | Renames target to sibling `Kt.removing-<pid>-<random12>`, removes `Kt.json`, removes `W/target` only if `read_link` equals that target spelling, releases reservation, sizes and recursively removes the aside tree. A partial failure can leave the aside, removed record, and/or link in different states. | `target.rs:359-425,1205,1686` |
| Dangling target link exception | If record is missing/invalid and target does not exist, a `W/target` link exactly matching the derived destination is removed with **no target reservation**, returning Removed(0). Structural link deletion is a real effect even with zero bytes. | `target.rs:371-384` |
| Shared savings metadata | For each positive-byte private/target removal, creates/locks `S/savings/v1/tally.lock`, reads/updates/replaces `tally.json`; advisory writer creates a same-directory random `.mbx-*` temporary file. Errors are quietly logged; tally failure does not necessarily fail clean. | `cli/clean.rs:208-217`; `savings.rs:111-162,418`; `util.rs:152-167` |

The actions **objects/results/claims are not intentionally removed by these clean branches**. Calling this “shared cache untouched” is still inaccurate because savings metadata lives under the shared actions store. `cache remove`, `gc`, `clean --under`, manual unlink/delete and collection of abandoned removals remain outside the proposed workspace action. Clean itself does not perform the abandoned-removal sweep (`target.rs:1703`).

Startup also runs dispatch/environment recovery, phase-timing initialization and logging before the command (`main.rs:3-52`). Those alternate inherited-environment paths were not exhaustively audited. An executable adapter would need a closed, reviewed environment and direct executable identity, not simply a narrowed argv. No claim is made here that Cargo metadata necessarily modifies this particular project; the missing guarantee is enough to prevent an “only these paths” assertion.

### Leases, partial results and why a saved digest is insufficient today

1. Private incremental and target cleanup are separate sequential operations. An incremental error or Active outcome does not prevent the target attempt (`cli/clean.rs:162-205`). Only report errors cause failure; warnings still return success (`:47-52`). Exit 0 cannot mean all selected bytes were removed.
2. mbx target leases protect cooperating mbx commands. `target::remove_workspace` does **not** call `cargo_locks`; that helper at `target.rs:1020` exists on other paths. Thus the current clean path does not establish protection against an unwrapped plain Cargo build using the link/target. Keep DevSweep's existing busy protections; do not infer a general busy guarantee from “mbx has leases.”
3. Target and private records are read before reservation/guard acquisition, without an expected record digest supplied by DevSweep. A pre-dispatch comparison cannot make the child atomically rebind the reviewed records, links, configuration and protected descendants. Holding the same exclusive reservation in DevSweep would cause clean to report Active; holding the incremental registrar would block its guard. There is no handoff/token interface.
4. A saved preview must describe all listed structural/metadata effects and conditional operations, including zero-byte link removal, stale lease removal and global savings updates. It must also verify all relevant protections, roots/ancestor identities, executable/config/Cargo inputs, and current owner/lease state. Existing generic target-path binding does not represent this envelope.
5. A timeout or nonzero exit cannot safely be retried as though nothing happened. The target may already be renamed and records/link removed. A new owner build can recreate a target after mbx releases its reservation but before removal finishes. Structured post-state disposition is needed to distinguish kept, removed, recreated, partially removed and unknown; current human output is insufficient.

### Safe observation recipe for B, and inputs for C

This is a bounded **non-authoritative observation** recipe. It can qualify owned logical footprint; it never emits an executable cleanup intent.

1. Probe version/path/config through the existing bounded command runner. Accept the reviewed **exact 1.24.0 contract** only; unsupported version/config is a distinct status. `cache dir --json` must be valid version 1 and its store must match resolved `C/actions`. Do not use aggregate stats or enumerate all cache projects. Do not read secrets or dump full environment/config into reports.
2. Start only from user-selected project/workspace roots and existing Cargo metadata ownership, keeping both canonical filesystem identity and the exact Cargo-reported spelling used by mbx. Failed/ambiguous Cargo ownership stays unresolved. Multiple requested members are one workspace observation. Do not call `mbx clean` to discover ownership. Do not silently accept fallback requested-root identity as Cargo proof.
3. Verify configured C/T and every traversed ancestor with no-follow metadata. Reject reparse/symlink ancestors and unsupported root aliases before descending. Bound reads of the two directly derived v1 ownership records; require exact W, expected key and version. Arbitrary target links, a hash-shaped directory alone or human project-table text prove nothing.
4. Inspect `W/target` as an entry with no-follow metadata and `read_link`, after proving W's ancestor chain. This reads the link value without traversing it. Normalize only for comparison inside a proven boundary; preserve the raw value as evidence. Unknown link type, relative/unexpected destination, changed identity, missing record or external/unverified root yields an unresolved association. Do not follow the link to size it.
5. For a recognized association, independently open the derived real managed directory under validated T, check it is an ordinary non-reparse directory, and apply DevSweep's bounded no-follow size walker there. Do the same for the matching private Ki directory under C/incremental. Recheck record/root/link identities after reads; changes make the observation stale/partial. A recognized recorded view can exist without W/target (Cargo can point at it directly); distinguish `recorded_view` from `matching_project_link` rather than requiring a link for every observation.
6. Correlate record+key+workspace+validated root+actual directory identity to mark an **observation owner as verified**, while keeping `cleanup_supported=false`. If any ownership step is missing, mark trusted owner false and exclude it from the exact project subtotal. Unsupported versions can show the link fact/path and reason, but no attributed bytes. This verification is not cleanup authorization.
7. Dedupe matching Cargo target, mbx target, project link alias and repeated workspace roots by canonical directory/file identity. Count target bytes once under W; do not count link bytes as another target. If two owners overlap, resolve the actual owner or report ambiguity and exclude exact attribution. Keep a source/alias list so the UI can explain suppression. Do not hide a separate supported generic action merely because an observation alias exists; generic symlink/reparse rejection remains intact.
8. Count only verified private target/build/test/tool-cache bytes toward C's project artifact subtotal. Never attribute S/actions, action claims or aggregate stats to one project. Dependencies and venv remain outside this subtotal. `logical bytes` are not physical reclaim: cross-tree hardlinks/shared allocation require an uncertainty flag, not a “free this amount” promise. Threshold is inclusive **5,368,709,120 bytes** by default, only after a user scan.

Suggested DTO semantics for implementers (field names are proposals, not existing schema): `owner_root`, exact `workspace_key_input`, `provider/version`, `artifact_kind`, `storage_path`, `storage_identity`, `alias_paths`, `ownership_evidence`, `owner_verified`, `association_status`, `logical_bytes: Option<u64>`, `size_completeness`, `overlap_status`, `shared_allocation_unknown`, `cleanup_supported=false`, and stable `reason_code`. Preserve `missing_tool`, `unsupported_version`, `unresolved_config`, `owner_unresolved`, `unrecognized_link`, `record_missing`, `record_invalid`, `record_mismatch`, `changed_during_scan`, `size_partial`, `verified_inspect_only` separately from zero bytes. Do not expose session contents or privileged mutation arguments.

For C: only `owner_verified && nonambiguous` observations contribute; incomplete sizing remains a lower bound/partial total, never an exact total or proof the project is below threshold. A lower bound already >= threshold can truthfully warn with partial-size wording. Unknown sizes remain unknown. Selection cannot alter occupancy totals. Scope of warning stays selected project roots; no background scan, no automatic clean.

### Exact open criteria and tests

- Owner contract: versioned read-only per-workspace report with canonical identity plus mbx spelling, effective roots, private/generated states, matching link and all ancillary mutation paths. No global sweep and no undocumented JSON flags.
- Execute contract: owner-side expected-state revalidation/lease transaction or proven handoff, protection checks covering every possible mutation, safe handling of plain Cargo users, symlink/reparse changes, reviewed subprocess/environment behavior, explicit structural-link disclosure and complete machine result disposition.
- Conformance: separately authorized disposable cache/workspace/home, pinned executable hash, before/after filesystem snapshot including metadata/locks, unrelated workspace and shared object sentinels, supported and unsupported config roots, active target lease + incremental lease + plain Cargo lock, missing/stale records, protected link/metadata, changed link/record, injected partial errors and timeout. No real user cache cleanup. Mocks are supplemental.
- Observation fixtures now: exact native key fixtures (Windows and Unix separately), member/workspace dedupe, relocated target root, absent/malformed/mismatched records, arbitrary symlink/junction/root reparse, traversal budget/permission failure, changing identities, Cargo/mbx alias overlap, shared store never attributed, unknown not zero, verified owner with unavailable cleanup, and parity through CLI/Tauri DTO serialization. Existing process-name busy checks must not be weakened for this observation work.

## Files and primary source references

Repository context: parent `prd.md`, `design.md`, `task-map.md`; child `.trellis/tasks/10-14-project-artifacts-mbx/{prd,design,implement}.md`; child C design/implementation; earlier `research/scan-contracts.md`. Relevant specs remain backend ownership/rules, provider execution, safety, scan coverage and model wire compatibility, as linked in the earlier report.

Pinned upstream files (links lead to the audited commit; blob SHA independently identifies the fetched file):

| File | Purpose | Git blob SHA |
| --- | --- | --- |
| [cli/clean.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/cli/clean.rs#L28) | Dispatch, partial outcomes, savings | `43a066ef3f8710bfb7ef2c70af7530ae4949d5b8` |
| [target.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/target.rs#L369) | Managed target, locks, records, link/aside deletion | `d8e5216d4c6e47cd7f6fc405d7fe0166d536cac9` |
| [incremental.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/incremental.rs#L221) | Private-state and lease mutations | `7421fbce9920e3cc62a999dc1a4400624abf2099` |
| [savings.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/savings.rs#L111) | Shared tally mutation | `b8d65e5e3638449adce133d29880986498072c81` |
| [util.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/util.rs#L152) | Advisory temp/replace behavior | `99eb575bbc02852887fcc6b9f38f2cb85db67f82` |
| [cli/cache.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/cli/cache.rs#L119) | Public reports and workspace resolution | `3b85796bbb3c278a9dd45c0a371058f65001a6cc` |
| [config.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/config.rs#L1225) | Effective cache/target roots | `a9725a8f9f536bd394eda517a4969012bcdd704f` |
| [mbx-cache-cargo/src/lib.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx-cache-cargo/src/lib.rs#L559) | Cargo probe and persistent probe metadata | `c26d071e595af69f11b6a148768e8dba28964557` |
| [cli/cargo.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/cli/cargo.rs#L1246) | Wrapper into resolver | `1947131a9cc6d8915566c2408c32701620597814` |
| [cli/settings.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/cli/settings.rs#L123) | Scalar setting output | `59002720393ccbffb93f825541f3136fd3e1dcc9` |
| [cli/mod.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/cli/mod.rs#L259) | CLI config/dispatch | `c9c7675a3604fa876e621262f55b0d7665a149fe` |
| [main.rs](https://github.com/jdx/mr-boxington/blob/1b62dc2c795c93630d4ce28c4d9d2540d6e776ea/crates/mbx/src/main.rs#L3) | Pre-command environment dispatch | `36bcbb7f57b624a9d53d5de4a6f0c3054ff296d4` |

## Caveats / Not Found

- Source-confirmed behavior is not installed-binary conformance. Actual pinned clean, lease concurrency, no-follow sizing implementation and native Windows filesystem behavior are **NOT_RUN** here.
- No stable public per-project ownership/footprint or clean expected-state/result contract was found in 1.24.0. Internal versioned records can support guarded observation only.
- Complete startup, custom Cargo wrappers, Cargo metadata effects and arbitrary inherited mbx session environment are not certified. The mutation table explicitly records a confirmed lower bound, not a false complete list.
- This report changes no product code or task status and does not authorize user cache cleanup. The pre-existing desktop Cargo manifest edit remains untouched.
