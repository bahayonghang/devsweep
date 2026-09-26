# Type Safety

## Contract Ownership

Rust core Serde models are the IPC source of truth. `desktop/src/api/types.gen.ts`
is generated from archived command/event samples and then reviewed against the
Rust declarations. It is not a second business model and must not add fields or
execution authority absent from backend payloads.

## Boundary Rules

- Tauri payload keys are snake_case unless the Rust type explicitly uses another
  Serde rename.
- Treat `invoke`, `Channel`, and event payloads as `unknown`. Decode them once in
  `desktop/src/api/contract.ts`; components and reducers import decoded types.
- Model tagged Rust enums as discriminated TypeScript unions using their actual
  `type` or `code` tag.
- Model `TargetId` and `ConfirmationDigest` as strings at the IPC boundary.
- Model `PathBuf` as a string. Never normalize, join, authorize, or infer cleanup
  behavior from it in TypeScript.
- Preserve optional fields and union variants. Do not coerce missing or malformed
  safety fields to permissive defaults.
- Core `ScanProgress.partial` is a trusted internal `CleanupPlan` and must never
  cross into the webview. Rust projects it into a purpose-built
  `ScanPreviewSnapshot` that omits plan version, action/program/argv/cwd, cleanup
  intent, and default selection. TypeScript decodes that closed-world shape; it
  must not recreate authority by stripping or interpreting plan fields.
- Desktop scan progress uses a command-scoped `Channel` envelope with a non-empty
  `scan_id` and positive safe-integer `sequence`. Unknown fields, duplicate target
  ids, malformed totals, and authority-bearing preview fields fail closed.
- Validate dry-run digest/report equality, execution mode, report counts, and
  requested-selection correlation at the IPC boundary before reducer dispatch.
- Exhaustively switch on command-error, action, outcome, capacity, evidence,
  scope, and diagnostic tags. Use an `assertNever` helper for render projections.

## Typed Adapter Boundary

The desktop shell and the CLI reach one typed `devsweep-core` service path. The
Tauri adapter never imports the CLI crate and never launches a `devsweep` child
process; `desktop/src-tauri/src/service_boundary.rs` proves both for every
adapter file and for the crate manifest.

Exactly three TypeScript files may import `@tauri-apps`:

| File | Scope |
| --- | --- |
| `desktop/src/api/bridge.ts` | every five-mode and supporting-domain operation |
| `desktop/src/i18n/index.ts` | presentation settings, a non-domain exception |
| `desktop/src/lifecycle.ts` | window controls, close lifecycle, and DEV fault injection, a non-domain exception |

Components and reducers never call `invoke`. `desktop/src/api/ipc-boundary.test.ts`
enforces the list. Adding a fourth adapter needs a spec change first.

## Fixture Parity

Files under `desktop/src/api/fixtures/` are the shipped wire, not a frontend
mock. `desktop/src-tauri/src/wire_parity.rs` decodes each one into the Rust type
the command returns and serializes it back, so a field either side adds, drops,
or renames fails the Rust gate. A fixture must therefore carry exactly what the
backend emits: a field the Rust type skips when empty is absent from the
fixture too.

`desktop/src/api/fixtures/errors/command-errors.json` holds one payload per
`CommandError` variant and is read by both surfaces. Do not write error payloads
inline in a test.

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
