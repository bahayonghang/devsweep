# Scan coverage contract

## 1. Scope / Trigger

Use this contract when changing project/global discovery, scan health, CLI
Clean output, or Tauri report decoding/presentation. Coverage explains what
was examined; it never grants cleanup authority. Core models in
`crates/devsweep-core/src/model/scan.rs` own the wire shape.

## 2. Signatures

```rust
pub struct ScanHealth {
    pub completeness: ScanCompleteness,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub totals: ScanTotals,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverage: Vec<ScanCoverage>,
}

pub struct ScanCoverage {
    pub id: String,
    pub scope: ScanCoverageScope,
    pub path: Option<PathBuf>,
    pub status: ScanCoverageStatus,
    pub target_count: usize,
    pub detail: String,
}
```

The scope wire tags are `project` and `provider`. Structs deny unknown fields;
enum variants serialize as `snake_case`. ScanReport remains version 1 and its
embedded untrusted cleanup plan remains version 2.

## 3. Contracts

- Global-only requests use `include_global=true`, `include_projects=false` and
  may have no roots. They never call project discovery. Project-only requests
  use the inverse flags and explicit roots; they never query global providers.
  Desktop requires absolute roots. CLI retains its `projects|global|all` grammar
  and default `all` scope with `.` root; do not transfer that implicit root to
  desktop requests.
- Emit one aggregate entry per `(scope, id)`. Provider fallback attempts belong
  to that provider's final outcome, not duplicate coverage rows. Repeated root
  selections must not duplicate the same effective project entry.
- Project coverage records the effective normalized root; provider coverage
  records its resolved path when one is available. An unresolved optional
  provider can have `path: null` and a precise typed outcome.
- `target_count` describes that coverage unit's observations. Do not sum these
  counts or multiply them by estimates to reconstruct report totals. The
  completed deduplicated plan owns capacity totals and selection eligibility.
- `detail` is sanitized explanatory text. Never parse it into a rule, path,
  action, command or permission decision; typed fields own classification.
- Missing legacy `coverage` decodes as an empty vector and reserializes without
  that field. It means coverage was not recorded, not that all folders were
  checked. Preserve legacy completeness/diagnostics without inventing scope.
- An independently failed root/provider does not erase healthy observations.
  Invalid external plans still fail whole-plan validation. Discovery diagnostics
  are not an exception to the execution trust boundary.
- Coverage stays visible in empty and partial results. CLI human/JSON and
  Tauri must expose equivalent core facts for equal effective input options.
- Progress observations remain read-only and cannot become a completed plan.
  Scope, selection, protection and reviewed-rescan changes invalidate stale
  preview/confirmation state through the existing workflow.
- An unsafe Windows cache root that cannot be represented by the trusted rule
  registry belongs in typed coverage, not in an invalid Inspect Only target
  that would make healthy siblings fail whole-plan validation.
- Bounded project-marker enumeration must distinguish exhausted/unreadable
  discovery from verified absence. A .NET marker beyond the cap yields an
  incomplete budget outcome; it must not silently disappear into empty coverage.
- Reviewed sizing replaces size observations, then rechecks a closed generated
  or Windows rule's current eligibility to retain a useful refusal reason.
  Successful sizing or newly idle ownership never promotes an existing Inspect
  Only intent; eligibility changes require a fresh scan and preview.

## 4. Validation & Error Matrix

| Coverage status | Completeness effect | Presentation |
| --- | --- | --- |
| `complete`, `empty` | No partial effect by itself | Successfully examined, with targets or no targets |
| `absent`, `unsupported` | Optional absence/platform support is not a probe failure | Explain absent tool/path or unsupported provider |
| `skipped`, `protected`, `busy`, `ownership_unknown` | Partial | Explain why requested observation could not complete safely |
| `failed`, `timeout`, `output_truncated` | Partial | Show provider/root failure; never claim zero cleanable data |
| `canceled`, `budget_limited` | Partial | Keep partial/unknown size semantics and recovery guidance |
| Missing legacy coverage | Preserve recorded legacy health | Coverage not recorded; no whole-machine claim |
| Unknown enum/authority-bearing field | Reject at the closed wire boundary | Structured decoding error, no permissive fallback |
| Duplicate `(scope, id)` coverage entry | Reject malformed external payload | Do not silently select one conflicting observation |

Sizing and other diagnostics may independently make an otherwise successful
coverage unit's overall report partial. Optional absence never suppresses an
actual failed selected-root observation.

## 5. Good / Base / Bad Cases

- Good: one unreadable project root and one healthy root produce a partial
  report with both coverage entries and the healthy targets intact.
- Base: a genuinely empty valid root yields an `empty` entry and zero targets.
- Good: an older report lacking coverage remains readable with an explicit
  unavailable-coverage explanation.
- Bad: a failed provider returns an empty target list and the UI claims the
  system is clean; or the adapter drops an invalid action to bypass validation.

## 6. Tests Required

- Real scan-to-report-to-v2-validation fixtures for mixed healthy/failed roots
  and provider failures; assert retained healthy targets and typed partial state.
- Legacy missing-field roundtrip, duplicate-unit rejection and closed unknown-field/enum rejection.
- Provider absence, fallback, timeout, truncated output, cancellation and
  budget fixtures; assert status/completeness mapping rather than text alone.
- CLI human/JSON and desktop empty/partial display parity, including no
  invented freed-space claim and no selection derived from coverage entries.
- Malformed/reparse cache roots beside healthy targets, marker discovery past
  the enumeration cap, and busy/unknown-owner reviewed rescans retaining the
  refusal and Inspect Only intent. Python discovery and live validation share
  the same marker catalogue; a nearer marker added after preview refuses
  execution before any runner call.
- Regenerated TypeScript sample parity with Rust Serde; skipped empty fields
  are absent in fixtures, and nonempty samples exercise every shipped tag.

## 7. Wrong vs Correct

Wrong: `targets.is_empty()` implies complete coverage or successful cleanup.

Correct: show completed-plan targets together with the recorded health and
coverage; execution outcomes come from the executor's separate result. A scope
entry never substitutes for a validated intent and live preview digest.
