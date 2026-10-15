# Proposed additive CLI contract

Status: design proposal; these additions are **not currently available commands**.
Existing grammar is documented by crates/devsweep-cli/src/application/cli.rs.

## Keep existing commands

`clean {scan,plan,preview,execute,protect,rules}`,
`software {inventory,plan,preview,uninstall}`,
`optimize {list,plan,preview,run}`,
`analyze scan`, `status {snapshot,live}`,
`history {list,show}`; bare devsweep remains the TUI.

## Proposed additions and delivery owner

| Family | Proposed leaves | Plan identity and dependency |
| --- | --- | --- |
| clean ai | scan; plan; preview; execute; recovery-list; restore-plan | C15 owns these parser additions after C01: provider/version/data-class identity; session/worktree opt-in authority and verified recovery. Restore-plan produces a restore intent for the same preview/execute path. Tool uninstall routes to Software. |
| software updates | check; plan; preview; execute | C01 exposes check; C05 adds exact package ID/source/version plan, provider fingerprint and post-update requery. |
| software startup | list; plan; preview; execute; restore-plan | C01 exposes current-user capabilities under strict wrappers; C05 adds versioned backup and conditional restore. Restore-plan goes through preview/execute too. |
| software leftovers | scan; plan; preview; execute | C01 exposes existing core; C05 expands only proven ownership/registry residue. |
| analyze | roots; list; explain; reveal; trash-plan; trash-preview; trash-execute | C01 exposes existing supported actions; C07 adds projections/explanations. Snapshot + root/node identity is revalidated live. |
| status | process show; process plan; process preview; process execute | C08 provides details and distinct termination authority bound to PID + creation identity; no implicit escalation. |
| optimize | existing leaves, with a versioned multi-selection plan when supported | C06 extends closed catalogue; legacy single-operation plans retain their meaning. |
| status keep-awake | run; status; stop | C08 owns the shared service and CLI before C13 consumes it. Explicit finite lease, confirmation and owner-bound session token; stop/expiry/owner exit releases only this request. No persistent power-setting change. |

The leaf names are a reviewable proposal, not a frozen public API. C01 freezes shared
Clap conventions; each delivery owner freezes its additions before implementation.
New mutation commands take a saved domain
plan, live `--preview-digest sha256:...`, and `--confirm`, except bounded
session control whose explicit finite lease/session contract is owned by C08.
No `--execute` or `--allow-permanent-delete` flag is introduced.

AI restore is a mutation: `clean ai restore-plan` records the recovery-bundle identity,
destination, provider/index version and expected current state; `clean ai preview`
revalidates them live and `clean ai execute` requires that preview digest and explicit
confirmation. Revalidate bundle integrity and refuse external changes or uncertain
ownership. Recovery listing alone cannot satisfy the restore acceptance requirement.

Inputs use stable IDs, not localized names, row indices, arbitrary registry commands
or reconstructed paths. Default scan behavior is explicit in help and never
expanded silently by a GPUI button. Read-only commands can save observations;
writing an observation is not execution authority.

## Output compatibility

Preserve V1 envelope meanings, existing exit classes, create-new output behavior,
stdout/stderr separation and locale-neutral codes. Version incompatible new data
instead of widening old deny-unknown-fields contracts silently. Human text and
generated references cover English and Simplified Chinese.

Plans and previews explain scope, capability, permissions, potential data loss,
restore limits, expected reboot/external installer and exact selected action.
Failures remain distinguishable from unknown-after-dispatch. No automatic retry.

## Compatibility and tests

Freeze command-reference fixtures from the actual Clap parser. Test no-GUI use of
every new capability, cross-consumer report/preview parity, missing plan/digest/
confirmation, corrupted/future schema, stale identities, ambiguous provider output,
partial sources, audit failure and sink failure. No destructive live user operations
are needed to validate parser or refusal behavior.
