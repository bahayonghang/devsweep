# Type Safety

## Contract Ownership

Rust core Serde models are the IPC source of truth. `desktop/src/api/types.gen.ts`
is generated from archived command/event samples and then reviewed against the
Rust declarations. It is not a second business model and must not add fields or
execution authority absent from backend payloads.

## Boundary Rules

- Tauri payload keys are snake_case unless the Rust type explicitly uses another
  Serde rename.
- Treat `invoke` and `listen` payloads as `unknown`. Decode them once in
  `desktop/src/api/contract.ts`; components and reducers import decoded types.
- Model tagged Rust enums as discriminated TypeScript unions using their actual
  `type` or `code` tag.
- Model `TargetId` and `ConfirmationDigest` as strings at the IPC boundary.
- Model `PathBuf` as a string. Never normalize, join, authorize, or infer cleanup
  behavior from it in TypeScript.
- Preserve optional fields and union variants. Do not coerce missing or malformed
  safety fields to permissive defaults.
- The desktop shell must clear `ScanProgress.partial` before emitting progress
  into the untrusted webview because the core value is a trusted `CleanupPlan`
  containing reconstructed actions. The reviewed desktop event contract accepts
  only `partial: null`; phase and message remain the progress source of truth.
- Validate dry-run digest/report equality, execution mode, report counts, and
  requested-selection correlation at the IPC boundary before reducer dispatch.
- Exhaustively switch on command-error, action, outcome, capacity, evidence,
  scope, and diagnostic tags. Use an `assertNever` helper for render projections.

## Generation And Review

`npm run types:generate` reads JSON samples under
`desktop/src/api/fixtures/`, validates their required envelopes, and writes the
deterministic generated contract file. The task research record states how each
sample was obtained and which Rust declarations were manually checked.

When backend models change:

1. Refresh real scan and dry-run samples.
2. Run `npm run types:generate`.
3. Review the generated diff against Rust Serde tags and field names.
4. Run typecheck, tests, and build.

Generated files carry a header naming the generator and must not be edited by
hand. Presentation-only types such as reducer state and view filters live
outside `types.gen.ts`.
