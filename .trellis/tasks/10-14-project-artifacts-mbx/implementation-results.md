# B implementation results — safe partial delivery

Date: 2026-10-14. User authorized implementation in task order. Parent confirmed
source-dependent baseline frozen and A's canonical project opt-in policy stable.
No real user-cache scan/cleanup, mbx stats/gc/clean, commit, archive, journal,
installer replacement, or publication was performed.

## Implemented

1. `project_footprints.rs` provides a pure crate-private projection for C. It groups
   same/nested lexical footprints independently of action, retains source row
   indices and original owner spelling, excludes shared/unresolved ownership from
   exact per-project attribution, and separates artifacts from dependencies/venvs.
   Partial parents can use disjoint-child lower bounds. Conflicting observations,
   invalid paths, mixed categories and arithmetic overflow cannot claim exact
   totals. No filesystem or command access occurs when settings reproject a report.
2. Known cross-owner Cargo target overlaps now become catalogue-backed
   `rust.target.shared` inspect-only observations. Owner-qualified IDs are stable
   under root reordering; normal `rust.target` IDs and single-owner commands remain
   unchanged. The registry accepts distinct observational owners but cannot rebuild
   a command/trash action for this rule. Executor selection is refused before
   preview/audit/dispatch. This resolves both shared-action ambiguity and duplicate
   rule/path IDs in a real scan fixture. It detects only owners observed in the scan.
3. Cargo target no-follow refusals and inspection failures now emit diagnostics
   even for targets outside selected roots. Selected-root coverage remains partial;
   an unavailable managed directory does not masquerade as a complete empty result.
4. Global-only provider coverage `mbx.shared-cache` uses bounded `--version`, and
   only exact reviewed 1.24.0 uses public `cache dir --json`. Missing, unsupported,
   malformed, relative/ambiguous, timeout, truncated, canceled and budget states are
   explicit. A valid reported location still states capacity is **unmeasured**, not
   zero, and is not attributed to projects. No new target or cleanup intent is
   emitted. No shared-store enumeration or internal metadata parser was added.
5. Existing Node marker/generated-content/tracked-file/sensitive-content/busy
   safeguards and generic no-follow policy were retained and exercised by their
   existing focused suites.

## C/D handoff

Import `crate::project_footprints::{project_footprints, FootprintCategory,
FootprintAttribution}` from core. Output `ProjectFootprint` fields are
`owner_roots`, `target_indices`, `category`, `logical_bytes`, `size_complete`, and
`attribution`. Categories are `Artifact|Dependency|Other|Mixed`; attribution is
`Owned|Shared|Unresolved`. Shared/unresolved owners must remain incomplete.

Projection uses lexical identity only; it cannot assert unproven physical aliases
or hardlinks equal and never promises physical reclaim. Scope rows remain the
original reports and cannot create cleanup authority.

The mbx provider id is `mbx.shared-cache`. The supported location-only case uses
`Unsupported` with explicit fixed explanatory detail and `target_count=0`; that
count is a count of cleanup targets, **not measured zero capacity**. Existing
closed ScanReport V1 and plan V2 wire fields are unchanged. New rule IDs are
strings, so no TypeScript enum generation is required for this rule. Older builds
which lack the new rule correctly reject it at registry validation; prior saved
plans remain readable by the new build.

Cargo skipped-target diagnostic stage is `Discovery`; outcome is `Skipped` for
unsafe/non-directory metadata or `Failed` for inspection I/O. Its path can be
external. C must also consult the affected selected-root coverage, not only path
containment of the diagnostic. No mbx-specific owner is inferred from a link.

## Verification

- `cargo test --locked --offline --target-dir dist -p devsweep-core project_footprints --lib`:
  **11 passed**, including 10k rows, root containment and overflow handling.
- `cargo test --locked --offline --target-dir dist -p devsweep-core scan:: --lib`:
  **88 passed** in normal Windows context. Earlier default-sandbox attempt had
  87 pass / 1 failure because an existing generated-rescan fixture's `git init`
  could not write its isolated Temp/.git/config. The same focused suite passed
  after approved normal-context execution; no test assertion was weakened.
- `cargo test --locked --offline --target-dir dist -p devsweep-core rules:: --lib`:
  **26 passed** in normal Windows context, using isolated Git fixtures.
- New real shared-Cargo fixture exercises scan -> V2 conversion -> production
  registry validation -> executor preview selection refusal; forged built-in
  intent is rejected and shared sentinel bytes remain unchanged.
- `cargo clippy --locked --offline --target-dir dist -p devsweep-core --lib --tests -- -D warnings`:
  detected one B needless borrow, corrected, and two C-owned in-progress lints
  (`manual_is_multiple_of`, `collapsible_if`), handed to coordinator/C. Final
  integrated lint rerun is owned by parent/F; do not report this attempt PASS.
- Scoped rustfmt and `git diff --check` passed before handoff. Final integrated
  `just ci`, CLI/native parity, native UI and performance remain parent/F work.

## Open requirements and limitations

**B remains in progress; executable mbx cleanup is NOT QUALIFIED / NOT IMPLEMENTED.**
The pinned source audit found no complete owner-side expected-state/lease/report
contract. This work does not implement per-workspace mbx attribution, managed
target/private-state sizing, shared-store occupancy measurement, live config/link/
tool drift binding, real busy/lease proof, or sentinel-tested mbx execution.
Fake process statuses test propagation only, not actual mbx busy detection.

The parent must not mark AC5 or the whole requested mbx cleanup delivered. A
qualified upstream/companion owner contract is the remaining technical dependency;
an isolated conformance run alone does not supply missing runtime guarantees.
No user confirmation is requested to bypass these safeguards. Independent review
and final gates remain pending.
