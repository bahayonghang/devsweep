# Independent backend, contract and asset review

Date: 2026-10-14. Role: independent `trellis-check`. Active review task is the
parent `10-14-cleanup-home-redesign`; the hook's D task context was supplemented
with the parent and A/B/C/E PRD/design/implementation artifacts and current specs.
No D UI/session/style/localization files were edited. No user-cache scan, cleanup,
commit, archive, journal, installer replacement or publication was performed.

## Findings fixed

- `desktop/src/api/fixture-bridge.ts`, `cleanAdvisories`: the original fixture
  replay returned the mixed archived report's `report_id` even for a different
  scoped input report. It also matched only selected project row fields. This
  weakened fixture evidence for observational report correlation. C fixed this
  during coordinated review with core-produced per-scope report/advisory pairs,
  exact replay matching, rejected unrecorded inputs and native parity tests.
- `desktop/src/api/fixture-bridge.ts`, `planDryRun` / `planExecute`: after scopes
  separated, the global-only report had only npm as a selectable row, but the
  replay supported only Cargo alone or Cargo plus npm. Global preview always
  failed with `invalid_plan`. C added the explicitly synthetic npm-only preview
  token/report, matching replay, wrong-token refusal and Rust wire-roundtrip
  coverage. These are controlled fixture results, not real execution evidence.

Both corrections were independently reviewed and verified with 75 API tests,
15 native wire tests, generated-type check, TypeScript and scoped ESLint. No
production defect requiring a reviewer-owned local fix was found in A/B/C/E.
No product source was changed directly by this reviewer.

## Findings not fixed / acceptance limits

- **B remains incomplete:** `scan/global/mbx.rs` reports only a reviewed public
  shared-store location and explicit unmeasured capacity. Per-workspace mbx
  attribution, private/managed output sizing, shared-store occupancy measurement,
  expected-state/lease enforcement, and executable cleanup are not implemented.
  The pinned source audit does not qualify the current `mbx clean` interface for
  DevSweep's saved preview contract. This is a documented product completion gap,
  not a mechanical issue to bypass. Parent AC5 and B executable acceptance remain
  open; observation-only delivery must not be called full mbx support.
- E native taskbar/tray/webview and installer acceptance, D behavior/visuals and
  F native/performance/integrated gates remain outside this bounded pass.
- Existing uv qualification accepts only major 0, minors 4 through 11 inclusive
  (`rules/configuration.rs:258`). The parent reports installed uv 0.13.0, so that
  local version remains unsupported for cleanup. The range was independently
  checked in source; the installed version is parent-provided current evidence.
  No version gate was broadened by this reviewer. Supporting 0.13 requires a
  separate qualification decision/evidence, not a UI compatibility assumption.

## Semantics checked

- A applies project opt-in in shared scan ranking while preserving global policy,
  explicit CLI selection, saved ScanReport V1 and plan V2 shapes. Empty-root global
  scanning and explicit project validation remain separate core/Tauri paths.
- B keeps Node proof and generic no-follow guards. Observed cross-owner Cargo
  overlaps become stable owner-qualified `rust.target.shared` inspect-only rows;
  the registry cannot reconstruct a command/trash action for that rule, and
  executor selection fails before audit or dispatch. Unknown external-target
  capacity produces incomplete root coverage instead of exact zero.
- Footprint grouping is pure lexical observation, not physical reclaim or new
  authority. Nested/equal paths, conflicting measurements, disjoint-child lower
  bounds, shared/mixed/unresolved owners, native root paths and overflow retain
  conservative totals and confidence. Shared bytes do not enter every owner sum.
- C aggregates build/test/tool artifacts separately from dependencies/venvs;
  inclusive 5 GiB and positive whole-GiB bounds are checked. Partial below the
  threshold remains incomplete, while a sufficient lower bound can warn. Settings
  use the existing atomic transaction helper, preserve invalid/future bytes, and
  default without writing. Reprojection does not modify report or selection.
- Advisory DTOs stay separate from cleanup authority. Closed Rust/TypeScript
  shapes, generated types, tagged confidence/assessment consistency and Tauri
  routing were reviewed. Report identity is observational, never a preview digest.
- E's converter uses separated program/arguments, a pinned Tauri CLI and unchanged
  generated artwork. ICNS record ordering changes container order only. Existing
  asset consumers and decorative accessibility remain intact. The actual small
  frame light/dark image was inspected; face/teeth/sage remain recognizable.

## Verification

Commands used `--locked --offline --target-dir dist` for Cargo, without changing
`CARGO_TARGET_DIR`. Frontend commands used the exact installed Node 22.23.2.

| Check | Result |
| --- | --- |
| Core `scan::` library tests | PASS: 88 |
| Core `rules::` library tests | PASS: 26 |
| Core `project_footprints` library tests | PASS: 11 |
| Core `clean_advisories` library tests | PASS: 11 in normal Windows context |
| Tauri `wire_parity::` tests, including scoped advisory and npm-only fixture parity | PASS: 15 |
| Tauri `service_boundary::` tests | PASS: 3 |
| CLI `provider_authority` integration tests | PASS: 5 |
| API `clean-advisories`, `bridge`, `contract`, `ipc-boundary`, `fixture-bridge` Vitest files | PASS: 75 after both coordinated corrections |
| Scoped API/brand/generator ESLint | PASS |
| TypeScript `tsc --noEmit -p tsconfig.json` | PASS |
| Generated types `scripts/generate-types.mjs --check` | PASS after final fixture changes |
| Core clippy `--lib --tests -- -D warnings` | PASS, exit 0; environmental incremental hard-link fallback warnings only |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS; Git line-ending notices only |
| Independent icon export | PASS: all 16 derived hashes match recorded family |
| Manifest/source verification | PASS: all 21 master/provenance/derived records match actual hashes |
| Pre-existing desktop Cargo.toml preservation | PASS: SHA-256 `b7d6f6a30067850f5c4b1f3afde6450667262eb57309d230ca05fe7a0dd84915`; no content diff |
| Final integrated `just ci`, all desktop gates, native/performance acceptance | NOT_RUN by this reviewer; parent/F owns these after concurrent D changes settle |

The sandbox advisory run had 8 pass / 3 failures: Windows denied atomic file
replacement in disposable sandbox Temp directories. The identical source/test
command passed 11/11 in normal Windows context after approved escalation. Scanner
and registry tests likewise used normal context for existing temporary Git fixture
initialization; assertions and production safety policy were not weakened.

Independent export output is retained in `dist/independent-brand-review/`, including
the conversion log, derived family and manifest. It is generated evidence, not a
new product asset location.
