# Research: C01 strict saved-plan wrappers

- Query: What must change before the existing startup toggle, Analyze actions, and leftover plan flow can safely consume saved CLI input?
- Scope: internal, narrow source and contract review; no implementation or test execution.
- Date: 2026-10-15
- Status: technical preparation while C16 validation is active. Proposed fields/codes below are recommendations for C01's technical freeze, not existing public contracts.

## Findings

### Conclusion

The three flows need domain-specific boundaries, not a new general service framework. Startup needs a new plan/preview/execute API with private live registry/file evidence. Analyze needs strict saved-snapshot validation and preview-to-execute filesystem identity binding before its crate-private trusted plan constructor is used. Leftovers can share a small extracted live-plan builder, but the current saved labels are only candidate-search hints and the current digest does not identify the live directory object. A standalone preview must not imply stronger provenance than that.

Two compatibility decisions must be frozen explicitly: durable pre-dispatch support auditing cannot be added to the closed Software V1 journal without considering older readers; and the existing leftover schema has no authenticated pre-uninstall display-name/publisher evidence. Merely matching a saved inventory hash to the uninstall audit does **not** repair the latter.

### Files found and relevant contracts

| Source | Role |
| --- | --- |
| `crates/devsweep-core/src/software/startup.rs:274` | Public list/bool-toggle API; source/name IDs, native approval read/write, isolated registry tests. |
| `crates/devsweep-core/src/software/arp.rs:474` | Existing typed registry reads; bounded raw value reader is private at line 558. |
| `crates/devsweep-core/src/software/leftovers.rs:387` | Discovery, plan construction, live rediscovery, selection, digest, trash execution. |
| `crates/devsweep-core/src/software/execution/audit.rs:100` | Closed Software V1 uninstall records and terminal-only support records. |
| `crates/devsweep-core/src/software/plan.rs:147` | Saved uninstall plans are revalidated against fresh inventory; prior inventory fingerprint remains caller-supplied plan context. |
| `crates/devsweep-core/src/analysis/actions.rs:288` | Path reconstruction, refusal checks, trusted Clean plan construction and execution. |
| `crates/devsweep-core/src/analysis/model.rs:129` | Deserializable snapshot, root, node, completeness and evidence DTOs; no persisted object identity. |
| `crates/devsweep-core/src/filesystem/identity.rs:56` | Existing crate-private `PathIdentity` and live Windows file-ID capture. |
| `crates/devsweep-core/src/plan/mod.rs:209` | `validated_trash_plan` assumes core-retained snapshot provenance. |
| `crates/devsweep-core/src/plan/digest.rs:84` | Clean canonical target includes path/size/mtime but excludes `PathIdentity`. |
| `crates/devsweep-core/src/execution/safety/mod.rs:188` | Live authorization, protected paths, identity rechecks. |
| `crates/devsweep-core/src/execution/mod.rs:824` | Clean durable `started` barrier before side effect. |
| `desktop/src-tauri/src/analyze.rs:108` | Retained snapshot/operation binding that a standalone CLI process does not inherit. |
| `desktop/src-tauri/src/software.rs:405` | Existing IPC accepts an inventory or leftover plan supplied by the caller; it is not an authenticated retained-inventory store. |

Related specifications: backend `quality-guidelines.md:73-105` (create-new outputs, exit classes, saved plan/live digest/confirmation); `:423-499` (Software support restrictions and tests); `:515-615` (Analyze retained snapshot, trusted-only plan seam, digest/refusals/audit); backend `database-guidelines.md:35-38,54-66` (validated plans, persistence tests, Software lock/sync/privacy). C01 `research/cli-entry-points.md:31-55` adds the stricter saved-input wrapper requirements. Preserve the current explicit ban on permanent delete and arbitrary cleanup commands.

## 1. Startup: source/name identity is not live entry identity

### What the current API actually guarantees

- `startup.rs:339` hashes only location and entry name. A replaced Run value or shortcut at the same name has the same ID.
- `set_with` checks confirmation, fresh enumeration, known ID and current-user eligibility, then opens the Software journal before writing (`:347-405`). It writes only fixed `StartupApproved` keys, re-reads the state and emits a terminal support record.
- Opening/locking the journal precedes the mutation; durable action intent does not. The registry write is at `:376`, the support append at `:397`. A crash or append failure can leave a changed approval without a persisted action start.
- Native enumeration reads Run value names, not their contents (`:459-497`); Startup-folder enumeration retains names of non-directory entries (`:500-557`). Approval decoding is permissive and collapses raw bytes into enabled/disabled/unknown (`:428`, `:560-573`). Different approval bytes can have the same visible state.
- HKLM stays view-only. No new wrapper should accept caller-supplied registry key, executable, command, argv, or shortcut target.

