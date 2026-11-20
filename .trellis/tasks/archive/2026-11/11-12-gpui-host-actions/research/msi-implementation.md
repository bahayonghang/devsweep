# MSI core/CLI implementation evidence

Date: 2026-11-15. Scope: approved steps 3–4; GPUI MSI step 11 remains closed.
Contract: [msi-implementation-contract.md](msi-implementation-contract.md).

## Delivered code

- `software/msi.rs` and `arp.rs`: positive current-user-unmanaged registration
  proof from Installer enumeration/exact state/package information, cached MSI
  summary, exact GUID restriction keys and Installer policy. No vendor command
  fields. Windows SDK `Msi.h:408` was inspected to verify that the exact product
  state property string is `State`; unknown-property is never absence.
- `software/file_authority.rs`, `msi_authority.rs`: shared physical-file binding
  for MSI and the parallel Updates implementation. Fixed local disks only;
  no-reparse, bounded content hash, retained read-only Windows handles, and exact
  held-handle/canonical-path file identity. System directory API resolution has
  no PATH/WINDIR fallback. Cached package guard is released immediately before
  dispatch; system executable guard remains held through ProcessRunner.
- `software/model.rs`, `mod.rs`, `plan.rs`: optional V1 registration evidence,
  preserved old IDs/fingerprints, one MSI selection, strict GUID and real MSI
  preview strategy. The executor independently reacquires live inventory; saved
  or fixture evidence cannot manufacture host authority.
- `software/execution/{mod,msi,audit}.rs`: typed MSI adapter/requery, exact fixed
  argv, numeric/interrupted result evidence, MSI-specific classification and
  accumulated-evidence replay. After-dispatch audit failure has a typed unknown
  error. Recovery never redispatches. Legacy MSI is readable but cannot authorize
  new leftovers; old MSIX semantics stay unchanged.
- Software audit additionally exports the existing support-lifecycle record
  types and adds Update/RebootRequired for the parallel Updates worker. That
  worker reuses the existing locked `software-support.jsonl`; History integration
  belongs to the parallel Status/History worker. No fake uninstall record.
- CLI Software handlers/presentation and localized messages: existing grammar,
  live MSI preview/execution, explicit confirmation/digest, locale-neutral JSON,
  numeric MSI result and distinct already-initiated restart copy. Necessary TUI
  enum/literal closures only; no new TUI flow. History `status` help manifests
  were synchronized at the Status worker's request.

Public additive fields:

```text
SoftwareEntryV1.msi_registration?: {
  user_sid, package_code, registration_digest, tool_digest
}
SoftwareAuditRecordV1.msi? / SoftwareActionOutcomeV1.msi?: {
  user_sid, process?: { exit_code?: u32,
    interruption?: timed_out | canceled | status_unavailable }
}
SoftwareExecutionError::AuditAfterDispatch(SoftwareAuditError)
```

Absent fields are omitted/defaulted, not fabricated evidence. New populated
fields require synchronized closed readers; old binaries' forward readability
is not promised.

## Checks actually run

All Cargo commands used `RUST_TEST_THREADS=4` and the shared root target slot.

| Check | Observed result |
| --- | --- |
| Initial core/CLI checks | MSI compile issues fixed; remaining intermediate errors belonged to concurrently edited Status/Updates, then their short core check passed. |
| `cargo test --locked -p devsweep-core software` (latest full Software run) | 95 passed, 5 failed, 2 ignored. All five failures were parallel Updates fixture/source-probe expectation mismatches and were sent to that owner. MSI new funnel/matrix/refusal/audit/recovery/legacy tests and old provider fixtures passed. This is not an all-Software pass. |
| `cargo test --locked -p devsweep-cli --lib software` | 24 passed. Includes MSI handler forwarding, confirmation refusal, bilingual `1641`, numeric JSON and hostile/closed fields. |
| `cargo test --locked -p devsweep-cli --test cli_contract` | 17 passed. Includes MSI saved plan/strict GUID/closed optional evidence, unchanged execution grammar and Status history help. |
| Targeted rustfmt and `git diff --check` | Passed for owned changes. |

The final small State-property, local-drive/file-identity and audit-closure
guards compiled with the parallel Status worker's passing core/CLI focused
checks. A new `msi_releases_cache_for_installer_maintenance_but_retains_tool_binding`
fixture and local-disk lexical policy test were added after the full Software
run; the main session must include these in the next root-target verification.
Root `just ci` / standalone `just gpui` are scheduled by the main session, not
claimed here. The Updates worker owns its remaining focused fixture rerun.

## Evidence limits

No Computer Use, UI automation, real `msiexec`, installed-software scan to create
fixtures, host uninstall/update/startup write/process action or power-state
change was performed by this worker. Native package behavior, external custom
actions/UAC, reboot suppression and native screenshots remain unverified.
No commit, push, archive, task-status change or shared spec rewrite was made.
