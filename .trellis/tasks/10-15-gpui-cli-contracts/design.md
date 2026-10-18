# C01 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Prefer domain adapters over a new generic service framework. Reuse the current scan/cancel/plan validators. Proposed CLI grammar is recorded in the parent research/cli-contract-proposal.md; that document distinguishes existing core APIs from new execution support.

## Ownership

- `crates/devsweep-core/src/services.rs`
- `crates/devsweep-core/src/software/`
- `crates/devsweep-core/src/analysis/`
- `crates/devsweep-cli/src/application/`
- `resources/i18n/`
- `docs/reference/cli.md`
- `docs/zh/reference/cli.md`

These paths identify responsibility, not permission to refactor every file.
One owner edits a shared file at a time. Workers are not alone in this repository
and must not revert other workers' changes. Main session owns cross-child design,
spec updates and integration decisions.

## Data flow and compatibility

CLI request -> domain observation/plan -> live validation -> explicitly confirmed execution where supported -> durable audit/read-back -> typed output. Discovery never mutates system state.

Preserve existing supported schemas, command semantics, refusal and cancellation.
Version incompatible shapes and consult the parent CLI proposal. Rust native views
call shared services, rather than parsing CLI text or cloning domain logic.
Follow the latest-compatible dependency policy; any older required pin or compiler
compatibility change is documented and tested.

## Risks and rollback

Identity drift, partial observations, late worker results and broadening authority
are relevant failure cases. GPUI-specific native accessibility/IME/graphics are
unproven until C09. Native uninstaller/helper/provider behavior cannot be inferred
from fixture success alone. Keep operation-specific close/cancel semantics.

Retain existing CLI/Tauri paths while this child is developed. Roll back isolated
adapter changes without changing unrelated work or historical evidence. Installation,
publishing and Tauri retirement are not implicit consequences of passing tests.

## Technical freeze (2026-10-17)

Source of the freeze: `research/cli-entry-points.md`, `research/strict-wrapper-findings.md`, and the parent `research/cli-contract-proposal.md`. Line numbers in those notes can move. Recheck function names before editing. This freeze is the C01 implementation authority. Do not reopen the four decisions below inside the implementation.

C16-AC3 hosted matrix and local Rust 1.88.0 stay FAIL or NOT_RUN. They are not PASS. Local `just ci` on rustc 1.99.0 exited 0 on 2026-10-17. Push is forbidden, so that hosted run cannot be replaced. C01 proceeds on the current working tree. Do not revert the uncommitted Software Unix `LOCK_UN` drop, the Unix provider-fixture canonical root, the `justfile` `--target-dir dist/ci-target` recipes, or the provider sentinel borrow fix.

### Additive CLI

Keep every existing leaf. C01 adds only:

- `software updates check` (read-only). No update plan, preview, or execute.
- `software startup list`, `plan`, `preview`, `execute`. No `restore-plan`.
- `software leftovers scan`, `plan`, `preview`, `execute`.
- `analyze reveal`, `trash-plan`, `trash-preview`, `trash-execute`.

`analyze roots`, `list`, and `explain` stay with C07. `optimize health` stays with C06. `status keep-awake` stays with C08. No command accepts an audit path, a registry key, an executable, argv, or a caller claim that uninstall succeeded.

### 1. Startup live identity

Closed `StartupTogglePlanV1`: `version`, `entry_id`, `requested_enabled`, `expected_entry_identity_sha256`, `expected_approval_identity_sha256`. Creation observes the live entry. Preview observes it again and returns the existing redacted row, before/after state, and a domain-separated `sha256:` digest. The caller does not supply registry coordinates or an operation id.

Private observation hashes Run value type and raw bytes, and Startup-folder no-follow identity plus bounded content/metadata. Missing or unreadable evidence refuses strict execution. Raw approval presence, type, and bytes stay private. Unknown or incomplete source is not writable proof. HKLM stays view-only.

Execute validates the saved plan, takes the existing Software lock, re-observes, compares both identities and the live digest, writes a durable start, rechecks immediately before the fixed approval write, then reads back. Do not validate and then call public `set_startup_enabled`. Use one private dispatch seam.

After successful validation, a request whose live state is already the requested state returns `already_in_state` and does not write. Do not report a fabricated toggle success. The registry sequence is not an atomic compare-and-swap.

Plans, JSON, and audit records do not contain raw command lines, registry bytes, or paths. The legacy bool API remains for current Tauri callers.

Refusal codes to add when no existing code applies: `unsupported_plan_version`, `invalid_plan`, `startup_source_unavailable`, `startup_identity_unavailable`, `startup_entry_changed`, `startup_state_changed`, `stale_confirmation`, `audit_unavailable`. Reuse unknown-entry, machine/administrator, unsupported-platform, and confirmation results. Post-write readback or terminal-audit failure is failure/unknown after dispatch, with no automatic retry.

