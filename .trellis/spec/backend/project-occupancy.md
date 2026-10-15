# Project ownership and observational occupancy

## Scope and boundaries

Use this contract when changing project deduplication, shared Cargo targets,
mbx reporting, or per-project capacity advisories. `Scope::Project { root }`
is the report's owner identity. Marker/generated-content proof and live cleanup
authorization remain separate from this read-only projection.

`project_footprints::project_footprints(&[UntrustedTarget])` is crate-private and
pure. It does not touch paths or run commands when a completed report is replayed
or a threshold changes. It returns owner roots, source row indices, category,
logical byte lower bound, completeness, and `Owned|Shared|Unresolved` attribution.

## Required behavior

- Equal and nested normalized paths form one observational footprint regardless
  of action. A complete enclosing measurement covers its children. Incomplete
  parent measurements retain the greater of their measured lower bound and the
  sum of disjoint child lower bounds. Conflicting equal-path complete sizes or
  children exceeding an enclosing total make that footprint incomplete.
- Normalize lexically with the shared platform-aware path function. Do not
  canonicalize, follow links, or capture new live identities during projection.
  Different unproven aliases and hardlinks are not asserted equal; all reported
  bytes are logical occupancy, never a promise of physical reclaim.
- Build/test/tool-cache observations are `Artifact`; dependencies and virtual
  environments are `Dependency`. An overlapping group with mixed categories is
  unresolved. Cross-project or project/global overlaps are shared. A missing,
  relative, non-Unicode, or invalid path/owner is unresolved. Never count shared
  or unresolved bytes once for each project.
- Existing action rows and selection are not mutated by projection. Generic
  no-follow traversal and Node marker/generated-content/busy rules stay intact.
- Scanner dedupe must not discard competing observed owners just because their
  action method matches. Same-owner normal deduplication keeps its prior rules.
- When Cargo target footprints overlap between distinct **observed** workspace
  owners, downgrade every such Cargo row to catalogue rule `rust.target.shared`.
  These rows have stable owner-qualified IDs, medium risk, reversible inspect-only
  intent, and no default selection. Retain each owner for explaining ambiguity.
  The registry supports only inspection for this rule; neither built-in commands
  nor trash can be reconstructed from it. Selection is rejected before preview,
  audit, or execution. Normal single-owner `rust.target` IDs/actions are unchanged.
  This detects consumers present in the scan, not all possible external users.
- A Cargo target with unsafe or unverifiable no-follow metadata emits a Discovery
  diagnostic and incomplete selected-root coverage, even when outside that root.
  Missing ordinary build output is still absent; an uninspected managed target
  must not become a complete zero-capacity observation.

## mbx reporting boundary

Global scans alone emit provider coverage `mbx.shared-cache`. Through the existing
bounded provider runner, query `--version` and only for exact reviewed 1.24.0 query
`cache dir --json`. Accept closed `{version:1,store:absolute-path}` data. The path
is a reported location, not proof it exists or is safe to traverse.

Missing tool is `absent`; unsupported tool/schema is `unsupported`; malformed,
relative, control-bearing or parent-traversing paths are `failed`. Runner timeout,
truncation, cancellation and phase-budget statuses remain typed. Supported location
reporting still yields `unsupported` coverage with explicit unmeasured-capacity and
unqualified-workspace-cleanup explanation, zero **targets**, and no capacity claim.

Do not run stats, parse project tables, inspect private metadata, size shared stores,
follow managed links, or emit `mbx clean`, `gc`, or `cache remove` actions from this
limited adapter. mbx workspace attribution and executable cleanup remain open until
the owner provides a qualified footprint/expected-state contract. The pinned source
audit is in the 10-14 cleanup-home parent research; successful fixtures do not
substitute for that missing runtime contract.

## Verification

Cover duplicate/nested paths, conflicting and partial sizes, disjoint child lower
bounds, shared/global overlap, dependencies, unresolved paths and 10k-row projection.
Use real project scan fixtures for Cargo workspace members, shared external outputs,
stable IDs under root reordering, closed report/registry validation, and inspect-only
preview refusal. Keep sentinel files intact. Exercise mbx absent/unsupported/invalid
and process failures with an allowlisted fake probe that fails on any traversal or
unexpected command. Retain the existing Node proof and no-follow tests.