### Minimal proposed boundary

Add closed `StartupTogglePlanV1` with `version`, `entry_id`, `requested_enabled`, `expected_entry_identity_sha256`, and `expected_approval_identity_sha256`. Creation observes the live entry; preview independently observes it again and returns the existing redacted row plus before/after state and a domain-separated `sha256:` digest. An operation ID can be allocated at execution and durably recorded before dispatch; no need for the caller to supply registry coordinates.

Private `observe_startup_entry(specs, entry_id)` should return a typed observation with:

- fixed location/hive/view/source identity and exact name;
- Run value type plus raw value bytes, hashed privately without expanding or executing them;
- for Startup-folder entries, safe no-follow file identity plus bounded content/metadata evidence sufficient to detect replacement/edit; missing identity or unreadable evidence refuses strict execution;
- raw approval value presence/type/bytes, distinguishing missing (currently enabled) from malformed/unreadable;
- source completeness and state. An unknown or incomplete source must not become a writable proof.

Reuse the bounded raw registry reader in `arp.rs:558` through a narrow crate-private observation helper; using `string()` would discard the original value type/trailing-byte distinctions. Do not expose raw evidence in plans, JSON output or audit.

Execute must validate saved fields, acquire the existing process/Software-lock boundary, re-observe and compare both expected identities and the live digest, persist a durable start, then recheck immediately before the fixed approval write and read back. A wrapper that validates and then calls public `set_startup_enabled(..., true)` has an unchecked re-enumeration gap and loses the captured evidence; use a private shared dispatch seam instead.

Recommended new refusal codes: `unsupported_plan_version`, `invalid_plan`, `startup_source_unavailable`, `startup_identity_unavailable`, `startup_entry_changed`, `startup_state_changed`, `stale_confirmation`, `audit_unavailable`. Reuse existing unknown-entry, machine/requires-administrator, unsupported-platform and confirmation semantics. Post-write readback or terminal-audit failure must report failure/unknown after dispatch, not a retry-safe preflight refusal.

Same-state requests need a frozen policy: recommend a no-write `already_in_state` result after successful validation, with no fabricated toggle success. A registry read/write sequence cannot provide a transactional guarantee against an external writer; document bounded immediate revalidation and readback rather than claiming atomic compare-and-swap.

### Compatibility and tests

Keep existing list/report V1 and bool API signatures while Tauri consumers migrate. Its legacy behavior is not proof that the new strict CLI contract is satisfied. Internal observation/dispatch helpers can be shared without allowing the CLI to bypass the strict wrapper.

Reuse tests at `startup.rs:637,655,674,696` and temporary-HKCU `temporary_key_lists_decodes_toggles_and_audits_only_approval_values` at `:796`. Add same-name Run-value replacement, raw approval drift without state change, shortcut replacement, unreadable/malformed evidence, missing digest/confirmation, durable-start failure (zero writes), readback mismatch and terminal-audit failure. Use only temporary test keys/files and injected journal I/O.

## 2. Analyze: saved snapshots need a stricter boundary than retained snapshots

### Existing guarantees and gaps

The Tauri coordinator retains the terminal snapshot behind its operation ID (`analyze.rs:108-137`); clients send IDs. Core rebuilds paths and applies protections/reparse/type/mtime checks, removes nested descendant selections and constructs a private `analyze.trash` Clean plan. General Clean plan files cannot import that rule (`actions.rs:944` test).

Deserialization into `AnalyzeSnapshotV1` is not equivalent to that retained provenance:

