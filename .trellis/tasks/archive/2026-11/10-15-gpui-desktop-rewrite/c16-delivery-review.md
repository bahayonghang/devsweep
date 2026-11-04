# C16 reviewed change and CI proposal

Date: 2026-10-15. This is a reviewable delivery proposal, not commit/push
authorization and not completion of the GPUI rewrite.

## Change to review

- The existing parent and 16 children retain Windows x64-first delivery,
  cross-platform CLI support, CLI-first ordering and approved performance budgets.
- C16 updates shared Rust dependencies and compatible Windows/SHA-2 call sites,
  preserves digest bytes and persisted formats, and records exact versions,
  upstream constraints, licenses, advisories and GPUI candidates.
- The independent review preserves the public Status sampler's thread-safety
  contract, corrects Unix Optimize lock release with a deterministic regression,
  and fixes an existing CLI test decoder's optional GPU/thermal outcome drift.
- CI explicitly selects core/CLI for non-Windows and MSRV jobs. Specs, code map
  and bilingual CI docs match the implemented behavior. No C01 or GPUI UI code
  is included; C01 has read-only implementation research.

## Verified local scope

- Windows Rust/Cargo 1.99.0: isolated workspace tests 794 PASS, 5 deliberately
  ignored; format/check/Clippy PASS.
- Existing Linux WSL Rust/Cargo 1.98.0: core/CLI check, 674 tests and Clippy PASS.
- Retained Node 22/npm 10 frontend: 471 Vitest + 5 launcher tests PASS;
  types/lint/typecheck/build PASS. Documentation build PASS.
- Current lock: cargo-audit and cargo-deny PASS under the unchanged policy;
  constrained legacy notices remain recorded.
- Final-source Tauri no-bundle build PASS; binary SHA-256
  `df36cc169970fa0b137eb87e4a7442af331c60fad9a2b93a6d46882439e2320f`.
  The outer receipt printer encountered a GBK display error after the successful
  build and saved receipt; it is recorded separately from subprocess exit 0.

See [C16 acceptance evidence](../10-15-gpui-latest-dependencies/acceptance-status.md)
and [independent review](../10-15-gpui-latest-dependencies/research/check-review.md)
for precise commands, source identity, retained failed attempts and limitations.

## Remaining acceptance boundary

The default shared Cargo target prevents a complete canonical `just ci`: a core
test executable is missing before execution, including in an unsandboxed attempt.
The isolated equivalent gates pass; the failed canonical receipts remain failures.
Current-source hosted Windows/Ubuntu/macOS jobs and exact declared MSRV jobs have
not run. Earlier mixed Linux 1.88-rustc/1.98-Cargo checks are not exact MSRV proof.
The user's selected local compiler is 1.99.0; no older toolchain was installed.

C16 stays in progress and C01 integration stays unstarted until its dependency
gate is accepted. No native GPUI functionality or performance has been accepted.

## Proposed next action requiring separate authorization

1. Commit the reviewed parent/child task documents, C16 source/lock/tests,
   specs and verification records on `codex/gpui-desktop-rewrite`. Generated
   `target/`, `dist/`, app binaries and ignored build output are excluded.
2. Push this branch to `origin` (`bahayonghang/devsweep`).
3. Dispatch the existing `CI` workflow against that exact branch commit and
   inspect its results. A baseline/default-branch run cannot validate these edits.
4. Resolve any verified failures, record evidence, and continue to C01 once
   required C16 evidence is accepted. This proposal does not archive the parent
   or label the rewrite finished.

Proposed commit subject:
`chore: prepare GPUI rewrite and refresh shared Rust dependencies`.

The approved [implementation scope](implementation-approval.md) reserves commit
and other delivery actions for separate authorization. None of the proposed
Git/remote actions above has been performed.

Subsequent user decision: the current snapshot commit/push is now explicitly
authorized by [delivery-authorization.md](delivery-authorization.md). The user
requested the remaining implementation as a next-conversation prompt; this does
not change the unaccepted C16 evidence state above.
