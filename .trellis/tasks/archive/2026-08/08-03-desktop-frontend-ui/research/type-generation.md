# Desktop IPC type generation

`desktop/src/api/types.gen.ts` is the reviewed TypeScript projection of the
Rust Serde boundary. `quicktype-core` is pinned as a development-only dependency
and `npm run types:generate` uses its API to derive TypeScript from named JSON
fixtures. The generator emits `just-types` output without a runtime converter.

`contract-variants.json` is a controlled sample catalog, not a backend response.
It supplies Serde tag variants, scalar enum values, nullable fields, and omitted
optional fields that do not all occur in one response. The generator infers
field presence and primitive shapes from fixture values. Its ownership graph
only assigns stable Rust-aligned names to nested DTOs. Because quicktype merges
tagged object shapes, the deterministic final step composes the separately
generated variant interfaces into discriminated unions using tag literals read
from the same variants fixture.

`scan-report.real.json` was captured from this read-only command against the
committed controlled fixture:

```powershell
cargo run --quiet --bin devsweep -- scan --json --projects .trellis/tasks/08-03-desktop-frontend-ui/research/fixtures/project
```

That sample exposed the Serde omission of an empty `sizing_warnings` field and
is checked against `model/plan.rs`, `model/scan.rs`, and `scan/mod.rs`. The dry-run
and execution fixtures are contract-complete controlled samples checked against
`execution/mod.rs` and the desktop command tests; the CLI does not expose the
desktop `DryRunOutcome` envelope. They are not evidence that cleanup ran.

The desktop event projection intentionally differs from core's internal
`ScanProgress.partial`: core retains an optional trusted `CleanupPlan`, while
`desktop/src-tauri` clears it before emit and the generated webview contract
therefore accepts only `partial: null`. This prevents reconstructed command
program/argv/cwd values from crossing into the untrusted webview.

All paths outside `scan-report.real.json` are synthetic. Tagged-union variants
not naturally present in one scan are retained so generation review covers
scope, evidence, cleanup intent, action, capacity, outcome, note,
process-status, and command-error tags.

`types-generation.test.ts` runs generation twice, compares it byte-for-byte to
the committed output, checks ASCII/type-only output, and proves that changing a
copied fixture changes the generated contract. The copied fixture is temporary
and is removed by the test.
