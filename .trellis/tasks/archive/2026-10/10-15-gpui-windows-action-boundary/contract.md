# Windows action boundary contract

Status: version 1, implemented 2026-10-17. This file records the C02 contract. It is not a host catalogue.

## Closed helper operations

The helper accepts only:

- `fixture.backup`
- `fixture.restore`
- `fixture.identity_check`

`external.confirm` is a reserved capability state. The helper never receives it. Any other operation string is unsupported. A string command is not an operation.

## Request

Schema version 1. JSON object, unknown fields rejected. Fields:

- `schema_version`
- `operation_id`
- `operation`
- `preview_digest` (`sha256:` plus 64 lowercase hex digits)
- `caller_user_sid`
- `tool_identity_hash` (`sha256:` plus 64 lowercase hex digits)
- `target_identity`

The request has no program path, argv, shell string, or registry path. The service loads the isolated root from the plan created by `prepare_windows_fixture`. The subject paths are fixed: `subject.bin` and `subject.hiv`.

## Privilege

The normal service does not open a Windows consent prompt and does not keep a service process. Tests inject `InjectedElevation::{Consented, Declined, MissingPrompt}` and the helper binary path. A real UAC prompt is **NOT_RUN**.

One elevated or user-scope call spawns the helper at most once. The child stdin is a sealed `DSWA1` frame. The child argv is empty.

## Journal

`audit/v1/actions.jsonl` under the isolated root. Schema version 1. Domain `windows_action`.

- `dispatch_started` is written and synced before the helper is spawned.
- Cancellation, an injected decline, and a missing prompt write a terminal record and do not write `dispatch_started`.
- Forged, replayed, stale, cross-user, tool-replacement, path-replacement, and unsupported requests write nothing.
- After `dispatch_started`, a helper disconnect, timeout, crash, or missing terminal stays without a success terminal. The service does not spawn the helper again for that call. The same operation id is spent.

## Backup

`backup/vN/` stores schema version 1. Files: `file.bin`, `hive.bin`, `registry-value.bin`, `manifest.json`. The manifest stores file identity, hive identity, and the SHA-256 of the exact bytes. It stores no path and no command.

`fixture.backup` writes the next version, then reads the subject again. Identity or byte drift deletes that version and is not success.

`fixture.restore` writes only when the live file identity and hive identity still equal the latest manifest. A changed subject does not receive the backup bytes. Restore of the latest version is not a generic undo of an older version.

The hive file is an offline `regf` hive opened with `offreg.dll`. It is not loaded into HKCU or HKLM.

## Outcomes that are not success

`authorization_declined`, `consent_missing`, `canceled`, `audit_failed`, `read_back_failed`, `refused`, and `unknown` (`timeout`, `disconnect`, `crash`, `missing_terminal`).