- `NodeIndex::new` collects into a hash map, silently replacing duplicate node IDs (`actions.rs:289-293`); the snapshot version, global tree shape and advertised budget/counts are not validated here.
- `snapshot_base` accepts the normalized-path fallback even when input and normalized roots disagree (`:349-364`). The root's stored volume string is not live object identity.
- `plain_name` rejects separators/dot segments but not Windows prefix/colon/ADS forms (`:368-369`). Require one normal platform component and explicit normalized containment; do not assume every deserialized string is a walker-produced filename.
- Duplicate selection IDs are silently ignored (`:513-515`); unknown selections become item refusals. Strict saved plans should reject duplicate IDs rather than silently rewrite intent.
- Type and mtime are checked, but `mtime_ms: None` bypasses timestamp comparison (`:468-491`); selected-node `Incomplete`/`Unknown` evidence is not a refusal by itself.
- `validated_trash_plan` captures object identity with `.ok()` (`plan/mod.rs:247`). Clean authorization later checks that execution-time identity, but the confirmation digest excludes it. Preview-to-execute replacement at the same path/type/mtime is therefore not bound by the existing digest. The snapshot DTO also lacks object identity. Do not describe this as no identity checks at all: execution does revalidate the identity captured when its in-memory plan was built.

### Minimal proposed saved contract and private seams

Use one closed `AnalyzeTrashPlanV1` containing `version`, `operation_id`, `snapshot` (existing V1 DTO), `selected_node_ids`, and frozen root/selected-node identity bindings captured during plan creation. Embed the snapshot to avoid a second mutable file reference. Bindings can expose a domain-separated hash of private `PathIdentity` rather than widening that internal type. Include selected node ID, kind and required observed mtime; hash the exact snapshot content/selection as part of the plan's canonical intent.

Introduce `validate_saved_snapshot` and a private validated-selection type before reaching `validated_trash_plan`. Required checks:

1. Supported versions, input byte limit before deserializing, existing node/warning/depth ceilings, independently computed memory/count checks; do not trust `accounted_owned_bytes`.
2. Exactly one directory root ID 0, unique node IDs, valid existing parents, no cycles/orphans, no file/reparse parents, and bounded single-component names. Validate root input/normalization/volume consistency and every rebuilt selected path's normalized containment.
3. Nonempty unique selection; known IDs; ancestor collapsing remains deliberate and is shown in preview. Reject malformed snapshots globally rather than letting map order select an alternate node.
4. Read-only reveal may use structurally valid partial snapshots. For mutation, refuse selected `Incomplete`/`Unknown` nodes or unavailable identity/mtime. A complete selected subtree in an otherwise partial snapshot can remain eligible; do not unnecessarily deny unrelated complete nodes.
5. Capture root and selected objects with existing no-follow inspection plus `capture_path_identity`; compare frozen identity at preview and execute. Re-evaluate live protection and refusal rules. Explicitly refuse unavailable identity instead of converting it to `None`.

The strict preview digest should bind canonical saved intent, current root/selected object bindings and the underlying Clean confirmation digest. Keep this a new Analyze wrapper digest rather than changing all Clean V1 digest semantics. A private execute seam can check that strict digest and frozen identities, construct the validated Clean plan once from the same observations, then call the existing Executor. It must not call the old public execute path if doing so discards validated bindings and reconstructs an unbound plan.

Preserve the existing Clean durable-start journal and protections. This does not require a second Analyze mutation journal. Selection/refusal changes after preview must cause `stale_confirmation` before any partial dispatch; do not silently execute a smaller set. Capture/display directory scope honestly: a directory object ID does not freeze every descendant's content. Full recursive content manifests are not part of the existing behavior and should not be invented in C01.

Recommended additional codes: `unsupported_snapshot_version`, `invalid_snapshot`, `duplicate_node_id`, `duplicate_selection`, `snapshot_limit_exceeded`, `snapshot_root_mismatch`, `path_outside_root`, `incomplete_observation`, `identity_unavailable`, `changed_since_snapshot`; preserve existing protected/system/root/reparse/not-found codes and `stale_confirmation`. Structural malformed input is usage/invalid-input; structurally valid but stale identity is stale-authority exit 3; unavailable native capability is exit 4.

### Compatibility and tests

Keep read-only snapshot V1 and current retained-snapshot APIs. New saved-plan semantics get a distinct DTO; no serialized `ValidatedPlan`, `CleanAction`, `analyze.trash` rule or display-path override is accepted. Unsupported platforms must return a typed unsupported response for reveal/native recycle rather than attempting Explorer everywhere.