### 2. Analyze saved input

Closed `AnalyzeTrashPlanV1`: `version`, `operation_id`, embedded `AnalyzeSnapshotV1`, `selected_node_ids`, and frozen root/selected-node identity bindings. Bind a domain-separated hash of private `PathIdentity`. Do not widen `PathIdentity` into the public DTO. Each selected binding includes node id, kind, and required observed mtime. The plan hash covers exact snapshot bytes and selection.

`validate_saved_snapshot` runs before `validated_trash_plan`. Enforce supported versions, a byte limit before deserialize, and the existing node/warning/depth ceilings. Recompute counts. Do not trust `accounted_owned_bytes`. Require one directory root id 0, unique node ids, valid parents, no cycles or orphans, no file or reparse parents, and one normal platform path component. Reject Windows prefix, colon, and ADS forms. Reject duplicate selection ids. Unknown ids and malformed structure fail the whole plan.

Reveal may use a structurally valid partial snapshot. Mutation refuses a selected `Incomplete` or `Unknown` node, or a selected node with unavailable identity or mtime. A complete selected node can remain eligible when unrelated nodes are partial. Ancestor collapsing stays visible in preview. A directory object id does not freeze descendant content. Do not add a recursive content manifest.

The strict preview digest binds saved intent, current root/selected object bindings, and the underlying Clean confirmation digest. Do not change Clean V1 digest meaning. Execute checks that digest and the frozen identities, builds one validated Clean plan, and calls the existing executor. Do not call a public execute path that drops those bindings. Selection or refusal changes after preview return `stale_confirmation` before any partial dispatch.

No second Analyze mutation journal. Keep the Clean durable-start journal. Do not accept a serialized `ValidatedPlan`, `CleanAction`, `analyze.trash` rule, or display-path override.

Additional codes: `unsupported_snapshot_version`, `invalid_snapshot`, `duplicate_node_id`, `duplicate_selection`, `snapshot_limit_exceeded`, `snapshot_root_mismatch`, `path_outside_root`, `incomplete_observation`, `identity_unavailable`, `changed_since_snapshot`. Keep existing protected, system, root, reparse, and not-found codes. Malformed input is usage exit 2. Stale identity is exit 3. Unavailable native capability is exit 4.

### 3. Leftover ownership

C01 keeps name-based discovery as untrusted hints. `display_name` and `publisher` are exact-name search hints, not authenticated ownership. A matching succeeded uninstall audit does not prove the folder belongs to that software and does not raise certainty. Candidates stay unselected by default. The result is uncertain, user-reviewed candidate intent.

Do not add a pre-uninstall association receipt in C01. Do not add `leftover_subject_unverified`. Broader proven ownership stays with C05.

Add standalone `preview_software_leftover_plan` through the same private live-plan builder as execution. Preview never calls execute. The public API does not accept an audit path or a caller-supplied uninstall-success claim. Preview lookup is read-only and must not create the journal directory, lock, or append file. Execution re-opens, locks, and revalidates.

The new strict preview and execute bind each selected candidate to current no-follow `PathIdentity` and compare it again before trash. Do not redefine the V1 digest. Return `candidate_changed` or `identity_unavailable` instead of moving a replacement. Also use `audit_unavailable` and `canceled` where needed. Reuse existing version, identity, selection, digest, and protection errors. Post-dispatch terminal-audit failure is unknown/failed after dispatch, with no automatic retry.

Supported uninstall execution remains current-user MSIX. Do not assume an ARP or MSI succeeded-uninstall path.

### 4. Support audit shape

Do not add a `started` variant or new required fields to `%LOCALAPPDATA%\DevSweep\audit\v1\software.jsonl`. Older readers must still accept that journal.

Strict startup and leftover lifecycle records go to a new file, `%LOCALAPPDATA%\DevSweep\audit\v1\software-support.jsonl`, with its own closed schema version and deny-unknown-fields. Records hold operation, plan, and preview identity plus a terminal result. They do not hold paths, registry bytes, or command lines. Serialize writers with the existing `software.lock`. A durable start is written before dispatch. An unfinished action after dispatch stays unknown and is only re-observed. Never automatically repeat a registry write or a trash move. Do not write a false terminal `failed` V1 record as a substitute for the start record.

Exit classes for the new routes: missing plan, digest, or confirmation is exit 2; changed authority is exit 3; unavailable source or permission is exit 4; truthful partial is exit 5; uncertain dispatch or persistence is exit 6; cancellation before dispatch is 130. A digest binds confirmation. It is not a signature or a proof of origin.
