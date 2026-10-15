# C implementation handoff — 2026-10-14

## Delivered contract

Implemented `devsweep_core::clean_advisories` with pure
`project_capacity_advisories(report, threshold_gib)`, the session-only validator
`clean_advisory_preferences`, and focused load/save functions. The public DTOs
are `CleanAdvisoriesV1`, `CleanAdvisoryPreferencesV1`,
`ProjectCapacityAdvisoryV1`, `ProjectCapacitySubtotalV1`, `CapacityConfidence`,
and `CapacityAssessment`. The approved default is inclusive 5 GiB. Build/test/tool
artifacts and dependencies remain separate. B's footprint seam owns aliases,
overlap and shared/unresolved attribution; this projection excludes ambiguous
bytes and preserves incomplete coverage. Successful zero-target coverage can
produce exact zero; missing/failed coverage cannot.

The new closed settings file is `clean-advisories-v1.json`; roots and appearance
stores are unchanged. Bounds are 1..=8,388,607 whole GiB. Missing reads are
write-free; corruption/future/unknown fields remain intact. Saves reuse the
existing lock and atomic replacement helper, including failure cleanup and
concurrent whole-value transactions.

Thin Tauri commands are `clean_advisory_preferences_get`,
`clean_advisory_preferences_set`, and `clean_advisories`, all using
`spawn_blocking`. DesktopBridge exposes `cleanAdvisoryPreferencesGet`,
`cleanAdvisoryPreferencesSet(thresholdGiB)`, and
`cleanAdvisories(report, thresholdGiB)`; IPC arguments use `thresholdGib`.
Strict decoders and generated types are included. No ScanReport V1/plan V2
extension or cleanup/selection/preview authority change was made.

## Changed files

- Core: new `src/clean_advisories/{mod,preferences,tests}.rs`, public `lib.rs`
  export, and `examples/clean_advisory_fixture.rs`.
- Native desktop: new `src-tauri/src/clean_advisories.rs`, command registration,
  service-boundary inventory, and wire/projection parity tests.
- API: bridge, contract decoders, fixture bridge, generated types and generator
  mappings; new `clean-advisories.test.ts` and extended bridge/fixture tests.
- Fixtures: controlled advisory input/output/preferences, archived scan advice,
  and Rust-generated scoped report/advisory replay pairs. Added explicit
  npm-only dry-run fixture so the global fixture lane can preview/confirm. Its
  `fixture-npm-only-preview-v1` token is controlled replay data, not a live
  cleanup preview digest. Historical saved scan fixtures remain unchanged.
- Analyze benchmark bridge stub: only added required unused advisory methods
  for interface parity; no benchmark rendering changes.
- Specs: new backend `clean-advisories.md`, backend index, desktop type safety,
  and code map entries.

## Verification evidence

- Core focused suite: final 11 tests independently passed on native Windows
  outside the sandbox; independent sandbox run passed 8 and encountered 3
  Windows atomic replacement access-denied failures. Earlier identical focused
  suite with 10 tests also passed outside the sandbox. No store test accesses
  user settings: all files are isolated tempfile fixtures.
- Pure projection tests: 7 passed locally, including T-1/T/T+1, accumulated
  smaller outputs, separate dependencies, partial/unknown assessments,
  nested/shared/unresolved storage, exact-empty coverage, original report
  preservation, safe integer bounds and drive-root/sibling-prefix coverage.
  The new volume-root regression initially failed and passed after normalizing
  ancestor comparison correctly (`C:/` normalizes to `c:` on Windows).
- Core clippy all-targets with `-D warnings` passed before the final small
  ancestor-test/fixture follow-up; integration owns final workspace clippy.
- Initial native advisory projection/wire test passed. Added final scoped
  replay parity and npm-only wire roundtrip; independent reviewer owns the
  final native suite run and reports its result to the parent.
- API focused run: 5 files / 74 tests passed after scope/identity fixes.
  Final npm-only fixture addition: `fixture-bridge.test.ts` 7/7 passed
  (previous full-run fixture count was 6). This is 75 distinct focused tests;
  do not claim a single final 75-test run unless the reviewer records it.
- TypeScript and scoped ESLint passed before the final fixture-only changes;
  final checks assigned to independent backend reviewer. Generated types were
  regenerated with exact Node 22.23.2, and prior generation `--check` passed.
- Rust fixture generator ran successfully after regeneration. Sandbox emitted
  incremental-cache hardlink fallback-to-copy warnings, without build failure.
- `git diff --check` passed after the implementation/doc changes.

Rust commands used `--locked --offline --target-dir dist`, without setting
`CARGO_TARGET_DIR`. JavaScript used the exact mise Node 22.23.2 executable.

## Scope and outstanding acceptance

C core/contract implementation is handed off. D owns settings controls,
session-only recovery/error copy, completed-scan presentation and UI retention
of selected IDs/preview/confirmation. Backend tests prove the projection does
not mutate the report; final cross-layer UI authority retention remains D/F.
No new CLI output shape was needed: CLI and Tauri share the existing report
types and core projection. Runtime mbx executable qualification and actual
owned artifact availability remain B's explicitly documented limitation; a
global shared path is not measurable per-project storage here.

Independent backend review is in progress; parent/F owns final `just ci`,
native UI/visual/performance gates and cross-platform execution. None of these
are claimed complete by this handoff. No real cleanup, commits, archive,
journal or installer replacement was performed. The pre-existing Tauri
Cargo.toml modification was preserved.

### Independent focused closeout

The backend reviewer subsequently reported final API 5 files / 75 tests PASS,
Tauri wire parity 15 PASS (including scoped report/advisory pairs and npm-only
preview), service boundary 3 PASS, TypeScript PASS, generated types `--check`
PASS, and scoped API/fixture/generator ESLint PASS. The two fixture integration
findings were fixed and independently verified; evidence is in the parent
`evidence/independent-backend-review.md`. Integrated/native acceptance remains
with parent/F.