Reuse `actions.rs` tests at `:752` (paths/cycles), `:794` (refusals), `:944` (no imported Analyze rule), `:973` (nested selection), `:996` (digest/operation staleness), `:1045` (fake trash + Clean audit), and Tauri retained-operation test `analyze.rs:295`. Add duplicate nodes/selection, future versions, mismatched roots, colon/prefix/ADS components, orphan/cyclic trees, forged budget fields, selected partial evidence, missing identity/mtime, root replacement, same-path same-mtime object replacement, and protection change. Reuse Clean `started_sync_failure_blocks_side_effect` (`execution/mod.rs:2418`) and `terminal_audit_failure_reports_unknown_after_side_effect` (`:2480`) as behavioral references.

## 3. Leftovers: share live preview, preserve uncertainty, close evidence gaps

### What is already reusable

`SoftwareLeftoverPlanV1` is a closed plan with software ID/identity, display name/publisher, succeeded uninstall operation ID and selected candidate IDs (`leftovers.rs:126`). It does not accept arbitrary candidate paths. Execution validates version and software ID, requires a matching successful terminal uninstall in the fixed locked journal, rediscovers candidates, rejects duplicate/empty/stale selection, verifies the digest, rechecks exclusions/reparse safety and moves to trash (`:538-614`).

Extract the validation/rediscovery/selection/digest portion into a private live-plan builder used by both standalone `preview_software_leftover_plan` and execution. A preview must never call execute with confirmation disabled. The builder should take a read-only audit view, production environment and cancel observer; the public API must not accept an audit path or a caller-supplied claim that uninstall succeeded.

`SoftwareAuditJournal::open` creates the directory, lock and append file (`audit.rs:437-461`). A new read-only preview should not create those merely to discover no succeeded uninstall. Factor/reuse parsing into a read-only audit inspection path; execution must still re-open, lock, revalidate and refuse corruption/unknown versions. Preview's successful audit lookup is not an execution permit.

### Two facts that the current V1 digest does not prove

1. **Label provenance:** discovery trusts `plan.display_name` and `plan.publisher` as exact-name search hints (`leftovers.rs:555-562,697`). Candidate IDs hash software ID, origin and normalized path (`:804`); plan digest includes the supplied labels and candidate path/origin/certainty (`:680-694`). Thus a saved caller can substitute another permitted exact-name folder hint and recompute a new selection/preview. Existing exclusions still apply and name matches remain uncertain/unselected by default, but the succeeded uninstall does not authenticate ownership of that folder.
2. **Live object continuity:** the digest excludes directory file ID, timestamp and size, while the immediate `live_refusal` checks only safe existing directory/exclusions (`:765`). Replacing a directory at the same candidate path does not necessarily change the digest. Estimated size is not the object's identity and is explicitly lower-bound evidence where applicable.

The audit is not an overlooked metadata store: uninstall V1 records contain identity, inventory fingerprint and preview digest, not display name/publisher/install location (`audit.rs:96-120`). Worse, `preview_selection_plan_live` intentionally preserves the saved plan's prior inventory fingerprint (`plan.rs:147-179`), and the executor records that fingerprint after rebuilding only exact MSIX strategy from live inventory (`execution/mod.rs:720-758`). Comparing a supplied inventory to that audit fingerprint cannot establish that its labels were observed live. The live uninstall preview's selected item also omits those labels (`plan.rs:258-265`).

### Minimum safe choices to freeze

- If C01 exposes unchanged V1 name-based behavior, explicitly model the labels as untrusted discovery hints and the result as uncertain, user-reviewed candidate intent. Keep selected-by-default false and require the new live preview. Do not call that authenticated app-owned cleanup, and do not upgrade certainty from a matching uninstall record. This is compatibility, not a provenance fix.
- If strict C01 execution is intended to require proven association, add a core-produced pre-uninstall subject/candidate receipt tied to exact live software identity and uninstall operation. Persist it before removal; use its digest/ID in a new strict leftover plan and reject old plans lacking evidence with `leftover_subject_unverified`. Capture label/install-location binding from live inventory/provider data, not saved JSON. That needs a narrow seam in uninstall execution and a separately versioned evidence store; freeze it explicitly rather than silently expanding V1.
- Whichever ownership policy is chosen, new strict preview/execute should bind each selected candidate to current no-follow object identity and compare it again before moving. Reuse private `PathIdentity`/capture helpers. Return `candidate_changed` or `identity_unavailable` rather than continuing with a replacement. A V2 plan/preview or distinct strict wrapper can add these bindings without redefining V1 digest meaning.
- The certain ARP install-location path is queried live, but supported uninstall execution is currently current-user MSIX; there is no general ARP/MSI succeeded-uninstall path to assume for C01 (`software/mod.rs:323-336`, `execution/mod.rs:732-750`). Broader providers remain C05.

