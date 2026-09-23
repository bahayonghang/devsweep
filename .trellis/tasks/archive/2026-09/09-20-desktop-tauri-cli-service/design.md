# Technical design

## Boundary and ownership

`devsweep-core` owns domain requests, safety policy, process execution, plan
authority, typed errors, and operation events. The CLI owns argument parsing
and terminal presentation. `desktop/src-tauri` owns Tauri command adaptation,
window-scoped event delivery, and mapping core results to the existing typed
wire contracts. `desktop/src` owns invocation, decoding, reducers, and
presentation. Tauri does not launch the DevSweep CLI; legitimate provider
commands remain under the existing core process runner with separate argv.

## Data flow

| Entry point | Path | Authority retained |
| --- | --- | --- |
| CLI | arguments → typed core request → core result/events → terminal renderer | core policy and plan/digest |
| Tauri | typed command → Tauri adapter → typed core request → event channel/result → bridge decoder | core policy and operation coordinator |
| Fixtures | typed request/event/result snapshots → shared contract decoder | no execution authority |

The existing `desktop/src/api/bridge.ts` and
`desktop/src/state/operation-coordinator.ts` remain the frontend seams. Tauri
commands continue to use `spawn_blocking` only where the existing core API
requires it; the adapter must not add a second lifecycle or process runner.

### Existing adapter map (TPR-04)

| Surface | Owner and contract | Verification |
| --- | --- | --- |
| Five modes; Protection, Rules, History | `desktop/src/api/bridge.ts:20-47`; corresponding Tauri/core functions | Existing bridge/decoder tests plus CLI/core parity fixtures |
| Presentation settings | `desktop/src/i18n/index.ts:155-167`; closed settings decoder and core persisted store | Existing i18n/App/settings tests; no CLI operation envelope is invented |
| Window close / DEV fault injection | `desktop/src/lifecycle.ts:21-35`; typed lifecycle adapter | Existing lifecycle/close tests; DEV fault command stays absent from release |

Scan/Analyze/Status live use Channels with identity/sequence checks. Software,
Optimize and Status snapshot return typed results; do not add streams solely
for apparent parity (`desktop/src/api/bridge.ts:21-37`). Review the existing
core operation used by each entry point before extracting any new public API.

## Contract rules

- Keep closed request/result/error unions and locale-neutral DTO fields.
- Preserve operation identity on every event and reject stale terminal events.
- Separate cancellation-request acknowledgement from joined terminal result.
  Cooperative operations retain their cancellation tokens; Clean dry-run and
  execute retain wait-for-completion behavior. Apply the performance child's
  operation-specific evidence table for TPR-06; never treat request ACK as join.
  Surface partial and unavailable states explicitly.
- Keep `program` and `argv` as separate values in all command-backed actions.
- Keep the validated plan, selected identities and current preview digest as
  execution inputs. CLI plan files/`--confirm` and desktop in-memory plan/dialog
  are adapter representations of the same authority checks, not new schemas.

## Compatibility and rollback

First align the shared service seam and fixtures, then update adapters only
where the current contract cannot express the core result. Avoid migration
shims or duplicate protocols. If parity tests fail, roll back the adapter seam
change while retaining research and fixture additions; do not introduce an
external subprocess fallback.