Reuse existing errors for unsupported version, invalid identity, empty/duplicate/stale candidates, missing succeeded uninstall, digest mismatch and protection failure. New proposed codes: `leftover_subject_unverified` (only if proven association becomes required), `candidate_changed`, `identity_unavailable`, `audit_unavailable`, `canceled`. Post-dispatch terminal-audit failure must be reported as unknown/failed after dispatch with no automatic retry.

Tests to reuse: `leftovers.rs:1049` certainty/default selection, `:1105` exclusions, `:1159` partial/unknown size evidence, `:1196` succeeded-uninstall/confirmation/digest/fake-trash/audit, `:1363` invalid software ID/stale candidate. Add standalone preview parity, absent/corrupt audit without new files, cancellation before authority, edited labels with self-consistent hashes, directory replacement after preview, fail-closed identity capture, and durable-start/terminal-audit failure. Fake trash must record zero calls for all pre-dispatch refusals.

## 4. Shared compatibility and audit decision

Startup and leftover support records are terminal-only, closed tagged variants (`audit.rs:151-186`); the parser rejects unknown versions/variants (`:560-575`). Appending a new `started` variant or new required fields to `software.jsonl` can make an older build refuse the entire journal. Neither existing support API currently offers a durable pre-dispatch receipt. Clean already has that mechanism; do not weaken it to match support behavior.

Recommended freeze: preserve the old journal shape and define a narrowly scoped, fixed-path versioned support-operation journal/evidence store for strict startup/leftover lifecycle records, using the existing Software lock to serialize relevant execution. Alternative: explicitly version/migrate the Software journal and document downgrade behavior. This choice belongs to the implementation/spec owner. Simply emitting a terminal `failed` V1 record before dispatch is semantically false and must not be used as an intent record.

A strict support lifecycle needs a durable start with operation/plan/preview identity, a terminal result and recovery behavior. Unfinished actions remain unknown after dispatch and can only be re-observed; never automatically repeat a registry write or trash move. Keep paths, registry bytes and command lines out of support audit records. A compact durable metadata receipt, if chosen for leftovers, is a separate privacy-reviewed evidence schema, not extra fields smuggled into the existing audit.

Across all new routes: strict bounded deserialization, domain version validation, create-new output preflight before mutation, no CLI-supplied audit paths, no implicit privilege escalation, and machine-stable errors. Missing plan/digest/confirmation is usage exit 2; changed authority is exit 3; unavailable sources/permissions is exit 4; truthful partial results are exit 5; uncertain dispatch/persistence is exit 6; cancellation before dispatch is 130. A digest is an integrity/confirmation binding, not a signature or proof of origin.

## Remaining technical decisions

1. Freeze the strict support audit location/version/recovery format and compatibility behavior before startup/leftover mutation work.
2. Decide whether C01 preserves explicitly uncertain name-hint leftovers or requires a core-produced pre-uninstall association receipt. Existing audit hashes cannot substitute for that decision.
3. Freeze Analyze saved-plan size limits, canonical identity/digest encoding, selected-incomplete behavior and root/volume validation. Reuse existing walker ceilings rather than trusting saved counters.
4. Freeze startup unknown/raw-value handling, bounded shortcut evidence and same-state no-op semantics. Reversible restore/backup remains C05; identity hashing alone is not a restore backup.

## Caveats / Not Found

- This is source analysis, not an executed exploit, test result, native registry/trash acceptance or atomicity proof. No tests, builds, installs, product edits or native actions were run.
- C16 is concurrently adapting dependencies; line anchors describe the inspected source and can shift. Function names and behaviors should be rechecked at C01's implementation freeze.
- The inspected trash and registry interfaces are path/key based. Immediate live checks narrow drift windows but do not prove a transaction against unrelated external writers.
- External references: none needed for this internal contract review. The findings do not assert current upstream OS/library behavior beyond the inspected repository code.
